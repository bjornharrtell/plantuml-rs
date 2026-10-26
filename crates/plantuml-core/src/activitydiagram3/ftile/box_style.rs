//! Shapes of an action box (`BoxStyle` and its `BoxStyle*` subclasses).

use crate::klimt::drawing::ugraphic::{UGraphic, ULine, UPath, UPolygon, URectangle, UTranslate};
use crate::stereo::Stereotype;

const DELTA_INPUT_OUTPUT: f64 = 10.0;
const DELTA_CONTINUOUS: f64 = 5.0;
const PADDING: f64 = 5.0;

/// The look of an action, chosen with `<<input>>` and friends after the `;`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxStyle {
    Plain,
    SdlInput,
    SdlOutput,
    SdlProcedure,
    SdlLoad,
    SdlSave,
    SdlContinuous,
    SdlTask,
    UmlObject,
    UmlObjectSignal,
    UmlTrigger,
    UmlSendSignal,
    UmlAcceptEvent,
    UmlTimeEvent,
}

const ALL: [BoxStyle; 14] = [
    BoxStyle::Plain,
    BoxStyle::SdlInput,
    BoxStyle::SdlOutput,
    BoxStyle::SdlProcedure,
    BoxStyle::SdlLoad,
    BoxStyle::SdlSave,
    BoxStyle::SdlContinuous,
    BoxStyle::SdlTask,
    BoxStyle::UmlObject,
    BoxStyle::UmlObjectSignal,
    BoxStyle::UmlTrigger,
    BoxStyle::UmlSendSignal,
    BoxStyle::UmlAcceptEvent,
    BoxStyle::UmlTimeEvent,
];

impl BoxStyle {
    /// Name used in `<<name>>`; `None` for the plain box.
    pub fn name(self) -> Option<&'static str> {
        Some(match self {
            BoxStyle::Plain => return None,
            BoxStyle::SdlInput => "input",
            BoxStyle::SdlOutput => "output",
            BoxStyle::SdlProcedure => "procedure",
            BoxStyle::SdlLoad => "load",
            BoxStyle::SdlSave => "save",
            BoxStyle::SdlContinuous => "continuous",
            BoxStyle::SdlTask => "task",
            BoxStyle::UmlObject => "object",
            BoxStyle::UmlObjectSignal => "objectSignal",
            BoxStyle::UmlTrigger => "trigger",
            BoxStyle::UmlSendSignal => "sendSignal",
            BoxStyle::UmlAcceptEvent => "acceptEvent",
            BoxStyle::UmlTimeEvent => "timeEvent",
        })
    }

    /// Extra width taken by the pointed end of the shape.
    pub fn shield(self) -> f64 {
        match self {
            BoxStyle::SdlInput
            | BoxStyle::SdlOutput
            | BoxStyle::UmlObjectSignal
            | BoxStyle::UmlTrigger
            | BoxStyle::UmlSendSignal
            | BoxStyle::UmlAcceptEvent
            | BoxStyle::UmlTimeEvent => 10.0,
            _ => 0.0,
        }
    }

    /// `fromString`: dashes are ignored, the match is case insensitive.
    pub fn from_string(style: &str) -> BoxStyle {
        let style = style.replace('-', "");
        ALL.into_iter()
            .find(|b| b.name().is_some_and(|n| n.eq_ignore_ascii_case(&style)))
            .unwrap_or(BoxStyle::Plain)
    }

    pub fn stereotype(self) -> Option<Stereotype> {
        self.name().map(|n| Stereotype::build(&format!("<<{n}>>")))
    }

    /// Draws the outline in the colors and stroke of `ug`.
    pub fn draw_me(self, ug: &UGraphic, width: f64, height: f64, shadowing: f64, round: f64) {
        let width = width - self.shield();
        match self {
            BoxStyle::Plain => {
                let mut r = URectangle::build(width, height).rounded(round);
                r.delta_shadow = shadowing;
                ug.draw(r);
            }
            BoxStyle::SdlTask | BoxStyle::UmlObject => {
                let mut r = URectangle::build(width, height);
                r.delta_shadow = shadowing;
                ug.draw(r);
            }
            BoxStyle::SdlProcedure => {
                let mut r = URectangle::build(width, height);
                r.delta_shadow = shadowing;
                ug.draw(r);
                let vline = ULine::vline(height);
                ug.apply_translate(UTranslate::dx(PADDING))
                    .draw(vline.clone());
                ug.apply_translate(UTranslate::dx(width - PADDING))
                    .draw(vline);
            }
            BoxStyle::SdlContinuous => {
                let mut p = UPath::new();
                p.move_to(DELTA_CONTINUOUS, 0.0);
                p.line_to(0.0, height / 2.0);
                p.line_to(DELTA_CONTINUOUS, height);
                p.move_to(width - DELTA_CONTINUOUS, 0.0);
                p.line_to(width, height / 2.0);
                p.line_to(width - DELTA_CONTINUOUS, height);
                p.delta_shadow = shadowing;
                ug.draw(p);
            }
            _ => {
                let mut p = self.polygon(width, height);
                p.delta_shadow = shadowing;
                ug.draw(p);
            }
        }
    }

    fn polygon(self, w: f64, h: f64) -> UPolygon {
        let d = DELTA_INPUT_OUTPUT;
        let pts: Vec<(f64, f64)> = match self {
            BoxStyle::SdlInput | BoxStyle::UmlTrigger => {
                vec![(0.0, 0.0), (w + d, 0.0), (w, h / 2.0), (w + d, h), (0.0, h)]
            }
            BoxStyle::SdlOutput | BoxStyle::UmlSendSignal => {
                vec![(0.0, 0.0), (w, 0.0), (w + d, h / 2.0), (w, h), (0.0, h)]
            }
            BoxStyle::SdlLoad => vec![(0.0, 0.0), (w - d, 0.0), (w, h), (d, h)],
            BoxStyle::SdlSave => vec![(d, 0.0), (w, 0.0), (w - d, h), (0.0, h)],
            BoxStyle::UmlObjectSignal => vec![
                (-d, 0.0),
                (w, 0.0),
                (w + d, h / 2.0),
                (w, h),
                (-d, h),
                (0.0, h / 2.0),
            ],
            BoxStyle::UmlAcceptEvent => {
                vec![(-d, 0.0), (w, 0.0), (w, h), (-d, h), (0.0, h / 2.0)]
            }
            BoxStyle::UmlTimeEvent => {
                let half = w / 2.0;
                let third = h / 3.0;
                vec![
                    (half - third, third),
                    (half + third, third),
                    (half - third, h),
                    (half + third, h),
                ]
            }
            _ => unreachable!("not a polygon style"),
        };
        UPolygon::new(pts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_string_ignores_dashes_and_case() {
        assert_eq!(BoxStyle::from_string("SDL-Input"), BoxStyle::Plain);
        assert_eq!(BoxStyle::from_string("input"), BoxStyle::SdlInput);
        assert_eq!(
            BoxStyle::from_string("Accept-Event"),
            BoxStyle::UmlAcceptEvent
        );
        assert_eq!(BoxStyle::from_string("#red"), BoxStyle::Plain);
        assert_eq!(BoxStyle::SdlOutput.shield(), 10.0);
        assert_eq!(BoxStyle::Plain.shield(), 0.0);
        assert!(BoxStyle::Plain.stereotype().is_none());
        assert!(BoxStyle::SdlTask.stereotype().is_some());
    }
}
