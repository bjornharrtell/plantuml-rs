//! The vertical flow of tiles and the final drawing (`PlayingSpace`, `TileBuilder`,
//! `PlayingSpaceWithParticipants`, `SequenceDiagramFileMakerTeoz`).

use super::communication::CommunicationTile;
use super::dolls::Dolls;
use super::grouping::{ElseTile, Frame, GroupingTile};
use super::lifeevent::LifeEventTile;
use super::living::{LivingSpace, LivingSpaces};
use super::notes::{NoteOnMessageTile, NoteTile, NotesTile, Side};
use super::reference::ReferenceTile;
use super::self_tile::CommunicationTileSelf;
use super::simple::{DelayTile, DividerTile, HSpaceTile, NewpageTile};
use super::{Bordered, Model, Tile, TileArguments, YGauge};
use crate::klimt::drawing::ugraphic::{find_limits, UGraphic, UTranslate};
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::VerticalAlignment;
use crate::klimt::shape::TextBlock;
use crate::real::{self, Real};
use crate::sequencediagram::model::{Event, GroupingType, NotePosition};
use crate::sequencediagram::SequenceDiagram;
use crate::skin::SkinParam;
use std::rc::Rc;

/// Builds the tile of one event (`TileBuilder.buildOne`); unsupported events are skipped.
pub fn build_one(
    args: &Rc<TileArguments>,
    cursor: &mut usize,
    event: usize,
    parent: Option<&Rc<Frame>>,
    current_y: &YGauge,
) -> Vec<Rc<dyn Tile>> {
    match &args.model.events[event] {
        Event::Message(m) => {
            let (mut result, reverse): (Rc<dyn Tile>, bool) = if m.is_self_message() {
                (
                    CommunicationTileSelf::new(args.clone(), event, current_y),
                    false,
                )
            } else {
                let tile = CommunicationTile::new(args.clone(), event, current_y);
                let reverse = tile.is_reverse();
                (tile, reverse)
            };
            let ls1 = args.living_spaces.get(m.p1);
            let ls2 = args.living_spaces.get(m.p2);
            for note in &m.core.notes {
                let (side, ls) = match (note.position, m.is_self_message()) {
                    (NotePosition::Left, true) => (Side::SelfLeft, ls1.clone()),
                    (NotePosition::Left, false) => {
                        (Side::Left, if reverse { ls2.clone() } else { ls1.clone() })
                    }
                    (NotePosition::Right, true) => (Side::SelfRight, ls1.clone()),
                    (NotePosition::Right, false) => {
                        (Side::Right, if reverse { ls1.clone() } else { ls2.clone() })
                    }
                    (NotePosition::Bottom, _) => (Side::Bottom, ls1.clone()),
                    (NotePosition::Top, _) => (Side::Top, ls1.clone()),
                    _ => continue,
                };
                result = NoteOnMessageTile::new(
                    args.clone(),
                    side,
                    result,
                    event,
                    note.clone(),
                    ls,
                    m.core.is_create(),
                );
            }
            vec![result]
        }
        Event::Reference(_) => {
            vec![ReferenceTile::new(args.clone(), event, current_y) as Rc<dyn Tile>]
        }
        Event::Note(_) => vec![NoteTile::new(args.clone(), event, current_y) as Rc<dyn Tile>],
        Event::Notes(_) => vec![NotesTile::new(args.clone(), event, current_y) as Rc<dyn Tile>],
        Event::LifeEvent(_) => {
            vec![LifeEventTile::new(args.clone(), event, current_y) as Rc<dyn Tile>]
        }
        Event::Divider { .. } => {
            vec![DividerTile::new(args.clone(), event, current_y) as Rc<dyn Tile>]
        }
        Event::Delay { .. } => vec![DelayTile::new(args.clone(), event, current_y) as Rc<dyn Tile>],
        Event::HSpace(_) => vec![HSpaceTile::new(args.clone(), event, current_y) as Rc<dyn Tile>],
        Event::Newpage { .. } => {
            vec![NewpageTile::new(args.clone(), event, current_y) as Rc<dyn Tile>]
        }
        Event::Grouping(g) if g.grouping_type.is_start() => {
            vec![GroupingTile::new(args.clone(), cursor, event, current_y) as Rc<dyn Tile>]
        }
        Event::Grouping(g) if g.grouping_type == GroupingType::Else => {
            let parent = parent.expect("else outside of a group");
            vec![ElseTile::new(args.clone(), event, parent.clone(), current_y) as Rc<dyn Tile>]
        }
        other => {
            log::warn!("TileBuilder: ignoring {:?}", std::mem::discriminant(other));
            Vec::new()
        }
    }
}

