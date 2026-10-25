//! HUSL (HSLuv) color conversion, ported from `klimt/color/HUSLColorConverter`
//! (itself taken from hsluv-java by Alexei Boronine), plus `ColorUtils.reverseHsluv`.
//!
//! RGB components are in `0.0..=1.0`. HSLuv components are `[H, S, L]` with
//! H in degrees and S, L in `0..=100`.

use std::f64::consts::PI;

const M: [[f64; 3]; 3] = [
    [3.240969941904521, -1.537383177570093, -0.498610760293],
    [-0.96924363628087, 1.87596750150772, 0.041555057407175],
    [0.055630079696993, -0.20397695888897, 1.056971514242878],
];

const MINV: [[f64; 3]; 3] = [
    [0.41239079926595, 0.35758433938387, 0.18048078840183],
    [0.21263900587151, 0.71516867876775, 0.072192315360733],
    [0.019330818715591, 0.11919477979462, 0.95053215224966],
];

const REF_Y: f64 = 1.0;
const REF_U: f64 = 0.19783000664283;
const REF_V: f64 = 0.46831999493879;
const KAPPA: f64 = 903.2962962;
const EPSILON: f64 = 0.0088564516;

fn dot(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a.iter().zip(b).fold(0.0, |sum, (x, y)| sum + x * y)
}

/// Bounding lines `[slope, intercept]` of the RGB gamut at lightness `l`.
fn bounds(l: f64) -> [[f64; 2]; 6] {
    let sub1 = (l + 16.0).powf(3.0) / 1560896.0;
    let sub2 = if sub1 > EPSILON { sub1 } else { l / KAPPA };

    let mut result = [[0.0; 2]; 6];
    let mut idx = 0;
    for m in M.iter() {
        let (m1, m2, m3) = (m[0], m[1], m[2]);
        for t in 0..2 {
            let t = t as f64;
            let top1 = (284517.0 * m1 - 94839.0 * m3) * sub2;
            let top2 =
                (838422.0 * m3 + 769860.0 * m2 + 731718.0 * m1) * l * sub2 - 769860.0 * t * l;
            let bottom = (632260.0 * m3 - 126452.0 * m2) * sub2 + 126452.0 * t;
            result[idx] = [top1 / bottom, top2 / bottom];
            idx += 1;
        }
    }
    result
}

/// Largest chroma inside the gamut for lightness `l` and hue `h` (degrees).
fn max_chroma_for_lh(l: f64, h: f64) -> f64 {
    let hrad = h / 360.0 * 2.0 * PI;
    let mut min = f64::MAX;
    for line in bounds(l) {
        let length = line[1] / (hrad.sin() - line[0] * hrad.cos());
        if length >= 0.0 {
            min = min.min(length);
        }
    }
    min
}

fn from_linear(c: f64) -> f64 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn to_linear(c: f64) -> f64 {
    if c > 0.04045 {
        ((c + 0.055) / (1.0 + 0.055)).powf(2.4)
    } else {
        c / 12.92
    }
}

fn xyz_to_rgb(xyz: [f64; 3]) -> [f64; 3] {
    [
        from_linear(dot(&M[0], &xyz)),
        from_linear(dot(&M[1], &xyz)),
        from_linear(dot(&M[2], &xyz)),
    ]
}

fn rgb_to_xyz(rgb: [f64; 3]) -> [f64; 3] {
    let rgbl = [to_linear(rgb[0]), to_linear(rgb[1]), to_linear(rgb[2])];
    [
        dot(&MINV[0], &rgbl),
        dot(&MINV[1], &rgbl),
        dot(&MINV[2], &rgbl),
    ]
}

fn y_to_l(y: f64) -> f64 {
    if y <= EPSILON {
        (y / REF_Y) * KAPPA
    } else {
        116.0 * (y / REF_Y).powf(1.0 / 3.0) - 16.0
    }
}

fn l_to_y(l: f64) -> f64 {
    if l <= 8.0 {
        REF_Y * l / KAPPA
    } else {
        REF_Y * ((l + 16.0) / 116.0).powf(3.0)
    }
}

fn xyz_to_luv(xyz: [f64; 3]) -> [f64; 3] {
    let [x, y, z] = xyz;
    let var_u = (4.0 * x) / (x + (15.0 * y) + (3.0 * z));
    let var_v = (9.0 * y) / (x + (15.0 * y) + (3.0 * z));

    let l = y_to_l(y);
    if l == 0.0 {
        return [0.0, 0.0, 0.0];
    }
    [l, 13.0 * l * (var_u - REF_U), 13.0 * l * (var_v - REF_V)]
}

fn luv_to_xyz(luv: [f64; 3]) -> [f64; 3] {
    let [l, u, v] = luv;
    if l == 0.0 {
        return [0.0, 0.0, 0.0];
    }
    let var_u = u / (13.0 * l) + REF_U;
    let var_v = v / (13.0 * l) + REF_V;

    let y = l_to_y(l);
    let x = 0.0 - (9.0 * y * var_u) / ((var_u - 4.0) * var_v - var_u * var_v);
    let z = (9.0 * y - (15.0 * var_v * y) - (var_v * x)) / (3.0 * var_v);
    [x, y, z]
}

fn luv_to_lch(luv: [f64; 3]) -> [f64; 3] {
    let [l, u, v] = luv;
    let c = (u * u + v * v).sqrt();
    let h = if c < 0.00000001 {
        0.0
    } else {
        let hrad = v.atan2(u);
        let h = (hrad * 180.0) / PI;
        if h < 0.0 {
            360.0 + h
        } else {
            h
        }
    };
    [l, c, h]
}

