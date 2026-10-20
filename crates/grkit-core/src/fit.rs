//! Fit of a target look, such as a `.cube` LUT, to the camera model of GRKit effects.
//!
//! The model and the fitting approach are ported from gr3-film-mods (MIT, see
//! `GR3_FILM_MODS_LICENSE.txt`): `cube_fit.py`, `joint_fit.py`, and `lab_fit.py`, FINDINGS
//! §11-§13 and §22l-§22m. For scene-linear RGB `L`:
//!
//!   code_c = Curve_c(BaseGamma(K (M L)_c)) / 16384,   display = clip(P code, 0, 1)
//!
//! `M` is the effect matrix (rows sum to 1), `Curve_c` the effect curves, and `P` the optional
//! post matrix on gamma-encoded RGB. The display output is compared with sRGB-encoded targets.
//! `BaseGamma` is the still-path Standard base gamma that the caller reads from the user's
//! firmware: 256 entries of 14-bit output over a linear index domain.

use anyhow::{ensure, Context, Result};
use serde::Serialize;

use crate::cube::{self, delta_e2000, Cube, Encoding, LabConverter, Layout, Matrix, Rgb};
use crate::{effects, packaging};

/// Assumption, not a measurement: 18% gray lands at base-gamma index 7.8 (Standard sRGB ~0.46).
pub const MID_INDEX: f64 = 7.8;
pub const K: f64 = MID_INDEX / 0.18;
/// The GR III Standard look matrix: the `standard` input encoding and the Standard comparison.
pub const STANDARD_LOOK: [i16; 9] = [8116, 743, -667, 15, 8598, -421, -244, 542, 7894];
pub const IDENTITY: [f64; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
pub const MAX_EXPOSURE: f64 = 8.0;

/// X-Rite ColorChecker Classic, CIE L*a*b* D50 reference values for charts made after
/// November 2014, in patch order (dark skin to black).
const COLORCHECKER_LAB: [[f64; 3]; 24] = [
    [37.54, 14.37, 14.92],
    [64.66, 19.27, 17.50],
    [49.32, -3.82, -22.54],
    [43.46, -12.74, 22.72],
    [54.94, 9.61, -24.79],
    [70.48, -32.26, -0.37],
    [62.73, 35.83, 56.50],
    [39.43, 10.75, -45.17],
    [50.57, 48.64, 16.67],
    [30.10, 22.54, -20.87],
    [71.77, -24.13, 58.19],
    [71.51, 18.24, 67.37],
    [28.37, 15.42, -49.80],
    [54.38, -39.72, 32.27],
    [42.43, 51.05, 28.62],
    [81.80, 2.67, 80.41],
    [50.63, 51.28, -14.12],
    [49.57, -29.71, -28.32],
    [95.19, -1.03, 2.93],
    [81.29, -0.57, 0.44],
    [66.89, -0.75, -0.06],
    [50.76, -0.13, 0.14],
    [35.63, -0.46, -0.48],
    [20.64, 0.07, -0.46],
];
// cube_fit.py: 2nd-difference smoothing of the curves, gray and chart weights.
const SMOOTHING: f64 = 3000.0;
const GRAY_WEIGHT: f64 = 150.0;
const CHART_WEIGHT: f64 = 20.0;
const SKIN_WEIGHT: f64 = 60.0;
/// Weight of the post-matrix row sums relative to the root of the total sample weight.
const NEUTRAL_WEIGHT: f64 = 300.0;
/// BT.601 RGB to YCbCr in 1/256 units: the YCbCr (CSP) matrix that the runtime multiplies by `P`.
pub const BT601: [[i32; 3]; 3] = [[77, 150, 29], [-43, -85, 128], [128, -107, -21]];
/// The CSP registers hold -256..255. The limit leaves room for the hue rotation.
pub const CSP_LIMIT: i32 = 240;
/// Weight of a CSP coefficient beyond the limit, in the units of `NEUTRAL_WEIGHT`.
const CSP_WEIGHT: f64 = 300.0;
const STEP: f64 = 1e-6;
/// The largest effect matrix coefficient that the fit uses. Of 329 third-party LUTs, 99% fit
/// with coefficients up to 1.5. A larger coefficient amplifies sensor noise in the colors, and
/// the camera's matrix stage is not known to handle one.
pub const MATRIX_LIMIT: f64 = 2.0;
/// Pulls the post matrix toward the identity, relative to the root of the total sample weight,
/// in the Lab units of the post fit. It only fixes directions that the samples barely
/// determine: a damping of 1.0, or a damping of the effect matrix, made strong looks worse.
const POST_DAMPING: f64 = 0.1;
/// A target is toned monochrome when its color at each output level varies by at most this.
pub const TONED_TOLERANCE: f64 = 0.02;
/// A target is monochrome when no channel differs from another by more than this.
pub const MONOCHROME_TOLERANCE: f64 = 0.01;
/// Rec. 709 luma weights, the start of a monochrome matrix fit.
const LUMA: [f64; 3] = [0.2126, 0.7152, 0.0722];

/// Scene-linear sRGB of the ColorChecker 24 patches (Bradford D50 to D65), clipped at 0.
pub fn colorchecker() -> [Rgb; 24] {
    COLORCHECKER_LAB.map(|lab| cube::lab_d50_to_srgb(lab).map(|v| v.max(0.0)))
}

fn rows(m: &[f64; 9]) -> Matrix {
    std::array::from_fn(|r| std::array::from_fn(|c| m[r * 3 + c]))
}

/// Linear interpolation over 256 entries with `np.interp` clipping.
fn lookup(table: impl Fn(usize) -> f64, x: f64) -> f64 {
    let x = x.clamp(0.0, 255.0);
    let low = (x as usize).min(254);
    let t = x - low as f64;
    table(low) * (1.0 - t) + table(low + 1) * t
}

/// Unquantized effect parameters: matrix and post matrix row-major, curves 0..16383.
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    pub matrix: [f64; 9],
    pub curves: [[f64; 256]; 3],
    pub post: [f64; 9],
}

