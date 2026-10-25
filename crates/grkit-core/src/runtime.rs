//! The private runtime layout and the production loader templates. The layout
//! values are those of GR III 2.10 unless a function takes the target.

use anyhow::{ensure, Result};
use serde_json::{json, Value};

use crate::{camera::Camera, Files};

pub use crate::abi::{
    HEADER_REFERENCE, HOST_ACTIVE, HOST_CAPABILITIES, HOST_COUNT, HOST_FRAME_RATIO,
    HOST_HEADER_SIZE as HEADER_SIZE, HOST_RECORDS, HOST_STATE_CHANGES, PICTURE_SCRATCH,
    RECORD_CODE, RECORD_PARAMETERS, RECORD_SIZE, TAIL_SIZE,
};
use crate::capabilities::{self, StateFile};

/// The icon slot of GR III: an 8-byte native descriptor, then 40 x 40 RGBA.
pub const ICON_STRIDE: usize = 6408;
/// The icon slot of each camera family. GR IV descriptors have 12 bytes.
/// The family header in `runtime/targets` defines the same value.
const FAMILY_ICON_STRIDES: [(&str, usize); 2] = [("gr3", ICON_STRIDE), ("gr4", 6412)];

/// The icon slot size of a runtime target that a camera release runs. The
/// slot belongs to the family, the target name before its first `-`.
pub fn icon_stride(target: &str) -> Result<usize> {
    let family = target.split_once('-').map(|(family, _)| family);
    match FAMILY_ICON_STRIDES
        .iter()
        .find(|(name, _)| Some(*name) == family)
    {
        Some(&(_, stride)) if crate::camera::CAMERAS.iter().any(|c| c.runs(target)) => Ok(stride),
        _ => anyhow::bail!("Unknown runtime target {target:?}"),
    }
}
pub const ICONS_PER_CHUNK: usize = 40;
/// The largest script array in words, which holds the modules.
pub const MODULE_ARRAY_WORDS: usize = 65536;
/// The most settings of all modules (`runtime/settings.h`).
pub const MODULE_SETTINGS_MAX: usize = 160;
/// The most values of a module setting.
pub const MODULE_SETTING_VALUES: usize = 65536;
/// The most values of a row of a native settings page (`SETTING_VALUES` in
/// runtime/core.h): one for each module that the Modules page lists.
pub const NATIVE_SETTING_VALUES: usize = 8;
/// The most settings rows of a module page (`GRM_SETTING_ROWS`). A page has 0,
/// or 2 to this many rows. The native settings page shows one row more
/// (`SETTING_ROWS` in `runtime/core.h`): in MENU, the Enable row of the
/// module comes first.
pub const NATIVE_SETTING_ROWS: usize = 5;
/// The most artwork tiles of all modules. The installer puts them in the
/// icon chunks after the effect icons.
pub const MODULE_TILES_MAX: usize = 64;
/// Off, GRKit, and Version.
/// The menu addresses the Version icon and the plugin tiles by offset, so the
/// host icons come first in chunk 0, and they must fit in it. The effect icons
/// follow them.
pub const HOST_ICON_COUNT: usize = 4;
const _: () = assert!(HOST_ICON_COUNT <= ICONS_PER_CHUNK);
/// Product limit for private Image Control effects. Each effect takes a
/// record of the registry array and an icon slot, about 10.5 KB of script
/// memory. The registry holds only the header and the records, so 40
/// records fit one array of 65536 words. The tail is an array of its own.
pub const MAX_COUNT: usize = 40;
/// The words of the tail array: the picture scratch, the menu text, and the
/// live-view labels (`abi::TAIL_SIZE`).
pub const TAIL_WORDS: usize = crate::abi::TAIL_SIZE / 4;
// One script array holds the header, every record, the scratch, and the text: at most 65536 words.
const _: () = assert!(HEADER_SIZE + MAX_COUNT * RECORD_SIZE <= 65536 * 4);
const _: () = assert!(TAIL_SIZE <= 65536 * 4);
const _: () = assert!(PICTURE_SCRATCH.is_multiple_of(4) && TAIL_SIZE.is_multiple_of(4));
/// One script array holds one icon chunk: 40 slots of at most 6412 bytes stay
/// below 65536 words.
pub const MAX_CHUNKS: usize =
    (MAX_COUNT + HOST_ICON_COUNT + MODULE_TILES_MAX).div_ceil(ICONS_PER_CHUNK);
/// The chunk addresses that the header holds (`GRKIT_HOST_ICON_CHUNKS`).
pub const ICON_CHUNK_SLOTS: usize = 5;
const _: () = assert!(MAX_CHUNKS <= ICON_CHUNK_SLOTS);
const _: () = assert!(ICONS_PER_CHUNK * 6412 <= 65536 * 4);
/// The data folder of GRKit on the card (`GRKIT`): the state files
/// (`capabilities::StateFile`), `REPORT.TXT`, the effects library in
/// `EFFECTS`, and the modules that a person adds in `MODULES`. No package
/// holds it, so a copy of a new package, by replace or by merge, keeps it.
/// The resident loader writes the state files through the camera's own file
/// system, never from Linux while the camera may write photos.
pub const DATA_FOLDER: &str = "C:\\GRKIT";
/// The package folder on the card: the loader, the core, the package list,
/// the package modules in `MODULES`, and the inbox of the package effects in
/// `EFFECTS`, which SCAN copies into the effects library.
pub const PACKAGE_FOLDER: &str = "SCRIPT/GRKIT";
/// The largest main state file that INSTALL reads (`STATE_TEXT_MAX` in
/// runtime/installer.c). It leaves room for keys of a later release.
pub const STATE_MAX_BYTES: usize = 16384;
/// The resident loader reads the selections at this interval. It writes the
/// state file when two reads in a row agree, so a change reaches the card
/// one to two intervals after the last change.
pub const STATE_POLL_MS: u32 = 2000;

