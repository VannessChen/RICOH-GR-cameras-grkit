//! The loader script (`LOADER.TTL`): it waits for Linux and the camera, runs
//! SCAN and INSTALL, then stays resident to restore and save the state.

use super::*;
use crate::abi::{
    HOST_PROCESSING_CALLS, HOST_PROFILE, MAILBOX_FAILED, MAILBOX_INSTALLED, MAILBOX_PLAN,
    MAILBOX_RESULT, MAILBOX_STATUS, MAILBOX_WORDS, PLAN_COUNT, PLAN_HEADER_WORDS, PLAN_ICON_CHUNKS,
    PLAN_MAGIC, PLAN_MODULE_TILES, PLAN_MODULE_WORDS, PROFILE, PROFILE_KINDS, PROFILE_MODULES,
    PROFILE_MODULE_SIZE, PROFILE_POINTS, PROFILE_SLOTS, PROFILE_SLOT_SIZE,
};
use crate::capabilities::{self, StateFile};
use crate::card::{
    DATA_FOLDER, LINUX_CARD, LINUX_DIRECTORY, LINUX_LOAD_COPY, LINUX_MOUNT, PACKAGE_DIRECTORY,
    RTOS_PACKAGE_FOLDER,
};

/// The resident loader reads the selections at this interval. It writes the
/// state file when two reads in a row agree, so a change reaches the card
/// one to two intervals after the last change.
pub const STATE_POLL_MS: u32 = 2000;
/// In the development profile (runtime/profile.h), the resident loader
/// writes the counts of the core to `PROFILE.LOG` in the data folder after
/// this many polls, and clears them.
pub const PROFILE_POLLS: u32 = 5;

/// Polls for Linux and camera readiness instead of waiting a fixed time.
/// `lcmd` requests are synchronous: each returns after the Linux command ends,
/// or with NG when Linux is not ready. The loader checks the mailbox array
/// after each request and repeats it after a short pause.
///
/// The only fixed wait comes before `lcmd_resp on`. The RTOS builds its host
/// command tables on a worker task after the script starts, and the
/// `lcmd_resp` handler writes through a null pointer until they exist, which
/// stops the camera at power-on. The script cannot ask for the tables. A wait
/// of 1 s or more before `lcmd_resp on` avoids the fault, so the loader waits
/// 2 s.
///
/// The camera mounts the card on Linux about 2 s after Linux runs commands.
/// The first request does not wait for that: unless Linux has mounted a
/// card folder, it mounts the card itself and runs `LOAD.SH`, which copies
/// the package and the data folder into RAM, unmounts the card, and scans
/// the copy (`runtime::files`). The command starts with a throwaway token:
/// the RTOS drops the first token of a request that starts with `-`.
///
/// A request of the RTOS is cut after about 245 bytes, and the RTOS removes
/// double quotes from it, so the long request stays below `REQUEST_MAX`
/// bytes and has no quotes.
pub fn loader() -> Vec<u8> {
    loader_for(ICON_STRIDE)
}

/// The most bytes of an `lcmd` request after `lcmd `: the RTOS cuts a
/// request after about 245 bytes.
pub const REQUEST_MAX: usize = 220;

/// The first request of the loader: the Linux side of the start. The script
/// runs from a copy in RAM, because a shell reads a script while it runs it,
/// and the script unmounts the card. The request removes the mount point
/// after the script, and the script leaves a file in it. So a retry after a
/// failed mount mounts again, and a retry after the script does not: each
/// mount is a chance to collide with the mount of the camera.
fn load_request() -> String {
    format!(
        ": -q ; d={PACKAGE_DIRECTORY}/LOAD.SH; if grep -q storage /proc/mounts; then sh {LINUX_CARD}$d; \
         else mkdir {LINUX_MOUNT} && mount -t ipcufs mnt {LINUX_MOUNT} -ofs=1 && cp {LINUX_MOUNT}$d {LINUX_LOAD_COPY} \
         && sh {LINUX_LOAD_COPY} {LINUX_MOUNT}; rmdir {LINUX_MOUNT}; :; fi"
    )
}

