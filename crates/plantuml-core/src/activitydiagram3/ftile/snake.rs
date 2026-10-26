//! Arrows between tiles: poly-lines with a head (`Worm`, `WormMutation`, `Snake`, `Arrows`).

use crate::activitydiagram3::rainbow::{HtmlColorAndStyle, Rainbow};
use crate::klimt::drawing::ugraphic::{UGraphic, ULine, UPolygon, UStroke, UTranslate};
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{Direction, HorizontalAlignment, MinMax, VerticalAlignment};
use crate::klimt::shape::TextBlock;
use crate::style::Style;
use std::rc::Rc;

type Pt = (f64, f64);

fn same(a: Pt, b: Pt) -> bool {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt() < 0.001
}

/// `Direction.fromVector`; `None` for a null vector.
fn direction(p1: Pt, p2: Pt) -> Option<Direction> {
    if p1 == p2 {
        return None;
    }
    if p1.0 == p2.0 {
        return Some(if p2.1 > p1.1 {
            Direction::Down
        } else {
            Direction::Up
        });
    }
    if p1.1 == p2.1 {
        return Some(if p2.0 > p1.0 {
            Direction::Right
        } else {
            Direction::Left
        });
    }
    panic!("Not a H or V line!");
}

fn short_code(d: Direction) -> char {
    match d {
        Direction::Right => 'R',
        Direction::Left => 'L',
        Direction::Down => 'D',
        Direction::Up => 'U',
    }
}

/// Arrow heads, regular or triangle (`ArrowsRegular`, `ArrowsTriangle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrows {
    Regular,
    Triangle,
}

impl Arrows {
    pub fn from_skin(skin: &crate::skin::SkinParam) -> Arrows {
        if skin.strict_uml_style() {
            Arrows::Triangle
        } else {
            Arrows::Regular
        }
    }

    pub fn as_to(self, d: Direction) -> UPolygon {
        const D1: f64 = 10.0;
        const D2: f64 = 4.0;
        let regular = self == Arrows::Regular;
        let pts: Vec<Pt> = match d {
            Direction::Up => {
                let mut v = vec![(-D2, D1), (0.0, 0.0), (D2, D1)];
                if regular {
                    v.push((0.0, D1 - 4.0));
                }
                v
            }
            Direction::Down => {
                if regular {
                    vec![(-D2, -D1), (0.0, 0.0), (D2, -D1), (0.0, -D1 + 4.0)]
                } else {
                    vec![(-D2, -D1), (D2, -D1), (0.0, 0.0)]
                }
            }
            Direction::Right => {
                let mut v = vec![(-D1, -D2), (0.0, 0.0), (-D1, D2)];
                if regular {
                    v.push((-D1 + 4.0, 0.0));
                }
                v
            }
            Direction::Left => {
                let mut v = vec![(D1, -D2), (0.0, 0.0), (D1, D2)];
                if regular {
                    v.push((D1 - 4.0, 0.0));
                }
                v
            }
        };
        UPolygon::new(pts)
    }
}

/// How much two snakes may be merged (`MergeStrategy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MergeStrategy {
    Full,
    Limited,
    None,
}

/// A poly-line of horizontal and vertical segments (`Worm`).
#[derive(Debug, Clone)]
pub struct Worm {
    points: Vec<Pt>,
    tr: Option<UTranslate>,
    style: Style,
    arrows: Arrows,
}

impl Worm {
    pub fn new(style: Style, arrows: Arrows) -> Self {
        Self {
            points: Vec::new(),
            tr: None,
            style,
            arrows,
        }
    }

    fn clone_empty(&self) -> Self {
        Self::new(self.style.clone(), self.arrows)
    }

    fn resolve(&self, p: Pt) -> Pt {
        match self.tr {
            None => p,
            Some(t) => (p.0 + t.dx, p.1 + t.dy),
        }
    }

    pub fn size(&self) -> usize {
        self.points.len()
    }

    pub fn point(&self, i: usize) -> Pt {
        self.resolve(self.points[i])
    }

