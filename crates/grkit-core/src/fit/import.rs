//! `.cube` and LUT image import: the fit target, the input guess, and effect.json.

use anyhow::{ensure, Result};
use serde::Serialize;
use serde_json::Value;

use crate::cube::{delta_e2000, Cube, Encoding, LabConverter, Layout, Rgb};
use crate::{effects, packaging};

use super::*;

pub const MAX_EXPOSURE: f64 = 8.0;

/// The fit target of a LUT: scene-linear RGB, exposed by `exposure` EV, encoded, then looked up.
pub fn cube_target<'a>(
    model: &'a Model,
    cube: &'a Cube,
    input: Encoding,
    exposure: f64,
) -> impl Fn(Rgb) -> Rgb + 'a {
    let gain = 2f64.powf(exposure);
    move |linear| {
        cube.apply(model.encode(input, linear.map(|v| v * gain)))
            .map(|v| v.clamp(0.0, 1.0))
    }
}

/// What `import` writes besides the fitted coefficients.
#[derive(Clone, Debug)]
pub struct Import<'a> {
    pub effect_id: &'a str,
    pub name: &'a str,
    /// The `icon` of effect.json: a label tile or a film cover (`effects::icon_pixels`).
    pub icon: &'a Value,
    /// The `.cube` or HALD PNG file name, recorded in the attribution.
    pub source: &'a str,
    /// The input encoding of the LUT, or `None` for the one that its data suggests
    /// (`guess_input`).
    pub input: Option<Encoding>,
    pub exposure: f64,
    pub options: Options,
}

#[derive(Serialize)]
struct Errors {
    mean_de: f64,
    max_de: f64,
    standard_mean_de: f64,
    csp_max: i32,
    holdout_mean_de: f64,
    holdout_p95_de: f64,
    grade: &'static str,
}

#[derive(Serialize)]
struct Attribution<'a> {
    origin: &'a str,
    source: &'a str,
    source_sha256: String,
    input: &'a str,
    exposure: f64,
    fit: Errors,
}

/// Field order follows the existing effect.json files.
#[derive(Serialize)]
struct Definition<'a> {
    schema: &'a str,
    effect_id: &'a str,
    name: &'a str,
    flags: u32,
    matrix: [i16; 9],
    icon: &'a Value,
    attribution: Attribution<'a>,
    curves: [&'a [u16]; 3],
    #[serde(skip_serializing_if = "Option::is_none")]
    post_matrix: Option<[i16; 9]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hue_sectors: Option<serde_json::Value>,
}

/// Pretty effect.json text that `effects::compile_definition` accepts.
fn definition_text(fit: &Fit, request: &Import, input: Encoding, sha256: String) -> Result<String> {
    let round = |v: f64| (v * 1000.0).round() / 1000.0;
    let definition = Definition {
        schema: effects::SCHEMA,
        effect_id: request.effect_id,
        name: request.name,
        flags: if fit.monochrome {
            effects::MONOCHROME
        } else {
            0
        },
        matrix: fit.matrix,
        icon: request.icon,
        attribution: Attribution {
            origin: "cube-fit",
            source: request.source,
            source_sha256: sha256,
            input: input.name(),
            exposure: request.exposure,
            fit: Errors {
                mean_de: round(fit.mean_de),
                max_de: round(fit.max_de),
                standard_mean_de: round(fit.standard_mean_de),
                csp_max: fit.csp_max,
                holdout_mean_de: round(fit.holdout_mean_de),
                holdout_p95_de: round(fit.holdout_p95_de),
                grade: Grade::of(fit).letter(),
            },
        },
        curves: [&fit.curves[0], &fit.curves[1], &fit.curves[2]],
        post_matrix: fit.post_matrix,
        hue_sectors: fit.sectors.map(|s| s.to_json()),
    };
    let mut text = serde_json::to_string_pretty(&definition)?;
    text.push('\n');
    effects::compile_definition(&effects::parse_json(text.as_bytes())?)?;
    Ok(text)
}