/// The loader for a target with `icon_stride`-byte icon slots.
pub fn loader_for(icon_stride: usize) -> Vec<u8> {
    let path = core_path;
    let mailbox = |offset: usize| format!("mailbox[{}]", offset / 4);
    let plan_word = |field: usize| mailbox(MAILBOX_PLAN + field);
    let (result, status) = (mailbox(MAILBOX_RESULT), mailbox(MAILBOX_STATUS));
    // Wait for the host command tables of the RTOS before lcmd_resp.
    let mut lines: Vec<String> = vec!["mpause 2000".into(), "sendln 'lcmd_resp on'".into()];
    // The results of the last start, which FINISH.SH copies to the card.
    for name in ["INSTALL.OK", "INSTALL.ERR"] {
        lines.push(format!("filedelete '{}'", path(name)));
    }
    lines.extend([
        // The mailbox (`abi::MAILBOX_FIELDS`): the first script array, which
        // SCAN and INSTALL find through the array table.
        format!("intdim mailbox {MAILBOX_WORDS}"),
        format!("{} = 0", plan_word(PLAN_MAGIC)),
        format!("{result} = 0"),
        format!("{status} = 0"),
        // Early requests fail before Linux runs commands. A failed SCAN stops
        // the loader; give up after 150 attempts without a plan.
        "attempts = 0".into(),
        ":scan".into(),
        format!("sendln 'lcmd {}'", load_request()),
        format!("posted = {result}"),
        format!("if posted = {MAILBOX_FAILED} goto reject"),
        format!("posted = {}", plan_word(PLAN_MAGIC)),
        format!(
            "if posted = {} goto plan_ready",
            crate::abi::PLAN_MAGIC_VALUE
        ),
        "mpause 200".into(),
        "attempts = attempts + 1".into(),
        "if attempts < 150 goto scan".into(),
        "goto reject".into(),
        ":plan_ready".into(),
        // The words of the plan header (`abi::PLAN_FIELDS`).
        format!("count = {}", plan_word(PLAN_COUNT)),
        "if count < 0 goto reject".into(),
        format!("if count > {MAX_COUNT} goto reject"),
        format!("words = {}", plan_word(PLAN_HEADER_WORDS)),
        format!("chunks = {}", plan_word(PLAN_ICON_CHUNKS)),
        format!("modulewords = {}", plan_word(PLAN_MODULE_WORDS)),
        "if modulewords < 0 goto reject".into(),
        format!("if modulewords > {MODULE_ARRAY_WORDS} goto reject"),
        format!("moduletiles = {}", plan_word(PLAN_MODULE_TILES)),
        "if moduletiles < 0 goto reject".into(),
        format!("if moduletiles > {MODULE_TILES_MAX} goto reject"),
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
        format!("posted = {result}"),
        format!("if posted = {MAILBOX_INSTALLED} goto installed"),
        format!("if posted = {MAILBOX_FAILED} goto reject"),
        "mpause 500".into(),
        "attempts = attempts + 1".into(),
        "if attempts > 60 mpause 9500".into(),
        "goto install".into(),
        // The results go to the card after the camera mounts it.
        ":installed".into(),
        format!("sendln 'lcmd sh {LINUX_DIRECTORY}/FINISH.SH'"),
    ]);
    lines.extend(state_script());
    lines.extend(profile_script());
    lines.extend([
        ":reject".into(),
        format!("sendln 'lcmd sh {LINUX_DIRECTORY}/FINISH.SH'"),
        "dispstr 'GR_MODULE_LOADER_REJECTED'".into(),
        "end".into(),
    ]);
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
        // The development profile has its own loop, so the usual poll
        // reads no other word.
        format!("profile = {}", word(HOST_PROFILE)),
        "if profile <> 0 goto profile_watch".into(),
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

/// The loader path of the profile file.
fn profile_path() -> String {
    format!("{DATA_FOLDER}\\PROFILE.LOG")
}

/// The development profile of the resident loader (runtime/profile.h). The
/// watch loop of the profile saves the state as the usual one does, and after
/// each `PROFILE_POLLS` polls appends a period to `PROFILE.LOG`: a line with
/// its number and the live-view frames, then a line for each counter slot
/// with calls: its name, the calls, the cycles, and the cycles of the largest
/// call. The loader clears each slot that it writes. The module names come
/// from the profile table, 8 ASCII bytes each.
fn profile_script() -> Vec<String> {
    let table = PROFILE / 4;
    let path = profile_path();
    let mut lines: Vec<String> = vec![
        ":profile_watch".into(),
        "ppolls = 0".into(),
        "pperiod = 0".into(),
        format!("pframes = registry[{}]", HOST_PROCESSING_CALLS / 4),
    ];
    // The module names: the bytes up to the first 0.
    for module in 0..PROFILE_MODULES {
        let base = table + module * PROFILE_MODULE_SIZE / 4 + 2;
        let end = format!("profile_name{module}_end");
        lines.push(format!("pname{module} = ''"));
        for byte in 0..8 {
            lines.extend([
                format!(
                    "pc = (tailarea[{}] >> {}) and 255",
                    base + byte / 4,
                    byte % 4 * 8
                ),
                format!("if pc = 0 goto {end}"),
                "code2str ps pc".into(),
                format!("strconcat pname{module} ps"),
            ]);
        }
        lines.push(format!(":{end}"));
    }
    lines.extend([
        // A new file, or the end of the file of an earlier start.
        format!("filesearch '{path}'"),
        "if result != 0 goto profile_append".into(),
        format!("filecreate pfh '{path}'"),
        "goto profile_opened".into(),
        ":profile_append".into(),
        format!("fileopen pfh '{path}' 1"),
        ":profile_opened".into(),
        "if pfh < 0 goto profile_poll".into(),
        "filewriteln pfh 'GRKit profile: for each period, the live-view frames; for each point, calls, CPU cycles, and the cycles of the largest call'".into(),
        "fileclose pfh".into(),
        ":profile_poll".into(),
        format!("mpause {STATE_POLL_MS}"),
        "ppolls = ppolls + 1".into(),
        format!("if ppolls >= {PROFILE_POLLS} call profile_write"),
        format!("changes = registry[{}]", HOST_STATE_CHANGES / 4),
        "if changes <> last goto profile_moved".into(),
        "if changes = saved goto profile_poll".into(),
        "stamp = changes".into(),
    ]);
    for file in StateFile::ALL {
        lines.push(format!("call save_{}", label(file)));
    }
    lines.extend([
        "goto profile_poll".into(),
        ":profile_moved".into(),
        "last = changes".into(),
        "goto profile_poll".into(),
        ":profile_write".into(),
        "ppolls = 0".into(),
        "pperiod = pperiod + 1".into(),
        format!("pf = registry[{}]", HOST_PROCESSING_CALLS / 4),
        "pframe = pf - pframes".into(),
        "pframes = pf".into(),
        format!("fileopen pfh '{path}' 1"),
        "if pfh < 0 goto profile_write_end".into(),
        "filewrite pfh 'period '".into(),
        "int2str ps pperiod".into(),
        "filewrite pfh ps".into(),
        "filewrite pfh ' frames '".into(),
        "int2str ps pframe".into(),
        "filewriteln pfh ps".into(),
    ]);
    let mut slot_lines = |slot: usize, name: Vec<String>| {
        let at = table + (PROFILE_SLOTS + slot * PROFILE_SLOT_SIZE) / 4;
        let end = format!("profile_slot{slot}");
        lines.extend([
            format!("pc = tailarea[{at}]"),
            format!("if pc = 0 goto {end}"),
            format!("pcy = tailarea[{}]", at + 1),
            format!("pmx = tailarea[{}]", at + 2),
            format!("tailarea[{at}] = 0"),
            format!("tailarea[{}] = 0", at + 1),
            format!("tailarea[{}] = 0", at + 2),
        ]);
        lines.extend(name);
        for (value, last) in [("pc", false), ("pcy", false), ("pmx", true)] {
            lines.push(format!("int2str ps {value}"));
            lines.push("filewrite pfh ps".into());
            lines.push(if last {
                "filewriteln pfh ''".into()
            } else {
                "filewrite pfh ' '".into()
            });
        }
        lines.push(format!(":{end}"));
    };
    for (slot, (name, _)) in PROFILE_POINTS.iter().enumerate() {
        slot_lines(slot, vec![format!("filewrite pfh '{name} '")]);
    }
    for module in 0..PROFILE_MODULES {
        for (kind, (name, _)) in PROFILE_KINDS.iter().enumerate() {
            let slot = PROFILE_POINTS.len() + module * PROFILE_KINDS.len() + kind;
            slot_lines(
                slot,
                vec![
                    format!("filewrite pfh pname{module}"),
                    format!("filewrite pfh ' {name} '"),
                ],
            );
        }
    }
    lines.extend([
        "fileclose pfh".into(),
        ":profile_write_end".into(),
        "return".into(),
    ]);
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

/// The loader path of a file of the package folder.
pub(super) fn core_path(name: &str) -> String {
    format!("{RTOS_PACKAGE_FOLDER}\\{name}")
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// The RTOS cuts a request after about 245 bytes and removes its double
    /// quotes. The first request mounts the card only when Linux has mounted
    /// no card folder, and always leaves the mount point behind for a retry.
    #[test]
    fn the_requests_fit_the_rtos() {
        let script = loader();
        let script = String::from_utf8_lossy(&script);
        let requests: Vec<_> = script
            .lines()
            .filter_map(|l| l.strip_prefix("sendln 'lcmd ")?.strip_suffix('\''))
            .collect();
        assert_eq!(requests.len(), 4, "{requests:?}");
        for request in &requests {
            assert!(request.len() <= REQUEST_MAX, "{}: {request}", request.len());
            assert!(
                !request.contains('"') && !request.contains('\''),
                "{request}"
            );
        }
        let load = requests[0];
        assert!(load.starts_with(": -q ; "));
        assert!(load.contains("if grep -q storage /proc/mounts; then sh /media/storage1$d; else mkdir /tmp/m && mount -t ipcufs mnt /tmp/m -ofs=1 && cp /tmp/m$d /tmp/l && sh /tmp/l /tmp/m; rmdir /tmp/m; :; fi"));
        assert!(load.contains("d=/SCRIPT/GRKIT/LOAD.SH;"));
        // SCAN and INSTALL run from the RAM copy of the card.
        assert_eq!(requests[1], "sh /tmp/gc/SCRIPT/GRKIT/INSTALL.SH");
        assert_eq!(requests[2], "sh /tmp/gc/SCRIPT/GRKIT/FINISH.SH");
        assert_eq!(requests[3], requests[2]);
        // The loader waits on the mailbox, the first array, not on card files.
        let mailbox = script
            .lines()
            .position(|l| l == "intdim mailbox 16")
            .unwrap();
        assert!(script
            .lines()
            .take(mailbox)
            .all(|l| !l.starts_with("intdim")));
        assert!(script.contains("posted = mailbox[10]\r\nif posted = 2 goto reject"));
        assert!(script.contains(&format!(
            "posted = mailbox[0]\r\nif posted = {} goto plan_ready",
            crate::abi::PLAN_MAGIC_VALUE
        )));
        assert!(script.contains("if posted = 1 goto installed"));
        assert!(!script.contains("filesearch 'C:\\SCRIPT\\GRKIT\\PLAN.BIN'"));
        assert!(!script.contains("fileopen fh"));
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
        // The plan, and the profile file, which the loader appends to.
        assert!(
            opens
                .iter()
                .all(|l| l.contains("PLAN.BIN") || l.ends_with("PROFILE.LOG' 1")),
            "{opens:?}"
        );
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
