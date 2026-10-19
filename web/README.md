# GRKit site

The site lets a user select a camera, the modules, and the effects, preview the effects, and download `GRKit.zip`. The browser builds the package with `grkit-wasm`. No server code runs.

The site serves these files:

| Path | Source |
|---|---|
| `index.html`, `assets/`, `_headers`, `THIRD_PARTY_LICENSES.txt` | `pnpm build` in this directory |
| `runtime/gr3-2.10/`, `runtime/gr4-1.11/` | `grkit site` |
| `effects/gr3-2.10/`, `effects/gr4-1.11/` | `grkit effects publish` |

`THIRD_PARTY_LICENSES.txt` gives the licenses of the bundled JavaScript packages and fonts, and of the Rust crates in the WebAssembly module. `pnpm build` writes it with `scripts/third-party-licenses.mjs`. The script reads the JavaScript licenses from the Vite build and the crate licenses from `cargo metadata`, so Cargo must be on `PATH`.

The site serves its fonts from `assets/` and sends no request to another host. The Noto Sans SC and TC styles load only on a Chinese page, and the Noto Sans JP styles only on a Japanese page.

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

## RAW preview

The page opens with a GR IIIx sample photo. Uploads accept a DNG from any camera, including converted DNG files, with up to 120 megapixels. Other formats are not supported.
The effects are fitted to the GR III, so the preview of another camera's photo is an approximation.

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
A second Worker applies the tables and encodes the JPEG previews, so a preview does not block scrolling or input.

These previews are approximations. The model's exposure calibration is assumed, and LibRaw's color processing differs from the camera's processing.
The 16-bit input clips values outside 0..1. It does not retain scene highlights above that range.

## Default sample

The default image is Snow in Akihabara by nagi usano, a GR IIIx JPEG released under CC BY 2.0. The README screenshot shows the same photo.
`public/samples/akihabara.json` records its source URL, license, source SHA-256, processing settings, dimensions, and output SHA-256.
The site also includes this attribution in `THIRD_PARTY_LICENSES.txt`, and the preview links to the source.

The preview applies tables to linear pixels, but a JPEG holds developed pixels. `scripts/build-sample.mjs` therefore estimates the linear pixels: it inverts the Standard development of `preview/standard.lut` for each pixel. The baseline of the sample then matches the JPEG, and each effect starts from the same pixels as on a RAW photo. The JPEG keeps the look that its author gave it, so the sample shows the effects on that look.

The checked-in `public/samples/akihabara.grlinear` contains gzip-compressed linear pixels, so visitors do not download or convert the full JPEG.
After decompression, the format is eight ASCII bytes `GRLIN001`, little-endian u32 width and height, then little-endian u16 RGB samples.
The custom extension prevents static servers from treating the inner gzip container as an HTTP content encoding.
The sample builder uses the same resizing function as the browser decoder.

To reproduce the sample:

1. Download the JPEG from the source URL in `public/samples/akihabara.json` to a directory outside the repository.
2. Get `preview/standard.lut` of `gr3-2.10` from the site data, for example `effects/gr3-2.10/preview/standard.lut` in `site.tar.gz` of a release.
3. Run `node scripts/build-sample.mjs /path/to/Snow_in_Akihabara_2022-01-06.jpg /path/to/standard.lut` from `web/`. The script checks the source SHA-256 before it decodes the JPEG.
4. Run `pnpm test`. The sample test checks its hash, attribution, dimensions, and pixel format.

The RAW decoder is loaded only for uploads. Its WASM and JavaScript files are served from the site.
LibRaw is distributed under CDDL-1.0. The site license file includes its notices and a link to the source and build instructions.
