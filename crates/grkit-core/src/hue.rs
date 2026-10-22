//! Hue sectors: per-effect parameters of the camera's MultiAxial color stage.
//!
//! The stage acts on linear RGB after the color matrix, before the base gamma. It computes
//! Y, Cb, and Cr with its own matrix (`STAGE`), finds the hue sector of (Cb, Cr), and mixes
//! the color toward that sector's 3x3 matrix by a weight that follows Y:
//! `out = v + w(Y) (B v - v)`. The sectors are 11 ranges of the stage's hue coordinate, and
//! each matrix row sums to 1, so gray stays gray.
//!
//! The hue coordinate splits the Cb/Cr plane into four quadrants. Within a quadrant it is
//! `2048 b / (a + b)`, where `a` and `b` are the chroma components along the quadrant's first
//! and second axis. Going round: +Cr (red) to +Cb (blue), to -Cr (green), to -Cb (yellow),
//! back to +Cr. The hardware holds four descending bounds and five bank selectors for each
//! quadrant; `BOUNDS` and `SELECTORS` give 11 equal sectors of the joined coordinate 0..8192,
//! sector k in bank k + 1. Bank 0 is the identity.
//!
//! Each bank holds five matrices, the levels. The stage picks a position between them from
//! Y and blends the two neighbouring levels. Control fields 1..8 set that choice: `LEVEL_CONTROL`
//! spreads the levels from mid gray to the highlights along `LEVEL_CURVE`, which camera
//! photos measured. Without levels, the fields keep every color on level 0.
//!
//! An effect stores each sector as six i8 off-diagonal coefficients in 1/128 (row r: columns
//! r + 1 and r + 2, mod 3) and the weight as four u8 values in 1/255 at the luminance knots
//! `KNOTS`. The GRF stores the weight as the hardware control fields 10..20. An effect with
//! levels also stores the sectors of levels 1 to 3; level 4 repeats level 3, so colors above
//! the measured range need no curve.
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

use crate::cube::{self, Rgb};

pub const SECTORS: usize = 11;
/// GRF bytes: the matrices, then 11 u16 control fields.
pub const SIZE: usize = SECTORS * 6 + 11 * 2;
/// Levels with their own matrices. The hardware's level 4 repeats level 3.
pub const LEVELS: usize = 4;
/// GRF bytes of levels 1 to 3.
pub const LEVEL_SIZE: usize = (LEVELS - 1) * SECTORS * 6;
/// Control fields 1..8 for an effect with levels: four gains, then four borders.
pub const LEVEL_CONTROL: [u16; 8] = [656, 656, 656, 586, 150, 650, 1150, 1650];
/// Control fields 1..8 for a capture with highlight correction, which halves the stage's
/// input: chart photos found these closest to `LEVEL_CURVE` at twice the luminance
/// (0.15 level RMS).
pub const LEVEL_CONTROL_HALVED: [u16; 8] = [1312, 1312, 1312, 1172, 75, 325, 575, 825];
/// The level position (0..3) at the stage's luminance with `LEVEL_CONTROL`, measured on
/// GR III photos of a chart, between the knots linearly. Above the last knot the position
/// stays 3 because level 4 repeats level 3.
pub const LEVEL_CURVE: [(f64, f64); 12] = [
    (0.040, 0.0),
    (0.055, 0.08),
    (0.072, 0.20),
    (0.093, 0.37),
    (0.121, 0.62),
    (0.158, 1.08),
    (0.205, 1.30),
    (0.266, 1.41),
    (0.346, 1.79),
    (0.450, 2.29),
    (0.585, 2.90),
    (0.660, 3.0),
];

/// The level position (0..=3) at the stage's luminance `y`.
pub fn level_position(y: f64) -> f64 {
    let first = LEVEL_CURVE[0];
    let last = LEVEL_CURVE[LEVEL_CURVE.len() - 1];
    if y <= first.0 {
        return first.1;
    }
    if y >= last.0 {
        return last.1;
    }
    let k = LEVEL_CURVE.windows(2).position(|w| y < w[1].0).unwrap();
    let (a, b) = (LEVEL_CURVE[k], LEVEL_CURVE[k + 1]);
    a.1 + (y - a.0) / (b.0 - a.0) * (b.1 - a.1)
}

