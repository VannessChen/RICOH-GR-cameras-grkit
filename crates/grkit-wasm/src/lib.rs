//! Browser entry points that assemble a GRKit SD package and import LUTs as effects.
//!
//! The site serves a runtime build as CI publishes it: `build.json` and the files under `sd/`.
//! The page fetches those files and the selected effects, then builds the ZIP locally.
//! `Builder` holds the logic so that native tests cover it. `Package` only adapts it for JS.
//! `import_lut` fits a LUT as `grkit effects import` does, over the base gamma of the catalog.

use anyhow::{ensure, Context, Result};
use grkit_core::{
    camera::{self, Camera},
    cube::Cube,
    effects, fit, lut_image,
    modules::Selection,
    packaging, runtime, site, Files,
};
use serde_json::Value;
use wasm_bindgen::prelude::*;

/// The largest LUT file that the site imports. A 16-bit TIFF HALD image of level 16 has
/// about 100 MB. The CLI accepts more, because it does not share the memory of a page.
pub const MAX_LUT_SIZE: usize = 128 * 1024 * 1024;

/// An effect that `import_lut` made from a LUT.
pub struct Imported {
    /// The effect.json text, as `grkit effects import` writes it.
    pub definition: String,
    /// The compiled `.GRF` effect for the package.
    pub blob: Vec<u8>,
    /// The RAW preview table, as `grkit effects publish` writes it for a catalog effect.
    pub preview: Vec<u8>,
    /// The ADJ icon: 40 x 40 RGBA pixels.
    pub icon: Vec<u8>,
    pub monochrome: bool,
    /// A to D: how closely the effect follows the LUT (`fit::Grade`). The page shows a
    /// description of it, not the letter.
    pub grade: &'static str,
    /// Conditions that the user should check before using the effect. The page describes the
    /// grade itself.
    pub warnings: Vec<String>,
}

/// What the user gives for an import. The fields are the `grkit effects import` options.
pub struct ImportRequest<'a> {
    pub effect_id: &'a str,
    pub name: &'a str,
    /// The `icon` of effect.json: a film cover (`template` box or cine) or a label tile.
    pub icon: &'a Value,
    /// The file name, recorded in the attribution.
    pub source: &'a str,
    /// The input encoding name: `standard`, `srgb`, `linear`, a log encoding, or `auto` for
    /// the one that the LUT data suggests.
    pub input: &'a str,
}

/// Rejects a LUT file over `MAX_LUT_SIZE` and reads the site catalog's base gamma.
fn lut_inputs(bytes: &[u8], base_gamma: &[u16]) -> Result<[u16; 256]> {
    ensure!(
        bytes.len() <= MAX_LUT_SIZE,
        "The LUT file is larger than {} MB",
        MAX_LUT_SIZE >> 20
    );
    site::base_gamma(base_gamma)
}

/// Fits a `.cube` file or a LUT image over `base_gamma`, the table of the site catalog.
pub fn import_lut(bytes: &[u8], base_gamma: &[u16], request: &ImportRequest) -> Result<Imported> {
    import_lut_with_progress(bytes, base_gamma, request, &|_| {})
}

/// `import_lut`, which reports the progress of the fit to `progress`.
pub fn import_lut_with_progress(
    bytes: &[u8],
    base_gamma: &[u16],
    request: &ImportRequest,
    progress: fit::Progress,
) -> Result<Imported> {
    let base_gamma = lut_inputs(bytes, base_gamma)?;
    let imported = fit::import_with_progress(
        bytes,
        &base_gamma,
        &fit::Import {
            effect_id: request.effect_id,
            name: request.name,
            icon: request.icon,
            source: request.source,
            input: match request.input {
                "auto" => None,
                name => Some(name.parse()?),
            },
            exposure: 0.0,
            options: fit::Options::default(),
        },
        progress,
    )?;
    let blob = effects::compile_definition(&effects::parse_json(imported.text.as_bytes())?)?;
    let (effect, icon) = effects::decode(&blob)?;
    Ok(Imported {
        preview: site::linear_preview_lut(&base_gamma, &effect)?,
        icon,
        blob,
        monochrome: imported.fit.monochrome,
        grade: fit::Grade::of(&imported.fit).letter(),
        warnings: imported.warnings,
        definition: imported.text,
    })
}

