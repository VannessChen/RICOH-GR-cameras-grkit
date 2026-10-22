//! Adobe / Resolve `.cube` LUTs, and LUT images through `lut_image`.
//!
//! RGB triples are display or scene values in sRGB primaries unless a name says otherwise.
//! The color transforms that the import needs are in `color`; this module re-exports them.

use anyhow::{bail, ensure, Context, Result};

pub use crate::color::*;
use crate::lut_image;
pub use crate::lut_image::Layout;

pub const MAX_3D_SIZE: usize = 256;
pub const MAX_1D_SIZE: usize = 65536;

/// A parsed LUT. `table` holds 3D entries with red changing fastest, or 1D rows.
#[derive(Clone, Debug)]
pub struct Cube {
    pub title: Option<String>,
    pub three_d: bool,
    pub size: usize,
    pub domain_min: Rgb,
    pub domain_max: Rgb,
    pub table: Vec<[f32; 3]>,
    /// A per-channel 1D table that the LUT applies before its 3D table, as in Resolve files
    /// that have both LUT_1D_SIZE and LUT_3D_SIZE.
    pub shaper: Option<Shaper>,
    /// Resolve's LUT_IN_VIDEO_RANGE: the table expects video levels at its input.
    pub video_input: bool,
    /// Resolve's LUT_OUT_VIDEO_RANGE: the table writes video levels.
    pub video_output: bool,
    /// How an image LUT held its table. None for a `.cube` file.
    pub image_layout: Option<Layout>,
}

/// A 1D table: one row per input level, interpolated per channel.
#[derive(Clone, Debug)]
pub struct Shaper {
    pub size: usize,
    pub domain_min: Rgb,
    pub domain_max: Rgb,
    pub table: Vec<[f32; 3]>,
}

fn numbers<const N: usize>(fields: &[&str], line: usize, keyword: &str) -> Result<[f64; N]> {
    ensure!(
        fields.len() == N,
        "Line {line}: {keyword} expects {N} values"
    );
    let mut result = [0.0; N];
    for (slot, field) in result.iter_mut().zip(fields) {
        *slot = field
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .with_context(|| format!("Line {line}: expected a finite number, found {field:?}"))?;
    }
    Ok(result)
}

/// Interpolates a 1D table per channel, with clamping to its domain.
fn interpolate_1d(table: &[[f32; 3]], min: &Rgb, max: &Rgb, rgb: Rgb) -> Rgb {
    let last = (table.len() - 1) as f64;
    std::array::from_fn(|c| {
        let unit = (rgb[c] - min[c]) / (max[c] - min[c]);
        let x = unit.clamp(0.0, 1.0) * last;
        let low = (x as usize).min(table.len() - 2);
        let (a, b) = (table[low][c] as f64, table[low + 1][c] as f64);
        a + (b - a) * (x - low as f64)
    })
}

