# Architecture

## Domains and crates

GRKit has three execution domains. Rust tools run on the developer computer. The installer runs in camera Linux.
Menu, processing, and Framing hooks run in the camera's RTOS domain.

- `grkit-core` implements artwork, GRF formats, effect compilation, runtime templates, and deterministic ZIP assembly. It does not read files or start processes.
- `grkit-build` checks firmware, generates patches, and invokes Clang and LLD.
- `grkit-cli` exposes commands and enforces filesystem rules.
- `grkit-wasm` assembles packages in the browser. It takes a runtime build as `grkit build` writes it (`build.json` and `sd/`) and compiled effects, and it uses the `grkit-core` checks that the CLI uses.
- `web/` is the static site. It fetches the runtime, the effect catalog, the selected effects, and the installation guide PDF, and calls `grkit-wasm` to build `GRKit.zip` in the browser.
- `grkit_core::guide` converts the Markdown guides in `resources/` to Typst. The release compiles them to `guides/<camera>/<language>.pdf`. A package holds the guide of its camera in one language, with the recovery guide as an appendix.

The site preview accepts DNG files from any camera. A browser Worker uses LibRaw to decode sensor data into half-size 16-bit linear sRGB.
It uses the recorded white balance and disables automatic brightness and content-dependent white-level adjustment.
The preview applies orientation and the DNG default crop, then resizes in linear light to a maximum edge of 1200 pixels.
The default sample is a GR IIIx JPEG. Its linear pixels are estimated by inverting the Standard development of the preview, then resized with the same function and distributed as compressed linear pixels.

`grkit effects publish` writes a `grkit-site-effects-v1` catalog with 33 x 33 x 33 preview tables.
An sRGB coordinate shaper distributes the grid over linear input in 0..1. Each effect table applies the fit model directly to that input.
`preview/standard.lut` applies the Standard look matrix and base gamma to the same pixels for the comparison baseline.