/// The input that a LUT most likely expects, from its data alone: see `fit::guess_input`.
pub fn guess_input(bytes: &[u8], base_gamma: &[u16]) -> Result<&'static str> {
    let base_gamma = lut_inputs(bytes, base_gamma)?;
    Ok(fit::guess_input(&base_gamma, &Cube::read(bytes)?).name())
}

/// Collects one runtime build and a set of compiled effects, then assembles the package.
pub struct Builder {
    manifest: Vec<u8>,
    paths: Vec<String>,
    runtime: Files,
    effects: Files,
    plugins: Selection,
    camera: &'static Camera,
    language: packaging::Language,
    guide: Option<Vec<u8>>,
}

impl Builder {
    /// Starts a package from the runtime's `build.json`.
    pub fn new(manifest: &[u8]) -> Result<Self> {
        let parsed = packaging::read_manifest(manifest)?;
        let paths = parsed["files"]
            .as_array()
            .context("Expected a manifest file list")?
            .iter()
            .map(|item| {
                item["path"]
                    .as_str()
                    .map(str::to_owned)
                    .context("Expected a runtime path")
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            manifest: manifest.to_vec(),
            paths,
            runtime: Files::new(),
            effects: Files::new(),
            plugins: Selection::ALL,
            camera: &camera::GR3,
            language: packaging::Language::English,
            guide: None,
        })
    }

    /// The runtime files to fetch, as paths under the build's `sd/` directory.
    pub fn runtime_paths(&self) -> &[String] {
        &self.paths
    }

    /// Adds one runtime file. `build` checks its size and SHA-256 against the manifest.
    pub fn add_runtime_file(&mut self, path: &str, bytes: &[u8]) -> Result<()> {
        ensure!(
            self.paths.iter().any(|known| known == path),
            "Not a runtime file of this build: {path}"
        );
        ensure!(
            !self.runtime.contains_key(path),
            "Duplicate runtime file: {path}"
        );
        self.runtime.insert(path.to_owned(), bytes.to_vec());
        Ok(())
    }

    /// Adds one compiled effect and returns its ID. The package filename comes from the ID.
    pub fn add_effect(&mut self, blob: &[u8]) -> Result<String> {
        let (effect, _) = effects::decode(blob)?;
        let id = effect["effect_id"]
            .as_str()
            .context("Expected an effect ID")?
            .to_owned();
        let name = effects::filename(&id)?;
        ensure!(
            !self.effects.contains_key(&name),
            "Duplicate effect ID or filename hash collision: {id}"
        );
        ensure!(
            self.effects.len() < runtime::MAX_COUNT,
            "This target supports at most {} effects",
            runtime::MAX_COUNT
        );
        self.effects.insert(name, blob.to_vec());
        Ok(id)
    }

    /// Leaves one plugin out of the package: `image-control`, `framing`, `guides`,
    /// `date-stamp`, or `grain`. The installer changes no firmware for it.
    /// Without effects, the installer leaves out Image Control by itself.
    pub fn omit_plugin(&mut self, name: &str) -> Result<()> {
        let plugins = self.plugins.without(name)?;
        ensure!(plugins != self.plugins, "Plugin omitted twice: {name}");
        self.plugins = plugins;
        Ok(())
    }

    /// Selects the camera: `gr3` (the default), `gr3x`, or `gr4`. The package gets its
    /// ModeSet file. `build` rejects a camera that does not run this runtime.
    pub fn set_camera(&mut self, id: &str) -> Result<()> {
        self.camera = camera::find(id)?;
        Ok(())
    }

    /// Selects the language of the installation guide: `en` (the default),
    /// `zh-Hans`, `zh-Hant`, or `ja`.
    pub fn set_language(&mut self, code: &str) -> Result<()> {
        self.language = packaging::Language::parse(code)?;
        Ok(())
    }

    /// Sets the PDF of the installation guide for the camera, in the language of
    /// `set_language`. The site publishes one for each camera and language.
    pub fn set_guide(&mut self, pdf: &[u8]) {
        self.guide = Some(pdf.to_vec());
    }

