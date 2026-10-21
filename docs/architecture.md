# Architecture

GRKit adds features to Ricoh GR cameras without changing the firmware on the camera.
A package on the SD card installs private code into RAM at each power-on, and hooks connect that code to the running RTOS.
This document describes how the parts fit together. The source code and its comments hold the addresses, formats, and limits.

## Overview

```text
 developer computer / browser            camera
 ────────────────────────────            ──────────────────────────────────────────
 grkit build    ──► runtime build        Linux:  loader script ──► INSTALL
 grkit effects  ──► GRF effects                     │  checks firmware, writes code,
 grkit package  ──► GRKit.zip  ── SD card ──►       │  records, icons, and hooks
 site (wasm)    ──► GRKit.zip                       ▼
                                         RTOS:   hooks ──► menu, Image Control,
                                                 Framing, picture encode
```

There are three execution domains:

- Rust tools run on the developer computer or, through WebAssembly, in the browser.
- The installer runs in the camera's Linux, started by an SD script.
- The hooks and plugins run in the camera's RTOS.

| Path | Contents |
| --- | --- |
| `crates/grkit-core` | Formats, effect compilation, artwork, runtime templates, package assembly. No file or process access. |
| `crates/grkit-build` | Firmware checks, patch generation, Clang and LLD invocation. |
| `crates/grkit-cli` | The `grkit` command and its filesystem rules. |
| `crates/grkit-wasm` | Package assembly in the browser, with the same checks as the CLI. |
| `runtime/` | Native C and assembly: installer, hooks, menu host, plugins. |
| `resources/` | Installation guides (Markdown, compiled to PDF by the release). |
| `effects/` | Built-in effect definitions. See [effects/README.md](../effects/README.md). |
| `web/` | The static site: package builder and effect preview. See [web/README.md](../web/README.md). |

The CLI embeds `runtime/` and `resources/`, so it works outside the checkout.

## Targets

A target is one firmware build. Each target has its own addresses in `runtime/targets/<target>.h` and its own regions and hooks in `components::Layout`.

| Target | Cameras |
| --- | --- |
| `gr3-2.10` | GR III 2.10 and 2.20, GR IIIx 1.60 and 1.70, and their HDF models |
| `gr4-1.11` | GR IV 1.11, GR IV HDF, GR IV Monochrome |

