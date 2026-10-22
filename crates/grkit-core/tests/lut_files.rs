//! `.cube` parsing and LUT images in PNG and TIFF.

mod common;

use common::*;
use grkit_core::cube::{self, Cube};
use grkit_core::lut_image;

fn error(text: &str) -> String {
    format!("{:#}", parse(text).unwrap_err())
}

#[test]
fn parses_3d_tables_with_comments_and_trilinear_lookup() {
    let text = cube_text(2, "# corner order: red fastest", |rgb| rgb.map(|v| v * 0.5));
    let cube = parse(&format!("\u{feff}{text}\n# trailing comment\n")).unwrap();
    assert!(cube.three_d);
    assert_eq!(
        (cube.size, cube.title.as_deref()),
        (2, Some("Synthetic #1"))
    );
    assert_eq!(cube.table[1], [0.5, 0.0, 0.0]);
    assert_eq!(cube.table[2], [0.0, 0.5, 0.0]);
    assert_eq!(cube.table[4], [0.0, 0.0, 0.5]);
    let out = cube.apply([0.25, 0.5, 1.5]);
    for (actual, expected) in out.iter().zip([0.125, 0.25, 0.5]) {
        assert!((actual - expected).abs() < 1e-7, "{out:?}");
    }
}

#[test]
fn parses_1d_tables_and_domains() {
    let text =
        "LUT_1D_SIZE 3\nDOMAIN_MIN 0 0 -1\nDOMAIN_MAX 2 2 1\n0 0 0\n0.2 0.4 0.6 # middle\n1 1 1\n";
    let cube = parse(text).unwrap();
    assert!(!cube.three_d);
    let out = cube.apply([1.0, 0.5, 0.0]);
    for (actual, expected) in out.iter().zip([0.2, 0.2, 0.6]) {
        assert!((actual - expected).abs() < 1e-6, "{out:?}");
    }
    assert_eq!(cube.apply([3.0, -1.0, 9.0]), [1.0, 0.0, 1.0]);
    let ranged = parse("LUT_1D_SIZE 2\nLUT_1D_INPUT_RANGE 0 4\n0 0 0\n1 1 1\n").unwrap();
    assert!((ranged.apply([1.0; 3])[0] - 0.25).abs() < 1e-9);
    let shaped = parse(&cube_text(
        2,
        "DOMAIN_MIN -1 -1 -1\nDOMAIN_MAX 1 1 1",
        |v| v,
    ))
    .unwrap();
    assert!((shaped.apply([0.0; 3])[1] - 0.5).abs() < 1e-9);
}

#[test]
fn rejects_malformed_tables() {
    let rows = "0 0 0\n1 0 0\n0 1 0\n1 1 0\n0 0 1\n1 0 1\n0 1 1\n";
    for (text, message) in [
        (
            format!("LUT_3D_SIZE 2\n{rows}"),
            "Expected 8 data rows, found 7",
        ),
        (
            format!("LUT_3D_SIZE 2\n{rows}1 1\n"),
            "Line 9: A data row expects 3 values",
        ),
        (
            format!("LUT_3D_SIZE 2\n{rows}1 1 1e999\n"),
            "Line 9: expected a finite number",
        ),
        (
            format!("LUT_3D_SIZE 2\n{rows}1 1 NaN\n"),
            "Line 9: expected a finite number",
        ),
        (
            format!("LUT_3D_SIZE 2\n{rows}1 1 x\n"),
            "Line 9: expected a finite number",
        ),
        (
            "LUT_3D_SIZE 1\n0 0 0\n".into(),
            "LUT_3D_SIZE must be 2 to 256",
        ),
        ("LUT_3D_SIZE 257\n".into(), "LUT_3D_SIZE must be 2 to 256"),
        (
            "LUT_3D_SIZE 2\nLUT_3D_SIZE 2\n".into(),
            "duplicate LUT_3D_SIZE",
        ),
        (
            format!("LUT_3D_SIZE 2\nLUT_1D_SIZE 2\n{rows}"),
            "Expected 10 data rows, found 7",
        ),
        ("0 0 0\n".into(), "Expected LUT_3D_SIZE or LUT_1D_SIZE"),
        (
            "LUT_1D_SIZE 2\n0 0 0\nDOMAIN_MIN 0 0 0\n1 1 1\n".into(),
            "after data rows",
        ),
        (
            "LUT_1D_SIZE 2\nDOMAIN_MAX 0 1 1\n0 0 0\n1 1 1\n".into(),
            "domain minimum below",
        ),
        (
            "LUT_1D_SIZE 2\nDOMAIN_MIN 0 0\n".into(),
            "DOMAIN_MIN expects 3 values",
        ),
        (
            "LUT_1D_SIZE 2\nLUT_3D_INPUT_RANGE 0 1\n".into(),
            "does not match",
        ),
        (
            "LUT_1D_SIZE 2\n@x 1\n".into(),
            "Line 2: expected a keyword or a data row",
        ),
        (
            "LUT_1D_SIZE 2\nFOO_BAR 3 4\n".into(),
            "Line 2: unsupported keyword FOO_BAR",
        ),
        (
            "LUT_1D_SIZE 2\nLUT_IN_VIDEO_RANGE 1\n".into(),
            "LUT_IN_VIDEO_RANGE takes no values",
        ),
        (
            "LUT_1D_SIZE 2\nLUT_3D_SIZE 2\nDOMAIN_MAX 2 2 2\n".into(),
            "ambiguous with a 1D and a 3D table",
        ),
        (
            "LUT_1D_SIZE 2\nLUT_3D_INPUT_RANGE 0 1\nLUT_1D_INPUT_RANGE 0 1\n".into(),
            "LUT_3D_INPUT_RANGE does not match",
        ),
    ] {
        let actual = error(&text);
        assert!(actual.contains(message), "{actual:?} lacks {message:?}");
    }
    assert!(Cube::parse(b"LUT_1D_SIZE 2\n\xff\n").is_err());
}

