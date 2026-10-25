# Writing a module

A module adds one feature to GRKit. It is one file, `<ID>.GRM`, that runs on each supported camera.
This guide shows how to write, test, build, and install a module.
[module-reference.md](module-reference.md) gives each key, limit, and error number.
[Modules](architecture.md#modules) in the architecture describes how the installer links a module.

The official modules in `modules/` are complete examples:

| Folder | What it shows |
| --- | --- |
| `modules/guides` | an own setting, a choice page, and a layer |
| `modules/framing` | a page and two layers of a capability, `CORE.ASPECT` |
| `modules/date` | two settings, a page with a native settings page, and a plane filter |
| `modules/grain` | a module with only data: a page of `CORE.GRAIN` |
| `modules/film-look` | a page of `CORE.TONE`, whose rows the core gives |

## Prerequisites

You need these tools:

- Rust 1.92 or later, to build the `grkit` command.
- Clang with ARM support. Apple Clang works. The build does not need an ARM libc or sysroot.
- LLD as `ld.lld`. If `ld.lld` is not on `PATH`, the Rust toolchain has one at `$(rustc --print sysroot)/lib/rustlib/<host>/bin/gcc-ld/ld.lld`. For example, `<host>` is `aarch64-apple-darwin` on an Apple silicon Mac.
- A host C compiler as `cc`, for the host tests.

To install the command and check the tools:

1. In the GRKit checkout, install the command: `cargo install --path crates/grkit-cli --locked`
2. Check the ARM toolchain: `grkit doctor`
3. If the command reports `Required tool not found: ld.lld`, give the path of LLD: `grkit doctor --lld <path of ld.lld>`

`grkit doctor` compiles and links a small ARM program. If it succeeds, it prints the Clang and LLD versions:

```json
{
  "grkit_version": "<version>",
  "toolchain": {
    "clang": "Apple clang version 21.0.0 (clang-2100.3.34.1)",
    "lld": "LLD 21.1.3 (...)"
  }
}
```

If you give `--lld` or `--clang` to `grkit doctor`, give the same option to `grkit module build`.

## Create a module folder

A module ID has 1 to 8 characters: `A`-`Z`, `0`-`9`, or `_`. The ID is also the file name, and the camera lists file names in upper case.

1. Select an ID that no other module on the card has.
2. Create the folder: `grkit module new --id HELLO --out hello`

The folder must not exist. The command prints the number of files that it writes:

```json
{
  "files": 9,
  "id": "HELLO",
  "output": "hello"
}
```

The new folder holds a working module. It has one setting, `MODE` (Off, On), a choice page for `MODE`, and a layer. While `MODE` is On, the layer draws a white square at the top right of the photo area.

| File | Contents |
| --- | --- |
| `module.toml` | the manifest: the ID, the sources, the tiles, the setting, the page, and the layer |
| `module.c` | the paint function of the layer |
| `test.c` | the host test of the paint function |
| `text/en.toml`, `text/zh-Hans.toml`, `text/zh-Hant.toml`, `text/ja.toml` | the page text in each menu language |
| `tiles/0-module.png`, `tiles/1-on.png` | the 40 x 40 tiles of the page |

The module builds and passes its test without changes. Change it one part at a time, and run the test after each change.

## Describe the module

`module.toml` tells `grkit module build` what the module gives the core. The build turns it into the descriptor of the module, `grkit_module`. You do not write the descriptor in C.

This is the manifest of the Guides module, `modules/guides/module.toml`:

```toml
id = "GUIDES"
version = 1
sources = ["guides.c"]
# Tile n shows value n. Tile 0 is the module tile, for Off at the root.
tiles = [
    "tiles/0-module.png",
    "tiles/1-thirds.png",
    "tiles/2-center.png",
    "tiles/3-golden.png",
    "tiles/4-diagonals.png",
    "tiles/5-thirds-center.png",
]

# 0 Off, 1 thirds, 2 center, 3 golden ratio, 4 diagonals, 5 thirds and center.
[[settings]]
key = "STYLE"
values = 6
saved = true
public = true

[page]
setting = "STYLE"
off_row = true

[[layers]]
paint = "guides_paint"
setting = "STYLE"
order = 20
follow_disp = true
```

The manifest gives these parts:

- `[[settings]]`: the setting `GUIDES.STYLE` has 6 values. The state file keeps its value (`saved`). Other modules can use it (`public`).
- `[page]`: a choice page of `STYLE` in the GRKit menu, with one row for each value. Row 0 shows the Off icon (`off_row`).
- `[[layers]]`: the core calls the C function `guides_paint` for each draw of the live view while `STYLE` is not 0. The layer paints after the layers with a lower `order`, and it hides while DISP hides the native information (`follow_disp`).

A module can also use a setting of another module or of the core. Name it in `uses` as `MODULE.KEY`. A name that starts with `?` is optional. For example, Framing uses the capability `CORE.ASPECT` and has no setting of its own:

```toml
uses = ["CORE.ASPECT"]

[page]
setting = "CORE.ASPECT"
off_row = true
```

A plane filter changes each recorded picture. Date Imprint has one:

```toml
[[filters]]
apply = "date_apply"
setting = "STYLE"
stage = "mark"
```

A manifest must give at least one page, layer, or filter. See [module.toml](module-reference.md#moduletoml) for each key and its limits.

## Write the text and the tiles

Each page has text in the four menu languages: `text/en.toml`, `text/zh-Hans.toml`, `text/zh-Hant.toml`, and `text/ja.toml`. Each file must exist. This is `modules/guides/text/en.toml`:

```toml
title = "Guides"
rows = ["Off", "Thirds", "Center", "Golden Ratio", "Diagonals", "Thirds + Center"]
```

- `title` is the name of the page. The GRKit section of MENU also shows it as the name of the module.
- `rows` has one label for each value of the page setting.
- `summaries` is optional. It gives a shorter value for the root row, which shows `<title><separator><value>`.

A page with a native settings page also gives the text of each row in `[settings.<KEY>]`. See `modules/date/text/en.toml`.

The tiles are 40 x 40 PNG files. Tile n shows value n of the page setting. Tile 0 is also the module tile: the root shows it at value 0. A choice page needs one tile for each value.

## Write the C code

A module source includes only `grkit.h` and the generated `grkit_module.h`. `grkit.h` is the module SDK in `runtime/sdk/`, and the build adds its folder to the include path. `grkit_module.h` defines a handle macro for each setting:

- `SETTING_<KEY>` for each setting in `[[settings]]`, such as `SETTING_STYLE`.
- `USED_<MODULE>_<KEY>` for each name in `uses`, such as `USED_CORE_ASPECT`.

The core calls the functions that the manifest names:

```c
#include "grkit.h"
#include "grkit_module.h"

/* A layer: called for each draw of the live view with the value of its setting. */
void hello_paint(const Surface *s, u32 value);
/* A plane filter: called for each recorded picture with the value of its setting. */
void hello_apply(PictureSet *set, u32 value);
```

The core does not call a layer or a filter while the value of its setting is 0. A function reads other settings with `setting_get`:

```c
void date_apply(PictureSet *set, u32 style)
{
    u32 look = setting_get(SETTING_LOOK);
    /* ... */
}
```

Obey these rules. The build or the installer rejects a module that breaks the first four:

- Call only the services that `grkit.h` declares: the settings, `overlay_label`, `local_time`, `divide`, the timer, and the UI.
- Do not divide by a variable with `/` or `%`. The compiler calls a helper for it, which the core does not export. Use `divide(n, d)`, or multiply by a constant reciprocal.
- Do not initialize or copy a large array or structure with one statement, such as `u32 b[200] = {0}`. The compiler can call a helper such as `__aeabi_memclr8`. Write a loop.
- Keep each stack frame at 1536 bytes or less. Module code runs on the stacks of firmware tasks.
- Do not keep a copy of a setting value. Read the setting each time that you use it, because the menu can change it at any time.
- Do not write firmware addresses or names. The same module file must run on each camera.

A module has no init function. The core calls the layers, the filters, and the event handler of the module. Static variables are writable: the installer puts them in a script array. Constant data stays in the code memory, which is read-only.

To run code when GRKit is ready or when a setting changes, add an event handler:

```toml
[events]
handle = "hello_event"
on = ["system_ready", "setting_changed"]
```

```c
void hello_event(u32 event, u32 a, u32 b)
{
    if (event == GRKIT_EVENT_SETTING_CHANGED && a == GRKIT_SETTING(0) && b)
        ui_toast(saved_text, 1500);
}
```

The handler runs in the App task, so it can start a timer and show a message box. A box shows only while the GRKit page of ADJ is open. See [Events](module-reference.md#events) and [UI](module-reference.md#ui).

See [Services](module-reference.md#services) for the functions and [Registrations](module-reference.md#registrations) for the surface and the planes.

## Test the module on the computer

`grkit module test` compiles the module with the host compiler and runs its `test.c`. A harness stands in for the core: it gives a surface, plane memory, the settings, `overlay_label`, `local_time`, `divide`, the timer, and the UI. `runtime/sdk/grkit_test.h` declares it.

1. Write the test in `test.c`. It has a `main` function that returns 0 and checks the results with `assert`.
2. Run the test: `grkit module test --dir hello`

If the test passes, the command prints:

```json
{
  "module": "hello",
  "test": "passed"
}
```

A module without C sources and without `test.c` has no test. The command then checks only the manifest and the text, and prints `"test": "manifest checked"`.

If an assertion fails, the command prints the assertion and exits with status 1:

```text
grkit: The test of HELLO failed
Assertion failed: (grkit_test_pixel(700, 20) == 0U), function main, file test.c, line 16.
```

A test uses these parts of the harness:

| Name | Use |
| --- | --- |
| `grkit_test_surface(box)` | a 720 x 480 surface over the photo area `box` {left, top, right, bottom}. Set `area` for a crop. |
| `grkit_test_clear()`, `grkit_test_pixel(x, y)`, `GRKIT_TEST_CLEAR` | clear the surface, and read a pixel after a paint |
| `GRKIT_TEST_PLANES`, `GRKIT_TEST_PLANE_BYTES`, `GRKIT_BYTES(address)` | plane memory for a `PictureSet` |
| `grkit_test_scratch` | the scratch of a `PictureSet` |
| `grkit_test_settings[SETTING_<KEY>]` | the value of each setting of the module. `SETTING_<KEY>` is already a handle: pass it to `setting_get` as it is. The harness starts the handles above 0, as the camera does. |
| `grkit_test_time` | the time that `local_time` gives |
| `grkit_test_labels`, `grkit_test_label` | the number of `overlay_label` calls, and the label, left, and top of the last one |
| `grkit_test_timers`, `grkit_test_fire_timers()` | the number of started timers, and a call of each timer, as the App task would |
| `grkit_test_ui_calls`, `grkit_test_ui_text`, `grkit_test_ui_shows` | the number of UI calls, the last text, and the result of a UI call (1 at start) |
| `grkit_test_ui_confirm_done`, `grkit_test_ui_message_done`, `grkit_test_ui_arg`, `grkit_test_ui_percent` | the callback and argument of the last confirm or message, which a test calls, and the last progress |
| `grkit_test_slider_changed`, `grkit_test_slider_done` | the callbacks of the last `ui_slider`, which a test calls |
| `grkit_test_native`, `grkit_test_native_back` | the screen and the callback of the last `ui_open_native`. A test calls the callback as the return from the screen would. |
| `grkit_module` | the generated descriptor, to check the counts of settings, registrations, and tiles |

The harness does not check the task of a caller. The harness has no used settings. To test a layer or a filter of a used setting, call it with the value directly. See `modules/framing/test.c` and `modules/date/test.c`.

A host test checks the logic and the pixels. It does not check the behavior on a camera.

## Build the module file

`grkit module build` compiles the sources for the camera, generates the descriptor, links the module, and writes the module file.

1. Build the file: `grkit module build --dir hello --out HELLO.GRM`
2. If `ld.lld` is not on `PATH`, add `--lld <path of ld.lld>`.

The command prints the ID and the file size:

```json
{
  "bytes": 13516,
  "id": "HELLO",
  "output": "HELLO.GRM"
}
```

The output file and the objects folder must not exist. The objects folder is next to the output by default, such as `HELLO.objects`. It holds the generated `grkit_module.c` and `grkit_module.h`, the objects, and `module.elf`. To build again, delete both, or give a new folder with `--objects`.

The build stops with a message for each rule that the module breaks. For example:

```text
grkit: The module uses __aeabi_uidiv, which the core does not export (runtime/sdk/grkit.h)
```

```text
error: stack frame size (4104) exceeds limit (1536) in 'module_big' [-Werror,-Wframe-larger-than]
```

The command writes the file name that `--out` gives. The installer loads a file only if its name is `<ID>.GRM`. Name the output by the module ID.

## Install the module on a card

The card must already hold a GRKit package for the camera: the folder `GRKIT` with `LOADER.TTL` and `CORE`.

1. Copy `HELLO.GRM` into `GRKIT/MODULES` on the card.
2. Put the card into the camera, and power on the camera.
3. Wait until the ADJ toolbar shows the GRKit entry. INSTALL adds the entry after it links the modules.
4. Power off the camera, and put the card into the computer.
5. Open `GRKIT/REPORT.TXT`.

If the entry does not show, GRKit did not install. The report then gives the error number of the installation.

The report gives one line for each module file:

```text
Modules (GRKIT\MODULES):
FRAMING.GRM  loaded
HELLO.GRM  loaded
```

If a module did not load, the line gives an error number and the reason, for example:

```text
HELLO.GRM  not loaded: error 55, it uses a setting that no module has: GUIDES.STYLE
```

[Report errors](module-reference.md#report-errors) gives the meaning of each error number.

A loaded module with a page shows a row in the GRKit menu. The root shows a page only if it has two rows or more. By default, the official modules come first, then the other modules in the order of their file names.

To remove a module, delete its file from `GRKIT/MODULES`. The state file keeps its saved settings.

## Turn a module on or off, or move it

The **GRKit** section at the end of the Customize tab of the native MENU has a row for each module file of the package and of `GRKIT/MODULES`, also a module that did not load. A module that did not load shows the error number after its name, such as `HELLO (55)`.

To turn a module on or off:

1. Push MENU, and open the Customize tab.
2. Select the row of the module in the GRKit section, and push Right.
3. Select **Enable**, then **On** or **Off**. Push OK.
4. Power off the camera, and power it on again.

To move the page of a module in the GRKit root of ADJ:

1. Select the **ADJ Order** row in the GRKit section, and push Right.
2. Select the module, and push Right.
3. Select its new position. Push OK.

Enable and the order apply at once. A module that is Off stops: its page leaves ADJ, and its code does not run. Its saved settings stay in the state file. A module that you add to `GRKIT/MODULES` loads at the next power-on; until then, the About GRKit row says to restart.

## Measure the CPU use on the camera

The development profile counts the calls and the CPU cycles of the GRKit core and of each module on the camera.

1. Put an empty file `GRKIT/PROFILE` on the card.
2. Power on the camera, and do what you want to measure: for example, keep the live view open, half-press the shutter button, or take photos.
3. Power off the camera, and put the card into the computer.
4. Open `GRKIT/PROFILE.TXT`.
5. To stop the profile, delete `GRKIT/PROFILE`.

While the profile runs, `REPORT.TXT` says so. The resident loader adds a period to `PROFILE.TXT` every 10 seconds, also after a new power-on:

```text
period <number> frames <live-view frames>
<point> <calls> <cycles> <cycles of the largest call>
<MODULE> <kind> <calls> <cycles> <cycles of the largest call>
```

Only a point that ran in the period has a line. The core points are `frame.bind`, `frame.prepare`, and `frame.paint` of the live view, `picture.post` and `picture.encode` of a picture, and `menu.icon`. `frame.paint` includes the module layers, and `picture.encode` includes the plane filters. A module line gives the kind of the call:

- `paint`: a live-view layer.
- `filter`: a plane filter.
- `call`: an event handler, a timer callback, or a UI callback.

The profile does not count the Image Control processing of each frame or the resident loader. While it runs, the loader writes the card every 10 seconds, so do not use it to measure battery life.

## If the camera stops at power-on

A module can stop the camera. Turn the module off with its Enable row in the GRKit section of MENU. If the camera stops before MENU opens, remove the module from `GRKIT/MODULES`, or put an empty file `GRKIT/SAFE` on the card: GRKit then starts in the safe mode, in which no module loads.

In the safe mode, `REPORT.TXT` says why no module loads, and the GRKit section of MENU stays available.

1. Delete the module that you added last from `GRKIT/MODULES`, or set its **Enable** to **Off** in the GRKit section of MENU.
2. If you made `GRKIT/SAFE`, delete it.
3. Power on the camera.

See [Report and safe mode](architecture.md#report-and-safe-mode).
