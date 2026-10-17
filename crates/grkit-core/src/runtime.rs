//! The private runtime layout and the production loader templates. The layout
//! values are those of GR III 2.10 unless a function takes the target.

use anyhow::{ensure, Result};
use serde_json::{json, Value};

use crate::{camera::Camera, Files};

pub use crate::abi::{
    HEADER_REFERENCE, HOST_ACTIVE, HOST_COUNT, HOST_FRAME_RATIO, HOST_GRAIN, HOST_GUIDE,
    HOST_HEADER_SIZE as HEADER_SIZE, HOST_PLUGINS, HOST_RECORDS, HOST_STAMP, PICTURE_SCRATCH,
    RECORD_CODE, RECORD_SIZE, TAIL_SIZE,
};

/// The icon slot of GR III 2.10: an 8-byte native descriptor, then 40 x 40 RGBA.
pub const ICON_STRIDE: usize = 6408;

/// The icon slot size of a runtime target. GR IV 1.11 has 12-byte descriptors.
pub fn icon_stride(target: &str) -> Result<usize> {
    match target {
        "gr3-2.10" => Ok(ICON_STRIDE),
        "gr4-1.11" => Ok(6412),
        _ => anyhow::bail!("Unknown runtime target {target:?}"),
    }
}
pub const ICONS_PER_CHUNK: usize = 40;
/// Off, GRKit, four Framing tiles, five Guides tiles, five Date Stamp tiles, and
/// four Grain tiles.
/// The menu addresses module tiles by offset, so they must stay contiguous in
/// one icon chunk.
pub const HOST_ICON_COUNT: usize = 20;
/// Product limit for private Image Control effects.
pub const MAX_COUNT: usize = 20;
// One script array holds the header, every record, the scratch, and the text: at most 65536 words.
const _: () = assert!(HEADER_SIZE + MAX_COUNT * RECORD_SIZE + TAIL_SIZE <= 65536 * 4);
const _: () = assert!(PICTURE_SCRATCH.is_multiple_of(4) && TAIL_SIZE.is_multiple_of(4));
pub const MAX_CHUNKS: usize = (MAX_COUNT + HOST_ICON_COUNT).div_ceil(ICONS_PER_CHUNK);
const _: () = assert!(MAX_COUNT + HOST_ICON_COUNT <= ICONS_PER_CHUNK);
/// Selections kept across power-off, written by the resident loader through the
/// camera's own file system, never from Linux while the camera may write photos.
pub const STATE_FILE: &str = "STATE.TXT";
/// The new state file before it replaces `STATE_FILE`.
pub const STATE_TEMPORARY: &str = "STATE.TMP";
/// The largest state file that the loader reads.
pub const STATE_MAX_BYTES: usize = 512;

/// A selection that `STATE.TXT` keeps. The file holds two lines for each key:
/// the key ID (`state_key_id`), then the value, both in decimal.
pub struct StateKey {
    /// `<module>.<key>`.
    pub name: &'static str,
    /// The header word that holds the value, or `None` for the Image Control
    /// effect, which the file keeps as the ImageTone code of the record.
    pub word: Option<usize>,
    pub max: u32,
    /// The bit of the plugin in `HOST_PLUGINS`. A plugin that is not
    /// installed restores 0.
    pub module: u32,
}

pub const STATE_KEYS: &[StateKey] = &[
    StateKey {
        name: "image-control.effect",
        word: None,
        max: 0xffff,
        module: 0,
    },
    StateKey {
        name: "framing.ratio",
        word: Some(HOST_FRAME_RATIO),
        max: 3,
        module: 1,
    },
    StateKey {
        name: "guides.style",
        word: Some(HOST_GUIDE),
        max: 4,
        module: 2,
    },
    StateKey {
        name: "date-stamp.style",
        word: Some(HOST_STAMP),
        max: 4,
        module: 3,
    },
    StateKey {
        name: "grain.level",
        word: Some(HOST_GRAIN),
        max: 3,
        module: 4,
    },
];

/// The most digits of a number in the state file. The loader parses the digits
/// itself, and 9 digits cannot overflow its 32-bit integers.
pub const STATE_DIGITS: usize = 9;

