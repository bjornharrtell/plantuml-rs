//! Notes beside activities (`FtileNoteAlone`, `FtileWithNoteOpale`, `FtileWithNotes`).

use super::factory::FtileRef;
use super::{same_ftile, Ftile, FtileGeometry, FtileGraphic};
use crate::activitydiagram3::instruction::PositionedNote;
use crate::activitydiagram3::rainbow::LinkRendering;
use crate::activitydiagram3::swimlanes::Swimlane;
use crate::klimt::color::colors::ColorType;
use crate::klimt::drawing::ugraphic::UTranslate;
use crate::klimt::font::StringBounder;
use crate::klimt::geom::{Direction, VerticalAlignment};
use crate::skin::SkinParam;
use crate::style::{queries, PName};
use crate::svek::image_note::{note_body, Opale};
use std::rc::Rc;

const SUPP_SPACE: f64 = 20.0;

/// The folded-corner box of one note (`Opale`).
pub(crate) fn opale_of(note: &PositionedNote, skin: &SkinParam) -> Opale {
    let query = queries::ACTIVITYDIAG_NOTE
        .clone()
        .with_stereotype_obj(note.stereotype.as_ref());
    let style = skin
        .style_builder()
        .merged_style(&query)
        .expect("note style");
    let colors = note.colors.clone().unwrap_or_default();
    let back = colors
        .color(ColorType::Back)
        .cloned()
        .unwrap_or_else(|| style.value(PName::BackGroundColor).as_color());
    Opale {
        shadowing: style.shadowing(),
        border: style.value(PName::LineColor).as_color(),
        back,
        text: note_body(
            note.display.clone(),
            skin,
            &style,
            style.horizontal_alignment(),
        ),
        stroke: colors.mute_stroke(style.stroke()),
        round_corner: f64::from(skin.as_int("roundcorner", 0)),
    }
}

/// A note with nothing to attach to (`FtileNoteAlone`).
pub struct FtileNoteAlone {
    opale: Opale,
    with_out_point: bool,
    swimlane: Option<Rc<Swimlane>>,
    skin: Rc<SkinParam>,
}

impl FtileNoteAlone {
    pub fn create(
        note: &PositionedNote,
        skin: Rc<SkinParam>,
        swimlane: Option<Rc<Swimlane>>,
    ) -> FtileRef {
        Rc::new(Self {
            opale: opale_of(note, &skin),
            with_out_point: !note.floating,
            swimlane,
            skin,
        })
    }
}

impl Ftile for FtileNoteAlone {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn swimlane_in(&self) -> Option<Rc<Swimlane>> {
        self.swimlane.clone()
    }

    fn swimlane_out(&self) -> Option<Rc<Swimlane>> {
        self.swimlane.clone()
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        self.swimlane.iter().cloned().collect()
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let d = self.opale.dimension(sb);
        if self.with_out_point {
            FtileGeometry::with_out(d.width, d.height, d.width / 2.0, 0.0, d.height)
        } else {
            FtileGeometry::new(d.width, d.height, d.width / 2.0, 0.0)
        }
    }

    fn draw(&self, g: &FtileGraphic) {
        self.opale.draw(g.ug(), None);
    }
}

/// A tile with one note beside it (`FtileWithNoteOpale`).
struct FtileWithNoteOpale {
    tile: FtileRef,
    opale: Opale,
    left: bool,
    with_link: bool,
    vertical: VerticalAlignment,
    swimlane_note: Option<Rc<Swimlane>>,
    skin: Rc<SkinParam>,
}

impl FtileWithNoteOpale {
    fn internal(&self, sb: &dyn StringBounder) -> (f64, f64) {
        let note = self.opale.dimension(sb);
        let tile = self.tile.calculate_dimension(sb);
        (
            tile.width + note.width + SUPP_SPACE,
            note.height.max(tile.height),
        )
    }

    fn translate_tile(&self, sb: &dyn StringBounder) -> UTranslate {
        let (_, h) = self.internal(sb);
        let note = self.opale.dimension(sb);
        let tile = self.tile.calculate_dimension(sb);
        let marge = if self.left {
            note.width + SUPP_SPACE
        } else {
            0.0
        };
        UTranslate::new(marge, (h - tile.height) / 2.0)
    }

    fn translate_opale(&self, sb: &dyn StringBounder) -> UTranslate {
        let (w, h) = self.internal(sb);
        let note = self.opale.dimension(sb);
        let y = if self.vertical == VerticalAlignment::Center {
            (h - note.height) / 2.0
        } else {
            0.0
        };
        if self.left {
            UTranslate::dy(y)
        } else {
            UTranslate::new(w - note.width, y)
        }
    }
}

impl Ftile for FtileWithNoteOpale {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn decorated(&self) -> Option<FtileRef> {
        Some(self.tile.clone())
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let (w, h) = self.internal(sb);
        let orig = self.tile.calculate_dimension(sb);
        let t = self.translate_tile(sb);
        if orig.has_point_out() {
            FtileGeometry::with_out(
                w,
                h,
                orig.left() + t.dx,
                orig.in_y + t.dy,
                orig.out_y + t.dy,
            )
        } else {
            FtileGeometry::new(w, h, orig.left() + t.dx, orig.in_y + t.dy)
        }
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        if same_ftile(child, self.tile.as_ref()) {
            return Some(UTranslate::default());
        }
        self.tile.translate_for(child, sb)
    }

