//! Lists every file under `runtime/` as `RUNTIME_SOURCES`, so a new runtime
//! file needs no entry in the build crate. Also lists the Rust sources and the
//! assets that can change a runtime build as `BUILD_SOURCES`, by their path in
//! the repository, for the source hashes of `build.json`.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// The files under `directory`, by their path from `root`, without those
/// under a directory named in `skip`.
fn collect(root: &Path, directory: &Path, skip: &[&str], files: &mut Vec<String>) {
    for entry in fs::read_dir(directory).expect("read a source directory") {
        let path = entry.expect("read a source entry").path();
        if path.is_dir() {
            if !skip.iter().any(|name| path.file_name().unwrap() == *name) {
                collect(root, &path, skip, files);
            }
        } else {
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .expect("a UTF-8 source path");
            files.push(name.replace('\\', "/"));
        }
    }
}

/// A constant with each file of `files` and its bytes. `base` is the path of
/// the names from the manifest directory.
fn table(name: &str, base: &str, files: &[String]) -> String {
    let mut out = format!("pub const {name}: &[(&str, &[u8])] = &[\n");
    for file in files {
        out.push_str(&format!(
            "    ({file:?}, include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), {base:?}, {file:?}))),\n"
        ));
    }
    out.push_str("];\n");
    out
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let repository = manifest.join("../..");
    let runtime = repository.join("runtime");
    println!("cargo:rerun-if-changed={}", runtime.display());
    let mut files = Vec::new();
    collect(&runtime, &runtime, &[], &mut files);
    files.sort();
    let mut out = table("RUNTIME_SOURCES", "/../../runtime/", &files);

    // The tests of the build crate do not change a build.
    let mut files = Vec::new();
    for (directory, skip) in [
        ("crates/grkit-build/src", &["tests"][..]),
        ("crates/grkit-core/src", &[]),
        ("crates/grkit-core/assets", &[]),
    ] {
        let path = repository.join(directory);
        println!("cargo:rerun-if-changed={}", path.display());
        collect(&repository, &path, skip, &mut files);
    }
    for file in [
        "Cargo.toml",
        "Cargo.lock",
        "crates/grkit-build/Cargo.toml",
        "crates/grkit-core/Cargo.toml",
    ] {
        println!("cargo:rerun-if-changed={}", repository.join(file).display());
        files.push(file.to_owned());
    }
    files.sort();
    out.push_str(&table("BUILD_SOURCES", "/../../", &files));
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("sources.rs"),
        out,
    )
    .expect("write the source lists");
}