    pub fn first(&self) -> Pt {
        self.point(0)
    }

    pub fn last(&self) -> Pt {
        self.point(self.size() - 1)
    }

    pub fn moved(&self, dx: f64, dy: f64) -> Worm {
        let mut r = self.clone();
        r.tr = Some(match self.tr {
            None => UTranslate::new(dx, dy),
            Some(t) => t.compose(UTranslate::new(dx, dy)),
        });
        r
    }

    pub fn add_point(&mut self, x: f64, y: f64) {
        let (mut x, mut y) = (x, y);
        if let Some(t) = self.tr {
            x -= t.dx;
            y -= t.dy;
        }
        assert!(!x.is_nan() && !y.is_nan(), "NaN point");
        if self.points.last() == Some(&(x, y)) {
            return;
        }
        self.points.push((x, y));
    }

    pub fn is_pure_horizontal(&self) -> bool {
        self.size() == 2 && self.point(0).1 == self.point(1).1
    }

    pub fn min_x(&self) -> f64 {
        self.points
            .iter()
            .map(|p| self.resolve(*p).0)
            .fold(self.point(0).0, f64::min)
    }

    pub fn max_x(&self) -> f64 {
        self.points
            .iter()
            .map(|p| self.resolve(*p).0)
            .fold(self.point(0).0, f64::max)
    }

    pub fn max_y(&self) -> f64 {
        self.points
            .iter()
            .map(|p| self.resolve(*p).1)
            .fold(self.point(0).1, f64::max)
    }

    pub fn directions_code(&self) -> String {
        (0..self.size().saturating_sub(1))
            .map(|i| direction(self.point(i), self.point(i + 1)).map_or('?', short_code))
            .collect()
    }

    pub fn does_horizontal_cross(&self, area: &MinMax) -> bool {
        (0..self.size().saturating_sub(1)).any(|i| {
            let (p1, p2) = (self.point(i), self.point(i + 1));
            p1.1 == p2.1
                && p1.1 >= area.min_y
                && p1.1 <= area.max_y
                && p1.0.min(p2.0) < area.max_x
                && p1.0.max(p2.0) > area.min_x
        })
    }

    pub fn move_first_point(&self, dx: f64, dy: f64) -> Worm {
        assert!(dx == 0.0 || dy == 0.0, "diagonal move");
        let mut r = self.clone_empty();
        let (mut x0, mut y0) = self.point(0);
        let (mut x1, mut y1) = self.point(1);
        if dx != 0.0 && x0 == x1 {
            x1 += dx;
        }
        if dy != 0.0 && y0 == y1 {
            y1 += dy;
        }
        x0 += dx;
        y0 += dy;
        r.add_point(x0, y0);
        r.add_point(x1, y1);
        for i in 2..self.size() {
            let p = self.point(i);
            r.add_point(p.0, p.1);
        }
        r
    }

    pub fn move_last_point(&self, dx: f64, dy: f64) -> Worm {
        assert!(dx == 0.0 || dy == 0.0, "diagonal move");
        let n = self.size();
        let mut r = self.clone_empty();
        let (mut x8, mut y8) = self.point(n - 2);
        let (mut x9, mut y9) = self.point(n - 1);
        if dx != 0.0 && x8 == x9 {
            x8 += dx;
        }
        if dy != 0.0 && y8 == y9 {
            y8 += dy;
        }
        x9 += dx;
        y9 += dy;
        for i in 0..n - 2 {
            let p = self.point(i);
            r.add_point(p.0, p.1);
        }
        r.add_point(x8, y8);
        r.add_point(x9, y9);
        r
    }

