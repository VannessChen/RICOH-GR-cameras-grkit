//! The color transforms of LUT import: input encodings, transfer functions, gamuts, 3x3
//! matrices, CIE L*a*b*, and CIEDE2000.

use anyhow::{Context, Result};
use std::str::FromStr;

pub type Matrix = [[f64; 3]; 3];
pub type Rgb = [f64; 3];

/// The declared encoding of a LUT's input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// The camera's Standard rendering: a LUT made for a GR JPEG in Standard.
    Standard,
    /// sRGB-encoded scene-linear RGB, clipped to 0..1, without a tone curve.
    Srgb,
    /// Scene-linear sRGB-primaries RGB.
    Linear,
    /// Panasonic V-Log over V-Gamut primaries.
    Vlog,
    /// Fujifilm F-Log over F-Gamut primaries.
    Flog,
    /// Fujifilm F-Log2 over F-Gamut primaries.
    Flog2,
    /// Fujifilm F-Log2 C: the F-Log2 curve over F-Gamut C primaries.
    Flog2C,
}

impl Encoding {
    pub const ALL: [Self; 7] = [
        Self::Standard,
        Self::Srgb,
        Self::Linear,
        Self::Vlog,
        Self::Flog,
        Self::Flog2,
        Self::Flog2C,
    ];

    /// Standard and sRGB: display-referred inputs, which the LUT data does not tell apart.
    pub fn is_display(self) -> bool {
        matches!(self, Self::Standard | Self::Srgb)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Srgb => "srgb",
            Self::Linear => "linear",
            Self::Vlog => "vlog",
            Self::Flog => "flog",
            Self::Flog2 => "flog2",
            Self::Flog2C => "flog2c",
        }
    }
}

impl FromStr for Encoding {
    type Err = anyhow::Error;
    fn from_str(name: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|e| e.name() == name)
            .context("Expected input encoding standard, srgb, linear, vlog, flog, flog2, or flog2c")
    }
}

pub fn srgb_encode(v: f64) -> f64 {
    if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

pub fn srgb_decode(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

// Panasonic V-Log / V-Gamut Reference Manual (rev. 1.0, 2014): reflection-domain curve,
// full-range normalized code values.
const VLOG_CUT1: f64 = 0.01;
const VLOG_CUT2: f64 = 0.181;
const VLOG_B: f64 = 0.00873;
const VLOG_C: f64 = 0.241514;
const VLOG_D: f64 = 0.598206;

pub fn vlog_encode(v: f64) -> f64 {
    if v < VLOG_CUT1 {
        5.6 * v + 0.125
    } else {
        VLOG_C * (v + VLOG_B).log10() + VLOG_D
    }
}

pub fn vlog_decode(v: f64) -> f64 {
    if v < VLOG_CUT2 {
        (v - 0.125) / 5.6
    } else {
        10f64.powf((v - VLOG_D) / VLOG_C) - VLOG_B
    }
}

// FUJIFILM F-Log Data Sheet (ver. 1.0) and F-Log2 Data Sheet (ver. 1.0): scene reflectance
// to full-range normalized code values.
const FLOG_A: f64 = 0.555556;
const FLOG_B: f64 = 0.009468;
const FLOG_C: f64 = 0.344676;
const FLOG_D: f64 = 0.790453;
const FLOG_E: f64 = 8.735631;
const FLOG_F: f64 = 0.092864;
const FLOG_CUT1: f64 = 0.00089;
const FLOG2_A: f64 = 5.555556;
const FLOG2_B: f64 = 0.064829;
const FLOG2_C: f64 = 0.245281;
const FLOG2_D: f64 = 0.384316;
const FLOG2_E: f64 = 8.799461;
const FLOG2_F: f64 = 0.092864;
const FLOG2_CUT1: f64 = 0.000889;

fn log_encode(v: f64, [a, b, c, d, e, f, cut]: [f64; 7]) -> f64 {
    if v < cut {
        e * v + f
    } else {
        c * (a * v + b).log10() + d
    }
}

fn log_decode(v: f64, [a, b, c, d, e, f, cut]: [f64; 7]) -> f64 {
    if v < e * cut + f {
        (v - f) / e
    } else {
        (10f64.powf((v - d) / c) - b) / a
    }
}

const FLOG: [f64; 7] = [FLOG_A, FLOG_B, FLOG_C, FLOG_D, FLOG_E, FLOG_F, FLOG_CUT1];
const FLOG2: [f64; 7] = [
    FLOG2_A, FLOG2_B, FLOG2_C, FLOG2_D, FLOG2_E, FLOG2_F, FLOG2_CUT1,
];

pub fn flog_encode(v: f64) -> f64 {
    log_encode(v, FLOG)
}

pub fn flog_decode(v: f64) -> f64 {
    log_decode(v, FLOG)
}

pub fn flog2_encode(v: f64) -> f64 {
    log_encode(v, FLOG2)
}

pub fn flog2_decode(v: f64) -> f64 {
    log_decode(v, FLOG2)
}

const D65: [f64; 2] = [0.3127, 0.3290];
const D50: [f64; 2] = [0.3457, 0.3585];
const SRGB_PRIMARIES: [[f64; 2]; 3] = [[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]];
const VGAMUT_PRIMARIES: [[f64; 2]; 3] = [[0.730, 0.280], [0.165, 0.840], [0.100, -0.030]];
/// F-Gamut uses the ITU-R BT.2020 primaries.
const FGAMUT_PRIMARIES: [[f64; 2]; 3] = [[0.708, 0.292], [0.170, 0.797], [0.131, 0.046]];
/// FUJIFILM F-Log2 C Data Sheet (ver. 1.0).
const FGAMUT_C_PRIMARIES: [[f64; 2]; 3] = [[0.7347, 0.2653], [0.0263, 0.9737], [0.1173, -0.0224]];

/// The rows of a row-major 3x3 matrix.
pub fn rows(m: &[f64; 9]) -> Matrix {
    std::array::from_fn(|r| std::array::from_fn(|c| m[r * 3 + c]))
}

pub fn mul(m: &Matrix, v: Rgb) -> Rgb {
    std::array::from_fn(|r| m[r][0] * v[0] + m[r][1] * v[1] + m[r][2] * v[2])
}

pub fn product(a: &Matrix, b: &Matrix) -> Matrix {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..3).map(|k| a[r][k] * b[k][c]).sum()))
}