fn lch_to_luv(lch: [f64; 3]) -> [f64; 3] {
    let [l, c, h] = lch;
    let hrad = h / 360.0 * 2.0 * PI;
    [l, hrad.cos() * c, hrad.sin() * c]
}

fn hsluv_to_lch(hsluv: [f64; 3]) -> [f64; 3] {
    let [h, s, l] = hsluv;
    if l > 99.9999999 {
        return [100.0, 0.0, h];
    }
    if l < 0.00000001 {
        return [0.0, 0.0, h];
    }
    let max = max_chroma_for_lh(l, h);
    [l, max / 100.0 * s, h]
}

fn lch_to_hsluv(lch: [f64; 3]) -> [f64; 3] {
    let [l, c, h] = lch;
    if l > 99.9999999 {
        return [h, 0.0, 100.0];
    }
    if l < 0.00000001 {
        return [h, 0.0, 0.0];
    }
    let max = max_chroma_for_lh(l, h);
    [h, c / max * 100.0, l]
}

fn rgb_to_lch(rgb: [f64; 3]) -> [f64; 3] {
    luv_to_lch(xyz_to_luv(rgb_to_xyz(rgb)))
}

fn lch_to_rgb(lch: [f64; 3]) -> [f64; 3] {
    xyz_to_rgb(luv_to_xyz(lch_to_luv(lch)))
}

/// RGB (0.0..=1.0) to HSLuv `[H, S, L]`.
pub fn rgb_to_hsluv(rgb: [f64; 3]) -> [f64; 3] {
    lch_to_hsluv(rgb_to_lch(rgb))
}

/// HSLuv `[H, S, L]` to RGB (0.0..=1.0).
pub fn hsluv_to_rgb(hsluv: [f64; 3]) -> [f64; 3] {
    lch_to_rgb(hsluv_to_lch(hsluv))
}

/// `ColorUtils.to255`: truncating conversion clamped to `0..=255`.
fn to255(value: f64) -> u8 {
    ((255.0 * value) as i32).clamp(0, 255) as u8
}

/// `ColorUtils.reverseHsluv`: lightness is shifted by 50 (wrapped) and
/// partially pulled toward 50, keeping hue and saturation.
pub fn reverse_hsluv(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let hsluv = rgb_to_hsluv([r as f64 / 256.0, g as f64 / 256.0, b as f64 / 256.0]);

    let h = hsluv[0];
    let s = hsluv[1];
    let mut l = (hsluv[2] + 50.0) % 100.0;
    l += 0.25 * (50.0 - l);

    let rgb = hsluv_to_rgb([h, s, l]);
    (to255(rgb[0]), to255(rgb[1]), to255(rgb[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_rgb_hsluv_rgb() {
        let colors = [
            [0.2, 0.4, 0.6],
            [0.9, 0.1, 0.3],
            [0.1, 0.8, 0.2],
            [0.5, 0.5, 0.5],
            [0.25, 0.05, 0.75],
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
        ];
        for rgb in colors {
            let back = hsluv_to_rgb(rgb_to_hsluv(rgb));
            for i in 0..3 {
                assert!((back[i] - rgb[i]).abs() < 1e-6, "{rgb:?} -> {back:?}");
            }
        }
    }

    #[test]
    fn hsluv_extremes() {
        let white = rgb_to_hsluv([1.0, 1.0, 1.0]);
        assert!((white[2] - 100.0).abs() < 1e-9);
        assert!(white[1].abs() < 1e-9);
        let black = rgb_to_hsluv([0.0, 0.0, 0.0]);
        assert!(black[2].abs() < 1e-9);
    }

    #[test]
    fn reverse_hsluv_gray_levels() {
        // White: L = 100 -> (100 + 50) % 100 = 50 -> 50 + 0.25 * 0 = 50 (mid gray, ~118).
        let (r, g, b) = reverse_hsluv(255, 255, 255);
        assert_eq!(r, g);
        assert_eq!(g, b);
        assert!((117..=119).contains(&r), "white -> {r}");

        // Black: L = 0 -> 50 -> same mid gray.
        let (r, g, b) = reverse_hsluv(0, 0, 0);
        assert_eq!(r, g);
        assert_eq!(g, b);
        assert!((117..=119).contains(&r), "black -> {r}");

        // Mid gray: L ~ 53.2 -> (103.2 % 100) = 3.2 -> 3.2 + 0.25 * 46.8 = 14.9 (dark gray).
        let (r, g, b) = reverse_hsluv(128, 128, 128);
        assert_eq!(r, g);
        assert_eq!(g, b);
        assert!((30..=40).contains(&r), "gray -> {r}");
    }

    #[test]
    fn reverse_hsluv_matches_formula() {
        for &(r, g, b) in &[(200u8, 30u8, 90u8), (10, 220, 130), (128, 128, 128)] {
            let hsluv = rgb_to_hsluv([r as f64 / 256.0, g as f64 / 256.0, b as f64 / 256.0]);
            let mut l = (hsluv[2] + 50.0) % 100.0;
            l += 0.25 * (50.0 - l);
            let rgb = hsluv_to_rgb([hsluv[0], hsluv[1], l]);
            let expected = (to255(rgb[0]), to255(rgb[1]), to255(rgb[2]));
            assert_eq!(reverse_hsluv(r, g, b), expected);
        }
    }

    #[test]
    fn to255_truncates_and_clamps() {
        assert_eq!(to255(0.5), 127);
        assert_eq!(to255(-0.2), 0);
        assert_eq!(to255(1.5), 255);
    }
}