/// The camera around an effect: the Standard base gamma of the user's firmware.
#[derive(Clone, Debug)]
pub struct Model {
    base: [f64; 256],
}

impl Model {
    pub fn new(base_gamma: &[u16; 256]) -> Self {
        Self {
            base: base_gamma.map(f64::from),
        }
    }

    pub fn base_gamma(&self, linear: f64) -> f64 {
        lookup(|i| self.base[i], K * linear)
    }

    /// The scene-linear value whose base gamma is `level`, within the table's range.
    fn base_gamma_inverse(&self, level: f64) -> f64 {
        let level = level.clamp(self.base[0], self.base[255]);
        let low = (0..255)
            .find(|&i| self.base[i + 1] >= level && self.base[i + 1] > self.base[i])
            .unwrap_or(254);
        let (a, b) = (self.base[low], self.base[low + 1]);
        let t = if b > a { (level - a) / (b - a) } else { 0.0 };
        (low as f64 + t.clamp(0.0, 1.0)) / K
    }

    /// The display RGB of the camera's Standard look, without an effect.
    pub fn standard(&self, linear: Rgb) -> Rgb {
        cube::mul(&rows(&STANDARD_LOOK.map(|v| v as f64 / 8192.0)), linear)
            .map(|v| (self.base_gamma(v) / 16384.0).clamp(0.0, 1.0))
    }

    /// Scene-linear sRGB-primaries RGB to the input of a LUT in `encoding`.
    pub fn encode(&self, encoding: Encoding, linear: Rgb) -> Rgb {
        let log =
            |to: Matrix, curve: fn(f64) -> f64| cube::mul(&to, linear).map(|v| curve(v.max(0.0)));
        match encoding {
            Encoding::Standard => self.standard(linear),
            Encoding::Srgb => linear.map(|v| cube::srgb_encode(v.clamp(0.0, 1.0))),
            Encoding::Linear => linear,
            Encoding::Vlog => log(cube::srgb_to_vgamut(), cube::vlog_encode),
            Encoding::Flog => log(cube::srgb_to_fgamut(), cube::flog_encode),
            Encoding::Flog2 => log(cube::srgb_to_fgamut(), cube::flog2_encode),
            Encoding::Flog2C => log(cube::srgb_to_fgamut_c(), cube::flog2_encode),
        }
    }

    /// The inverse of `encode` within its unclipped range.
    pub fn decode(&self, encoding: Encoding, encoded: Rgb) -> Rgb {
        let log = |from: Matrix, curve: fn(f64) -> f64| cube::mul(&from, encoded.map(curve));
        match encoding {
            Encoding::Standard => {
                let look = cube::inverse(&rows(&STANDARD_LOOK.map(|v| v as f64 / 8192.0))).unwrap();
                cube::mul(&look, encoded.map(|v| self.base_gamma_inverse(v * 16384.0)))
            }
            Encoding::Srgb => encoded.map(cube::srgb_decode),
            Encoding::Linear => encoded,
            Encoding::Vlog => log(cube::vgamut_to_srgb(), cube::vlog_decode),
            Encoding::Flog => log(cube::fgamut_to_srgb(), cube::flog_decode),
            Encoding::Flog2 => log(cube::fgamut_to_srgb(), cube::flog2_decode),
            Encoding::Flog2C => log(cube::fgamut_c_to_srgb(), cube::flog2_decode),
        }
    }

    /// Gamma-encoded RGB after the curves and before the post matrix.
    pub fn codes(&self, params: &Params, linear: Rgb) -> Rgb {
        let mixed = cube::mul(&rows(&params.matrix), linear);
        std::array::from_fn(|c| {
            lookup(|i| params.curves[c][i], self.base_gamma(mixed[c]) / 64.0) / 16384.0
        })
    }

    /// The display RGB that the camera renders for scene-linear `linear`.
    pub fn render(&self, params: &Params, linear: Rgb) -> Rgb {
        cube::mul(&rows(&params.post), self.codes(params, linear)).map(|v| v.clamp(0.0, 1.0))
    }
}

/// Fit controls. `Default` is the production fit; tests may use fewer samples or rounds.
#[derive(Clone, Debug)]
pub struct Options {
    pub post: bool,
    pub random_samples: usize,
    /// Alternations of curves and matrix before the post matrix.
    pub rounds: usize,
    /// Alternations of curves, matrix, and post matrix after the first post fit.
    pub joint_rounds: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            post: true,
            random_samples: 6000,
            rounds: 8,
            joint_rounds: 6,
        }
    }
}