/// How closely an effect follows its LUT, from the held-out colors.
///
/// The bounds are provisional. They were chosen so that the grades agree with the pixel errors
/// of 328 third-party LUTs on two natural sample pictures, where A is a mean dE2000 up to 1.5
/// and a 95th percentile up to 3, B up to 3 and 6, and C a mean up to 5. 277 grades agree,
/// and 11 are one grade too good. Calibrate them again with more pictures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grade {
    A,
    B,
    C,
    D,
}

impl Grade {
    /// (mean, 95th percentile) bounds of the held-out errors for A, B, and C.
    const BOUNDS: [(f64, f64); 3] = [(0.9, 3.75), (1.8, 6.5), (5.5, f64::INFINITY)];

    pub fn of(fit: &Fit) -> Self {
        let within =
            |(mean, p95): (f64, f64)| fit.holdout_mean_de <= mean && fit.holdout_p95_de <= p95;
        match Self::BOUNDS.iter().position(|b| within(*b)) {
            Some(0) => Self::A,
            Some(1) => Self::B,
            Some(_) => Self::C,
            None => Self::D,
        }
    }

    pub fn letter(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::A => "nearly the same as the LUT",
            Self::B => "the same look with small differences",
            Self::C => "a similar look with visible color differences",
            Self::D => "a different look",
        }
    }
}

/// A warning for a grade of C or D, for a caller that does not describe the grade otherwise.
pub fn grade_warning(fit: &Fit) -> Option<String> {
    let grade = Grade::of(fit);
    (grade >= Grade::C).then(|| {
        format!(
            "Grade {}: {}. The look needs changes that one color matrix and three tone \
             curves cannot make.",
            grade.letter(),
            grade.summary()
        )
    })
}

/// The result of `import`.
#[derive(Clone, Debug)]
pub struct Imported {
    pub fit: Fit,
    /// The effect.json text.
    pub text: String,
    /// The input encoding that the fit used: the requested one, or the guess.
    pub input: Encoding,
    /// Conditions that the user should check before using the effect. The grade is in `fit`
    /// (`Grade::of`, `grade_warning`).
    pub warnings: Vec<String>,
}

/// The inputs that `guess_input` chooses from. Linear is left out: a look LUT does not take
/// scene-linear values.
const GUESSES: [Encoding; 6] = [
    Encoding::Standard,
    Encoding::Srgb,
    Encoding::Vlog,
    Encoding::Flog,
    Encoding::Flog2,
    Encoding::Flog2C,
];
/// How many times closer to Standard a log input must keep the LUT than the best of Standard
/// and sRGB, for `guess_input` to choose it. On 32 LUTs of known input, 1.15 to 1.5 gave the
/// same choices.
const LOG_MARGIN: f64 = 1.3;

/// How far a LUT read with `input` is from the camera's Standard rendering: the RMS of the
/// neutral luma from -6 to +3 EV, plus the mean CIEDE2000 / 100 of the ColorChecker at -1, 0,
/// and +1 EV. A look changes a normal rendering moderately, so with the input that it expects,
/// a LUT stays close. With another input it loses or doubles the contrast of a log curve.
pub fn input_distance(model: &Model, cube: &Cube, input: Encoding) -> f64 {
    let target = cube_target(model, cube, input, 0.0);
    let luma = |c: Rgb| LUMA[0] * c[0] + LUMA[1] * c[1] + LUMA[2] * c[2];
    let grays: Vec<f64> = (0..=36)
        .map(|i| 0.18 * 2f64.powf(-6.0 + f64::from(i) * 0.25))
        .collect();
    let tone = (grays
        .iter()
        .map(|g| (luma(target([*g; 3])) - luma(model.standard([*g; 3]))).powi(2))
        .sum::<f64>()
        / grays.len() as f64)
        .sqrt();
    let converter = LabConverter::default();
    let chart = colorchecker();
    let mut color = 0.0;
    for ev in [-1, 0, 1] {
        for patch in chart {
            let c = patch.map(|v| v * 2f64.powi(ev));
            color += delta_e2000(converter.lab(target(c)), converter.lab(model.standard(c)));
        }
    }
    tone + color / (3.0 * chart.len() as f64) / 100.0
}

