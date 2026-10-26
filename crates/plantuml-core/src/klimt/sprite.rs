//! Pixel sprites (`sprite $name [WxH/depth] { ... }`) and how a label draws them.
//!
//! Only the grey-level form is decoded, which is what PlantUML's stdlib (and C4-PlantUML in
//! particular) uses. A sprite is drawn as one rectangle per run of equal pixels of a row.

use super::color::HColor;
use super::drawing::ugraphic::{UGraphic, URectangle, UStroke, UTranslate};
use super::font::Dimension;

/// One pixel of a sprite: a grey level, 0 being transparent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pixel(u8);

impl Pixel {
    fn is_transparent(self) -> bool {
        self.0 == 0
    }
}

/// A sprite definition (`Sprite`), as a grid of grey levels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite {
    name: String,
    width: usize,
    height: usize,
    pixels: Vec<Pixel>,
}

impl Sprite {
    /// Decodes `sprite $name [WxH/depth] { ... }`: one hex digit per pixel when the depth is 16.
    pub fn new(
        name: &str,
        rows: &[String],
        width: usize,
        height: usize,
        depth: usize,
    ) -> Option<Self> {
        if width == 0 || height == 0 || depth != 16 {
            return None;
        }
        let mut pixels = Vec::with_capacity(width * height);
        for row in rows {
            let row = row.trim();
            if row.is_empty() {
                continue;
            }
            for c in row.chars() {
                let level = c.to_digit(16)? as u8;
                pixels.push(Pixel(level * 17));
            }
        }
        if pixels.len() < width * height {
            return None;
        }
        pixels.truncate(width * height);
        Some(Self {
            name: name.to_string(),
            width,
            height,
            pixels,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Size of the sprite in points: one pixel is one point.
    pub fn dimension(&self) -> Dimension {
        Dimension::new(self.width as f64, self.height as f64)
    }

    fn level(&self, x: usize, y: usize) -> Pixel {
        self.pixels[y * self.width + x]
    }

    /// Draws the sprite with the current colour, one rectangle per run of opaque pixels.
    pub fn draw(&self, ug: &UGraphic) {
        let color = ug.color().clone();
        let ug = ug
            .apply_back_color(color)
            .apply_stroke(UStroke::with_thickness(0.0));
        for y in 0..self.height {
            let mut x = 0;
            while x < self.width {
                if self.level(x, y).is_transparent() {
                    x += 1;
                    continue;
                }
                let start = x;
                while x < self.width && !self.level(x, y).is_transparent() {
                    x += 1;
                }
                ug.apply_translate(UTranslate::new(start as f64, y as f64))
                    .draw(URectangle::build((x - start) as f64, 1.0));
            }
        }
    }
}

/// Registry of sprites defined by the diagram, looked up by `<$name>` in a label.
pub type SpriteRegistry = std::collections::HashMap<String, std::rc::Rc<Sprite>>;

/// Colour used for a sprite pixel; kept as a helper so the grey level can be honoured later.
#[allow(dead_code)]
pub fn pixel_color(base: &HColor, _level: Pixel) -> HColor {
    base.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<String> {
        vec![
            "0F00".to_string(),
            "FF00".to_string(),
            "0F00".to_string(),
            "0000".to_string(),
        ]
    }

    #[test]
    fn decodes_hex_rows() {
        let sprite = Sprite::new("s", &rows(), 4, 4, 16).expect("sprite");
        assert_eq!(sprite.name(), "s");
        assert_eq!(sprite.dimension(), Dimension::new(4.0, 4.0));
        assert!(sprite.level(0, 0).is_transparent());
        assert!(!sprite.level(1, 0).is_transparent());
        assert!(!sprite.level(0, 1).is_transparent());
    }

    #[test]
    fn rejects_what_it_cannot_decode() {
        assert!(Sprite::new("s", &rows(), 4, 4, 8).is_none());
        assert!(Sprite::new("s", &rows(), 0, 4, 16).is_none());
        assert!(Sprite::new("s", &["0Z".to_string()], 2, 1, 16).is_none());
        assert!(Sprite::new("s", &["00".to_string()], 4, 1, 16).is_none());
    }
}
