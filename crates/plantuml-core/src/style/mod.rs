//! Style sheets (`<style>` blocks and `.skin` files), ported from PlantUML's `style` package.

pub mod names;
pub mod parser;
pub mod queries;
pub mod skinparam;
pub mod value;

use crate::klimt::drawing::ugraphic::UStroke;
use crate::klimt::font::UFont;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

pub use names::{PName, SName};
pub use value::{Specificity, Value};

/// One element of a style signature: an `SName` or a stereotype (`StyleAtom`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StyleAtom {
    Name(SName),
    Stereotype(String),
}

impl StyleAtom {
    /// Stereotypes are lowercased with `_` and `.` removed.
    pub fn stereotype(s: &str) -> Self {
        StyleAtom::Stereotype(
            s.chars()
                .filter(|c| *c != '_' && *c != '.')
                .flat_map(char::to_lowercase)
                .collect(),
        )
    }
}

impl PartialOrd for StyleAtom {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StyleAtom {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (StyleAtom::Name(a), StyleAtom::Name(b)) => a.cmp(b),
            (StyleAtom::Name(_), _) => Ordering::Less,
            (_, StyleAtom::Name(_)) => Ordering::Greater,
            (StyleAtom::Stereotype(a), StyleAtom::Stereotype(b)) => a.cmp(b),
        }
    }
}

/// Sorted, deduplicated set of atoms (`AtomArray`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct AtomArray(Vec<StyleAtom>);