    fn draw_line(&self, ug: &UGraphic, p1: Pt, p2: Pt, dir: Option<Direction>) {
        let ug = ug.apply_translate(UTranslate::new(p1.0, p1.1));
        if let Some(d) = dir {
            ug.apply_translate(UTranslate::new((p2.0 - p1.0) / 2.0, (p2.1 - p1.1) / 2.0))
                .draw(self.arrows.as_to(d));
        }
        ug.draw(ULine::new(p2.0 - p1.0, p2.1 - p1.1));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_internal_one_color(
        &self,
        start_decoration: Option<&UPolygon>,
        ug: &UGraphic,
        color: &HtmlColorAndStyle,
        stroke_value: f64,
        emphasize: Option<Direction>,
        end_decoration: Option<&UPolygon>,
    ) {
        if color.style.is_invisible() {
            return;
        }
        let arrow = color.arrow_color.clone();
        let mut ug = ug.apply_color(arrow.clone()).apply_back_color(arrow);
        ug = if color.style.is_normal() {
            ug.apply_stroke(self.style.stroke())
        } else {
            ug.apply_stroke(color.style.go_thickness(stroke_value).stroke())
        };
        let mut drawn = false;
        for i in 0..self.size().saturating_sub(1) {
            let (p1, p2) = (self.point(i), self.point(i + 1));
            if !drawn && emphasize.is_some() && direction(p1, p2) == emphasize {
                self.draw_line(&ug, p1, p2, emphasize);
                drawn = true;
            } else {
                self.draw_line(&ug, p1, p2, None);
            }
        }
        let head = color.arrow_head_color.clone();
        ug = ug.apply_color(head.clone()).apply_back_color(head);
        if let Some(d) = start_decoration {
            let p = self.point(0);
            ug.apply_stroke(UStroke::with_thickness(1.5))
                .apply_translate(UTranslate::new(p.0, p.1))
                .apply_stroke(UStroke::simple())
                .draw(d.clone());
        }
        if let Some(d) = end_decoration {
            let p = self.point(self.size() - 1);
            ug.apply_stroke(UStroke::with_thickness(1.5))
                .apply_translate(UTranslate::new(p.0, p.1))
                .apply_stroke(UStroke::simple())
                .draw(d.clone());
        }
    }

    /// Merges `other`, which starts where this one ends (`merge`).
    pub fn merge(&self, other: &Worm, strategy: MergeStrategy) -> Worm {
        assert!(same(self.last(), other.first()), "not contiguous");
        let mut r = self.clone_empty();
        for i in 0..self.size() {
            let p = self.point(i);
            r.add_point(p.0, p.1);
        }
        for i in 0..other.size() {
            let p = other.point(i);
            r.add_point(p.0, p.1);
        }
        r.merge_me(strategy);
        r
    }

    fn dir_at(&self, i: usize) -> Option<Direction> {
        if i + 1 >= self.size() {
            return None;
        }
        direction(self.point(i), self.point(i + 1))
    }

    fn pattern_at(&self, i: usize) -> [Option<Direction>; 4] {
        [
            self.dir_at(i),
            self.dir_at(i + 1),
            self.dir_at(i + 2),
            self.dir_at(i + 3),
        ]
    }

    fn merge_me(&mut self, strategy: MergeStrategy) {
        loop {
            let mut change = self.remove_null_vector();
            change = change || self.remove_redundant_direction();
            change = change || self.remove_pattern1();
            change = change || self.remove_pattern2();
            change = change || self.remove_pattern3();
            change = change || self.remove_pattern4();
            change = change || self.remove_pattern5();
            change = change || self.remove_pattern6();
            change = change || self.remove_pattern7();
            if strategy == MergeStrategy::Full {
                change = change || self.remove_pattern8();
            }
            if !change {
                break;
            }
        }
    }

    fn remove_null_vector(&mut self) -> bool {
        for i in 0..self.size().saturating_sub(1) {
            if self.dir_at(i).is_none() {
                self.points.remove(i);
                return true;
            }
        }
        false
    }

    fn remove_redundant_direction(&mut self) -> bool {
        for i in 0..self.size().saturating_sub(2) {
            if self.dir_at(i) == self.dir_at(i + 1) {
                self.points.remove(i + 1);
                return true;
            }
        }
        false
    }

    /// Replaces points `i+1..=i+3` by one point.
    fn collapse(&mut self, i: usize, new_point: Pt) {
        self.points.drain(i + 1..=i + 3);
        self.points.insert(i + 1, new_point);
    }

    fn matches(&self, i: usize, p: [Direction; 4]) -> bool {
        self.pattern_at(i) == p.map(Some)
    }

    fn remove_pattern1(&mut self) -> bool {
        use Direction::*;
        for i in 0..self.size().saturating_sub(5) {
            if self.matches(i, [Down, Left, Down, Right])
                || self.matches(i, [Down, Right, Down, Left])
            {
                let np = (self.point(i + 1).0, self.point(i + 3).1);
                self.collapse(i, np);
                return true;
            }
        }
        false
    }

    fn remove_pattern7(&mut self) -> bool {
        use Direction::*;
        if self.size() > 4
            && self.matches(0, [Right, Down, Left, Down])
            && self.point(3).0 > self.point(0).0
        {
            let np = (self.point(3).0, self.point(0).1);
            self.points.drain(1..=2);
            self.points.insert(1, np);
            return true;
        }
        false
    }

    fn remove_pattern2(&mut self) -> bool {
        use Direction::*;
        for i in 0..self.size().saturating_sub(5) {
            if self.matches(i, [Right, Down, Right, Up]) || self.matches(i, [Left, Down, Left, Up])
            {
                let np = (self.point(i + 3).0, self.point(i + 1).1);
                self.collapse(i, np);
                return true;
            }
        }
        false
    }

    fn remove_pattern3(&mut self) -> bool {
        use Direction::*;
        for i in 0..self.size().saturating_sub(4) {
            if self.matches(i, [Down, Right, Down, Right])
                || self.matches(i, [Down, Left, Down, Left])
            {
                let np = (self.point(i + 1).0, self.point(i + 3).1);
                self.collapse(i, np);
                return true;
            }
        }
        false
    }

    fn remove_pattern4(&mut self) -> bool {
        use Direction::*;
        if self.size() >= 5 {
            let i = self.size() - 5;
            if self.matches(i, [Down, Left, Down, Right]) {
                let (p1, p4) = (self.point(i + 1), self.point(i + 4));
                if p4.0 > p1.0 {
                    let np = (p1.0, self.point(i + 3).1);
                    self.collapse(i, np);
                    return true;
                }
            }
        }
        false
    }

    fn remove_pattern5(&mut self) -> bool {
        use Direction::*;
        if self.size() >= 5 {
            let i = self.size() - 5;
            if self.matches(i, [Down, Right, Down, Left]) {
                let (p1, p4) = (self.point(i + 1), self.point(i + 4));
                if p4.0 + 4.0 < p1.0 {
                    let np = (p1.0, self.point(i + 3).1);
                    self.collapse(i, np);
                    return true;
                }
            }
        }
        false
    }

    fn remove_pattern6(&mut self) -> bool {
        for i in 0..self.size().saturating_sub(2) {
            if let (Some(a), Some(b)) = (self.dir_at(i), self.dir_at(i + 1)) {
                if a == b.inv() {
                    self.points.remove(i + 1);
                    return true;
                }
            }
        }
        false
    }

    fn remove_pattern8(&mut self) -> bool {
        use Direction::*;
        for i in 0..self.size().saturating_sub(4) {
            if self.matches(i, [Left, Down, Left, Down])
                || self.matches(i, [Right, Down, Right, Down])
            {
                let np = (self.point(i + 3).0, self.point(i + 1).1);
                self.collapse(i, np);
                return true;
            }
        }
        false
    }
}

/// Offsets for each point of a worm drawn in several colors (`WormMutation`).
pub struct WormMutation {
    translations: Vec<UTranslate>,
}

impl WormMutation {
    fn definition(signature: &str) -> Option<&'static str> {
        Some(match signature {
            "D" | "U" => "33",
            "L" | "R" => "55",
            "RD" => "123",
            "RU" => "543",
            "LD" => "187",
            "DL" => "345",
            "DR" => "765",
            "UL" => "321",
            "UR" => "781",
            _ => return None,
        })
    }

