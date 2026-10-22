# Image Control effect sources

Each directory with `effect.json` defines one effect. The directory name is its stable effect ID.
Edit the name, signed Q13 matrix, RGB curves or curve knots, and icon to change an effect. Add a directory to add an effect.

The compiler writes self-contained GRFX0001 files with a 40 x 40 icon. CRC32 detects corruption. Neither the CRC nor the ID authenticates an author.
`golden.json` pins the compiled file and artwork hashes. Change it only for an intentional definition or artwork change.

## Definition fields

| Field | Required | Value |
|---|---|---|
| `schema` | Yes | `grkit-effect-v1` |
| `effect_id` | Yes | The stable ID, the same as the directory name |
| `name` | Yes | 1 to 16 printable ASCII characters |
| `flags` | Yes | 0, or 1 for monochrome |
| `matrix` | Yes | Nine signed Q13 coefficients, row-major. Each row sums to 8192. |
| `curves` or `curve_knots` | Yes | Three 256-sample curves of 0 to 16383, or knots for each channel |
| `icon` | Yes | A label tile, a film box, or a motion picture film label. [Icons](#icons) gives the fields. |
| `attribution` | Yes | An object with the origin, source, and license |
| `post_matrix` | No | Nine signed Q13 coefficients, row-major, after the tone curves |
| `selective_color` | No | `hue` in degrees (0 to 360), `lower` and `higher` `[inner, outer]` half widths in degrees (0 to 75), and `chroma` `[c0, c1]` (c0 < c1 <= 181) |
| `hue_sectors` | No | `matrices`: 11 sectors of six off-diagonal coefficients in 1/128 (-127 to 127; the two of a row sum to -127 or more). `weight`: four values 0 to 255 at luminance 0, 0.055, 0.218, and 1.365 (`hue::KNOTS`); the first two differ by at most about 160, because highlight correction doubles the slopes. Optional `levels`: the sectors of levels 1 to 3, three sets like `matrices` |

A monochrome effect cannot have `post_matrix`, `selective_color`, or `hue_sectors`.

`selective_color` keeps the chroma of one hue range in recorded pictures and removes it elsewhere. The viewfinder shows the effect in monochrome. `crates/grkit-core/src/selective.rs` defines the hue angle and the half widths.

## Icons

The compiler draws each icon in gray. An icon has one of three forms:

| Form | Fields | Picture |
|---|---|---|
| Label tile | `label` (1 to 4 capitals or digits) and `shape` (`film`, `filter`, or `plain`) | The label on a dark tile |
| Film box | `template` `"box"`, `brand`, `name`, an optional `number`, and `band` (`true` or `false`) | A light band with the brand, the name and a large number on a dark body, and a light bottom band if `band` is `true` |
| Motion picture film label | `template` `"cine"`, `brand`, `name`, and an optional `number` | As a film box, with three light stripes at the bottom |

A film box and a film label span the full 40-pixel width of the tile. Their brand uses Inter Black if it fits at full width. Otherwise, it uses a 5-pixel font, which has only capitals, digits, spaces, `-`, and `+`.

The name uses the 5-pixel font, or Inter Black if that font cannot draw it. The number uses Inter Black. A cover without `number` shows the name large, in Inter Black. If a text does not fit, the compiler stops with an error.

The icon shows the effect name and the brand, and nothing else:

- The `name` and `number` of an icon hold the words of the effect name, in capitals. A hyphen can split a word, so `FP-100C` gives `FP` and `100C`.
- The number is the speed, or the model if the film has no single speed (`IV`).
- A motion picture film label shows the film number in place of the series name, because photographers know these films by number: Vision3 500T shows `5219` above `500T`. The target description must give that number.
- Other stock codes, papers, and process details go in the target description, not in the icon or the name.

A test checks that the icon words of each published effect match its name, or the film number of its target.

## Effect catalog

The site and the release packages publish every effect in this directory:

| Effect | ID | Target |
|---|---|---|
| Portra 400 | `portra-400-endura-v1` | Kodak Portra 400 on Portra Endura paper |
| Gold 200 | `gold-200-premier-v1` | Kodak Gold 200 on Endura Premier paper |
| Ektar 100 | `ektar-100-edge-v1` | Kodak Ektar 100 on Ektacolor Edge paper |
| Pro 400H | `pro-400h-crystal-v1` | Fujifilm Pro 400H on Crystal Archive Type II paper |
| Superia 400 | `xtra-400-crystal-v1` | Fujifilm Superia X-TRA 400 on Crystal Archive Type II paper |
| Vision3 250D | `vision3-250d-2383-v1` | Kodak Vision3 250D 5207 on Vision 2383 print film |
| Vision3 500T | `vision3-500t-2393-v1` | Kodak Vision3 500T 5219 on Vision Premier 2393 print film |
| Portra 800 +2 | `portra-800-push2-v1` | Kodak Portra 800 pushed two stops (EI 3200), on Portra Endura paper |
| Kodachrome 64 | `kodachrome-64-v1` | Kodak Kodachrome 64 slide film, scanned |
| Velvia 100 | `velvia-100-v1` | Fujifilm Velvia 100 slide film, scanned |
| Portra 160 | `portra-160-ultra-v1` | Kodak Portra 160 on Ultra Endura paper |
| Tri-X 400 | `trix-400-mono-v1` | Kodak Tri-X 400 on Polymax Fine-Art paper, grade 2 |
| Tri-X 400 Red | `red-filter-mono-v1` | Kodak Tri-X 400 behind a Wratten 25 red filter, on Polymax Fine-Art paper |
| FP-100C | `fp-100c-v1` | Fujifilm FP-100C instant film |
| Aerocolor IV | `aerocolor-iv-v1` | Kodak Aerocolor IV 2460 aerial negative on Portra Endura paper |
| EXR 200T | `exr-200t-2383-v1` | Kodak EXR 200T 5293 on Vision 2383 print film |

Each effect is a fit of one film simulation LUT. Their menu names are the names of the simulated films. [NOTICE.md](NOTICE.md) gives the attribution, the licenses, and the trademark notice.

The first eleven LUTs come from [spektrafilm](https://github.com/andreavolpato/spektrafilm) 0.3.4. The other five come from [Spectral Film LUT](https://github.com/JanLohse/spectral_film_lut). `sources/` holds the scripts that bake each LUT, the upstream notices, and `manifest.json`, which records the SHA-256 of each LUT. The LUTs themselves are not in the repository.

To refit an effect:

1. Install the generator at the revision in `sources/manifest.json`.
2. Run `python sources/bake_spektrafilm.py LUTS` or `python sources/bake_spectral_film_lut.py LUTS`.
3. Compare the SHA-256 of `LUTS/ID.cube` with the manifest. Floating-point results can differ on another platform.
4. Run `grkit effects import --input vlog` with the effect's ID, name, label, and shape.
   The import writes a label tile. To show a film box or a film label, replace `icon` after the import.
5. Copy `license`, `target`, and `generator` from the old `attribution`.

The two monochrome fits have identical matrix rows and curves. Set `flags` to 1 for them.

## Processing domain

Effects run in the camera's processing: the matrix acts before the Standard base gamma, and the curves and `post_matrix` act after it. They are not display-sRGB JPEG transforms. Effects add no grain or glow.

`hue_sectors` drives the camera's hue-sector stage after the matrix, in recorded pictures and the live view. With `levels`, the sector matrices also change with luminance. `crates/grkit-core/src/hue.rs` describes the stage.

## Import a .cube preset

`grkit effects import` fits a 3D or 1D `.cube` LUT to the effect model and writes a new definition.
The fit needs the Standard base gamma. The command reads this table from your copy of the official GR III 2.10 firmware. The repository contains no firmware data.

```sh
grkit effects import --cube look.cube --firmware fwdc239b.bin --input srgb \
  --id my-look-v1 --name "My Look" --label FV --shape film --out effects
```

1. Set `--firmware` to the official GR III 2.10 firmware file.
2. Optional: select `--input` for the encoding of the LUT input. The default is `auto`.
   - `auto`: the importer reads the LUT with each input (`fit::guess_input`). It keeps a log input only if the result is much closer to the camera's Standard rendering. Otherwise it uses `standard`. It does not read the file name, and it never selects `srgb` or `linear`.
   - `standard`: a picture of the camera's Standard rendering, as a GR JPEG.
   - `srgb`: sRGB-encoded display RGB. The importer clips scene-linear light to 0..1 before the sRGB curve.
   - `linear`: scene-linear RGB with sRGB primaries.
   - `vlog`: Panasonic V-Log over V-Gamut primaries, as spektrafilm writes its LUTs.
   - `flog`, `flog2`, `flog2c`: Fujifilm F-Log or F-Log2 over F-Gamut, or F-Log2 over F-Gamut C.

   If you select an input and the LUT data suggests a different one, the import adds a warning.
3. Optional: add `--exposure EV` to change the scene-linear light before the LUT. The default is 0.
4. Optional: add `--no-post` to fit no post-curve matrix.
5. Run the command. The command writes `OUT/ID/effect.json` and prints a JSON summary.
6. Add the license of the LUT to `attribution`. Then run `grkit effects build`.

If `OUT/ID` exists or a check fails, the command stops and writes nothing.

The site imports a LUT in the browser with the same fit, over the base gamma of the published effect catalog, so the user needs no firmware. See [LUT import](../web/README.md#lut-import).

The fit follows [gr3-film-mods](https://github.com/idontknownihil/gr3-film-mods) ([MIT](../crates/grkit-core/GR3_FILM_MODS_LICENSE.txt)). `crates/grkit-core/src/fit.rs` describes the model and its assumptions.

`attribution.fit` records the color error of the fit on a ColorChecker. A matrix, three curves, a post matrix, and 11 hue sectors copy hue-selective changes only in part. Examine the errors before you use an imported effect.