impl AtomArray {
    pub fn plus(&self, atom: StyleAtom) -> AtomArray {
        match self.0.binary_search(&atom) {
            Ok(_) => self.clone(),
            Err(i) => {
                let mut v = self.0.clone();
                v.insert(i, atom);
                AtomArray(v)
            }
        }
    }
    pub fn plus_all(&self, other: &AtomArray) -> AtomArray {
        let mut v = self.0.clone();
        v.extend(other.0.iter().cloned());
        v.sort();
        v.dedup();
        AtomArray(v)
    }
    pub fn has_stereotype(&self) -> bool {
        matches!(self.0.last(), Some(StyleAtom::Stereotype(_)))
    }
    pub fn iter(&self) -> impl Iterator<Item = &StyleAtom> {
        self.0.iter()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Depth restriction and `*` flag of a signature (`LevelConstraint`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LevelConstraint {
    level: i32,
    star: bool,
}

impl LevelConstraint {
    pub const NO_LEVEL: i32 = -1;
    pub const NONE: LevelConstraint = LevelConstraint {
        level: -1,
        star: false,
    };

    pub fn of(level: i32, star: bool) -> Self {
        assert!(
            level == Self::NO_LEVEL || (0..64).contains(&level),
            "depth level out of range (must be 0..63): {level}"
        );
        Self { level, star }
    }
    pub fn level(&self) -> i32 {
        self.level
    }
    pub fn has_level(&self) -> bool {
        self.level != Self::NO_LEVEL
    }
    pub fn is_star(&self) -> bool {
        self.star
    }
    fn accepted_levels_mask(&self) -> i64 {
        if !self.has_level() {
            -1
        } else if self.star {
            -1i64 << self.level
        } else {
            1i64 << self.level
        }
    }
    pub fn matches(declaration: LevelConstraint, query: LevelConstraint) -> bool {
        if declaration.has_level() {
            if !query.has_level() {
                return false;
            }
            if declaration.accepted_levels_mask() & (1i64 << query.level) == 0 {
                return false;
            }
        }
        !(query.star && !declaration.star)
    }
}

/// A style signature (`StyleQuery`, formerly `StyleSignature`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StyleQuery {
    atoms: AtomArray,
    level: LevelConstraint,
}

impl StyleQuery {
    pub fn of(names: &[SName]) -> Self {
        let mut atoms = AtomArray::default();
        for n in names {
            atoms = atoms.plus(StyleAtom::Name(*n));
        }
        Self {
            atoms,
            level: LevelConstraint::NONE,
        }
    }
    pub fn empty() -> Self {
        Self::of(&[])
    }
    pub fn with_stereotype(&self, s: &str) -> Self {
        Self {
            atoms: self.atoms.plus(StyleAtom::stereotype(s)),
            level: self.level,
        }
    }
    /// Adds `.name` atoms for each label of the stereotype (`withStereotype(Stereotype)`).
    pub fn with_stereotype_obj(&self, stereotype: Option<&crate::stereo::Stereotype>) -> Self {
        let Some(st) = stereotype else {
            return self.clone();
        };
        if st.style_names().is_empty() {
            return self.clone();
        }
        let labels = st.labels(crate::stereo::Guillemet::None);
        labels
            .iter()
            .fold(self.clone(), |q, l| q.with_stereotype(l))
    }

    /// Query for the `stereotype` element of a stereotyped entity (`forStereotypeItself`).
    pub fn for_stereotype_itself(&self, stereotype: Option<&crate::stereo::Stereotype>) -> Self {
        let Some(st) = stereotype else {
            return self.clone();
        };
        if st.style_names().is_empty() {
            return self.clone();
        }
        let labels = st.labels(crate::stereo::Guillemet::None);
        if labels.is_empty() {
            return self.clone();
        }
        let base = Self {
            atoms: self.atoms.plus(StyleAtom::Name(SName::Stereotype)),
            level: self.level,
        };
        labels.iter().fold(base, |q, l| q.with_stereotype(l))
    }

    /// Adds the `clickable` element when the element has a link (`addClickable`).
    pub fn add_clickable(&self, has_url: bool) -> Self {
        if has_url {
            self.add(SName::Clickable)
        } else {
            self.clone()
        }
    }

    /// Merges the signatures of other styles (`mergeWith(List<Style>)`).
    pub fn merge_with_styles(&self, others: &[Style]) -> Self {
        others
            .iter()
            .fold(self.clone(), |q, s| q.merge_with(s.query()))
    }

    pub fn add_level(&self, level: i32) -> Self {
        Self {
            atoms: self.atoms.clone(),
            level: LevelConstraint::of(level, self.level.star),
        }
    }
    pub fn add_star(&self) -> Self {
        Self {
            atoms: self.atoms.clone(),
            level: LevelConstraint::of(self.level.level, true),
        }
    }
    pub fn add(&self, name: SName) -> Self {
        Self {
            atoms: self.atoms.plus(StyleAtom::Name(name)),
            level: self.level,
        }
    }
    pub fn add_all(&self, names: &[SName]) -> Self {
        names.iter().fold(self.clone(), |q, n| q.add(*n))
    }
    fn merge_with(&self, other: &StyleQuery) -> Self {
        Self {
            atoms: self.atoms.plus_all(&other.atoms),
            level: LevelConstraint::of(
                self.level.level.max(other.level.level),
                self.level.star || other.level.star,
            ),
        }
    }
    pub fn has_stereotype(&self) -> bool {
        self.atoms.has_stereotype()
    }
    pub(crate) fn stereotype_count(&self) -> usize {
        self.atoms
            .iter()
            .filter(|a| matches!(a, StyleAtom::Stereotype(_)))
            .count()
    }
}

/// How to merge values that already exist (`MergeStrategy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    KeepExistingValueOfStereotype,
    OverwriteExistingValue,
}

/// A set of property values for a signature (`Style`).
#[derive(Debug, Clone)]
pub struct Style {
    query: StyleQuery,
    map: BTreeMap<PName, Value>,
}

impl Style {
    pub fn new(query: StyleQuery, map: BTreeMap<PName, Value>) -> Self {
        Self { query, map }
    }
    pub fn query(&self) -> &StyleQuery {
        &self.query
    }
    /// The value, or [`Value::Null`] when absent.
    pub fn value(&self, name: PName) -> Value {
        self.map.get(&name).cloned().unwrap_or(Value::Null)
    }
    pub fn has_value(&self, name: PName) -> bool {
        self.map.contains_key(&name)
    }
    pub fn with_ancestor_rank(&self, rank: i32) -> Style {
        let map = self
            .map
            .iter()
            .map(|(k, v)| (*k, v.with_ancestor_rank(rank)))
            .collect();
        Style::new(self.query.clone(), map)
    }

    pub fn merge_with(&self, other: &Style, strategy: MergeStrategy) -> Style {
        let mut both = self.map.clone();
        for (k, v) in &other.map {
            let previous = self.map.get(k);
            if let Some(p) = previous {
                if p.specificity().has_stereotype()
                    && strategy == MergeStrategy::KeepExistingValueOfStereotype
                {
                    continue;
                }
            }
            both.insert(*k, v.merge_with(previous));
        }
        Style::new(self.query.merge_with(&other.query), both)
    }