    fn translation(kind: char, d: f64) -> UTranslate {
        match kind {
            '1' => UTranslate::dy(-d),
            '2' => UTranslate::new(d, -d),
            '3' => UTranslate::dx(d),
            '4' => UTranslate::new(d, d),
            '5' => UTranslate::dy(d),
            '6' => UTranslate::new(-d, d),
            '7' => UTranslate::dx(-d),
            '8' => UTranslate::new(-d, -d),
            _ => panic!("bad mutation"),
        }
    }

    fn from_definition(def: &str, delta: f64) -> Self {
        Self {
            translations: def.chars().map(|c| Self::translation(c, delta)).collect(),
        }
    }

    pub fn create(worm: &Worm, delta: f64) -> Self {
        let signature = worm.directions_code();
        match Self::definition(&signature) {
            Some(def) => Self::from_definition(def, delta),
            None => Self::from_long_signature(&signature, delta),
        }
    }

    fn from_long_signature(signature: &str, delta: f64) -> Self {
        let mut result = Self {
            translations: Vec::new(),
        };
        let n = signature.len();
        for i in 0..n.saturating_sub(1) {
            let def = Self::definition(&signature[i..i + 2]).expect("two directions");
            let mut tmp = Self::from_definition(def, delta);
            if i == 0 {
                result.translations.push(tmp.translations[0]);
            } else {
                let last = *result.translations.last().expect("not empty");
                let first = tmp.translations[0];
                if (last.dx - first.dx).abs() > 0.0001 || (last.dy - first.dy).abs() > 0.0001 {
                    tmp = tmp.reverse();
                }
            }
            result.translations.push(tmp.translations[1]);
            if i == n - 2 {
                result.translations.push(tmp.translations[2]);
            }
        }
        result
    }

