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

The core is per target. It owns the firmware hooks, the state of each capability, and the hot paths. A module is the same file on each camera. It gives data, such as settings, pages, text, and tiles, and callbacks that the core calls, such as layers and plane filters. A module reaches the firmware only through services and capabilities of the core.

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
| `runtime/` | Native C and assembly: installer, hooks, and the core: the menu host and the hosts of the extension points. `runtime/sdk/` is the module SDK. |
| `modules/` | The official modules (`grkit_core::modules::OFFICIAL`). |
| `resources/` | Installation guides (Markdown, compiled to PDF by the release). |
| `effects/` | Built-in effect definitions. See [effects/README.md](../effects/README.md). |
| `web/` | The static site: package builder and effect preview. See [web/README.md](../web/README.md). |

The CLI embeds `runtime/` and `resources/`, so it works outside the checkout.

## Targets

A target is one firmware build. Its module in `crates/grkit-build/src/target/` holds its data, the private tail, and the site of each hook in its build.

The targets of one camera line form a family, such as the GR IV builds. They share the runtime code and the hooks. The family module in `crates/grkit-build/src/target/` lists each hook once: its symbol, the code component that holds it, its extension point, and whether its site gets a B or a BL. A build gives only the address and the stock instruction of each hook, so the builds of a family have the same hooks.
Each family has the header `runtime/targets/<family>.h`, which each build header of the family includes. It holds the object layouts, events, and constants of the family. It also holds the values that the shared code takes from each family, such as the icon slot and the Framing ratios. The shared code has no default for them, so a new family that does not define one does not build.
The build header `runtime/targets/<target>.h` holds the firmware addresses and the values that differ in that build.

Tests require each build of a family to have a site for each hook of the family and the same header definitions. A build declares the definitions that only it has in `BUILD_ONLY_DEFINITIONS`. A change for one build therefore fails the tests until it reaches each build of the family.

| Target | Cameras |
| --- | --- |
| `gr3-2.10` | GR III 2.10 and 2.20, GR IIIx 1.60 and 1.70, and their HDF models |
| `gr4-1.11` | GR IV 1.11, GR IV HDF, GR IV Monochrome |
| `gr4-1.20` | GR IV 1.20 and GR IV HDF 1.20, GR IV Monochrome 1.10 |
| `gr4-1.20-0528` | GR IV 1.20, the factory build of GR IV 30th Anniversary Edition cameras |