    /// Verifies the runtime against its manifest and returns the verified package ZIP.
    pub fn build(&self) -> Result<Vec<u8>> {
        let (runtime, manifest) = packaging::load_runtime(&self.manifest, &self.runtime)?;
        packaging::assemble(
            &runtime,
            &manifest,
            &self.effects,
            self.plugins,
            self.camera,
            &packaging::Guide {
                language: self.language,
                pdf: self
                    .guide
                    .as_deref()
                    .context("Set the installation guide first")?,
            },
        )
    }
}

/// Builds a GRKit SD package from a runtime build and compiled effects.
///
/// Each failing call throws an `Error` whose message has the full cause chain.
#[wasm_bindgen]
pub struct Package(Builder);

#[wasm_bindgen]
impl Package {
    /// Starts a package from the runtime's `build.json` bytes.
    #[wasm_bindgen(constructor)]
    pub fn new(manifest: &[u8]) -> Result<Package, JsError> {
        Builder::new(manifest).map(Package).map_err(js_error)
    }

    /// The runtime files to fetch, as paths under the build's `sd/` directory.
    #[wasm_bindgen(js_name = runtimePaths)]
    pub fn runtime_paths(&self) -> Vec<String> {
        self.0.runtime_paths().to_vec()
    }

    /// Adds one runtime file. `build()` checks its size and SHA-256.
    #[wasm_bindgen(js_name = addRuntimeFile)]
    pub fn add_runtime_file(&mut self, path: &str, bytes: &[u8]) -> Result<(), JsError> {
        self.0.add_runtime_file(path, bytes).map_err(js_error)
    }

    /// Adds one compiled `.GRF` effect and returns its effect ID.
    #[wasm_bindgen(js_name = addEffect)]
    pub fn add_effect(&mut self, blob: &[u8]) -> Result<String, JsError> {
        self.0.add_effect(blob).map_err(js_error)
    }

    /// Leaves one plugin out of the package: `image-control`, `framing`, `guides`,
    /// `date-stamp`, or `grain`.
    #[wasm_bindgen(js_name = omitPlugin)]
    pub fn omit_plugin(&mut self, name: &str) -> Result<(), JsError> {
        self.0.omit_plugin(name).map_err(js_error)
    }

    /// Selects the camera: `gr3` (the default), `gr3x`, or `gr4`.
    #[wasm_bindgen(js_name = setCamera)]
    pub fn set_camera(&mut self, id: &str) -> Result<(), JsError> {
        self.0.set_camera(id).map_err(js_error)
    }

    /// Selects the language of the installation guide: `en`, `zh-Hans`, `zh-Hant`, or `ja`.
    #[wasm_bindgen(js_name = setLanguage)]
    pub fn set_language(&mut self, code: &str) -> Result<(), JsError> {
        self.0.set_language(code).map_err(js_error)
    }

    /// Sets the PDF of the installation guide for the camera, in the selected language.
    #[wasm_bindgen(js_name = setGuide)]
    pub fn set_guide(&mut self, pdf: &[u8]) {
        self.0.set_guide(pdf);
    }

    /// Returns the verified package as ZIP bytes.
    pub fn build(&self) -> Result<Vec<u8>, JsError> {
        self.0.build().map_err(js_error)
    }
}

/// Returns the ZIP that turns SD scripts off on a camera: `gr3`, `gr3x`, or `gr4`.
#[wasm_bindgen(js_name = scriptsOffPackage)]
pub fn scripts_off_package(camera: &str) -> Result<Vec<u8>, JsError> {
    grkit_core::camera::find(camera)
        .and_then(grkit_core::packaging::assemble_scripts_off)
        .map_err(js_error)
}

/// An effect fitted from a LUT. See `importLut`.
#[wasm_bindgen(js_name = ImportedEffect)]
pub struct ImportedEffect(Imported);

#[wasm_bindgen(js_class = ImportedEffect)]
impl ImportedEffect {
    /// The effect.json text.
    #[wasm_bindgen(getter)]
    pub fn definition(&self) -> String {
        self.0.definition.clone()
    }