    /// Replaces a property by a forced-override string (`eventuallyOverride`).
    pub fn override_string(&self, name: PName, value: &str) -> Style {
        let mut map = self.map.clone();
        map.insert(name, Value::regular(value, Specificity::forced_override()));
        Style::new(self.query.clone(), map)
    }

    /// A number rendered as Java's `"" + double` does.
    pub fn override_number(&self, name: PName, value: f64) -> Style {
        self.override_string(name, &format!("{value:?}"))
    }

    pub fn override_color(&self, name: PName, color: Option<crate::klimt::color::HColor>) -> Style {
        let Some(color) = color else {
            return self.clone();
        };
        let spec = self
            .map
            .get(&name)
            .map_or_else(Specificity::forced_override, |v| v.specificity());
        let mut map = self.map.clone();
        map.insert(name, Value::Color(color, spec));
        Style::new(self.query.clone(), map)
    }

    pub fn shadowing(&self) -> f64 {
        match self.map.get(&PName::Shadowing) {
            None => 0.0,
            Some(v) => v.as_double_default_to(1.5),
        }
    }

    pub fn font(&self) -> UFont {
        let name = self.value(PName::FontName).as_string();
        let size = match self.value(PName::FontSize).as_int_or_minus_one() {
            -1 => 14,
            s => s,
        };
        let mut face = self.value(PName::FontStyle).as_font_face();
        let weight = self.value(PName::FontWeight).as_font_face();
        if weight.weight != 400 {
            face.weight = weight.weight;
        }
        UFont::new(&name, face, size)
    }

    pub fn stroke(&self) -> UStroke {
        self.stroke_of(PName::LineThickness, PName::LineStyle)
    }

    pub fn stroke_of(&self, thickness: PName, style: PName) -> UStroke {
        let thickness = self.value(thickness).as_double();
        let dash = self.value(style).as_string();
        if dash.is_empty() {
            return UStroke::with_thickness(thickness);
        }
        let mut it = dash.split(['-', ';', ',']).filter(|s| !s.is_empty());
        let visible = it.next().and_then(|s| s.trim().parse::<f64>().ok());
        let Some(visible) = visible else {
            return UStroke::with_thickness(thickness);
        };
        let space = match it.next() {
            None => visible,
            Some(s) => match s.trim().parse::<f64>() {
                Ok(v) => v,
                Err(_) => return UStroke::with_thickness(thickness),
            },
        };
        UStroke::new(visible, space, thickness)
    }

    /// Font, color and hyperlink look of the style (`getFontConfiguration`).
    pub fn font_configuration(
        &self,
        colors: Option<&crate::klimt::color::colors::Colors>,
    ) -> crate::klimt::font::config::FontConfiguration {
        use crate::klimt::color::colors::ColorType;
        let font = self.font();
        let color = colors
            .and_then(|c| c.color(ColorType::Text).cloned())
            .unwrap_or_else(|| self.value(PName::FontColor).as_color());
        let hyperlink_color = self.value(PName::HyperLinkColor).as_color();
        let stroke = self.stroke_of(
            PName::HyperlinkUnderlineThickness,
            PName::HyperlinkUnderlineStyle,
        );
        crate::klimt::font::config::FontConfiguration::create(
            font,
            color,
            hyperlink_color,
            Some(stroke),
            8,
        )
    }

    /// Text framed with this style's border, background and padding
    /// (`createTextBlockBordered`).
    pub fn create_text_block_bordered(
        &self,
        note: &crate::klimt::creole::display::Display,
        skin: &crate::skin::SkinParam,
        line_break: &crate::klimt::creole::LineBreakStrategy,
    ) -> std::rc::Rc<dyn crate::klimt::shape::TextBlock> {
        use crate::klimt::shape::{with_margin, TextBlockBordered};
        let text_block = note.create_block(
            &self.font_configuration(None),
            self.horizontal_alignment(),
            &skin.skin_simple(),
            line_break,
            crate::klimt::creole::CreoleMode::Full,
            None,
            None,
            0.0,
            0.0,
        );
        let bordered = std::rc::Rc::new(TextBlockBordered {
            text_block,
            stroke: Some(self.stroke()),
            border_color: Some(self.value(PName::LineColor).as_color()),
            background_color: Some(self.value(PName::BackGroundColor).as_color()),
            corner_size: f64::from(self.value(PName::RoundCorner).as_int()),
            margins: self.padding(),
        });
        with_margin(bordered, self.margin())
    }

