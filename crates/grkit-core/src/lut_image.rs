//! 3D LUTs stored as images.
//!
//! A HALD CLUT of level L is a square image of side L^3 whose pixels, in scanline order, are the
//! table with red changing fastest. Two other layouts have the same sizes or are common:
//!
//! - Square tiles (for example OBS, 512 x 512): t x t tiles of c x c pixels with c = t^2. In a
//!   tile, x is red and y is green. Tiles are in row order by blue. The side t^3 is also the
//!   side of a HALD of level t.
//! - Strips (for example 256 x 16 or 1024 x 32): c tiles of c x c pixels side by side, ordered
//!   by blue. Some engines store green from the bottom up.
//!
//! The pixel count cannot tell a HALD from square tiles. The reader decodes each layout that
//! fits the size and keeps the smoothest table, because a wrong layout makes jumps between
//! neighbouring lattice points. The two orientations of a strip are equally smooth; the gray
//! axis tells them apart.

use anyhow::{bail, ensure, Result};

/// Decoded samples, 0..1, with gray expanded to RGB and alpha dropped.
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[f32; 3]>,
}

/// The HALD levels that an identity image can have: 2 (8 x 8) to 16 (4096 x 4096).
pub const LEVELS: std::ops::RangeInclusive<usize> = 2..=16;
/// The most pixels that the PNG and TIFF readers decode: four times a HALD image of level
/// 16. A larger header fails before any allocation.
pub(crate) const MAX_PIXELS: usize = 1 << 26;

/// A 16-bit PNG identity HALD image of `level`: apply a look to it in an editor, export it
/// without changing its size, and import the result as a LUT.
pub fn identity_png(level: usize) -> Result<Vec<u8>> {
    ensure!(
        LEVELS.contains(&level),
        "Expected a HALD level from {} to {}",
        LEVELS.start(),
        LEVELS.end()
    );
    let c = level * level;
    let side = level * c;
    let step = |i: usize| ((i * 65535) as f64 / (c - 1) as f64).round() as u16;
    let mut samples = Vec::with_capacity(side * side * 3);
    for b in 0..c {
        for g in 0..c {
            for r in 0..c {
                samples.extend([step(r), step(g), step(b)]);
            }
        }
    }
    crate::png::encode_rgb16(side, side, &samples)
}

/// A noise level above which a LUT image was probably exported with grain or sharpening.
/// On 294 clean HALD images the noise is at most 0.024. Grain of 1/255 gives at least 0.031,
/// and sharpening of radius 1 exceeds the bound in 260 of 294 images.
pub const NOISE_LIMIT: f64 = 0.028;

/// The noise limit for a table of `c` points per axis. The second difference of a smooth look
/// grows with the square of the lattice step, so a coarse table needs a larger limit: clean
/// strips of 16 and 32 points and a HALD of 36 points measured 0.14, 0.06, and 0.04. From 64
/// points on, the limit is `NOISE_LIMIT`.
pub fn noise_limit(c: usize) -> f64 {
    let step = 63.0 / (c.max(2) - 1) as f64;
    NOISE_LIMIT * step.powi(2).max(1.0)
}

/// The 99.5th percentile of the absolute second difference of a 3D table along red, the axis
/// whose lattice neighbours are neighbouring pixels in each layout. A smooth look stays near
/// the quantization step there, and grain or sharpening of the image does not.
/// Effects that change slowly across the image, such as a vignette or clarity, do not show.
pub fn noise(table: &[[f32; 3]], c: usize) -> f64 {
    if c < 3 {
        return 0.0;
    }
    let mut values: Vec<f32> = table
        .chunks_exact(c)
        .flat_map(|row| {
            row.windows(3).map(|w| {
                (0..3)
                    .map(|k| (w[0][k] - 2.0 * w[1][k] + w[2][k]).abs())
                    .fold(0.0f32, f32::max)
            })
        })
        .collect();
    let rank = values.len() * 995 / 1000;
    *values.select_nth_unstable_by(rank, f32::total_cmp).1 as f64
}

/// How an image holds its table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Hald,
    SquareTiles,
    Strip,
    /// A strip with green increasing from the bottom row up.
    FlippedStrip,
}

impl Layout {
    pub fn name(self) -> &'static str {
        match self {
            Self::Hald => "HALD",
            Self::SquareTiles => "square tiles",
            Self::Strip => "strip",
            Self::FlippedStrip => "strip with green from the bottom up",
        }
    }
}

/// The best layout must have at most this share of the next layout's roughness.
const MARGIN: f64 = 0.8;

/// Maps lattice point (r, g, b) to its pixel index for one layout with lattice size `c`.
struct Geometry {
    layout: Layout,
    c: usize,
    width: usize,
    /// The tiles per row of a square-tile image, or the HALD level.
    level: usize,
}