/// A quantized fit and its CIEDE2000 errors on the ColorChecker at 0 EV.
#[derive(Clone, Debug)]
pub struct Fit {
    pub matrix: [i16; 9],
    pub curves: [[u16; 256]; 3],
    pub post_matrix: Option<[i16; 9]>,
    pub mean_de: f64,
    pub max_de: f64,
    pub standard_mean_de: f64,
    /// The largest CSP coefficient magnitude, 1/256 units, at hue 0.
    pub csp_max: i32,
    /// The target is gray, so the fit has equal matrix rows, one curve, and no post matrix.
    pub monochrome: bool,
    /// The target is a toned monochrome, such as sepia: equal matrix rows, three curves, and
    /// no post matrix. It is not a MONOCHROME effect, because the curves differ.
    pub toned: bool,
    /// Mean and 95th percentile CIEDE2000 on colors that the fit does not see (`holdout`).
    pub holdout_mean_de: f64,
    pub holdout_p95_de: f64,
}

impl Fit {
    pub fn params(&self) -> Params {
        let unit = |m: [i16; 9]| m.map(|v| v as f64 / 8192.0);
        Params {
            matrix: unit(self.matrix),
            curves: self.curves.map(|c| c.map(f64::from)),
            post: self.post_matrix.map_or(IDENTITY, unit),
        }
    }
}

/// Colors that the fit does not see, for an error that the fit cannot overfit: CIE L*a*b*
/// with lightness 10 to 100, any hue, and chroma up to 70 biased toward low values as in
/// natural scenes, within sRGB, at -1 to +1 EV.
pub fn holdout() -> Vec<Rgb> {
    let mut rng = Random(2);
    let mut colors = Vec::with_capacity(HOLDOUT_COLORS);
    while colors.len() < HOLDOUT_COLORS {
        let l = 10.0 + 90.0 * rng.next();
        let chroma = 70.0 * rng.next().powi(2);
        let hue = std::f64::consts::TAU * rng.next();
        let exposure = 2f64.powf(2.0 * rng.next() - 1.0);
        let linear = cube::lab_d50_to_srgb([l, chroma * hue.cos(), chroma * hue.sin()]);
        if linear.iter().all(|v| (0.0..=1.0).contains(v)) {
            colors.push(linear.map(|v| v * exposure));
        }
    }
    colors
}

const HOLDOUT_COLORS: usize = 3000;

/// SplitMix64, so the random samples are deterministic on every host.
struct Random(u64);