/// A state file keeps the state keys of the capabilities
/// (`capabilities::state_keys`) and the saved module settings.
/// The file holds two lines for each key with a value other than 0: the key
/// ID (`state_key_id`), then the value, both in decimal. A missing key and a
/// plugin that is not installed restore 0.
/// The most digits of a number in the state file. The loader parses the digits
/// itself, and 9 digits cannot overflow its 32-bit integers.
pub const STATE_DIGITS: usize = 9;

/// The ID of a state key in the file, as for a module setting
/// (`grm::state_id`): the CRC-32 of its name, modulo 10^9, so it has at most
/// `STATE_DIGITS` digits, out of the range of the Fn adjustments and not 0.
pub fn state_key_id(name: &str) -> u32 {
    let mut id = crc32fast::hash(name.as_bytes()) % 1_000_000_000;
    if id >= PARAMETER_KEY_BASE {
        id -= 10_000_000;
    }
    id.max(1)
}

/// The Fn adjustments of an effect are a state entry of their own: the key ID
/// is this base plus the low 15 bits of the effect's ImageTone code, and the
/// value is the nine stored bytes (`RECORD_PARAMETERS`) as base-9 digits, the
/// first adjustment lowest. Neutral adjustments have no entry.
pub const PARAMETER_KEY_BASE: u32 = 990_000_000;
/// The largest packed adjustment value: 9^9 - 1.
pub const PARAMETER_VALUE_MAX: u32 = 387_420_488;
/// The number of adjustments of an effect.
pub const PARAMETER_COUNT: usize = 9;

/// The package folder on Linux: the installers, their scripts, the package
/// files, and the files of each boot.
pub const LINUX_DIRECTORY: &str = "/media/storage1/SCRIPT/GRKIT";
/// `DATA_FOLDER` on Linux.
pub const LINUX_DATA_DIRECTORY: &str = "/media/storage1/GRKIT";
/// The folder of GRKit 1.2 and earlier on Linux: its state file and its
/// effects. INSTALL reads them once, when `DATA_FOLDER` has no state file or
/// no effects library.
pub const LINUX_LEGACY_DIRECTORY: &str = "/media/storage1/SCRIPT/PLUGINS";

/// The state keys of GRKit 1.2 and the keys that now hold the same values.
/// The values keep their meaning, and the Fn adjustment entries keep their
/// key IDs.
pub const LEGACY_STATE_KEYS: [(&str, &str); 6] = [
    ("image-control.effect", "CORE.TONE"),
    ("framing.ratio", "CORE.ASPECT"),
    ("guides.style", "GUIDES.STYLE"),
    ("date-stamp.style", "DATE.STYLE"),
    ("date-stamp.look", "DATE.LOOK"),
    ("grain.level", "CORE.GRAIN"),
];

/// The ID of a GRKit 1.2 state key in its state file: the CRC-32 of its
/// name, modulo 10^9.
pub fn legacy_state_key_id(name: &str) -> u32 {
    crc32fast::hash(name.as_bytes()) % 1_000_000_000
}

/// The pairs of (GRKit 1.2 key ID, current key ID) of `LEGACY_STATE_KEYS`.
pub fn legacy_state_ids() -> Vec<(u32, u32)> {
    LEGACY_STATE_KEYS
        .iter()
        .map(|(old, new)| (legacy_state_key_id(old), state_key_id(new)))
        .collect()
}

pub fn layout(count: usize) -> Result<Value> {
    ensure!(count <= MAX_COUNT, "Expected 0..{MAX_COUNT} effects");
    let icon_words: Vec<_> = (0..count + HOST_ICON_COUNT)
        .step_by(ICONS_PER_CHUNK)
        .map(|start| ICONS_PER_CHUNK.min(count + HOST_ICON_COUNT - start) * ICON_STRIDE / 4)
        .collect();
    Ok(
        json!({"words": (HEADER_SIZE + count * RECORD_SIZE) / 4, "tail_words": TAIL_WORDS, "icon_words": icon_words}),
    )
}

/// The files that enable SD scripts when the camera starts with MENU held.
pub fn activation_files(camera: &Camera) -> Files {
    modeset_files(camera, b"[SCRIPT_EN_MODE EN]\r\n")
}

/// The files that turn SD scripts off again, in the same way. The firmware
/// has this command next to the one that enables them.
pub fn deactivation_files(camera: &Camera) -> Files {
    modeset_files(camera, b"[SCRIPT_EN_MODE DIS]\r\n")
}

fn modeset_files(camera: &Camera, command: &[u8]) -> Files {
    Files::from([
        (camera.modeset.into(), command.to_vec()),
        (
            "DEVELOP.MOD".into(),
            vec![7, 1, 44, 31, 16, 3, 30, 22, 5, 45],
        ),
    ])
}