fn is_keyword(word: &str) -> bool {
    word.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && word.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

// Resolve's developer documentation defines video range as 64 - 940 in 10-bit scale, but not
// the arithmetic. These conversions assume full range /1023 for each RGB channel.
const VIDEO_BLACK: f64 = 64.0 / 1023.0;
const VIDEO_SPAN: f64 = 876.0 / 1023.0;

/// Full-range (data) levels to video levels.
fn to_video(v: f64) -> f64 {
    VIDEO_BLACK + VIDEO_SPAN * v
}

/// Video levels to full-range (data) levels.
fn from_video(v: f64) -> f64 {
    (v - VIDEO_BLACK) / VIDEO_SPAN
}

impl Cube {
    /// Parses a `.cube` file in the Adobe 1.0 or the Resolve flavor. Keywords may be in any
    /// case, a TITLE may be unquoted, and data values may be separated by commas, as some
    /// exporters write them. A DESCRIPTION line is skipped as text. An unknown keyword fails,
    /// because it can change the transform.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        // Comments may hold text in a legacy encoding. Invalid bytes elsewhere fail as tokens.
        let text = String::from_utf8_lossy(bytes);
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        let text = text.replace("\r\n", "\n");
        let mut title = None;
        let (mut size_3d, mut size_1d) = (None, None);
        let (mut domain_min, mut domain_max) = (None, None);
        let (mut range_3d, mut range_1d) = (None, None);
        let mut rows = Vec::new();
        let (mut video_input, mut video_output) = (false, false);
        // Lines end with LF, CRLF, or a bare CR.
        for (index, raw) in text.split(['\n', '\r']).enumerate() {
            let line = index + 1;
            let raw = raw.trim();
            let starts_with = |keyword: &str| {
                raw.get(..keyword.len())
                    .is_some_and(|word| word.eq_ignore_ascii_case(keyword))
                    && raw[keyword.len()..]
                        .chars()
                        .next()
                        .is_none_or(char::is_whitespace)
            };
            // Some exporters describe the LUT in a DESCRIPTION line. It is text only.
            if starts_with("DESCRIPTION") {
                continue;
            }
            let is_title = starts_with("TITLE");
            // A TITLE keeps any '#' in its text. Other lines may end with a comment.
            let content = if is_title {
                raw
            } else {
                raw.split('#').next().unwrap().trim()
            };
            if content.is_empty() {
                continue;
            }
            let fields: Vec<&str> = content
                .split(|c: char| c.is_whitespace() || c == ',')
                .filter(|field| !field.is_empty())
                .collect();
            if fields[0].parse::<f64>().is_ok() {
                let row = numbers::<3>(&fields, line, "A data row")?;
                rows.push(row.map(|v| v as f32));
                continue;
            }
            let keyword = fields[0].to_ascii_uppercase();
            ensure!(
                rows.is_empty(),
                "Line {line}: keyword {keyword} after data rows"
            );
            let duplicate = |set: bool| -> Result<()> {
                ensure!(!set, "Line {line}: duplicate {keyword}");
                Ok(())
            };
            match keyword.as_str() {
                "TITLE" => {
                    duplicate(title.is_some())?;
                    let value = content[5..].trim();
                    let value = value
                        .strip_prefix('"')
                        .and_then(|v| v.strip_suffix('"'))
                        .unwrap_or(value);
                    title = Some(value.to_owned());
                }
                "LUT_3D_SIZE" | "LUT_1D_SIZE" => {
                    let three_d = keyword == "LUT_3D_SIZE";
                    let slot = if three_d { &mut size_3d } else { &mut size_1d };
                    duplicate(slot.is_some())?;
                    let limit = if three_d { MAX_3D_SIZE } else { MAX_1D_SIZE };
                    let size = (fields.len() == 2)
                        .then(|| fields[1].parse::<usize>().ok())
                        .flatten()
                        .filter(|n| (2..=limit).contains(n))
                        .with_context(|| format!("Line {line}: {keyword} must be 2 to {limit}"))?;
                    *slot = Some(size);
                }
                "DOMAIN_MIN" | "DOMAIN_MAX" => {
                    let slot = if keyword == "DOMAIN_MIN" {
                        &mut domain_min
                    } else {
                        &mut domain_max
                    };
                    duplicate(slot.is_some())?;
                    *slot = Some(numbers::<3>(&fields[1..], line, &keyword)?);
                }
                "LUT_3D_INPUT_RANGE" | "LUT_1D_INPUT_RANGE" => {
                    let slot = if keyword == "LUT_3D_INPUT_RANGE" {
                        &mut range_3d
                    } else {
                        &mut range_1d
                    };
                    duplicate(slot.is_some())?;
                    *slot = Some(numbers::<2>(&fields[1..], line, &keyword)?);
                }
                "LUT_IN_VIDEO_RANGE" | "LUT_OUT_VIDEO_RANGE" => {
                    let slot = if keyword == "LUT_IN_VIDEO_RANGE" {
                        &mut video_input
                    } else {
                        &mut video_output
                    };
                    duplicate(*slot)?;
                    ensure!(fields.len() == 1, "Line {line}: {keyword} takes no values");
                    *slot = true;
                }
                _ => {
                    ensure!(
                        is_keyword(fields[0]),
                        "Line {line}: expected a keyword or a data row, found {:?}",
                        fields[0]
                    );
                    bail!("Line {line}: unsupported keyword {keyword}");
                }
            }
        }
        ensure!(
            size_3d.is_some() || size_1d.is_some(),
            "Expected LUT_3D_SIZE or LUT_1D_SIZE"
        );
        ensure!(
            size_3d.is_none()
                || size_1d.is_none()
                || (domain_min.is_none() && domain_max.is_none()),
            "DOMAIN_MIN / DOMAIN_MAX is ambiguous with a 1D and a 3D table. Use \
             LUT_1D_INPUT_RANGE and LUT_3D_INPUT_RANGE"
        );
        for (keyword, range, size) in [
            ("LUT_3D_INPUT_RANGE", range_3d, size_3d),
            ("LUT_1D_INPUT_RANGE", range_1d, size_1d),
        ] {
            ensure!(
                range.is_none() || size.is_some(),
                "{keyword} does not match the table dimension"
            );
            ensure!(
                range.is_none() || (domain_min.is_none() && domain_max.is_none()),
                "Use either DOMAIN_MIN / DOMAIN_MAX or {keyword}"
            );
        }
        // A table without its own input range takes DOMAIN_MIN / DOMAIN_MAX, or 0..1.
        let domain = |range: Option<[f64; 2]>| -> Result<(Rgb, Rgb)> {
            let (min, max) = match range {
                Some([low, high]) => ([low; 3], [high; 3]),
                None => (
                    domain_min.unwrap_or([0.0; 3]),
                    domain_max.unwrap_or([1.0; 3]),
                ),
            };
            ensure!(
                (0..3).all(|c| min[c] < max[c]),
                "Expected each domain minimum below its maximum"
            );
            Ok((min, max))
        };
        let shaper_rows = size_1d.unwrap_or(0);
        let expected = shaper_rows + size_3d.map_or(0, |n| n.pow(3));
        ensure!(
            rows.len() == expected,
            "Expected {expected} data rows, found {}",
            rows.len()
        );
        ensure!(
            rows.iter().flatten().all(|v| v.is_finite()),
            "Data values exceed the supported range"
        );
        let Some(size) = size_3d else {
            let (domain_min, domain_max) = domain(range_1d)?;
            return Ok(Self {
                title,
                three_d: false,
                size: shaper_rows,
                domain_min,
                domain_max,
                table: rows,
                shaper: None,
                video_input,
                video_output,
                image_layout: None,
            });
        };
        // A Resolve file lists the 1D rows first.
        let table = rows.split_off(shaper_rows);
        let shaper = match size_1d {
            Some(size) => {
                let (domain_min, domain_max) = domain(range_1d)?;
                Some(Shaper {
                    size,
                    domain_min,
                    domain_max,
                    table: rows,
                })
            }
            None => None,
        };
        let (domain_min, domain_max) = domain(range_3d)?;
        Ok(Self {
            title,
            three_d: true,
            size,
            domain_min,
            domain_max,
            table,
            shaper,
            video_input,
            video_output,
            image_layout: None,
        })
    }

    /// Reads a `.cube` file, or a LUT image in PNG or TIFF.
    pub fn read(bytes: &[u8]) -> Result<Self> {
        if bytes.starts_with(crate::png::SIGNATURE) {
            Self::from_image(crate::png::decode(bytes)?)
        } else if crate::tiff::is_tiff(bytes) {
            Self::from_image(crate::tiff::decode(bytes)?)
        } else {
            Self::parse(bytes)
        }
    }

    /// Reads a LUT image: a HALD CLUT, square tiles, or a strip (see `lut_image`).
    pub fn from_image(image: lut_image::Image) -> Result<Self> {
        let (size, table, layout) = lut_image::table(image.width, image.height, image.pixels)?;
        Ok(Self {
            title: None,
            three_d: true,
            size,
            domain_min: [0.0; 3],
            domain_max: [1.0; 3],
            table,
            shaper: None,
            video_input: false,
            video_output: false,
            image_layout: Some(layout),
        })
    }

    /// Applies the LUT with domain clamping: the shaper per channel if there is one, then the
    /// table, trilinear for 3D and per channel for 1D.
    pub fn apply(&self, rgb: Rgb) -> Rgb {
        let rgb = if self.video_input {
            rgb.map(to_video)
        } else {
            rgb
        };
        let out = self.lookup(rgb);
        if self.video_output {
            out.map(from_video)
        } else {
            out
        }
    }

    fn lookup(&self, rgb: Rgb) -> Rgb {
        let rgb = match &self.shaper {
            Some(shaper) => {
                interpolate_1d(&shaper.table, &shaper.domain_min, &shaper.domain_max, rgb)
            }
            None => rgb,
        };
        if !self.three_d {
            return interpolate_1d(&self.table, &self.domain_min, &self.domain_max, rgb);
        }
        let last = (self.size - 1) as f64;
        let position = |c: usize| {
            let unit = (rgb[c] - self.domain_min[c]) / (self.domain_max[c] - self.domain_min[c]);
            let x = unit.clamp(0.0, 1.0) * last;
            let low = (x as usize).min(self.size - 2);
            (low, x - low as f64)
        };
        let (r, g, b) = (position(0), position(1), position(2));
        let n = self.size;
        let mut result = [0.0; 3];
        for corner in 0..8 {
            let (dr, dg, db) = (corner & 1, (corner >> 1) & 1, corner >> 2);
            let weight = [(r, dr), (g, dg), (b, db)]
                .iter()
                .map(|((_, t), d)| if *d == 1 { *t } else { 1.0 - t })
                .product::<f64>();
            if weight == 0.0 {
                continue;
            }
            let entry = self.table[(r.0 + dr) + (g.0 + dg) * n + (b.0 + db) * n * n];
            for (slot, value) in result.iter_mut().zip(entry) {
                *slot += weight * value as f64;
            }
        }
        result
    }
}
