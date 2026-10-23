use grkit_core::effects::{self, Effect};
use grkit_core::{cube, fit, packaging, site};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../effects")
}

fn definition(id: &str) -> Value {
    effects::parse_json(&fs::read(root().join(id).join("effect.json")).unwrap()).unwrap()
}

/// The analytic stand-in for the firmware table from the import tests.
fn synthetic_base() -> [u16; 256] {
    let scale = 1.2 * 0.18 / fit::MID_INDEX;
    let white = 255.0 * scale;
    std::array::from_fn(|i| {
        let x = i as f64 * scale;
        let shoulder = x * (1.0 + x / (white * white)) / (1.0 + x);
        (16384.0 * cube::srgb_encode(shoulder)).round() as u16
    })
}

/// Decodes padded standard base64, as the site does with `atob`.
fn from_base64(text: &str) -> Vec<u8> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => panic!("unexpected base64 byte {c}"),
    };
    let mut bytes = Vec::new();
    for quad in text.as_bytes().chunks(4) {
        let data: Vec<u8> = quad
            .iter()
            .take_while(|c| **c != b'=')
            .map(|c| value(*c))
            .collect();
        let word = data
            .iter()
            .enumerate()
            .fold(0u32, |word, (i, v)| word | u32::from(*v) << (18 - 6 * i));
        bytes.extend((0..data.len() - 1).map(|i| (word >> (16 - 8 * i)) as u8));
    }
    bytes
}

/// A selective color definition that keeps only red. No published effect uses the feature.
fn keep_red() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/keep-red.json");
    effects::parse_json(&fs::read(path).unwrap()).unwrap()
}

fn decoded(definition: &Value) -> Effect {
    effects::decode(&effects::compile_definition(definition).unwrap())
        .unwrap()
        .0
}

fn entry(table: &[u8], r: usize, g: usize, b: usize) -> [u8; 3] {
    let index = ((b * site::LUT_SIZE + g) * site::LUT_SIZE + r) * 3;
    table[index..index + 3].try_into().unwrap()
}

#[test]
fn standard_preview_returns_each_input() {
    let curve: Vec<u16> = (0..256).map(|v| v * 64).collect();
    let standard = Effect::from_json(&json!({"effect_id": "standard-v1", "name": "Standard",
        "flags": 0, "matrix": fit::STANDARD_LOOK, "curves": [&curve, &curve, &curve]}))
    .unwrap();
    let table = site::preview_lut(&synthetic_base(), &standard).unwrap();
    assert_eq!(table.len(), site::LUT_SIZE.pow(3) * 3);
    let level = |i: usize| (i as f64 * 255.0 / (site::LUT_SIZE - 1) as f64).round() as i32;
    let step = |i: usize| i as f64 / (site::LUT_SIZE - 1) as f64;
    // The highest code of the curves above.
    let highest = curve[255] as f64 / 16384.0;
    let mut checked = 0;
    for b in 0..site::LUT_SIZE {
        for g in 0..site::LUT_SIZE {
            for r in 0..site::LUT_SIZE {
                let actual = entry(&table, r, g, b);
                // A Standard JPEG has the camera's chroma suppression. Near black and white,
                // the camera cannot write a saturated color: its codes before the suppression
                // leave the range of the curves. The table returns each color that the
                // camera can write.
                let codes = fit::undisplay([step(r), step(g), step(b)]);
                if codes.iter().any(|v| !(-1e-9..=highest).contains(v)) {
                    continue;
                }
                checked += 1;
                for (value, expected) in actual.iter().zip([level(r), level(g), level(b)]) {
                    assert!(
                        (*value as i32 - expected).abs() <= 1,
                        "{r} {g} {b}: {actual:?}"
                    );
                }
            }
        }
    }
    assert!(checked * 10 > site::LUT_SIZE.pow(3) * 8, "{checked}");
}

