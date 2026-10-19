# GRKit

English | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md)

[![Tests](https://github.com/ahonn/grkit/actions/workflows/tests.yml/badge.svg)](https://github.com/ahonn/grkit/actions/workflows/tests.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

GRKit adds film looks and shooting tools to RICOH GR cameras. It loads from the SD card and does not write the camera firmware.

![The GRKit page in the ADJ menu](https://github.com/ahonn/grkit/raw/main/resources/images/grkit-root.png)

Photo: [Snow in Akihabara](https://www.flickr.com/photos/cafetricot/51803682755/) by nagi usano, [CC BY 2.0](https://creativecommons.org/licenses/by/2.0/).

## Features

GRKit adds a sixth entry, GRK, to the ADJ menu. It has five modules:

- **Image Control**: 16 film looks, such as Portra 400, Velvia 100, and Tri-X 400. The viewfinder previews the look.
- **Framing**: XPan (65:24), 16:9, or 4:3 photos. The viewfinder dims the area outside the photo.
- **Guides**: thirds, center, golden ratio, or diagonal lines.
- **Date Stamp**: an orange date on the photo, like a film camera's data back.
- **Grain**: film grain on the photo, in three strengths.

The last row shows the GRKit version.

You select the modules when you make a package. The menu shows only those modules.

## Supported cameras

| Camera | Firmware | Status |
| --- | --- | --- |
| RICOH GR III | 2.10 | Tested on a camera |
| RICOH GR IIIx | 1.60 | Not tested on a camera |
| RICOH GR IV, GR IV HDF | 1.11 | Experimental, not tested on a camera. Framing offers only XPan. |

Other models and firmware versions are not supported. To see the firmware version, open the camera menu.

> [!WARNING]
> GRKit is experimental. Use it at your own risk.
> GRKit changes the camera memory only until the camera turns off. The installation turns on the camera's SD script setting, and this setting stays on.
> If GRKit fails, the camera can stop during startup. Read the [recovery guide](resources/RECOVERY.md) before you install GRKit.

## Install

To make a package, go to [grkit.app](https://grkit.app). Select the camera, the modules, and the effects, then download `GRKit.zip`.

Each package contains the installation guide for its camera as a PDF, in the language that you select. The recovery guide is at its end. The sources are:

- [GR III and GR IIIx](resources/INSTALLATION.md)
- [GR IV and GR IV HDF](resources/INSTALLATION-GR4.md)

To remove GRKit, turn the camera off and delete the `SCRIPT` folder from the card.

## Build from source

You need Rust 1.92 or later, Clang and LLD with ARM support, and the official firmware file from Ricoh. The repository does not contain firmware.

```sh
cargo install --path crates/grkit-cli --locked
grkit effects build --source effects --out build/effects
grkit build --target gr3-2.10 --firmware firmware/fwdc239b.bin --out build/runtime
sh .github/scripts/install-guide-tools.sh build/guide-tools
sh .github/scripts/build-guides.sh grkit build/guide-tools build/guides
grkit package --build build/runtime --effects build/effects --guide build/guides/gr3/en.pdf --out dist/GRKit.zip
grkit verify dist/GRKit.zip
```

- Guides: `install-guide-tools.sh` downloads Typst and the Noto Sans SC, TC, and JP fonts and checks their SHA-256, on Linux x86_64 or macOS arm64. `build-guides.sh` writes `build/guides/<camera>/<language>.pdf`.
- GR IIIx: package with `--camera gr3x` and `--guide build/guides/gr3x/en.pdf`. The runtime is the same as for GR III.
- GR IV: build with `--target gr4-1.11` from `fwdc248b.bin`, then package with `--camera gr4` and `--guide build/guides/gr4/en.pdf`.
- Fewer plugins: add `--plugins` with the plugins to install, for example `--plugins framing,guides`. The installer makes no firmware change for the other plugins. Without `--effects`, the package has no Image Control.
- Guide language: add `--language zh-Hans`, `--language zh-Hant`, or `--language ja`, and give the guide in that language, for example `build/guides/gr3/zh-Hans.pdf`.

## Repository layout

| Path | Contents |
| --- | --- |
| `crates/grkit-core` | Effect formats and package rules, with no platform I/O |
| `crates/grkit-build` | Runtime builds from the firmware file |
| `crates/grkit-cli` | The `grkit` command |
| `crates/grkit-wasm` | Package assembly in the browser |
| `runtime/` | The code that runs on the camera |
| `effects/` | The sources of the Image Control effects |
| `web/` | The site that makes packages in the browser |
| `resources/` | The installation and recovery guides |
| `docs/` | The architecture and the release process |

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) for the setup, the tests, and the pull request rules.
If you test GRKit on a camera, open a device report issue with the result, as a pass or a failure. Reports for GR IIIx and GR IV are the most useful.

## Documentation

- [Architecture](docs/architecture.md)
- [Release process](docs/releasing.md)
- [Effects](effects/README.md)
- [Web site](web/README.md)

## License

The code is licensed under the [MIT License](LICENSE). Each package contains `LICENSES.md`, which gives the licenses of its files.

- The effects are licensed under CC BY-SA 4.0 or MIT. [effects/NOTICE.md](effects/NOTICE.md) gives the license of each effect.
- The icon text uses the [Inter](https://github.com/rsms/inter) typeface under the [SIL Open Font License 1.1](crates/grkit-core/assets/Inter-LICENSE.txt).
- The effect fit is ported from [gr3-film-mods](https://github.com/idontknownihil/gr3-film-mods) under the [MIT License](crates/grkit-core/GR3_FILM_MODS_LICENSE.txt).

RICOH and GR are trademarks of Ricoh Company, Ltd. Film names are trademarks of their owners. GRKit is an independent project. It is not affiliated with, endorsed by, or supported by Ricoh or by a film manufacturer.