/// Polls for Linux and camera readiness instead of waiting a fixed time.
/// `lcmd` requests are synchronous: each returns after the Linux command ends,
/// or with NG when Linux is not ready. The loader checks the result file after
/// each request and repeats it after a short pause.
///
/// The only fixed wait comes before `lcmd_resp on`. The RTOS builds its host
/// command tables on a worker task after the script starts, and the
/// `lcmd_resp` handler writes through a null pointer until they exist, which
/// stops the camera at power-on. The script cannot ask for the tables. A wait
/// of 1 s or more before `lcmd_resp on` avoids the fault, so the loader waits
/// 2 s.
pub fn loader() -> Vec<u8> {
    loader_for(ICON_STRIDE)
}

/// The loader for a target with `icon_stride`-byte icon slots.
pub fn loader_for(icon_stride: usize) -> Vec<u8> {
    let path = core_path;
    let (plan, done, failed) = (path("MODULE.BIN"), path("INSTALL.OK"), path("INSTALL.ERR"));
    // Wait for the host command tables of the RTOS before lcmd_resp.
    let mut lines: Vec<String> = vec!["mpause 2000".into(), "sendln 'lcmd_resp on'".into()];
    for name in [
        "MODULE.BIN",
        "MODULE.TMP",
        "INSTALL.OK",
        "INSTALL.ERR",
        "INSTALL.LCK",
    ] {
        lines.push(format!("filedelete '{}'", path(name)));
    }
    lines.extend([
        format!("filesearch '{plan}'"),
        "if result != 0 goto reject".into(),
        format!("filesearch '{done}'"),
        "if result != 0 goto reject".into(),
        // Early requests fail before Linux mounts the card. Give up after 150 attempts.
        "attempts = 0".into(),
        ":scan".into(),
        format!("sendln 'lcmd sh {LINUX_DIRECTORY}/SCAN.SH'"),
        format!("filesearch '{plan}'"),
        "if result != 0 goto plan_ready".into(),
        "mpause 200".into(),
        "attempts = attempts + 1".into(),
        "if attempts < 150 goto scan".into(),
        "goto reject".into(),
        ":plan_ready".into(),
        format!("fileopen fh '{plan}' 0"),
        "if fh < 0 goto reject".into(),
        "call readword".into(),
        "if value != 827150919 goto reject_close".into(),
        "call readword".into(),
        "count = value".into(),
        "if count < 0 goto reject_close".into(),
        format!("if count > {MAX_COUNT} goto reject_close"),
        "call readword".into(),
        "words = value".into(),
        "call readword".into(),
        "chunks = value".into(),
        // The plan size, then the script words of the modules.
        "call readword".into(),
        "call readword".into(),
        "modulewords = value".into(),
        "if modulewords < 0 goto reject_close".into(),
        format!("if modulewords > {MODULE_ARRAY_WORDS} goto reject_close"),
        "call readword".into(),
        "moduletiles = value".into(),
        "if moduletiles < 0 goto reject_close".into(),
        format!("if moduletiles > {MODULE_TILES_MAX} goto reject_close"),
        "fileclose fh".into(),
        format!(
            "expected = {} + count * {}",
            HEADER_SIZE / 4,
            RECORD_SIZE / 4
        ),
        "if words != expected goto reject".into(),
        format!(
            "expectedchunks = (count + moduletiles + {}) / {ICONS_PER_CHUNK}",
            HOST_ICON_COUNT + ICONS_PER_CHUNK - 1
        ),
        "if chunks != expectedchunks goto reject".into(),
        "intdim registry words".into(),
    ]);
    for i in 0..MAX_CHUNKS {
        lines.extend([
            format!("if chunks <= {i} goto icons_ready"),
            format!(
                "items = count + moduletiles + {HOST_ICON_COUNT} - {}",
                i * ICONS_PER_CHUNK
            ),
            format!("if items > {ICONS_PER_CHUNK} items = {ICONS_PER_CHUNK}"),
            format!("iconwords = items * {}", icon_stride / 4),
            format!("intdim artwork{i} iconwords"),
        ]);
    }
    lines.extend([
        ":icons_ready".into(),
        // The modules (runtime/module_link.h) follow the icon chunks.
        "if modulewords <= 0 goto modules_ready".into(),
        "intdim modulecode modulewords".into(),
        ":modules_ready".into(),
        // The tail: the picture scratch, the menu text, and the labels.
        format!("intdim tailarea {TAIL_WORDS}"),
        // Not-ready attempts release the install lock. Retry every 0.5 s for about
        // 30 s, then every 10 s, so a later shooting-mode start can still install.
        "attempts = 0".into(),
        ":install".into(),
        format!("sendln 'lcmd sh {LINUX_DIRECTORY}/INSTALL.SH'"),
        format!("filesearch '{done}'"),
        "if result != 0 goto resident".into(),
        format!("filesearch '{failed}'"),
        "if result != 0 goto reject".into(),
        "mpause 500".into(),
        "attempts = attempts + 1".into(),
        "if attempts > 60 mpause 9500".into(),
        "goto install".into(),
    ]);
    lines.extend(state_script());
    lines.extend(
        [
            ":reject_close",
            "fileclose fh",
            ":reject",
            "dispstr 'GR_MODULE_LOADER_REJECTED'",
            "end",
            ":readword",
            "value = 0",
            "weight = 1",
            "for byteindex 0 3",
            "fileread fh 1 octet",
            "if result != 0 goto reject_close",
            "str2code bytevalue octet",
            "value = value + bytevalue * weight",
            "if byteindex < 3 weight = weight * 256",
            "next",
            "return",
        ]
        .map(String::from),
    );
    (lines.join("\r\n") + "\r\n").into_bytes()
}