/// The two levels around a position and the share of the upper one.
pub fn level_blend(position: f64) -> (usize, usize, f64) {
    let lower = (position.floor() as usize).min(LEVELS - 1);
    let upper = (lower + 1).min(LEVELS - 1);
    (lower, upper, position - lower as f64)
}
/// The weight curve's x for the stage's luminance Y: x = WEIGHT_SCALE Y. GR III photos of
/// a chart through a weight ramp measured 2950 to 3050.
pub const WEIGHT_SCALE: f64 = 3000.0;
/// The breakpoints of control fields 18..20 in x.
pub const BORDERS: [u16; 3] = [164, 655, 4095];
/// Luminance knots of the weight curve, Y of the stage on linear RGB: 0 and `BORDERS`.
pub const KNOTS: [f64; 4] = [
    0.0,
    BORDERS[0] as f64 / WEIGHT_SCALE,
    BORDERS[1] as f64 / WEIGHT_SCALE,
    BORDERS[2] as f64 / WEIGHT_SCALE,
];
/// The stage's RGB to YCbCr matrix, 1.0 = 256, as Standard's parameter set holds it.
pub const STAGE: [[f64; 3]; 3] = [
    [77.0, 150.0, 29.0],
    [-43.0, -85.0, 128.0],
    [128.0, -107.0, -21.0],
];
/// The hardware quadrant of each quarter of the joined coordinate.
const QUADRANT_AT: [usize; 4] = [0, 3, 2, 1];
/// Four descending bounds per hardware quadrant (12-bit), for 11 equal sectors.
pub const BOUNDS: [u16; 16] = [
    1490, 745, 0, 0, 1304, 559, 0, 0, 1862, 1118, 373, 0, 1676, 931, 187, 0,
];
/// The bank of each of the five segments of a hardware quadrant.
pub const SELECTORS: [u8; 20] = [3, 2, 1, 0, 0, 11, 10, 9, 0, 0, 9, 8, 7, 6, 0, 6, 5, 4, 3, 0];

/// The joined hue coordinate 0..8192 of a linear color, or `None` for gray.
pub fn coordinate(v: Rgb) -> Option<f64> {
    let cb = (STAGE[1][0] * v[0] + STAGE[1][1] * v[1] + STAGE[1][2] * v[2]) / 256.0;
    let cr = (STAGE[2][0] * v[0] + STAGE[2][1] * v[1] + STAGE[2][2] * v[2]) / 256.0;
    let (quarter, a, b) = if cr >= 0.0 {
        if cb >= 0.0 {
            (0, cr, cb)
        } else {
            (3, -cb, cr)
        }
    } else if cb >= 0.0 {
        (1, cb, -cr)
    } else {
        (2, -cr, -cb)
    };
    (a + b > 1e-12).then(|| quarter as f64 * 2048.0 + 2048.0 * b / (a + b))
}

/// The levels with a share of a color of stage luminance `y`, among `levels` levels (one or
/// `LEVELS`), and their shares.
pub fn level_shares(levels: usize, y: f64) -> Vec<(usize, f64)> {
    if levels == 1 {
        return vec![(0, 1.0)];
    }
    let (a, b, f) = level_blend(level_position(y));
    if a == b || f == 0.0 {
        vec![(a, 1.0)]
    } else if f == 1.0 {
        vec![(b, 1.0)]
    } else {
        vec![(a, 1.0 - f), (b, f)]
    }
}

/// The unit (Cb, Cr) direction at a joined coordinate, the inverse of `coordinate`.
pub fn direction(coordinate: f64) -> (f64, f64) {
    let coordinate = coordinate.rem_euclid(8192.0);
    let quarter = ((coordinate / 2048.0) as usize).min(3);
    let t = (coordinate - 2048.0 * quarter as f64) / 2048.0;
    let (a, b) = (1.0 - t, t);
    let (cb, cr) = match quarter {
        0 => (b, a),
        1 => (a, -b),
        2 => (-b, -a),
        _ => (-a, b),
    };
    let norm = cb.hypot(cr);
    (cb / norm, cr / norm)
}

