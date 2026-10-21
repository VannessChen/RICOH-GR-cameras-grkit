# Architecture

GRKit adds features to Ricoh GR cameras without changing the firmware on the camera.
A package on the SD card installs private code into RAM at each power-on, and hooks connect that code to the running RTOS.
This document describes how the parts fit together. The source code and its comments hold the exact addresses and offsets.

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
| `web/` | The static site: package builder and effect preview. |

The CLI embeds `runtime/` and `resources/`, so it works outside the checkout.

## Targets

A target is one firmware build. Each target has its own addresses in `runtime/targets/<target>.h` and its own regions and hooks in `components::Layout`.
The build defines `GRKIT_TARGET_<target>`, and the compilation stops for an unknown target.

| Target | Cameras | ModeSet file |
| --- | --- | --- |
| `gr3-2.10` | GR III, GR III HDF | `00078350.588` |
| `gr3-2.10` | GR IIIx 1.60, GR IIIx HDF | `00078490.609` |
| `gr4-1.11` | GR IV, GR IV HDF | `00078560.636` |

GR IIIx 1.60 runs the RTOS code of GR III 2.10. None of the bytes that differ is a patch site, so both cameras use one runtime. The cameras differ only in the ModeSet file that enables SD scripts.
An HDF camera runs the firmware of its base camera and uses the package of that camera.

