//! Helpers that the LUT tests share: a `.cube` writer, PNG and TIFF writers, and a base gamma
//! table of the firmware's shape.
#![allow(dead_code)]

use grkit_core::cube::{self, Cube};
use grkit_core::fit::{self, Model};
use std::fmt::Write;

pub fn cube_text(size: usize, header: &str, map: impl Fn([f64; 3]) -> [f64; 3]) -> String {
    let mut text = format!("# synthetic\nTITLE \"Synthetic #1\"\nLUT_3D_SIZE {size}\n{header}\n");
    let last = (size - 1) as f64;
    for b in 0..size {
        for g in 0..size {
            for r in 0..size {
                let out = map([r as f64 / last, g as f64 / last, b as f64 / last]);
                writeln!(text, "{:.6} {:.6} {:.6}", out[0], out[1], out[2]).unwrap();
            }
        }
    }
    text
}

pub const GRAY: u8 = 0;
pub const RGB: u8 = 2;
pub const RGBA: u8 = 6;

/// A PNG writer for the tests. Row y uses filter y % 5, so a decode covers each filter.
pub fn png(width: usize, height: usize, color: u8, depth: u8, data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let channels = [1, 0, 3, 0, 2, 0, 4][color as usize];
    let bpp = channels * depth as usize / 8;
    let stride = width * bpp;
    let mut raw = Vec::new();
    for y in 0..height {
        let filter = (y % 5) as u8;
        raw.push(filter);
        let row = &data[y * stride..(y + 1) * stride];
        for x in 0..stride {
            let a = if x >= bpp { row[x - bpp] } else { 0 };
            let b = if y > 0 { data[(y - 1) * stride + x] } else { 0 };
            let c = if y > 0 && x >= bpp {
                data[(y - 1) * stride + x - bpp]
            } else {
                0
            };
            let paeth = {
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
            };
            let predicted = [0, a, b, ((a as u16 + b as u16) / 2) as u8, paeth][filter as usize];
            raw.push(row[x].wrapping_sub(predicted));
        }
    }
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&raw).unwrap();
    let mut header = Vec::new();
    header.extend((width as u32).to_be_bytes());
    header.extend((height as u32).to_be_bytes());
    header.extend([depth, color, 0, 0, 0]);
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    for (kind, body) in [
        (b"IHDR", header),
        (b"IDAT", encoder.finish().unwrap()),
        (b"IEND", vec![]),
    ] {
        bytes.extend((body.len() as u32).to_be_bytes());
        let chunk = [kind.as_slice(), &body].concat();
        bytes.extend(&chunk);
        bytes.extend(crc32fast::hash(&chunk).to_be_bytes());
    }
    bytes
}

/// An analytic stand-in for the firmware table: sRGB over a Reinhard shoulder in the linear
/// index domain, with 18% gray near 0.46 at index 7.8 and white at index 255.
pub fn synthetic_base() -> [u16; 256] {
    let scale = 1.2 * 0.18 / fit::MID_INDEX;
    let white = 255.0 * scale;
    std::array::from_fn(|i| {
        let x = i as f64 * scale;
        let shoulder = x * (1.0 + x / (white * white)) / (1.0 + x);
        (16384.0 * cube::srgb_encode(shoulder)).round() as u16
    })
}

pub fn model() -> Model {
    Model::new(&synthetic_base())
}

/// A baseline TIFF writer for the tests: RGB, two rows per strip, uncompressed or Deflate,
/// with or without the horizontal predictor.
pub fn tiff(
    width: usize,
    height: usize,
    rgb16: &[u16],
    little: bool,
    depth: u16,
    deflate: bool,
    predictor: bool,
) -> Vec<u8> {
    use std::io::Write;
    let u16b = |v: u16| {
        if little {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        }
    };
    let u32b = |v: u32| {
        if little {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        }
    };
    let bytes_per = depth as usize / 8;
    let row = width * 3 * bytes_per;
    let mut rows: Vec<Vec<u8>> = rgb16
        .chunks_exact(width * 3)
        .map(|line| {
            let values: Vec<u16> = line
                .iter()
                .map(|v| {
                    if depth == 8 {
                        ((*v as u32 * 255 + 32767) / 65535) as u16
                    } else {
                        *v
                    }
                })
                .collect();
            let mut out = values.clone();
            if predictor {
                for i in (3..values.len()).rev() {
                    out[i] = values[i].wrapping_sub(values[i - 3]);
                }
            }
            out.iter()
                .flat_map(|v| {
                    if depth == 8 {
                        vec![*v as u8]
                    } else {
                        u16b(*v).to_vec()
                    }
                })
                .collect()
        })
        .collect();
    let strips: Vec<Vec<u8>> = rows
        .chunks_mut(2)
        .map(|chunk| {
            let raw = chunk.concat();
            assert_eq!(raw.len(), chunk.len() * row);
            if deflate {
                let mut e =
                    flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                e.write_all(&raw).unwrap();
                e.finish().unwrap()
            } else {
                raw
            }
        })
        .collect();
    let mut data = vec![];
    let mut offsets = vec![];
    for strip in &strips {
        offsets.push(8 + data.len() as u32);
        data.extend(strip);
    }
    let strip_table = 8 + data.len() as u32;
    for o in &offsets {
        data.extend(u32b(*o));
    }
    let size_table = 8 + data.len() as u32;
    for s in &strips {
        data.extend(u32b(s.len() as u32));
    }
    let bits_at = 8 + data.len() as u32;
    for _ in 0..3 {
        data.extend(u16b(depth));
    }
    let ifd = 8 + data.len() as u32;
    let entries: Vec<(u16, u16, u32, u32)> = vec![
        (256, 4, 1, width as u32),
        (257, 4, 1, height as u32),
        (258, 3, 3, bits_at),
        (259, 3, 1, if deflate { 8 } else { 1 }),
        (262, 3, 1, 2),
        (273, 4, strips.len() as u32, strip_table),
        (277, 3, 1, 3),
        (278, 4, 1, 2),
        (279, 4, strips.len() as u32, size_table),
        (317, 3, 1, if predictor { 2 } else { 1 }),
    ];
    let mut out = if little {
        b"II*\0".to_vec()
    } else {
        b"MM\0*".to_vec()
    };
    out.extend(u32b(ifd));
    out.extend(data);
    out.extend(u16b(entries.len() as u16));
    for (tag, kind, count, value) in entries {
        out.extend(u16b(tag));
        out.extend(u16b(kind));
        out.extend(u32b(count));
        if kind == 3 && count == 1 {
            out.extend(u16b(value as u16));
            out.extend([0, 0]);
        } else {
            out.extend(u32b(value));
        }
    }
    out.extend([0; 4]);
    out
}

pub fn parse(text: &str) -> anyhow::Result<Cube> {
    Cube::parse(text.as_bytes())
}