impl Random {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// cube_fit.py samples: a gray ramp, the chart over -2..+2 EV, and random scene colors.
fn samples(random: usize) -> (Vec<Rgb>, Vec<f64>) {
    let mut linear = Vec::new();
    let mut weights = Vec::new();
    for i in 0..600 {
        let v = 0.18 * 2f64.powf(-7.0 + 11.5 * i as f64 / 599.0);
        linear.push([v; 3]);
        weights.push(GRAY_WEIGHT);
    }
    let chart = colorchecker();
    for ev in -2..=2 {
        for (patch, rgb) in chart.iter().enumerate() {
            linear.push(rgb.map(|v| v * 2f64.powi(ev)));
            weights.push(if patch < 2 { SKIN_WEIGHT } else { CHART_WEIGHT });
        }
    }
    let mut rng = Random(1);
    for _ in 0..random {
        let rgb: Rgb = std::array::from_fn(|_| rng.next().powf(2.2));
        let scale = 2f64.powf(-4.0 + 6.0 * rng.next()) * 0.6;
        linear.push(rgb.map(|v| v * scale));
        weights.push(1.0);
    }
    (linear, weights)
}

/// Solves a symmetric positive definite system whose nonzeros lie within `band` of the diagonal.
fn solve(n: usize, band: usize, mut a: Vec<f64>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    for j in 0..n {
        let first = j.saturating_sub(band);
        let d = a[j * n + j] - (first..j).map(|k| a[j * n + k].powi(2)).sum::<f64>();
        if d.is_nan() || d <= 0.0 {
            return None;
        }
        let d = d.sqrt();
        a[j * n + j] = d;
        for i in j + 1..n.min(j + band + 1) {
            let start = i.saturating_sub(band).max(first);
            let s: f64 = (start..j).map(|k| a[i * n + k] * a[j * n + k]).sum();
            a[i * n + j] = (a[i * n + j] - s) / d;
        }
    }
    for i in 0..n {
        let s: f64 = (i.saturating_sub(band)..i)
            .map(|k| a[i * n + k] * b[k])
            .sum();
        b[i] = (b[i] - s) / a[i * n + i];
    }
    for i in (0..n).rev() {
        let s: f64 = (i + 1..n.min(i + band + 1))
            .map(|k| a[k * n + i] * b[k])
            .sum();
        b[i] = (b[i] - s) / a[i * n + i];
    }
    b.iter().all(|v| v.is_finite()).then_some(b)
}

/// cube_fit.py `fit_curve`: weighted least squares of a 256-knot curve over g / 64 with
/// 2nd-difference smoothing, then clipped and made non-decreasing.
fn fit_curve(levels: impl Iterator<Item = (f64, f64, f64)>) -> [f64; 256] {
    let mut a = vec![0.0; 256 * 256];
    let mut b = vec![0.0; 256];
    for (level, target, weight) in levels {
        let x = (level / 64.0).clamp(0.0, 255.0);
        let low = (x as usize).min(254);
        let t = x - low as f64;
        let pairs = [(low, 1.0 - t), (low + 1, t)];
        for (i, u) in pairs {
            for (j, v) in pairs {
                a[i * 256 + j] += weight * u * v;
            }
            b[i] += weight * u * target * 16384.0;
        }
    }
    for row in 0..254 {
        let d = [(row, 1.0), (row + 1, -2.0), (row + 2, 1.0)];
        for (i, u) in d {
            for (j, v) in d {
                a[i * 256 + j] += SMOOTHING * u * v;
            }
        }
    }
    for i in 0..256 {
        a[i * 257] += 1e-6;
    }
    let solution = solve(256, 2, a, b).expect("the regularized curve system is positive definite");
    let mut result = [0.0; 256];
    let mut floor = 0.0f64;
    for (slot, value) in result.iter_mut().zip(solution) {
        floor = floor.max(value.clamp(0.0, 16383.0));
        *slot = floor;
    }
    result
}

fn sum_squares(values: &[f64]) -> f64 {
    values.iter().map(|v| v * v).sum()
}

/// Levenberg-Marquardt with a forward-difference Jacobian.
fn minimize(
    start: &[f64],
    iterations: usize,
    residuals: &dyn Fn(&[f64], &mut Vec<f64>),
) -> Vec<f64> {
    let n = start.len();
    let mut x = start.to_vec();
    let (mut r, mut trial) = (Vec::new(), Vec::new());
    residuals(&x, &mut r);
    let mut cost = sum_squares(&r);
    let mut lambda = 1e-3;
    let mut columns = vec![Vec::new(); n];
    for _ in 0..iterations {
        for (j, column) in columns.iter_mut().enumerate() {
            let mut y = x.clone();
            y[j] += STEP;
            residuals(&y, column);
            for (value, base) in column.iter_mut().zip(&r) {
                *value = (*value - base) / STEP;
            }
        }
        let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
        let mut normal = vec![0.0; n * n];
        let mut gradient = vec![0.0; n];
        for a in 0..n {
            gradient[a] = -dot(&columns[a], &r);
            for b in 0..=a {
                let value = dot(&columns[a], &columns[b]);
                normal[a * n + b] = value;
                normal[b * n + a] = value;
            }
        }
        let mut improved = false;
        while lambda < 1e10 {
            let mut damped = normal.clone();
            for a in 0..n {
                damped[a * n + a] += lambda * normal[a * n + a].max(1e-12);
            }
            if let Some(delta) = solve(n, n, damped, gradient.clone()) {
                let y: Vec<f64> = x.iter().zip(&delta).map(|(x, d)| x + d).collect();
                residuals(&y, &mut trial);
                let next = sum_squares(&trial);
                if next < cost {
                    let gain = (cost - next) / cost;
                    (x, cost) = (y, next);
                    std::mem::swap(&mut r, &mut trial);
                    lambda = (lambda / 3.0).max(1e-9);
                    improved = gain > 1e-10;
                    break;
                }
            }
            lambda *= 4.0;
        }
        if !improved {
            break;
        }
    }
    x
}

/// A matrix whose rows sum to 1 from its six off-diagonal coefficients, or from the green and
/// blue weights of one row that all three rows share.
fn matrix_from(offsets: &[f64]) -> [f64; 9] {
    if let [green, blue] = *offsets {
        let row = [1.0 - green - blue, green, blue];
        return std::array::from_fn(|n| row[n % 3]);
    }
    let mut m = [0.0; 9];
    for r in 0..3 {
        let others = [(r + 1) % 3, (r + 2) % 3];
        for (k, c) in others.into_iter().enumerate() {
            m[r * 3 + c] = offsets[r * 2 + k];
        }
        m[r * 4] = 1.0 - offsets[r * 2] - offsets[r * 2 + 1];
    }
    m
}

/// The shape of the fitted effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    Color,
    /// Equal matrix rows and three curves.
    Toned,
    /// Equal matrix rows and one curve.
    Monochrome,
}

/// Whether each output level of the target has one color: true for gray and toned looks.
fn toned(targets: &[Rgb]) -> bool {
    const BINS: usize = 64;
    let level = |t: &Rgb| t.iter().sum::<f64>() / 3.0;
    let bin = |t: &Rgb| ((level(t) * BINS as f64) as usize).min(BINS - 1);
    let mut sums = vec![([0.0; 3], 0usize); BINS];
    for t in targets {
        let (sum, count) = &mut sums[bin(t)];
        for c in 0..3 {
            sum[c] += t[c] - level(t);
        }
        *count += 1;
    }
    targets.iter().all(|t| {
        let (sum, count) = sums[bin(t)];
        (0..3).all(|c| (t[c] - level(t) - sum[c] / count as f64).abs() <= TONED_TOLERANCE)
    })
}

struct Problem {
    model: Model,
    linear: Vec<Rgb>,
    roots: Vec<f64>,
    targets: Vec<Rgb>,
    labs: Vec<[f64; 3]>,
    converter: LabConverter,
    neutral: f64,
    /// The root of the total sample weight, the scale of the damping terms.
    scale: f64,
    shape: Shape,
}