    fn reverse(&self) -> Self {
        Self {
            translations: self.translations.iter().map(|t| t.reverse()).collect(),
        }
    }

    pub fn first(&self) -> UTranslate {
        self.translations[0]
    }

    pub fn last(&self) -> UTranslate {
        *self.translations.last().expect("not empty")
    }

    pub fn size(&self) -> usize {
        self.translations.len()
    }

    pub fn text_translate(&self, size: usize) -> UTranslate {
        let (mut min, mut max) = (f64::MAX, f64::MIN_POSITIVE);
        for t in &self.translations {
            max = max.max(t.dx);
            min = min.min(t.dx);
        }
        let extreme = if max.abs() > min.abs() { max } else { min };
        UTranslate::dx(extreme * (size as f64 - 1.0))
    }

    pub fn is_dx_negative(&self) -> bool {
        self.translations[0].dx < 0.0
    }

    pub fn mute(&self, original: &Worm) -> Worm {
        let mut r = original.clone_empty();
        for i in 0..original.size() {
            let p = original.point(i);
            let t = self.translations[i];
            r.add_point(p.0 + t.dx, p.1 + t.dy);
        }
        r
    }
}

/// A label next to an arrow.
#[derive(Clone)]
struct SnakeText {
    block: Rc<dyn TextBlock>,
    vertical: Option<VerticalAlignment>,
    horizontal: Option<HorizontalAlignment>,
}

impl SnakeText {
    fn has_text(&self, sb: &dyn StringBounder) -> bool {
        let d = self.block.calculate_dimension(sb);
        !(d.width == 0.0 && d.height == 0.0)
    }
}

/// An arrow: a worm, heads, colors and labels (`Snake`).
#[derive(Clone)]
pub struct Snake {
    worm: Worm,
    start_decoration: Option<UPolygon>,
    end_decoration: Option<UPolygon>,
    color: Rainbow,
    texts: Vec<SnakeText>,
    mergeable: MergeStrategy,
    emphasize_direction: Option<Direction>,
}

impl Snake {
    pub fn create(
        skin: &crate::skin::SkinParam,
        color: Rainbow,
        end_decoration: Option<UPolygon>,
    ) -> Snake {
        Self::create_full(skin, None, color, end_decoration)
    }