/// The resident part of the loader: it restores the selections, then saves them
/// after each change.
///
/// The private header is the `registry` array, so the script reads and writes
/// it directly. A runtime error ends the script, and the interpreter then frees
/// the arrays that the hooks use. So each array index is checked before use,
/// and no expression divides by a value from the card or from the header.
fn state_script() -> Vec<String> {
    let word = |offset: usize| format!("registry[{}]", offset / 4);
    let code_word = |index: &str| {
        format!(
            "registry[{} + {index} * {} + {}]",
            HEADER_SIZE / 4,
            RECORD_SIZE / 4,
            RECORD_CODE / 4
        )
    };
    let record_word = |index: &str, offset: usize| {
        format!(
            "registry[{} + {index} * {} + {}]",
            HEADER_SIZE / 4,
            RECORD_SIZE / 4,
            offset / 4
        )
    };
    let parameter_word =
        |index: &str, word: usize| record_word(index, RECORD_PARAMETERS + word * 4);
    // The record count for the adjustments, or 0 if it is out of range.
    let parameter_count = [
        format!("pcount = {}", word(HOST_COUNT)),
        "if pcount < 0 pcount = 0".to_string(),
        format!("if pcount > {MAX_COUNT} pcount = 0"),
    ];
    // The module settings (runtime/settings.h) in the `modulecode` array that
    // the loader allocates for the modules: the count, or 0 if it is out of
    // range for the array. Entry i has its ID at 2 + 3i, its value at 3 + 3i,
    // and its number of values at 4 + 3i.
    let module_count = [
        "mcount = 0".to_string(),
        "if modulewords < 2 goto module_count_end".into(),
        "mcount = modulecode[0]".into(),
        "mmax = (modulewords - 2) / 3".into(),
        "if mcount < 0 mcount = 0".into(),
        "if mcount > mmax mcount = 0".into(),
        format!("if mcount > {MODULE_SETTINGS_MAX} mcount = 0"),
        ":module_count_end".into(),
    ];
    let state_keys = capabilities::state_keys();
    let keys = state_keys.iter().enumerate();

    // INSTALL has restored the saved values (runtime/installer.c), so the
    // values at the start are those of the file. The loader writes only after
    // the next change: a value that the camera could not restore, such as
    // the effect of a missing file, keeps its entry until then. Without a
    // state file in the data folder, as at the first start after an update
    // from GRKit 1.2, INSTALL restored the old file, and the loader saves at
    // the first poll.
    //
    // The core counts each change of a saved selection in one header word
    // (GRKIT_STATE_CHANGED), so each poll reads that word only: the
    // interpreter spends thousands of instructions on each line, more with
    // each variable of the script. The loader saves when two polls in a row
    // read the same count and the file does not have it yet. A series of
    // changes, such as Fn adjustments, then writes the card once after its
    // last change.
    let changes = word(HOST_STATE_CHANGES);
    let mut lines: Vec<String> = vec![
        ":resident".into(),
        "stateok = 1".into(),
        format!("foldersearch '{DATA_FOLDER}'"),
        format!("if result = 0 foldercreate '{DATA_FOLDER}'"),
        format!("foldersearch '{DATA_FOLDER}'"),
        "if result = 0 stateok = 0".into(),
        format!("changes = {changes}"),
        "saved = changes".into(),
        "last = changes".into(),
        format!("filesearch '{}'", state_path(StateFile::Main, "TXT")),
        "if result = 0 saved = changes - 1".into(),
        ":watch".into(),
        format!("mpause {STATE_POLL_MS}"),
        format!("changes = {changes}"),
        "if changes <> last goto watch_moved".into(),
        "if changes = saved goto watch".into(),
        "stamp = changes".into(),
    ];
    for file in StateFile::ALL {
        lines.push(format!("call save_{}", label(file)));
    }
    lines.extend([
        "goto watch".into(),
        ":watch_moved".into(),
        "last = changes".into(),
        "goto watch".into(),
    ]);

    // Save each file after a change of one of its keys. `filerename` cannot
    // replace a file, and its result is 0 for success and failure, so each
    // step checks the files. If power fails between the steps, the restore
    // reads the temporary file. A key with the value 0 has no entry.
    for file in StateFile::ALL {
        let name = label(file);
        let (state, temporary) = (state_path(file, "TXT"), state_path(file, "TMP"));
        let main = file == StateFile::Main;
        let end = format!("save_{name}_end");
        let own: Vec<_> = keys.clone().filter(|(_, key)| key.file == file).collect();
        lines.extend([
            format!(":save_{name}"),
            format!("if stateok = 0 goto {end}"),
            "call snapshot".into(),
            format!("filedelete '{temporary}'"),
            format!("filecreate sfh '{temporary}'"),
            format!("if sfh < 0 goto {end}"),
        ]);
        for (i, key) in &own {
            lines.extend([
                format!("if value{i} = 0 goto save_key{i}"),
                format!("filewriteln sfh '{}'", state_key_id(&key.name)),
                format!("int2str line value{i}"),
                "filewriteln sfh line".into(),
                format!(":save_key{i}"),
            ]);
        }
        if main {
            // The adjustments of each effect that has a non-neutral one.
            lines.extend(parameter_count.clone());
            lines.extend([
                "pindex = 0".into(),
                ":save_parameters".into(),
                "if pindex >= pcount goto save_parameters_end".into(),
                format!("pw0 = {}", parameter_word("pindex", 0)),
                format!("pw1 = {}", parameter_word("pindex", 1)),
                format!("pw2 = {}", parameter_word("pindex", 2)),
                "pvalue = 0".into(),
            ]);
            for digit in (0..PARAMETER_COUNT).rev() {
                lines.extend([
                    format!("pdigit = (pw{} >> {}) and 255", digit / 4, digit % 4 * 8),
                    "if pdigit > 8 pdigit = 0".into(),
                    "pvalue = pvalue * 9 + pdigit".into(),
                ]);
            }
            lines.extend([
                "if pvalue = 0 goto save_parameters_next".into(),
                format!(
                    "pkey = {PARAMETER_KEY_BASE} + ({} and 32767)",
                    code_word("pindex")
                ),
                "int2str line pkey".into(),
                "filewriteln sfh line".into(),
                "int2str line pvalue".into(),
                "filewriteln sfh line".into(),
                ":save_parameters_next".into(),
                "pindex = pindex + 1".into(),
                "goto save_parameters".into(),
                ":save_parameters_end".into(),
            ]);
            // The saved module settings that are not 0.
            lines.extend(
                module_count
                    .iter()
                    .map(|line| line.replace("module_count_end", "save_module_count_end")),
            );
            lines.extend([
                "mindex = 0".into(),
                ":save_modules".into(),
                "if mindex >= mcount goto save_modules_end".into(),
                "mid = modulecode[2 + mindex * 3]".into(),
                "mvalue = modulecode[3 + mindex * 3]".into(),
                "if mid <= 0 goto save_modules_next".into(),
                "if mvalue <= 0 goto save_modules_next".into(),
                "int2str line mid".into(),
                "filewriteln sfh line".into(),
                "int2str line mvalue".into(),
                "filewriteln sfh line".into(),
                ":save_modules_next".into(),
                "mindex = mindex + 1".into(),
                "goto save_modules".into(),
                ":save_modules_end".into(),
            ]);
        }
        lines.extend([
            "fileclose sfh".into(),
            format!("filesearch '{temporary}'"),
            format!("if result = 0 goto {end}"),
            format!("filedelete '{state}'"),
            format!("filesearch '{state}'"),
            format!("if result != 0 goto {end}"),
            format!("filerename '{temporary}' '{state}'"),
            format!("filesearch '{state}'"),
            format!("if result = 0 goto {end}"),
        ]);
        lines.extend(["saved = stamp".into(), format!(":{end}"), "return".into()]);
    }

    // The current selections. The effect is the ImageTone code of the active
    // record, or 0 for Off and for any pointer that is not a record.
    lines.extend([
        ":snapshot".into(),
        "value0 = 0".into(),
        format!("count = {}", word(HOST_COUNT)),
        "if count < 1 goto snapshot_values".into(),
        format!("if count > {MAX_COUNT} goto snapshot_values"),
        format!("offset = {} - {}", word(HOST_ACTIVE), word(HOST_RECORDS)),
        "if offset < 0 goto snapshot_values".into(),
        format!("rest = offset % {RECORD_SIZE}"),
        "if rest != 0 goto snapshot_values".into(),
        format!("index = offset / {RECORD_SIZE}"),
        "if index >= count goto snapshot_values".into(),
        format!("value0 = {} and 65535", code_word("index")),
        ":snapshot_values".into(),
    ]);
    for (i, key) in keys.clone() {
        if let Some(offset) = key.word {
            lines.push(format!("value{i} = {}", word(offset)));
        }
    }
    lines.push("return".into());

    lines
}

