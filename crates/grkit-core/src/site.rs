//! Effect files for the website: compiled effects, preview tables, and their catalog.
//!
//! Published preview tables render scene-linear sRGB from decoded RAW photos. The sRGB
//! transfer function spaces the table coordinates more densely in the shadows. The
//! Standard baseline and each effect render the same linear input independently.

use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;

use crate::cube::{self, Encoding};
use crate::fit::{self, Model, Params};
use crate::selective::{self, Selective};
use crate::{effects, hue, packaging, Files};

/// Points on each axis of a preview table.
pub const LUT_SIZE: usize = 33;
pub const CATALOG_SCHEMA: &str = "grkit-site-effects-v1";
const STANDARD_PREVIEW: &str = "preview/standard.lut";
/// A bound on the catalog. A package still holds at most `runtime::MAX_COUNT` effects.
pub const MAX_EFFECTS: usize = 100;

fn unit(m: &[i16; 9]) -> [f64; 9] {
    m.map(|v| v as f64 / 8192.0)
}

fn coefficients<const N: usize>(value: &Value, what: &str) -> Result<[i64; N]> {
    let values = value
        .as_array()
        .filter(|v| v.len() == N)
        .with_context(|| format!("Expected {N} {what} values"))?;
    let mut result = [0; N];
    for (slot, value) in result.iter_mut().zip(values) {
        *slot = value
            .as_i64()
            .with_context(|| format!("Expected integer {what} values"))?;
    }
    Ok(result)
}

/// The model parameters of a decoded effect.
pub fn params(effect: &Value) -> Result<Params> {
    let q13 = |value: &Value| -> Result<[f64; 9]> {
        Ok(unit(&coefficients::<9>(value, "matrix")?.map(|v| v as i16)))
    };
    let curves = effect["curves"]
        .as_array()
        .filter(|c| c.len() == 3)
        .context("Expected three curves")?;
    let mut result = Params {
        matrix: q13(&effect["matrix"])?,
        curves: [[0.0; 256]; 3],
        post: fit::IDENTITY,
        sectors: effect
            .get("hue_sectors")
            .map(|value| hue::HueSectors::from_json(value).map(|s| s.stage()))
            .transpose()?,
    };
    for (slot, curve) in result.curves.iter_mut().zip(curves) {
        *slot = coefficients::<256>(curve, "curve")?.map(|v| v as f64);
    }
    if let Some(post) = effect.get("post_matrix") {
        result.post = q13(post)?;
    }
    Ok(result)
}

/// A legacy Standard-JPEG input table, red fastest, then green, then blue.
/// Published catalogs use `linear_preview_lut` instead.
pub fn preview_lut(base_gamma: &[u16; 256], effect: &Value) -> Result<Vec<u8>> {
    let params = params(effect)?;
    let selective = effect
        .get("selective_color")
        .map(|value| Selective::from_json(value).map(|s| s.table()))
        .transpose()?;
    let model = Model::new(base_gamma);
    let step = |i: usize| i as f64 / (LUT_SIZE - 1) as f64;
    let mut table = Vec::with_capacity(LUT_SIZE.pow(3) * 3);
    for b in 0..LUT_SIZE {
        for g in 0..LUT_SIZE {
            for r in 0..LUT_SIZE {
                // Scene-linear light that Standard renders as this display color.
                let linear = model.decode(Encoding::Standard, [step(r), step(g), step(b)]);
                let mut display = model.render(&params, linear);
                if let Some(table) = &selective {
                    display = selective::apply_rgb(table, display);
                }
                table.extend(display.map(|v| (v * 255.0).round() as u8));
            }
        }
    }
    Ok(table)
}

/// Samples linear sRGB through an sRGB coordinate shaper, with 8-bit display RGB output.
fn linear_lut(render: impl Fn([f64; 3]) -> [f64; 3]) -> Vec<u8> {
    let step = |i: usize| cube::srgb_decode(i as f64 / (LUT_SIZE - 1) as f64);
    let mut table = Vec::with_capacity(LUT_SIZE.pow(3) * 3);
    for b in 0..LUT_SIZE {
        for g in 0..LUT_SIZE {
            for r in 0..LUT_SIZE {
                let display = render([step(r), step(g), step(b)]);
                table.extend(display.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8));
            }
        }
    }
    table
}

/// Renders RAW linear sRGB, using sRGB-encoded lookup coordinates in red-fastest order.
/// The output contains `LUT_SIZE` cubed 8-bit RGB samples.
pub fn linear_preview_lut(base_gamma: &[u16; 256], effect: &Value) -> Result<Vec<u8>> {
    let params = params(effect)?;
    let selective = effect
        .get("selective_color")
        .map(|value| Selective::from_json(value).map(|s| s.table()))
        .transpose()?;
    let model = Model::new(base_gamma);
    Ok(linear_lut(|linear| {
        let display = model.render(&params, linear);
        match &selective {
            Some(table) => selective::apply_rgb(table, display),
            None => display,
        }
    }))
}

