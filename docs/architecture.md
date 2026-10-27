# Architecture

GRKit adds features to Ricoh GR cameras without changing the firmware on the camera.
A package on the SD card installs private code into RAM at each power-on, and hooks connect that code to the running RTOS.
This document describes how the parts fit together. The source code and its comments hold the addresses, formats, and limits.

## Overview

```text
 developer computer / browser            camera
 ────────────────────────────            ──────────────────────────────────────────
 grkit build    ──► runtime build        Linux:  loader script ──► INSTALL
 grkit module   ──► GRM modules                     │  checks firmware, links modules,
 grkit effects  ──► GRF effects                     │  writes records, icons, hooks
 grkit package  ──► GRKit.zip  ── SD card ──►       ▼
 site (wasm)    ──► GRKit.zip                RTOS:   hooks ──► core ──► modules
```

The core is per target. It owns the firmware hooks, the state of each capability, and the hot paths. A module is the same file on each camera. It gives data, such as settings, pages, text, and tiles, and callbacks that the core calls. A module reaches the firmware only through services and capabilities of the core.

There are three execution domains:

- Rust tools run on the developer computer or, through WebAssembly, in the browser.
- The installer runs in the camera's Linux, started by an SD script.
- The hooks, the core, and the modules run in the camera's RTOS.

| Path | Contents |
| --- | --- |
| `crates/grkit-core` | Formats, effect compilation, artwork, runtime templates, package assembly. No file or process access. |
| `crates/grkit-build` | Firmware checks, patch generation, Clang and LLD invocation. |
| `crates/grkit-cli` | The `grkit` command and its filesystem rules. |
| `crates/grkit-wasm` | Package assembly in the browser, with the same checks as the CLI. |
| `runtime/` | Native C and assembly: the installer, the hooks, and the core. `runtime/sdk/` is the module SDK. |
| `modules/` | The official modules. |
| `resources/` | Installation guides (Markdown, compiled to PDF by the release). |
| `effects/` | Built-in effect definitions. See [effects/README.md](../effects/README.md). |
| `web/` | The static site: package builder and effect preview. See [web/README.md](../web/README.md). |

The CLI embeds `runtime/` and `resources/`, so it works outside the checkout.

## Targets

A target is one firmware build. Its module in `crates/grkit-build/src/target/` gives the address and the stock instruction of each hook site, and `runtime/targets/<target>.h` gives its firmware addresses.

The targets of one camera line form a family, such as the GR IV builds. A family lists its hooks once, with the extension point of each, and shares the runtime code and the header `runtime/targets/<family>.h`. Tests require each build of a family to have each hook and each header definition of the family. A change for one build therefore fails until it reaches each build.

Each build has each feature of the runtime. If a build lacks a header definition that a feature needs, the build fails. The runtime does not turn off a feature for a missing definition. A conditional selects only between the variants of the families, such as where a family keeps the camera language.

| Target | Cameras |
| --- | --- |
| `gr3-2.10` | GR III 2.10 and 2.20, GR IIIx 1.60 and 1.70, and their HDF models |
| `gr4-1.11` | GR IV 1.11, GR IV HDF, GR IV Monochrome |
| `gr4-1.20` | GR IV 1.20 and GR IV HDF 1.20, GR IV Monochrome 1.10 |
| `gr4-1.20-0528` | GR IV 1.20, the factory build of GR IV 30th Anniversary Edition cameras |

