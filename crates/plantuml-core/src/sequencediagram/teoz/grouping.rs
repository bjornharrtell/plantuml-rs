//! Frames around events: `alt`, `loop`, `group`, `partition`... (`GroupingTile`, `PartitionTile`,
//! `ElseTile`, `Blotter`).

use super::playing::build_one;
use super::{Tile, TileArguments, YGauge};
use crate::klimt::color::HColor;
use crate::klimt::creole::display::Display;
use crate::klimt::drawing::ugraphic::{UGraphic, UPath, URectangle, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::real::{self, Real};
use crate::sequencediagram::model::{Event, Grouping, GroupingType, Note};
use crate::skin::component::{Area, Component};
use crate::skin::rose::Rose;
use crate::style::PName;
use std::cell::RefCell;
use std::rc::Rc;

pub const EXTERNAL_MARGIN_X1: f64 = 3.0;
pub const EXTERNAL_MARGIN_X2: f64 = 9.0;
pub const EXTERNAL_MARGIN_Y: f64 = 4.0;
const MARGIN_X: f64 = 16.0;
const MARGIN_Y_MAGIC: f64 = 20.0;
const TITLE_VPAD: f64 = 4.0;

/// Horizontal extent of a frame, shared with the `else` tiles it holds (`getFrameMinX`...).
#[derive(Default)]
pub struct Frame {
    min: RefCell<Option<Real>>,
    max: RefCell<Option<Real>>,
    /// `getMinX()` of the group tile, i.e. including margins and notes.
    min_x: RefCell<Option<Real>>,
}

impl Frame {
    fn min(&self) -> Real {
        self.min.borrow().clone().expect("frame min")
    }

    fn max(&self) -> Real {
        self.max.borrow().clone().expect("frame max")
    }

    fn min_x(&self) -> Real {
        self.min_x.borrow().clone().expect("frame min x")
    }
}

/// Background stripes of a frame, one color per branch (`Blotter`).
struct Blotter {
    dim: Dimension,
    default_back: HColor,
    round: f64,
    last: HColor,
    changes: Vec<(f64, HColor)>,
}

impl Blotter {
    fn new(dim: Dimension, default_back: Option<HColor>, round: f64) -> Blotter {
        let default_back = default_back.unwrap_or(HColor::TRANSPARENT);
        Blotter {
            dim,
            last: default_back.clone(),
            default_back,
            round,
            changes: Vec::new(),
        }
    }

    fn put(&mut self, y: f64, color: HColor) {
        match self.changes.iter().position(|(k, _)| *k >= y) {
            Some(i) if self.changes[i].0 == y => self.changes[i].1 = color,
            Some(i) => self.changes.insert(i, (y, color)),
            None => self.changes.push((y, color)),
        }
    }

    fn add_change(&mut self, y: f64, color: Option<HColor>) {
        let color = color.unwrap_or(HColor::TRANSPARENT);
        if color == self.last {
            return;
        }
        self.put(y, color.clone());
        self.last = color;
    }

    fn close_changes(&mut self) {
        let h = self.dim.height;
        let d = self.default_back.clone();
        self.put(h, d);
    }

    fn rectangle(&self, i: usize, height: f64) -> crate::klimt::drawing::ugraphic::UShape {
        let width = self.dim.width;
        let round = self.round;
        if round == 0.0 {
            return URectangle::build(width, height).into();
        }
        if self.changes.len() == 1 {
            return URectangle::build(width, height).rounded(round).into();
        }
        let r = round / 2.0;
        if i == 0 {
            let mut p = UPath::new();
            p.move_to(r, 0.0);
            p.line_to(width - r, 0.0);
            p.arc_to(r, r, 0.0, false, true, width, r);
            p.line_to(width, height);
            p.line_to(0.0, height);
            p.line_to(0.0, r);
            p.arc_to(r, r, 0.0, false, true, r, 0.0);
            p.close_path();
            return p.into();
        }
        if i == self.changes.len() - 1 {
            let mut p = UPath::new();
            p.move_to(0.0, 0.0);
            p.line_to(width, 0.0);
            p.line_to(width, height - r);
            p.arc_to(r, r, 0.0, false, true, width - r, height);
            p.line_to(r, height);
            p.arc_to(r, r, 0.0, false, true, 0.0, height - r);
            p.line_to(0.0, 0.0);
            p.close_path();
            return p.into();
        }
        URectangle::build(width, height).into()
    }

    fn draw(&self, ug: &UGraphic) {
        let mut current = self.default_back.clone();
        let mut y = 0.0;
        for (i, (key, color)) in self.changes.iter().enumerate() {
            if !current.is_transparent() {
                let rect = self.rectangle(i, key - y);
                ug.apply_color(current.clone())
                    .apply_back_color(current.clone())
                    .apply_translate(UTranslate::dy(y))
                    .draw(rect);
            }
            y = *key;
            current = color.clone();
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Group,
    Partition,
}

pub struct GroupingTile {
    args: Rc<TileArguments>,
    kind: Kind,
    start_idx: usize,
    start: Grouping,
    end: Option<usize>,
    tiles: Vec<Rc<dyn Tile>>,
    frame: Rc<Frame>,
    y_gauge: YGauge,
    title: String,
    comment: Option<String>,
    body_height: f64,
}

/// Header of a `partition`: just a centered title (`PartitionTile.getComponent`).
struct PartitionComponent {
    args: Rc<TileArguments>,
    start: Grouping,
}

impl PartitionComponent {
    fn title_block(&self) -> Rc<dyn TextBlock> {
        let styles = self
            .start
            .used_styles(self.start.back_color_general.as_ref());
        let header = styles[1].as_ref().expect("partition header style");
        Display::create(&[self.start.comment.clone().unwrap_or_default()]).create_simple_block(
            &header.font_configuration(None),
            HorizontalAlignment::Left,
            &self.args.skin_param.skin_simple(),
        )
    }

    fn width(&self) -> f64 {
        self.args.border2() - self.args.border1()
    }
}

impl Component for PartitionComponent {
    fn preferred_width(&self, _sb: &dyn StringBounder) -> f64 {
        panic!("unsupported operation")
    }

    fn preferred_height(&self, _sb: &dyn StringBounder) -> f64 {
        panic!("unsupported operation")
    }

    fn preferred_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let d = self.title_block().calculate_dimension(sb);
        Dimension::new(d.width, d.height + 2.0 * TITLE_VPAD)
    }

    /// Draws in both passes, like the anonymous component of `PartitionTile`.
    fn draw(&self, ug: &UGraphic, area: &Area, _background: bool) {
        self.draw_internal(ug, area);
    }

    fn draw_internal(&self, ug: &UGraphic, area: &Area) {
        let sb = ug.string_bounder();
        let styles = self
            .start
            .used_styles(self.start.back_color_general.as_ref());
        let frame = styles[0].as_ref().expect("partition style");
        let title = self.title_block();
        let border1 = self.args.border1();
        let delta = (self.width() - title.calculate_dimension(sb).width) / 2.0;
        title.draw(&ug.apply_translate(UTranslate::new(border1 + delta, TITLE_VPAD)));
        let round = frame.value(PName::RoundCorner).as_double();
        let dim = area.dimension_to_use;
        let rect = URectangle::build(dim.width, dim.height).rounded(round);
        ug.apply_stroke(frame.stroke())
            .apply_color(frame.value(PName::LineColor).as_color())
            .apply_translate(UTranslate::dx(border1))
            .draw(rect);
    }
}

impl GroupingTile {
    /// Reads the events up to the matching `end` (`GroupingTile`).
    pub fn new(
        args: Rc<TileArguments>,
        cursor: &mut usize,
        start_idx: usize,
        current_y: &YGauge,
    ) -> Rc<GroupingTile> {
        let Event::Grouping(start) = &args.model.events[start_idx] else {
            panic!("not a grouping");
        };
        let start = start.clone();
        let kind = if start.grouping_type == GroupingType::StartPartition {
            Kind::Partition
        } else {
            Kind::Group
        };
        let (title, comment) = if start.title == "group" {
            (start.comment.clone().unwrap_or_default(), None)
        } else {
            (start.title.clone(), start.comment.clone())
        };
        let frame = Rc::new(Frame::default());
        let mut tile = GroupingTile {
            args: args.clone(),
            kind,
            start_idx,
            start,
            end: None,
            tiles: Vec::new(),
            frame: frame.clone(),
            y_gauge: YGauge::new(current_y.min().clone(), current_y.max().clone()),
            title,
            comment,
            body_height: 0.0,
        };
        let sb = args.sb();
        let dim1 = tile.component().preferred_dimension(sb);
        let previous_max = current_y.max().clone();
        let parallel = tile.start.parallel;
        let first_y = if parallel {
            current_y.min().clone()
        } else {
            previous_max.clone()
        };
        let h = dim1.height + (MARGIN_Y_MAGIC / 2.0).trunc() + EXTERNAL_MARGIN_Y;
        let mut y = YGauge::create(&first_y.add_at_least(h), 0.0);
        let mut tiles: Vec<Rc<dyn Tile>> = Vec::new();
        while *cursor < args.model.events.len() {
            let ev = *cursor;
            *cursor += 1;
            if let Event::Grouping(g) = &args.model.events[ev] {
                if g.grouping_type == GroupingType::End {
                    tile.end = Some(ev);
                    break;
                }
            }
            for t in build_one(&args, cursor, ev, Some(&frame), &y) {
                y = t.y_gauge();
                tiles.push(t);
            }
        }
        tile.body_height = compute_body_height(&args, &tiles);
        let mut min2: Vec<Real> = Vec::new();
        let mut max2: Vec<Real> = Vec::new();
        let mut all_elses: Vec<Rc<dyn Tile>> = Vec::new();
        for t in &tiles {
            match t.event().map(|e| &args.model.events[e]) {
                Some(Event::Grouping(g)) if g.grouping_type == GroupingType::Else => {
                    all_elses.push(t.clone());
                    continue;
                }
                Some(Event::HSpace(_)) => continue,
                _ => {}
            }
            min2.push(t.drawn_min_x().add_fixed(-MARGIN_X));
            max2.push(t.drawn_max_x().add_fixed(MARGIN_X));
        }
        let width = dim1.width;
        if min2.is_empty() {
            min2.push(args.xorigin.clone());
        }
        let min = real::min(&min2);
        *frame.min.borrow_mut() = Some(min.clone());
        tile.tiles = tiles;
        let notes_left = tile.notes_width(true);
        *frame.min_x.borrow_mut() = Some(min.add_fixed(-EXTERNAL_MARGIN_X1 - notes_left));
        for e in &all_elses {
            max2.push(e.max_x());
        }
        max2.push(min.add_fixed(width + 16.0));
        let max = real::max(&max2);
        *frame.max.borrow_mut() = Some(max);
        let preferred = tile.preferred_height();
        tile.y_gauge = if parallel {
            let parallel_max = first_y.add_at_least(preferred);
            parallel_max.ensure_bigger_than(&previous_max);
            YGauge::new(first_y, parallel_max)
        } else {
            YGauge::new(first_y.clone(), first_y.add_at_least(preferred))
        };
        Rc::new(tile)
    }

    fn component(&self) -> Rc<dyn Component> {
        match self.kind {
            Kind::Group => Rose::create_grouping_header(
                true,
                &self
                    .start
                    .used_styles(self.start.back_color_general.as_ref()),
                &self.title,
                self.comment.as_deref(),
                &self.args.skin_param,
            ),
            Kind::Partition => Rc::new(PartitionComponent {
                args: self.args.clone(),
                start: self.start.clone(),
            }),
        }
    }

    fn frame_y(&self) -> f64 {
        self.y_gauge.min().value() + EXTERNAL_MARGIN_Y
    }

    pub fn frame_min(&self) -> Real {
        self.frame.min()
    }

    pub fn frame_max(&self) -> Real {
        self.frame.max()
    }

    pub fn child_tiles(&self) -> &[Rc<dyn Tile>] {
        &self.tiles
    }

    fn total_height(&self, sb: &dyn StringBounder) -> f64 {
        let dim = self.component().preferred_dimension(sb);
        self.body_height + dim.height + (MARGIN_Y_MAGIC / 2.0).trunc()
    }

    fn area(&self, sb: &dyn StringBounder) -> Area {
        match self.kind {
            Kind::Group => Area::create(
                self.frame.max().value() - self.frame.min().value(),
                self.total_height(sb),
            ),
            Kind::Partition => Area::create(
                self.args.border2() - self.args.border1(),
                self.total_height(sb),
            ),
        }
    }

    fn min_for_drawing(&self) -> f64 {
        match self.kind {
            Kind::Group => self.frame.min().value(),
            Kind::Partition => 0.0,
        }
    }

    fn draw_background(&self, ug: &UGraphic, area: &Area) {
        let styles = self
            .start
            .used_styles(self.start.back_color_general.as_ref());
        let style = styles[0].as_ref().expect("group style");
        let back = style.value(PName::BackGroundColor).as_color();
        let round = style.value(PName::RoundCorner).as_double();
        let ug = ug.apply_translate(UTranslate::dy(self.frame_y()));
        match self.kind {
            Kind::Group => {
                let mut blotter = Blotter::new(area.dimension_to_use, Some(back.clone()), round);
                for tile in &self.tiles {
                    let Some(Event::Grouping(g)) = tile.event().map(|e| &self.args.model.events[e])
                    else {
                        continue;
                    };
                    if g.grouping_type != GroupingType::Else {
                        continue;
                    }
                    let ypos = tile.y_gauge().min().value() - self.frame_y();
                    let back_else = g.back_color_general.clone().or_else(|| Some(back.clone()));
                    blotter.add_change(ypos + 1.0, back_else);
                }
                blotter.close_changes();
                blotter.draw(&ug.apply_translate(UTranslate::dx(self.frame.min().value())));
            }
            Kind::Partition => {
                let dim = area.dimension_to_use;
                let rect = URectangle::build(dim.width, dim.height).rounded(round);
                ug.apply_translate(UTranslate::dx(self.args.border1()))
                    .apply_back_color(back)
                    .draw(rect);
            }
        }
    }

    fn end_notes(&self) -> Vec<Note> {
        match self.end.map(|e| &self.args.model.events[e]) {
            Some(Event::Grouping(g)) => g.notes.clone(),
            _ => Vec::new(),
        }
    }

    /// Width taken on one side by notes attached to the end of the group (`getNotesWidth`).
    fn notes_width(&self, left: bool) -> f64 {
        let _ = (left, self.end_notes());
        0.0
    }
}

fn is_parallel(args: &TileArguments, tile: &Rc<dyn Tile>) -> bool {
    tile.event()
        .is_some_and(|e| args.model.events[e].is_parallel())
}

fn is_life_event(args: &TileArguments, tile: &Rc<dyn Tile>) -> bool {
    matches!(
        tile.event().map(|e| &args.model.events[e]),
        Some(Event::LifeEvent(_))
    )
}

fn flush_pending(pending: &mut Vec<Rc<dyn Tile>>, was_cluster: bool) -> f64 {
    if pending.is_empty() {
        return 0.0;
    }
    let result = if was_cluster {
        let mut contact: f64 = 0.0;
        let mut after: f64 = 0.0;
        for t in pending.iter() {
            contact = contact.max(t.contact_point_relative());
            after = after.max(t.zzz());
        }
        contact + after
    } else {
        pending.iter().map(|t| t.preferred_height()).sum()
    };
    pending.clear();
    result
}

fn compute_body_height(args: &TileArguments, tiles: &[Rc<dyn Tile>]) -> f64 {
    let mut total = 0.0;
    let mut pending: Vec<Rc<dyn Tile>> = Vec::new();
    let mut cluster_open = false;
    for tile in tiles {
        if is_parallel(args, tile) {
            cluster_open = true;
            pending.push(tile.clone());
        } else if is_life_event(args, tile) {
            pending.push(tile.clone());
        } else {
            total += flush_pending(&mut pending, cluster_open);
            cluster_open = false;
            pending.push(tile.clone());
        }
    }
    total + flush_pending(&mut pending, cluster_open)
}

fn touch(touched: &mut Vec<usize>, participant: usize) {
    if !touched.contains(&participant) {
        touched.push(participant);
    }
}

fn touch_note(args: &TileArguments, touched: &mut Vec<usize>, note: &Note) {
    if note.p.is_none() && note.p2.is_none() {
        for p in args.living_spaces.participants() {
            touch(touched, p);
        }
        return;
    }
    for p in [note.p, note.p2].into_iter().flatten() {
        if args.living_spaces.try_get(p).is_some() {
            touch(touched, p);
        }
    }
}

fn collect_touched(args: &TileArguments, tiles: &[Rc<dyn Tile>], touched: &mut Vec<usize>) {
    for tile in tiles {
        match tile.event().map(|e| &args.model.events[e]) {
            Some(Event::Message(m)) => {
                touch(touched, m.p1);
                touch(touched, m.p2);
            }
            Some(Event::MessageExo(m)) => touch(touched, m.participant),
            Some(Event::LifeEvent(l)) => touch(touched, l.participant),
            Some(Event::Note(n)) => touch_note(args, touched, n),
            Some(Event::Notes(notes)) => {
                for n in notes {
                    touch_note(args, touched, n);
                }
            }
            Some(Event::Reference(r)) => {
                for p in &r.participants {
                    if args.living_spaces.try_get(*p).is_some() {
                        touch(touched, *p);
                    }
                }
            }
            _ => {}
        }
        if let Some(g) = tile.as_grouping() {
            collect_touched(args, &g.tiles, touched);
        }
    }
}

fn is_self_message(args: &TileArguments, tile: &Rc<dyn Tile>) -> bool {
    matches!(
        tile.event().map(|e| &args.model.events[e]),
        Some(Event::Message(m)) if m.is_self_message()
    )
}

fn stable_max_x(args: &TileArguments, tile: &Rc<dyn Tile>) -> Vec<Real> {
    let stable = is_self_message(args, tile)
        || matches!(
            tile.event().map(|e| &args.model.events[e]),
            Some(Event::LifeEvent(_) | Event::Reference(_) | Event::Divider { .. })
        );
    if stable {
        vec![tile.drawn_max_x()]
    } else {
        Vec::new()
    }
}

fn stable_min_x(args: &TileArguments, tile: &Rc<dyn Tile>) -> Vec<Real> {
    let stable = is_self_message(args, tile)
        || matches!(
            tile.event().map(|e| &args.model.events[e]),
            Some(Event::LifeEvent(_) | Event::Reference(_))
        );
    if stable {
        vec![tile.drawn_min_x()]
    } else {
        Vec::new()
    }
}

impl GroupingTile {
    fn sb(&self) -> &dyn StringBounder {
        self.args.sb()
    }

    fn ensure_following_participant_clears_frame(&self) {
        let spaces = &self.args.living_spaces;
        let mut touched = Vec::new();
        collect_touched(&self.args, &self.tiles, &mut touched);
        let Some(rightmost) = spaces
            .values()
            .into_iter()
            .rev()
            .find(|s| touched.contains(&s.participant.id))
        else {
            return;
        };
        let Some(next) = spaces.next(&rightmost) else {
            return;
        };
        let next_pos_c = next.pos_c(self.sb());
        let frame_margin = MARGIN_X + EXTERNAL_MARGIN_X2;
        next_pos_c.ensure_bigger_than(&rightmost.pos_c(self.sb()).add_fixed(frame_margin));
        self.ensure_clears_frame_of(self, &next_pos_c, frame_margin);
    }

    fn ensure_preceding_participant_clears_frame(&self) {
        let spaces = &self.args.living_spaces;
        let mut touched = Vec::new();
        collect_touched(&self.args, &self.tiles, &mut touched);
        let Some(leftmost) = spaces
            .values()
            .into_iter()
            .find(|s| touched.contains(&s.participant.id))
        else {
            return;
        };
        let Some(previous) = spaces.previous(&leftmost) else {
            return;
        };
        let previous_pos_c = previous.pos_c(self.sb());
        let frame_margin = MARGIN_X + EXTERNAL_MARGIN_X2;
        leftmost
            .pos_c(self.sb())
            .ensure_bigger_than(&previous_pos_c.add_fixed(frame_margin));
        self.ensure_clears_frame_of_left(self, &previous_pos_c, frame_margin);
    }

    fn ensure_clears_frame_of_left(
        &self,
        group: &GroupingTile,
        previous_pos_c: &Real,
        margin: f64,
    ) {
        for tile in &group.tiles {
            if let Some(nested) = tile.as_grouping() {
                let notes = nested.notes_width(true);
                self.ensure_clears_frame_of_left(
                    nested,
                    previous_pos_c,
                    margin + MARGIN_X + EXTERNAL_MARGIN_X2 + notes,
                );
                continue;
            }
            for min_x in stable_min_x(&self.args, tile) {
                min_x.ensure_bigger_than(&previous_pos_c.add_fixed(margin));
            }
        }
    }

    fn ensure_clears_frame_of(&self, group: &GroupingTile, next_pos_c: &Real, margin: f64) {
        self.ensure_clears_title_of(group, next_pos_c, margin);
        for tile in &group.tiles {
            if let Some(nested) = tile.as_grouping() {
                let notes = nested.notes_width(false);
                self.ensure_clears_frame_of(
                    nested,
                    next_pos_c,
                    margin + MARGIN_X + EXTERNAL_MARGIN_X2 + notes,
                );
                continue;
            }
            for max_x in stable_max_x(&self.args, tile) {
                next_pos_c.ensure_bigger_than(&max_x.add_fixed(margin));
            }
        }
    }

    fn ensure_clears_title_of(&self, group: &GroupingTile, next_pos_c: &Real, margin: f64) {
        let spaces = &self.args.living_spaces;
        let mut touched = Vec::new();
        collect_touched(&self.args, &group.tiles, &mut touched);
        let Some(leftmost) = spaces
            .values()
            .into_iter()
            .find(|s| touched.contains(&s.participant.id))
        else {
            return;
        };
        let overhang = 0.0;
        let width = group.component().preferred_dimension(self.sb()).width;
        next_pos_c.ensure_bigger_than(
            &leftmost
                .pos_c(self.sb())
                .add_fixed(width + 16.0 - MARGIN_X - MARGIN_X - overhang + margin),
        );
    }
}

impl Tile for GroupingTile {
    fn preferred_height(&self) -> f64 {
        let dim = self.component().preferred_dimension(self.sb());
        dim.height + self.body_height + MARGIN_Y_MAGIC + 2.0 * EXTERNAL_MARGIN_Y
    }

    fn on_gauge_resolved(&self) {
        for t in &self.tiles {
            t.on_gauge_resolved();
        }
    }

    fn y_gauge(&self) -> YGauge {
        self.y_gauge.clone()
    }

    fn add_constraints(&self) {
        for t in &self.tiles {
            t.add_constraints();
        }
        self.ensure_following_participant_clears_frame();
        self.ensure_preceding_participant_clears_frame();
    }

    fn min_x(&self) -> Real {
        self.frame.min_x()
    }

    fn max_x(&self) -> Real {
        self.frame
            .max()
            .add_fixed(EXTERNAL_MARGIN_X2 + self.notes_width(false))
    }

    fn event(&self) -> Option<usize> {
        Some(self.start_idx)
    }

    fn contact_point_relative(&self) -> f64 {
        0.0
    }

    fn as_grouping(&self) -> Option<&GroupingTile> {
        Some(self)
    }

    fn draw(&self, ug: &UGraphic) {
        let sb = ug.string_bounder();
        let area = self.area(sb);
        let comp = self.component();
        let background = ug.is_background_layer();
        if background {
            self.draw_background(ug, &area);
        }
        comp.draw(
            &ug.apply_translate(UTranslate::new(self.min_for_drawing(), self.frame_y())),
            &area,
            background,
        );
        for t in &self.tiles {
            t.draw(ug);
        }
    }
}

/// Separator between the branches of a group (`ElseTile`).
pub struct ElseTile {
    args: Rc<TileArguments>,
    event: usize,
    anelse: Grouping,
    parent: Rc<Frame>,
    y_gauge: YGauge,
}

impl ElseTile {
    pub fn new(
        args: Rc<TileArguments>,
        event: usize,
        parent: Rc<Frame>,
        current_y: &YGauge,
    ) -> Rc<ElseTile> {
        let Event::Grouping(g) = &args.model.events[event] else {
            panic!("not an else");
        };
        let mut tile = ElseTile {
            anelse: g.clone(),
            args,
            event,
            parent,
            y_gauge: YGauge::new(current_y.min().clone(), current_y.max().clone()),
        };
        tile.y_gauge = YGauge::create(current_y.max(), tile.preferred_height());
        Rc::new(tile)
    }

    fn component(&self) -> Rc<dyn Component> {
        let styles = self
            .anelse
            .used_styles(self.anelse.back_color_general.as_ref());
        Rose::create_grouping_else(
            true,
            styles[0].as_ref().expect("else style"),
            self.anelse.comment.as_deref(),
            &self.args.skin_param,
        )
    }
}

impl Tile for ElseTile {
    fn preferred_height(&self) -> f64 {
        self.component().preferred_dimension(self.args.sb()).height
    }

    fn y_gauge(&self) -> YGauge {
        self.y_gauge.clone()
    }

    fn add_constraints(&self) {}

    fn min_x(&self) -> Real {
        self.parent.min_x()
    }

    fn max_x(&self) -> Real {
        let dim = self.component().preferred_dimension(self.args.sb());
        self.min_x().add_fixed(dim.width)
    }

    fn event(&self) -> Option<usize> {
        Some(self.event)
    }

    fn contact_point_relative(&self) -> f64 {
        0.0
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = ug.apply_translate(UTranslate::dy(self.y_gauge.min().value()));
        let comp = self.component();
        let dim = comp.preferred_dimension(self.args.sb());
        let min = self.parent.min();
        let max = self.parent.max();
        let area = Area::create(max.value() - min.value(), dim.height);
        let background = ug.is_background_layer();
        comp.draw(
            &ug.apply_translate(UTranslate::dx(min.value())),
            &area,
            background,
        );
    }
}