#[test]
fn effect_previews_follow_the_effect() {
    let base = synthetic_base();
    let mono = site::preview_lut(&base, &decoded(&definition("red-filter-mono-v1"))).unwrap();
    assert!(mono
        .chunks_exact(3)
        .all(|rgb| rgb[0] == rgb[1] && rgb[1] == rgb[2]));
    let ektar = site::preview_lut(&base, &decoded(&definition("ektar-100-edge-v1"))).unwrap();
    let gray = |table: &[u8], i| entry(table, i, i, i);
    // Brighter input stays brighter, and the film look differs from Standard.
    for i in 1..site::LUT_SIZE {
        assert!(gray(&ektar, i)[1] >= gray(&ektar, i - 1)[1]);
    }
    let level = |i: usize| (i * 255 / (site::LUT_SIZE - 1)) as i32;
    let red = entry(&ektar, 28, 6, 4);
    let shift = (red[0] as i32 - level(28)).abs() + (red[1] as i32 - level(6)).abs();
    assert!(shift > 8, "{red:?}");
}

#[test]
fn raw_previews_render_linear_input_without_inverting_standard() {
    let base = synthetic_base();
    let effect = decoded(&definition("ektar-100-edge-v1"));
    let model = fit::Model::new(&base);
    let params = site::params(&effect);
    let table = site::linear_preview_lut(&base, &effect).unwrap();
    let baseline = site::standard_linear_preview_lut(&base);
    let standard = std::array::from_fn(|r| {
        std::array::from_fn(|c| fit::STANDARD_LOOK[r * 3 + c] as f64 / 8192.0)
    });
    assert_eq!(table.len(), site::LUT_SIZE.pow(3) * 3);
    assert_eq!(baseline.len(), table.len());
    for [r, g, b] in [[0, 0, 0], [1, 1, 1], [16, 16, 16], [28, 6, 4], [32, 32, 32]] {
        let linear = [r, g, b].map(|v| cube::srgb_decode(v as f64 / 32.0));
        let expected = model
            .render(&params, linear)
            .map(|v| (v * 255.0).round() as u8);
        assert_eq!(entry(&table, r, g, b), expected);
        let expected_baseline = cube::mul(&standard, linear)
            .map(|v| (model.base_gamma(v) / 16384.0 * 255.0).round() as u8);
        assert_eq!(entry(&baseline, r, g, b), expected_baseline);
    }
    let legacy = site::preview_lut(&base, &effect).unwrap();
    assert_ne!(entry(&table, 16, 16, 16), entry(&legacy, 16, 16, 16));
}

#[test]
fn catalog_base_gamma_is_checked() {
    let base = synthetic_base();
    assert_eq!(site::base_gamma(&base).unwrap(), base);
    assert!(site::base_gamma(&base[..255]).is_err());
    let mut decreasing = base;
    decreasing[100] = decreasing[99] - 1;
    assert!(site::base_gamma(&decreasing).is_err());
    let mut offset = base;
    offset[0] = 1;
    assert!(site::base_gamma(&offset).is_err());
    assert!(site::base_gamma(&[0; 256]).is_err());
}

#[test]
fn raw_standard_baseline_does_not_clip_through_effect_curves() {
    // A saturated base gamma exposes the 16320 ceiling of identity effect curves.
    let base = [16384; 256];
    let baseline = site::standard_linear_preview_lut(&base);
    assert!(baseline.iter().all(|value| *value == 255));
}