impl Geometry {
    fn index(&self, [r, g, b]: [usize; 3]) -> usize {
        let c = self.c;
        let (x, y) = match self.layout {
            // The pixels in scanline order are the table.
            Layout::Hald => return r + c * g + c * c * b,
            Layout::SquareTiles => (r + c * (b % self.level), g + c * (b / self.level)),
            Layout::Strip => (r + c * b, g),
            Layout::FlippedStrip => (r + c * b, c - 1 - g),
        };
        x + y * self.width
    }
}

/// The layouts whose geometry fits a width x height image.
fn candidates(width: usize, height: usize) -> Vec<Geometry> {
    let geometry = |layout, c, level| Geometry {
        layout,
        c,
        width,
        level,
    };
    let mut result = Vec::new();
    if width == height {
        // Side L^3 for L of 2 to 16 gives L^2 lattice points per axis.
        if let Some(level) = (2..=16).find(|l: &usize| l.pow(3) == width) {
            result.push(geometry(Layout::Hald, level * level, level));
            result.push(geometry(Layout::SquareTiles, level * level, level));
        }
    } else if height >= 2 && width == height * height {
        result.push(geometry(Layout::Strip, height, 0));
        result.push(geometry(Layout::FlippedStrip, height, 0));
    }
    result
}

/// The total variation along green and blue, where the layouts differ (red is the same in
/// all).
fn roughness(pixels: &[[f32; 3]], geometry: &Geometry) -> f64 {
    let c = geometry.c;
    let mut sum = 0.0;
    for b in 0..c {
        for g in 0..c {
            for r in 0..c {
                let at = pixels[geometry.index([r, g, b])];
                for next in [
                    (g + 1 < c).then(|| [r, g + 1, b]),
                    (b + 1 < c).then(|| [r, g, b + 1]),
                ]
                .into_iter()
                .flatten()
                {
                    let next = pixels[geometry.index(next)];
                    sum += (0..3).map(|k| (next[k] - at[k]).abs() as f64).sum::<f64>();
                }
            }
        }
    }
    sum
}

/// Reads the table of a LUT image: the lattice size, the entries with red changing fastest,
/// and the layout that was used.
pub fn table(
    width: usize,
    height: usize,
    pixels: Vec<[f32; 3]>,
) -> Result<(usize, Vec<[f32; 3]>, Layout)> {
    let scored: Vec<(f64, Geometry)> = candidates(width, height)
        .into_iter()
        .map(|geometry| (roughness(&pixels, &geometry), geometry))
        .collect();
    if scored.is_empty() {
        bail!(
            "Expected a HALD image (square, side 8, 27, 64, ... 4096) or a strip of c tiles of \
             c x c pixels, found {width} x {height}"
        );
    }
    // A strip read upside down has the same total variation, so the gray axis decides the
    // orientation: a look keeps black darker than white. The other orientation reverses the
    // green input, which darkens the white end. A LUT that mixes the channels, such as a hue
    // rotation, keeps its gray axis only in the right orientation; a negative darkens it both
    // ways, and the fit then refuses it.
    let gray_rise = |geometry: &Geometry| {
        let level = |k: usize| {
            let v = pixels[geometry.index([k; 3])];
            f64::from(v[0] + v[1] + v[2]) / 3.0
        };
        level(geometry.c - 1) - level(0)
    };
    let is_strip = |g: &Geometry| matches!(g.layout, Layout::Strip | Layout::FlippedStrip);
    let rise = scored
        .iter()
        .filter(|s| is_strip(&s.1))
        .map(|s| gray_rise(&s.1))
        .fold(f64::NEG_INFINITY, f64::max);
    let mut scored: Vec<(f64, Geometry)> = scored
        .into_iter()
        .filter(|s| !is_strip(&s.1) || gray_rise(&s.1) >= rise)
        .collect();
    scored.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (best, geometry) = scored.swap_remove(0);
    let next = scored.iter().map(|s| s.0).fold(f64::INFINITY, f64::min);
    if best >= MARGIN * next {
        bail!(
            "Cannot tell the layout of this {width} x {height} LUT image: read as {} it is \
             about as smooth as another layout",
            geometry.layout.name()
        );
    }
    let c = geometry.c;
    let table = if geometry.layout == Layout::Hald {
        pixels
    } else {
        let mut table = Vec::with_capacity(c * c * c);
        for b in 0..c {
            for g in 0..c {
                for r in 0..c {
                    table.push(pixels[geometry.index([r, g, b])]);
                }
            }
        }
        table
    };
    Ok((c, table, geometry.layout))
}