GR IV has its own layout. It has all plugins, but Framing offers only XPan (see [Framing](#framing)).
Some GR IV cameras show version 1.11 but run an unpublished build of it (issue #9). The installer relocates the private code to the running build (see [Relocation](#relocation)).

`grkit scripts-off` and the site give a separate ZIP with the ModeSet file and the command `[SCRIPT_EN_MODE DIS]`, which turns SD scripts off. It holds no GRKit code.

## Installation

At power-on, the loader script polls until Linux mounts the card (about 5 s). Then it runs two steps:

1. SCAN selects the effects and writes an allocation plan. The startup script allocates the arrays and stays resident, so the arrays stay allocated.
2. INSTALL checks the allocations, writes records and icons, publishes the code, and installs the hooks last. It writes `INSTALL.OK`, or `INSTALL.ERR` with a decimal status.

INSTALL changes nothing on a firmware that it does not expect:

- Each firmware address must resolve in the running image (status 47, see [Relocation](#relocation)).
- Firmware guards (`components::Layout::guards`) must show no data after the image end (status 45).
- Each hook site must hold its stock instruction (status 16).

Each hook belongs to one extension point (`components::point_of`). INSTALL installs only the points of the selected plugins, so a plugin that is not selected changes no firmware.
`SCRIPT/PLUGINS/PLUGINS.TXT` selects the plugins. A missing file selects all plugins, and an empty file selects none.

An invalid effect file does not stop the other effects. INSTALL keeps at most 20 effects, the ones with the smallest IDs. This is a product limit, not a format limit.
On exFAT cards, the camera returns an empty name for a file name longer than 12 characters, such as a macOS `._` file. The installer skips that entry (`runtime/directory.h`).

`grkit_core::abi` defines the shared header, the effect records, and the picture scratch. It generates `runtime/abi.h`, and a test fails if the checked-in file differs.

### Saved selections

The resident loader restores and saves the selections of each plugin and the Fn adjustments of each effect in `SCRIPT/PLUGINS/STATE.TXT`.
Each entry is two decimal lines: a key ID (CRC-32 of the key name modulo 10^9) and the value. The plugin registry gives the keys.

The camera's file API cannot truncate or replace a file, so the loader writes the state in three steps:

1. Delete `STATE.TMP`, then write the state to it.
2. Delete `STATE.TXT`.
3. Rename `STATE.TMP` to `STATE.TXT`.

The loader writes only after a change, and only through the camera's own file system.
A script error frees the arrays that the hooks use, so the resident script checks each array index and divides only by constants. Tests check both rules.

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

A point with several plugins calls their handlers in registry order. The viewfinder draws the Framing mask, the guides, then the Framing badge. The encoder applies selective color, then the date stamp.

### Registry

`grkit_core::plugins::PLUGINS` is the only list of plugins. Its order is the menu order.
Each entry gives the ID, the extension points, the header words, the saved keys, the artwork tiles, and the handlers.
The generated files (`runtime/generated/*.h`, `web/src/generated/plugins.ts`) come from the registry, and the `generated` test fails if one differs.

### Add a plugin

1. Add the C code in `runtime/plugins/<id>/`: `page.h`, and a handler file if the plugin has layers or encoders. Include only `runtime/plugin.h`.
2. In `page.h`, define `<ID>_PAGE`, with `<ID>_TILES` as its tile count.
3. Add the entry to `PLUGINS` in `crates/grkit-core/src/plugins.rs`. Declare its header words in `words`. A saved key refers to one of them with `Slot::Own`.
4. Add its tiles to `grkit_core::artwork`, its menu text to `grkit_core::menu_text`, and its site text to `web/src/i18n.ts`.
5. Run `GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated`.
6. Run the tests. They fail if a tile count, a handler order, or an extension point does not agree with the code.

| Resource | Limit | Used |
| --- | --- | --- |
| Host icons in the first icon chunk | 40 | 20 |
| Plugin header words | 64 | 1 |
| Menu text | 8192 bytes | about 3,100 bytes |
| Plugins | 32 | 5 |
| Code regions | See [Code areas](#code-areas) | `build.json` gives each size |

## Menu

GRKit adds a private entry to the ADJ toolbar, drawn like the native entries. The root lists the installed plugins in registry order, then About.
Each plugin page owns its rows, labels, icons, and committed row. MENU on a page returns to the root, and MENU on the root closes ADJ.

GRKit opens on the row of the module that was open or highlighted when the last ADJ session ended. If ADJ closed while GRKit was open, the next ADJ opens on GRKit.
OK on a page row commits the row and closes ADJ. An ADJ press commits the row and closes ADJ too. Left and Right move to the next ADJ entry.
The private menu uses the native value list of ADJ, so a long list scrolls as a native list does.

The menu text is in English, Simplified Chinese, Traditional Chinese, and Japanese, and follows the camera language. `grkit_core::menu_text` builds the text table. The text is data, not code, because the menu region is small.

## Image Control

Image Control selects an effect for still pictures and the live view. Selection changes a private active-record pointer. Off keeps the native processing.
A highlighted row previews in the live view only. Still pictures always use the committed record.

An effect (GRFX0001) replaces the Standard color matrix and tone curves. An optional extension adds two features, each behind a feature bit:

- A post matrix multiplies the camera's YCbCr matrix, so it acts on gamma-encoded RGB.
- Selective color keeps the chroma of one hue range in recorded pictures. The live view shows that effect in monochrome.

### Fn adjustments

Each effect record has its own Fn adjustments: saturation, hue, key, contrast, highlight, shadow, sharpness, shading, and clarity.
Fn opens the native parameter page, and hooks redirect its model to the record, so native settings and effect settings stay separate.
Fn works on an effect row of the Image Control page and on the Image Control row of the root. The page then returns to where Fn was pressed.

### Photo metadata

Photos record the effect in the maker-note ImageTone tag as `0x8000` plus 15 bits of a hash of the effect ID.
The native code fills the adjustment tag (`0x247`) from the native settings. A hook writes the Fn adjustments of the effect into it.
Playback shows the effect icon and its adjustments.

## Site preview

The site previews effects on a sample photo or on a DNG from any camera. A browser Worker decodes the DNG with LibRaw into linear sRGB.
`grkit effects publish` writes a catalog with a 33 x 33 x 33 preview table for each effect and for Standard.
See [web/README.md](../web/README.md) for the input limits and the sample format.

## Framing

Framing draws a 50% black mask and a ratio badge into the viewfinder overlay, below the native status.
Guides draws its lines in the same path, inside the photo area of the selected ratio. The lines match the native grid guide: 2 pixels wide, black at 50% alpha, and the native thirds positions.

On GR III, Framing captures 4:3, 16:9, and 65:24 (XPan) through hidden capture aspects, while the native aspect setting stays 3:2. Patches keep these aspects on the 3:2 live-view and AF paths.
65:24 shares the 16:9 aspect, and hooks select its sizes. DNG files keep the full sensor data and describe the crop.

GR IV has a live-view mode for each native aspect, so Framing offers only Off and XPan there. In native 3:2, hooks keep the live view in 3:2 for XPan. In native 16:9, XPan crops the native picture.

## Recorded pictures

A hook on the JPEG encode processes each picture plane before encoding: the main picture, the screennail, and the thumbnail. It applies selective color, then the date stamp.

- **Shot ring**: each still build stores its effect record under its shot settings pointer. The maker-note tags and the encode of a shot read the record of their own build, also in continuous shooting.
- **Grain**: hooks set the native grain stage of recorded pictures to the selected level. The live view and DNG files do not have grain.
- **Date stamp**: renders seven-segment digits with an amber glow at the bottom right of the visible area. Smaller planes sample the mask of the main plane, so the stamp keeps its position.

## Relocation

Each runtime is built from one published image, the pinned build: GR III 2.10 or GR IV 1.11. INSTALL finds each firmware address of the runtime in the running image and moves each reference to it.
On the pinned build, each address stays unchanged.

- Each firmware address in the source is a symbol (`FIRMWARE(address)`). The link keeps each reference as a relocation, and `grkit_build::relocation` turns them into tables.
- For each address, the build stores a window of the pinned image around it, with the bits that move masked out. The window grows until it is unique within 64 KiB.
- INSTALL (`runtime/relocation.h`) searches each window in the running image. Each window must match once, and all sources of an address must agree.

A failure stops INSTALL with status 47 before it changes the firmware. `INSTALL.ERR` then has up to 8 lines. Each line gives four hexadecimal numbers: the reason, the index, the pinned address, and a count or a value.

| Reason | Meaning |
| --- | --- |
| 1 | The startup literals or the startup code differ |
| 2 | A window has no match, or a symbol has no source |
| 3 | A window has more than one match |
| 4 | A source word is not a B or a BL |
| 5 | The window of a function does not match at the branch target |
| 6 | A symbol lies outside its region |
| 7 | The sources of a symbol differ |

## Code areas

All private code is in the reservation after the RTOS image. GRKit changes no byte of the image except the hook sites and a few in-place patches.

| Component | Content |
| --- | --- |
| Processing | A32 assembly: the tuning hooks |
| Menu | Thumb-2 C at `-Oz` |
| Framing | A32 C at `-Os` and assembly: Framing, Guides, the effect extension, the picture hooks |

`build.json` gives the address, size, and limit of each component. The link has no division helpers, so menu C must not divide, and picture code uses its own division routine.
The builder rejects overflow, code outside the reservation, changed hook instructions, and overlapping patches.

## Testing

- Rust tests pin the GRF bytes, the artwork, the generated headers, and the loader scripts.
- Host C tests check the GRF parser, the effect catalog, the menu, Framing, and the picture code.
- The build rejects writable static data and limits the stack frame of each C function.
- `--ignored` tests build the runtime with an ARM toolchain and a local firmware. See [CONTRIBUTING.md](../CONTRIBUTING.md).
