use super::*;
use crate::klimt::drawing::svg::SvgOption;
use crate::klimt::drawing::ugraphic::render_svg;
use crate::klimt::font::EmbeddedFontBounder;
use std::f64::consts::{FRAC_PI_2, PI};
use std::rc::Rc;

const ALL_DECORS: [LinkDecor; 25] = [
    LinkDecor::None,
    LinkDecor::Extends,
    LinkDecor::Composition,
    LinkDecor::Aggregation,
    LinkDecor::NotNavigable,
    LinkDecor::Redefines,
    LinkDecor::DefinedBy,
    LinkDecor::Crowfoot,
    LinkDecor::CircleCrowfoot,
    LinkDecor::CircleLine,
    LinkDecor::DoubleLine,
    LinkDecor::LineCrowfoot,
    LinkDecor::Arrow,
    LinkDecor::ArrowTriangle,
    LinkDecor::ArrowAndCircle,
    LinkDecor::Circle,
    LinkDecor::CircleFill,
    LinkDecor::CircleConnect,
    LinkDecor::Parenthesis,
    LinkDecor::Square,
    LinkDecor::CircleCross,
    LinkDecor::Plus,
    LinkDecor::HalfArrowUp,
    LinkDecor::HalfArrowDown,
    LinkDecor::SquareToBeRemoved,
];

fn svg(f: impl FnOnce(&UGraphic)) -> String {
    render_svg(
        SvgOption::default(),
        0,
        Rc::new(EmbeddedFontBounder),
        |ug| {
            f(&ug
                .apply_color(HColor::BLACK)
                .apply_back_color(HColor::WHITE))
        },
    )
}

fn draw_ext(decor: LinkDecor, angle: f64, side: Option<Side>) -> Option<String> {
    let f = extremity_factory_complete(decor, &HColor::WHITE)?;
    Some(svg(|ug| {
        let e = f.create((50.0, 40.0), angle, side);
        assert!(e.some_point().is_some());
        e.draw(ug);
        e.draw_line_if_transparent(ug);
        let _ = (e.delta_for_kal(), e.decoration_length());
    }))
}

#[test]
fn every_decor_draws_at_all_angles() {
    for decor in ALL_DECORS {
        for angle in [0.0, 0.7, FRAC_PI_2, 3.1, 4.7, 6.0] {
            for side in [None, Some(Side::North), Some(Side::West)] {
                match draw_ext(decor, angle, side) {
                    Some(out) => assert!(!out.is_empty(), "{decor:?}"),
                    None => assert!(
                        matches!(decor, LinkDecor::None | LinkDecor::SquareToBeRemoved),
                        "{decor:?}"
                    ),
                }
            }
        }
    }
}

#[test]
fn arrow_geometry() {
    let out = draw_ext(LinkDecor::Arrow, 0.0, None).unwrap();
    assert!(out.contains("<polygon"), "{out}");
    assert!(out.contains("50,40"), "{out}");
    assert!(out.contains("41,36"), "{out}");
}

#[test]
fn extends_triangle_decoration_length() {
    let f = extremity_factory_complete(LinkDecor::Extends, &HColor::WHITE).unwrap();
    assert_eq!(f.create((0.0, 0.0), 0.0, None).decoration_length(), 18.0);
    let f = extremity_factory_complete(LinkDecor::ArrowTriangle, &HColor::WHITE).unwrap();
    assert_eq!(f.create((0.0, 0.0), 0.0, None).decoration_length(), 8.0);
}

#[test]
fn diamond_delta_for_kal() {
    let f = extremity_factory_complete(LinkDecor::Composition, &HColor::WHITE).unwrap();
    let e = f.create((10.0, 10.0), 0.0, None);
    let d = e.delta_for_kal();
    assert!((d.dx.abs() - 12.0).abs() < 1e-9 || (d.dy.abs() - 12.0).abs() < 1e-9);
    assert_eq!(e.decoration_length(), 12.0);
}

#[test]
fn circle_cross_has_two_lines() {
    let out = draw_ext(LinkDecor::CircleCross, 0.0, None).unwrap();
    assert_eq!(out.matches("<line").count(), 2, "{out}");
    assert!(out.contains("<ellipse"), "{out}");
}

#[test]
fn crowfoot_sides_flatten_wings() {
    let n = draw_ext(LinkDecor::Crowfoot, 1.0, Some(Side::North)).unwrap();
    let w = draw_ext(LinkDecor::Crowfoot, 1.0, Some(Side::West)).unwrap();
    assert_ne!(n, w);
}

#[test]
fn half_arrow_up_and_down_differ() {
    let up = draw_ext(LinkDecor::HalfArrowUp, 0.3, None).unwrap();
    let down = draw_ext(LinkDecor::HalfArrowDown, 0.3, None).unwrap();
    assert_ne!(up, down);
    assert_eq!(up.matches("<line").count(), 2);
}

#[test]
fn extends_like_and_extends() {
    let bg = HColor::WHITE;
    let red = extremity_factory_complete(LinkDecor::Redefines, &bg).unwrap();
    let out = svg(|ug| red.create((30.0, 30.0), 1.0, None).draw(ug));
    assert!(out.contains("<polygon") && out.contains("<line"), "{out}");
    let def = extremity_factory_complete(LinkDecor::DefinedBy, &bg).unwrap();
    let out = svg(|ug| def.create((30.0, 30.0), 1.0, None).draw(ug));
    assert_eq!(out.matches("<ellipse").count(), 2, "{out}");
    let ext = ExtremityFactoryExtends::new(bg);
    let out = svg(|ug| ext.create((30.0, 30.0), 1.0, None).draw(ug));
    assert!(out.contains("<polygon"), "{out}");
}

#[test]
fn not_navigable_is_a_path() {
    let out = draw_ext(LinkDecor::NotNavigable, 0.0, None).unwrap();
    assert!(out.contains("<path"), "{out}");
}

#[test]
fn middle_decors_draw() {
    let b = HColor::WHITE;
    let d = HColor::BLACK;
    for (m, shapes) in [
        (LinkMiddleDecor::Circle, 1),
        (LinkMiddleDecor::CircleCircled, 4),
        (LinkMiddleDecor::CircleCircled1, 2),
        (LinkMiddleDecor::CircleCircled2, 2),
        (LinkMiddleDecor::None, 0),
    ] {
        let f = middle_factory(m, &b, &d);
        let out = svg(|ug| {
            f.create(30.0)
                .draw(&ug.apply_translate(UTranslate::new(20.0, 20.0)))
        });
        let n = out.matches("<ellipse").count() + out.matches("<path").count();
        assert_eq!(n, shapes, "{m:?}: {out}");
    }
    for m in [LinkMiddleDecor::Subset, LinkMiddleDecor::Superset] {
        let f = middle_factory(m, &b, &d);
        let out = svg(|ug| f.create(30.0).draw(ug));
        assert_eq!(out.matches("<line").count(), 2, "{m:?}: {out}");
    }
}

#[test]
fn manage_round_snaps() {
    assert_eq!(manage_round(0.0001), 0.0);
    assert_eq!(manage_round(PI + 0.0001), PI);
    assert_eq!(manage_round(2.0 * PI), 0.0);
    assert_eq!(manage_round(1.0), 1.0);
}
