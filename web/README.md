# GRKit site

The site lets a user select a camera, the modules, and the effects, preview the effects, and download `GRKit.zip`. The browser builds the package with `grkit-wasm`. No server code runs.

The site serves these files:

| Path | Source |
|---|---|
| `index.html`, `assets/`, `_headers`, `THIRD_PARTY_LICENSES.txt` | `pnpm build` in this directory |
| `runtime/gr3-2.10/`, `runtime/gr4-1.11/` | `grkit site` |
| `effects/gr3-2.10/`, `effects/gr4-1.11/` | `grkit effects publish` |

`THIRD_PARTY_LICENSES.txt` gives the licenses of the bundled JavaScript packages and fonts, and of the Rust crates in the WebAssembly module. `pnpm build` writes it with `scripts/third-party-licenses.mjs`. The script reads the JavaScript licenses from the Vite build and the crate licenses from `cargo metadata`, so Cargo must be on `PATH`.

The site serves its fonts and scripts itself and sends no request to another host.

## Develop

The tools are Node.js 24, pnpm, the `wasm32-unknown-unknown` Rust target, and wasm-bindgen 0.2.128. The wasm-bindgen version must equal the version that `Cargo.toml` pins.

1. Install the dependencies: `pnpm install`.
2. Build the browser bindings: `pnpm wasm`. If `wasm-bindgen` is not on PATH, set `WASM_BINDGEN` to its path.
3. Make the site data from the repository root. The commands need the official firmware and the ARM toolchain. See [CONTRIBUTING.md](../CONTRIBUTING.md).

   ```sh
   grkit build --target gr3-2.10 --firmware fwdc239b.bin --out dist/runtime-local
   grkit site --build dist/runtime-local --out dist/site-data
   grkit effects publish --source effects --firmware fwdc239b.bin --out dist/site-data/effects/gr3-2.10
   sh .github/scripts/install-guide-tools.sh dist/guide-tools
   sh .github/scripts/build-guides.sh grkit dist/guide-tools dist/site-data/guides
   ```

   For GR IV, also build `gr4-1.11` from `fwdc248b.bin`, add it with `grkit site`, and publish its effects with `--target gr4-1.11`. Without them, only the GR III and GR IIIx packages work.

4. Start the development server: `pnpm dev`. The server reads the site data from `../dist/site-data`. To use another directory, set `GRKIT_SITE_DATA`.

To add Plausible analytics to a build, set `PLAUSIBLE_HOST` to the Plausible origin, for example `https://plausible.example.com`. The build then adds the script to `index.html` and allows it in the Content-Security-Policy of `_headers`. Release reads the repository variable `PLAUSIBLE_HOST`.

The site data must come from the current source. If the runtime templates change, the browser rejects an older runtime with "Runtime scripts differ from the production templates".

## Check

```sh
pnpm typecheck
pnpm test
pnpm build
```

## Preview

The page opens with a GR IIIx sample photo. A user can also select a DNG from any camera. A Worker decodes it with LibRaw into linear sRGB in the browser, and a second Worker applies the preview tables of `grkit effects publish`. Photos are not uploaded to a server.

The effects are fitted to the GR III, and LibRaw's color processing differs from the camera's. The previews are therefore approximations.

LibRaw is distributed under CDDL-1.0. `THIRD_PARTY_LICENSES.txt` includes its notices and a link to its source.

## Default sample

The default image is Snow in Akihabara by nagi usano, a GR IIIx JPEG under CC BY 2.0. `public/samples/akihabara.json` records its source, license, and hashes. The site includes the attribution in `THIRD_PARTY_LICENSES.txt`, and the preview links to the source.

`public/samples/akihabara.grlinear` holds linear pixels that `scripts/build-sample.mjs` estimates from the JPEG. To make it again:

1. Download the JPEG from the source URL in `public/samples/akihabara.json` to a directory outside the repository.
2. Get `effects/gr3-2.10/preview/standard.lut` from the site data, for example from the site archive of a release.
3. From `web/`, run `node scripts/build-sample.mjs /path/to/Snow_in_Akihabara_2022-01-06.jpg /path/to/standard.lut`. The script checks the SHA-256 of the JPEG first.
4. Run `pnpm test`. The sample test checks the hash, the attribution, the dimensions, and the pixel format.