pub fn inverse(m: &Matrix) -> Option<Matrix> {
    let cofactor = |r: usize, c: usize| {
        let (r1, r2, c1, c2) = ((r + 1) % 3, (r + 2) % 3, (c + 1) % 3, (c + 2) % 3);
        m[r1][c1] * m[r2][c2] - m[r1][c2] * m[r2][c1]
    };
    let determinant: f64 = (0..3).map(|c| m[0][c] * cofactor(0, c)).sum();
    (determinant.abs() > 1e-12)
        .then(|| std::array::from_fn(|r| std::array::from_fn(|c| cofactor(c, r) / determinant)))
}

fn white(xy: [f64; 2]) -> Rgb {
    [xy[0] / xy[1], 1.0, (1.0 - xy[0] - xy[1]) / xy[1]]
}

/// RGB to XYZ for chromaticity primaries and a white point (SMPTE RP 177).
fn rgb_to_xyz(primaries: &[[f64; 2]; 3], whitepoint: [f64; 2]) -> Matrix {
    let columns: Matrix = std::array::from_fn(|r| std::array::from_fn(|c| white(primaries[c])[r]));
    let scale = mul(&inverse(&columns).unwrap(), white(whitepoint));
    std::array::from_fn(|r| std::array::from_fn(|c| columns[r][c] * scale[c]))
}

pub fn srgb_to_xyz() -> Matrix {
    rgb_to_xyz(&SRGB_PRIMARIES, D65)
}

/// Both spaces use D65, so the conversion needs no chromatic adaptation.
pub fn srgb_to_vgamut() -> Matrix {
    product(
        &inverse(&rgb_to_xyz(&VGAMUT_PRIMARIES, D65)).unwrap(),
        &srgb_to_xyz(),
    )
}

pub fn vgamut_to_srgb() -> Matrix {
    inverse(&srgb_to_vgamut()).unwrap()
}

/// Both spaces use D65, so the conversion needs no chromatic adaptation.
pub fn srgb_to_fgamut() -> Matrix {
    product(
        &inverse(&rgb_to_xyz(&FGAMUT_PRIMARIES, D65)).unwrap(),
        &srgb_to_xyz(),
    )
}

pub fn fgamut_to_srgb() -> Matrix {
    inverse(&srgb_to_fgamut()).unwrap()
}

/// Both spaces use D65, so the conversion needs no chromatic adaptation.
pub fn srgb_to_fgamut_c() -> Matrix {
    product(
        &inverse(&rgb_to_xyz(&FGAMUT_C_PRIMARIES, D65)).unwrap(),
        &srgb_to_xyz(),
    )
}

pub fn fgamut_c_to_srgb() -> Matrix {
    inverse(&srgb_to_fgamut_c()).unwrap()
}

