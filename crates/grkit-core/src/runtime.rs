//! The private runtime layout and the production loader templates. The layout
//! values are those of GR III 2.10 unless a function takes the target.

use anyhow::{ensure, Result};
use serde_json::{json, Value};

use crate::{camera::Camera, Files};

pub use crate::abi::{
    HEADER_REFERENCE, HOST_ACTIVE, HOST_COUNT, HOST_FRAME_RATIO, HOST_GUIDE,
    HOST_HEADER_SIZE as HEADER_SIZE, HOST_PARAMETER_CHANGES, HOST_PLUGINS, HOST_RECORDS,
    HOST_STAMP, PICTURE_SCRATCH, RECORD_CODE, RECORD_PARAMETERS, RECORD_SIZE, TAIL_SIZE,
};
use crate::plugins;

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
/// Off, GRKit, Version, then the tiles of each plugin (`plugins::TILES`).
/// The menu addresses the Version icon and the plugin tiles by offset, so the
/// host icons come first in chunk 0, and they must fit in it. The effect icons
/// follow them.
pub const HOST_ICON_COUNT: usize = 3 + plugins::TILES;
const _: () = assert!(HOST_ICON_COUNT <= ICONS_PER_CHUNK);
/// Product limit for private Image Control effects.
pub const MAX_COUNT: usize = 20;
// One script array holds the header, every record, the scratch, and the text: at most 65536 words.
const _: () = assert!(HEADER_SIZE + MAX_COUNT * RECORD_SIZE + TAIL_SIZE <= 65536 * 4);
const _: () = assert!(PICTURE_SCRATCH.is_multiple_of(4) && TAIL_SIZE.is_multiple_of(4));
/// One script array holds one icon chunk: 40 slots of at most 6412 bytes stay
/// below 65536 words.
pub const MAX_CHUNKS: usize = (MAX_COUNT + HOST_ICON_COUNT).div_ceil(ICONS_PER_CHUNK);
/// The chunk addresses that the header holds (`GRKIT_HOST_ICON_CHUNKS`).
pub const ICON_CHUNK_SLOTS: usize = 5;
const _: () = assert!(MAX_CHUNKS <= ICON_CHUNK_SLOTS);
const _: () = assert!(ICONS_PER_CHUNK * 6412 <= 65536 * 4);
/// Selections kept across power-off, written by the resident loader through the
/// camera's own file system, never from Linux while the camera may write photos.
pub const STATE_FILE: &str = "STATE.TXT";
/// The new state file before it replaces `STATE_FILE`.
pub const STATE_TEMPORARY: &str = "STATE.TMP";
/// The largest state file that the loader reads.
pub const STATE_MAX_BYTES: usize = 1024;
/// The resident loader reads the selections at this interval. It writes the
/// state file when two reads in a row agree, so a change reaches the card
/// one to two intervals after the last change.
pub const STATE_POLL_MS: u32 = 2000;

/// `STATE.TXT` keeps the state keys of the plugins (`plugins::state_keys`).
/// The file holds two lines for each key: the key ID (`state_key_id`), then
/// the value, both in decimal. A plugin that is not installed restores 0.
/// The most digits of a number in the state file. The loader parses the digits
/// itself, and 9 digits cannot overflow its 32-bit integers.
pub const STATE_DIGITS: usize = 9;