    pub fn create_full(
        skin: &crate::skin::SkinParam,
        start_decoration: Option<UPolygon>,
        color: Rainbow,
        end_decoration: Option<UPolygon>,
    ) -> Snake {
        assert!(color.size() > 0, "empty rainbow");
        let style = skin
            .style_builder()
            .merged_style(&crate::style::queries::ACTIVITYDIAG_ACTIVITY_ARROW)
            .expect("activity arrow style");
        Snake {
            worm: Worm::new(style, Arrows::from_skin(skin)),
            start_decoration,
            end_decoration,
            color,
            texts: Vec::new(),
            mergeable: MergeStrategy::Full,
            emphasize_direction: None,
        }
    }

    pub fn moved(&self, dx: f64, dy: f64) -> Snake {
        Snake {
            worm: self.worm.moved(dx, dy),
            ..self.clone()
        }
    }

    pub fn emphasize_direction(mut self, d: Direction) -> Snake {
        self.emphasize_direction = Some(d);
        self
    }

    pub fn without_end_decoration(mut self) -> Snake {
        self.end_decoration = None;
        self
    }

    pub fn with_merge(mut self, m: MergeStrategy) -> Snake {
        self.mergeable = m;
        self
    }

    pub fn with_label(mut self, block: Option<Rc<dyn TextBlock>>, h: HorizontalAlignment) -> Snake {
        if let Some(block) = block {
            self.texts.push(SnakeText {
                block,
                vertical: None,
                horizontal: Some(h),
            });
        }
        self
    }

    pub fn with_vertical_label(
        mut self,
        block: Option<Rc<dyn TextBlock>>,
        v: VerticalAlignment,
    ) -> Snake {
        if let Some(block) = block {
            self.texts.push(SnakeText {
                block,
                vertical: Some(v),
                horizontal: None,
            });
        }
        self
    }

    pub fn add_point(&mut self, x: f64, y: f64) {
        self.worm.add_point(x, y);
    }

    pub fn first(&self) -> Pt {
        self.worm.first()
    }

    pub fn last(&self) -> Pt {
        self.worm.last()
    }

    pub fn worm(&self) -> &Worm {
        &self.worm
    }

    pub fn cannot_be_touched(&self) -> bool {
        self.mergeable != MergeStrategy::Full || self.worm.is_pure_horizontal()
    }

    pub fn does_horizontal_cross(&self, area: &MinMax) -> bool {
        self.worm.does_horizontal_cross(area)
    }

    pub fn draw_internal(&self, ug: &UGraphic) {
        if self.color.size() > 1 {
            self.draw_rainbow(ug);
        } else {
            self.worm.draw_internal_one_color(
                self.start_decoration.as_ref(),
                ug,
                &self.color.colors()[0],
                1.5,
                self.emphasize_direction,
                self.end_decoration.as_ref(),
            );
            self.draw_internal_label(ug);
        }
    }

    fn draw_rainbow(&self, ug: &UGraphic) {
        let mut colors: Vec<HtmlColorAndStyle> = self.color.colors().to_vec();
        let space = self.color.color_arrow_separation_space();
        let mv = 2.0 + f64::from(space);
        let mutation = WormMutation::create(&self.worm, mv);
        if mutation.is_dx_negative() {
            colors.reverse();
        }
        let global = -(colors.len() as f64 - 1.0) / 2.0;
        let f = mutation.first().scaled(global);
        let mut current = self.worm.move_first_point(f.dx, f.dy);
        if mutation.size() > 2 {
            let l = mutation.last().scaled(global);
            current = current.move_last_point(l.dx, l.dy);
        }
        for (i, c) in colors.iter().enumerate() {
            let mut stroke = 1.5;
            if space == 0 {
                stroke = if i == colors.len() - 1 { 2.0 } else { 3.0 };
            }
            current.draw_internal_one_color(
                self.start_decoration.as_ref(),
                ug,
                c,
                stroke,
                self.emphasize_direction,
                self.end_decoration.as_ref(),
            );
            current = mutation.mute(&current);
        }
        self.draw_internal_label(&ug.apply_translate(mutation.text_translate(colors.len())));
    }