impl Problem {
    fn curves(&self, matrix: &[f64; 9], post: &[f64; 9]) -> [[f64; 256]; 3] {
        // joint_fit.py: fit the curves to the target pulled back through the post matrix.
        let inverse = cube::inverse(&rows(post)).unwrap_or(rows(&IDENTITY));
        let pulled: Vec<Rgb> = self
            .targets
            .iter()
            .map(|t| cube::mul(&inverse, *t).map(|v| v.clamp(0.0, 1.0)))
            .collect();
        let mixed: Vec<Rgb> = self
            .linear
            .iter()
            .map(|l| cube::mul(&rows(matrix), *l).map(|v| self.model.base_gamma(v)))
            .collect();
        if self.shape == Shape::Monochrome {
            // The rows are equal, so each channel has the same level. Fit the mean target.
            let curve = fit_curve((0..self.linear.len()).map(|i| {
                let gray = pulled[i].iter().sum::<f64>() / 3.0;
                (mixed[i][0], gray, self.roots[i].powi(2))
            }));
            return [curve; 3];
        }
        std::array::from_fn(|c| {
            fit_curve(
                (0..self.linear.len()).map(|i| (mixed[i][c], pulled[i][c], self.roots[i].powi(2))),
            )
        })
    }

    fn matrix(&self, offsets: &[f64], curves: &[[f64; 256]; 3], post: &[f64; 9]) -> Vec<f64> {
        minimize(offsets, 30, &|p, out| {
            let params = Params {
                matrix: matrix_from(p),
                curves: *curves,
                post: *post,
            };
            out.clear();
            for ((l, t), w) in self.linear.iter().zip(&self.targets).zip(&self.roots) {
                let d = self.model.render(&params, *l);
                out.extend((0..3).map(|c| w * (d[c] - t[c])));
            }
            // A look that the model cannot follow, such as infrared false color, drives the
            // coefficients without bound. Keep them within the limit.
            out.extend(
                params
                    .matrix
                    .iter()
                    .map(|v| self.neutral * (v.abs() - MATRIX_LIMIT).max(0.0)),
            );
        })
    }

    /// Lab residuals, the neutral penalty on the row sums, and the CSP penalty.
    fn post(&self, matrix: &[f64; 9], curves: &[[f64; 256]; 3], start: &[f64; 9]) -> [f64; 9] {
        let params = Params {
            matrix: *matrix,
            curves: *curves,
            post: IDENTITY,
        };
        let codes: Vec<Rgb> = self
            .linear
            .iter()
            .map(|l| self.model.codes(&params, *l))
            .collect();
        let limit = CSP_LIMIT as f64 / 256.0;
        let bt601: Matrix = BT601.map(|r| r.map(|v| v as f64 / 256.0));
        let result =
            minimize(start, 30, &|p, out| {
                let post = rows(p.try_into().unwrap());
                out.clear();
                for ((code, lab), w) in codes.iter().zip(&self.labs).zip(&self.roots) {
                    let fitted = self.converter.lab(cube::mul(&post, *code));
                    out.extend((0..3).map(|c| w * (fitted[c] - lab[c])));
                }
                out.extend(
                    (0..3).map(|r| self.neutral * (p[r * 3..r * 3 + 3].iter().sum::<f64>() - 1.0)),
                );
                let csp = cube::product(&bt601, &post);
                out.extend(csp.iter().flatten().map(|v| {
                    CSP_WEIGHT / NEUTRAL_WEIGHT * self.neutral * (v.abs() - limit).max(0.0)
                }));
                // Directions that the samples do not determine, such as the mixing of equal
                // channels, would otherwise drift and amplify noise in the colors.
                out.extend(
                    p.iter()
                        .zip(IDENTITY)
                        .map(|(v, i)| POST_DAMPING * self.scale * (v - i)),
                );
            });
        result.try_into().unwrap()
    }
}

const MATRIX_RANGE: &str =
    "The LUT mixes the color channels more than the camera matrix can represent";

/// Rounds a matrix to Q13 with each row summing to 8192.
fn quantize(m: &[f64; 9]) -> Result<[i16; 9]> {
    let mut result = [0; 9];
    for r in 0..3 {
        let mut values: Vec<i64> = m[r * 3..r * 3 + 3]
            .iter()
            .map(|v| (v * 8192.0).round_ties_even() as i64)
            .collect();
        values[r] += 8192 - values.iter().sum::<i64>();
        for (c, value) in values.into_iter().enumerate() {
            result[r * 3 + c] = i16::try_from(value).ok().context(MATRIX_RANGE)?;
        }
    }
    Ok(result)
}

/// Quantizes the shared row of a monochrome or toned matrix, so that the rows stay equal.
fn quantize_monochrome(m: &[f64; 9]) -> Result<[i16; 9]> {
    let mut row = [0i64; 3];
    for (slot, v) in row.iter_mut().zip(&m[..3]) {
        *slot = (v * 8192.0).round_ties_even() as i64;
    }
    // Put the rounding remainder on the largest weight, where it changes the least.
    let largest = (0..3).max_by_key(|&c| row[c]).unwrap();
    row[largest] += 8192 - row.iter().sum::<i64>();
    let mut result = [0; 9];
    for (n, slot) in result.iter_mut().enumerate() {
        *slot = i16::try_from(row[n % 3]).ok().context(MATRIX_RANGE)?;
    }
    Ok(result)
}

/// The largest |BT601 x P| coefficient with the rounding of `picture_post` in the runtime.
pub fn csp_max(post: &[i16; 9]) -> i32 {
    (0..9)
        .map(|n| {
            let (i, j) = (n / 3, n % 3);
            let sum: i32 = (0..3).map(|k| BT601[i][k] * post[k * 3 + j] as i32).sum();
            ((sum + 4096) >> 13).abs()
        })
        .max()
        .unwrap()
}