GR IIIx 1.60 runs the RTOS code of GR III 2.10, so both cameras use one runtime. They differ only in the ModeSet file that enables SD scripts.
GR III 2.20 and GR IIIx 1.70 change only the end screen. Their code moves by less than the relocation radius, so INSTALL [relocates](#relocation) the `gr3-2.10` runtime to them. `camera.rs` lists the releases of each camera and the runtime build of each release.
GR IV 1.20 changes the code around several hooks and adds the monochrome Image Control types 31 and 32. It therefore has its own target (see [Porting](#porting)). Its hooks are those of 1.11.
GR IV 30th Anniversary Edition cameras have an earlier 1.20 build from the factory, with code of both 1.11 and 1.20, so it has its own target. Ricoh does not publish it, so the target builds from a dump of a camera (`Source::Dump`).
An HDF camera runs the firmware of its base camera and uses the package of that camera.
GR IV has its own layout and all extension points.

## Installation

At power-on, the loader script waits until Linux mounts the card. Then it runs two steps:

1. SCAN lists the modules and the effects, and writes an allocation plan that holds them. The loader allocates the arrays and stays resident, so the arrays stay allocated.
2. INSTALL links the modules, writes the records of the planned effects and the icons, restores the saved state, and installs the hooks last. It writes `INSTALL.OK`, or `INSTALL.ERR` with a status, and then `GRKIT/REPORT.TXT`.

A camera has one package. When the camera has several targets, the package holds the installer of each, and `INSTALL.SH` selects one at each power-on: first by the identity word of the running build, then by [relocation](#relocation) for a build that no target names (`runtime::package_runtime_files`).

INSTALL changes nothing on a firmware that it does not expect. Each firmware address must resolve in the running image (see [Relocation](#relocation)), and each hook site must hold its stock instruction.

Each hook belongs to one extension point. INSTALL installs only the points that the loaded modules need, so a feature without its module changes no firmware.
### Card layout

A package holds only `SCRIPT`. `SCRIPT/STARTUP.TTL`, which the camera runs, includes the loader in the package folder `SCRIPT/GRKIT`. `GRKIT` at the card root is the data folder: no package holds it, so a copy of a new package, by replace or by merge, keeps it. INSTALL and the loader create it.

| Path | Contents |
| --- | --- |
| `SCRIPT/GRKIT/LOADER.TTL` | the loader script |
| `SCRIPT/GRKIT` | the installers, their shell scripts, the package list `PACKAGE.TXT`, and the files of the current boot |
| `SCRIPT/GRKIT/MODULES` | the modules of the package, `<ID>.GRM` |
| `SCRIPT/GRKIT/EFFECTS` | the inbox: the effects of the package, `<ID>.GRF` |
| `GRKIT/EFFECTS` | the effects library |
| `GRKIT/MODULES` | the modules that a person adds |
| `GRKIT/STATE.TXT` | the saved settings |
| `GRKIT/REPORT.TXT` | the result of the last boot |
| `GRKIT/IMPORTED.TXT` | the package that the library has |
| `GRKIT/SAFE` | an empty file that turns on the safe mode |

`PACKAGE.TXT` lists the effect and module files of the package, and the other package files have the same names in each release. A package module loads only if the list names it, so a merge copy that leaves an old module behind gives the same result as a replace copy. A module in `GRKIT/MODULES` replaces the package module with the same name.

When a package is new, SCAN copies its listed effects into the library, where each replaces the file with the same name. `GRKIT/IMPORTED.TXT` records the package that the library has, with a hash of each file. So a copy of the same package changes nothing, and an effect that a person deleted from the library stays deleted until a package with a different file brings it. The first boot without the library copies every valid effect of `SCRIPT/PLUGINS/EFFECTS`, where GRKit 1.2 kept its effects, then every valid inbox file. A listed effect that SCAN could not copy installs from the inbox. By default, the effects of the package come first, then the other effects of the library, each group in ID order.

GRKit lists at most 64 effect files and installs at most 40 of them (see [Options](#options)). `REPORT.TXT` gives the status of each listed effect. Without a module that uses `CORE.TONE`, no effect installs. The installer deletes and renames no file in these folders. On the camera's Linux, a rename does not replace a file. On GR IV exFAT, a delete made later lookups fail.

### Report and safe mode

At each boot, INSTALL writes `GRKIT/REPORT.TXT` when its text changes. The report gives the GRKit version and target, the result of each module, the installed effects, and the safe mode. It writes the file in place, because a rename does not replace a file.

In the safe mode, SCAN loads no module, so INSTALL applies only the patches of no extension point and the menu patches for module management. The safe mode starts while `GRKIT/SAFE` exists. It does not start by itself: a count of unfinished boots also counts each quick power-off, which is how many people use the camera.

### State

`grkit_core::abi` defines the shared header, the effect records, and the picture scratch. It generates `runtime/abi.h`.

The resident loader saves the values of the capabilities, the saved module settings, and the Fn adjustments in `GRKIT/STATE.TXT`. It writes only after a change, and only through the camera's own file system. It keeps a key with the value 0 out of the file.
INSTALL restores the file before it installs the hooks (`runtime/state.h`), so the first frame already has the saved values. It restores a capability only when a loaded module uses it, and a module setting only when its module loads. A missing `STATE.TXT` reads `STATE.TMP`, which a save that a power cut stopped leaves.
Without both, INSTALL reads the state file of GRKit 1.2 in `SCRIPT/PLUGINS` and maps its keys to the current keys (`runtime::LEGACY_STATE_KEYS`). The values and the Fn adjustment entries keep their meaning. The loader then saves at its first poll, so the next boot reads `GRKIT/STATE.TXT`.
A script error frees the arrays that the hooks use, so the resident script checks each array index and divides only by constants.

## Modules

Each feature is a module in its own file, `GRKIT/MODULES/<ID>.GRM`. The same file runs on each camera, because a module calls the firmware only through services that the core exports. A test checks that no module source names a firmware symbol or address.

| Module | Folder | Contents |
| --- | --- | --- |
| `FILMLOOK` | `modules/film-look` | a page of `CORE.TONE` |
| `FRAMING` | `modules/framing` | a page of `CORE.ASPECT`, the mask, and the ratio label |
| `GUIDES` | `modules/guides` | its style, a page, and the guides |
| `DATE` | `modules/date` | its style and look, a page with a native settings page, and the date |
| `GRAIN` | `modules/grain` | a page of `CORE.GRAIN` |

`grkit_core::modules::OFFICIAL` lists the official modules in their root order. The release builds each one once into `site/modules`, and the camera packages and the site add the selected ones.

A module folder has these files:

- `module.toml`: the ID, the sources, the tiles, the settings, the menu page, the layers, and the plane filters.
- `text/<language>.toml`: the page text in each menu language.
- C sources that include only `runtime/sdk/grkit.h`. A module with only data, such as Grain, has none.
- `test.c`: the host test of a module with C sources.

[Writing a module](modules.md) is the guide for module authors, and [the module reference](module-reference.md) gives the keys, services, limits, and error numbers. `grkit module new --id <ID> --out <folder>` writes a new module folder that builds and passes its test. `grkit module build --dir <folder>` makes the module file, and `grkit module test --dir <folder>` runs the host test. The build rejects a module that calls a function that the core does not export, such as a compiler helper for a division. `grkit package --module <file>` adds module files to a package.

At power-on, SCAN reserves a script array for the data of the modules, and INSTALL links each module (`runtime/module_link.h`):

1. INSTALL copies the code of each module after the tail. The RTOS maps that area read-only and executable, and no firmware code uses it.
2. INSTALL copies the data of each module into the array, after the settings table and the context of each module.
3. INSTALL moves each address word of the module. It fills each import with the address of a core service or with the context of the module.
4. INSTALL registers the menu page, the layers, and the plane filters of the module in the header, and copies its tiles after the effect icons.

A module that does not link is skipped, and the others load. By default, INSTALL loads the official modules in their order, then the others in the order of their IDs. [The GRKit section of MENU](#grkit-section-of-menu) changes the order and turns modules off.

The core keeps the value of each module setting (`runtime/settings.h`). INSTALL restores the saved settings from the state file. The resident loader saves them after the core counts a change. A setting of another module is available only if it is public.

The capabilities of the core are public settings too, `CORE.<KEY>` (`grkit_core::capabilities`). The core keeps the value of a capability in a header word, and its hooks read that word, so a module never runs in a hot path. INSTALL applies the patches of a capability only when a loaded module uses its setting. The loader saves the value under the key `CORE.<KEY>`. INSTALL restores it only when a loaded module uses the capability.

| Capability | Values |
| --- | --- |
| `CORE.TONE` | the effect (see Film Look) |
| `CORE.ASPECT` | the aspect of still pictures (see Framing) |
| `CORE.GRAIN` | the grain level of recorded pictures |

A module page is a choice page: one row for each value of a setting of the module or of a used setting, with the text and tiles of the module. The rows of `CORE.ASPECT` are the ratios of the camera, in the row order of the camera. The core gives the rows of a page of `CORE.TONE`: the effects. A page of an own setting can also give 2 to 5 rows of a native settings page. OK on the root row opens the page of the module for a quick change; Fn opens the native settings page, where the camera has one. Each row opens a borrowed native value page; the core borrows five of them on each camera. The native submenus have five rows or fewer, and the submenu of a settings page has six: four hooks make its constructor build six row views, hide the rows past the count up to the sixth, and give the sixth row the kind of the first. The submenu table entry has room for five rows, so OK on the sixth row changes to the kind of the first row, which no controller has. The controller change hook turns it into the value page of the first row, which then shows the sixth row (`SETTINGS_LAST_ROW`). Each module also has a row in the GRKit section of the native MENU (see [GRKit section of MENU](#grkit-section-of-menu)). A row targets a controller ID that does not exist, and the controller change hook turns it into the settings submenu of the module: Enable, then the rows of its page. A value page needs the submenu that opens it, so a page without settings rows gets a submenu with the row of the page setting. The Parameters row of a page of `CORE.TONE` opens the native parameter page with the committed effect, as Fn in ADJ does; the core first takes the model snapshot that a cancel restores, with a stand-in for the ADJ controller whose movie flag reads still.

A module layer paints the value of its setting, and the core skips it at 0. Module layers paint in their order. A layer can hide while DISP hides the native information, show only with the full display, or stay off the ADJ canvas. On an open module page, the ADJ canvas shows the highlighted value.

A plane filter changes the planes of each recorded picture with the value of its setting, and the core skips it at 0. Filters run in a fixed stage order, after the color of the effect: frames, then marks such as the date. The core exports `local_time` and `divide` for them.

### Services

The core gives modules events, timers, and a UI (`runtime/services.h`, `runtime/ui.h`). Their state is a block of the module array, and `GRKIT_HOST_SERVICES` holds its address.

- Events: INSTALL registers one subscriber for each module that has `[events]`. The core sends `SETTING_CHANGED` from `setting_store`, and `SYSTEM_READY` from the hook of the shooting screen at its first bind after INSTALL. Both run in the App task. An event that a handler causes is not sent again.
- Timers: each timer is a one-shot App timer of the firmware. The firmware calls the handler of the core in the App task, and the core calls the callback of the module.
- UI: the core shows the native message box on the ADJ canvas of the GRKit page. The firmware allows view calls only in the App task. The core counts its calls of module code in the App task (event handlers, timer callbacks, UI callbacks), and the timer and UI services refuse a call outside them. OK on a page row keeps ADJ open when the commit shows a box. A native screen opens as the parameter page does: the core leaves the GRKit page, changes the controller with ADJ as the screen to return to, and the return state opens the GRKit page again. The core tells the parent after it attaches a view, because a new view does not mark its parent dirty. Before it deletes a view, it hides each view of the subtree and purges it from the overlap lists, because other views keep raw pointers to it. The box shows the text ID of the borrowed settings page title, and the text hook gives the text of the module in the menu language. ADJ exit closes the box.

## Menu

GRKit adds a private entry to the ADJ toolbar, drawn like the native entries. The root lists the module pages that have rows, then Modules, then About. A page of `CORE.TONE` has rows only with effects.
The private menu reuses the native ADJ value list, so it behaves like a native entry.

### GRKit section of MENU

The native MENU has one place for GRKit: a section titled GRKit at the end of the Customize tab. At the first controller change after INSTALL, the core copies the Customize tab descriptor, adds the section and its rows, and points the RAM tab table at the copy. A MENU that was built before gives up its instance in the controller factory, so the next MENU builds its tabs again. The rows, in order:

- One row for each module file of the package and of `GRKIT/MODULES`, also a module that did not load (module management, `runtime/manage.h`). The row shows the value of the module page. It opens **Enable**, which turns the module on or off: the setting `CORE.OFF.<ID>`, 1 for Off; then the settings of the page. The row of a page of `CORE.TONE` opens Enable, Options, **Film Stock**, which lists Off and the shown effects in their ADJ order, then **Parameters**, the native parameter page of the committed effect. A module without a loaded page has only Enable.
- **ADJ Order**, one row for each group of six modules. Each row of its settings page is a module, in their order. Its values are the positions 1 to the module count, and a choice moves the module to that position: the setting `CORE.POS.<ID>`, the position + 1, or 0 for the default place. A move writes the position of each module, so the page shows the new order at once. The page of a module shows in the GRKit root of ADJ while the module is loaded.
- **About GRKit** shows the GRKit version and the project address on the native camera information page (`INFO_CONTROLLER`): an entry hook of its update (`menu_info_update`) sets its lines, and the text hook gives its title and labels, only after this row opens it. The row shows the version, safe mode, or that a change waits for the next start.

The choices are settings in the settings table, so the resident loader saves them as other settings. SCAN reads them from `STATE.TXT` and loads the modules in the order of their positions. A module that is Off loads too, stopped. The choices apply at once:

- The root of ADJ and the module rows of the section follow the positions.
- Off stops a module (`manage_suspend`). Its page leaves ADJ, the core calls none of its code (layers, plane filters, event handlers, timer and UI callbacks; `services_suspended`), and the capabilities that only it uses count as unused (`GRKIT_HOST_CAPABILITIES`): `CORE.TONE` gives the native processing, `CORE.ASPECT` no crop, and `CORE.GRAIN` no grain. Its settings keep their values. On starts it again.

Only On of a module that did not load, such as a module added after the start, waits for the next start, and the About GRKit row says to restart until then.

A text that does not fit the value column of a native row moves left, so that it ends at the end of the column (`fit_value`). The core measures it with the string view of the camera.

The settings table also keeps each saved value that no setting of this start takes, such as a setting of a module that is Off. The next save of the loader then keeps the value. The Fn adjustments of a missing effect are not kept.

### Options

A module page can have options (`GRKIT_CHOICE_OPTIONS`, `options = true` in `module.toml`). The settings page of the module in the GRKit section then has an **Options** row after Enable; without room, the first settings of the page leave it. The row opens a native value page that lists every option, with `(Off)` after an option that is off. Its title names the module, such as Framing Options. Its MENU guide reads Back; its OK guide reads Toggle in the Chinese and Japanese menus (切换, 切換, 切替), a word as wide as the native one. A value page of more values than the value slots, such as Film Stock, keeps the text of each value in the same entries as the options. OK turns the option on or off, and the page stays open: the controller change that follows is dropped (`MENU_ROWS_STAY`), the row is drawn again, and the page keeps its cursor on the option. A change applies at once:

- The options of a page of a setting are its values but Off. `CORE.ROWS.<ID>` keeps a bit for each value that is Off, at most 16 values. The page in ADJ and the value page in MENU leave those values out (`choice_rows` in `runtime/modules.h`). Off of the value of the setting sets it to Off.
- The options of a page of `CORE.TONE` are the listed effects (`runtime/effects.h`), at most 64 effect files, those with the smallest IDs, also an effect that did not install. One setting keeps the choice of each: `CORE.EFFECT.<effect ID>`, with Off in bit 0 and the position + 1, or 0 for the default place, above. Off of the committed effect commits Off (`effects_turn`).

SCAN orders the listed effects by their positions, and installs at most 40: those that are not Off, then those that are Off while there is room. Film Look shows Off, then the installed effects that are not Off, in that order (`runtime/tone.h`). On of an effect that did not install waits for the next start, and the About GRKit row says to restart until then. `REPORT.TXT` gives the status of each listed effect.

The menu hooks install when GRKIT/MODULES lists a module, also in safe mode. So the GRKit section can turn off the module that stopped the camera.

An ADJ that opened before INSTALL refreshes at its first event after INSTALL, so the GRK slot shows.

The menu text follows the camera language. It is data, not code, because the menu region is small.

## Film Look

The capability `CORE.TONE` selects an effect for still pictures and the live view by changing a private active-record pointer (`runtime/tone.h`). Off keeps the native processing. The Film Look module is its page.

An effect (GRFX0001) replaces the Standard color matrix and tone curves. An optional extension adds a post matrix and selective color.

An effect can also drive the camera's hue-sector stage, which changes colors by hue and by luminance in still pictures and the live view. The installer writes a parameter set for each effect with sectors, and hooks point the stage at it. Effects without sectors keep the native set.
Highlight correction halves the input of the stage. The installer therefore writes a second set for that case, and the hooks pick it by the camera's flag for the capture. `grkit_core::hue` describes the stage.

After the color stages, the camera reduces the chroma of very dark and very light areas. The fit model and the site previews include this reduction, so the previews show no more tint than the photos.

Each effect record has its own Fn adjustments. Fn opens the native parameter page, and hooks redirect its model to the record, so native settings and effect settings stay separate.
Photos record the effect and its adjustments in the maker note, so playback shows them.

## Framing

The capability `CORE.ASPECT` holds the aspect of still pictures (`runtime/aspect.h`). The Framing module is its page, a mask outside the photo area, and a ratio label. The overlay gives each layer the crop as its capture area, so the lines of Guides also draw inside the photo area of the selected ratio.

After INSTALL, the overlay core also shows a short "GRKit" label in the first still live view, so the user sees that GRKit has loaded. The label uses the overlay hooks, so it shows only when a module layer is installed.

The live-view labels, the load cue and the Framing ratios, are rendered in Roboto Condensed Light at build time, in the size and style of the native live-view text (`grkit_core::artwork::overlay_labels`). The installer copies them after the menu text, because the code region has no room for them.

Framing lists the same ratios on every camera. A ratio that the camera has natively sets the native aspect, so the native viewfinder, photo, and DNG apply it: 1:1 on GR III, and 16:9, 4:3, and 1:1 on GR IV. The page shows the ratio that the camera applies, so a change of the native aspect in the camera menu shows there too.
Framing crops the other ratios. A crop sets the native aspect to 3:2 and captures through a hidden capture aspect. The live view stays 3:2, and DNG files keep the full sensor data and describe the crop.
XPan and 2.35:1 share the 16:9 capture aspect with their own heights. 5:4 shares the 4:3 capture aspect with its own widths, which the pipeline crops from the center. Playback finds the geometry of a photo from its size.
GR IV has a live-view mode for each capture aspect, so its live-view hooks keep the 3:2 mode for a crop. In native 16:9, XPan and 2.35:1 also crop the native photos.
The camera saves a crop as a value, not as a menu row, so a release can add ratios without changing the meaning of a saved value.

## Recorded pictures

A hook on the JPEG encode processes each picture plane before encoding: the main picture, the screennail, and the thumbnail. It applies selective color of the effect, then the plane filters of the modules, such as the date of Date Imprint.
Each still build stores its effect record with its shot settings, so continuous shooting keeps the effect of each shot.
`CORE.GRAIN` sets the native grain stage of recorded pictures. The live view and DNG files do not have grain.

## Relocation

Some cameras show a supported version but run another build of it. Each runtime is built from one published image, the pinned build. INSTALL finds each firmware address in the running image and moves each reference to it.

- Each firmware address in the source is a symbol (`FIRMWARE(address)`). `grkit_build::relocation` turns the link relocations into tables.
- For each address, the build stores a window of the pinned image around it that is unique.
- INSTALL (`runtime/relocation.h`) searches each window in the running image. Each window must match once.

A failure stops INSTALL with status 47 before it changes the firmware. `INSTALL.ERR` then lists the addresses that failed. `runtime/relocation.h` gives the reason codes.

## Porting

A new firmware version needs one of two changes:

- If its code moves by less than the relocation radius, the existing target runs on it. Add the release to `camera.rs` with the existing target.
- If its code moves farther, or code at a hook changed, it needs a new target in the family of the existing one. The package of the camera then holds the installer of each target (see [Installation](#installation)).

`grkit port` finds each address of an existing target in another build. It uses the windows of [Relocation](#relocation), within a larger radius. It takes the hook and patch sites of the target module, the `FIRMWARE()` addresses of the build header, and the data addresses of `Target`. It writes drafts of the build header and of the target module for the new build.
The port cannot find an address whose code or data changed. It also does not check values that are not addresses, such as object offsets, text IDs, and enum values.

A data address that code loads gets that code as a second source, and the sources must agree. A port whose sources are all in windows of mostly addresses needs a review, because a table that grew can match at another start.
`"relocatable": true` means that the plan of the installer resolves each address of the target in the other build. A build has more symbols than the target names, so a test of the target on the new firmware must still confirm it.
The family header has no firmware addresses, because the port reads only the build header.

To add a target for a new firmware build:

1. Run `grkit port --target <target> --firmware <its firmware> --to <new firmware> --out <directory>`. Use `--to-rtos` for an RTOS image, such as a GRDUMP capture.
2. If the output gives `"relocatable": true`, add the release to the existing target instead. Test the existing target on the new firmware.
3. Make `runtime/targets/<new target>.h` from `header.h` of the output. Resolve each `PORT` comment: port the address by hand, or check the offset that an expression adds. Check each value that is not an address.
4. Save `target.rs` of the output as `crates/grkit-build/src/target/<module>.rs`. The module name is the target name with `_` for `-` and without `.`. Resolve each `PORT` comment. Give the name, the firmware, its SHA-256 values, its download, its identity word, and the header definition of the new target. Port each address that has no port, and check each value that is not an address. Add the module to `crates/grkit-build/src/target.rs`, to `TARGETS`, and to `SOURCES`.
5. If the code at a site changed, change the hook of the family so that it fits the code of each build. The runtime tests a definition that only the new build has with `#ifdef`. Add that definition to `BUILD_ONLY_DEFINITIONS`.
6. Select the header in `runtime/firmware.h`. Add the release to `camera.rs` as experimental.
7. Run `GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated`. The site packages each target of a camera, so `web/src/cameras.ts` changes only for a new camera.
8. Add the firmware zip, or the zip of the dump, to the private firmware backup (`docs/releasing.md`). The release workflow builds each target of `grkit release-plan`, so it needs no change.
9. Run `grkit port` again with `--check <new target>`. No address may differ without a review.
10. Run the tests, build the target from the official firmware, and test it on the new firmware. Set the release to not experimental only after a test on a camera.

If a step is missing, a test names it, except steps 8 and 10.

A firmware update that cannot be undone moves each camera to the new build, so the old build gets fewer users over time.
A maintainer can freeze such a build when keeping it current costs more than it gives. A frozen build still builds and gets fixes, but a new feature does not have to reach it.

To freeze a build, let `components::hooks` leave out the hooks that the build does not get. Exempt the build in the family test for the definitions that it does not get. The runtime must then leave the feature out on that build, as the settings page does without `SETTINGS_SUBMENU`.
The C code gates a feature on a header definition, and the assembly and INSTALL follow the hooks of the build. If a build has one of them without the other, the assembly stops the build.

## Code areas

All private code is in the reservation after the RTOS image. GRKit changes no byte of the image except the hook sites and a few in-place patches.
`build.json` gives the address, size, and limit of each code component. The builder rejects overflow, code outside the reservation, changed hook instructions, and overlapping patches.

## Testing

- Rust tests pin the GRF bytes, the artwork, the generated headers, and the loader scripts.
- Host C tests check the GRF parser, the effect catalog, the state file parser, the module linker, the menu with module management, the options, and the effects, Framing, and the picture code. The menu, Framing, and picture tests run for each target, with the expectations of its family where the behavior differs.
- `grkit module test` runs the host test of a module against a fake core (`runtime/sdk/grkit_test.h`).
- `--ignored` tests build the runtime with an ARM toolchain and a local firmware. See [CONTRIBUTING.md](../CONTRIBUTING.md).
- The synthetic `--ignored` tests need no firmware. They build a target of each family on a pseudo-random image, as the release builds them, and resolve the relocation tables of INSTALL against that image.