    fn draw_internal_label(&self, ug: &UGraphic) {
        for text in &self.texts {
            if text.has_text(ug.string_bounder()) {
                let p = self.text_position(ug.string_bounder(), text);
                text.block
                    .draw(&ug.apply_translate(UTranslate::new(p.0, p.1)));
            }
        }
    }

    pub fn max_x(&self, sb: &dyn StringBounder) -> f64 {
        let mut result = self.worm.max_x();
        for text in &self.texts {
            let p = self.text_position(sb, text);
            let dim = text.block.calculate_dimension(sb);
            result = result.max(p.0 + dim.width);
        }
        result
    }

    fn text_position(&self, sb: &dyn StringBounder, text: &SnakeText) -> Pt {
        let pt1 = self.worm.point(0);
        let pt2 = self.worm.point(1);
        let dim = text.block.calculate_dimension(sb);
        let mut x = pt1.0.max(pt2.0) + 4.0;
        let code = self.worm.directions_code();
        let zigzag = code.starts_with("DLD") || code.starts_with("DRD");
        let mut y = (pt1.1 + pt2.1) / 2.0 - dim.height / 2.0;
        if text.vertical == Some(VerticalAlignment::Bottom) {
            x = self.worm.min_x();
            y = self.worm.max_y();
        } else if text.vertical == Some(VerticalAlignment::Center) {
            x = self.worm.min_x();
            y = (self.worm.first().1 + self.worm.last().1 - 10.0) / 2.0 - dim.height / 2.0;
        } else if text.horizontal == Some(HorizontalAlignment::Center) && zigzag {
            let pt3 = self.worm.point(2);
            x = (pt2.0 + pt3.0) / 2.0 - dim.width / 2.0;
        } else if text.horizontal == Some(HorizontalAlignment::Right) && zigzag {
            x = pt1.0.max(pt2.0) - dim.width - 4.0;
        } else if code == "RD" {
            x = pt1.0.max(pt2.0);
            y = (pt1.1 + self.worm.point(2).1) / 2.0 - dim.height / 2.0;
        } else if code == "LD" {
            x = pt1.0.min(pt2.0);
            y = (pt1.1 + self.worm.point(2).1) / 2.0 - dim.height / 2.0;
        }
        (x, y)
    }

    /// `merge`: joins `other` when it starts where this one ends.
    pub fn merge(&self, other: &Snake, sb: &dyn StringBounder) -> Option<Snake> {
        let strategy = self.mergeable.max(other.mergeable);
        if strategy == MergeStrategy::None {
            return None;
        }
        if other.texts.iter().any(|t| t.has_text(sb)) {
            return None;
        }
        if same(self.last(), other.first()) {
            let one_of = other
                .end_decoration
                .clone()
                .or_else(|| self.end_decoration.clone());
            assert!(
                self.start_decoration.is_none() && other.start_decoration.is_none(),
                "Not yet coded: to be done"
            );
            let mut texts = self.texts.clone();
            texts.extend(other.texts.iter().cloned());
            return Some(Snake {
                worm: self.worm.merge(&other.worm, strategy),
                start_decoration: None,
                end_decoration: one_of,
                color: self.color.clone(),
                texts,
                mergeable: strategy,
                emphasize_direction: self.emphasize_direction.or(other.emphasize_direction),
            });
        }
        if same(self.first(), other.last()) {
            return other.merge(self, sb);
        }
        None
    }

    pub fn touches(&self, other: &Snake) -> bool {
        if other.mergeable != MergeStrategy::Full || other.worm.is_pure_horizontal() {
            return false;
        }
        same(self.last(), other.first())
    }
}