/// The distance of each input that `guess_input` chooses from.
fn guess_distances(model: &Model, cube: &Cube) -> Vec<(Encoding, f64)> {
    GUESSES
        .iter()
        .map(|input| (*input, input_distance(model, cube, *input)))
        .collect()
}

/// The guess from `guess_distances`: a log input only when it is `LOG_MARGIN` times closer
/// than the closest display input.
fn choose(distances: &[(Encoding, f64)]) -> Encoding {
    let best = |log: bool| {
        distances
            .iter()
            .filter(|(input, _)| log != input.is_display())
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .copied()
            .expect("GUESSES has log and display inputs")
    };
    let (log, display) = (best(true), best(false));
    if display.1 >= LOG_MARGIN * log.1 {
        log.0
    } else {
        Encoding::Standard
    }
}

/// The input that a LUT most likely expects, from its data alone (`input_distance`). A log
/// input wins only when it is `LOG_MARGIN` times closer than Standard and sRGB; otherwise the
/// choice is Standard, as the data does not tell Standard from sRGB reliably.
pub fn guess_input(base_gamma: &[u16; 256], cube: &Cube) -> Encoding {
    choose(&guess_distances(&Model::new(base_gamma), cube))
}

/// Fits a `.cube` file or HALD PNG over `base_gamma` and returns the fit and its effect.json.
pub fn import(bytes: &[u8], base_gamma: &[u16; 256], request: &Import) -> Result<Imported> {
    ensure!(
        request.exposure.is_finite() && request.exposure.abs() <= MAX_EXPOSURE,
        "Expected an exposure from -{MAX_EXPOSURE} to {MAX_EXPOSURE} EV"
    );
    // Reject an invalid ID, name, or icon before the fit, which takes seconds.
    effects::check_identity(request.effect_id, request.name)?;
    effects::icon_pixels(request.icon)?;
    let cube = Cube::read(bytes)?;
    let model = Model::new(base_gamma);
    // The data, not the file name, tells the input.
    let distances = guess_distances(&model, &cube);
    let guess = choose(&distances);
    let input = request.input.unwrap_or(guess);
    let fit = fit(
        base_gamma,
        &cube_target(&model, &cube, input, request.exposure),
        &request.options,
    )?;
    let text = definition_text(&fit, request, input, packaging::sha256(bytes))?;
    let mut warnings = Vec::new();
    let limit = (MATRIX_LIMIT * 8192.0) as i32 - 64;
    if fit.matrix.iter().any(|v| (*v as i32).abs() >= limit) {
        warnings.push(format!(
            "The fit reached the matrix limit of {MATRIX_LIMIT}. The effect mixes the color \
             channels strongly and can show color noise at high ISO."
        ));
    }
    if cube.image_layout.is_some()
        && crate::lut_image::noise(&cube.table, cube.size) > crate::lut_image::NOISE_LIMIT
    {
        warnings.push(
            "The LUT image is noisy, as from grain or sharpening. Export it again with grain, \
             sharpening, clarity, texture, dehaze, vignette, and noise reduction off."
                .to_owned(),
        );
    }
    if let Some(layout) = cube.image_layout.filter(|l| *l != Layout::Hald) {
        warnings.push(format!(
            "Read the image as a LUT in {}, not as a HALD image",
            layout.name()
        ));
    }
    // A guess that keeps the LUT much closer to Standard than the chosen input does. The data
    // does not tell Standard from sRGB, so the two do not warn about each other.
    let distance = |input: Encoding| {
        distances
            .iter()
            .find(|(guessed, _)| *guessed == input)
            .map_or_else(|| input_distance(&model, &cube, input), |(_, d)| *d)
    };
    if guess != input
        && !(guess.is_display() && input.is_display())
        && distance(input) >= LOG_MARGIN * distance(guess)
    {
        warnings.push(format!(
            "The LUT data suggests {} input, but the import used {}",
            guess.name(),
            input.name()
        ));
    }
    if cube.video_input || cube.video_output {
        warnings.push(
            "The LUT uses video levels. GRKit converts 64-940 of 1023, an assumption that \
             Resolve does not document."
                .to_owned(),
        );
    }
    Ok(Imported {
        fit,
        text,
        input,
        warnings,
    })
}
