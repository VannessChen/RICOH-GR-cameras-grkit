//! A PNG decoder for LUT images: non-interlaced grayscale or RGB, with or without alpha,
//! at 8 or 16 bits. It uses the zlib inflater that the package writer already depends on.

use anyhow::{bail, ensure, Context, Result};
use std::io::{Read, Write};

use crate::lut_image::{Image, MAX_PIXELS};

pub const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

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
    let rows = decode_rows(bytes)?;
    let (depth, channels) = (rows.depth, rows.channels);
    let sample = |pixel: &[u8], channel: usize| {
        if depth == 8 {
            pixel[channel] as f32 / 255.0
        } else {
            u16::from_be_bytes([pixel[channel * 2], pixel[channel * 2 + 1]]) as f32 / 65535.0
        }
    };
    let pixels = rows
        .data
        .chunks_exact(channels * depth as usize / 8)
        .map(|pixel| std::array::from_fn(|c| sample(pixel, if channels < 3 { 0 } else { c })))
        .collect();
    Ok(Image {
        width: rows.width,
        height: rows.height,
        pixels,
    })
}

/// Decodes a PNG image into 8-bit RGBA samples: its width, its height, and
/// four bytes for each pixel. A 16-bit image keeps the high byte of each
/// sample, and an image without alpha is opaque.
pub fn decode_rgba8(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>)> {
    let rows = decode_rows(bytes)?;
    let (depth, channels) = (rows.depth as usize, rows.channels);
    let mut rgba = Vec::with_capacity(rows.width * rows.height * 4);
    for pixel in rows.data.chunks_exact(channels * depth / 8) {
        let sample = |channel: usize| pixel[channel * depth / 8];
        let (gray, alpha) = (channels < 3, channels == 2 || channels == 4);
        for c in 0..3 {
            rgba.push(sample(if gray { 0 } else { c }));
        }
        rgba.push(if alpha { sample(channels - 1) } else { 255 });
    }
    Ok((rows.width, rows.height, rgba))
}

/// The unfiltered rows of a PNG image.
struct Rows {
    width: usize,
    height: usize,
    depth: u8,
    channels: usize,
    data: Vec<u8>,
}

fn decode_rows(bytes: &[u8]) -> Result<Rows> {
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
    ensure!(
        width
            .checked_mul(height)
            .is_some_and(|pixels| pixels <= MAX_PIXELS),
        "Unsupported PNG image size {width} x {height}"
    );
    let bytes_per_pixel = channels * depth as usize / 8;
    let stride = width
        .checked_mul(bytes_per_pixel)
        .context("The PNG image is too large")?;
    let size = height
        .checked_mul(stride + 1)
        .context("The PNG image is too large")?;
    // The decoder grows the buffer with the data, so a header alone allocates nothing.
    let mut raw = Vec::new();
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

    Ok(Rows {
        width,
        height,
        depth,
        channels,
        data: rows,
    })
}

/// Encodes 16-bit RGB samples as a PNG image without filters.
pub fn encode_rgb16(width: usize, height: usize, samples: &[u16]) -> Result<Vec<u8>> {
    ensure!(
        samples.len() == width * height * 3,
        "Expected {width} x {height} RGB samples"
    );
    let bytes: Vec<u8> = samples.iter().flat_map(|v| v.to_be_bytes()).collect();
    encode(width, height, 16, 2, &bytes)
}

/// Encodes 8-bit RGBA samples as a PNG image without filters.
pub fn encode_rgba8(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>> {
    ensure!(
        rgba.len() == width * height * 4,
        "Expected {width} x {height} RGBA samples"
    );
    encode(width, height, 8, 6, rgba)
}

/// A PNG image of `samples`, rows in order, of the given depth and color type.
fn encode(width: usize, height: usize, depth: u8, color: u8, samples: &[u8]) -> Result<Vec<u8>> {
    let stride = samples.len() / height.max(1);
    let mut raw = Vec::with_capacity(height * (1 + stride));
    for row in samples.chunks_exact(stride.max(1)) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&raw)?;
    let mut header = Vec::with_capacity(13);
    header.extend((width as u32).to_be_bytes());
    header.extend((height as u32).to_be_bytes());
    header.extend([depth, color, 0, 0, 0]);
    let mut bytes = SIGNATURE.to_vec();
    for (kind, body) in [
        (b"IHDR", header),
        (b"IDAT", encoder.finish()?),
        (b"IEND", Vec::new()),
    ] {
        bytes.extend((body.len() as u32).to_be_bytes());
        let start = bytes.len();
        bytes.extend(kind);
        bytes.extend(&body);
        let crc = crc32fast::hash(&bytes[start..]);
        bytes.extend(crc.to_be_bytes());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba8_round_trips() {
        let rgba: Vec<u8> = (0..3 * 2 * 4).map(|i| (i * 37) as u8).collect();
        let png = encode_rgba8(3, 2, &rgba).unwrap();
        assert_eq!(decode_rgba8(&png).unwrap(), (3, 2, rgba));
        // An RGB image is opaque.
        let rgb = encode_rgb16(1, 1, &[0x1234, 0x5678, 0x9abc]).unwrap();
        assert_eq!(
            decode_rgba8(&rgb).unwrap(),
            (1, 1, vec![0x12, 0x56, 0x9a, 255])
        );
    }
}