/// The Standard baseline for RAW previews, with the same coordinates as effect tables.
/// Standard applies its look matrix and base gamma directly, without effect curves.
pub fn standard_linear_preview_lut(base_gamma: &[u16; 256]) -> Vec<u8> {
    let model = Model::new(base_gamma);
    linear_lut(|linear| model.standard(linear))
}

/// Standard base64 with padding. The catalog carries each 40 x 40 RGBA icon
/// in this form, so the site shows the pixels that the camera shows.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let word = chunk.iter().enumerate().fold(0u32, |word, (i, byte)| {
            word | u32::from(*byte) << (16 - 8 * i)
        });
        for i in 0..4 {
            out.push(if i <= chunk.len() {
                char::from(ALPHABET[(word >> (18 - 6 * i) & 63) as usize])
            } else {
                '='
            });
        }
    }
    out
}

fn text<'a>(value: &'a Value, what: &str) -> Result<&'a str> {
    value
        .as_str()
        .with_context(|| format!("Expected effect {what}"))
}

/// Checks a Standard base gamma table from a site catalog: 256 values from 0 that do
/// not decrease and end near 16384, as `grkit_build::firmware::standard_base_gamma` reads them.
pub fn base_gamma(values: &[u16]) -> Result<[u16; 256]> {
    let table: [u16; 256] = values
        .try_into()
        .context("Expected 256 base gamma values")?;
    ensure!(
        table[0] == 0
            && table.windows(2).all(|pair| pair[0] <= pair[1])
            && (16000..=16384).contains(&table[255]),
        "Unexpected Standard base gamma"
    );
    Ok(table)
}

/// Compiles effect definitions for the site catalog of `target`, whose firmware holds
/// `base_gamma`. The result holds `catalog.json`, one `.GRF` per effect under its
/// package filename, `preview/<id>.lut`, and the Standard baseline preview table.
/// The catalog also gives `base_gamma`, so the site can import LUTs without the firmware.
pub fn publish_effects(
    definitions: &[Value],
    base_gamma: &[u16; 256],
    target: &str,
) -> Result<Files> {
    ensure!(
        (1..=MAX_EFFECTS).contains(&definitions.len()),
        "Expected 1 to {MAX_EFFECTS} effects"
    );
    let mut files = Files::new();
    files.insert(
        STANDARD_PREVIEW.into(),
        standard_linear_preview_lut(base_gamma),
    );
    let mut ids = BTreeSet::new();
    let mut entries = Vec::new();
    for definition in definitions {
        let blob = effects::compile_definition(definition)?;
        let (effect, icon) = effects::decode(&blob)?;
        let id = text(&effect["effect_id"], "ID")?;
        let file = effects::filename(id)?;
        ensure!(
            ids.insert(id.to_owned()) && !files.contains_key(&file),
            "Duplicate effect ID or filename hash collision: {id}"
        );
        let attribution = &definition["attribution"];
        let preview = format!("preview/{id}.lut");
        ensure!(
            !files.contains_key(&preview),
            "Effect preview conflicts with the Standard baseline: {id}"
        );
        let mut entry = json!({
            "id": id,
            "name": effect["name"],
            "icon": base64(&icon),
            "monochrome": effect["flags"].as_u64() == Some(effects::MONOCHROME.into()),
            "look": text(&attribution["target"], "target look")?,
            "origin": text(&attribution["origin"], "origin")?,
            "license": text(&attribution["license"], "license")?,
            "file": file,
            "sha256": packaging::sha256(&blob),
            "preview": preview,
        });
        // The site draws a film cover icon from its fields with web fonts, so it
        // stays sharp at any size. The pixels remain the camera's icon.
        if definition["icon"].get("template").is_some() {
            entry["cover"] = definition["icon"].clone();
        }
        entries.push(entry);
        files.insert(preview, linear_preview_lut(base_gamma, &effect)?);
        files.insert(file, blob);
    }
    let catalog = json!({
        "schema": CATALOG_SCHEMA,
        "target": target,
        // The site fits imported LUTs over this table, as `grkit effects import` does with
        // the table that it reads from the firmware. See `base_gamma`.
        "base_gamma": base_gamma.as_slice(),
        "preview": {
            "size": LUT_SIZE,
            "order": "rgb",
            "input": "linear-srgb16",
            "shaper": "srgb",
            "baseline": STANDARD_PREVIEW,
        },
        "effects": entries,
    });
    files.insert("catalog.json".into(), packaging::json_bytes(&catalog)?);
    Ok(files)
}
