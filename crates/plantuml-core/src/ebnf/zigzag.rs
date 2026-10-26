//! S-shaped connector of the optional tile (`Zigzag`).

use crate::klimt::drawing::ugraphic::UPath;

/// A cubic zigzag of the given control offset, width and height (`Zigzag`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zigzag {
    ctrl: f64,
    width: f64,
    height: f64,
}

impl Zigzag {
    pub fn new(ctrl: f64, width: f64, height: f64) -> Self {
        Self {
            ctrl,
            width,
            height,
        }
    }

    /// The path from the top left corner to the bottom right corner.
    pub fn path_down(&self) -> UPath {
        let mut path = UPath::new();
        let xm = self.width / 2.0;
        let ym = self.height / 2.0;
        path.move_to(0.0, 0.0);
        path.cubic_to(self.ctrl, 0.0, xm, ym - self.ctrl, xm, ym);
        path.cubic_to(
            xm,
            ym + self.ctrl,
            self.width - self.ctrl,
            self.height,
            self.width,
            self.height,
        );
        path
    }

    /// The path from the bottom left corner to the top right corner.
    pub fn path_up(&self) -> UPath {
        let mut path = UPath::new();
        let xm = self.width / 2.0;
        let ym = self.height / 2.0;
        path.move_to(0.0, self.height);
        path.cubic_to(self.ctrl, self.height, xm, ym + self.ctrl, xm, ym);
        path.cubic_to(
            xm,
            ym - self.ctrl,
            self.width - self.ctrl,
            0.0,
            self.width,
            0.0,
        );
        path
    }
}