pub fn build_several(args: &Rc<TileArguments>, mut current_y: YGauge) -> Vec<Rc<dyn Tile>> {
    let mut tiles: Vec<Rc<dyn Tile>> = Vec::new();
    let mut cursor = 0;
    while cursor < args.model.events.len() {
        let event = cursor;
        cursor += 1;
        for tile in build_one(args, &mut cursor, event, None, &current_y) {
            current_y = tile.y_gauge();
            tiles.push(tile);
        }
    }
    tiles
}

const STARTING_Y: f64 = 8.0;

/// All tiles of a page and the extent they need (`PlayingSpace`).
pub struct PlayingSpace {
    min: Real,
    max: Real,
    show_footbox: bool,
    tiles: Vec<Rc<dyn Tile>>,
    living_spaces: Rc<LivingSpaces>,
    args: Rc<TileArguments>,
}

impl PlayingSpace {
    pub fn new(args: &Rc<TileArguments>, dolls: &Dolls) -> PlayingSpace {
        let mut mins = vec![args.xorigin.clone()];
        let mut maxs = vec![args.xorigin.clone()];
        if dolls.size() > 0 {
            mins.push(dolls.min_x());
            maxs.push(dolls.max_x());
        }
        let ycurrent = YGauge::create(&args.yorigin.add_fixed(STARTING_Y), 0.0);
        let tiles = build_several(args, ycurrent);
        for t in &tiles {
            mins.push(t.min_x());
            maxs.push(t.max_x());
        }
        for ls in args.living_spaces.values() {
            maxs.push(ls.pos_d(args.sb()));
            maxs.push(ls.pos_c2(args.sb()));
        }
        PlayingSpace {
            min: real::min(&mins),
            max: real::max(&maxs),
            show_footbox: args.model.show_footbox,
            tiles,
            living_spaces: args.living_spaces.clone(),
            args: args.clone(),
        }
    }

    pub fn add_constraints(&self) {
        for t in &self.tiles {
            t.add_constraints();
        }
        self.add_parallel_sibling_disjoint_constraints();
    }

    fn add_parallel_sibling_disjoint_constraints(&self) {
        let mut cluster: Vec<Rc<dyn Tile>> = Vec::new();
        for tile in &self.tiles {
            if self.is_parallel(tile) {
                for other in &cluster {
                    self.ensure_disjoint(other, tile);
                }
                cluster.push(tile.clone());
            } else {
                cluster.clear();
                cluster.push(tile.clone());
            }
        }
    }

    fn is_parallel(&self, tile: &Rc<dyn Tile>) -> bool {
        tile.event()
            .is_some_and(|e| self.args.model.events[e].is_parallel())
    }

    fn ensure_disjoint(&self, a: &Rc<dyn Tile>, b: &Rc<dyn Tile>) {
        let (Some(a_anchor), Some(b_anchor)) = (
            self.find_anchor_living_space(a),
            self.find_anchor_living_space(b),
        ) else {
            return;
        };
        if Rc::ptr_eq(&a_anchor, &b_anchor) {
            return;
        }
        if a_anchor.pos_b().value() <= b_anchor.pos_b().value() {
            b.min_x().ensure_bigger_than(&a.max_x());
        } else {
            a.min_x().ensure_bigger_than(&b.max_x());
        }
    }

