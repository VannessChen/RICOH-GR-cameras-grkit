# GRKit site

The site lets a user select a camera, the modules, and the effects, preview the effects, and download `GRKit.zip`. The browser builds the package with `grkit-wasm`. No server code runs.

The site serves these files:

| Path | Source |
|---|---|
| `index.html`, `assets/`, `_headers`, `THIRD_PARTY_LICENSES.txt` | `pnpm build` in this directory |
| `runtime/gr3-2.10/`, `runtime/gr4-1.11/` | `grkit site` |
| `effects/gr3-2.10/`, `effects/gr4-1.11/` | `grkit effects publish` |

`THIRD_PARTY_LICENSES.txt` gives the licenses of the bundled JavaScript packages and fonts, and of the Rust crates in the WebAssembly module. `pnpm build` writes it with `scripts/third-party-licenses.mjs`. The script reads the JavaScript licenses from the Vite build and the crate licenses from `cargo metadata`, so Cargo must be on `PATH`.

The site serves its fonts from `assets/` and sends no request to another host.

## Develop

The tools are Node.js 24, pnpm, the `wasm32-unknown-unknown` Rust target, and wasm-bindgen 0.2.128. The wasm-bindgen version must equal the version that `Cargo.toml` pins.

1. Install the dependencies: `pnpm install`.
2. Build the browser bindings: `pnpm wasm`. If `wasm-bindgen` is not on PATH, set `WASM_BINDGEN` to its path.
3. Make the site data from the repository root. The commands need the official firmware and the ARM toolchain. See [CONTRIBUTING.md](../CONTRIBUTING.md).

   ```sh
   grkit build --target gr3-2.10 --firmware fwdc239b.bin --out dist/runtime-local
   grkit site --build dist/runtime-local --out dist/site-data
   grkit effects publish --source effects --firmware fwdc239b.bin --out dist/site-data/effects/gr3-2.10
   ```

   For GR IV, also build `gr4-1.11` from `fwdc248b.bin`, add it with `grkit site`, and publish its effects with `--target gr4-1.11`. Without them, only the GR III and GR IIIx packages work.

4. Start the development server: `pnpm dev`. The server reads the site data from `../dist/site-data`. To use another directory, set `GRKIT_SITE_DATA`.

The site data must come from the current source. If the runtime templates change, the browser rejects an older runtime with "Runtime scripts differ from the production templates".

## Check

```sh
pnpm typecheck
pnpm test
pnpm build
```

## RAW preview

The page opens with a GR III sample photo. Uploads accept original GR III or GR IIIx DNG files up to 64 MiB.
JPEG, PNG, other camera models, and converted DNG files are outside this preview's supported input.
The decoder checks DNG metadata and rejects known conversion software. It does not authenticate the camera that made a file.

A dedicated Worker decodes each upload with `@colorhythm/libraw-wasm` 1.1.1 (LibRaw 0.22.1).
It reads sensor data, uses the recorded white balance, and produces half-size 16-bit linear sRGB.
Automatic brightness and content-dependent white-level adjustment are disabled.
The decoder applies orientation. The preview then applies the DNG default crop and resizes in linear light to a maximum edge of 1200 pixels.

The worker terminates after completion, failure, cancellation, or a 60-second timeout.
Selecting another photo or the default sample cancels the pending upload. Photos are not uploaded to a server.

The `grkit-site-effects-v1` catalog describes linear-input LUTs with an sRGB coordinate shaper.
The shaper allocates more grid points to shadows. It does not convert the input into a Standard JPEG.
`preview/standard.lut` renders the baseline, and each effect table renders the same decoded pixels with the GRKit fit model.
Both sides use the GR III model, including when the selected package targets GR IV.

These previews are approximations. The model's exposure calibration is assumed, and LibRaw's color processing differs from the camera's processing.
The 16-bit input clips values outside 0..1. It does not retain scene highlights above that range.

## Default sample

The default image comes from raw.pixls.us record 3115, a GR III DNG released under CC0-1.0.
`public/samples/gr-iii.json` records its source URL, license, source SHA-256, decoder settings, dimensions, and output SHA-256.
The site also includes this attribution in `THIRD_PARTY_LICENSES.txt`.

The checked-in `public/samples/gr-iii.grlinear` contains gzip-compressed linear pixels, so visitors do not download or decode the full DNG.
After decompression, the format is eight ASCII bytes `GRLIN001`, little-endian u32 width and height, then little-endian u16 RGB samples.
The custom extension prevents static servers from treating the inner gzip container as an HTTP content encoding.
The browser decoder and the sample builder use the same decoding, cropping, and resizing functions.

To reproduce the sample:

1. Download the DNG from the source URL in `public/samples/gr-iii.json` to a directory outside the repository.
2. Run `node scripts/build-sample.mjs /path/to/gr-iii.dng` from `web/`. The script checks the source SHA-256 before decoding it.
3. Run `pnpm test`. The sample test checks its hash, attribution, dimensions, and pixel format.

The RAW decoder is loaded only for uploads. Its WASM and JavaScript files are served from the site.
LibRaw is distributed under CDDL-1.0. The site license file includes its notices and a link to the source and build instructions.
