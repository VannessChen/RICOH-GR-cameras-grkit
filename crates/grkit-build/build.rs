//! Lists every file under `runtime/` as `RUNTIME_SOURCES`, so a new runtime
//! file needs no entry in the build crate.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn collect(root: &Path, directory: &Path, files: &mut Vec<String>) {
    for entry in fs::read_dir(directory).expect("read the runtime directory") {
        let path = entry.expect("read a runtime entry").path();
        if path.is_dir() {
            collect(root, &path, files);
        } else {
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .expect("a UTF-8 runtime path");
            files.push(name.replace('\\', "/"));
        }
    }
}

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../runtime");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    collect(&root, &root, &mut files);
    files.sort();
    let mut out = String::from("pub const RUNTIME_SOURCES: &[(&str, &[u8])] = &[\n");
    for name in files {
        out.push_str(&format!(
            "    ({name:?}, include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../runtime/\", {name:?}))),\n"
        ));
    }
    out.push_str("];\n");
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("runtime_sources.rs"),
        out,
    )
    .expect("write the runtime source list");
}