    fn find_anchor_living_space(&self, tile: &Rc<dyn Tile>) -> Option<Rc<LivingSpace>> {
        let spaces = &self.args.living_spaces;
        match tile.event().map(|e| &self.args.model.events[e]) {
            Some(Event::Message(m)) => return spaces.try_get(m.p1),
            Some(Event::MessageExo(m)) => return spaces.try_get(m.participant),
            Some(Event::Note(n)) => return n.p.and_then(|p| spaces.try_get(p)),
            Some(Event::Notes(ns)) => {
                if let Some(n) = ns.first() {
                    return n.p.and_then(|p| spaces.try_get(p));
                }
            }
            _ => {}
        }
        if let Some(g) = tile.as_grouping() {
            for child in g.child_tiles() {
                if let Some(found) = self.find_anchor_living_space(child) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn draw_internal(&self, ug: &UGraphic) -> f64 {
        for tile in &self.tiles {
            tile.draw(ug);
        }
        match self.tiles.last() {
            None => STARTING_Y,
            Some(t) => t.y_gauge().max().value(),
        }
    }

    pub fn draw_background(&self, ug: &UGraphic) {
        self.draw_internal(&ug.apply_background_layer(true));
    }

    pub fn draw_foreground(&self, ug: &UGraphic) {
        self.draw_internal(&ug.apply_background_layer(false));
    }

    pub fn preferred_height(&self, sb: Rc<dyn StringBounder>) -> f64 {
        let mut final_y = 0.0;
        let m = find_limits(sb, true, |ug| {
            final_y = self.draw_internal(&ug.apply_background_layer(false));
        });
        m.height().max(final_y) + 10.0
    }

    pub fn tiles(&self) -> &[Rc<dyn Tile>] {
        &self.tiles
    }

    pub fn min_x(&self) -> Real {
        self.min.clone()
    }

    pub fn max_x(&self) -> Real {
        self.max.clone()
    }

    pub fn is_show_footbox(&self) -> bool {
        self.show_footbox
    }

    pub fn living_spaces(&self) -> &Rc<LivingSpaces> {
        &self.living_spaces
    }

    pub fn on_gauge_resolved(&self) {
        for t in &self.tiles {
            t.on_gauge_resolved();
        }
    }
}

impl Bordered for PlayingSpace {
    fn border1(&self) -> f64 {
        self.min.value()
    }

    fn border2(&self) -> f64 {
        self.max.value()
    }
}

/// Heads, lifelines and tiles of one page (`PlayingSpaceWithParticipants`).
pub struct PlayingSpaceWithParticipants {
    playing_space: Rc<PlayingSpace>,
    string_bounder: Rc<dyn StringBounder>,
}

impl PlayingSpaceWithParticipants {
    pub fn new(playing_space: Rc<PlayingSpace>, string_bounder: Rc<dyn StringBounder>) -> Self {
        Self {
            playing_space,
            string_bounder,
        }
    }

    fn page_height(&self, full_height: f64) -> f64 {
        full_height
    }

    pub fn min_x(&self) -> Real {
        self.playing_space.min_x()
    }
}

impl TextBlock for PlayingSpaceWithParticipants {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let ps = &self.playing_space;
        let width = ps.max_x().value() - ps.min_x().value();
        let full_height = ps.preferred_height(self.string_bounder.clone());
        let page_height = self.page_height(full_height);
        let factor = if ps.is_show_footbox() { 2.0 } else { 1.0 };
        let height = page_height + factor * ps.living_spaces().head_height(sb);
        Dimension::new(width, height)
    }

    fn draw(&self, ug: &UGraphic) {
        let ps = &self.playing_space;
        let sb = ug.string_bounder();
        let living = ps.living_spaces();
        let head_height = living.head_height(sb);
        let full_height = ps.preferred_height(self.string_bounder.clone());
        let page_height = self.page_height(full_height);
        let ug_body = ug.apply_translate(UTranslate::dy(head_height));
        ps.draw_background(&ug_body);
        living.draw_life_lines(&ug_body, false, full_height);
        living.draw_heads(ug, false, VerticalAlignment::Bottom);
        if ps.is_show_footbox() {
            living.draw_heads(
                &ug.apply_translate(UTranslate::dy(page_height + head_height)),
                false,
                VerticalAlignment::Top,
            );
        }
        ps.draw_foreground(&ug_body);
    }
}

/// Lays the diagram out and returns a block drawing it (`SequenceDiagramFileMakerTeoz`).
pub struct SequenceBlock {
    body: PlayingSpaceWithParticipants,
    min1: Real,
    dolls: Dolls,
    height_englober1: f64,
    height_englober2: f64,
    string_bounder: Rc<dyn StringBounder>,
}

impl SequenceBlock {
    pub fn new(diagram: &SequenceDiagram, sb: Rc<dyn StringBounder>, skin: Rc<SkinParam>) -> Self {
        let model = Rc::new(Model {
            participants: diagram
                .participants()
                .iter()
                .cloned()
                .map(Rc::new)
                .collect(),
            events: diagram.events().to_vec(),
            show_footbox: diagram.is_show_footbox(),
            englobers: Vec::new(),
            link_anchors: diagram.link_anchors().to_vec(),
        });
        let xorigin = real::create_origin();
        let yorigin = real::create_origin();
        let living_spaces = Rc::new(LivingSpaces::new());
        let mut xcurrent = xorigin.add_at_least(0.0);
        for p in &model.participants {
            let ls = Rc::new(LivingSpace::new(
                p.clone(),
                diagram.englober(p.id),
                skin.clone(),
                xcurrent.clone(),
                model.clone(),
            ));
            living_spaces.put(p.id, ls.clone());
            xcurrent = ls.pos_d(sb.as_ref()).add_at_least(0.0);
        }
        let args = Rc::new(TileArguments::new(
            sb.clone(),
            living_spaces.clone(),
            skin,
            xorigin.clone(),
            yorigin.clone(),
            model,
        ));
        let dolls = Dolls::new(&args);
        dolls.add_constraints();
        let main = Rc::new(PlayingSpace::new(&args, &dolls));
        living_spaces.add_constraints(sb.as_ref());
        main.add_constraints();
        xorigin.compile_now();
        yorigin.compile_now();
        args.set_bordered(main.clone());
        main.on_gauge_resolved();
        let body = PlayingSpaceWithParticipants::new(main, sb.clone());
        let min1 = body.min_x();
        let height_englober1 = dolls.offset_for_englobers();
        let height_englober2 = if height_englober1 == 0.0 { 0.0 } else { 10.0 };
        SequenceBlock {
            body,
            min1,
            dolls,
            height_englober1,
            height_englober2,
            string_bounder: sb,
        }
    }
}

impl TextBlock for SequenceBlock {
    fn calculate_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        let d = self.body.calculate_dimension(sb);
        Dimension::new(
            d.width + 10.0,
            d.height + self.height_englober1 + self.height_englober2 + 10.0,
        )
    }

    fn draw(&self, ug: &UGraphic) {
        let ug = ug.apply_translate(UTranslate::new(5.0, 5.0));
        let ug = ug.apply_translate(UTranslate::dx(-self.min1.value()));
        let body_height = self
            .body
            .calculate_dimension(self.string_bounder.as_ref())
            .height;
        self.dolls.draw_englobers(
            &ug,
            body_height + self.height_englober1 + self.height_englober2 / 2.0,
        );
        let ug = ug.apply_translate(UTranslate::dy(self.height_englober1));
        self.body.draw(&ug);
    }
}