pub(super) fn points_same(a: Pt, b: Pt) -> bool {
    same(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::klimt::color::HColor;
    use crate::skin::SkinParam;

    fn red() -> Rainbow {
        Rainbow::from_color(HColor::parse("red"), None)
    }

    fn snake(pts: &[Pt]) -> Snake {
        let skin = SkinParam::new();
        let mut s = Snake::create(&skin, red(), Some(Arrows::Regular.as_to(Direction::Down)));
        for p in pts {
            s.add_point(p.0, p.1);
        }
        s
    }

    #[test]
    fn worm_directions_and_merge() {
        let a = snake(&[(0.0, 0.0), (0.0, 10.0)]);
        let b = snake(&[(0.0, 10.0), (0.0, 30.0)]);
        assert_eq!(a.worm().directions_code(), "D");
        let sb = crate::klimt::font::EmbeddedFontBounder;
        let m = a.merge(&b, &sb).expect("contiguous");
        assert_eq!(m.worm().size(), 2);
        assert_eq!(m.last(), (0.0, 30.0));
        // reversed order merges as well
        assert!(b.merge(&a, &sb).is_some());
        let far = snake(&[(5.0, 5.0), (5.0, 8.0)]);
        assert!(a.merge(&far, &sb).is_none());
        assert!(a.touches(&b));
        assert!(!a.touches(&far));
        assert!(a
            .clone()
            .with_merge(MergeStrategy::None)
            .merge(&b, &sb)
            .is_none());
    }

    #[test]
    fn worm_patterns_simplify() {
        let s = snake(&[
            (0.0, 0.0),
            (0.0, 10.0),
            (5.0, 10.0),
            (5.0, 20.0),
            (10.0, 20.0),
            (10.0, 30.0),
        ]);
        let t = snake(&[(10.0, 30.0), (10.0, 40.0)]);
        let sb = crate::klimt::font::EmbeddedFontBounder;
        let m = s.merge(&t, &sb).unwrap();
        assert!(m.worm().size() < 7);
        assert!(m.worm().directions_code().starts_with('D'));
        assert_eq!(m.first(), (0.0, 0.0));
    }

    #[test]
    fn moves_and_geometry() {
        let s = snake(&[(0.0, 0.0), (0.0, 10.0), (5.0, 10.0)]);
        let m = s.moved(2.0, 3.0);
        assert_eq!(m.first(), (2.0, 3.0));
        assert_eq!(m.worm().max_x(), 7.0);
        assert_eq!(m.worm().max_y(), 13.0);
        assert_eq!(m.worm().min_x(), 2.0);
        assert_eq!(s.worm().directions_code(), "DR");
        let f = s.worm().move_first_point(1.0, 0.0);
        assert_eq!(f.first(), (1.0, 0.0));
        let l = s.worm().move_last_point(0.0, 1.0);
        assert_eq!(l.last(), (5.0, 11.0));
        assert!(s
            .worm()
            .does_horizontal_cross(&MinMax::empty(true).add_point(10.0, 20.0)));
        assert!(!s.cannot_be_touched());
        assert!(snake(&[(0.0, 0.0), (4.0, 0.0)]).cannot_be_touched());
    }

    #[test]
    fn mutation_for_rainbow() {
        let s = snake(&[(0.0, 0.0), (0.0, 10.0), (5.0, 10.0), (5.0, 20.0)]);
        let m = WormMutation::create(s.worm(), 3.0);
        assert_eq!(m.size(), 4);
        let muted = m.mute(s.worm());
        assert_eq!(muted.size(), 4);
        assert!(m.text_translate(2).dx.abs() >= 0.0);
        assert!(!m.is_dx_negative() || m.first().dx < 0.0);
        assert_eq!(m.last(), m.last());
    }

    #[test]
    fn arrow_heads() {
        assert_eq!(Arrows::Regular.as_to(Direction::Down).points.len(), 4);
        assert_eq!(Arrows::Triangle.as_to(Direction::Down).points.len(), 3);
        for d in [Direction::Up, Direction::Left, Direction::Right] {
            assert_eq!(Arrows::Regular.as_to(d).points.len(), 4);
            assert_eq!(Arrows::Triangle.as_to(d).points.len(), 3);
        }
    }
}