/// Quantizes `post` and, if its CSP exceeds the limit, moves it toward the identity until it fits.
fn constrain(post: &[f64; 9]) -> Result<[i16; 9]> {
    let toward = |s: f64| -> Result<[i16; 9]> {
        quantize(&std::array::from_fn(|n| {
            IDENTITY[n] + s * (post[n] - IDENTITY[n])
        }))
    };
    let fits = |s: f64| toward(s).is_ok_and(|q| csp_max(&q) <= CSP_LIMIT);
    if fits(1.0) {
        return toward(1.0);
    }
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..40 {
        let middle = (low + high) / 2.0;
        if fits(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    toward(low)
}

/// Fits `target`, a map from scene-linear RGB to sRGB-encoded display RGB.
pub fn fit(base_gamma: &[u16; 256], target: &dyn Fn(Rgb) -> Rgb, options: &Options) -> Result<Fit> {
    let (linear, weights) = samples(options.random_samples);
    let targets: Vec<Rgb> = linear
        .iter()
        .map(|l| target(*l).map(|v| v.clamp(0.0, 1.0)))
        .collect();
    ensure!(
        targets.iter().flatten().all(|v| v.is_finite()),
        "The target produced non-finite values"
    );
    let ramp: Vec<f64> = targets
        .iter()
        .zip(&linear)
        .filter(|(_, l)| l[0] == l[1] && l[1] == l[2])
        .map(|(t, _)| t.iter().sum::<f64>() / 3.0)
        .collect();
    // The curves cannot decrease, so they cannot follow a negative.
    ensure!(
        ramp.first() <= ramp.last(),
        "The LUT inverts brightness, which the camera curves cannot represent"
    );
    let monochrome = targets
        .iter()
        .all(|t| t.iter().fold(0.0f64, |m, v| m.max((v - t[0]).abs())) <= MONOCHROME_TOLERANCE);
    let shape = if monochrome {
        Shape::Monochrome
    } else if toned(&targets) {
        Shape::Toned
    } else {
        Shape::Color
    };
    let converter = LabConverter::default();
    let scale = weights.iter().sum::<f64>().sqrt();
    let problem = Problem {
        model: Model::new(base_gamma),
        labs: targets.iter().map(|t| converter.lab(*t)).collect(),
        roots: weights.iter().map(|w| w.sqrt()).collect(),
        neutral: NEUTRAL_WEIGHT * scale,
        scale,
        converter,
        linear,
        targets,
        shape,
    };
    let shared_rows = shape != Shape::Color;
    let mut offsets = if shared_rows {
        vec![LUMA[1], LUMA[2]]
    } else {
        vec![0.0; 6]
    };
    let mut post = IDENTITY;
    for _ in 0..options.rounds {
        let curves = problem.curves(&matrix_from(&offsets), &post);
        offsets = problem.matrix(&offsets, &curves, &post);
    }
    let mut curves = problem.curves(&matrix_from(&offsets), &post);
    if options.post && !shared_rows {
        post = problem.post(&matrix_from(&offsets), &curves, &post);
        for _ in 0..options.joint_rounds {
            curves = problem.curves(&matrix_from(&offsets), &post);
            offsets = problem.matrix(&offsets, &curves, &post);
            curves = problem.curves(&matrix_from(&offsets), &post);
            post = problem.post(&matrix_from(&offsets), &curves, &post);
        }
    }
    let identity = quantize(&IDENTITY)?;
    let post_matrix = Some(constrain(&post)?).filter(|p| *p != identity);
    let mut fit = Fit {
        matrix: if shared_rows {
            quantize_monochrome(&matrix_from(&offsets))?
        } else {
            quantize(&matrix_from(&offsets))?
        },
        curves: curves.map(|c| c.map(|v| v.round() as u16)),
        post_matrix,
        mean_de: 0.0,
        max_de: 0.0,
        standard_mean_de: 0.0,
        csp_max: csp_max(&post_matrix.unwrap_or(identity)),
        monochrome,
        toned: shape == Shape::Toned,
        holdout_mean_de: 0.0,
        holdout_p95_de: 0.0,
    };
    let params = fit.params();
    let model = &problem.model;
    let lab = |rgb: Rgb| problem.converter.lab(rgb);
    let chart = colorchecker();
    let (mut sum, mut standard_sum) = (0.0, 0.0);
    for patch in chart {
        let expected = lab(target(patch).map(|v| v.clamp(0.0, 1.0)));
        let de = delta_e2000(lab(model.render(&params, patch)), expected);
        sum += de;
        fit.max_de = fit.max_de.max(de);
        standard_sum += delta_e2000(lab(model.standard(patch)), expected);
    }
    fit.mean_de = sum / chart.len() as f64;
    fit.standard_mean_de = standard_sum / chart.len() as f64;
    let mut errors: Vec<f64> = holdout()
        .into_iter()
        .map(|l| {
            let expected = lab(target(l).map(|v| v.clamp(0.0, 1.0)));
            delta_e2000(lab(model.render(&params, l)), expected)
        })
        .collect();
    errors.sort_by(f64::total_cmp);
    fit.holdout_mean_de = errors.iter().sum::<f64>() / errors.len() as f64;
    fit.holdout_p95_de = errors[errors.len() * 95 / 100];
    Ok(fit)
}

/// The fit target of a LUT: scene-linear RGB, exposed by `exposure` EV, encoded, then looked up.
pub fn cube_target<'a>(
    model: &'a Model,
    cube: &'a Cube,
    input: Encoding,
    exposure: f64,
) -> impl Fn(Rgb) -> Rgb + 'a {
    let gain = 2f64.powf(exposure);
    move |linear| {
        cube.apply(model.encode(input, linear.map(|v| v * gain)))
            .map(|v| v.clamp(0.0, 1.0))
    }
}