/// The bank (0..=11) that the hardware picks at a joined coordinate.
pub fn bank_at(coordinate: f64) -> usize {
    let quarter = ((coordinate / 2048.0) as usize).min(3);
    let q = QUADRANT_AT[quarter];
    let x = (coordinate - 2048.0 * quarter as f64).floor();
    let n = BOUNDS[q * 4..q * 4 + 4]
        .iter()
        .position(|b| x >= f64::from(*b))
        .unwrap_or(4);
    SELECTORS[5 * q + n] as usize
}

/// The sector (0..SECTORS) whose matrix the stage applies to a linear color, or `None` for
/// gray and bank 0.
pub fn sector(v: Rgb) -> Option<usize> {
    coordinate(v)
        .map(bank_at)
        .and_then(|bank| bank.checked_sub(1))
}

/// The stage's luminance of a linear color.
pub fn luminance(v: Rgb) -> f64 {
    (STAGE[0][0] * v[0] + STAGE[0][1] * v[1] + STAGE[0][2] * v[2]) / 256.0
}

/// The 3x3 matrix (row-major) of six off-diagonal coefficients; each row sums to 1.
pub fn matrix(offsets: &[f64]) -> [f64; 9] {
    let mut m = [0.0; 9];
    for r in 0..3 {
        m[r * 3 + (r + 1) % 3] = offsets[r * 2];
        m[r * 3 + (r + 2) % 3] = offsets[r * 2 + 1];
        m[r * 4] = 1.0 - offsets[r * 2] - offsets[r * 2 + 1];
    }
    m
}

/// The weight curve in the hardware's form: start weight and slope (per 32 units of x,
/// 1.0 = 1024) for each segment from the breakpoints 0, `BORDERS`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curve {
    pub starts: [f64; 4],
    pub slopes: [f64; 4],
}

impl Curve {
    /// The curve through weights `w` (0..1) at `KNOTS`, without rounding.
    pub fn through(w: [f64; 4]) -> Self {
        let starts = w.map(|v| v * 1024.0);
        Self {
            starts,
            slopes: slopes(starts),
        }
    }

    /// The weight at the stage's luminance `y`.
    pub fn weight(&self, y: f64) -> f64 {
        let x = (WEIGHT_SCALE * y).clamp(0.0, 4095.0);
        let breaks = [
            0.0,
            f64::from(BORDERS[0]),
            f64::from(BORDERS[1]),
            f64::from(BORDERS[2]),
        ];
        let k = (0..4).rev().find(|&k| x >= breaks[k]).unwrap_or(0);
        ((self.starts[k] + self.slopes[k] * (x - breaks[k]) / 32.0) / 1024.0).clamp(0.0, 1.0)
    }
}

fn slopes(starts: [f64; 4]) -> [f64; 4] {
    let breaks = [
        0.0,
        f64::from(BORDERS[0]),
        f64::from(BORDERS[1]),
        f64::from(BORDERS[2]),
    ];
    std::array::from_fn(|k| {
        if k < 3 {
            (starts[k + 1] - starts[k]) * 32.0 / (breaks[k + 1] - breaks[k])
        } else {
            0.0
        }
    })
}

/// The stage of an effect for the model: sector matrices and the weight curve.
#[derive(Clone, Debug, PartialEq)]
pub struct Stage {
    /// The sector matrices of each level: one entry without levels, `LEVELS` with them.
    pub levels: Vec<[[f64; 9]; SECTORS]>,
    pub curve: Curve,
}

impl Stage {
    /// The stage on a linear color after the color matrix.
    pub fn apply(&self, v: Rgb) -> Rgb {
        let Some(k) = sector(v) else {
            return v;
        };
        let y = luminance(v);
        let w = self.curve.weight(y);
        let m = self.matrix(k, y);
        let mixed = cube::mul(&cube::rows(&m), v);
        std::array::from_fn(|c| (v[c] + w * (mixed[c] - v[c])).max(0.0))
    }

