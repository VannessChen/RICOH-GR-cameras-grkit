# Contributing

## Set up

- Rust 1.92 or later. Use the checked-in `Cargo.lock`.
- Clang and LLD with ARM support for runtime builds. No ARM libc or sysroot is necessary. Effect builds and package verification need no ARM compiler.
- The official GR III 2.10 firmware for runtime builds. Its SHA-256 is `d6a3c9c288d784080e7b283222ecdd8752adfeed6faa36dab44f73115354bfd4`. Other firmware fails validation.

If a compiler is not on `PATH`, pass `--clang` or `--lld`. `grkit doctor` checks the toolchain.
The reference toolchain is Apple Clang 21.0.0 with LLD 21.1.8. Other versions can change instructions or code sizes.

`grkit build` writes `sd/` (the runtime files), `objects/` (sources, intermediate objects, and the installer), and `build.json` (the hashes of the inputs, sources, tools, and outputs).
Identical inputs give identical outputs.

## Test

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
GRKIT_REQUIRE_NATIVE_TESTS=1 cargo test --workspace --locked
cargo deny --locked check
```

`cargo deny` checks the dependency licenses, the security advisories, and the crate sources against `deny.toml`. Each ignored advisory in that file gives its reason.

Host C contract tests compile the runtime's GRF parser, effect catalog, menu, and Framing code with a host C compiler.
They use the GR III target. The picture contract also runs for GR IV, and a syntax check compiles the C sources of each target.
If no host compiler is available, the tests are skipped. `GRKIT_REQUIRE_NATIVE_TESTS=1` makes them fail instead.

The ARM builds are ignored tests. The second command needs the firmware:

```sh
GRKIT_CLANG=clang GRKIT_LLD=ld.lld cargo test -p grkit-build --locked synthetic_ -- --ignored
GRKIT_TEST_FIRMWARE="$PWD/firmware/fwdc239b.bin" GRKIT_CLANG=clang GRKIT_LLD=ld.lld \
  cargo test -p grkit-build --locked pinned_firmware_build_is_reproducible -- --ignored
```

The GR IV relocation test builds from the pinned official 1.11 container. With
the complete local February 18, 2026 GR IV HDF RTOS capture from issue #9, it also
checks relocation against independently verified addresses in that build:

```sh
GRKIT_TEST_FIRMWARE_GR4="$PWD/firmware/fwdc248b.bin" \
  GRKIT_TEST_RTOS_GR4_111_20260218="$PWD/firmware/gr4-hdf-20260218-rtos.bin" \
  GRKIT_CLANG=clang GRKIT_LLD=ld.lld \
  cargo test -p grkit-build --locked gr4_relocation_resolves_other_builds -- --ignored
```

The capture must be 20,791,616 bytes with SHA-256
`60250bf7c497af13d6579fc58736cfddbffa32ffb4f8ff3a1403854b9da40d5e`.
It is test input, not a firmware update. Keep it out of Git. The regression
checks address resolution; it does not establish camera operation of a release.

For the web site, see [web/README.md](web/README.md).

## Rules

- Keep filesystem and process access out of `grkit-core`.
- Put target addresses in the target definition or its native adapter. Check the hook bytes before you patch them.
- Do not edit `runtime/abi.h`, `runtime/generated/`, or `web/src/generated/`. Change the layout tables in `crates/grkit-core/src/abi.rs` or the plugin registry in `crates/grkit-core/src/plugins.rs`. Then run `GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated`.
- To add a plugin, follow [Add a plugin](docs/architecture.md#add-a-plugin).
- Keep firmware details in the core. A plugin under `runtime/plugins/<id>/` includes only `runtime/plugin.h`. If a plugin needs a firmware function, add a core service to `runtime/plugin.h`.
- Give each new hook an extension point in `components::point_of`. A hook without a point fails the build.
- Keep processing records valid while camera tasks use them.
- Do not change persistent native camera settings.
- Add focused tests for new behavior. Keep GRF bytes unchanged in compatible refactors.
- Host tests and builds do not verify device behavior. State in the pull request what you tested on a camera.
- Changes to cache synchronization or camera task lifetimes need a camera test.
- Use conventional commits.
- Add a change file in `.changeset/` for each change that a user can see. [docs/releasing.md](docs/releasing.md#record-a-change) describes the format.
- Keep firmware, build outputs, archives, and local paths out of Git.
- Keep third-party notices. Review each new effect input and its license.
- Change the English, Simplified Chinese, Traditional Chinese, and Japanese guides in `resources/` together. Keep their headings and steps in the same order. A test checks the camera phrase that the GR IIIx guide replaces. The guides use headings, paragraphs, lists, images, strong text, code spans, and links only, because `grkit guides` converts them to Typst. To check the PDFs, run `.github/scripts/build-guides.sh`.

## License of contributions

- A code contribution is licensed under the MIT License.
- An effect contribution uses the license that its source requires. A fit of a spektrafilm LUT uses CC BY-SA 4.0. A fit of a Spectral Film LUT uses the MIT License.
- For a new effect source, add its attribution and license to `effects/NOTICE.md`. Add its license text and its changes to `effects/sources/`.
- Do not contribute an effect if its source license does not permit redistribution of derived data.

Maintainers release GRKit as [docs/releasing.md](docs/releasing.md) describes.