/// What `import` writes besides the fitted coefficients.
#[derive(Clone, Debug)]
pub struct Import<'a> {
    pub effect_id: &'a str,
    pub name: &'a str,
    pub label: &'a str,
    pub shape: &'a str,
    /// The `.cube` or HALD PNG file name, recorded in the attribution.
    pub source: &'a str,
    pub input: Encoding,
    pub exposure: f64,
    pub options: Options,
}

#[derive(Serialize)]
struct Icon<'a> {
    label: &'a str,
    shape: &'a str,
}

#[derive(Serialize)]
struct Errors {
    mean_de: f64,
    max_de: f64,
    standard_mean_de: f64,
    csp_max: i32,
    holdout_mean_de: f64,
    holdout_p95_de: f64,
    grade: &'static str,
}

#[derive(Serialize)]
struct Attribution<'a> {
    origin: &'a str,
    source: &'a str,
    source_sha256: String,
    input: &'a str,
    exposure: f64,
    fit: Errors,
}

/// Field order follows the existing effect.json files.
#[derive(Serialize)]
struct Definition<'a> {
    schema: &'a str,
    effect_id: &'a str,
    name: &'a str,
    flags: u32,
    matrix: [i16; 9],
    icon: Icon<'a>,
    attribution: Attribution<'a>,
    curves: [&'a [u16]; 3],
    #[serde(skip_serializing_if = "Option::is_none")]
    post_matrix: Option<[i16; 9]>,
}

/// Pretty effect.json text that `effects::compile_definition` accepts.
fn definition_text(fit: &Fit, request: &Import, sha256: String) -> Result<String> {
    let round = |v: f64| (v * 1000.0).round() / 1000.0;
    let definition = Definition {
        schema: effects::SCHEMA,
        effect_id: request.effect_id,
        name: request.name,
        flags: if fit.monochrome {
            effects::MONOCHROME
        } else {
            0
        },
        matrix: fit.matrix,
        icon: Icon {
            label: request.label,
            shape: request.shape,
        },
        attribution: Attribution {
            origin: "cube-fit",
            source: request.source,
            source_sha256: sha256,
            input: request.input.name(),
            exposure: request.exposure,
            fit: Errors {
                mean_de: round(fit.mean_de),
                max_de: round(fit.max_de),
                standard_mean_de: round(fit.standard_mean_de),
                csp_max: fit.csp_max,
                holdout_mean_de: round(fit.holdout_mean_de),
                holdout_p95_de: round(fit.holdout_p95_de),
                grade: Grade::of(fit).letter(),
            },
        },
        curves: [&fit.curves[0], &fit.curves[1], &fit.curves[2]],
        post_matrix: fit.post_matrix,
    };
    let mut text = serde_json::to_string_pretty(&definition)?;
    text.push('\n');
    effects::compile_definition(&effects::parse_json(text.as_bytes())?)?;
    Ok(text)
}

/// How closely an effect follows its LUT, from the held-out colors.
///
/// The bounds are provisional. They were chosen so that the grades agree with the pixel errors
/// of 328 third-party LUTs on two natural sample pictures, where A is a mean dE2000 up to 1.5
/// and a 95th percentile up to 3, B up to 3 and 6, and C a mean up to 5. 277 grades agree,
/// and 11 are one grade too good. Calibrate them again with more pictures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grade {
    A,
    B,
    C,
    D,
}

impl Grade {
    /// (mean, 95th percentile) bounds of the held-out errors for A, B, and C.
    const BOUNDS: [(f64, f64); 3] = [(0.9, 3.75), (1.8, 6.5), (5.5, f64::INFINITY)];

    pub fn of(fit: &Fit) -> Self {
        let within =
            |(mean, p95): (f64, f64)| fit.holdout_mean_de <= mean && fit.holdout_p95_de <= p95;
        match Self::BOUNDS.iter().position(|b| within(*b)) {
            Some(0) => Self::A,
            Some(1) => Self::B,
            Some(_) => Self::C,
            None => Self::D,
        }
    }

    pub fn letter(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::A => "nearly the same as the LUT",
            Self::B => "the same look with small differences",
            Self::C => "a similar look with visible color differences",
            Self::D => "a different look",
        }
    }
}

/// The result of `import`.
#[derive(Clone, Debug)]
pub struct Imported {
    pub fit: Fit,
    /// The effect.json text.
    pub text: String,
    /// Conditions that the user should check before using the effect.
    pub warnings: Vec<String>,
}

/// The inputs that `guess_input` chooses from. Linear is left out: a look LUT does not take
/// scene-linear values.
const GUESSES: [Encoding; 6] = [
    Encoding::Standard,
    Encoding::Srgb,
    Encoding::Vlog,
    Encoding::Flog,
    Encoding::Flog2,
    Encoding::Flog2C,
];
/// How many times closer to Standard a log input must keep the LUT than the best of Standard
/// and sRGB, for `guess_input` to choose it. On 32 LUTs of known input, 1.15 to 1.5 gave the
/// same choices.
const LOG_MARGIN: f64 = 1.3;