GR IIIx 1.60 runs the RTOS code of GR III 2.10. GR III 2.20 and GR IIIx 1.70 move their code by less than the relocation radius, so INSTALL [relocates](#relocation) the `gr3-2.10` runtime to them. GR IV 1.20 changed the code around several hooks, so it has its own target. An HDF camera runs the firmware of its base camera. `camera.rs` lists the releases of each camera and the runtime build of each release.

## Installation

At power-on, the loader script waits until Linux runs commands, which is about 2 s before the camera mounts the card on Linux. It does not wait for that mount. The loader allocates a small mailbox array, and then runs three steps on Linux:

1. `LOAD.SH` copies the card into RAM. Unless Linux has mounted a card folder, the loader mounts the card itself, and the script copies the package folder and the data folder into a folder in RAM, unmounts the card, and runs SCAN there. The camera's own mount of the card fails while another mount holds it, and an unmount after that failure stops Linux. So the window with the mount is short, and if the camera tried its mount during the window, the script moves its mount to the camera's path instead of an unmount.
2. SCAN lists the modules and the effects, writes an allocation plan, and writes the plan header into the mailbox. The loader allocates the arrays and stays resident, so the arrays stay allocated. The plan goes to `GRKIT/PLAN.BIN` on the card with a key: the listing of the effect and module folders, the package list, the modes, and the saved values that the plan depends on. A later start with the same key keeps that plan and reads no effect file, which saves the copy of each effect file through the card file system.
3. INSTALL links the modules, writes the effect records and the icons, restores the saved state, and installs the hooks last. It writes its result into the mailbox, and `INSTALL.OK`, or `INSTALL.ERR` with a status, and `GRKIT/REPORT.TXT` into the RAM copy. `FINISH.SH` copies them to the card after the camera mounts it, and imports the new effects of the package into the effects library on the card.

Each installer takes the card root as its argument: the RAM copy, or the card for the import. When a camera has several targets, its package holds the installer of each, and `INSTALL.SH` selects one by the identity of the running build, or by [relocation](#relocation).

INSTALL changes nothing on a firmware that it does not expect. Each firmware address must resolve in the running image, and each hook site must hold its stock instruction. Each hook belongs to one extension point, and INSTALL installs only the points that the loaded modules need.

### Card layout

A package holds only `SCRIPT`. `GRKIT` at the card root is the data folder: no package holds it, so a copy of a new package, by replace or by merge, keeps it.

| Path | Contents |
| --- | --- |
| `SCRIPT/GRKIT` | the loader, the installers, and the package list `PACKAGE.TXT` |
| `SCRIPT/GRKIT/MODULES` | the modules of the package, `<ID>.GRM` |
| `SCRIPT/GRKIT/EFFECTS` | the effects of the package, `.GRF` files |
| `GRKIT/EFFECTS` | the effects library |
| `GRKIT/MODULES` | the modules that a person adds |
| `GRKIT/STATE.TXT` | the saved settings |
| `GRKIT/REPORT.TXT` | the result of the last boot |
| `GRKIT/SAFE`, `GRKIT/PROFILE` | files that start the safe mode or the development profile; each can also end in `.TXT` |

A package module loads only if `PACKAGE.TXT` names it, so a merge copy gives the same result as a replace copy. A module in `GRKIT/MODULES` replaces the package module with the same name.

When a package is new, SCAN copies its effects into the library. `GRKIT/IMPORTED.TXT` records the package that the library has, so an effect that a person deleted stays deleted until a package brings a different file. At the first boot after GRKit 1.2, SCAN also copies the effects of `SCRIPT/PLUGINS/EFFECTS`. GRKit installs at most 20 effects.

The installer never renames or deletes a file in the data folder. On the camera's Linux, a rename does not replace a file. On GR IV exFAT, a delete makes later lookups fail.

### Report and safe mode

INSTALL writes `GRKIT/REPORT.TXT` when its text changes. It gives the GRKit version and target, the result of each module, and the installed effects.

While `GRKIT/SAFE` exists, no module loads. INSTALL then applies only the menu patches for module management, so the GRKit section of MENU can turn off the module that stopped the camera. The safe mode does not start by itself, because a count of unfinished boots also counts each quick power-off.

### Development profile

While `GRKIT/PROFILE` exists, the core counts the calls and the CPU cycles of its hook points and of each call of module code (`runtime/profile.h`). It reads the PMU cycle counter, which the firmware lets each task read and does not use. The counter is per core, so a call that moved to another core is not counted. Every 10 seconds, the resident loader appends the counts to `GRKIT/PROFILE.LOG` and clears them. Without the file, each point reads one header word, and the loader polls as usual.

### State

`grkit_core::abi` defines the shared header, the effect records, and the tail. It generates `runtime/abi.h`.

The resident loader saves the capabilities, the saved module settings, and the Fn adjustments in `GRKIT/STATE.TXT`. The core counts each change in one header word, which the loader polls. The loader writes only after the count settles, and only through the camera's own file system. INSTALL restores the file before it installs the hooks, so the first frame has the saved values. Without the file, INSTALL reads the state file of GRKit 1.2 and maps its keys (`runtime::LEGACY_STATE_KEYS`).

A script error frees the arrays that the hooks use, so the resident script checks each array index and divides only by constants.

## Modules

Each feature is a module, `<ID>.GRM`. The same file runs on each camera, because a module calls the firmware only through the core. A test checks that no module source names a firmware symbol or address.

| Module | Contents |
| --- | --- |
| `FILMLOOK` | a page of `CORE.TONE` |
| `FRAMING` | a page of `CORE.ASPECT`, the mask, and the ratio label |
| `GUIDES` | its style, a page, and the guides |
| `DATE` | its style and look, a page with a native settings page, and the date |
| `GRAIN` | a page of `CORE.GRAIN` with a row of `CORE.GRAIN_SIZE` |

At power-on, INSTALL links each module (`runtime/module_link.h`). It copies the code after the tail, in an area that no firmware code uses, and the data into a script array. It relocates the module, fills each import with a core service, and registers the page, the layers, and the plane filters. A module that does not link is skipped, and the others load.

The core keeps the value of each module setting (`runtime/settings.h`). The capabilities are settings of the core, `CORE.<KEY>`. The core keeps a capability in a header word, and its hooks read that word, so module code never runs in a hot path. INSTALL applies the patches of a capability only when a loaded module uses it.

| Capability | Values |
| --- | --- |
| `CORE.TONE` | the effect (see [Film Look](#film-look)) |
| `CORE.ASPECT` | the aspect of still pictures (see [Framing](#framing)) |
| `CORE.GRAIN` | the grain level of recorded pictures |
| `CORE.GRAIN_SIZE` | the grain size of recorded pictures |

The core calls module code at these points:

- A page gives the rows of a setting in ADJ and in MENU.
- A layer paints on the live view, in the order of the layers.
- A plane filter changes each recorded picture, after the color of the effect.
- Events, timers, and UI callbacks run in the App task (`runtime/services.h`, `runtime/ui.h`). The timer and UI services refuse a call from another task.

[Writing a module](modules.md) is the guide for module authors, and [the module reference](module-reference.md) gives the keys, services, limits, and error numbers.

## Menu

GRKit adds an entry to the ADJ toolbar. It lists the module pages and reuses the native ADJ value list, so it behaves like a native entry. OK on a row makes a quick change, and Fn opens the native settings page of the module.

### GRKit section of MENU

The native MENU has a section titled GRKit at the end of the Customize tab. At the first controller change after INSTALL, the core copies the Customize tab descriptor and adds the section. The section has:

- A row for each module file, also for a module that did not load (`runtime/manage.h`). It opens **Enable**, then the settings of the module page. The native settings pages that the rows open are borrowed from the camera (`runtime/menu_settings.h`). The tab has room for a fixed number of rows. Each build header gives the rows of its stock tab, and the build fails if the rows of the most modules do not fit. On a build with more stock rows, such as a relocated build, the last modules get no row, and ADJ Order and About GRKit stay.
- **ADJ Order**, which moves a module page in ADJ.
- **About GRKit**, which shows the version and the project address on the native camera information page.

The rows of the section follow the default order: the `order` of each module, then its ID. ADJ Order changes only the order in ADJ. The choices are settings, so the loader saves them, and they apply at once. Off stops a module: its page leaves ADJ, the core calls none of its code, and the capabilities that only it uses count as unused. A module that is Off at a start loads stopped, so On also applies at once.

A page can have options (`options = true`). Its **Options** row turns each value of the page on or off in ADJ, or each effect for a page of `CORE.TONE` (`runtime/effects.h`). SCAN installs the effects that are on first. The options page shows native checkboxes. Each native list shares one SelectMenuController with radio rows, so the controller factory gives the options page a second one, built with checkbox rows (`runtime/menu_settings.h`).

The menu text follows the camera language. It is data in the tail, not code, because the menu region is small.

## Film Look

The capability `CORE.TONE` selects an effect for still pictures and the live view through a private active-record pointer (`runtime/tone.h`). Off keeps the native processing.

An effect (GRF) replaces the Standard color matrix and tone curves. An optional extension adds a post matrix and selective color. An effect can also drive the camera's hue-sector stage (`grkit_core::hue`), for which the installer writes a parameter set and a second set for highlight correction.

The live-view highlight warning compares the shown luma with a threshold of the display. With an effect, the threshold is the luma of the effect's white, less 1, because the curves of a color effect end below the native threshold (`runtime/framing.c`).

Each effect has its own Fn adjustments. Fn opens the native parameter page, and hooks redirect its model to the effect record, so native settings and effect settings stay separate. Photos record the effect and its adjustments in the maker note.

## Framing

The capability `CORE.ASPECT` holds the aspect of still pictures (`runtime/aspect.h`). Framing lists the same ratios on each camera. The core crops each ratio through a hidden capture aspect, also a ratio that the camera has natively, so the live view stays 3:2 for each ratio and DNG files keep the full sensor data. A native aspect other than 3:2 from the camera menu pauses the crop. The camera saves a crop as a value, so a release can add ratios.

The live-view labels are rendered at build time in the style of the native live-view text (`grkit_core::artwork::overlay_labels`). After INSTALL, the first still live view shows a short "GRKit" label.

## Recorded pictures

A hook on the JPEG encode processes the main picture, the screennail, and the thumbnail. It applies the selective color of the effect, then the plane filters of the modules. Each still build stores its effect with its shot settings, so continuous shooting keeps the effect of each shot. It also stores the posture of the electronic level. The planes keep the landscape layout of the sensor, and a plane filter gets the quarter turns that show the picture upright, as the EXIF `Orientation` of the photo does. `CORE.GRAIN` sets the native grain stage, so the live view and DNG files have no grain. `CORE.GRAIN_SIZE` sets the grain size: the native sizes on GR IV, and a blur of the grain noise on GR III.

## Relocation

Some cameras show a supported version but run another build of it. Each runtime is built from one pinned image. For each firmware address (`FIRMWARE(address)`), the build stores a window of the pinned image that is unique. INSTALL (`runtime/relocation.h`) finds each window once in the running image and moves each reference. A camera reads the image uncached, so the search takes seconds. INSTALL therefore checks each window at a known address first: its pinned address, or the address that an earlier search of the same build found, which INSTALL keeps in `GRKIT/RELOC.BIN`. Only a build without such addresses gets the search, which also checks that each window is unique. A failure stops INSTALL with status 47 before it changes the firmware, and `INSTALL.ERR` lists the addresses that failed. SCAN resolves the same addresses to find the mailbox.

## Porting

If the code of a new firmware version moves by less than the relocation radius, the existing target runs on it: add the release to `camera.rs`. Otherwise, the version needs a new target in the family of the existing one.

`grkit port` finds each address of an existing target in another build, and writes drafts of the build header and of the target module. It cannot find an address whose code changed, and it does not check values that are not addresses, such as object offsets and text IDs.

1. Run `grkit port --target <target> --firmware <its firmware> --to <new firmware> --out <directory>`. Use `--to-rtos` for an RTOS image, such as a GRDUMP capture.
2. If the output gives `"relocatable": true`, add the release to the existing target instead, and test it on the new firmware.
3. Make `runtime/targets/<new target>.h` from `header.h`, and `crates/grkit-build/src/target/<module>.rs` from `target.rs`. Resolve each `PORT` comment, and check each value that is not an address.
4. Add the target to `crates/grkit-build/src/target.rs`, to `runtime/firmware.h`, and to `camera.rs` as experimental. If the code at a hook site changed, change the hook of the family so that it fits each build.
5. Run `GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated`.
6. Add the firmware zip to the private firmware backup (`docs/releasing.md`).
7. Run `grkit port` again with `--check <new target>`, then the tests. Mark the release as not experimental only after a test on a camera.

## Code areas

All private code is in the reservation after the RTOS image. GRKit changes no byte of the image except the hook sites and a few in-place patches. `build.json` gives the address, size, and limit of each code component, and the builder rejects an overflow, a changed hook instruction, and overlapping patches.

## Testing

- Rust tests pin the GRF bytes, the artwork, the generated headers, and the loader scripts.
- Host C tests compile parts of the runtime with a host compiler. They cover the GRF parser, the state parser, the module linker, the menu, Framing, and the picture code of each family.
- `grkit module test` runs the host test of a module against a fake core (`runtime/sdk/grkit_test.h`).
- `--ignored` tests build the runtime with an ARM toolchain, from a local firmware or from a pseudo-random image. See [CONTRIBUTING.md](../CONTRIBUTING.md).