#[test]
fn parses_resolve_shapers_and_lenient_syntax() {
    // A 3-point shaper maps 0, 1, 2 to 0, 1, 4, linear between. A 2-point 3D table over 0..4
    // then halves the value. Keywords in lower
    // case, an unquoted title, and comma-separated rows come from real exporters.
    let text = "title Shaped # not a comment\nlut_1d_size 3\nLUT_1D_INPUT_RANGE 0 2\n\
                LUT_3D_SIZE 2\nLUT_3D_INPUT_RANGE 0 4\n\
                0, 0, 0\n1, 1, 1\n4, 4, 4\n";
    let mut text = text.to_owned();
    for b in 0..2 {
        for g in 0..2 {
            for r in 0..2 {
                text.push_str(&format!("{} {} {}\n", r * 2, g * 2, b * 2));
            }
        }
    }
    let cube = parse(&text).unwrap();
    assert_eq!(cube.title.as_deref(), Some("Shaped # not a comment"));
    assert!(cube.three_d && cube.size == 2 && cube.table.len() == 8);
    let shaper = cube.shaper.as_ref().unwrap();
    assert_eq!((shaper.size, shaper.domain_max), (3, [2.0; 3]));
    assert_eq!(cube.domain_max, [4.0; 3]);
    // Input 1 maps to 1 in the shaper, then to 1 / 4 of the 3D range, which is 0.5.
    for (input, expected) in [(1.0, 0.5), (2.0, 2.0), (0.5, 0.25)] {
        let out = cube.apply([input; 3]);
        assert!(
            out.iter().all(|v| (v - expected).abs() < 1e-6),
            "{input}: {out:?}"
        );
    }
    let unshaped = parse(&cube_text(2, "", |v| v)).unwrap();
    assert!(unshaped.shaper.is_none());
    // A DESCRIPTION line is text, whatever it holds.
    let described = parse(&cube_text(
        2,
        "DESCRIPTION \"Night street, 3 # 4\"\n",
        |v| v,
    ))
    .unwrap();
    assert_eq!(described.apply([0.25; 3]), [0.25; 3]);
}

#[test]
fn converts_resolve_video_range() {
    let header = |flags: &str| format!("LUT_3D_SIZE 2\n{flags}");
    let identity = |flags: &str| {
        parse(&cube_text(2, "", |v| v).replacen("LUT_3D_SIZE 2\n", &header(flags), 1)).unwrap()
    };
    // Video in only: data level 0 looks up the table at 64 / 1023.
    let input = identity("LUT_IN_VIDEO_RANGE\n");
    assert!(input.video_input && !input.video_output);
    assert!((input.apply([0.0; 3])[0] - 64.0 / 1023.0).abs() < 1e-9);
    // Video in and out cancel for an identity table.
    let both = identity("lut_in_video_range\nLUT_OUT_VIDEO_RANGE\n");
    for v in [0.0, 0.3, 1.0] {
        assert!(both.apply([v; 3]).iter().all(|x| (x - v).abs() < 1e-9));
    }
    // Video out expands 940 / 1023 to 1.
    let output = identity("LUT_OUT_VIDEO_RANGE\n");
    assert!((output.apply([940.0 / 1023.0; 3])[2] - 1.0).abs() < 1e-9);
}

#[test]
fn reads_cr_line_ends_and_legacy_comment_bytes() {
    let text = cube_text(2, "", |v| v).replace('\n', "\r");
    assert!(parse(&text).unwrap().three_d);
    let mut bytes = b"# caf\xe9 LUT\r\n".to_vec();
    bytes.extend(cube_text(2, "", |v| v).replace('\n', "\r\n").bytes());
    assert_eq!(Cube::parse(&bytes).unwrap().size, 2);
}