    /// The matrix of sector `k` at the stage's luminance `y`.
    pub fn matrix(&self, k: usize, y: f64) -> [f64; 9] {
        if self.levels.len() == 1 {
            return self.levels[0][k];
        }
        let (a, b, f) = level_blend(level_position(y));
        let (ma, mb) = (self.levels[a][k], self.levels[b][k]);
        std::array::from_fn(|n| ma[n] + f * (mb[n] - ma[n]))
    }
}

/// Six off-diagonal coefficients in 1/128 for each sector.
pub type Sectors = [[i8; 6]; SECTORS];

/// The stored form of an effect's hue sectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HueSectors {
    /// The sectors of level 0, or of every level without `levels`.
    pub matrices: Sectors,
    /// The weight in 1/255 at `KNOTS`.
    pub weight: [u8; 4],
    /// The sectors of levels 1 to 3.
    pub levels: Option<[Sectors; LEVELS - 1]>,
}

impl HueSectors {
    /// The values that the stage uses, as the camera rounds them.
    pub fn stage(&self) -> Stage {
        let unit = |sectors: &Sectors| sectors.map(|m| matrix(&m.map(|v| f64::from(v) / 128.0)));
        let mut levels = vec![unit(&self.matrices)];
        if let Some(upper) = &self.levels {
            levels.extend(upper.iter().map(unit));
        }
        let fields = self.control();
        let signed = |v: u16| {
            f64::from(if v & 0x100 != 0 {
                v as i32 - 0x200
            } else {
                v as i32
            })
        };
        Stage {
            levels,
            curve: Curve {
                starts: std::array::from_fn(|k| f64::from(fields[k])),
                slopes: std::array::from_fn(|k| signed(fields[4 + k])),
            },
        }
    }

    /// Control fields 10..20 of the stage: start weights (1.0 = 1024), 9-bit slopes per 32
    /// units of x, and the breakpoints.
    pub fn control(&self) -> [u16; 11] {
        let starts = self.weight.map(|w| (f64::from(w) * 1024.0 / 255.0).round());
        let slopes = slopes(starts);
        let mut fields = [0u16; 11];
        for k in 0..4 {
            fields[k] = starts[k] as u16;
            fields[4 + k] = (slopes[k].round() as i32 & 0x1ff) as u16;
        }
        fields[8..].copy_from_slice(&BORDERS);
        fields
    }

    pub fn from_json(value: &Value) -> Result<Self> {
        let message = "Expected hue_sectors with 11 matrices of six coefficients in 1/128, four weights 0..255, and optional levels of three such matrix sets";
        let object = value.as_object().context(message)?;
        ensure!(
            object.contains_key("matrices")
                && object.contains_key("weight")
                && object
                    .keys()
                    .all(|k| ["matrices", "weight", "levels"].contains(&k.as_str())),
            "{message}"
        );
        let sectors = |value: &Value| -> Result<Sectors> {
            let matrices = value
                .as_array()
                .filter(|m| m.len() == SECTORS)
                .context(message)?;
            let mut out = [[0; 6]; SECTORS];
            for (slot, values) in out.iter_mut().zip(matrices) {
                let values = values
                    .as_array()
                    .filter(|v| v.len() == 6)
                    .context(message)?;
                for (v, value) in slot.iter_mut().zip(values) {
                    *v = value
                        .as_i64()
                        .filter(|n| (-127..=127).contains(n))
                        .context(message)? as i8;
                }
            }
            Ok(out)
        };
        let weight = value["weight"]
            .as_array()
            .filter(|w| w.len() == 4)
            .context(message)?;
        let mut result = Self {
            matrices: sectors(&value["matrices"])?,
            weight: [0; 4],
            levels: None,
        };
        for (w, value) in result.weight.iter_mut().zip(weight) {
            *w = value.as_u64().filter(|n| *n <= 255).context(message)? as u8;
        }
        if let Some(levels) = value.get("levels") {
            let levels = levels
                .as_array()
                .filter(|l| l.len() == LEVELS - 1)
                .context(message)?;
            let mut upper = [[[0; 6]; SECTORS]; LEVELS - 1];
            for (slot, level) in upper.iter_mut().zip(levels) {
                *slot = sectors(level)?;
            }
            result.levels = Some(upper);
        }
        result.check()?;
        Ok(result)
    }

