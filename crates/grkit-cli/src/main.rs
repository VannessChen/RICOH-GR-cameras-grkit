//! Native command-line adapter. Formats and package rules live in grkit-core.

mod storage;

use anyhow::{ensure, Context, Result};
use clap::{Parser, Subcommand};
use grkit_core::{cube, effects, fit, modules::Selection, packaging, site, Files};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Room for a 256-point 3D table written with generous precision.
const MAX_LUT_SIZE: usize = 1024 * 1024 * 1024;

#[derive(Parser)]
#[command(name = "grkit", version, about = "Build GRKit plugins and SD effects")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check the local ARM toolchain.
    Doctor {
        #[arg(long, default_value = "clang")]
        clang: String,
        #[arg(long, default_value = "ld.lld")]
        lld: String,
    },
    /// Build the production runtime from official firmware.
    Build {
        #[arg(long, value_parser = grkit_build::target::TARGETS.map(|target| target.name))]
        target: String,
        #[arg(long)]
        firmware: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "clang")]
        clang: String,
        #[arg(long, default_value = "ld.lld")]
        lld: String,
    },
    /// Port the firmware addresses of a runtime build to another build of the
    /// firmware, such as a new version (docs/architecture.md, Porting).
    Port {
        /// The runtime build to port.
        #[arg(long, value_parser = grkit_build::target::TARGETS.map(|target| target.name))]
        target: String,
        /// The official firmware of the runtime build.
        #[arg(long)]
        firmware: PathBuf,
        /// The official firmware of the other build.
        #[arg(long, required_unless_present = "to_rtos", conflicts_with = "to_rtos")]
        to: Option<PathBuf>,
        /// The RTOS image of the other build, such as a GRDUMP capture.
        #[arg(long)]
        to_rtos: Option<PathBuf>,
        /// The search range on each side of an address, in bytes, such as
        /// 0x80000.
        #[arg(long, default_value_t = grkit_build::port::RADIUS, value_parser = parse_u32)]
        radius: u32,
        /// A runtime build of the other firmware to compare the port with.
        #[arg(long, value_parser = grkit_build::target::TARGETS.map(|target| target.name))]
        check: Option<String>,
        /// A new directory for report.json, the draft header, and the draft
        /// site tables.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Compile effect definitions.
    Effects {
        #[command(subcommand)]
        command: EffectsCommand,
    },
    /// Package a verified local runtime and effects.
    Package {
        #[arg(long)]
        build: PathBuf,
        /// Compiled effects. Without effects, the installer does not install Image Control.
        #[arg(long)]
        effects: Option<PathBuf>,
        /// The plugins to install: image-control, framing, guides, date-stamp,
        /// grain. Without this option, the package installs each plugin.
        #[arg(long, value_delimiter = ',')]
        plugins: Option<Vec<String>>,
        /// The camera that the package enables SD scripts on.
        #[arg(long, default_value = "gr3", value_parser = grkit_core::camera::CAMERAS.map(|camera| camera.id))]
        camera: String,
        /// The language of the installation guide.
        #[arg(long, default_value = "en", value_parser = packaging::Language::ALL.map(packaging::Language::code))]
        language: String,
        /// The PDF of the installation guide for the camera, in the language of
        /// `--language`. `grkit guides` writes its source.
        #[arg(long)]
        guide: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Write the Typst source of each installation guide to
    /// OUT/<camera>/<language>/guide.typ, next to the images that it shows.
    Guides {
        #[arg(long)]
        out: PathBuf,
    },
    /// Check package layout, profiles and checksums.
    Verify { archive: PathBuf },
    /// List the cameras, the firmware releases that each runs, and the runtime
    /// build for each release.
    Cameras,
    /// Write the ZIP that turns SD scripts off on a camera: its ModeSet file
    /// with the disable command and DEVELOP.MOD.
    ScriptsOff {
        #[arg(long, value_parser = grkit_core::camera::CAMERAS.map(|camera| camera.id))]
        camera: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Write the ZIP of the firmware dump tool, which copies the running RTOS
    /// image of a camera to its SD card. One ZIP serves each camera.
    Dump {
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "clang")]
        clang: String,
        #[arg(long, default_value = "ld.lld")]
        lld: String,
    },
    /// Write a verified runtime build into a static site directory. The site
    /// must not have a runtime for the same target yet.
    Site {
        #[arg(long)]
        build: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum EffectsCommand {
    Build {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Write effects, preview tables, and their catalog into a new site directory.
    Publish {
        #[arg(long)]
        source: PathBuf,
        /// The runtime target whose catalog this is.
        #[arg(long, default_value = "gr3-2.10")]
        target: String,
        /// The official firmware of the target, the source of the Standard base gamma.
        #[arg(long)]
        firmware: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Fit a .cube LUT or a HALD CLUT PNG into a new effect definition at OUT/ID/effect.json.
    Import {
        #[arg(long, visible_alias = "cube")]
        lut: PathBuf,
        /// Official GR III 2.10 firmware, the source of the Standard base gamma.
        #[arg(long)]
        firmware: PathBuf,
        /// What the LUT expects: standard for a GR JPEG in Standard, srgb for sRGB-encoded
        /// scene light without a tone curve, or a log encoding. auto guesses it from the LUT
        /// data (`fit::guess_input`): Standard or a log encoding, never srgb or linear.
        #[arg(long, default_value = "auto",
            value_parser = ["auto", "standard", "srgb", "linear", "vlog", "flog", "flog2", "flog2c"])]
        input: String,
        /// Exposure offset in EV, applied to scene-linear light before the LUT.
        #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
        exposure: f64,
        /// Fit no post-curve matrix.
        #[arg(long)]
        no_post: bool,
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        label: String,
        #[arg(long, value_parser = ["film", "filter", "plain"])]
        shape: String,
        #[arg(long)]
        out: PathBuf,
    },
}

/// A number in decimal or with a 0x prefix.
fn parse_u32(text: &str) -> Result<u32, std::num::ParseIntError> {
    match text.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => text.parse(),
    }
}

fn run(command: Command) -> Result<Value> {
    match command {
        Command::Doctor { clang, lld } => {
            let tools = grkit_build::Toolchain::discover(&clang, &lld)?;
            Ok(json!({"toolchain": tools.manifest(), "grkit_version": env!("CARGO_PKG_VERSION")}))
        }
        Command::Build {
            target,
            firmware,
            out,
            clang,
            lld,
        } => {
            let target = grkit_build::target::find(&target)?;
            let result = grkit_build::build_runtime(target, &firmware, &out, &clang, &lld)?;
            Ok(
                json!({"output": result.output, "target": result.manifest["target"], "files": result.files.len()}),
            )
        }
        Command::Port {
            target,
            firmware,
            to,
            to_rtos,
            radius,
            check,
            out,
        } => {
            let target = grkit_build::target::find(&target)?;
            if let Some(out) = &out {
                storage::validate_output(out)?;
            }
            let base = grkit_build::verify_firmware(&firmware, target)?;
            let other = match (to, to_rtos) {
                (Some(path), _) => {
                    let decoded = grkit_build::firmware::decode(
                        &std::fs::read(&path)
                            .with_context(|| format!("Read {}", path.display()))?,
                    )?;
                    grkit_build::firmware::rtos_section(&decoded)?.to_vec()
                }
                (None, Some(path)) => {
                    std::fs::read(&path).with_context(|| format!("Read {}", path.display()))?
                }
                (None, None) => anyhow::bail!("Give --to or --to-rtos"),
            };
            let port = grkit_build::port::port(&base, &other, radius)?;
            let check = check
                .map(|name| grkit_build::port::check(&port, grkit_build::target::find(&name)?))
                .transpose()?;
            if let Some(out) = &out {
                let files: Files = [
                    ("report.json", serde_json::to_vec_pretty(&port.report())?),
                    ("header.h", port.draft_header()?.into_bytes()),
                    ("sites.rs", port.draft_layout()?.into_bytes()),
                ]
                .into_iter()
                .map(|(name, bytes)| (name.to_string(), bytes))
                .collect();
                storage::write_directory(out, &files)?;
            }
            let unported: Vec<Value> = port
                .unported()
                .map(|ported| {
                    json!({"name": ported.address.name, "base": format!("{:#x}", ported.address.address),
                        "error": ported.port.as_ref().err()})
                })
                .collect();
            Ok(json!({
                "target": target.name,
                "addresses": port.addresses.len(),
                "ported": port.addresses.len() - unported.len(),
                "unported": unported,
                "largest_move": format!("{:#x}", port.largest_move()),
                "relocatable": port.relocatable,
                "check": check,
                "output": out,
            }))
        }
        Command::Effects {
            command: EffectsCommand::Build { source, out },
        } => {
            let compiled = storage::compile_directory(&source)?;
            storage::validate_output(&out)?;
            storage::write_directory(&out, &compiled)?;
            Ok(json!({"output": out, "effects": compiled.len()}))
        }
        Command::Effects {
            command:
                EffectsCommand::Publish {
                    source,
                    target,
                    firmware,
                    out,
                },
        } => {
            let target = grkit_build::target::find(&target)?;
            let definitions = storage::read_definitions(&source)?;
            storage::validate_output(&out)?;
            let image = grkit_build::verify_firmware(&firmware, target)?;
            let base_gamma = grkit_build::firmware::standard_base_gamma(&image)?;
            let files = site::publish_effects(&definitions, &base_gamma, target.name)?;
            storage::write_directory(&out, &files)?;
            Ok(json!({"output": out, "target": target.name, "effects": definitions.len()}))
        }
        Command::Effects {
            command:
                EffectsCommand::Import {
                    lut,
                    firmware,
                    input,
                    exposure,
                    no_post,
                    id,
                    name,
                    label,
                    shape,
                    out,
                },
        } => {
            ensure!(effects::valid_id(&id), "Invalid effect ID");
            let output = out.join(&id);
            storage::validate_output(&output)?;
            let image = grkit_build::verify_firmware(&firmware, &grkit_build::target::GR3_210)?;
            let base_gamma = grkit_build::firmware::standard_base_gamma(&image)?;
            let bytes = storage::read_bounded(&lut, MAX_LUT_SIZE)?;
            let input = if input == "auto" {
                fit::guess_input(&base_gamma, &cube::Cube::read(&bytes)?)
            } else {
                input.parse()?
            };
            let request = fit::Import {
                effect_id: &id,
                name: &name,
                label: &label,
                shape: &shape,
                source: lut
                    .file_name()
                    .and_then(|name| name.to_str())
                    .context("Expected a UTF-8 LUT filename")?,
                input,
                exposure,
                options: fit::Options {
                    post: !no_post,
                    ..fit::Options::default()
                },
            };
            let imported = fit::import(&bytes, &base_gamma, &request)?;
            let errors =
                effects::parse_json(imported.text.as_bytes())?["attribution"]["fit"].take();
            storage::write_directory(
                &output,
                &Files::from([("effect.json".to_owned(), imported.text.into_bytes())]),
            )?;
            Ok(json!({"output": output, "effect_id": id, "fit": errors,
                "post_matrix": imported.fit.post_matrix.is_some(),
                "monochrome": imported.fit.monochrome, "toned": imported.fit.toned,
                "warnings": imported.warnings}))
        }
        Command::Package {
            build,
            effects,
            plugins,
            camera,
            language,
            guide,
            out,
        } => {
            let (runtime, manifest) = storage::load_build(&build)?;
            let effects = match effects {
                Some(directory) => storage::read_effects(&directory)?,
                None => Files::new(),
            };
            let plugins = match plugins {
                Some(names) => Selection::from_names(names.iter().map(String::as_str))?,
                None => Selection::ALL,
            };
            let camera = grkit_core::camera::find(&camera)?;
            let language = packaging::Language::parse(&language)?;
            let pdf = storage::read_bounded(&guide, packaging::MAX_FILE_SIZE)?;
            let bytes = packaging::assemble(
                &runtime,
                &manifest,
                &effects,
                plugins,
                camera,
                &packaging::Guide {
                    language,
                    pdf: &pdf,
                },
            )?;
            let verified = packaging::verify_package(&bytes)?;
            storage::write_new(&out, &bytes)?;
            Ok(
                json!({"path": out, "sha256": packaging::sha256(&bytes), "camera": verified["camera"],
                "entries": verified["entries"], "plugins": verified["plugins"],
                "language": verified["language"]}),
            )
        }
        Command::Guides { out } => {
            storage::validate_output(&out)?;
            let mut tree = Files::new();
            for camera in grkit_core::camera::CAMERAS {
                for language in packaging::Language::ALL {
                    for (name, bytes) in grkit_core::guide::source(camera, language)? {
                        tree.insert(format!("{}/{}/{name}", camera.id, language.code()), bytes);
                    }
                }
            }
            storage::write_directory(&out, &tree)?;
            Ok(
                json!({"output": out, "guides": grkit_core::camera::CAMERAS.len() * packaging::Language::ALL.len()}),
            )
        }
        Command::Cameras => Ok(
            json!({"cameras": grkit_core::camera::CAMERAS.map(|camera| json!({
                "id": camera.id, "name": camera.name, "hdf": camera.hdf, "modeset": camera.modeset,
                "releases": camera.releases.iter().map(|release| json!({
                    "firmware": release.firmware, "target": release.target,
                    "experimental": release.experimental,
                })).collect::<Vec<_>>(),
            }))}),
        ),
        Command::Verify { archive } => {
            // Include room for ZIP metadata while bounding the archive read itself.
            let bytes = storage::read_bounded(&archive, 64 * 1024 * 1024)?;
            let verified = packaging::verify_package(&bytes)?;
            Ok(
                json!({"valid": true, "camera": verified["camera"], "target": verified["target"],
                "effects": verified["effect_ids"].as_array().map_or(0, Vec::len),
                "plugins": verified["plugins"], "language": verified["language"]}),
            )
        }
        Command::ScriptsOff { camera, out } => {
            let camera = grkit_core::camera::find(&camera)?;
            let bytes = packaging::assemble_scripts_off(camera)?;
            storage::write_new(&out, &bytes)?;
            Ok(
                json!({"path": out, "sha256": packaging::sha256(&bytes), "camera": camera.id,
                "modeset": camera.modeset}),
            )
        }
        Command::Dump { out, clang, lld } => {
            let tools = grkit_build::Toolchain::discover(&clang, &lld)?;
            let bytes = packaging::assemble_dump(grkit_build::dump::files(&tools)?)?;
            storage::write_new(&out, &bytes)?;
            Ok(json!({"path": out, "sha256": packaging::sha256(&bytes)}))
        }
        Command::Site { build, out } => {
            let (target, files) = storage::export_runtime(&build, &out)?;
            Ok(json!({"output": out, "target": target, "files": files}))
        }
    }
}

fn main() -> std::process::ExitCode {
    match run(Cli::parse().command) {
        Ok(result) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&result).expect("JSON result is serializable")
            );
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("grkit: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