#[test]
fn reads_hald_images_in_cube_order() {
    // Level 2: a 4-point LUT in an 8 x 8 image, red changing fastest as in a .cube table.
    let level = |i: usize| (i * 85) as u8;
    let mut rgb = Vec::new();
    let mut rgba16 = Vec::new();
    for b in 0..4 {
        for g in 0..4 {
            for r in 0..4 {
                let out = [level(r), level(g) / 2, level(b)];
                rgb.extend(out);
                for v in out {
                    rgba16.extend((v as u16 * 257).to_be_bytes());
                }
                rgba16.extend([255, 255]);
            }
        }
    }
    let eight = png(8, 8, RGB, 8, &rgb);
    let sixteen = png(8, 8, RGBA, 16, &rgba16);
    for bytes in [&eight, &sixteen] {
        let cube = Cube::read(bytes).unwrap();
        assert!(cube.three_d && cube.size == 4);
        assert_eq!(cube.table[1], [85.0 / 255.0, 0.0, 0.0]);
        assert_eq!(cube.table[4], [0.0, 42.0 / 255.0, 0.0]);
        let out = cube.apply([1.0, 1.0, 1.0 / 3.0]);
        for (a, e) in out.iter().zip([1.0, 127.0 / 255.0, 85.0 / 255.0]) {
            assert!((a - e).abs() < 1e-6, "{out:?}");
        }
    }
    // A gray HALD: luma of the lattice point, so each axis changes the value.
    let gray: Vec<u8> = (0..64)
        .map(|i| (i % 4 * 85 * 3 / 10 + (i / 4 % 4) * 85 * 6 / 10 + (i / 16) * 85 / 10) as u8)
        .collect();
    let cube = Cube::read(&png(8, 8, GRAY, 8, &gray)).unwrap();
    assert_eq!(cube.image_layout, Some(cube::Layout::Hald));
    assert_eq!(cube.table[63], [gray[63] as f32 / 255.0; 3]);
    let error = |bytes: &[u8]| format!("{:#}", Cube::read(bytes).unwrap_err());
    let message = error(&png(8, 4, RGB, 8, &rgb[..96]));
    assert!(message.contains("8 x 4"), "{message}");
    let mut damaged = eight.clone();
    let last = damaged.len() - 20;
    damaged[last] ^= 1;
    assert!(error(&damaged).contains("checksum"));
    assert!(error(&eight[..eight.len() - 12]).contains("truncated"));
    // A constant image fits every layout equally well.
    let message = error(&png(8, 8, RGB, 8, &[128; 192]));
    assert!(message.contains("Cannot tell the layout"), "{message}");
}

/// The 8-bit pixels of a LUT image with lattice size `c`, written by `place`.
fn layout_image(
    width: usize,
    height: usize,
    c: usize,
    map: impl Fn([f64; 3]) -> [f64; 3],
    place: impl Fn([usize; 3]) -> (usize, usize),
) -> Vec<u8> {
    let mut data = vec![0u8; width * height * 3];
    for b in 0..c {
        for g in 0..c {
            for r in 0..c {
                let out = map([r, g, b].map(|v| v as f64 / (c - 1) as f64));
                let (x, y) = place([r, g, b]);
                for k in 0..3 {
                    data[(x + y * width) * 3 + k] = (out[k] * 255.0).round() as u8;
                }
            }
        }
    }
    data
}

