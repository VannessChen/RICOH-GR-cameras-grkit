//! The camera's chroma suppression (CSY) in the fit model.
use grkit_core::fit;

#[test]
fn gain_follows_the_four_lines_of_the_camera() {
    // 512 = 1 at the 10-bit luminance borders 56 and 768, 0 at black.
    assert_eq!(fit::chroma_gain(0.0), 0.0);
    assert!((fit::chroma_gain(28.0 / 1023.0) - 73.0 * 28.0 / 8.0 / 512.0).abs() < 1e-9);
    assert_eq!(fit::chroma_gain(56.0 / 1023.0), 1.0);
    assert_eq!(fit::chroma_gain(0.5), 1.0);
    assert!((fit::chroma_gain(856.0 / 1023.0) - 432.0 / 512.0).abs() < 1e-9);
    assert!((fit::chroma_gain(1.0) - (432.0 - 14.0 * 167.0 / 8.0) / 512.0).abs() < 1e-9);
}

#[test]
fn display_keeps_gray_and_mid_tones_and_reduces_dark_chroma() {
    for v in [0.0, 0.02, 0.5, 0.95] {
        let out = fit::display([v; 3]);
        assert!(out.iter().all(|c| (c - v).abs() < 1e-9), "{v}: {out:?}");
    }
    let mid = [0.6, 0.4, 0.3];
    assert_eq!(fit::display(mid), mid);
    // A dark blue keeps its luminance and loses chroma.
    let dark = [0.02, 0.03, 0.08];
    let out = fit::display(dark);
    let y = |c: [f64; 3]| (77.0 * c[0] + 150.0 * c[1] + 29.0 * c[2]) / 256.0;
    assert!((y(out) - y(dark)).abs() < 1e-9);
    assert!(out[2] - out[0] < dark[2] - dark[0]);
}

#[test]
fn undisplay_inverts_display_above_the_floor() {
    for codes in [[0.03, 0.035, 0.06], [0.9, 0.95, 0.7], [0.4, 0.5, 0.6]] {
        // Gains about 0.68, 0.60, and 1.
        let back = fit::undisplay(fit::display(codes));
        assert!(
            back.iter().zip(codes).all(|(a, b)| (a - b).abs() < 1e-9),
            "{codes:?}: {back:?}"
        );
    }
}