/// CIE L*a*b* D50 to linear sRGB with Bradford adaptation to D65.
pub fn lab_d50_to_srgb(lab: [f64; 3]) -> Rgb {
    const BRADFORD: Matrix = [
        [0.8951, 0.2664, -0.1614],
        [-0.7502, 1.7135, 0.0367],
        [0.0389, -0.0685, 1.0296],
    ];
    let finv = |t: f64| {
        if t > 6.0 / 29.0 {
            t.powi(3)
        } else {
            3.0 * (6.0f64 / 29.0).powi(2) * (t - 4.0 / 29.0)
        }
    };
    let fy = (lab[0] + 16.0) / 116.0;
    let source = white(D50);
    let xyz = [
        source[0] * finv(fy + lab[1] / 500.0),
        finv(fy),
        source[2] * finv(fy - lab[2] / 200.0),
    ];
    let (from, to) = (mul(&BRADFORD, source), mul(&BRADFORD, white(D65)));
    let gain: Matrix = std::array::from_fn(|r| {
        std::array::from_fn(|c| if r == c { to[r] / from[r] } else { 0.0 })
    });
    let adapt = product(&inverse(&BRADFORD).unwrap(), &product(&gain, &BRADFORD));
    mul(&inverse(&srgb_to_xyz()).unwrap(), mul(&adapt, xyz))
}

/// CIE L*a*b* (D65) of sRGB-encoded display RGB, clipped to 0..1.
pub struct LabConverter(Matrix);

impl Default for LabConverter {
    fn default() -> Self {
        Self(srgb_to_xyz())
    }
}

impl LabConverter {
    pub fn lab(&self, display: Rgb) -> [f64; 3] {
        let xyz = mul(&self.0, display.map(|v| srgb_decode(v.clamp(0.0, 1.0))));
        let reference = white(D65);
        let f = |t: f64| {
            if t > (6.0f64 / 29.0).powi(3) {
                t.cbrt()
            } else {
                t / (3.0 * (6.0f64 / 29.0).powi(2)) + 4.0 / 29.0
            }
        };
        let [fx, fy, fz] = std::array::from_fn(|c| f(xyz[c] / reference[c]));
        [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
    }
}

/// CIEDE2000 with unit weights (Sharma, Wu, and Dalal 2005).
pub fn delta_e2000(a: [f64; 3], b: [f64; 3]) -> f64 {
    use std::f64::consts::PI;
    let c1 = a[1].hypot(a[2]);
    let c2 = b[1].hypot(b[2]);
    let mean_c7 = ((c1 + c2) / 2.0).powi(7);
    let g = 0.5 * (1.0 - (mean_c7 / (mean_c7 + 25f64.powi(7))).sqrt());
    let (a1, a2) = (a[1] * (1.0 + g), b[1] * (1.0 + g));
    let (c1, c2) = (a1.hypot(a[2]), a2.hypot(b[2]));
    let hue = |y: f64, x: f64| {
        if x == 0.0 && y == 0.0 {
            0.0
        } else {
            y.atan2(x).rem_euclid(2.0 * PI)
        }
    };
    let (h1, h2) = (hue(a[2], a1), hue(b[2], a2));
    let delta_l = b[0] - a[0];
    let delta_c = c2 - c1;
    let mut delta_h = h2 - h1;
    if c1 * c2 == 0.0 {
        delta_h = 0.0;
    } else if delta_h > PI {
        delta_h -= 2.0 * PI;
    } else if delta_h < -PI {
        delta_h += 2.0 * PI;
    }
    let delta_hh = 2.0 * (c1 * c2).sqrt() * (delta_h / 2.0).sin();
    let mean_l = (a[0] + b[0]) / 2.0;
    let mean_c = (c1 + c2) / 2.0;
    let mean_h = if c1 * c2 == 0.0 {
        h1 + h2
    } else if (h1 - h2).abs() <= PI {
        (h1 + h2) / 2.0
    } else if h1 + h2 < 2.0 * PI {
        (h1 + h2 + 2.0 * PI) / 2.0
    } else {
        (h1 + h2 - 2.0 * PI) / 2.0
    };
    let t = 1.0 - 0.17 * (mean_h - PI / 6.0).cos()
        + 0.24 * (2.0 * mean_h).cos()
        + 0.32 * (3.0 * mean_h + PI / 30.0).cos()
        - 0.20 * (4.0 * mean_h - 63f64.to_radians()).cos();
    let delta_theta = 30f64.to_radians() * (-((mean_h.to_degrees() - 275.0) / 25.0).powi(2)).exp();
    let mean_c7 = mean_c.powi(7);
    let rc = 2.0 * (mean_c7 / (mean_c7 + 25f64.powi(7))).sqrt();
    let sl = 1.0 + 0.015 * (mean_l - 50.0).powi(2) / (20.0 + (mean_l - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * mean_c;
    let sh = 1.0 + 0.015 * mean_c * t;
    let rt = -(2.0 * delta_theta).sin() * rc;
    ((delta_l / sl).powi(2)
        + (delta_c / sc).powi(2)
        + (delta_hh / sh).powi(2)
        + rt * (delta_c / sc) * (delta_hh / sh))
        .sqrt()
}