#[test]
fn reads_tiled_and_strip_lut_images() {
    let look = |[r, g, b]: [f64; 3]| [0.9 * r + 0.1 * g, 0.8 * g + 0.1, 0.7 * b + 0.2 * r];
    let check = |bytes: &[u8], layout: cube::Layout| {
        let cube = Cube::read(bytes).unwrap();
        assert_eq!(cube.image_layout, Some(layout));
        for rgb in [[0.2, 0.5, 0.9], [1.0, 0.0, 0.4]] {
            let (out, expected) = (cube.apply(rgb), look(rgb));
            assert!(
                out.iter().zip(expected).all(|(a, e)| (a - e).abs() < 0.01),
                "{out:?}"
            );
        }
    };
    // 8 x 8 tiles of 64 x 64 (OBS), the size of a level 8 HALD.
    let tiles = layout_image(512, 512, 64, look, |[r, g, b]| {
        (r + 64 * (b % 8), g + 64 * (b / 8))
    });
    check(&png(512, 512, RGB, 8, &tiles), cube::Layout::SquareTiles);
    // A 16-point strip, and the same strip with green from the bottom up.
    let strip = layout_image(256, 16, 16, look, |[r, g, b]| (r + 16 * b, g));
    check(&png(256, 16, RGB, 8, &strip), cube::Layout::Strip);
    let flipped = layout_image(256, 16, 16, look, |[r, g, b]| (r + 16 * b, 15 - g));
    check(&png(256, 16, RGB, 8, &flipped), cube::Layout::FlippedStrip);
    // A hue rotation keeps green flat along the green input; the gray axis still tells the
    // orientation.
    let rotate = |[r, g, b]: [f64; 3]| [g, b, r];
    let check_rotation = |bytes: &[u8], layout: cube::Layout| {
        let cube = Cube::read(bytes).unwrap();
        assert_eq!(cube.image_layout, Some(layout));
        let out = cube.apply([0.2, 0.5, 0.9]);
        assert!(
            out.iter()
                .zip([0.5, 0.9, 0.2])
                .all(|(a, e)| (a - e).abs() < 0.01),
            "{out:?}"
        );
    };
    let strip = layout_image(256, 16, 16, rotate, |[r, g, b]| (r + 16 * b, g));
    check_rotation(&png(256, 16, RGB, 8, &strip), cube::Layout::Strip);
    let flipped = layout_image(256, 16, 16, rotate, |[r, g, b]| (r + 16 * b, 15 - g));
    check_rotation(&png(256, 16, RGB, 8, &flipped), cube::Layout::FlippedStrip);
    // A clean 16-point table of a strong curve bends more between its coarse steps than a
    // noise-free fine table may; the limit for its size allows it.
    let curve = |rgb: [f64; 3]| rgb.map(|v| v.powf(1.0 / 2.2));
    let strip = layout_image(256, 16, 16, curve, |[r, g, b]| (r + 16 * b, g));
    let cube = Cube::read(&png(256, 16, RGB, 8, &strip)).unwrap();
    let noise = lut_image::noise(&cube.table, cube.size);
    assert!(
        noise > lut_image::NOISE_LIMIT && noise < lut_image::noise_limit(16),
        "{noise}"
    );
    // The same look as a level 4 HALD stays a HALD.
    let hald = layout_image(64, 64, 16, look, |[r, g, b]| {
        let i = r + 16 * g + 256 * b;
        (i % 64, i / 64)
    });
    check(&png(64, 64, RGB, 8, &hald), cube::Layout::Hald);
}

#[test]
fn image_headers_too_large_fail_before_decoding() {
    // A 1 x 1 PNG whose header claims 32768 x 32768 pixels, with the CRC updated.
    let mut bytes = png(1, 1, RGB, 8, &[0, 0, 0]);
    bytes[16..20].copy_from_slice(&32768u32.to_be_bytes());
    bytes[20..24].copy_from_slice(&32768u32.to_be_bytes());
    let crc = crc32fast::hash(&bytes[12..29]);
    bytes[29..33].copy_from_slice(&crc.to_be_bytes());
    let error = Cube::read(&bytes).err().unwrap().to_string();
    assert!(error.contains("Unsupported PNG image size"), "{error}");
}

#[test]
fn identity_hald_round_trips_through_png_and_tiff() {
    assert!(lut_image::identity_png(1).is_err() && lut_image::identity_png(17).is_err());
    let png = lut_image::identity_png(4).unwrap();
    let cube = Cube::read(&png).unwrap();
    assert_eq!(
        (cube.size, cube.image_layout),
        (16, Some(cube::Layout::Hald))
    );
    for (i, entry) in cube.table.iter().enumerate() {
        let expected = [i % 16, i / 16 % 16, i / 256].map(|v| v as f32 / 15.0);
        assert!(
            entry
                .iter()
                .zip(expected)
                .all(|(a, e)| (a - e).abs() < 1e-4),
            "{i}"
        );
    }
    assert!(lut_image::noise(&cube.table, cube.size) < 1e-6);
    // A graded look stored as TIFF in each supported variant reads as the PNG does.
    let look = |v: f64| (0.05 + 0.9 * v.powf(0.8)).min(1.0);
    let samples: Vec<u16> = cube
        .table
        .iter()
        .flat_map(|e| e.map(|v| (look(v as f64) * 65535.0).round() as u16))
        .collect();
    for (little, depth, deflate, predictor) in [
        (true, 8, false, false),
        (true, 16, true, true),
        (false, 16, true, false),
        (false, 8, true, true),
    ] {
        let bytes = tiff(64, 64, &samples, little, depth, deflate, predictor);
        let read = Cube::read(&bytes).unwrap();
        let tolerance = if depth == 8 { 0.5 / 255.0 + 1e-6 } else { 1e-6 };
        for (entry, expected) in read.table.iter().zip(samples.chunks_exact(3)) {
            for (a, e) in entry.iter().zip(expected) {
                assert!(
                    (a - *e as f32 / 65535.0).abs() <= tolerance as f32,
                    "{little} {depth}"
                );
            }
        }
    }
}
