//! A TIFF decoder for LUT images, such as a HALD image that Lightroom exports: baseline RGB or
//! grayscale in strips, 8 or 16 bits, uncompressed, LZW, or Deflate, with or without the
//! horizontal predictor. Alpha is dropped. Tiles, planar files, and float samples fail.

use anyhow::{bail, ensure, Context, Result};
use std::io::Read;

use crate::lut_image::{Image, MAX_PIXELS};

pub fn is_tiff(bytes: &[u8]) -> bool {
    bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*")
}

struct Reader<'a> {
    bytes: &'a [u8],
    little: bool,
}

impl Reader<'_> {
    fn u16(&self, at: usize) -> Result<u16> {
        let b: [u8; 2] = self
            .bytes
            .get(at..at + 2)
            .context("The TIFF image is truncated")?
            .try_into()
            .unwrap();
        Ok(if self.little {
            u16::from_le_bytes(b)
        } else {
            u16::from_be_bytes(b)
        })
    }

    fn u32(&self, at: usize) -> Result<u32> {
        let b: [u8; 4] = self
            .bytes
            .get(at..at + 4)
            .context("The TIFF image is truncated")?
            .try_into()
            .unwrap();
        Ok(if self.little {
            u32::from_le_bytes(b)
        } else {
            u32::from_be_bytes(b)
        })
    }

    /// The values of an IFD entry of type SHORT or LONG.
    fn values(&self, entry: usize) -> Result<Vec<u32>> {
        let (kind, count) = (self.u16(entry + 2)?, self.u32(entry + 4)? as usize);
        let size = match kind {
            3 => 2,
            4 => 4,
            _ => bail!("Unsupported TIFF field type {kind}"),
        };
        ensure!(count <= 1 << 20, "The TIFF image has too many strips");
        let start = if size * count <= 4 {
            entry + 8
        } else {
            self.u32(entry + 8)? as usize
        };
        (0..count)
            .map(|i| match size {
                2 => self.u16(start + 2 * i).map(u32::from),
                _ => self.u32(start + 4 * i),
            })
            .collect()
    }
}

/// TIFF LZW: MSB-first codes of 9 to 12 bits, with the code width growing one code early.
fn lzw(data: &[u8], limit: usize) -> Result<Vec<u8>> {
    const CLEAR: usize = 256;
    const END: usize = 257;
    let mut out = Vec::new();
    let mut table: Vec<Vec<u8>> = Vec::new();
    let reset = |table: &mut Vec<Vec<u8>>| {
        table.clear();
        table.extend((0..=255u8).map(|b| vec![b]));
        table.extend([vec![], vec![]]);
    };
    reset(&mut table);
    let (mut bits, mut buffer, mut at) = (0u32, 0u32, 0usize);
    let mut previous: Option<usize> = None;
    loop {
        let width = match table.len() + 1 {
            0..=511 => 9,
            512..=1023 => 10,
            1024..=2047 => 11,
            _ => 12,
        };
        while bits < width {
            let Some(&byte) = data.get(at) else {
                return Ok(out);
            };
            buffer = (buffer << 8) | byte as u32;
            bits += 8;
            at += 1;
        }
        let code = ((buffer >> (bits - width)) & ((1 << width) - 1)) as usize;
        bits -= width;
        if code == END {
            return Ok(out);
        }
        if code == CLEAR {
            reset(&mut table);
            previous = None;
            continue;
        }
        let entry = match (table.get(code), previous) {
            (Some(entry), _) if code != CLEAR && code != END => entry.clone(),
            (None, Some(p)) if code == table.len() => {
                let mut e = table[p].clone();
                e.push(table[p][0]);
                e
            }
            _ => bail!("The TIFF image data is damaged"),
        };
        out.extend_from_slice(&entry);
        ensure!(out.len() <= limit, "The TIFF image data is too large");
        if let Some(p) = previous {
            if table.len() < 4096 {
                let mut e = table[p].clone();
                e.push(entry[0]);
                table.push(e);
            }
        }
        previous = Some(code);
    }
}