/// The label stem of a state file in the loader.
fn label(file: StateFile) -> String {
    file.stem().to_ascii_lowercase()
}

/// The loader path of a state file with the extension `extension`.
fn state_path(file: StateFile, extension: &str) -> String {
    format!("{DATA_FOLDER}\\{}.{extension}", file.stem())
}

/// The loader path of a file of the package folder, `SCRIPT\GRKIT`.
fn core_path(name: &str) -> String {
    format!("C:\\SCRIPT\\GRKIT\\{name}")
}

/// The size of the largest file that the loader writes for `file`: each key
/// at its largest value, and for the main file, an adjustment entry for each
/// effect. Each line ends with CR LF.
pub fn worst_state_size(file: StateFile) -> usize {
    let line = |value: u32| value.to_string().len() + 2;
    let keys: usize = capabilities::state_keys()
        .iter()
        .filter(|key| key.file == file)
        .map(|key| line(state_key_id(&key.name)) + line(key.max))
        .sum();
    let parameters = match file {
        StateFile::Main => {
            MAX_COUNT * (line(PARAMETER_KEY_BASE + 32767) + line(PARAMETER_VALUE_MAX))
                + MODULE_SETTINGS_MAX
                    * (line(PARAMETER_KEY_BASE - 1) + line(MODULE_SETTING_VALUES as u32 - 1))
        }
    };
    keys + parameters
}

pub fn runtime_files(binary: &[u8]) -> Files {
    runtime_files_for(binary, ICON_STRIDE)
}

/// The runtime files of a target with `icon_stride`-byte icon slots.
pub fn runtime_files_for(binary: &[u8], icon_stride: usize) -> Files {
    package_runtime_files(&[binary], icon_stride)
}

