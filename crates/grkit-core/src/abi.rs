//! The private memory layout that the loader, the installer, and the ARM code
//! share. These tables are its only source: `header()` generates
//! `runtime/abi.h`, and the `generated` test keeps the checked-in file current.

use std::fmt::Write;

use crate::{effects, hue, menu_text, plugins, runtime};

/// A byte range of the private header or of a record.
pub struct Field {
    /// The C name without the `GRKIT_` prefix.
    pub name: &'static str,
    pub offset: usize,
    pub size: usize,
    pub doc: &'static str,
}

/// Declares a `usize` constant for each field and a table of the fields.
macro_rules! fields {
    ($table:ident { $($(#[doc = $doc:literal])* $name:ident = $offset:expr, $size:expr;)* }) => {
        $($(#[doc = $doc])* pub const $name: usize = $offset;)*
        pub const $table: &[Field] = &[$(Field {
            name: stringify!($name),
            offset: $offset,
            size: $size,
            doc: concat!($($doc, "\n"),*),
        }),*];
    };
}

/// The core fields of the header, then the plugin words.
pub const HOST_HEADER_SIZE: usize = 512;
/// The placeholder for the header address in ARM code. The installer replaces
/// each literal word with this value by the address of the `registry` array.
pub const HEADER_REFERENCE: u32 = 0x58012340;
/// `GRM1`.
pub const HOST_MAGIC_VALUE: u32 = 0x314d5247;
pub const HOST_VERSION_VALUE: u32 = 1;

fields!(HEADER_FIELDS {
    /// Number of Image Control records.
    HOST_COUNT = 0, 4;
    /// Address of record 0. The records follow the header without gaps.
    HOST_RECORDS = 4, 4;
    /// Selected record, or 0 for native processing.
    HOST_ACTIVE = 8, 4;
    /// Record of the last private processing packet. A diagnostic.
    HOST_LAST_APPLIED = 12, 4;
    /// GRKIT_HOST_MAGIC_VALUE after the installer writes the header.
    HOST_MAGIC = 16, 4;
    /// GRKIT_HOST_VERSION_VALUE.
    HOST_VERSION = 20, 4;
    /// Context word (+0x2ac) of the last parameter build. A diagnostic.
    HOST_PROCESSING_CONTEXT = 24, 4;
    /// Number of parameter builds. A diagnostic.
    HOST_PROCESSING_CALLS = 28, 4;
    /// ADJ controller that shows the private menu, or 0.
    HOST_MENU_CONTROLLER = 32, 4;
    /// Private menu level: 0 native, MENU_ROOT, or MENU_MODULE (runtime/modules.h).
    HOST_MENU_LEVEL = 36, 4;
    /// Highlighted row of the private menu.
    HOST_MENU_CURSOR = 40, 4;
    /// Native ADJ slot when the private menu opened. A diagnostic.
    HOST_MENU_SLOT = 44, 4;
    /// Guide style previewed on the ADJ canvas while GRKIT_HOST_FRAME_PREVIEW is set.
    HOST_GUIDE_PREVIEW = 48, 4;
    /// Date stamp style: 0 Off, 1 'YY M D, 2 'YY M D H:MM, 3 M D 'YY, 4 D M 'YY.
    HOST_STAMP = 52, 4;
    /// Installed plugins: bit n is plugin n of grkit_core::plugins::PLUGINS and
    /// module n of the menu. A plugin that needs effects is set only with effects.
    HOST_PLUGINS = 56, 4;
    /// Nonzero while the menu rebuilds the native list. The list callback and the
    /// native Image Control preview ignore the changes.
    HOST_MENU_REBUILDING = 60, 4;
    /// Native list position, saved when the private menu opens.
    HOST_MENU_LIST = 64, 8;
    /// The module + 1 whose native settings page the menu opened from its root
    /// row (Module.settings, runtime/plugin.h), or 0. While it is set, the
    /// hooks of the borrowed native page show the settings of that module. A
    /// change to a controller outside the page clears it.
    HOST_SETTINGS = 72, 4;
    /// Number of ADJ events. GRKIT_HOST_MENU_EVENT_RING holds the last ones.
    HOST_MENU_EVENTS = 76, 4;
    /// Number of Image Control selections. A diagnostic.
    HOST_MENU_APPLIES = 80, 4;
    /// Icon of Off, or 0 for the native icon.
    HOST_OFF_ICON = 84, 4;
    /// How the private menu opened: 1 or 2 across an edge of the native slots,
    /// 3 by touch or on the return from the native parameters.
    HOST_MENU_EDGE = 88, 4;
    /// MENU was pressed, and its confirm event has not arrived yet.
    HOST_BACK_PENDING = 92, 4;
    /// Text descriptor of the ADJ title: (length + 1) | 0x100, then the address.
    HOST_MENU_TITLE = 96, 8;
    /// Text descriptor of the ADJ detail line.
    HOST_MENU_DETAIL = 104, 8;
    /// The last ADJ events: kind | argument << 16.
    HOST_MENU_EVENT_RING = 112, 32;
    /// The record whose adjustments the native parameter page edits, or 0. Fn
    /// in GRKit Image Control stores the record with bit 0 set, and the page
    /// start clears bit 0. A page that native ADJ opens clears the word.
    HOST_PARAMETERS = 144, 4;
    /// Number of adjustment changes. The live view rebuilds its stages when it
    /// changes.
    HOST_PARAMETER_CHANGES = 148, 4;
    /// GRKIT_HOST_PARAMETER_CHANGES at the last live-view build.
    HOST_LIVE_PARAMETER_CHANGES = 152, 4;
    /// Icon chunk addresses: chunk n holds icon slots 40 n to 40 n + 39. The
    /// host icons come first in chunk 0, then the effect icons.
    HOST_ICON_CHUNKS = 156, 4 * runtime::ICON_CHUNK_SLOTS;
    /// Icon of the GRKit tile.
    HOST_PLUGINS_ICON = 176, 4;
    /// Private toolbar cell, or 0.
    HOST_TOOLBAR_WIDGET = 180, 4;
    /// ADJ controller of the private toolbar cell.
    HOST_TOOLBAR_OWNER = 184, 4;
    /// Module that the private menu shows at MENU_MODULE. At the end of a
    /// private session it holds the module of the root row where the next
    /// session opens (menu_reopen).
    HOST_MODULE = 188, 4;
    /// Reserved and zero.
    HOST_RESERVED_192 = 192, 4;
    /// Aspect id 2 variant for the photo shown in playback: 0 none (shooting uses
    /// the Framing ratio), 1 not 65:24, 2 65:24.
    HOST_FRAME_FILE = 196, 4;
    /// Record of the last live-view build: an owned record, or 0 for native processing.
    HOST_LIVE_RECORD = 200, 4;
    /// Committed composition guide style: 0 Off, 1 thirds, 2 center, 3 golden
    /// ratio, 4 diagonals.
    HOST_GUIDE = 204, 4;
    /// Framing ratio: 0 native, then the Framing rows.
    HOST_FRAME_RATIO = 208, 4;
    /// ADJ Framing page preview: 0 none, else the highlighted ratio + 1.
    HOST_FRAME_PREVIEW = 212, 4;
    /// Number of Framing paints. A diagnostic.
    HOST_FRAME_DRAWS = 216, 4;
    /// ADJ canvas of the preview. It stays set until a full draw erases the mask.
    HOST_FRAME_PREVIEW_CANVAS = 220, 4;
    /// Icon of the first plugin tile. The tiles of each plugin follow in plugin
    /// order (grkit_core::plugins::tiles).
    HOST_TILE_ICON = 224, 4;
    /// Live view controller that Framing draws on, or 0.
    HOST_FRAME_OWNER = 228, 4;
    /// Canvas of that controller.
    HOST_FRAME_CANVAS = 232, 4;
    /// Bitmap of the last Framing paint, or 0 when nothing is drawn.
    HOST_FRAME_BITMAP = 236, 4;
    /// Number of paints without a bitmap. A diagnostic.
    HOST_FRAME_SKIPPED = 240, 4;
    /// Live-view-only override: 0 none, 1 native processing, else an owned record.
    HOST_PREVIEW = 244, 4;
    /// Record applied by the last non-live-view parameter build, or 0. A
    /// diagnostic: capture metadata and the encode read the shot ring.
    HOST_STILL = 248, 4;
    /// Return to GRKit Image Control after the native parameter page: 1 opening,
    /// 2 away. 3: ADJ exited from GRKit, so the next ADJ opens GRKit. 4: MENU
    /// or OK on GRKit passed the event that closes ADJ to the native list.
    HOST_RETURN = 252, 4;
    /// The words of the plugins (grkit_core::plugins::Plugin::words), in plugin
    /// order, then zero words. GRKIT_PLUGIN_<PLUGIN>_<WORD> gives each offset.
    HOST_PLUGIN_AREA = 256, HOST_HEADER_SIZE - 256;
});

pub const RECORD_SIZE: usize = 0x1100;

fields!(RECORD_FIELDS {
    /// The native processing record: a prefix, then the tone curves.
    RECORD_COEFFICIENTS = 0x54, 0x668 - 0x54;
    /// UTF-16 effect name with its NUL.
    RECORD_NAME = 0x668, 34;
    /// Nonzero for a monochrome effect.
    RECORD_MONOCHROME = 0x690, 4;
    /// Address of the effect icon.
    RECORD_ICON = 0x694, 4;
    /// Effect ID with its NUL.
    RECORD_ID = 0x698, 32;
    /// u16 ImageTone code recorded in photos: 0x8000 | hash of the effect ID.
    RECORD_CODE = 0x6f8, 2;
    /// GRF extension, zero for an effect without features.
    RECORD_FEATURES = 0x700, 4;
    /// i16[9] signed Q13 RGB matrix after the tone curves, applied through the
    /// YCbCr matrix.
    RECORD_POST = 0x704, 18;
    /// Selective color for recorded pictures: i16 cos and sin of the target hue
    /// (Q12), u16 inner and outer tangents toward smaller hue angles, then toward
    /// larger ones (Q8), and u8 chroma gate c0 and c1. Bytes 0x724..0x72b are
    /// reserved and zero.
    RECORD_SELECTIVE = 0x716, 0x72c - 0x716;
    /// Fn adjustments of the effect: saturation, hue, high/low key, contrast,
    /// highlight contrast, shadow contrast, sharpness, shading, clarity. Each
    /// byte is (native value - 4) mod 9, so zero is neutral. A native value is
    /// 0..8, and 4 is neutral.
    RECORD_PARAMETERS = 0x72c, 9;
    /// The MultiAxial parameter set that INSTALL expands from the hue sectors of the GRF
    /// (feature 8): the address of the set at +0, the address of the highlight-correction
    /// set at +4, the set (six pointers) at +8, the bounds at +0x20, the bank selectors at
    /// +0x40, the matrix header at +0x58, the control fields at +0x6c, 60 3x3 i32 Q10
    /// matrices at +0x98, the highlight-correction set at +0x908, and its control fields
    /// at +0x920. Zero without hue sectors.
    RECORD_MULTIAXIAL = 0x740, 0x980;
});

/// Bytes of the date stamp core and halo masks at the start of the picture scratch.
pub const PICTURE_MASKS: usize = 141376;
/// Entries of the shot ring.
pub const SHOT_ENTRIES: usize = 8;
/// The shot ring: a write count, a pad word, then entry n at
/// `SHOT_ENTRY + SHOT_STRIDE * n`, which holds the key at `SHOT_KEY` and the
/// record at `SHOT_RECORD`.
pub const SHOT_ENTRY: usize = 8;
pub const SHOT_STRIDE: usize = 8;
pub const SHOT_KEY: usize = 0;
pub const SHOT_RECORD: usize = 4;
/// The picture scratch after the records: the masks, then the shot ring.
pub const PICTURE_SCRATCH: usize = PICTURE_MASKS + SHOT_ENTRY + SHOT_STRIDE * SHOT_ENTRIES;
/// Everything after the records: the picture scratch, then the menu text.
pub const TAIL_SIZE: usize = PICTURE_SCRATCH + menu_text::TEXT_SIZE;
// The summaries of every record in every menu language end before the
// parameter page title.
const _: () = assert!(
    menu_text::TEXT_SUMMARY
        + runtime::MAX_COUNT * crate::packaging::Language::ALL.len() * menu_text::SUMMARY_UNITS * 2
        <= menu_text::TEXT_SETTINGS
);

/// Wraps `text` into a C comment.
pub(crate) fn comment(out: &mut String, text: &str) {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return;
    }
    let mut lines = vec![String::new()];
    for word in words {
        let line = lines.last_mut().unwrap();
        if !line.is_empty() && line.len() + 1 + word.len() > 76 {
            lines.push(word.to_owned());
        } else {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
    }
    for (index, line) in lines.iter().enumerate() {
        out.push_str(if index == 0 { "/* " } else { "   " });
        out.push_str(line);
        out.push_str(if index + 1 == lines.len() {
            " */\n"
        } else {
            "\n"
        });
    }
}

fn define(out: &mut String, doc: &str, name: &str, value: impl std::fmt::Display) {
    comment(out, doc);
    writeln!(out, "#define GRKIT_{name} {value}").unwrap();
}

fn table(out: &mut String, fields: &[Field], hex: bool) {
    for field in fields {
        if hex {
            define(out, field.doc, field.name, format!("{:#x}", field.offset));
        } else {
            define(out, field.doc, field.name, field.offset);
        }
        if field.size > 4 && field.size % 4 == 0 && !hex {
            writeln!(out, "#define GRKIT_{}_WORDS {}", field.name, field.size / 4).unwrap();
        }
    }
}

fn array(out: &mut String, doc: &str, name: &str, values: &[impl std::fmt::Display]) {
    let values: Vec<String> = values.iter().map(ToString::to_string).collect();
    define(out, doc, name, format!("{{ {} }}", values.join(", ")));
}

/// The GRF extension features, and the hue sectors that INSTALL expands into the
/// MultiAxial set of a record (`effects`, `hue`).
fn hue_tables(out: &mut String) {
    writeln!(out, "#define GRKIT_FEATURE_POST {}U", effects::FEATURE_POST).unwrap();
    writeln!(
        out,
        "#define GRKIT_FEATURE_SELECTIVE {}U",
        effects::FEATURE_SELECTIVE
    )
    .unwrap();
    writeln!(
        out,
        "#define GRKIT_FEATURE_HUE {}U",
        effects::FEATURE_HUE_SECTORS
    )
    .unwrap();
    writeln!(
        out,
        "#define GRKIT_FEATURE_HUE_LEVELS {}U",
        effects::FEATURE_HUE_LEVELS
    )
    .unwrap();
    define(
        out,
        "Hue sectors of feature 8 in a GRF: 11 x 6 i8 coefficients (GRKIT_HUE_MATRICES \
         bytes), then the u16 control fields 10..20 of the weight curve.",
        "HUE_OFFSET",
        format!("{}U", effects::HUE_OFFSET),
    );
    writeln!(out, "#define GRKIT_HUE_SIZE {}U", hue::SIZE).unwrap();
    writeln!(out, "#define GRKIT_HUE_MATRICES {}U", hue::SECTORS * 6).unwrap();
    define(
        out,
        "Hue levels of feature 16: the sectors of levels 1 to 3 after the first \
         GRKIT_GRF_SIZE bytes.",
        "HUE_LEVEL_OFFSET",
        format!("{}U", effects::SIZE),
    );
    writeln!(out, "#define GRKIT_HUE_LEVEL_SIZE {}U", hue::LEVEL_SIZE).unwrap();
    array(
        out,
        "The fixed breakpoints of the weight curve, control fields 18..20.",
        "HUE_BORDERS",
        &hue::BORDERS,
    );
    array(
        out,
        "The quadrant bounds and the bank of each segment: 11 equal sectors, sector k \
         in bank k + 1.",
        "HUE_BOUNDS",
        &hue::BOUNDS,
    );
    array(out, "", "HUE_SELECTORS", &hue::SELECTORS);
    array(
        out,
        "Control fields 1..8 that spread the levels, and those for a capture with \
         highlight correction, which halves the stage's input.",
        "HUE_LEVEL_CONTROL",
        &hue::LEVEL_CONTROL,
    );
    array(
        out,
        "",
        "HUE_LEVEL_CONTROL_HALVED",
        &hue::LEVEL_CONTROL_HALVED,
    );
}

/// The contents of `runtime/abi.h`.
pub fn header() -> String {
    let mut out = String::new();
    comment(
        &mut out,
        "Generated from crates/grkit-core/src/abi.rs. Do not edit: change the \
         tables there, then run GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core \
         --test generated.",
    );
    out.push_str("#ifndef GRKIT_ABI_H\n#define GRKIT_ABI_H\n\n");

    comment(
        &mut out,
        "The private header: the start of the registry array. ARM code addresses \
         it through the literal GRKIT_HEADER_REFERENCE, which the installer relocates.",
    );
    writeln!(out, "#define GRKIT_HEADER_REFERENCE {HEADER_REFERENCE:#x}").unwrap();
    writeln!(out, "#define GRKIT_HOST_HEADER_SIZE {HOST_HEADER_SIZE}").unwrap();
    writeln!(out, "#define GRKIT_HOST_MAGIC_VALUE {HOST_MAGIC_VALUE:#x}").unwrap();
    writeln!(out, "#define GRKIT_HOST_VERSION_VALUE {HOST_VERSION_VALUE}").unwrap();
    table(&mut out, HEADER_FIELDS, false);
    for word in plugins::words() {
        define(&mut out, word.doc, &word.name, word.offset);
    }
    out.push('\n');

    comment(
        &mut out,
        "An Image Control record. The records follow the header.",
    );
    writeln!(out, "#define GRKIT_RECORD_SIZE {RECORD_SIZE:#x}").unwrap();
    table(&mut out, RECORD_FIELDS, true);
    hue_tables(&mut out);
    define(
        &mut out,
        "The longest menu string, with its NUL: an Image Control summary is a \
         prefix and a 16-character effect name.",
        "SUMMARY_UNITS",
        menu_text::SUMMARY_UNITS,
    );
    // The icon slot differs between camera families, so the family header in
    // runtime/targets defines GRKIT_ICON_STRIDE.
    out.push('\n');

    define(
        &mut out,
        "Picture scratch after the records: the date stamp core and halo masks, \
         then the shot ring.",
        "PICTURE_MASKS",
        PICTURE_MASKS,
    );
    define(
        &mut out,
        "Shot ring: the record of each recent non-live-view parameter build, keyed \
         by its shot settings pointer, which the maker-note setter (+0x18) and the \
         JPEG encode configuration (+0x34) also hold. A later shot can start \
         processing before an earlier one is written, so a single \"last build\" \
         word is not enough. Word 0 counts the writes. Entry n is at GRKIT_SHOT_ENTRY \
         + GRKIT_SHOT_STRIDE n and holds the key at GRKIT_SHOT_KEY and the record at \
         GRKIT_SHOT_RECORD.",
        "SHOT_RING",
        PICTURE_MASKS,
    );
    for (name, value) in [
        ("ENTRIES", SHOT_ENTRIES),
        ("ENTRY", SHOT_ENTRY),
        ("STRIDE", SHOT_STRIDE),
        ("KEY", SHOT_KEY),
        ("RECORD", SHOT_RECORD),
    ] {
        writeln!(out, "#define GRKIT_SHOT_{name} {value}").unwrap();
    }
    writeln!(out, "#define GRKIT_PICTURE_SCRATCH {PICTURE_SCRATCH}").unwrap();
    out.push('\n');

    define(
        &mut out,
        "Menu text after the picture scratch, in each menu language: 0 English, 1 \
         Simplified Chinese, 2 Traditional Chinese, 3 Japanese. The installer copies the label \
         table of the build to offset 0: for each language, GRKIT_TEXT_STRINGS u16 \
         byte offsets from the text start, then NUL-terminated UTF-16 strings. At \
         GRKIT_TEXT_SUMMARY, the installer writes the Image Control summary of each \
         record in each language, GRKIT_SUMMARY_UNITS units each, language-major.",
        "TEXT_SIZE",
        menu_text::TEXT_SIZE,
    );
    define(
        &mut out,
        "Everything after the records: the script array must hold it.",
        "TAIL_SIZE",
        TAIL_SIZE,
    );
    writeln!(
        out,
        "#define GRKIT_TEXT_SUMMARY {}",
        menu_text::TEXT_SUMMARY
    )
    .unwrap();
    define(
        &mut out,
        "The parameter page title of a GRKit session, after the summaries: the \
         page, its vtable, and the descriptor of the effect name.",
        "TEXT_PARAMETER_TITLE",
        menu_text::TEXT_PARAMETER_TITLE,
    );
    define(
        &mut out,
        "The native settings page of a GRKit session, before the parameter page \
         title: the borrowed submenu entry, the text descriptors, and the views \
         that show settings (runtime/menu.c).",
        "TEXT_SETTINGS",
        menu_text::TEXT_SETTINGS,
    );
    writeln!(
        out,
        "#define GRKIT_TEXT_LANGUAGES {}",
        crate::packaging::Language::ALL.len()
    )
    .unwrap();
    writeln!(out, "#define GRKIT_TEXT_RECORDS {}", runtime::MAX_COUNT).unwrap();
    out.push_str(
        "#define GRKIT_TEXT(h) ((h)[GRKIT_HOST_RECORDS/4]+(h)[GRKIT_HOST_COUNT/4]*GRKIT_RECORD_SIZE+GRKIT_PICTURE_SCRATCH)\n",
    );
    comment(
        &mut out,
        "String IDs of the label table, in grkit_core::menu_text order. A run of \
         IDs follows the rows or values that it names.",
    );
    for (run, id) in menu_text::ids() {
        comment(&mut out, run.doc);
        writeln!(out, "#define TEXT_{} {id}", run.name).unwrap();
    }
    writeln!(out, "#define GRKIT_TEXT_STRINGS {}", menu_text::STRINGS).unwrap();
    out.push_str("\n#endif\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(fields: &[Field], size: usize, cover: bool) {
        let mut next = 0;
        for field in fields {
            assert!(field.size > 0, "{} is empty", field.name);
            if cover {
                assert_eq!(
                    field.offset, next,
                    "{} leaves a gap or overlaps",
                    field.name
                );
            } else {
                assert!(
                    field.offset >= next,
                    "{} overlaps the field before it",
                    field.name
                );
            }
            next = field.offset + field.size;
        }
        assert!(next <= size, "The fields end at {next}, after {size}");
        if cover {
            assert_eq!(next, size, "The fields leave the end of the header unnamed");
        }
    }

    #[test]
    fn every_header_byte_belongs_to_exactly_one_field() {
        assert!(HEADER_FIELDS
            .iter()
            .all(|field| field.offset % 4 == 0 && field.size % 4 == 0));
        check(HEADER_FIELDS, HOST_HEADER_SIZE, true);
    }

    #[test]
    fn plugin_words_fit_the_header() {
        let words = plugins::words();
        for (index, word) in words.iter().enumerate() {
            assert_eq!(word.offset, HOST_PLUGIN_AREA + 4 * index, "{}", word.name);
        }
        assert!(HOST_PLUGIN_AREA + 4 * words.len() <= HOST_HEADER_SIZE);
    }

    #[test]
    fn record_fields_do_not_overlap() {
        check(RECORD_FIELDS, RECORD_SIZE, false);
    }
}