The old JPEG inversion function remains available to Rust callers, but the site does not use it.
Both preview sides use the GR III model. Exposure calibration remains an assumption, and decoder color processing can differ from camera processing.
See [RAW preview](../web/README.md#raw-preview) for the input limits and sample format.

## Cameras

GR IIIx 1.60 runs the RTOS code of GR III 2.10. The two RTOS images have the same size and layout. They differ only in the model name, the version, the build time, the default year, the ModeSet numbers, and one byte of the model selection.
None of these bytes is a patch site or a firmware input of GRKit. A runtime built from either firmware is therefore byte-identical, and both cameras run the `gr3-2.10` runtime.

The cameras differ in the ModeSet file that enables SD scripts: `00078350.588` on GR III and `00078490.609` on GR IIIx. `grkit_core::camera` lists the cameras.
A package holds the ModeSet file of one camera, and its installation guide names that camera and file. `grkit verify` identifies the camera from that file.

Ricoh publishes the GR III 2.10 firmware also for GR III HDF, and GR IIIx 1.60 also for GR IIIx HDF. The RTOS selects the HDF model from a factory adjustment, not from the ModeSet numbers.
An HDF camera therefore uses the ModeSet file and the package of its base camera. `Camera::hdf` names the model, and the guides name both models. GRKit has not been tested on an HDF camera.

GR IV 1.11 is a different build, with the target `gr4-1.11`. GR IV and GR IV HDF run the same firmware and ModeSet file, `00078560.636`.
Its RTOS image is larger (0x13d2ac0 bytes), and its data, functions, and object layouts are at other addresses. `runtime/targets/<target>.h` holds these values for the C and assembly sources.

The build defines `GRKIT_TARGET_<target>` (`components::Layout::define`), and `runtime/firmware.h` selects the header from it. Without a known definition, the compilation stops.
`components::Layout` holds the regions and hooks of each target, and the build generates `hooks.h` from it.

The GR IV runtime has the menu, Image Control, Framing, Guides, Date Stamp, and Grain. GR IV Framing offers only XPan (see Framing).
GR IV processing hooks are in `runtime/targets/gr4-1.11-processing.S`, after the Framing region. GR IV reads the type tables through getters.
A private record therefore sets the Standard index (11) where all type paths join, and the record store keeps the private record.

The GR IV packet tail has no special type path. Its gamma object holds the built-curve flag at `+0x382c`. `target::ImageControl` holds the tables, the Standard record, and the settings offsets of each target.

GR IV photos record the plugin effect as on GR III. The hooks are the ImageTone call `0x53810804`, the DISP icon `0x531916c0`, and the detail `0x533a7614`.
GR IV extraction compares each native tag value, and its default case (`0x537d8a54`) keeps a non-native tag. GR IV also uses the native values `0x8100` to `0x8105`, so the installer skips codes `0x8100` to `0x81ff` there.
The effects use the GR III fit model.

Neither camera reads one fixed base gamma table. Each picture build computes the base from 15 ROM curves, the tone setting, and a gain. GR IV uses the GR III function (`0x5391dad4`) and the same curves.
`grkit effects publish` reads the curve for tone 0 and the lowest gain (GR IV `0x5407f4b4`). The gain of real GR IV stills is not known, so their shadows can differ from the fit.
The GR IV Standard look matrix differs from the GR III one.

GR IV icon descriptors have 12 bytes, and GR IV draws icons from a copy of the icon pixels. The installer reads the base of that copy from the icon drawer.

Native sources live under `runtime/`, and package documents live under `resources/`. The CLI embeds both, so it works outside the checkout.
Rust tests pin GRF bytes, artwork, generated headers, and loader scripts.

The build rejects writable static data in the private code. It also limits the stack frame of each C function to 256 bytes in the menu and 1536 bytes in the Framing component. Host C tests check the GRF parser, the effect catalog, the menu, and Framing on GR III, and the picture code on both targets. A syntax check compiles the C sources of each target.

## Loading

The loader script starts at power-on and polls. `lcmd` requests return NG until Linux mounts the card, about 5 s after power-on, so the loader repeats SCAN until the allocation plan appears.

SCAN selects the effect catalog and writes the plan. The startup script allocates the arrays and stays resident, so they remain allocated.

INSTALL checks the allocations, prepares records and icons, then publishes code before entry hooks.
It returns not-ready statuses while the camera initializes, and the loader repeats it. The result is `INSTALL.OK`, or `INSTALL.ERR` with the decimal status.
`O_EXCL` files serialize overlapping attempts. The loader deletes stale result and lock files at startup.

Before INSTALL writes anything, it compares the firmware guards (`components::Layout::guards`) with the running image. A difference stops it with status 45.
The guards are the startup literals of the `.data` copy at `0x53000450`: source, destination, and size. Source plus size is the end of the RTOS image.
On GR IV, the guards also include the icon catalog getter (`0x533ef604`).

A statically patched firmware that extends the RTOS image changes them. Its additions can lie in the private regions after the image, which INSTALL would otherwise overwrite without a check.
One such GR IV firmware extends the image from `0x543d2ac0` to `0x5440eac0`, over the Framing and processing regions.
That firmware also changes three GRKit hook sites, so the hook check (status 16) would stop it too. A patched firmware that changes no hook site passes that check, but not the guards.

The tail reservation itself has no content that INSTALL can check. On a GR III camera after power-on, it holds the noise of uninitialized DRAM. No word of it was zero, and most bytes were `0x55` or `0xd5`. After a restart, it can hold the code of an earlier GRKit session. The guards are therefore the check for a firmware that uses the tail.

An unusable effect file does not stop the other effects or Framing. Unusable files include names outside the `NAME.GRF` 8.3 rule, symbolic links, unreadable files, and invalid records.
The `._` companions that macOS writes are therefore ignored.

INSTALL also reads `SCRIPT/PLUGINS/PLUGINS.TXT`, which names the plugins to install (`runtime/selection.h`, `grkit_core::modules`). A missing file installs each plugin, and an empty file installs none.
An unknown or repeated name stops the installation with status 43, and a read error with status 44. A plugin that needs effects counts as not installed without effects. The installer stores the result in header `+56` (`GRKIT_HOST_PLUGINS`).

Each hook and in-place patch belongs to one extension point (`components::point_of`). Each plugin needs the points that the plugin registry gives (see [Plugins](#plugins)).

INSTALL checks and publishes only the patches of the points that the installed plugins need. A plugin that is not installed therefore changes no firmware, and an empty `PLUGINS.TXT` installs no hook. The code in the tail is always published, because nothing calls it without a hook. `build.json` gives the number of patches of each point.

With more than 20 effects, the installer keeps the 20 smallest effect IDs.
For a duplicate ID, the file with the smallest name wins. The result does not depend on directory order. Directory read errors stop the installation.

The 20-effect capacity is a product limit, not a GRF format limit. It fits one script array and one icon chunk.
The script array holds the header, the records, and the picture scratch.
The scratch has 141,376 bytes for the date stamp masks, then the 72-byte shot ring (see Recorded pictures).

Off, GRKit, and the eighteen module tiles (four Framing, five Guides, five Date Stamp, four Grain) use twenty host icon records outside that capacity. With 20 effects, they fill the 40-icon chunk.
The module tiles are contiguous from icon ID `0x7fec` to `0x7ffd`, below the GRKit tile (`0x7ffe`) and Off (`0x7fff`). The menu derives each one from the first Framing tile.

`grkit_core::abi` defines the header, the records, and the scratch. Each of the 64 header words has a name, and a test checks that each byte belongs to exactly one field.
`runtime/abi.h` is generated from these tables, and the `generated` test fails if the checked-in file differs. The C code, the assembly, and the loader take their offsets from these names.

### Saved selections

After `INSTALL.OK`, the resident loader restores and saves the effect, the Fn adjustments of each effect, the Framing ratio, the Guides style, the Date Stamp style, and the Grain level.
The private header is the loader's `registry` array, so the script reads and writes it directly.

`SCRIPT/PLUGINS/STATE.TXT` holds two decimal lines for each key: the key ID, then the value.
The key ID is the CRC-32 of the key name modulo 10^9 (`runtime::state_key_id`), so each number has at most 9 digits. The plugin registry gives each key, its header word, and its largest value (`plugins::state_keys`).
The Image Control key holds the ImageTone code of the effect, and 0 is Off. A file in the earlier single-integer format restores nothing.

An effect with Fn adjustments has an entry of its own. Its key ID is `990000000` plus the low 15 bits of the effect's ImageTone code (`runtime::PARAMETER_KEY_BASE`), a range that no plugin key uses.
Its value holds the nine stored bytes of `RECORD_PARAMETERS` as base-9 digits, the first adjustment lowest. Neutral adjustments have no entry.
The restore applies an entry to each installed effect with that code. The menu counts each adjustment change in header `+148`, and the loader writes the file when the count changes.

The loader checks the state every 2 s and writes the file only after a change. It writes through the camera's own file system, as photos do, never from Linux while the camera may write photos.
A missing, empty, or too large file restores nothing. A pair with an unknown key or an out-of-range value is skipped. An effect that is no longer installed stays Off, and its file entry stays until the next change.

The camera's `filecreate` does not truncate an existing file, and `filerename` cannot replace one. The loader writes the state in these steps:

1. Delete `STATE.TMP`, then write the state to it.
2. Delete `STATE.TXT`.
3. Rename `STATE.TMP` to `STATE.TXT`.

The loader checks each file with `filesearch`, because `filerename` gives result 0 for success and failure. If only `STATE.TMP` exists, the restore reads it. A plugin that is not installed restores Off, because no menu can turn it off.

A runtime error ends the script, and the interpreter then frees the arrays that the hooks use. The resident script therefore checks each computed array index, and it divides only by constants. A test checks both rules.

A read past the end of a file blocks the interpreter. A `filereadln` of a last line without a newline also blocks it. The restore therefore reads the file byte by byte, up to the size that `filestat` gives, and parses the digits itself. CR and LF end a line. A line with another character or with more than 9 digits is invalid, and its pair is skipped.

## Plugins

The core is `runtime/`: the hooks, the installer, the menu host, and the hosts of the extension points. The five features are plugins under `runtime/plugins/<id>/`.
A plugin includes only `runtime/plugin.h`. That file holds the page and handler types and the core services that a plugin can call. A test checks that no plugin file names a firmware symbol or an RTOS address.

The build links each plugin into the core statically. `runtime/plugin.h` is therefore a set of C declarations, not a table of function pointers, and the build checks each call.

| Plugin | Files | Extension points |
| --- | --- | --- |
| Image Control | `page.h`, `selective.c` | `menu.page`, `tuning`, `picture.encode` |
| Framing | `page.h`, `framing.c` | `menu.page`, `viewfinder.overlay`, `capture.aspect` |
| Guides | `page.h`, `guides.c` | `menu.page`, `viewfinder.overlay` |
| Date Stamp | `page.h`, `stamp.c` | `menu.page`, `picture.encode` |
| Grain | `page.h` | `menu.page`, `pipeline.grain` |

### Registry

The plugin registry is `grkit_core::plugins::PLUGINS`. It is the only list of the plugins. Its order is the menu order, and plugin n is bit n of `GRKIT_HOST_PLUGINS`. Each entry gives these facts:

- The ID: the name in `PLUGINS.TXT` and the directory under `runtime/plugins/`.
- The extension points whose patches the plugin needs.
- Whether the plugin shows only with effects.
- The state keys that `STATE.TXT` keeps, with their header words and largest values.
- The artwork tiles.
- The handler file, with its overlay layers or its encoders and their order.

The rest of the repository reads the registry:

| Reader | Facts |
| --- | --- |
| `grkit_core::modules` | `PLUGINS.TXT` names |
| `grkit_core::runtime` | The state keys of the loader, and the number of host icons |
| `grkit_build::loader` | The extension points of each plugin, and the artwork |
| `runtime/generated/plugins.h` | `PLUGIN_<ID>`, the names, the tile counts, and the largest state values |
| `runtime/generated/pages.h` | The page of each plugin, for `runtime/modules.h` |
| `runtime/generated/layers.h`, `encoders.h` | The handler files and the handler tables, for `runtime/framing.c` and `runtime/picture.c` |
| `web/src/generated/plugins.ts` | The site keys and the plugins of each camera |

The `generated` test of `grkit-core` fails if a generated file differs from the registry. The build crate stages each file under `runtime/`, so a new plugin file needs no list entry.

### Add a plugin

1. Add the C code in `runtime/plugins/<id>/`: `page.h`, and a handler file if the plugin has layers or encoders. Include only `runtime/plugin.h`.
2. In `page.h`, define `<ID>_PAGE`. Give it `<ID>_TILES` as its tile count.
3. Add the header words of its state to `crates/grkit-core/src/abi.rs`.
4. Add the entry to `PLUGINS` in `crates/grkit-core/src/plugins.rs`.
5. Add its tiles to `grkit_core::artwork`, its menu text to `grkit_core::menu_text`, and its site text to `web/src/i18n.ts`.
6. Run `GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated`.
7. Run the tests. The tests fail if a tile count, a handler order, or an extension point does not agree with the code.

The ADJ icon chunk has 40 slots, and the private header has 64 words. Each is full in this release, so a new plugin first needs room in both.

### Hosts

The hosts call plugins in two ways:

- A point with several plugins has a table of handlers in registry order. `runtime/framing.c` paints the overlay layers: the Framing mask, the guides, then the Framing badge. `runtime/picture.c` runs the encoders: selective color, then the date stamp. A host calls only the handlers of installed plugins (`PLUGIN_INSTALLED`).
- A point with one plugin is assembly in the core that reads the plugin's value from its header word. `pipeline.grain` reads the Grain level, and `capture.aspect` reads the Framing ratio. The `tuning` hooks read the active record of Image Control.

The Framing ratio is also the capture area of the other plugins. The overlay host gives the guides the photo area of that ratio. The encode host gives the date stamp the visible area of each plane.

## Menu

The ADJ toolbar shows five native entries and a private GRKit entry. All six cells use a 50-pixel pitch with a 40-pixel icon.
A private LineView draws the native translucent background before the shared selection frame. The private icon draws after the frame.

The ADJ controller owns both objects. Exit hides them and restores the native geometry. Destruction unlinks and releases them. Movie mode keeps the five-entry layout.

`runtime/modules.h` lists the page of each plugin in registry order (`runtime/generated/pages.h`). The core About row (`runtime/about.h`) follows the plugin rows.
Each page owns its title, summary, rows, labels, icons, selection checks, activation, and committed row. A page opens at its committed row.

A page returns its icons as tiles of its own artwork (`GRKIT_ICON_TILE`). The menu maps them to global IDs. The tiles of all plugins follow each other in registry order and end below the GRKit tile `0x7ffe`.

Two page callbacks are optional. `highlight` receives each change of the highlighted row, and Image Control previews the row in the live view. `parameters` makes Fn apply the row and open the native parameter page.

The root lists only the installed plugins (header `+56`), in registry order, then About. A plugin that needs effects, such as Image Control, shows only with effects.
If no plugin is installed, the installer installs no menu hook, so the GRKit entry does not appear. ADJ always opens at the root.
Module state stays in the private host header.

About is not a plugin, so `PLUGINS.TXT` does not select it. It uses the GRKit tile, and its summary gives the version, for example `Version 0.1.0`. It has no page: OK on it changes nothing.

MENU returns one level. From the GRKit root it returns to the native ADJ items. The private menu restores the native list on its supported exit paths.
List restoration suppresses the native selection callbacks.

### Menu text

The menu text is in English, Simplified Chinese, Traditional Chinese, and Japanese. The menu uses the camera language if it is one of them, and English otherwise. Effect names stay as their definitions give them.
`grkit_core::menu_text` holds the strings and builds a label table: for each language, 41 u16 offsets, then NUL-terminated UTF-16 strings. The string IDs are in `runtime/abi.h`.

The build gives the table the workspace version for the About row. Tests build it with the version `0.0.0`, so a new version does not change the pinned header hash.

The text is data, not menu code, because the menu region has little room. The script array has 8192 bytes of text after the picture scratch.
The installer copies the label table to the start of that area. At offset 4096, it writes the Image Control summary of each record in each language: the prefix from the table, then the effect name.

The menu finds the text from the record count, so the header needs no pointer. A summary slot comes from the record offset: `x * 2260 >> 16` equals `x / 29` for each record offset divided by 64, so no division helper is necessary.

## Image Control

Selection changes a private active-record pointer. Each processing task captures that pointer once. Off follows the native processing path.
Each record has its own Fn adjustments (`RECORD_PARAMETERS`): saturation, hue, high/low key, contrast, highlight contrast, shadow contrast, sharpness, shading, and clarity.
The parameter hook fills the native Standard adjustments of the processing packet from them, so the native Image Control settings do not change a record, and a record does not change them.

A preview word overrides the active record in the live-view context only. Still captures always use the committed record.
Highlighting a row previews it. OK commits it. MENU, other exits, and controller destruction clear the preview.

The live-view diff at `0x536dc860` sets the stage flags (`+4`..`+0x12` of the tuning object) from native inputs only.
When the live-view record or the adjustment count (header `+148`) changes, the gamma hook therefore rebuilds the curves and sets the color matrix flag (`+5`).
The matrix stage runs before the gamma hook, so the matrix changes one update later.
Without this, the color matrix keeps the previous effect until the white balance changes. The matrix carries the monochrome weights.

Native Image Control pages apply each highlighted value, which previews native processing while the private effect stays committed.
OK or an ADJ press on the native slot, and OK on the native MENU page, disable the private effect.
MENU on the ADJ slot keeps it, because MENU emits its switch event before the same confirm event as OK.

Fn on an effect row commits that row and opens the native parameter page (ImageControlDetailController) for the adjustments of the effect. Off has no Fn hint and ignores Fn.
Fn uses the native slot's call: event `0x1b` with argument 2, then `ChangeController(adj, 0x2000014, 0x2000000)`.
It first refreshes the model snapshot with `0x531a0150`, because a cancel restores the snapshot type persistently.
On return, header `+252` reopens GRKit Image Control at the committed row.

The page reads and writes the native settings through the Image Control model, so GRKit redirects the model while the page edits an effect (`components::PARAMETER_HOOKS`):

- Fn stores the record in header `+144` with bit 0 set. The page start (vtable slot 7) clears bit 0, which starts the session. A page that native ADJ opens clears the word.
- The model getters and setters of the nine adjustments read and write the record instead of the settings. Only the page calls them. The settings reads in the row texts of the page's list are redirected at their call sites, because other code also uses those reads.
- After the page reads its type and set kinds, it shows the Standard rows of the base set for any native type.
- The page keeps a snapshot of the adjustments in the model (`+3` plus the row kind), and a cancel writes the snapshot back through the setters. After the native snapshot load, the snapshot holds the record's values, so a cancel restores the record.
- The page event that opens another controller from the page does nothing during a session.

GR IV has one more row kind before shading, so shading and clarity are row kinds 14 and 15 there (13 and 14 on GR III).

Photos record the effect in the maker-note ImageTone tag (`0x4f`, hook `0x53608080`). Each record has a stable code at `+0x6f8`: `0x8000` plus 15 bits of an FNV-1a hash of the effect ID.
The tag hook reads the record of its own shot from the shot ring (see Recorded pictures). A shot without an entry keeps the native tag.
Native readers accept only 256 to 267, so in-camera RAW development falls back to Standard.

Playback extraction keeps a non-native tag in metadata key `0x25` (instruction `0x535c251c`). The DISP icon hook (`0x53179b7c`) shows the effect icon.
The detail hook (`0x532b7cfc`) shows GRKit photos as Standard. The still-viewfinder icon adapter substitutes the active effect icon outside movie mode.

## Framing

### Viewfinder

Framing draws black at 50% alpha into the still-viewfinder OSD bitmap. It draws after the canvas background clear and before the child views, so native status stays on top.
It checks the still mode, the native 3:2 aspect, the canvas owner, the dimensions, the pixel format, and the allocation first.
The OSD pixel buffer moves between boots (`0x5f83df80` and `0x619e1200` were seen), so the allocation must lie in `0x40000000`..`0x80000000`.

The ratio badge occupies `[204,46,66,22]`. A new session starts with the saved selections.
A still live view redraws its canvas only on a change. A selection restored at power-on has not drawn on the canvas yet. Such a selection invalidates the canvas (`0x533ad308`) on the next live-view update.

ADJ has its own canvas in slot zero (`controller+4`), so the live-view mask is not visible there.
While ADJ is open, Framing draws the committed ratio's mask on the ADJ canvas below the menu controls. The Framing page draws the highlighted ratio instead.

### Guides

Guides draws 1-pixel lines inside the photo area of the committed Framing ratio, in the same drawing path as the mask. Off uses the full 3:2 canvas.
The styles are thirds, a center cross with 20-pixel arms, the golden section (0.381966), and corner-to-corner diagonals.
Grid positions are fractions of the photo size in 1/65536, rounded. The diagonals are Bresenham lines, rounded half up. Neither needs division.

The line color is opaque gray (`0xffc0c0c0`), which has the same value with straight or premultiplied alpha. The format of the OSD bitmap is not confirmed.
The mask draws first and the badge last, so the badge covers the lines that cross it. Native child views, which may include the native grid guide, draw after both.

The Guides page previews its highlighted style on the ADJ canvas, inside the committed ratio. The Framing page previews its ratio with the committed style.
The preview style is in header `+48`, and it applies only while the Framing preview word is set.

### Capture

Capture uses the hidden GR III aspect ids. Id 1 is 4:3 (5328x4000). Id 2 is 16:9 (6000x3376) or 65:24 (6000x2216). The menu, icon, and badge name 65:24 XPan.
The native StillAspectRatio value stays 0 (3:2), so menus, AF areas, and the live view stay native.
A stored 1 or 2 would also make the stock firmware assert in AF code after a boot without GRKit.

The still aspect property handler (`0x535423f4`, property `0xd4`) maps a 3:2 request to the Framing id. Other native values pass unchanged.
A remote or shell request for 3:2 also records the Framing ratio.
A Framing menu change calls the native setter with the unchanged 3:2 value. Each live-view update does the same when the property's next copy differs from the Framing id.

GR III implements only live-view modes 0 (3:2) and 3 (1:1). Four in-place changes keep ids 1 and 2 on the 3:2 paths:

- The live-view mode table at `0x53d553c8` maps ids 1 and 2 to mode 0.
- The mode lookup at `0x534371d8` reads its table only for id 3. Without this change the live view times out after a capture.
- The pipeline row function at `0x536a3200` returns row 0 for ids 1 and 2 without an assertion.
- The AF and touch rectangle function at `0x535ee890` uses the 3:2 rectangle for ids 1 and 2.

65:24 shares id 2. Hooks in GetResolution (`0x536a2a30`) select the id 2 heights and the screennail and thumbnail insets.
Shooting follows the Framing ratio. In playback, the size classifier (`0x532b3988`) records whether the shown photo has a 65:24 size.
The image aspect getter (`0x532b3ca0`) reports id 2 for 65:24 sizes.

65:24 sizes are 6000x2216, 4800x1776, 3360x1240, and 1920x712. Their heights are multiples of 8. The maker note records 16:9 for them.

DNG files keep the full 6112x4064 sensor data, as native 1:1 does. DefaultCropOrigin and DefaultCropSize describe the crop.
In-camera Crop, Resize, Digital Filter, and RAW development do not know the 65:24 variant.

GR IV has a live-view mode for each capture aspect, so the GR III changes above would break native 4:3 and 16:9. GR IV Framing therefore offers Off and XPan.

In native 3:2, XPan works as on GR III. The still aspect property handler (`0x5374f264`, property `0xdf`) maps a 3:2 request to id 2.

Two hooks keep the live view in 3:2. They replace the capture aspect getter call in the live-view mode function (`0x5388c7b8`) and in the live-view mode check (`0x5363fa84`). Each call returns 0 for id 2 while the UI aspect (settings `+0x49f`) is 3:2.

The still pipeline uses the native 16:9 row for id 2 (`0x538d0894`). The AF area of id 2 is the 16:9 band of the 3:2 frame (`0x537f6aac`).
A Framing change and a live-view update resend the still aspect with `0x53284454`. With equal UI and applied aspects, it posts no UI event.

In native 16:9, id 2 is native, and XPan crops native 16:9 photos. Native 4:3 and 1:1 ignore XPan.

The GR IV sizes come from `ImageResolutionTranslatorKb636`. Its GetResolution (`0x538cfbfc`) has the id 2 height hooks, and slots 9 and 10 have the inset hooks.
The `Kb588` class holds the GR III sizes and is not used. The GR IV size classifier (`0x533a2820`) keeps the width in `r6` and the class pointer in `r5`.

GR IV 65:24 sizes are 6192x2288, 4944x1824, 3504x1296, and 1920x712. In native 3:2 and native 16:9 the XPan mask uses the GR III XPan area. In native 4:3 and 1:1 the viewfinder shows no GRKit drawing.
The date stamp follows the same rule. For an XPan capture in native 3:2 or native 16:9, it uses the XPan area of the screennail and the thumbnail.

## Effect extension

Bytes 84..127 of a GRFX0001 file hold an optional extension.
It holds u32 features at 84, an i16[9] signed Q13 post matrix at 88 (bit 1), and selective color parameters at 106 (bit 2). Bytes 120..127 are reserved and zero.
Each feature bit owns its field, and a field without its bit stays zero. A monochrome effect has no features.

An effect without features has zeros in the whole extension. Bit 4 is reserved, and the parsers reject it.

The installer copies the extension to record `+0x700`: features at `+0x700`, the post matrix at `+0x704`, and selective color at `+0x716`. Records are 0x740 bytes.

The processing region has little room left, so the extension hooks and the live-view diff hook link into the Framing region (`runtime/picture.*`).

### Post matrix

The YCbCrMatrix stage computes the hue rotation times BT.601 (`0x536e4720`), and its Make calls that at `0x536e4774`.
The hook multiplies the result by the effect's post matrix P: `CSP' = CSP x P`. P therefore acts on gamma-encoded RGB after the tone curves, and the Hue setting still rotates after it.

The coefficients are 9-bit two's complement with 256 = 1.0. The register writer packs 9 bits without saturation, so the hook sign-extends each one and saturates the products to -256..255.
The live-view diff also sets the YCbCr stage flag (`+7`) when the live-view record changes. Its stage runs before the gamma hook, so the post matrix changes one update later.

### Selective color

Selective color keeps the chroma of one hue range in recorded pictures and removes it elsewhere. The hue is the angle of (Cb - 128, Cr - 128).
The parameters are an i16 Q12 direction (cos and sin of the target hue), then u16 Q8 tangents and a u8 chroma gate.
Each side of the target has an inner and an outer half width. Inside the inner one a pair keeps all of its chroma, and outside the outer one none.
Between them, the weight falls linearly in tan(offset).

The side toward larger angles is separate, because skin lies only on that side of red. The gate fades out pairs whose chroma along the target is low.

The effect compiler converts degrees to these integers, so the runtime needs no trigonometry. The parsers require a direction of length 0.95 to 1.03, inner < outer <= 1024, and c0 < c1 <= 181.

For each picture, `picture_encode` builds a 128 x 128 weight table in the picture scratch with the shift-subtract division. One cell covers a 2 x 2 block of Cb and Cr values.
Then each pair costs one lookup and two multiplies: 22 A32 instructions in the loop.
`grkit_core::selective` computes the same table, and a test compares the two byte for byte. Site previews use it.

The live view shows the effect in monochrome. `picture_post` reads the tuning context byte at `settings + 0x82ac` through the Make's third argument (`settings + 0x120`): 1 is ImageTuningLiveview and 0 a still build.
For a live-view build of a selective effect it clears the Cb and Cr rows of the YCbCr matrix. The viewfinder then shows the luma that the removed hues keep in the picture, and the target hue is gray too.
Still builds keep the whole matrix, and their encode applies selective color. The encode time on a camera is not measured.

## Recorded pictures

`JpegEncodeMacroProcess::Execute` (`0x535047d4`) receives four image descriptors at configuration `+0x3c`.
They are the main picture, the 720 x 480 screennail, the 160 x 120 thumbnail, and a second full-size encode of the main planes.
Each descriptor holds Y at `+4`, CbCr at `+8`, and u16 width, height, and shared pitch at `+0x10`. The CbCr plane interleaves Cb and Cr for each pixel pair, in full rows (4:2:2) or half rows (4:2:0).

The entry hook processes each plane set once, before the encode is submitted.
It applies selective color first and then the date stamp, so the stamp keeps its color. It cleans and invalidates the data cache for the planes before and after (`0x5383c8a8`), because the image hardware writes and reads them.

### Shot ring

The tag and the encode of a shot read the record of its own build, not a single "last still build" word (header `+248`).
The single word is correct only if no later shot builds its parameters before the tag and the encode of the earlier shot.
In continuous shooting, each shot was built, encoded, and tagged before the next shot was built. The ring does not depend on that order, which is not established for every drive mode and timing.

Each shot has two non-live-view parameter builds, before its encode. The ring keeps the second one.
Both builds read the active record during the same capture, so they normally have the same record.
A test gave the two builds different records. The picture had the effect of the first build, and the tag the effect of the second. The single word gives the same result.

The parameter hook therefore stores the record of each non-live-view build in a ring.
The key is the shot settings pointer of the build: `r7`, which is `[context + 0x82d0]` on GR IV.
The maker-note setter holds the same pointer at `+0x18` (in `r2` at the ImageTone call), and the JPEG encode configuration at `+0x34`. Both targets use these offsets.

The ring follows the date stamp masks in the picture scratch: a write count, a pad word, and eight `{key, record}` entries. A native build stores record 0.
The writer clears the key, writes the record, then writes the key, with a `dmb ish` between the stores. A reader searches from the newest entry and reads the key again after the record.

An entry leaves the ring after eight later builds. A reused shot settings pointer matches its newest entry. A photo whose own build has not run yet can therefore get the previous build of that pointer.
Header `+248` still holds the last non-live-view record, as a diagnostic.

### Grain

The Grain module drives the native grain stage of `DigitalFilterProcess`, which runs for recorded pictures only. The live view does not show it, and DNG files do not have it.
Its `SetEffectParameter` reads the grain settings from the shot settings, and hooks replace those loads with the level in header `+72`. Off keeps the native values.
The stage gate ignores grain for Standard, so a wrapper of the gate call sets the stage flag for any level.

| | GR III 2.10 | GR IV 1.11 |
| --- | --- | --- |
| `SetEffectParameter` | `0x5350304c` | `0x53701c40` |
| Grain loads | Particle `+0x795` (0..3) at `0x535031b0` | on `+0x7b5` at `0x53701e14`, strength `+0x7b7` (0..2) at `0x53701e34`, size `+0x7b6` (0..2) at `0x53701e58` |
| Gate call | `0x5351361c` to `0x535131c0`, flag `+0x24` | `0x5371557c` to `0x537150fc`, flag `+0x25` |
| Level 1..3 | Particle 3..1 | on 1, strength 0..2 |

On GR III, the granulation (`0x5367ad3c`) scales the noise amplitude by 256/256 for Particle 1, 128/256 for Particle 2, and 64/256 for Particle 3. Particle 1 therefore gives the strongest grain, and the hook writes `4 - level`.

The GR IV grain size follows the ISO of the shot: small to ISO 400, medium to ISO 800, and large above.
The shot settings hold the float APEX Sv at `+0xba0`, which the EXIF ISO also uses (`0x537fe338`). The native ISO index is `round((Sv - 1) * 3)` (`0x538f5568`), so ISO 400 ends at Sv 43/6 and ISO 800 at Sv 49/6.
The hook compares the bit pattern of the float, which orders as the value for a positive float. A negative value, an infinity, or a NaN gives small grain.

The hooks read the level when the stage runs, as Date Stamp reads its style. In-camera RAW development therefore gets the level selected at that time.
With a level selected, the level also replaces the grain settings of native styles that have them. The grain pixels have not been checked on a camera.

### Screennail and thumbnail areas

The screennail and the thumbnail have black bars that depend on the capture aspect. The hook uses the visible area from the GetResolution inset tables and the Framing insets.

### Date stamp

The Date Stamp module prints the local time (`0x536b6510`) at the bottom right of the visible area, as a film data back exposes it.
The styles are `'YY M D`, `'YY M D  H:MM`, `M D 'YY`, and `D M 'YY`. Months, days, and hours have no leading zero.

The digits are italic seven-segment characters at 4.5% of the short side, at least 8 pixels. They render into a coverage mask of at most 48 rows (32 for long text), with horizontal anti-aliasing.
Two box blurs of the mask give an amber core and a red-orange halo. The hook scales the mask up and adds the light with a screen blend in RGB.
The digits therefore glow on dark areas and fade on bright ones. A monochrome GRKit effect gets the luma of the light only.

Only the largest plane of an encode renders the mask. The screennail and the thumbnail sample that mask at their own scale, with 2 x 2 subsamples for each pixel.
Playback shows the screennail first and then the main picture. The stamp therefore keeps its position and total width, and only the strokes become sharper.
If the largest plane has digits of 24 pixels or less, its mask has twice that size, and each pixel averages 2 x 2 cells.

The masks use the picture scratch after the records, before the shot ring. The blur line copy is on the stack (1280 bytes), because the code region is read-only.
Every encode that passes through the hook gets the stamp, including in-camera RAW development and edits. Those show the time of the edit.

## Code areas

All private code lives in the tail reservation after the decoded RTOS image. GRKit changes no byte of the image except the hook sites and the in-place patches.

| Component | GR III 2.10 | GR IV 1.11 | Notes |
| --- | --- | --- | --- |
| Unused | `0x540b6000`–`0x540b6100` | `0x543d3000`–`0x543d3100` | Not written |
| Processing | `0x540b6100`–`0x540b6900` | `0x543d3100`–`0x543d3900` | A32 assembly |
| Menu | `0x540b6900`–`0x540b9000` | `0x543d3900`–`0x543d6000` | Thumb-2 C at `-Oz`, A32 entry points |
| Framing | `0x540b9000`–`0x540be000` | `0x543d6000`–`0x543dc000` | A32 C at `-Os` and assembly: Framing, Guides, and the effect extension and picture hooks |

`build.json` gives the size and the limit of each component. The freestanding link has no division helpers, so menu C must not divide.
Picture code divides with its own shift-subtract routine, once per picture for each reciprocal.

The decoded RTOS image ends at `0x540b55c0` on GR III and `0x543d2ac0` on GR IV. The RTOS maps the tail read-only and executable up to `0x55000000`, and no firmware code refers to it. On GR IV, a camera test has not confirmed this mapping.

BOOT loads only the image, so the tail can keep the previous session's code after a restart. The installer therefore overwrites the tail without a stock-byte check.
All other patches keep that check. The builder rejects overflow, code outside the tail, changed hook instructions, and overlapping patches.
