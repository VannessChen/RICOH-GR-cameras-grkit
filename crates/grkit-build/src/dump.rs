//! The firmware dump tool (`runtime/dump/`): a read-only Linux program that
//! copies the running RTOS image to the SD card, and the startup script that
//! runs it. It needs no firmware, so one build serves each camera.

use crate::{components::path, toolchain, Toolchain, RUNTIME_SOURCES};
use anyhow::{Context, Result};
use grkit_core::Files;
use std::fs;

fn source(name: &str) -> Result<&'static [u8]> {
    RUNTIME_SOURCES
        .iter()
        .find(|(file, _)| *file == name)
        .map(|(_, bytes)| *bytes)
        .with_context(|| format!("The runtime has no {name}"))
}

/// The files of the dump ZIP by their path in it. The program is compiled with
/// the options of the installer and linked at the default address.
pub fn files(tools: &Toolchain) -> Result<Files> {
    let directory = tempfile::Builder::new().prefix("grkit-dump-").tempdir()?;
    let source_path = directory.path().join("dump.c");
    let object = directory.path().join("dump.o");
    let binary = directory.path().join("DUMP.BIN");
    fs::write(&source_path, source("dump/dump.c")?)?;
    toolchain::run(
        &tools.clang,
        [
            "--target=armv7-linux-gnueabi",
            "-march=armv7-a",
            "-marm",
            "-O2",
            "-ffreestanding",
            "-fno-builtin",
            "-fno-stack-protector",
            "-fomit-frame-pointer",
            "-fno-pic",
            "-c",
            path(&source_path)?,
            "-o",
            path(&object)?,
        ],
    )?;
    toolchain::run(
        &tools.lld,
        [
            "-m",
            "armelf_linux_eabi",
            "-static",
            "-e",
            "_start",
            "--build-id=none",
            "-o",
            path(&binary)?,
            path(&object)?,
        ],
    )?;
    Ok(Files::from([
        ("README.txt".into(), source("dump/README.txt")?.to_vec()),
        (
            "SD_ROOT/SCRIPT/STARTUP.TTL".into(),
            source("dump/STARTUP.TTL")?.to_vec(),
        ),
        (
            "SD_ROOT/SCRIPT/GRDUMP/DUMP.SH".into(),
            source("dump/DUMP.SH")?.to_vec(),
        ),
        ("SD_ROOT/SCRIPT/GRDUMP/DUMP.BIN".into(), fs::read(binary)?),
    ]))
}