    /// Back/line colors, stroke, shadow and corners (`getSymbolContext`).
    pub fn symbol_context(
        &self,
        colors: Option<&crate::klimt::color::colors::Colors>,
    ) -> crate::klimt::fashion::Fashion {
        use crate::klimt::color::colors::ColorType;
        let back = colors
            .and_then(|c| c.color(ColorType::Back).cloned())
            .unwrap_or_else(|| self.value(PName::BackGroundColor).as_color());
        let fore = colors
            .and_then(|c| c.color(ColorType::Line).cloned())
            .unwrap_or_else(|| self.value(PName::LineColor).as_color());
        let round = self.value(PName::RoundCorner).as_double();
        let diagonal = self.value(PName::DiagonalCorner).as_double();
        crate::klimt::fashion::Fashion::new(Some(back), Some(fore))
            .with_stroke(self.stroke())
            .with_delta_shadow(self.shadowing())
            .with_corner(round, diagonal)
    }

    pub fn horizontal_alignment(&self) -> crate::klimt::geom::HorizontalAlignment {
        self.value(PName::HorizontalAlignment)
            .as_horizontal_alignment()
    }

    pub fn wrap_width(&self) -> crate::klimt::creole::LineBreakStrategy {
        crate::klimt::creole::LineBreakStrategy(Some(self.value(PName::MaximumWidth).as_string()))
    }

    /// Forced back/line/text colors (`eventuallyOverride(Colors)`).
    pub fn override_colors(&self, colors: &crate::klimt::color::colors::Colors) -> Style {
        use crate::klimt::color::colors::ColorType;
        let mut r = self.clone();
        for (t, p) in [
            (ColorType::Back, PName::BackGroundColor),
            (ColorType::Line, PName::LineColor),
            (ColorType::Text, PName::FontColor),
        ] {
            if let Some(c) = colors.color(t) {
                r = r.override_color(p, Some(c.clone()));
            }
        }
        r
    }

    /// Forces the stroke (`eventuallyOverride(UStroke)`).
    pub fn override_stroke(&self, stroke: &UStroke) -> Style {
        let r = self.override_string(PName::LineThickness, &format!("{:?}", stroke.thickness));
        r.override_string(
            PName::LineStyle,
            &format!("{:?}-{:?}", stroke.dash_visible, stroke.dash_space),
        )
    }

    /// Style where only the values of `nested` differing from `parent` override (`mergeNestedChildOver`).
    pub fn merge_nested_child_over(
        &self,
        nested: Option<&Style>,
        nested_parent: Option<&Style>,
        strategy: MergeStrategy,
    ) -> Style {
        let Some(nested) = nested else {
            return self.clone();
        };
        let divergent: BTreeMap<PName, Value> = nested
            .map
            .iter()
            .filter(|(k, v)| {
                nested_parent
                    .and_then(|p| p.map.get(k))
                    .is_none_or(|a| a.as_string() != v.as_string())
            })
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        if divergent.is_empty() {
            return self.clone();
        }
        self.merge_with(&Style::new(nested.query.clone(), divergent), strategy)
    }

    pub fn padding(&self) -> Margins {
        Margins::read(&self.value(PName::Padding).as_string())
    }

    pub fn margin(&self) -> Margins {
        Margins::read(&self.value(PName::Margin).as_string())
    }
}

/// CSS-like `top right bottom left` (`ClockwiseTopRightBottomLeft`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Margins {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Margins {
    pub const fn new(top: f64, right: f64, bottom: f64, left: f64) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
    pub const fn same(v: f64) -> Self {
        Self::new(v, v, v, v)
    }
    pub fn read(value: &str) -> Self {
        if value.is_empty() || !value.chars().all(|c| c.is_ascii_digit() || c == ' ') {
            return Self::default();
        }
        let n: Vec<f64> = value
            .split_whitespace()
            .filter_map(|s| s.parse::<i32>().ok().map(f64::from))
            .collect();
        match n[..] {
            [a] => Self::new(a, a, a, a),
            [a, b] => Self::new(a, b, a, b),
            [a, b, c] => Self::new(a, b, c, b),
            [a, b, c, d] => Self::new(a, b, c, d),
            _ => Self::default(),
        }
    }
    pub fn is_zero(&self) -> bool {
        *self == Self::default()
    }
    pub fn width(&self, w: f64) -> f64 {
        self.left + w + self.right
    }
    pub fn height(&self, h: f64) -> f64 {
        self.top + h + self.bottom
    }
}

