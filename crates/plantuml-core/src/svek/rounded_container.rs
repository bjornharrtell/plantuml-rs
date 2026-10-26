//! The rounded north/center/south container of a state cluster
//! (`RoundedContainer`).

use crate::klimt::color::HColor;
use crate::klimt::drawing::ugraphic::{UGraphic, UPath, URectangle, UShape, UStroke, UTranslate};

pub struct RoundedContainer {
    width: f64,
    height: f64,
    name_height: f64,
    description_height: f64,
    border: HColor,
    north_back: HColor,
    center_back: HColor,
    south_back: HColor,
    stroke: UStroke,
    rounded: f64,
    shadowing: f64,
}

impl RoundedContainer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        border: HColor,
        width: f64,
        height: f64,
        name_height: f64,
        description_height: f64,
        north_back: HColor,
        center_back: HColor,
        south_back: HColor,
        stroke: UStroke,
        rounded: f64,
        shadowing: f64,
    ) -> Self {
        Self {
            width,
            height,
            name_height,
            description_height,
            border,
            north_back,
            center_back,
            south_back,
            stroke,
            rounded,
            shadowing,
        }
    }

    pub fn draw(&self, ug: &UGraphic) {
        let ug = ug
            .apply_color(self.border.clone())
            .apply_stroke(self.stroke);
        let mut rect = URectangle::build(self.width, self.height).rounded(self.rounded);
        if self.shadowing > 0.0 {
            rect.delta_shadow = self.shadowing;
            ug.apply_back_color(HColor::TRANSPARENT).draw(rect.clone());
            rect.delta_shadow = 0.0;
        }
        rounded_north(
            &ug,
            self.width,
            self.name_height,
            &self.north_back,
            self.rounded,
        );
        self.draw_center(&ug);
        rounded_south(
            &ug.apply_translate(UTranslate::dy(self.name_height + self.description_height)),
            self.width,
            self.height - self.name_height - self.description_height,
            &self.south_back,
            self.rounded,
        );
        ug.apply_back_color(HColor::TRANSPARENT).draw(rect);
        if self.name_height > 0.0 {
            ug.apply_translate(UTranslate::dy(self.name_height))
                .draw(crate::klimt::drawing::ugraphic::ULine::hline(self.width));
        }
        if self.description_height > 0.0 && self.name_height + self.description_height > 0.0 {
            ug.apply_translate(UTranslate::dy(self.name_height + self.description_height))
                .draw(crate::klimt::drawing::ugraphic::ULine::hline(self.width));
        }
    }

    fn draw_center(&self, ug: &UGraphic) {
        if self.description_height == 0.0 {
            return;
        }
        let rect = URectangle::build(self.width, self.description_height);
        ug.apply_stroke(UStroke::simple())
            .apply_color(self.center_back.clone())
            .apply_back_color(self.center_back.clone())
            .apply_translate(UTranslate::dy(self.name_height))
            .draw(rect);
    }
}

fn rounded_north(ug: &UGraphic, width: f64, height: f64, back: &HColor, rounded: f64) {
    if back.is_transparent() {
        return;
    }
    let shape: UShape = if rounded == 0.0 {
        URectangle::build(width, height).into()
    } else {
        let mut path = UPath::new();
        path.move_to(rounded / 2.0, 0.0);
        path.line_to(width - rounded / 2.0, 0.0);
        path.arc_to(
            rounded / 2.0,
            rounded / 2.0,
            0.0,
            false,
            true,
            width,
            rounded / 2.0,
        );
        path.line_to(width, height);
        path.line_to(0.0, height);
        path.line_to(0.0, rounded / 2.0);
        path.arc_to(
            rounded / 2.0,
            rounded / 2.0,
            0.0,
            false,
            true,
            rounded / 2.0,
            0.0,
        );
        path.close_path();
        path.into()
    };
    ug.apply_stroke(UStroke::simple())
        .apply_color(back.clone())
        .apply_back_color(back.clone())
        .draw(shape);
}

fn rounded_south(ug: &UGraphic, width: f64, height: f64, back: &HColor, rounded: f64) {
    if back.is_transparent() {
        return;
    }
    let shape: UShape = if rounded == 0.0 {
        URectangle::build(width, height).into()
    } else {
        let mut path = UPath::new();
        path.move_to(0.0, 0.0);
        path.line_to(width, 0.0);
        path.line_to(width, height - rounded / 2.0);
        path.arc_to(
            rounded / 2.0,
            rounded / 2.0,
            0.0,
            false,
            true,
            width - rounded / 2.0,
            height,
        );
        path.line_to(rounded / 2.0, height);
        path.arc_to(
            rounded / 2.0,
            rounded / 2.0,
            0.0,
            false,
            true,
            0.0,
            height - rounded / 2.0,
        );
        path.line_to(0.0, 0.0);
        path.close_path();
        path.into()
    };
    ug.apply_stroke(UStroke::simple())
        .apply_color(back.clone())
        .apply_back_color(back.clone())
        .draw(shape);
}
