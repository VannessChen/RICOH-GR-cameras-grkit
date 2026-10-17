//! `runtime/abi.h` is generated from `grkit_core::abi`. Set GRKIT_UPDATE_ABI=1
//! to rewrite it after a change of the tables.

use std::{fs, path::Path};

#[test]
fn checked_in_abi_header_matches_the_tables() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime/abi.h");
    let expected = grkit_core::abi::header();
    if std::env::var_os("GRKIT_UPDATE_ABI").is_some() {
        fs::write(&path, &expected).unwrap();
        return;
    }
    let actual = fs::read_to_string(&path).unwrap();
    assert!(
        actual == expected,
        "runtime/abi.h is stale. Run GRKIT_UPDATE_ABI=1 cargo test -p grkit-core --test abi"
    );
}