struct TrieNode {
    children: HashMap<StyleAtom, TrieNode>,
    here: Vec<(LevelConstraint, usize)>,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            here: Vec::new(),
        }
    }

    fn collect(
        &self,
        atoms: &[StyleAtom],
        from: usize,
        level: LevelConstraint,
        out: &mut Vec<usize>,
    ) {
        for (lc, idx) in &self.here {
            if LevelConstraint::matches(*lc, level) {
                out.push(*idx);
            }
        }
        for i in from..atoms.len() {
            if let Some(child) = self.children.get(&atoms[i]) {
                child.collect(atoms, i + 1, level, out);
            }
        }
    }
}

/// All styles of a sheet with signature lookup (`StyleIndex`).
#[derive(Clone, Default)]
pub struct StyleIndex {
    all: Rc<Vec<Style>>,
    /// The signature trie, built on first lookup. Rebuilding it per lookup used to cost more than
    /// the lookup itself (a hash of every atom of every style).
    trie: Rc<std::cell::OnceCell<Rc<TrieNode>>>,
}

impl StyleIndex {
    fn merge_or_append(list: &mut Vec<Style>, new: Style) {
        for s in list.iter_mut() {
            if s.query == new.query {
                *s = s.merge_with(&new, MergeStrategy::OverwriteExistingValue);
                return;
            }
        }
        list.push(new);
    }

    pub fn with_loaded(&self, new: Style) -> StyleIndex {
        self.with_muted(vec![new])
    }

    pub fn with_muted(&self, new: Vec<Style>) -> StyleIndex {
        if new.is_empty() {
            return self.clone();
        }
        let mut v = (*self.all).clone();
        for s in new {
            Self::merge_or_append(&mut v, s);
        }
        StyleIndex {
            all: Rc::new(v),
            trie: Rc::new(std::cell::OnceCell::new()),
        }
    }

    pub fn all_styles(&self) -> &[Style] {
        &self.all
    }

    /// Styles whose signature is contained in `query`, in trie order.
    pub fn find_matching(&self, query: &StyleQuery) -> Vec<&Style> {
        let trie = self
            .trie
            .get_or_init(|| Rc::new(Self::build_trie(&self.all)));
        let atoms: Vec<StyleAtom> = query.atoms.iter().cloned().collect();
        let mut idx = Vec::new();
        trie.collect(&atoms, 0, query.level, &mut idx);
        idx.into_iter().map(|i| &self.all[i]).collect()
    }

    fn build_trie(styles: &[Style]) -> TrieNode {
        let mut root = TrieNode::new();
        for (i, s) in styles.iter().enumerate() {
            let mut cur = &mut root;
            for atom in s.query.atoms.iter() {
                cur = cur
                    .children
                    .entry(atom.clone())
                    .or_insert_with(TrieNode::new);
            }
            cur.here.push((s.query.level, i));
        }
        root
    }

    pub fn merged_style(&self, query: &StyleQuery) -> Option<Style> {
        let mut merged: Option<Style> = None;
        for s in self.find_matching(query) {
            merged = Some(match merged {
                None => s.clone(),
                Some(m) => m.merge_with(s, MergeStrategy::OverwriteExistingValue),
            });
        }
        merged
    }
}

/// Style lookup for one diagram (`StyleBuilder`).
#[derive(Clone, Default)]
pub struct StyleBuilder {
    index: StyleIndex,
    counter: i32,
}

