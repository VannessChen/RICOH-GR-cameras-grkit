//! Fixtures for the tests of this crate and of the crates that use it. Not a stable API.

use serde_json::{json, Value};

use crate::{effects, packaging, runtime, Files};

/// A stand-in for `INSTALL.BIN`: an ARM ELF32 header with nothing after it, which
/// `packaging::validate_runtime` accepts.
pub fn installer() -> Vec<u8> {
    let mut binary = vec![0; 52];
    binary[..6].copy_from_slice(b"\x7fELF\x01\x01");
    binary[16..20].copy_from_slice(b"\x02\x00\x28\x00");
    binary
}

/// The production runtime files of `target` around `installer`.
pub fn runtime_files(installer: &[u8], target: &str) -> Files {
    runtime::runtime_files_for(installer, runtime::icon_stride(target).unwrap())
}

/// The `build.json` bytes of a runtime build of `target` with the `sd/` files `files`.
pub fn manifest(target: &str, files: &Files) -> Vec<u8> {
    packaging::json_bytes(&Value::Object(packaging::manifest(target, files))).unwrap()
}

/// A runtime build of `target` with the stand-in installer: its `build.json` bytes and
/// its `sd/` files.
pub fn build(target: &str) -> (Vec<u8>, Files) {
    let files = runtime_files(&installer(), target);
    (manifest(target, &files), files)
}

/// The verified runtime of `build(target)`.
pub fn runtime(target: &str) -> packaging::VerifiedRuntime {
    let (manifest, files) = build(target);
    packaging::load_runtime(&manifest, &files).unwrap()
}

/// The effect set of `effect(id)` for each of `ids`.
pub fn effect_set(ids: &[&str]) -> packaging::EffectSet {
    let mut set = packaging::EffectSet::default();
    for id in ids {
        set.insert(effect(id)).unwrap();
    }
    set
}

/// A compiled effect with the identity matrix, linear curves, and a transparent icon.
pub fn effect(id: &str) -> Vec<u8> {
    let curve: Vec<_> = (0..256).map(|i| i * 64).collect();
    let value = json!({"effect_id": id, "name": "Test", "flags": 0,
        "matrix": [8192, 0, 0, 0, 8192, 0, 0, 0, 8192],
        "curves": [&curve, &curve, &curve]});
    effects::encode(&value, &vec![0; effects::ICON_SIZE]).unwrap()
}
