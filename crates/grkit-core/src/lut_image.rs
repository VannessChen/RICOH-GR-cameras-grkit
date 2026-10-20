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
//! neighbouring lattice points.

use anyhow::{bail, Result};

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
/// all), and the total change of the green output along the green input.
fn roughness(pixels: &[[f32; 3]], geometry: &Geometry) -> (f64, f64) {
    let c = geometry.c;
    let (mut sum, mut green) = (0.0, 0.0);
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
                if g + 1 < c {
                    green += (pixels[geometry.index([r, g + 1, b])][1] - at[1]) as f64;
                }
            }
        }
    }
    (sum, green)
}

/// Reads the table of a LUT image: the lattice size, the entries with red changing fastest,
/// and the layout that was used.
pub fn table(
    width: usize,
    height: usize,
    pixels: Vec<[f32; 3]>,
) -> Result<(usize, Vec<[f32; 3]>, Layout)> {
    let scored: Vec<(f64, f64, Geometry)> = candidates(width, height)
        .into_iter()
        .map(|geometry| {
            let (sum, green) = roughness(&pixels, &geometry);
            (sum, green, geometry)
        })
        .collect();
    if scored.is_empty() {
        bail!(
            "Expected a HALD image (square, side 8, 27, 64, ... 4096) or a strip of c tiles of \
             c x c pixels, found {width} x {height}"
        );
    }
    // A layout read upside down makes the green output fall as the green input rises. The
    // total variation is the same both ways, so the trend decides the orientation.
    let rising = scored.iter().any(|s| s.1 > 0.0);
    let mut scored: Vec<(f64, Geometry)> = scored
        .into_iter()
        .filter(|s| !rising || s.1 > 0.0)
        .map(|(sum, _, geometry)| (sum, geometry))
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