GR IIIx 1.60 runs the RTOS code of GR III 2.10, so both cameras use one runtime. They differ only in the ModeSet file that enables SD scripts.
GR III 2.20 and GR IIIx 1.70 change only the end screen. Their code moves by less than the relocation radius, so INSTALL [relocates](#relocation) the `gr3-2.10` runtime to them. `camera.rs` lists the releases of each camera and the runtime build of each release.
An HDF camera runs the firmware of its base camera and uses the package of that camera.
GR IV has its own layout and all plugins, but Framing offers only XPan (see [Framing](#framing)).

## Installation

At power-on, the loader script waits until Linux mounts the card. Then it runs two steps:

1. SCAN selects the effects and writes an allocation plan. The startup script allocates the arrays and stays resident, so the arrays stay allocated.
2. INSTALL writes records and icons, publishes the code, and installs the hooks last. It writes `INSTALL.OK`, or `INSTALL.ERR` with a status.

INSTALL changes nothing on a firmware that it does not expect. Each firmware address must resolve in the running image (see [Relocation](#relocation)), and each hook site must hold its stock instruction.

Each hook belongs to one extension point. INSTALL installs only the points of the selected plugins, so a plugin that is not selected changes no firmware.
`SCRIPT/PLUGINS/PLUGINS.TXT` selects the plugins.

`grkit_core::abi` defines the shared header, the effect records, and the picture scratch. It generates `runtime/abi.h`.

The resident loader saves the plugin selections and the Fn adjustments in `SCRIPT/PLUGINS/STATE.TXT`. It writes only after a change, and only through the camera's own file system.
A script error frees the arrays that the hooks use, so the resident script checks each array index and divides only by constants.

## Plugins

The core is in `runtime/`: hooks, installer, menu host, and the hosts of the extension points. Each feature is a plugin in `runtime/plugins/<id>/`, linked statically into the core.
A plugin includes only `runtime/plugin.h`, and a test checks that no plugin names a firmware symbol or address.

| Plugin | Extension points |
| --- | --- |
| Image Control | `menu.page`, `tuning`, `picture.encode` |
| Framing | `menu.page`, `viewfinder.overlay`, `capture.aspect` |
| Guides | `menu.page`, `viewfinder.overlay` |
| Date Stamp | `menu.page`, `picture.encode` |
| Grain | `menu.page`, `pipeline.grain` |

A point with several plugins calls their handlers in registry order.

`grkit_core::plugins::PLUGINS` is the only list of plugins. Its order is the menu order.
The generated files (`runtime/generated/*.h`, `web/src/generated/plugins.ts`) come from the registry, and a test fails if one differs.

### Add a plugin

1. Add the C code in `runtime/plugins/<id>/`: `page.h`, and a handler file if the plugin has layers or encoders. Include only `runtime/plugin.h`.
2. In `page.h`, define `<ID>_PAGE`, with `<ID>_TILES` as its tile count.
3. Add the entry to `PLUGINS` in `crates/grkit-core/src/plugins.rs`. Declare its header words in `words`. A saved key refers to one of them with `Slot::Own`.
4. Add its tiles to `grkit_core::artwork`, its menu text to `grkit_core::menu_text`, and its site text to `web/src/i18n.ts`.
5. Run `GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated`.
6. Run the tests. They fail if a tile count, a handler order, or an extension point does not agree with the code.

## Menu

GRKit adds a private entry to the ADJ toolbar, drawn like the native entries. The root lists the installed plugins, then About. Each plugin page owns its rows.
The private menu reuses the native ADJ value list, so it behaves like a native entry.

The menu text follows the camera language. It is data, not code, because the menu region is small.

## Image Control

Image Control selects an effect for still pictures and the live view by changing a private active-record pointer. Off keeps the native processing.

An effect (GRFX0001) replaces the Standard color matrix and tone curves. An optional extension adds a post matrix and selective color.

Each effect record has its own Fn adjustments. Fn opens the native parameter page, and hooks redirect its model to the record, so native settings and effect settings stay separate.
Photos record the effect and its adjustments in the maker note, so playback shows them.

## Framing

Framing draws a mask and a ratio badge into the viewfinder overlay. Guides draws its lines in the same path, inside the photo area of the selected ratio.

On GR III, Framing captures through hidden capture aspects, while the native aspect setting stays 3:2. DNG files keep the full sensor data and describe the crop.
GR IV has a live-view mode for each native aspect, so Framing offers only Off and XPan there.

## Recorded pictures

A hook on the JPEG encode processes each picture plane before encoding: the main picture, the screennail, and the thumbnail. It applies selective color, then the date stamp.
Each still build stores its effect record with its shot settings, so continuous shooting keeps the effect of each shot.
Grain sets the native grain stage of recorded pictures. The live view and DNG files do not have grain.

## Relocation

Some cameras show a supported version but run another build of it. Each runtime is built from one published image, the pinned build. INSTALL finds each firmware address in the running image and moves each reference to it.

- Each firmware address in the source is a symbol (`FIRMWARE(address)`). `grkit_build::relocation` turns the link relocations into tables.
- For each address, the build stores a window of the pinned image around it that is unique.
- INSTALL (`runtime/relocation.h`) searches each window in the running image. Each window must match once.

A failure stops INSTALL with status 47 before it changes the firmware. `INSTALL.ERR` then lists the addresses that failed. `runtime/relocation.h` gives the reason codes.

## Code areas

All private code is in the reservation after the RTOS image. GRKit changes no byte of the image except the hook sites and a few in-place patches.
`build.json` gives the address, size, and limit of each code component. The builder rejects overflow, code outside the reservation, changed hook instructions, and overlapping patches.

## Testing

- Rust tests pin the GRF bytes, the artwork, the generated headers, and the loader scripts.
- Host C tests check the GRF parser, the effect catalog, the menu, Framing, and the picture code.
- `--ignored` tests build the runtime with an ARM toolchain and a local firmware. See [CONTRIBUTING.md](../CONTRIBUTING.md).