    fn swimlanes(&self) -> Vec<Rc<Swimlane>> {
        let mut lanes = self.tile.swimlanes();
        if let Some(l) = &self.swimlane_note {
            if !lanes.iter().any(|x| Rc::ptr_eq(x, l)) {
                lanes.push(l.clone());
            }
        }
        lanes
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.tile.in_link_rendering()
    }

    fn draw(&self, g: &FtileGraphic) {
        let sb = g.string_bounder();
        let note = self.opale.dimension(sb);
        let mid = note.height / 2.0;
        let pointer = self.with_link.then(|| {
            if self.left {
                (
                    Direction::Right,
                    (note.width, mid),
                    (note.width + SUPP_SPACE, mid),
                )
            } else {
                (Direction::Left, (0.0, mid), (-SUPP_SPACE, mid))
            }
        });
        self.opale
            .draw(&g.ug().apply_translate(self.translate_opale(sb)), pointer);
        g.apply_translate(self.translate_tile(sb))
            .draw_ftile(self.tile.as_ref());
    }
}

/// A tile with several notes, stacked on each side, without pointers (`FtileWithNotes`).
struct FtileWithNotes {
    tile: FtileRef,
    left: Vec<Opale>,
    right: Vec<Opale>,
    vertical: VerticalAlignment,
    skin: Rc<SkinParam>,
}

const NOTE_MARGIN: f64 = 10.0;

impl FtileWithNotes {
    fn stack(side: &[Opale], sb: &dyn StringBounder) -> (f64, f64) {
        let mut width = 0.0f64;
        let mut height = 0.0;
        for o in side {
            let d = o.dimension(sb);
            width = width.max(d.width + 2.0 * NOTE_MARGIN);
            height += d.height + 2.0 * NOTE_MARGIN;
        }
        (width, height)
    }

    fn internal(&self, sb: &dyn StringBounder) -> (f64, f64) {
        let tile = self.tile.calculate_dimension(sb);
        let (lw, lh) = Self::stack(&self.left, sb);
        let (rw, rh) = Self::stack(&self.right, sb);
        (tile.width + lw + rw, tile.height.max(lh).max(rh))
    }

    fn y_of(&self, content_height: f64, sb: &dyn StringBounder) -> f64 {
        if self.vertical == VerticalAlignment::Top {
            0.0
        } else {
            (self.internal(sb).1 - content_height) / 2.0
        }
    }

    fn translate_tile(&self, sb: &dyn StringBounder) -> UTranslate {
        let tile = self.tile.calculate_dimension(sb);
        UTranslate::new(Self::stack(&self.left, sb).0, self.y_of(tile.height, sb))
    }
}

impl Ftile for FtileWithNotes {
    fn skin(&self) -> Rc<SkinParam> {
        self.skin.clone()
    }

    fn decorated(&self) -> Option<FtileRef> {
        Some(self.tile.clone())
    }

    fn calculate_dimension(&self, sb: &dyn StringBounder) -> FtileGeometry {
        let (w, h) = self.internal(sb);
        let orig = self.tile.calculate_dimension(sb);
        let t = self.translate_tile(sb);
        if orig.has_point_out() {
            FtileGeometry::with_out(
                w,
                h,
                orig.left() + t.dx,
                orig.in_y + t.dy,
                orig.out_y + t.dy,
            )
        } else {
            FtileGeometry::new(w, h, orig.left() + t.dx, orig.in_y + t.dy)
        }
    }

    fn translate_for(&self, child: &dyn Ftile, sb: &dyn StringBounder) -> Option<UTranslate> {
        let t = self.translate_tile(sb);
        if same_ftile(child, self.tile.as_ref()) {
            return Some(t);
        }
        self.tile.translate_for(child, sb).map(|c| c.compose(t))
    }

    fn in_link_rendering(&self) -> LinkRendering {
        self.tile.in_link_rendering()
    }

    fn draw(&self, g: &FtileGraphic) {
        let sb = g.string_bounder();
        let (total_w, _) = self.internal(sb);
        let draw_side = |side: &[Opale], x: f64, width: f64| {
            let (_, height) = Self::stack(side, sb);
            let mut y = self.y_of(height, sb);
            for o in side {
                let d = o.dimension(sb);
                y += NOTE_MARGIN;
                o.draw(
                    &g.ug()
                        .apply_translate(UTranslate::new(x + (width - d.width) / 2.0, y)),
                    None,
                );
                y += d.height + NOTE_MARGIN;
            }
        };
        let (lw, _) = Self::stack(&self.left, sb);
        let (rw, _) = Self::stack(&self.right, sb);
        draw_side(&self.left, 0.0, lw);
        draw_side(&self.right, total_w - rw, rw);
        g.apply_translate(self.translate_tile(sb))
            .draw_ftile(self.tile.as_ref());
    }
}

/// Puts the notes beside a tile (`FtileWithNoteOpale.create`).
pub fn with_notes(
    tile: FtileRef,
    notes: &[PositionedNote],
    with_link: bool,
    vertical: VerticalAlignment,
) -> FtileRef {
    let skin = tile.skin();
    if notes.len() > 1 {
        let (left, right) = notes.iter().partition::<Vec<_>, _>(|n| n.left);
        return Rc::new(FtileWithNotes {
            tile,
            left: left.iter().map(|n| opale_of(n, &skin)).collect(),
            right: right.iter().map(|n| opale_of(n, &skin)).collect(),
            vertical,
            skin,
        });
    }
    let note = &notes[0];
    Rc::new(FtileWithNoteOpale {
        opale: opale_of(note, &skin),
        left: note.left,
        with_link: with_link && !note.floating,
        vertical,
        swimlane_note: note.swimlane.clone(),
        tile,
        skin,
    })
}