/// The ID of a state key in the file: the CRC-32 of its name, modulo 10^9, so
/// it has at most `STATE_DIGITS` digits.
pub fn state_key_id(name: &str) -> u32 {
    crc32fast::hash(name.as_bytes()) % 1_000_000_000
}

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
    Files::from([
        (camera.modeset.into(), b"[SCRIPT_EN_MODE EN]\r\n".to_vec()),
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
pub fn loader() -> Vec<u8> {
    loader_for(ICON_STRIDE)
}

/// The loader for a target with `icon_stride`-byte icon slots.
pub fn loader_for(icon_stride: usize) -> Vec<u8> {
    let path = |name: &str| format!("C:\\SCRIPT\\PLUGINS\\{name}");
    let (plan, done, failed) = (path("MODULE.BIN"), path("INSTALL.OK"), path("INSTALL.ERR"));
    let mut lines: Vec<String> = vec!["sendln 'lcmd_resp on'".into()];
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
    let keys = STATE_KEYS.iter().enumerate();
    let mut lines: Vec<String> = vec![":resident".into(), "call restore".into()];

    // Save after a change. `filerename` cannot replace a file, and its result
    // is 0 for success and failure, so each step checks the files. If power
    // fails between the steps, the restore reads STATE.TMP.
    lines.extend([
        ":watch".into(),
        "mpause 2000".into(),
        "call snapshot".into(),
        "call save".into(),
        "goto watch".into(),
        ":save".into(),
        "changed = 0".into(),
    ]);
    for (i, _) in keys.clone() {
        lines.push(format!("if value{i} <> saved{i} changed = 1"));
    }
    lines.extend([
        "if changed = 0 goto save_end".into(),
        format!("filedelete '{temporary}'"),
        format!("filecreate sfh '{temporary}'"),
        "if sfh < 0 goto save_end".into(),
    ]);
    for (i, key) in keys.clone() {
        lines.extend([
            format!("filewriteln sfh '{}'", state_key_id(key.name)),
            format!("int2str line value{i}"),
            "filewriteln sfh line".into(),
        ]);
    }
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
            state_key_id(key.name)
        ));
    }
    lines.push("goto restore_reset".into());
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
            format!("bit = (plugins >> {}) and 1", key.module),
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
    ]);
    for (i, _) in keys {
        lines.push(format!("saved{i} = value{i}"));
    }
    lines.push("return".into());
    lines
}

pub fn runtime_files(binary: &[u8]) -> Files {
    runtime_files_for(binary, ICON_STRIDE)
}

/// The runtime files of a target with `icon_stride`-byte icon slots.
pub fn runtime_files_for(binary: &[u8], icon_stride: usize) -> Files {
    let target = "C:\\SCRIPT\\PLUGINS\\PLUGINS.TTL";
    let startup = format!("filesearch '{target}'\r\nif result = 0 goto done\r\ninclude '{target}'\r\n:done\r\nend\r\n");
    let wrapper = |mode: &str| {
        format!("{LINUX_DIRECTORY}/INSTALL.BIN --{mode}\nstatus=$?\necho GR_MODULE_{}_EXIT_$status\nexit 0\n", mode.to_uppercase()).into_bytes()
    };
    Files::from([
        ("SCRIPT/STARTUP.TTL".into(), startup.into_bytes()),
        ("SCRIPT/PLUGINS/PLUGINS.TTL".into(), loader_for(icon_stride)),
        ("SCRIPT/PLUGINS/INSTALL.BIN".into(), binary.to_vec()),
        ("SCRIPT/PLUGINS/SCAN.SH".into(), wrapper("scan")),
        ("SCRIPT/PLUGINS/INSTALL.SH".into(), wrapper("install")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_keys_have_distinct_positive_ids() {
        let mut ids: Vec<_> = STATE_KEYS
            .iter()
            .map(|key| state_key_id(key.name))
            .collect();
        assert!(ids
            .iter()
            .all(|id| *id > 0 && id.to_string().len() <= STATE_DIGITS));
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), STATE_KEYS.len());
        assert!(STATE_KEYS[0].word.is_none());
        assert!(STATE_KEYS[1..].iter().all(|key| key.word.is_some()));
    }

    /// A runtime error ends the script and frees the arrays, so the resident
    /// part must not index outside the header or divide by a variable.
    #[test]
    fn the_resident_script_cannot_stop_on_its_own_arithmetic() {
        let script = state_script().join("\n");
        let record_code = format!(
            "{} + index * {} + {}",
            HEADER_SIZE / 4,
            RECORD_SIZE / 4,
            RECORD_CODE / 4
        );
        for (at, _) in script.match_indices("registry[") {
            let index = &script[at + 9..];
            let index = &index[..index.find(']').unwrap()];
            match index.parse::<usize>() {
                Ok(word) => assert!(word < HEADER_SIZE / 4, "registry[{word}]"),
                // The only computed index is the code of a record below `count`.
                Err(_) => assert_eq!(index, record_code),
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
    }
}
