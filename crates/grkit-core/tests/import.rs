use grkit_core::cube::{self, Cube, Encoding};
use grkit_core::effects;
use grkit_core::fit::{self, Import, Model, Options, Params, IDENTITY};
use std::fmt::Write;

fn cube_text(size: usize, header: &str, map: impl Fn([f64; 3]) -> [f64; 3]) -> String {
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

fn parse(text: &str) -> anyhow::Result<Cube> {
    Cube::parse(text.as_bytes())
}

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

const GRAY: u8 = 0;
const RGB: u8 = 2;
const RGBA: u8 = 6;

/// A PNG writer for the tests. Row y uses filter y % 5, so a decode covers each filter.
fn png(width: usize, height: usize, color: u8, depth: u8, data: &[u8]) -> Vec<u8> {
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
fn lut_image(
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
    let tiles = lut_image(512, 512, 64, look, |[r, g, b]| {
        (r + 64 * (b % 8), g + 64 * (b / 8))
    });
    check(&png(512, 512, RGB, 8, &tiles), cube::Layout::SquareTiles);
    // A 16-point strip, and the same strip with green from the bottom up.
    let strip = lut_image(256, 16, 16, look, |[r, g, b]| (r + 16 * b, g));
    check(&png(256, 16, RGB, 8, &strip), cube::Layout::Strip);
    let flipped = lut_image(256, 16, 16, look, |[r, g, b]| (r + 16 * b, 15 - g));
    check(&png(256, 16, RGB, 8, &flipped), cube::Layout::FlippedStrip);
    // The same look as a level 4 HALD stays a HALD.
    let hald = lut_image(64, 64, 16, look, |[r, g, b]| {
        let i = r + 16 * g + 256 * b;
        (i % 64, i / 64)
    });
    check(&png(64, 64, RGB, 8, &hald), cube::Layout::Hald);
}

#[test]
fn color_transforms_match_published_values() {
    for v in [0.0, 0.002, 0.18, 0.5, 1.0] {
        assert!((cube::srgb_decode(cube::srgb_encode(v)) - v).abs() < 1e-12);
    }
    // V-Log: 0 % reflectance at code 128 / 1023 full range, 18 % at 433 / 1023.
    assert!((cube::vlog_encode(0.0) - 0.125).abs() < 1e-12);
    assert!((cube::vlog_encode(0.18) * 1023.0 - 433.0).abs() < 0.5);
    for v in [0.0, 0.005, 0.18, 4.0] {
        assert!((cube::vlog_decode(cube::vlog_encode(v)) - v).abs() < 1e-9);
    }
    // F-Log: 0 % at 95 / 1023, 18 % at 470, 90 % at 705. F-Log2: 95, 400, 570.
    for (encode, decode, codes) in [
        (
            cube::flog_encode as fn(f64) -> f64,
            cube::flog_decode as fn(f64) -> f64,
            [95.0, 470.0, 705.0],
        ),
        (cube::flog2_encode, cube::flog2_decode, [95.0, 400.0, 570.0]),
    ] {
        for (v, code) in [0.0, 0.18, 0.9].into_iter().zip(codes) {
            assert!(
                (encode(v) * 1023.0 - code).abs() < 0.6,
                "{v}: {}",
                encode(v) * 1023.0
            );
        }
        for v in [0.0, 0.0005, 0.18, 4.0] {
            assert!((decode(encode(v)) - v).abs() < 1e-9);
        }
    }
    // Linear F-Gamut C to BT.709 from the data sheet primaries, D65 on both sides.
    let to709 = cube::fgamut_c_to_srgb();
    for (actual, expected) in to709[0].iter().zip([2.119851, -1.075705, -0.044146]) {
        assert!((actual - expected).abs() < 1e-5, "{to709:?}");
    }
    // The data, not the file name, tells the input: a LUT made for an input keeps the
    // Standard rendering when it reads that input.
    let model = model();
    let made_for = |input: Encoding| {
        let text = cube_text(33, "", |x| model.standard(model.decode(input, x)));
        Cube::read(text.as_bytes()).unwrap()
    };
    for input in [Encoding::Vlog, Encoding::Flog, Encoding::Flog2] {
        assert_eq!(
            fit::guess_input(&synthetic_base(), &made_for(input)),
            input,
            "{input:?}"
        );
    }
    // A look over a GR JPEG, and the identity, read Standard.
    let identity = Cube::read(cube_text(17, "", |x| x).as_bytes()).unwrap();
    assert_eq!(
        fit::guess_input(&synthetic_base(), &identity),
        Encoding::Standard
    );
    let warm = cube_text(17, "", |x| [x[0] * 0.95 + 0.05, x[1], x[2] * 0.9]);
    assert_eq!(
        fit::guess_input(&synthetic_base(), &Cube::read(warm.as_bytes()).unwrap()),
        Encoding::Standard
    );
    let gray = cube::mul(&cube::srgb_to_fgamut(), [0.18; 3]);
    assert!(gray.iter().all(|v| (v - 0.18).abs() < 1e-9));
    let gray = cube::mul(&cube::srgb_to_vgamut(), [0.18; 3]);
    assert!(gray.iter().all(|v| (v - 0.18).abs() < 1e-9));
    let red = cube::mul(&cube::srgb_to_vgamut(), [1.0, 0.0, 0.0]);
    assert!(red[0] < 1.0 && red[1] > 0.0 && red[2] > 0.0);
    // Sharma, Wu, and Dalal (2005) test pairs 1, 7, and 17.
    for (a, b, expected) in [
        ([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485], 2.0425),
        ([50.0, 0.0, 0.0], [50.0, -1.0, 2.0], 2.3669),
        ([50.0, 2.5, 0.0], [73.0, 25.0, -18.0], 27.1492),
    ] {
        assert!((cube::delta_e2000(a, b) - expected).abs() < 1e-4);
    }
    let chart = fit::colorchecker();
    let white = cube::LabConverter::default().lab(chart[18].map(cube::srgb_encode));
    assert!((white[0] - 95.19).abs() < 0.5 && white[1].abs() < 1.5 && white[2].abs() < 3.5);
}

/// An analytic stand-in for the firmware table: sRGB over a Reinhard shoulder in the linear
/// index domain, with 18% gray near 0.46 at index 7.8 and white at index 255.
fn synthetic_base() -> [u16; 256] {
    let scale = 1.2 * 0.18 / fit::MID_INDEX;
    let white = 255.0 * scale;
    std::array::from_fn(|i| {
        let x = i as f64 * scale;
        let shoulder = x * (1.0 + x / (white * white)) / (1.0 + x);
        (16384.0 * cube::srgb_encode(shoulder)).round() as u16
    })
}

fn model() -> Model {
    Model::new(&synthetic_base())
}

/// Smooth curves that reach white at scene-linear 1, where sRGB and linear LUTs clip.
fn known(post: [f64; 9]) -> Params {
    let white = model().base_gamma(1.0) / 64.0;
    let curve = |a: f64, toe: f64| -> [f64; 256] {
        std::array::from_fn(|i| {
            let s = (1.0 - (-a * i as f64 / white).exp()) / (1.0 - (-a).exp());
            (toe + (16383.0 - toe) * s).min(16383.0).round()
        })
    };
    Params {
        matrix: [0.9, 0.12, -0.02, 0.03, 0.92, 0.05, -0.01, 0.06, 0.95],
        curves: [curve(3.0, 160.0), curve(2.8, 96.0), curve(3.3, 240.0)],
        post,
    }
}

const POST: [f64; 9] = [1.08, -0.05, -0.03, -0.04, 1.06, -0.02, -0.02, -0.06, 1.08];

fn synthetic_cube(input: Encoding, size: usize, params: &Params) -> Cube {
    let model = model();
    parse(&cube_text(size, "", |x| {
        model.render(params, model.decode(input, x))
    }))
    .unwrap()
}

fn fit_cube(input: Encoding, size: usize, params: &Params) -> fit::Fit {
    let (model, cube) = (model(), synthetic_cube(input, size, params));
    let target = fit::cube_target(&model, &cube, input, 0.0);
    fit::fit(&synthetic_base(), &target, &Options::default()).unwrap()
}

/// The chart error of the known parameters against their own LUT: the interpolation floor.
fn floor(input: Encoding, size: usize, params: &Params) -> (f64, f64) {
    let (model, lab) = (model(), cube::LabConverter::default());
    let cube = synthetic_cube(input, size, params);
    let target = fit::cube_target(&model, &cube, input, 0.0);
    let errors = fit::colorchecker()
        .map(|p| cube::delta_e2000(lab.lab(model.render(params, p)), lab.lab(target(p))));
    (
        errors.iter().sum::<f64>() / 24.0,
        errors.iter().copied().fold(0.0, f64::max),
    )
}

fn round_trip(input: Encoding, size: usize, params: &Params) -> fit::Fit {
    let started = std::time::Instant::now();
    let result = fit_cube(input, size, params);
    let elapsed = started.elapsed();
    let (floor_mean, floor_max) = floor(input, size, params);
    let fitted = result.params();
    let distance = |a: &[f64; 9], b: &[f64; 9]| {
        a.iter()
            .zip(b)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max)
    };
    let (matrix, post) = (
        distance(&fitted.matrix, &params.matrix),
        distance(&fitted.post, &params.post),
    );
    // Compare the curves from 5% to white. Below 5%, a cell of a 65-point linear LUT spans
    // dozens of curve entries, and the fit follows the LUT's chord. Above white, sRGB and
    // linear LUTs clip.
    let index = |linear: f64| (model().base_gamma(linear) / 64.0) as usize;
    let (black, white) = (index(0.05), index(1.0));
    let curve = (0..3)
        .flat_map(|c| (black..white).map(move |i| (c, i)))
        .map(|(c, i)| (fitted.curves[c][i] - params.curves[c][i]).abs())
        .fold(0.0, f64::max);
    eprintln!(
        "{} {size}: mean {:.3} max {:.3} (floor {floor_mean:.3} / {floor_max:.3}) standard {:.3} csp {} | matrix {matrix:.4} post {post:.4} curve {curve:.0} | {elapsed:.1?}",
        input.name(),
        result.mean_de,
        result.max_de,
        result.standard_mean_de,
        result.csp_max,
    );
    assert!(
        result.mean_de < 0.5 && result.max_de < floor_max + 1.0,
        "{result:?}"
    );
    assert!(matrix < 0.02 && post < 0.02 && curve < 100.0);
    assert!(result.standard_mean_de > result.mean_de);
    assert!(result.csp_max <= fit::CSP_LIMIT);
    result
}

#[test]
fn synthetic_base_gamma_matches_the_firmware_shape() {
    let base = synthetic_base();
    assert_eq!((base[0], base[255]), (0, 16384));
    assert!(base.windows(2).all(|p| p[0] <= p[1]));
    let gray = model().base_gamma(0.18) / 16384.0;
    assert!((gray - 0.46).abs() < 0.01, "{gray}");
}

#[test]
fn srgb_round_trip_recovers_the_model() {
    let result = round_trip(Encoding::Srgb, 33, &known(POST));
    assert!(result.post_matrix.is_some());
}

/// The fit errors without the curves, for test logs.
fn brief(fit: &fit::Fit) -> String {
    format!(
        "mean {:.3} max {:.3} standard {:.3} csp {} mono {} matrix {:?} post {:?}",
        fit.mean_de,
        fit.max_de,
        fit.standard_mean_de,
        fit.csp_max,
        fit.monochrome,
        fit.matrix,
        fit.post_matrix
    )
}

#[test]
fn standard_lut_fits_its_target() {
    // Standard clips saturated colors, so the LUT cannot hold the known parameters there.
    // The fit is checked against the target, not against the parameters.
    let result = fit_cube(Encoding::Standard, 33, &known(POST));
    let (_, floor_max) = floor(Encoding::Standard, 33, &known(POST));
    eprintln!("standard 33: {} floor max {floor_max:.3}", brief(&result));
    assert!(result.mean_de < 1.0 && result.max_de < floor_max + 3.0);
    assert!(result.csp_max <= fit::CSP_LIMIT);
}

#[test]
fn flog_round_trips_recover_the_model() {
    round_trip(Encoding::Flog, 65, &known(POST));
    round_trip(Encoding::Flog2, 65, &known(POST));
    round_trip(Encoding::Flog2C, 65, &known(POST));
}

#[test]
fn standard_encoding_inverts_and_makes_an_identity_lut_neutral() {
    let model = model();
    for linear in [[0.18; 3], [0.05, 0.2, 0.6], [0.5, 0.3, 0.1]] {
        let back = model.decode(Encoding::Standard, model.encode(Encoding::Standard, linear));
        assert!(
            back.iter().zip(linear).all(|(a, b)| (a - b).abs() < 1e-3),
            "{back:?}"
        );
    }
    // An identity LUT on a Standard image is Standard itself.
    let cube = parse(&cube_text(33, "", |x| x)).unwrap();
    let target = fit::cube_target(&model, &cube, Encoding::Standard, 0.0);
    let result = fit::fit(&synthetic_base(), &target, &Options::default()).unwrap();
    eprintln!("standard identity: {}", brief(&result));
    assert!(result.mean_de < 0.5 && result.standard_mean_de < 0.5);
}

#[test]
fn linear_round_trip_recovers_the_model() {
    round_trip(Encoding::Linear, 65, &known(POST));
}

#[test]
fn vlog_round_trip_recovers_the_model() {
    round_trip(Encoding::Vlog, 65, &known(POST));
}

#[test]
fn post_matrix_stays_within_the_csp_limit() {
    assert_eq!(fit::csp_max(&[8192, 0, 0, 0, 8192, 0, 0, 0, 8192]), 150);
    // Cb x P reaches 301 / 256 in the blue column, beyond the limit of 240.
    let saturated = [1.9, -0.45, -0.45, -0.45, 1.9, -0.45, -0.45, -0.45, 1.9];
    let result = fit_cube(Encoding::Vlog, 17, &known(saturated));
    let post = result.post_matrix.unwrap();
    eprintln!(
        "saturated: csp {} mean {:.3} post {post:?}",
        result.csp_max, result.mean_de
    );
    assert!(result.csp_max <= fit::CSP_LIMIT && result.csp_max >= 200);
    assert_eq!(result.csp_max, fit::csp_max(&post));
    assert!(post
        .chunks(3)
        .all(|row| row.iter().map(|v| *v as i32).sum::<i32>() == 8192));
}

#[test]
fn identity_lut_fits_a_neutral_effect() {
    let cube = parse(&cube_text(17, "", |x| x)).unwrap();
    let options = Options {
        post: false,
        ..Options::default()
    };
    let model = model();
    let target = fit::cube_target(&model, &cube, Encoding::Srgb, 0.0);
    let result = fit::fit(&synthetic_base(), &target, &options).unwrap();
    eprintln!(
        "identity: mean {:.3} max {:.3} matrix {:?}",
        result.mean_de, result.max_de, result.matrix
    );
    assert!(result.mean_de < 1.0, "{result:?}");
    assert!(result.post_matrix.is_none());
    let params = result.params();
    assert!(params
        .matrix
        .iter()
        .zip(IDENTITY)
        .all(|(a, b)| (a - b).abs() < 0.03));
}

#[test]
fn import_writes_a_compilable_definition() {
    let text = cube_text(9, "", |x| x.map(|v| v * 0.9 + 0.05));
    let mut request = Import {
        effect_id: "faded-v1",
        name: "Faded",
        label: "FG",
        shape: "filter",
        source: "faded.cube",
        input: Encoding::Srgb,
        exposure: -0.5,
        options: Options {
            random_samples: 300,
            rounds: 2,
            joint_rounds: 1,
            post: true,
        },
    };
    let imported = fit::import(text.as_bytes(), &synthetic_base(), &request).unwrap();
    let (result, json) = (&imported.fit, &imported.text);
    assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
    // The file name does not decide the input: a log name gives no warning for a look over a
    // JPEG.
    let named = Import {
        source: "FLog2C_to_Faded_VLog.cube",
        ..request.clone()
    };
    let warnings = fit::import(text.as_bytes(), &synthetic_base(), &named)
        .unwrap()
        .warnings;
    assert!(
        !warnings.iter().any(|w| w.contains("suggests")),
        "{warnings:?}"
    );
    // A LUT made for V-Log warns when it is read as Standard, and not as V-Log.
    let model = model();
    let vlog = cube_text(17, "", |x| model.standard(model.decode(Encoding::Vlog, x)));
    let read_as = |input| Import {
        input,
        ..request.clone()
    };
    let loud = fit::import(
        vlog.as_bytes(),
        &synthetic_base(),
        &read_as(Encoding::Standard),
    )
    .unwrap()
    .warnings;
    assert!(
        loud.iter().any(|w| w.contains("data suggests vlog input")),
        "{loud:?}"
    );
    let quiet = fit::import(vlog.as_bytes(), &synthetic_base(), &read_as(Encoding::Vlog))
        .unwrap()
        .warnings;
    assert!(!quiet.iter().any(|w| w.contains("suggests")), "{quiet:?}");
    assert!(!result.monochrome);
    let definition = effects::parse_json(json.as_bytes()).unwrap();
    effects::compile_definition(&definition).unwrap();
    let keys: Vec<&str> = json
        .lines()
        .filter_map(|l| l.strip_prefix("  \"")?.split('"').next())
        .collect();
    let mut expected = vec![
        "schema",
        "effect_id",
        "name",
        "flags",
        "matrix",
        "icon",
        "attribution",
        "curves",
    ];
    if result.post_matrix.is_some() {
        expected.push("post_matrix");
    }
    assert_eq!(keys, expected);
    let attribution = &definition["attribution"];
    assert_eq!(attribution["origin"], "cube-fit");
    assert_eq!(attribution["source"], "faded.cube");
    assert_eq!(attribution["input"], "srgb");
    assert_eq!(attribution["exposure"], -0.5);
    assert_eq!(
        attribution["source_sha256"],
        grkit_core::packaging::sha256(text.as_bytes())
    );
    assert!(attribution["fit"]["mean_de"].as_f64().unwrap() < 2.0);
    assert_eq!(attribution["fit"]["csp_max"], result.csp_max);
    assert_eq!(attribution["fit"]["grade"], fit::Grade::of(result).letter());
    assert!(attribution["fit"]["holdout_mean_de"].as_f64().unwrap() > 0.0);
    assert!(definition.get("glow").is_none());

    request.label = "ab";
    let failure = format!(
        "{:#}",
        fit::import(text.as_bytes(), &synthetic_base(), &request).unwrap_err()
    );
    assert!(failure.contains("label"), "{failure}");
    request.label = "FG";
    request.exposure = f64::NAN;
    assert!(fit::import(text.as_bytes(), &synthetic_base(), &request).is_err());
}

fn quick() -> Options {
    Options {
        random_samples: 1500,
        rounds: 4,
        joint_rounds: 2,
        post: true,
    }
}

fn import_request<'a>(input: Encoding) -> Import<'a> {
    Import {
        effect_id: "imported-v1",
        name: "Imported",
        label: "IM",
        shape: "film",
        source: "imported.cube",
        input,
        exposure: 0.0,
        options: quick(),
    }
}

#[test]
fn gray_luts_import_as_monochrome_effects() {
    // A red-filter black and white look with a lifted black.
    let text = cube_text(17, "", |[r, g, b]| {
        [0.08 + 0.9 * (0.6 * r + 0.35 * g + 0.05 * b); 3]
    });
    let imported = fit::import(
        text.as_bytes(),
        &synthetic_base(),
        &import_request(Encoding::Standard),
    )
    .unwrap();
    let fit = &imported.fit;
    eprintln!("mono: {}", brief(fit));
    assert!(fit.monochrome && fit.post_matrix.is_none());
    // The LUT mixes Standard-encoded values, which a linear matrix approximates.
    assert!(fit.mean_de < 3.0);
    let definition = effects::parse_json(imported.text.as_bytes()).unwrap();
    assert_eq!(definition["flags"], effects::MONOCHROME);
    effects::compile_definition(&definition).unwrap();
    // The red weight leads, as the LUT does.
    assert!(fit.matrix[0] > fit.matrix[1] && fit.matrix[1] > fit.matrix[2]);
}

#[test]
fn unrepresentable_luts_fail_clearly_or_fit_loosely() {
    let base = synthetic_base();
    let negative = cube_text(9, "", |x| x.map(|v| 1.0 - v));
    let message = format!(
        "{:#}",
        fit::import(negative.as_bytes(), &base, &import_request(Encoding::Srgb)).unwrap_err()
    );
    assert!(message.contains("inverts brightness"), "{message}");
    // Infrared false color: the channels trade places, beyond what one matrix plus curves fit.
    let infrared = cube_text(17, "", |[r, g, b]| {
        let swap = [b, r, g];
        let gray = (r + g + b) / 3.0;
        swap.map(|v| (4.0 * (v - gray) + gray).clamp(0.0, 1.0))
    });
    let imported =
        fit::import(infrared.as_bytes(), &base, &import_request(Encoding::Srgb)).unwrap();
    eprintln!("infrared: {} {:?}", brief(&imported.fit), imported.warnings);
    let limit = (fit::MATRIX_LIMIT * 8192.0) as i16 + 2;
    assert!(imported.fit.matrix.iter().all(|v| v.abs() <= limit));
    assert_eq!(fit::Grade::of(&imported.fit), fit::Grade::D);
    assert!(
        imported.warnings.iter().any(|w| w.starts_with("Grade D")),
        "{:?}",
        imported.warnings
    );
}

#[test]
fn toned_luts_share_one_matrix_row_and_no_post_matrix() {
    // Sepia over a red-filter black and white look.
    let text = cube_text(17, "", |[r, g, b]| {
        let y = 0.05 + 0.9 * (0.6 * r + 0.35 * g + 0.05 * b);
        [y.powf(0.9), y, y.powf(1.15)]
    });
    let imported = fit::import(
        text.as_bytes(),
        &synthetic_base(),
        &import_request(Encoding::Standard),
    )
    .unwrap();
    let fit = &imported.fit;
    eprintln!("toned: {}", brief(fit));
    assert!(fit.toned && !fit.monochrome && fit.post_matrix.is_none());
    assert!(fit.matrix[..3] == fit.matrix[3..6] && fit.matrix[..3] == fit.matrix[6..]);
    assert!(fit.curves[0] != fit.curves[2]);
    assert!(fit.mean_de < 3.0);
    let definition = effects::parse_json(imported.text.as_bytes()).unwrap();
    assert_eq!(definition["flags"], 0);
    effects::compile_definition(&definition).unwrap();
}
