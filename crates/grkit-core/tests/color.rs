//! Input encodings, gamut matrices, and the input guess from LUT data.

mod common;

use common::*;
use grkit_core::cube::{self, Cube, Encoding};
use grkit_core::fit;

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