/// How far a LUT read with `input` is from the camera's Standard rendering: the RMS of the
/// neutral luma from -6 to +3 EV, plus the mean CIEDE2000 / 100 of the ColorChecker at -1, 0,
/// and +1 EV. A look changes a normal rendering moderately, so with the input that it expects,
/// a LUT stays close. With another input it loses or doubles the contrast of a log curve.
pub fn input_distance(model: &Model, cube: &Cube, input: Encoding) -> f64 {
    let target = cube_target(model, cube, input, 0.0);
    let luma = |c: Rgb| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    let grays: Vec<f64> = (0..=36)
        .map(|i| 0.18 * 2f64.powf(-6.0 + f64::from(i) * 0.25))
        .collect();
    let tone = (grays
        .iter()
        .map(|g| (luma(target([*g; 3])) - luma(model.standard([*g; 3]))).powi(2))
        .sum::<f64>()
        / grays.len() as f64)
        .sqrt();
    let converter = LabConverter::default();
    let chart = colorchecker();
    let mut color = 0.0;
    for ev in [-1, 0, 1] {
        for patch in chart {
            let c = patch.map(|v| v * 2f64.powi(ev));
            color += delta_e2000(converter.lab(target(c)), converter.lab(model.standard(c)));
        }
    }
    tone + color / (3.0 * chart.len() as f64) / 100.0
}

/// The input that a LUT most likely expects, from its data alone (`input_distance`). A log
/// input wins only when it is `LOG_MARGIN` times closer than Standard and sRGB; otherwise the
/// choice is Standard, as the data does not tell Standard from sRGB reliably.
pub fn guess_input(base_gamma: &[u16; 256], cube: &Cube) -> Encoding {
    let model = Model::new(base_gamma);
    let distances: Vec<(Encoding, f64)> = GUESSES
        .iter()
        .map(|input| (*input, input_distance(&model, cube, *input)))
        .collect();
    let best = |log: bool| {
        distances
            .iter()
            .filter(|(input, _)| log != matches!(input, Encoding::Standard | Encoding::Srgb))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .copied()
            .expect("GUESSES has log and display inputs")
    };
    let (log, display) = (best(true), best(false));
    if display.1 >= LOG_MARGIN * log.1 {
        log.0
    } else {
        Encoding::Standard
    }
}

/// Fits a `.cube` file or HALD PNG over `base_gamma` and returns the fit and its effect.json.
pub fn import(bytes: &[u8], base_gamma: &[u16; 256], request: &Import) -> Result<Imported> {
    ensure!(
        request.exposure.is_finite() && request.exposure.abs() <= MAX_EXPOSURE,
        "Expected an exposure from -{MAX_EXPOSURE} to {MAX_EXPOSURE} EV"
    );
    let ramp: [u16; 256] = std::array::from_fn(|i| (i * 64) as u16);
    let neutral = Fit {
        matrix: quantize(&IDENTITY)?,
        curves: [ramp; 3],
        post_matrix: None,
        mean_de: 0.0,
        max_de: 0.0,
        standard_mean_de: 0.0,
        csp_max: 0,
        monochrome: false,
        toned: false,
        holdout_mean_de: 0.0,
        holdout_p95_de: 0.0,
    };
    // Reject an invalid ID, name, or icon before the fit, which takes seconds.
    definition_text(&neutral, request, String::new())?;
    let cube = Cube::read(bytes)?;
    let model = Model::new(base_gamma);
    let fit = fit(
        base_gamma,
        &cube_target(&model, &cube, request.input, request.exposure),
        &request.options,
    )?;
    let text = definition_text(&fit, request, packaging::sha256(bytes))?;
    let mut warnings = Vec::new();
    let limit = (MATRIX_LIMIT * 8192.0) as i32 - 64;
    if fit.matrix.iter().any(|v| (*v as i32).abs() >= limit) {
        warnings.push(format!(
            "The fit reached the matrix limit of {MATRIX_LIMIT}. The effect mixes the color \
             channels strongly and can show color noise at high ISO."
        ));
    }
    if let Some(layout) = cube.image_layout.filter(|l| *l != Layout::Hald) {
        warnings.push(format!(
            "Read the image as a LUT in {}, not as a HALD image",
            layout.name()
        ));
    }
    // The data, not the file name, tells the input: a guess that keeps the LUT much closer to
    // Standard than the chosen input does. The data does not tell Standard from sRGB, so the
    // two do not warn about each other.
    let guess = guess_input(base_gamma, &cube);
    let display = |input: Encoding| matches!(input, Encoding::Standard | Encoding::Srgb);
    if guess != request.input
        && !(display(guess) && display(request.input))
        && input_distance(&model, &cube, request.input)
            >= LOG_MARGIN * input_distance(&model, &cube, guess)
    {
        warnings.push(format!(
            "The LUT data suggests {} input, but the import used {}",
            guess.name(),
            request.input.name()
        ));
    }
    if cube.video_input || cube.video_output {
        warnings.push(
            "The LUT uses video levels. GRKit converts 64-940 of 1023, an assumption that \
             Resolve does not document."
                .to_owned(),
        );
    }
    let grade = Grade::of(&fit);
    if grade >= Grade::C {
        warnings.push(format!(
            "Grade {}: {}. The look needs changes that one color matrix and three tone \
             curves cannot make.",
            grade.letter(),
            grade.summary()
        ));
    }
    Ok(Imported {
        fit,
        text,
        warnings,
    })
}