impl StyleBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a style sheet (`StyleLoader.loadSkin` on already-read text).
    pub fn from_sheet(text: &str) -> Result<StyleBuilder, parser::StyleParsingError> {
        let mut b = StyleBuilder::new();
        let styles = b.parse_style_text(text)?;
        if styles.is_empty() {
            return Err(parser::StyleParsingError("No style found".into()));
        }
        for s in styles {
            b.load_internal(s);
        }
        Ok(b)
    }

    /// The built-in `plantuml.skin` style sheet.
    pub fn default_skin() -> StyleBuilder {
        StyleBuilder::from_sheet(include_str!("plantuml.skin")).expect("built-in skin is valid")
    }

    pub fn next_int(&mut self) -> i32 {
        self.counter += 1;
        self.counter
    }

    pub fn parse_style_text(
        &mut self,
        text: &str,
    ) -> Result<Vec<Style>, parser::StyleParsingError> {
        parser::parse_style_text(text, &mut || self.next_int())
    }

    pub fn load_internal(&mut self, style: Style) {
        assert!(!style.query.level.is_star());
        self.index = self.index.with_loaded(style);
    }

    /// Returns a builder with `styles` merged over this one (`muteStyle`).
    pub fn mute_style(&self, styles: Vec<Style>) -> StyleBuilder {
        StyleBuilder {
            index: self.index.with_muted(styles),
            counter: self.counter,
        }
    }

    pub fn merged_style(&self, query: &StyleQuery) -> Option<Style> {
        self.index.merged_style(query)
    }

    /// `getMergedStyleSpecial`: star styles get an ancestor rank.
    pub fn merged_style_special(&self, query: &StyleQuery, ancestor_rank: i32) -> Option<Style> {
        let mut merged: Option<Style> = None;
        for s in self.index.find_matching(query) {
            let tmp = if s.query.level.is_star() {
                s.with_ancestor_rank(ancestor_rank)
            } else {
                s.clone()
            };
            merged = Some(match merged {
                None => tmp,
                Some(m) => m.merge_with(&tmp, MergeStrategy::OverwriteExistingValue),
            });
        }
        merged
    }

    /// Bare `.name { ... }` style (`createStyleStereotype`).
    pub fn create_style_stereotype(&self, name: &str) -> Style {
        let q = StyleQuery::empty().with_stereotype(&name.to_lowercase());
        let mut result: Option<Style> = None;
        for s in self.index.all_styles() {
            if s.query == q {
                result = Some(match result {
                    None => s.clone(),
                    Some(r) => r.merge_with(s, MergeStrategy::OverwriteExistingValue),
                });
            }
        }
        result.unwrap_or_else(|| Style::new(q, BTreeMap::new()))
    }

    /// Root properties that a sheet must define (`StyleLoader.getMissingRootProperties`).
    pub fn missing_root_properties(&self) -> Vec<PName> {
        const MANDATORY: [PName; 8] = [
            PName::FontName,
            PName::FontSize,
            PName::FontStyle,
            PName::FontColor,
            PName::LineColor,
            PName::LineThickness,
            PName::BackGroundColor,
            PName::HorizontalAlignment,
        ];
        let root = self.merged_style(&StyleQuery::of(&[SName::Root]));
        MANDATORY
            .into_iter()
            .filter(|p| root.as_ref().is_none_or(|r| !r.has_value(*p)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_skin_has_root_properties() {
        let b = StyleBuilder::default_skin();
        assert!(b.missing_root_properties().is_empty());
        let q = StyleQuery::of(&[
            SName::Root,
            SName::Element,
            SName::ClassDiagram,
            SName::Class,
        ]);
        let s = b.merged_style(&q).unwrap();
        assert_eq!(s.value(PName::FontSize).as_int(), 14);
        assert!(s.value(PName::BackGroundColor).as_string().starts_with('#'));
    }

    #[test]
    fn query_helpers() {
        let q = StyleQuery::of(&[SName::Note]).add_all(&[SName::Title, SName::Header]);
        assert!(!q.has_stereotype());
        assert!(q.with_stereotype("A_b.c").has_stereotype());
        assert_eq!(
            StyleAtom::stereotype("A_b.c"),
            StyleAtom::Stereotype("abc".into())
        );
        assert!(AtomArray::default().is_empty());
        assert_eq!(q.atoms.len(), 3);
        assert!(LevelConstraint::matches(
            LevelConstraint::NONE,
            LevelConstraint::of(2, false)
        ));
        assert!(!LevelConstraint::matches(
            LevelConstraint::of(1, false),
            LevelConstraint::NONE
        ));
        assert!(LevelConstraint::matches(
            LevelConstraint::of(1, true),
            LevelConstraint::of(5, true)
        ));
        assert!(!LevelConstraint::matches(
            LevelConstraint::of(1, false),
            LevelConstraint::of(5, false)
        ));
        assert_eq!(LevelConstraint::of(3, false).level(), 3);
    }
}