/// The most installers that a package holds: `INSTALL.BIN`, then
/// `INSTALL2.BIN` to `INSTALL9.BIN`, so each name stays 8.3.
pub const MAX_INSTALLERS: usize = 9;

/// The path of installer `index` in the runtime files.
pub fn installer_path(index: usize) -> String {
    match index {
        0 => format!("{PACKAGE_FOLDER}/INSTALL.BIN"),
        _ => format!("{PACKAGE_FOLDER}/INSTALL{}.BIN", index + 1),
    }
}

/// The runtime files of a package with the installer of each runtime build of
/// a camera, in release order, for `icon_stride`-byte icon slots. The builds of
/// a camera have the same scripts and plan, so the first installer scans.
///
/// With one installer, `INSTALL.SH` runs `--install`. With more, it runs each
/// installer with `--known` first, which reads one word to find the build. If
/// no installer names the build, it tries each one with `--try`, which runs the
/// relocation, and the last one with `--install`, which writes `INSTALL.ERR`
/// if it also fails. A known build thus installs without a relocation of
/// another build, and an unknown build still gets each runtime.
///
/// After Linux deletes a file in the folder, the camera does not find the next
/// new name that it looks up there, and it keeps that miss. So `SCAN.SH`, the
/// first script of a power-on, looks up each installer before it scans.
pub fn package_runtime_files(binaries: &[&[u8]], icon_stride: usize) -> Files {
    assert!((1..=MAX_INSTALLERS).contains(&binaries.len()));
    let target = core_path("LOADER.TTL");
    let startup = format!("filesearch '{target}'\r\nif result = 0 goto done\r\ninclude '{target}'\r\n:done\r\nend\r\n");
    let name = |index: usize| installer_path(index).rsplit('/').next().unwrap().to_owned();
    // Each script body sets `status` to the exit status of the installer.
    let wrapper = |body: String, mode: &str| {
        format!(
            "{body}echo GR_MODULE_{}_EXIT_$status\nexit 0\n",
            mode.to_uppercase()
        )
        .into_bytes()
    };
    let single = |mode: &str| format!("{LINUX_DIRECTORY}/{} --{mode}\nstatus=$?\n", name(0));
    let names = |range: std::ops::Range<usize>| range.map(name).collect::<Vec<_>>().join(" ");
    let (scan, install) = match binaries.len() {
        1 => (single("scan"), single("install")),
        count => (
            format!(
                "for installer in {}; do [ -e {LINUX_DIRECTORY}/$installer ]; done\n{}",
                names(1..count),
                single("scan")
            ),
            format!(
                "for installer in {all}; do\n\
                 \x20 {LINUX_DIRECTORY}/$installer --known\n\
                 \x20 status=$?\n\
                 \x20 case $status in 48) ;; *) break ;; esac\n\
                 done\n\
                 case $status in 48)\n\
                 \x20 for installer in {tried}; do\n\
                 \x20   {LINUX_DIRECTORY}/$installer --try\n\
                 \x20   status=$?\n\
                 \x20   case $status in 47) ;; *) break ;; esac\n\
                 \x20 done\n\
                 \x20 case $status in 47) {LINUX_DIRECTORY}/{last} --install; status=$? ;; esac\n\
                 \x20 ;;\n\
                 esac\n",
                all = names(0..count),
                tried = names(0..count - 1),
                last = name(count - 1),
            ),
        ),
    };
    let mut files = Files::from([
        ("SCRIPT/STARTUP.TTL".into(), startup.into_bytes()),
        (
            format!("{PACKAGE_FOLDER}/LOADER.TTL"),
            loader_for(icon_stride),
        ),
        (format!("{PACKAGE_FOLDER}/SCAN.SH"), wrapper(scan, "scan")),
        (
            format!("{PACKAGE_FOLDER}/INSTALL.SH"),
            wrapper(install, "install"),
        ),
    ]);
    for (index, binary) in binaries.iter().enumerate() {
        files.insert(installer_path(index), binary.to_vec());
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_keys_have_distinct_positive_ids() {
        let keys = capabilities::state_keys();
        let mut ids: Vec<_> = keys.iter().map(|key| state_key_id(&key.name)).collect();
        assert!(ids
            .iter()
            .all(|id| *id > 0 && id.to_string().len() <= STATE_DIGITS));
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), keys.len());
        // The loader handles the effect of CORE.TONE, the one key without a
        // header word, as value 0.
        assert_eq!(keys[0].name, "CORE.TONE");
        assert!(keys[0].word.is_none());
        assert!(keys[1..].iter().all(|key| key.word.is_some()));
    }

    #[test]
    fn legacy_state_keys_map_to_the_current_keys() {
        let ids = legacy_state_ids();
        let mut old: Vec<_> = ids.iter().map(|(old, _)| *old).collect();
        old.sort_unstable();
        old.dedup();
        assert_eq!(old.len(), LEGACY_STATE_KEYS.len());
        // An old key is neither an Fn adjustment entry nor 0.
        assert!(old
            .iter()
            .all(|id| *id > 0 && !(PARAMETER_KEY_BASE..=PARAMETER_KEY_BASE + 32767).contains(id)));
        // The key IDs in the state files of GRKit 1.2.
        assert_eq!(
            old,
            [161811731, 495043802, 541741532, 684634731, 733922165, 957718554]
        );
        // Each target is a capability or a setting of an official module.
        let capabilities: Vec<_> = capabilities::state_keys()
            .iter()
            .map(|key| state_key_id(&key.name))
            .collect();
        let modules = [
            crate::grm::state_id("GUIDES", "STYLE"),
            crate::grm::state_id("DATE", "STYLE"),
            crate::grm::state_id("DATE", "LOOK"),
        ];
        for (_, new) in &ids {
            assert!(capabilities.contains(new) || modules.contains(new));
        }
    }

    /// A runtime error ends the script and frees the arrays, so the resident
    /// part must not index outside the header or divide by a variable.
    #[test]
    fn the_resident_script_cannot_stop_on_its_own_arithmetic() {
        let script = state_script().join("\n");
        let record = |index: &str, offset: usize| {
            format!(
                "{} + {index} * {} + {}",
                HEADER_SIZE / 4,
                RECORD_SIZE / 4,
                offset / 4
            )
        };
        // The computed indexes are words of a record below `count` or `pcount`:
        // its code, and the three words of its adjustments.
        let allowed = [
            record("index", RECORD_CODE),
            record("pindex", RECORD_CODE),
            record("pindex", RECORD_PARAMETERS),
            record("pindex", RECORD_PARAMETERS + 4),
            record("pindex", RECORD_PARAMETERS + 8),
        ];
        for (at, _) in script.match_indices("registry[") {
            let index = &script[at + 9..];
            let index = &index[..index.find(']').unwrap()];
            match index.parse::<usize>() {
                Ok(word) => assert!(word < HEADER_SIZE / 4, "registry[{word}]"),
                Err(_) => assert!(allowed.iter().any(|a| a == index), "registry[{index}]"),
            }
        }
        for line in script.lines() {
            for operator in [" / ", " % "] {
                if let Some(at) = line.find(operator) {
                    let divisor = line[at + 3..].split_whitespace().next().unwrap();
                    assert!(
                        divisor.parse::<u32>().is_ok_and(|value| value > 0),
                        "{line}"
                    );
                }
            }
        }
        // A `filereadln` of a last line without a newline blocks the interpreter.
        assert!(!script.contains("filereadln"));
        // Each computed record index is below `count`, which is at most MAX_COUNT.
        assert!(script.contains(&format!("if count > {MAX_COUNT} goto snapshot_values")));
        assert!(script.contains("if pcount < 0 pcount = 0"));
        assert!(script.contains(&format!("if pcount > {MAX_COUNT} pcount = 0")));
        assert_eq!(script.matches("if pindex >= pcount goto").count(), 1);
    }

    /// A series of changes writes the card once: each poll reads only the
    /// change count of the core, and the watch loop calls the saves only when
    /// two polls in a row read the same count and the file does not have it.
    /// The start sets each variable that the loop reads, because an unset
    /// variable is a runtime error.
    #[test]
    fn the_resident_script_saves_after_the_selections_settle() {
        let script = state_script();
        let at = |line: &str| {
            script
                .iter()
                .position(|l| l == line)
                .unwrap_or_else(|| panic!("{line}"))
        };
        let read = format!("changes = registry[{}]", HOST_STATE_CHANGES / 4);
        let (start, watch) = (at(":resident"), at(":watch"));
        let poll = at(&format!("mpause {STATE_POLL_MS}"));
        assert_eq!(script[poll + 1], read);
        let moved = at("if changes <> last goto watch_moved");
        let settled = at("if changes = saved goto watch");
        assert!(watch < poll && poll < moved && moved < settled);
        assert_eq!(script[settled + 1], "stamp = changes");
        for file in StateFile::ALL {
            assert!(settled < at(&format!("call save_{}", label(file))));
        }
        // An idle poll runs four lines: the pause, the read, two compares.
        assert_eq!(settled - poll + 1, 4);
        assert_eq!(script[at(":watch_moved") + 1], "last = changes");
        for name in ["last", "saved"] {
            assert!(script[start..watch]
                .iter()
                .any(|l| l.starts_with(&format!("{name} ="))));
        }
        assert!(script[start..watch].contains(&read));
        // A save reads the selections, and records the count that it saved.
        for file in StateFile::ALL {
            let save = at(&format!(":save_{}", label(file)));
            assert_eq!(script[save + 2], "call snapshot");
        }
        assert!(script.iter().any(|l| l == "saved = stamp"));
    }

    #[test]
    fn adjustment_entries_fit_the_state_file() {
        // Nine base-9 digits, and keys of at most nine digits that no plugin
        // key uses.
        assert_eq!(PARAMETER_VALUE_MAX, 9u32.pow(PARAMETER_COUNT as u32) - 1);
        assert_eq!(
            PARAMETER_COUNT,
            crate::abi::RECORD_FIELDS
                .iter()
                .find(|field| field.name == "RECORD_PARAMETERS")
                .unwrap()
                .size
        );
        let last = PARAMETER_KEY_BASE + 32767;
        assert!(last.to_string().len() <= STATE_DIGITS);
        assert!(capabilities::state_keys().iter().all(|key| {
            let id = state_key_id(&key.name);
            !(PARAMETER_KEY_BASE..=last).contains(&id)
        }));
        // Each key and value line, with CR LF, for every effect, plugin key,
        // and module setting.
        let line = STATE_DIGITS + 2;
        assert!(
            worst_state_size(StateFile::Main)
                <= 2 * line * (MAX_COUNT + capabilities::state_keys().len() + MODULE_SETTINGS_MAX)
        );
        // Module setting IDs stay below the adjustment entries, and values
        // fit the digits that the loader parses.
        assert!(((MODULE_SETTING_VALUES - 1).to_string().len()) <= STATE_DIGITS);
        assert!(crate::grm::state_id("GUIDES", "STYLE") < PARAMETER_KEY_BASE);
    }

    /// The installer, the module SDK, and the menu agree on the values of a
    /// native settings row.
    #[test]
    fn native_setting_rows_have_one_value_limit() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime");
        for (file, name) in [
            ("module_link.h", "GRM_SETTING_ROW_VALUES"),
            ("sdk/grkit.h", "GRKIT_SETTING_ROW_VALUES"),
            ("core.h", "SETTING_VALUES"),
        ] {
            let text = std::fs::read_to_string(root.join(file)).unwrap();
            let value = text
                .lines()
                .find_map(|line| line.strip_prefix(&format!("#define {name} ")))
                .and_then(|value| value.trim_end_matches('U').parse::<usize>().ok());
            assert_eq!(value, Some(NATIVE_SETTING_VALUES), "{file}");
        }
        for (file, name, rows) in [
            ("module_link.h", "GRM_SETTING_ROWS", NATIVE_SETTING_ROWS),
            ("sdk/grkit.h", "GRKIT_SETTING_ROWS", NATIVE_SETTING_ROWS),
            ("core.h", "SETTING_ROWS", NATIVE_SETTING_ROWS + 1),
        ] {
            let text = std::fs::read_to_string(root.join(file)).unwrap();
            let value = text
                .lines()
                .find_map(|line| line.strip_prefix(&format!("#define {name} ")))
                .and_then(|value| value.trim_end_matches('U').parse::<usize>().ok());
            assert_eq!(value, Some(rows), "{file}");
        }
    }

    /// INSTALL reads the largest file that the loader writes: its bytes and
    /// its pairs (runtime/state.h).
    #[test]
    fn the_read_limit_covers_the_largest_state_file() {
        for file in StateFile::ALL {
            assert!(worst_state_size(file) <= STATE_MAX_BYTES, "{file:?}");
        }
        let header = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime/state.h"),
        )
        .unwrap();
        let pairs: usize = header
            .lines()
            .find_map(|line| line.strip_prefix("#define STATE_PAIRS "))
            .and_then(|value| value.trim_end_matches('U').parse().ok())
            .unwrap();
        assert!(MAX_COUNT + capabilities::state_keys().len() + MODULE_SETTINGS_MAX <= pairs);
    }

    fn at(script: &[String], line: &str) -> usize {
        script
            .iter()
            .position(|l| l == line)
            .unwrap_or_else(|| panic!("{line}"))
    }

    /// The state files are in the data folder, which holds the loader. Without
    /// the folder, no save writes.
    #[test]
    fn the_loader_saves_in_the_data_folder() {
        let script = state_script();
        let check = at(&script, "if result = 0 stateok = 0");
        assert_eq!(script[check - 1], "foldersearch 'C:\\GRKIT'");
        assert!(check < at(&script, ":watch"));
        for file in StateFile::ALL {
            let save = at(&script, &format!(":save_{}", label(file)));
            assert_eq!(
                script[save + 1],
                format!("if stateok = 0 goto save_{}_end", label(file))
            );
        }
        let writes: Vec<_> = script
            .iter()
            .filter(|l| l.starts_with("filecreate") || l.starts_with("filerename"))
            .collect();
        assert_eq!(writes.len(), 2 * StateFile::ALL.len());
        assert!(
            writes.iter().all(|l| l.contains("C:\\GRKIT\\")),
            "{writes:?}"
        );
    }

    /// INSTALL restores the state before the hooks start (runtime/state.h),
    /// so the loader only writes the file. Its start is the restored values.
    #[test]
    fn the_loader_reads_no_state_file() {
        let script = loader();
        let script = String::from_utf8_lossy(&script);
        let opens: Vec<_> = script
            .lines()
            .filter(|l| l.starts_with("fileopen"))
            .collect();
        assert!(opens.iter().all(|l| l.contains("MODULE.BIN")), "{opens:?}");
        assert!(!script.contains("SCRIPT\\PLUGINS"));
        let state = state_script();
        let start = at(&state, "saved = changes");
        assert!(start < at(&state, ":watch"));
    }

    /// INSTALL restores the state of GRKit 1.2 when the data folder has no
    /// state file, so the loader then saves at the first poll.
    #[test]
    fn the_loader_saves_at_once_without_a_state_file() {
        let state = state_script();
        let force = at(&state, "if result = 0 saved = changes - 1");
        assert_eq!(state[force - 1], "filesearch 'C:\\GRKIT\\STATE.TXT'");
        assert!(at(&state, "saved = changes") < force);
        assert!(force < at(&state, ":watch"));
    }

    /// A key with the value 0 has no entry, because a missing key restores 0.
    #[test]
    fn the_loader_writes_only_keys_with_a_value() {
        let script = state_script();
        for (i, key) in capabilities::state_keys().iter().enumerate() {
            let write = at(
                &script,
                &format!("filewriteln sfh '{}'", state_key_id(&key.name)),
            );
            assert_eq!(
                script[write - 1],
                format!("if value{i} = 0 goto save_key{i}")
            );
            assert_eq!(script[write + 3], format!(":save_key{i}"));
        }
    }
}
