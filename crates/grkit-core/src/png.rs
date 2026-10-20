//! A PNG decoder for LUT images: non-interlaced grayscale or RGB, with or without alpha,
//! at 8 or 16 bits. It uses the zlib inflater that the package writer already depends on.

use anyhow::{bail, ensure, Context, Result};
use std::io::Read;

pub const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Decoded samples, 0..1, with gray expanded to RGB and alpha dropped.
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[f32; 3]>,
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap())
}

/// The PNG filter predictor of `a` (left), `b` (up), and `c` (up left).
fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i16 + b as i16 - c as i16;
    let (pa, pb, pc) = (
        (p - a as i16).abs(),
        (p - b as i16).abs(),
        (p - c as i16).abs(),
    );
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

pub fn decode(bytes: &[u8]) -> Result<Image> {
    ensure!(bytes.starts_with(SIGNATURE), "Expected a PNG image");
    let mut at = SIGNATURE.len();
    let mut header = None;
    let mut data = Vec::new();
    loop {
        ensure!(at + 12 <= bytes.len(), "The PNG image is truncated");
        let length = u32_at(bytes, at) as usize;
        let end = at
            .checked_add(12 + length)
            .filter(|end| *end <= bytes.len())
            .context("The PNG image is truncated")?;
        let kind = &bytes[at + 4..at + 8];
        let body = &bytes[at + 8..at + 8 + length];
        ensure!(
            crc32fast::hash(&bytes[at + 4..at + 8 + length]) == u32_at(bytes, end - 4),
            "The PNG image is damaged: a chunk checksum does not match"
        );
        match kind {
            b"IHDR" => {
                ensure!(length == 13 && header.is_none(), "Expected one PNG header");
                header = Some(body.to_vec());
            }
            b"IDAT" => data.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        at = end;
    }
    let header = header.context("Expected a PNG header")?;
    let (width, height) = (u32_at(&header, 0) as usize, u32_at(&header, 4) as usize);
    let (depth, color, interlace) = (header[8], header[9], header[12]);
    let channels = match color {
        0 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        3 => bail!("Expected an RGB or grayscale PNG, found a palette image"),
        _ => bail!("Unsupported PNG color type {color}"),
    };
    ensure!(
        depth == 8 || depth == 16,
        "Expected an 8-bit or 16-bit PNG, found {depth}-bit"
    );
    ensure!(interlace == 0, "Expected a PNG without interlacing");
    ensure!(width > 0 && height > 0, "The PNG image is empty");
    let bytes_per_pixel = channels * depth as usize / 8;
    let stride = width
        .checked_mul(bytes_per_pixel)
        .context("The PNG image is too large")?;
    let size = height
        .checked_mul(stride + 1)
        .context("The PNG image is too large")?;
    let mut raw = Vec::with_capacity(size);
    flate2::read::ZlibDecoder::new(data.as_slice())
        .take(size as u64 + 1)
        .read_to_end(&mut raw)
        .context("The PNG image data is damaged")?;
    ensure!(raw.len() == size, "The PNG image data has the wrong size");

    let mut rows = vec![0u8; height * stride];
    for y in 0..height {
        let filter = raw[y * (stride + 1)];
        let line = &raw[y * (stride + 1) + 1..(y + 1) * (stride + 1)];
        let (done, rest) = rows.split_at_mut(y * stride);
        let up = (y > 0).then(|| &done[(y - 1) * stride..]);
        let row = &mut rest[..stride];
        for x in 0..stride {
            let a = if x >= bytes_per_pixel {
                row[x - bytes_per_pixel]
            } else {
                0
            };
            let b = up.map_or(0, |up| up[x]);
            let c = match up {
                Some(up) if x >= bytes_per_pixel => up[x - bytes_per_pixel],
                _ => 0,
            };
            let predicted = match filter {
                0 => 0,
                1 => a,
                2 => b,
                3 => ((a as u16 + b as u16) / 2) as u8,
                4 => paeth(a, b, c),
                _ => bail!("The PNG image uses an unknown filter {filter}"),
            };
            row[x] = line[x].wrapping_add(predicted);
        }
    }

    let sample = |pixel: &[u8], channel: usize| {
        if depth == 8 {
            pixel[channel] as f32 / 255.0
        } else {
            u16::from_be_bytes([pixel[channel * 2], pixel[channel * 2 + 1]]) as f32 / 65535.0
        }
    };
    let pixels = rows
        .chunks_exact(bytes_per_pixel)
        .map(|pixel| std::array::from_fn(|c| sample(pixel, if channels < 3 { 0 } else { c })))
        .collect();
    Ok(Image {
        width,
        height,
        pixels,
    })
}