    pub fn to_json(&self) -> Value {
        let mut value = json!({"matrices": self.matrices.to_vec(), "weight": self.weight});
        if let Some(levels) = &self.levels {
            value["levels"] = json!(levels.iter().map(|l| l.to_vec()).collect::<Vec<_>>());
        }
        value
    }

    /// Whether the slopes of the weight curve stay within their 9-bit field (-256..255)
    /// when doubled. With highlight correction the camera halves the stage's input, so the
    /// installer writes a second set whose breakpoints halve and whose slopes double.
    pub fn slopes_double(&self) -> bool {
        let fields = self.control();
        (0..4).all(|k| {
            let v = i32::from(fields[4 + k] & 0x1ff);
            let slope = if v & 0x100 != 0 { v - 0x200 } else { v };
            (-128..=127).contains(&slope)
        })
    }

    /// The diagonal of each row is 1 minus its two coefficients. It must fit the 12-bit
    /// register field in Q10 (-2048..2047). The weight curve must keep `slopes_double`.
    fn check(&self) -> Result<()> {
        let upper = self.levels.iter().flatten();
        ensure!(
            std::iter::once(&self.matrices)
                .chain(upper)
                .flatten()
                .flat_map(|m| m.chunks_exact(2))
                .all(|p| p[0] != i8::MIN
                    && p[1] != i8::MIN
                    && i32::from(p[0]) + i32::from(p[1]) >= -127),
            "A hue-sector row leaves the register range"
        );
        ensure!(
            self.slopes_double(),
            "The hue-sector weight curve changes too fast for highlight correction, which doubles its slopes"
        );
        Ok(())
    }

    pub fn to_bytes(&self) -> [u8; SIZE] {
        let mut out = [0u8; SIZE];
        for (k, v) in self.matrices.iter().flatten().enumerate() {
            out[k] = *v as u8;
        }
        for (k, field) in self.control().iter().enumerate() {
            out[SECTORS * 6 + k * 2..SECTORS * 6 + k * 2 + 2].copy_from_slice(&field.to_le_bytes());
        }
        out
    }

    /// The GRF bytes of levels 1 to 3, if the effect has levels.
    pub fn level_bytes(&self) -> Option<[u8; LEVEL_SIZE]> {
        self.levels.map(|levels| {
            let mut out = [0u8; LEVEL_SIZE];
            for (k, v) in levels.iter().flatten().flatten().enumerate() {
                out[k] = *v as u8;
            }
            out
        })
    }

    /// Reads the GRF bytes, and the bytes of levels 1 to 3 for an effect with levels. The
    /// control fields must be those of their start weights.
    pub fn from_bytes(bytes: &[u8], levels: Option<&[u8]>) -> Result<Self> {
        ensure!(bytes.len() == SIZE, "Expected {SIZE} hue-sector bytes");
        let mut result = Self {
            matrices: [[0; 6]; SECTORS],
            weight: [0; 4],
            levels: None,
        };
        for (k, v) in result.matrices.iter_mut().flatten().enumerate() {
            *v = bytes[k] as i8;
        }
        if let Some(levels) = levels {
            ensure!(
                levels.len() == LEVEL_SIZE,
                "Expected {LEVEL_SIZE} hue-level bytes"
            );
            let mut upper = [[[0; 6]; SECTORS]; LEVELS - 1];
            for (k, v) in upper.iter_mut().flatten().flatten().enumerate() {
                *v = levels[k] as i8;
            }
            result.levels = Some(upper);
        }
        let field = |k: usize| {
            u16::from_le_bytes([bytes[SECTORS * 6 + k * 2], bytes[SECTORS * 6 + k * 2 + 1]])
        };
        for (k, w) in result.weight.iter_mut().enumerate() {
            *w = (f64::from(field(k)) * 255.0 / 1024.0)
                .round()
                .clamp(0.0, 255.0) as u8;
        }
        result.check()?;
        ensure!(
            (0..11).all(|k| field(k) == result.control()[k]),
            "Invalid hue-sector weight fields"
        );
        Ok(result)
    }
}