pub fn decode(bytes: &[u8]) -> Result<Image> {
    ensure!(is_tiff(bytes), "Expected a TIFF image");
    let r = Reader {
        bytes,
        little: bytes[0] == b'I',
    };
    let ifd = r.u32(4)? as usize;
    let count = r.u16(ifd)? as usize;
    let mut fields = std::collections::HashMap::new();
    for i in 0..count {
        let entry = ifd + 2 + 12 * i;
        let tag = r.u16(entry)?;
        // Skip fields of other types, such as ASCII text and the ICC profile.
        if matches!(r.u16(entry + 2)?, 3 | 4) {
            fields.insert(tag, r.values(entry)?);
        }
    }
    let one = |tag: u16, default: Option<u32>| -> Result<u32> {
        fields
            .get(&tag)
            .and_then(|v| v.first().copied())
            .or(default)
            .with_context(|| format!("The TIFF image lacks field {tag}"))
    };
    ensure!(
        !fields.contains_key(&322),
        "Expected a TIFF image in strips, found tiles"
    );
    let (width, height) = (one(256, None)? as usize, one(257, None)? as usize);
    let samples = one(277, Some(1))? as usize;
    let depth = one(258, Some(1))?;
    let photometric = one(262, None)?;
    let compression = one(259, Some(1))?;
    let predictor = one(317, Some(1))?;
    ensure!(
        width > 0
            && height > 0
            && width
                .checked_mul(height)
                .is_some_and(|pixels| pixels <= MAX_PIXELS),
        "Unsupported TIFF image size {width} x {height}"
    );
    ensure!(
        fields
            .get(&258)
            .is_none_or(|v| v.iter().all(|d| *d == depth)),
        "Expected the same bit depth in each TIFF channel"
    );
    ensure!(
        depth == 8 || depth == 16,
        "Expected an 8-bit or 16-bit TIFF image, found {depth}-bit"
    );
    ensure!(one(339, Some(1))? == 1, "Expected integer TIFF samples");
    ensure!(
        one(284, Some(1))? == 1,
        "Expected a TIFF image with interleaved channels"
    );
    let color = match (photometric, samples) {
        (2, 3 | 4) => 3,
        (1, 1 | 2) => 1,
        _ => bail!("Expected an RGB or grayscale TIFF image"),
    };
    ensure!(
        predictor == 1 || predictor == 2,
        "Unsupported TIFF predictor {predictor}"
    );
    let bytes_per_sample = depth as usize / 8;
    let row = width * samples * bytes_per_sample;
    let rows_per_strip = (one(278, Some(height as u32))? as usize).clamp(1, height);
    let offsets = fields.get(&273).context("The TIFF image lacks strips")?;
    let lengths = fields
        .get(&279)
        .context("The TIFF image lacks strip sizes")?;
    ensure!(
        offsets.len() == lengths.len() && offsets.len() == height.div_ceil(rows_per_strip),
        "The TIFF strips do not match the image height"
    );

    let mut raw = Vec::with_capacity(row * height);
    for (strip, (&offset, &length)) in offsets.iter().zip(lengths).enumerate() {
        let data = bytes
            .get(offset as usize..(offset as usize).saturating_add(length as usize))
            .context("The TIFF image is truncated")?;
        let rows = rows_per_strip.min(height - strip * rows_per_strip);
        let size = rows * row;
        let mut part = match compression {
            1 => data.to_vec(),
            5 => lzw(data, size)?,
            8 | 32946 => {
                let mut out = Vec::with_capacity(size);
                flate2::read::ZlibDecoder::new(data)
                    .take(size as u64)
                    .read_to_end(&mut out)
                    .context("The TIFF image data is damaged")?;
                out
            }
            _ => bail!("Unsupported TIFF compression {compression}"),
        };
        ensure!(part.len() >= size, "The TIFF image data has the wrong size");
        part.truncate(size);
        if predictor == 2 {
            for line in part.chunks_exact_mut(row) {
                undo_predictor(line, samples, bytes_per_sample, r.little);
            }
        }
        raw.extend_from_slice(&part);
    }

    let sample = |at: usize| -> f32 {
        if bytes_per_sample == 1 {
            raw[at] as f32 / 255.0
        } else {
            let b = [raw[at], raw[at + 1]];
            let v = if r.little {
                u16::from_le_bytes(b)
            } else {
                u16::from_be_bytes(b)
            };
            v as f32 / 65535.0
        }
    };
    let pixel_size = samples * bytes_per_sample;
    let pixels = (0..width * height)
        .map(|i| {
            std::array::from_fn(|c| {
                let channel = if color == 1 { 0 } else { c };
                sample(i * pixel_size + channel * bytes_per_sample)
            })
        })
        .collect();
    Ok(Image {
        width,
        height,
        pixels,
    })
}

/// Horizontal differencing: each sample stores its difference from the one to its left.
fn undo_predictor(line: &mut [u8], samples: usize, bytes_per_sample: usize, little: bool) {
    if bytes_per_sample == 1 {
        for i in samples..line.len() {
            line[i] = line[i].wrapping_add(line[i - samples]);
        }
        return;
    }
    let read = |b: [u8; 2]| {
        if little {
            u16::from_le_bytes(b)
        } else {
            u16::from_be_bytes(b)
        }
    };
    let stride = samples * 2;
    for i in (stride..line.len()).step_by(2) {
        let left = read([line[i - stride], line[i - stride + 1]]);
        let value = read([line[i], line[i + 1]]).wrapping_add(left);
        let b = if little {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        };
        line[i..i + 2].copy_from_slice(&b);
    }
}