    /// The compiled `.GRF` effect, for `Package.addEffect`.
    #[wasm_bindgen(getter)]
    pub fn blob(&self) -> Vec<u8> {
        self.0.blob.clone()
    }

    /// The RAW preview table: 33 x 33 x 33 RGB bytes over sRGB-shaped linear input.
    #[wasm_bindgen(getter)]
    pub fn preview(&self) -> Vec<u8> {
        self.0.preview.clone()
    }

    /// The ADJ icon: 40 x 40 RGBA pixels.
    #[wasm_bindgen(getter)]
    pub fn icon(&self) -> Vec<u8> {
        self.0.icon.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn monochrome(&self) -> bool {
        self.0.monochrome
    }

    /// A to D: how closely the effect follows the LUT.
    #[wasm_bindgen(getter)]
    pub fn grade(&self) -> String {
        self.0.grade.to_owned()
    }

    /// Conditions that the user should check before using the effect, in English.
    #[wasm_bindgen(getter)]
    pub fn warnings(&self) -> Vec<String> {
        self.0.warnings.clone()
    }
}

#[wasm_bindgen]
extern "C" {
    /// A function that takes the progress of an import, a fraction from 0 to 1.
    #[wasm_bindgen(typescript_type = "(fraction: number) => void")]
    pub type ImportProgress;

    #[wasm_bindgen(method, js_name = call)]
    fn report(this: &ImportProgress, context: &JsValue, fraction: f64);
}

/// Fits a `.cube` file or a LUT image (PNG or TIFF) into an effect, as `grkit effects import`
/// does. `baseGamma` is the `base_gamma` array of the site catalog. The fit takes seconds,
/// so call it from a Worker. `progress` receives the progress of the fit, a fraction from 0
/// to 1 that does not decrease.
/// `icon` is the JSON text of the effect.json `icon`, such as
/// `{"template":"box","brand":"Kodak","name":"PORTRA","number":"400","band":true}`.
#[wasm_bindgen(js_name = importLut)]
// The arguments are the fields of the worker's import request, one JavaScript value each.
#[allow(clippy::too_many_arguments)]
pub fn import_lut_js(
    bytes: &[u8],
    source: &str,
    input: &str,
    base_gamma: &[u16],
    effect_id: &str,
    name: &str,
    icon: &str,
    progress: &ImportProgress,
) -> Result<ImportedEffect, JsError> {
    let icon: Value =
        serde_json::from_str(icon).map_err(|error| JsError::new(&error.to_string()))?;
    let request = ImportRequest {
        effect_id,
        name,
        icon: &icon,
        source,
        input,
    };
    import_lut_with_progress(bytes, base_gamma, &request, &|fraction| {
        progress.report(&JsValue::NULL, fraction)
    })
    .map(ImportedEffect)
    .map_err(js_error)
}

/// The input that a `.cube` file or LUT image most likely expects (`standard`, `vlog`,
/// `flog`, `flog2`, or `flog2c`), from its data alone. `baseGamma` is the `base_gamma` array
/// of the site catalog. It reads the whole file, so call it from a Worker.
#[wasm_bindgen(js_name = guessInput)]
pub fn guess_input_js(bytes: &[u8], base_gamma: &[u16]) -> Result<String, JsError> {
    guess_input(bytes, base_gamma)
        .map(str::to_owned)
        .map_err(js_error)
}

/// The camera pixels of an effect.json `icon` (JSON text): 40 x 40 RGBA. It fails with the
/// reason when a text does not fit, so the page can check an icon before an import.
#[wasm_bindgen(js_name = iconPixels)]
pub fn icon_pixels_js(icon: &str) -> Result<Vec<u8>, JsError> {
    let icon: Value =
        serde_json::from_str(icon).map_err(|error| JsError::new(&error.to_string()))?;
    effects::icon_pixels(&icon).map_err(js_error)
}

/// An identity HALD PNG of `level` (2 to 16), to turn an editor preset into a LUT image.
#[wasm_bindgen(js_name = haldImage)]
pub fn hald_image(level: usize) -> Result<Vec<u8>, JsError> {
    lut_image::identity_png(level).map_err(js_error)
}

fn js_error(error: anyhow::Error) -> JsError {
    JsError::new(&format!("{error:#}"))
}