/// The ID of a state key in the file: the CRC-32 of its name, modulo 10^9, so
/// it has at most `STATE_DIGITS` digits.
pub fn state_key_id(name: &str) -> u32 {
    crc32fast::hash(name.as_bytes()) % 1_000_000_000
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

pub const LINUX_DIRECTORY: &str = "/media/storage1/SCRIPT/PLUGINS";

pub fn layout(count: usize) -> Result<Value> {
    ensure!(count <= MAX_COUNT, "Expected 0..{MAX_COUNT} effects");
    let icon_words: Vec<_> = (0..count + HOST_ICON_COUNT)
        .step_by(ICONS_PER_CHUNK)
        .map(|start| ICONS_PER_CHUNK.min(count + HOST_ICON_COUNT - start) * ICON_STRIDE / 4)
        .collect();
    Ok(
        json!({"words": (HEADER_SIZE + count * RECORD_SIZE + TAIL_SIZE) / 4, "icon_words": icon_words}),
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
    let path = |name: &str| format!("C:\\SCRIPT\\PLUGINS\\{name}");
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
        "fileclose fh".into(),
        format!(
            "expected = {} + count * {}",
            (HEADER_SIZE + TAIL_SIZE) / 4,
            RECORD_SIZE / 4
        ),
        "if words != expected goto reject".into(),
        format!(
            "expectedchunks = (count + {}) / {ICONS_PER_CHUNK}",
            HOST_ICON_COUNT + ICONS_PER_CHUNK - 1
        ),
        "if chunks != expectedchunks goto reject".into(),
        "intdim registry words".into(),
    ]);
    for i in 0..MAX_CHUNKS {
        lines.extend([
            format!("if chunks <= {i} goto icons_ready"),
            format!(
                "items = count + {HOST_ICON_COUNT} - {}",
                i * ICONS_PER_CHUNK
            ),
            format!("if items > {ICONS_PER_CHUNK} items = {ICONS_PER_CHUNK}"),
            format!("iconwords = items * {}", icon_stride / 4),
            format!("intdim artwork{i} iconwords"),
        ]);
    }
    lines.extend([
        ":icons_ready".into(),
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
    let state = format!("C:\\SCRIPT\\PLUGINS\\{STATE_FILE}");
    let temporary = format!("C:\\SCRIPT\\PLUGINS\\{STATE_TEMPORARY}");
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
    let state_keys = plugins::state_keys();
    let keys = state_keys.iter().enumerate();
    let mut lines: Vec<String> = vec![":resident".into(), "call restore".into()];

    // Save after a change, when two polls in a row read the same selections.
    // A series of changes, such as Fn adjustments, then writes the card once
    // after its last change. `filerename` cannot replace a file, and its
    // result is 0 for success and failure, so each step checks the files. If
    // power fails between the steps, the restore reads STATE.TMP.
    lines.extend([
        ":watch".into(),
        format!("mpause {STATE_POLL_MS}"),
        "call snapshot".into(),
        "call save".into(),
        "goto watch".into(),
        ":save".into(),
        "stable = 1".into(),
    ]);
    for (i, _) in keys.clone() {
        lines.extend([
            format!("if value{i} <> last{i} stable = 0"),
            format!("last{i} = value{i}"),
        ]);
    }
    lines.extend([
        "if pchanges <> plast stable = 0".into(),
        "plast = pchanges".into(),
        "if stable = 0 goto save_end".into(),
        "changed = 0".into(),
    ]);
    for (i, _) in keys.clone() {
        lines.push(format!("if value{i} <> saved{i} changed = 1"));
    }
    lines.push("if pchanges <> psaved changed = 1".into());
    lines.extend([
        "if changed = 0 goto save_end".into(),
        format!("filedelete '{temporary}'"),
        format!("filecreate sfh '{temporary}'"),
        "if sfh < 0 goto save_end".into(),
    ]);
    for (i, key) in keys.clone() {
        lines.extend([
            format!("filewriteln sfh '{}'", state_key_id(&key.name)),
            format!("int2str line value{i}"),
            "filewriteln sfh line".into(),
        ]);
    }
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
    lines.extend([
        "fileclose sfh".into(),
        format!("filesearch '{temporary}'"),
        "if result = 0 goto save_end".into(),
        format!("filedelete '{state}'"),
        format!("filesearch '{state}'"),
        "if result != 0 goto save_end".into(),
        format!("filerename '{temporary}' '{state}'"),
        format!("filesearch '{state}'"),
        "if result = 0 goto save_end".into(),
    ]);
    for (i, _) in keys.clone() {
        lines.push(format!("saved{i} = value{i}"));
    }
    lines.push("psaved = pchanges".into());
    lines.extend([":save_end".into(), "return".into()]);

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
    // The menu counts each adjustment change.
    lines.push(format!("pchanges = {}", word(HOST_PARAMETER_CHANGES)));
    lines.push("return".into());

    // Restore. A missing, empty, or too large file restores nothing. `fileopen`
    // creates a missing file, so the loader searches for it first.
    //
    // A read past the end of the file blocks the interpreter, and so does a
    // `filereadln` of a last line without a newline. The loader therefore reads
    // the file byte by byte, up to the size that `filestat` gives, and parses
    // the decimal lines itself. CR and LF end a line. A line with another
    // character, or with more than 9 digits, is invalid. Lines form key and
    // value pairs. A pair with an invalid line, an unknown key, or an
    // out-of-range value is skipped.
    lines.push(":restore".into());
    for (i, _) in keys.clone() {
        lines.push(format!("stored{i} = 0"));
    }
    lines.extend(parameter_count);
    lines.extend([
        format!("statefile = '{state}'"),
        "filesearch statefile".into(),
        "if result != 0 goto restore_open".into(),
        format!("statefile = '{temporary}'"),
        "filesearch statefile".into(),
        "if result = 0 goto restore_apply".into(),
        ":restore_open".into(),
        "size = 0".into(),
        "filestat statefile size".into(),
        "if size < 1 goto restore_apply".into(),
        format!("if size > {STATE_MAX_BYTES} goto restore_apply"),
        "fileopen sfh statefile 0".into(),
        "if sfh < 0 goto restore_apply".into(),
        "position = 0".into(),
        "ended = 0".into(),
        "expecting = 0".into(),
        "key = 0".into(),
        "keyinvalid = 0".into(),
        "token = 0".into(),
        "digits = 0".into(),
        "invalid = 0".into(),
        ":restore_byte".into(),
        "if position >= size goto restore_last".into(),
        "fileread sfh 1 octet".into(),
        "if result != 0 goto restore_close".into(),
        "position = position + 1".into(),
        "str2code character octet".into(),
        "if character = 13 goto restore_token".into(),
        "if character = 10 goto restore_token".into(),
        "if character < 48 invalid = 1".into(),
        "if character > 57 invalid = 1".into(),
        format!("if digits >= {STATE_DIGITS} invalid = 1"),
        "if invalid = 1 goto restore_byte".into(),
        "token = token * 10 + character - 48".into(),
        "digits = digits + 1".into(),
        "goto restore_byte".into(),
        ":restore_last".into(),
        "ended = 1".into(),
        // A line ends. Empty lines, as between CR and LF, are not tokens.
        ":restore_token".into(),
        "filled = digits + invalid".into(),
        "if filled = 0 goto restore_continue".into(),
        "if expecting = 1 goto restore_value".into(),
        "key = token".into(),
        "keyinvalid = invalid".into(),
        "expecting = 1".into(),
        "goto restore_reset".into(),
        ":restore_value".into(),
        "expecting = 0".into(),
        "bad = keyinvalid + invalid".into(),
        "if bad != 0 goto restore_reset".into(),
    ]);
    for (i, key) in keys.clone() {
        lines.push(format!(
            "if key = {} goto restore_key{i}",
            state_key_id(&key.name)
        ));
    }
    // An adjustment entry applies to each installed effect with its code. An
    // effect that is not installed keeps nothing.
    lines.extend([
        format!("if key < {PARAMETER_KEY_BASE} goto restore_reset"),
        format!("if key > {} goto restore_reset", PARAMETER_KEY_BASE + 32767),
        format!("if token > {PARAMETER_VALUE_MAX} goto restore_reset"),
        format!("pcode = key - {PARAMETER_KEY_BASE}"),
        "pindex = 0".into(),
        ":restore_parameters".into(),
        "if pindex >= pcount goto restore_reset".into(),
        format!("pkey = {} and 32767", code_word("pindex")),
        "if pkey != pcode goto restore_parameters_next".into(),
        "pvalue = token".into(),
    ]);
    for digit in 0..PARAMETER_COUNT {
        lines.extend([
            format!("pd{digit} = pvalue % 9"),
            "pvalue = pvalue / 9".into(),
        ]);
    }
    lines.extend([
        format!(
            "{} = pd0 + pd1 * 256 + pd2 * 65536 + pd3 * 16777216",
            parameter_word("pindex", 0)
        ),
        format!(
            "{} = pd4 + pd5 * 256 + pd6 * 65536 + pd7 * 16777216",
            parameter_word("pindex", 1)
        ),
        // Bytes after the ninth adjustment are not part of the record fields.
        format!("pw2 = {} >> 8", parameter_word("pindex", 2)),
        format!("{} = pw2 * 256 + pd8", parameter_word("pindex", 2)),
        ":restore_parameters_next".into(),
        "pindex = pindex + 1".into(),
        "goto restore_parameters".into(),
    ]);
    for (i, key) in keys.clone() {
        lines.extend([
            format!(":restore_key{i}"),
            format!("if token > {} goto restore_reset", key.max),
            format!("stored{i} = token"),
            "goto restore_reset".into(),
        ]);
    }
    lines.extend([
        ":restore_reset".into(),
        "token = 0".into(),
        "digits = 0".into(),
        "invalid = 0".into(),
        ":restore_continue".into(),
        "if ended = 0 goto restore_byte".into(),
    ]);
    lines.extend([
        ":restore_close".into(),
        "fileclose sfh".into(),
        ":restore_apply".into(),
        // A plugin that is not installed has no menu to turn it off, so it
        // restores 0.
        format!("plugins = {}", word(HOST_PLUGINS)),
    ]);
    for (i, key) in keys.clone() {
        lines.extend([
            format!("bit = (plugins >> {}) and 1", key.plugin),
            format!("if bit = 0 stored{i} = 0"),
        ]);
        if let Some(offset) = key.word {
            lines.push(format!("{} = stored{i}", word(offset)));
        }
    }
    // An effect that is no longer installed stays Off.
    lines.extend([
        format!("count = {}", word(HOST_COUNT)),
        format!("if count > {MAX_COUNT} count = 0"),
        "index = 0".into(),
        ":restore_find".into(),
        "if stored0 = 0 goto restore_end".into(),
        "if index >= count goto restore_end".into(),
        format!("code = {} and 65535", code_word("index")),
        "if code != stored0 goto restore_next".into(),
        format!(
            "{} = {} + index * {RECORD_SIZE}",
            word(HOST_ACTIVE),
            word(HOST_RECORDS)
        ),
        "goto restore_end".into(),
        ":restore_next".into(),
        "index = index + 1".into(),
        "goto restore_find".into(),
        // Write only after the next change, so a missing effect keeps its file
        // entry until the user selects something else.
        ":restore_end".into(),
        "call snapshot".into(),
        "psaved = pchanges".into(),
        "plast = pchanges".into(),
    ]);
    // `save` reads `last` and `saved`: an unset variable is a runtime error.
    for (i, _) in keys {
        lines.extend([
            format!("saved{i} = value{i}"),
            format!("last{i} = value{i}"),
        ]);
    }
    lines.push("return".into());
    lines
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
        0 => "SCRIPT/PLUGINS/INSTALL.BIN".into(),
        _ => format!("SCRIPT/PLUGINS/INSTALL{}.BIN", index + 1),
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
    let target = "C:\\SCRIPT\\PLUGINS\\PLUGINS.TTL";
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
        ("SCRIPT/PLUGINS/PLUGINS.TTL".into(), loader_for(icon_stride)),
        ("SCRIPT/PLUGINS/SCAN.SH".into(), wrapper(scan, "scan")),
        (
            "SCRIPT/PLUGINS/INSTALL.SH".into(),
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
        let keys = plugins::state_keys();
        let mut ids: Vec<_> = keys.iter().map(|key| state_key_id(&key.name)).collect();
        assert!(ids
            .iter()
            .all(|id| *id > 0 && id.to_string().len() <= STATE_DIGITS));
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), keys.len());
        // The loader handles the Image Control effect, the one key without a
        // header word, as value 0.
        assert_eq!(keys[0].name, "image-control.effect");
        assert!(keys[0].word.is_none());
        assert!(keys[1..].iter().all(|key| key.word.is_some()));
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
        assert!(script.contains(&format!("if count > {MAX_COUNT} count = 0")));
        assert!(script.contains("if pcount < 0 pcount = 0"));
        assert!(script.contains(&format!("if pcount > {MAX_COUNT} pcount = 0")));
        assert_eq!(script.matches("if pindex >= pcount goto").count(), 2);
    }

    /// A series of changes writes the card once: `save` writes only when two
    /// polls in a row read the same selections. Restore sets each variable
    /// that `save` reads, because an unset variable is a runtime error.
    #[test]
    fn the_resident_script_saves_after_the_selections_settle() {
        let script = state_script();
        let at = |line: &str| {
            script
                .iter()
                .position(|l| l == line)
                .unwrap_or_else(|| panic!("{line}"))
        };
        let save = at(":save");
        let settle = at("if stable = 0 goto save_end");
        let write = at("filecreate sfh 'C:\\SCRIPT\\PLUGINS\\STATE.TMP'");
        assert!(save < settle && settle < at("changed = 0") && settle < write);
        assert!(at(&format!("mpause {STATE_POLL_MS}")) < save);
        let restored = at(":restore_end");
        let mut read = vec!["plast".to_string()];
        read.extend((0..plugins::state_keys().len()).map(|i| format!("last{i}")));
        for name in read {
            let compare = script[save..settle]
                .iter()
                .position(|l| l.contains(&format!("<> {name} ")))
                .unwrap_or_else(|| panic!("{name}"));
            assert!(script[save..save + compare]
                .iter()
                .all(|l| !l.starts_with(&format!("{name} ="))));
            assert!(script[restored..]
                .iter()
                .any(|l| l.starts_with(&format!("{name} ="))));
        }
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
        assert!(plugins::state_keys().iter().all(|key| {
            let id = state_key_id(&key.name);
            !(PARAMETER_KEY_BASE..=last).contains(&id)
        }));
        // Each key and value line, with CR LF, for every effect and plugin key.
        let line = STATE_DIGITS + 2;
        assert!(2 * line * (MAX_COUNT + plugins::state_keys().len()) <= STATE_MAX_BYTES);
    }
}