#[test]
fn published_effects_match_their_definitions() {
    let ids = ["ektar-100-edge-v1", "red-filter-mono-v1"];
    let definitions: Vec<Value> = ids.iter().map(|id| definition(id)).collect();
    let files = site::publish_effects(&definitions, &synthetic_base(), "gr3-2.10").unwrap();
    let catalog = effects::parse_json(&files["catalog.json"]).unwrap();
    assert_eq!(catalog["schema"], "grkit-site-effects-v1");
    assert_eq!(catalog["target"], "gr3-2.10");
    let published: Vec<u16> = catalog["base_gamma"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u16)
        .collect();
    assert_eq!(site::base_gamma(&published).unwrap(), synthetic_base());
    assert_eq!(
        catalog["preview"],
        json!({
            "size": site::LUT_SIZE,
            "order": "rgb",
            "input": "linear-srgb16",
            "shaper": "srgb",
            "baseline": "preview/standard.lut",
        })
    );
    assert_eq!(
        files["preview/standard.lut"],
        site::standard_linear_preview_lut(&synthetic_base())
    );
    let entries = catalog["effects"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(files.len(), 2 + 2 * entries.len());
    for (entry, definition) in entries.iter().zip(&definitions) {
        let id = entry["id"].as_str().unwrap();
        let blob = &files[entry["file"].as_str().unwrap()];
        assert_eq!(entry["file"], effects::filename(id).unwrap());
        assert_eq!(blob, &effects::compile_definition(definition).unwrap());
        assert_eq!(entry["sha256"], packaging::sha256(blob));
        assert_eq!(entry["name"], definition["name"]);
        assert_eq!(entry["cover"], definition["icon"]);
        assert_eq!(
            from_base64(entry["icon"].as_str().unwrap()),
            effects::decode(blob).unwrap().1
        );
        assert_eq!(entry["look"], definition["attribution"]["target"]);
        assert_eq!(
            files[entry["preview"].as_str().unwrap()],
            site::linear_preview_lut(&synthetic_base(), &decoded(definition)).unwrap()
        );
    }
    assert_eq!(entries[0]["monochrome"], false);
    assert_eq!(entries[1]["monochrome"], true);
    assert!(entries.iter().all(|entry| entry.get("label").is_none()));

    let duplicate = [definitions[0].clone(), definitions[0].clone()];
    let error = site::publish_effects(&duplicate, &synthetic_base(), "gr3-2.10").unwrap_err();
    assert!(error.to_string().contains("Duplicate effect ID"), "{error}");
    assert!(site::publish_effects(&[], &synthetic_base(), "gr3-2.10").is_err());

    let mut reserved = definitions[0].clone();
    reserved["effect_id"] = json!("standard");
    let error = site::publish_effects(&[reserved], &synthetic_base(), "gr3-2.10").unwrap_err();
    assert!(error.to_string().contains("Standard baseline"), "{error}");
}

#[test]
fn selective_color_previews_keep_red_and_gray_other_hues() {
    let base = synthetic_base();
    let accent = site::preview_lut(&base, &decoded(&keep_red())).unwrap();
    let mut standard = decoded(&keep_red());
    standard.selective = None;
    let plain = site::preview_lut(&base, &standard).unwrap();
    let last = site::LUT_SIZE - 1;
    // Saturated red keeps its color; green and blue turn neutral.
    let red = entry(&accent, last, 0, 0);
    assert!(red[0] as i32 - red[1] as i32 > 100, "{red:?}");
    for rgb in [entry(&accent, 0, last, 0), entry(&accent, 0, 0, last)] {
        assert!(
            rgb.iter().max().unwrap() - rgb.iter().min().unwrap() <= 2,
            "{rgb:?}"
        );
    }
    // The gray ramp has no chroma to keep or remove. The 8-bit YCbCr round trip,
    // as in a camera JPEG, can move a channel by one level.
    for i in 0..site::LUT_SIZE {
        let (a, p) = (entry(&accent, i, i, i), entry(&plain, i, i, i));
        assert!(
            a.iter().zip(p).all(|(a, p)| a.abs_diff(p) <= 1),
            "{a:?} {p:?}"
        );
    }
}

#[test]
fn raw_previews_apply_selective_color_after_rendering() {
    let base = synthetic_base();
    let effect = decoded(&keep_red());
    let params = site::params(&effect);
    let selective = effect.selective.unwrap().table();
    let model = fit::Model::new(&base);
    let table = site::linear_preview_lut(&base, &effect).unwrap();
    for [r, g, b] in [[32, 0, 0], [0, 32, 0], [0, 0, 32], [16, 16, 16]] {
        let linear = [r, g, b].map(|v| cube::srgb_decode(v as f64 / 32.0));
        let expected = grkit_core::selective::apply_rgb(&selective, model.render(&params, linear))
            .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
        assert_eq!(entry(&table, r, g, b), expected);
    }
}

#[test]
fn the_preview_target_is_a_camera_runtime() {
    assert!(grkit_core::camera::CAMERAS
        .iter()
        .any(|camera| camera.runs(site::PREVIEW_TARGET)));
}
