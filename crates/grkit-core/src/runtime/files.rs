//! The runtime files of a package: the startup and loader scripts, the
//! package scripts, the core files, and the files that turn SD scripts on or
//! off.

use super::loader::{core_path, loader_for};
use super::ICON_STRIDE;
use crate::camera::Camera;
use crate::capabilities::StateFile;
use crate::card::{
    DATA_DIRECTORY, FAST_OFF, FAST_ON, FAST_TRY, FINISH_FILE, INSTALL_FILE, LEGACY_DIRECTORY,
    LINUX_CARD, LINUX_DIRECTORY, LINUX_SLOW_START, LINUX_STAGE, LOADER_FILE, LOAD_FILE,
    PACKAGE_DIRECTORY, PACKAGE_FOLDER, SCAN_FILE,
};
use crate::Files;

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

pub fn runtime_files(binary: &[u8]) -> Files {
    runtime_files_for(binary, ICON_STRIDE)
}

/// The runtime files of a target with `icon_stride`-byte icon slots.
pub fn runtime_files_for(binary: &[u8], icon_stride: usize) -> Files {
    package_runtime_files(&[binary], icon_stride)
}

/// The exit statuses of a core file that the package scripts test
/// (`runtime/install_status.h`): the core does not relocate to the running
/// build, or (`--known`) does not name it.
pub const STATUS_RELOCATION: u32 = 47;
pub const STATUS_OTHER_BUILD: u32 = 48;

/// The core file of a runtime build: the core of one firmware build and its
/// installer, which SCAN.SH and INSTALL.SH run.
pub use crate::card::CORE_FILE;

/// The most core files that a package holds: `CORE1.BIN` to `CORE9.BIN`, so
/// each name stays 8.3.
pub const MAX_CORE_FILES: usize = 9;

/// The path of core file `index` of `count` in a package: `CORE.BIN` for one
/// build, else `CORE1.BIN` and on, in release order.
pub fn core_path_of(index: usize, count: usize) -> String {
    if count == 1 {
        CORE_FILE.to_owned()
    } else {
        format!("{PACKAGE_FOLDER}/CORE{}.BIN", index + 1)
    }
}

/// The most seconds that LOAD.SH and FINISH.SH wait for the camera to mount
/// the card.
pub(super) const CARD_WAIT_SECONDS: u32 = 60;

/// The runtime files of a package with the core file of each runtime build of
/// a camera, in release order, for `icon_stride`-byte icon slots. The builds of
/// a camera have the same scripts and plan.
///
/// The loader runs the scripts on Linux (`crate::card`). `LOAD.SH` runs
/// first, on the mount of the card that the loader made, or on the mount of
/// the camera when the camera was first: it copies the package folder and
/// the data folder into the RAM copy of the card and runs `SCAN.SH` there,
/// then unmounts the card. The window with the mount is short, because the
/// camera's own mount of the card fails while another mount holds it, and
/// an unmount after that failure stops Linux. If the camera tried its mount
/// during the window, `LOAD.SH` moves its mount to the camera's path instead
/// of an unmount. Each installer takes the card root as its argument.
///
/// With one core file, `INSTALL.SH` runs it with `--install`. With more, it
/// runs each with `--known` first, which reads one word to find the build. If
/// no core file names the build, it tries each one with `--try`, which runs
/// the relocation, and the last one with `--install`, which writes
/// `INSTALL.ERR` if it also fails. A known build thus installs without a relocation of
/// another build, and an unknown build still gets each runtime. `SCAN.SH`
/// selects the core file in the same way with `--scan-known`, `--scan-try`,
/// and `--scan`, because SCAN posts its plan to the mailbox, whose address
/// only the runtime of the build resolves.
///
/// `FINISH.SH` runs after INSTALL: it waits until the camera mounts the
/// card, copies the results to it, imports the new effects of the package
/// into the effects library on the card, and frees the RAM copy.
pub fn package_runtime_files(binaries: &[&[u8]], icon_stride: usize) -> Files {
    let count = binaries.len();
    assert!((1..=MAX_CORE_FILES).contains(&count));
    let target = core_path("LOADER.TTL");
    let startup = format!("filesearch '{target}'\r\nif result = 0 goto done\r\ninclude '{target}'\r\n:done\r\nend\r\n");
    let name = |index: usize| {
        core_path_of(index, count)
            .rsplit('/')
            .next()
            .unwrap()
            .to_owned()
    };
    // Each script body sets `status` to the exit status of the installer.
    // SCAN.SH exits with it, for LOAD.SH; the loader ignores exit statuses.
    // Each also adds its start and its status to the boot log.
    let wrapper = |body: String, mode: &str, exit: &str| {
        format!(
            "read t _ </proc/uptime; echo \"$t {mode}-start\" >>{LINUX_STAGE}/{BOOT_LOG}\n\
             {body}read t _ </proc/uptime; echo \"$t {mode} $status\" >>{LINUX_STAGE}/{BOOT_LOG}\n\
             echo GR_MODULE_{}_EXIT_$status\nexit {exit}\n",
            mode.to_uppercase()
        )
        .into_bytes()
    };
    let single = |mode: &str| {
        format!(
            "{LINUX_DIRECTORY}/{} --{mode} {LINUX_STAGE}\nstatus=$?\n",
            name(0)
        )
    };
    let names = |range: std::ops::Range<usize>| range.map(name).collect::<Vec<_>>().join(" ");
    // The core file of the build: each core with `known` first, then each
    // with `tried`, then the last one with `last`.
    let chain = |known: &str, tried_mode: &str, last_mode: &str| {
        format!(
            "for core in {all}; do\n\
             \x20 {LINUX_DIRECTORY}/$core --{known} {LINUX_STAGE}\n\
             \x20 status=$?\n\
             \x20 case $status in {other}) ;; *) break ;; esac\n\
             done\n\
             case $status in {other})\n\
             \x20 for core in {tried}; do\n\
             \x20   {LINUX_DIRECTORY}/$core --{tried_mode} {LINUX_STAGE}\n\
             \x20   status=$?\n\
             \x20   case $status in {relocation}) ;; *) break ;; esac\n\
             \x20 done\n\
             \x20 case $status in {relocation}) {LINUX_DIRECTORY}/{last} --{last_mode} {LINUX_STAGE}; status=$? ;; esac\n\
             \x20 ;;\n\
             esac\n",
            all = names(0..count),
            tried = names(0..count - 1),
            last = name(count - 1),
            other = STATUS_OTHER_BUILD,
            relocation = STATUS_RELOCATION,
        )
    };
    let (scan, install) = match count {
        1 => (single("scan"), single("install")),
        _ => (
            chain("scan-known", "scan-try", "scan"),
            chain("known", "try", "install"),
        ),
    };
    let mut files = Files::from([
        ("SCRIPT/STARTUP.TTL".into(), startup.into_bytes()),
        (LOADER_FILE.into(), loader_for(icon_stride)),
        (LOAD_FILE.into(), load_script(count).into_bytes()),
        (SCAN_FILE.into(), wrapper(scan, "scan", "$status")),
        (INSTALL_FILE.into(), wrapper(install, "install", "0")),
        (FINISH_FILE.into(), finish_script(&name(0)).into_bytes()),
    ]);
    for (index, binary) in binaries.iter().enumerate() {
        files.insert(core_path_of(index, count), binary.to_vec());
    }
    files
}

/// The RAM copy of the card is a tmpfs of its own, because the `/tmp` of a
/// camera can forbid execution, and the copy then takes at most this much
/// memory.
const STAGE_SIZE_MB: u32 = 16;

/// The exit status of SCAN that asks LOAD.SH for the effect folders
/// (`runtime/install_status.h`).
pub const STATUS_PLAN_STALE: u32 = 49;

/// The boot log: one line for each step of the start, with the Linux uptime
/// in seconds. The scripts count lines with `grep -c`, because the camera's
/// Linux has no `wc` and no `find`. The package scripts write it into the RAM copy. `LOAD.SH`
/// adds its lines to `GRKIT/BOOT.TXT` on the card before it releases its
/// mount, because a failed release stops Linux, and `FINISH.SH` adds the
/// rest. A person sends the file with a report of a start that failed.
pub const BOOT_LOG: &str = "BOOT.TXT";
/// The number of lines of the boot log that `LOAD.SH` put on the card.
const BOOT_LOG_SENT: &str = "BOOT.N";
/// The most lines of the boot log on the card: about 20 starts.
const BOOT_LOG_LINES: u32 = 400;

/// The boot log in the RAM copy, and the file with its number of lines on the card.
fn boot_log_paths() -> (String, String) {
    (
        format!("{LINUX_STAGE}/{BOOT_LOG}"),
        format!("{LINUX_STAGE}/{BOOT_LOG_SENT}"),
    )
}

/// The fast path of the start (`runtime::loader`): `LOAD.SH` gives up the
/// loader's mount this many centiseconds before the measured time of the
/// camera's mount.
const FAST_SAFETY_CS: u32 = 50;
/// The window in centiseconds when FAST.ON holds no number.
const FAST_DEFAULT_CS: u32 = 150;
/// The longest window in centiseconds that `LOAD.SH` uses.
const FAST_WINDOW_MAX_CS: u32 = 400;
/// A slow start turns the fast path on when the measured window is at least
/// this long, so that the deadline leaves this much less `FAST_SAFETY_CS`
/// for the copy and the first SCAN. Their time on the slow path is no measure
/// of the fast path: there the camera uses the card too. A copy that does
/// not fit only meets the deadline.
const FAST_ENABLE_CS: u32 = 110;
/// A fast start that released its mount less than this before the camera
/// tried to mount the card turns the fast path off.
const FAST_NEAR_CS: u32 = 30;
/// On the slow path, the most centiseconds from the camera's mount to the
/// next request of the loader, which runs `LOAD.SH`.
const SLOW_POLL_CS: u32 = 30;

/// `LOAD.SH`: the RAM copy of the card, then SCAN. Its argument is the mount
/// of the card that the loader made (the fast path), or none for the mount
/// of the camera (the slow path).
///
/// The copy of a file through the card file system costs about 30 ms on a
/// camera, so the script copies the effect folders only when SCAN needs
/// them for a new plan: it copies the plan of an earlier start with its key
/// and a listing of the folders, and SCAN keeps that plan when the key
/// matches (`installer.c`: plan_key). For a new plan, the script first
/// releases its mount and waits for the camera's mount, then copies the
/// effect folders from it.
///
/// On the fast path, each copy and SCAN first check a deadline: the time
/// that a slow start measured from the first request to the camera's mount
/// (FAST.ON), less `FAST_SAFETY_CS`. A late step releases the mount, waits
/// for the camera's mount, and copies the files again from it, as the slow
/// path does. Just before the release, the script writes FAST.TRY through its
/// mount, which the loader removes when the request returns: a FAST.TRY at
/// power-on is a start that stopped in the release. A process watches the
/// camera's try to mount the card and its mount for the boot log.
fn load_script(count: usize) -> String {
    let (log, sent) = boot_log_paths();
    let stage = LINUX_STAGE;
    let card = LINUX_CARD;
    let legacy = LEGACY_DIRECTORY;
    let package = format!("{LINUX_STAGE}{PACKAGE_DIRECTORY}");
    let data = format!("{LINUX_STAGE}{DATA_DIRECTORY}");
    let (package_dir, data_dir) = (
        PACKAGE_DIRECTORY.trim_start_matches('/'),
        DATA_DIRECTORY.trim_start_matches('/'),
    );
    // The data files that SCAN and INSTALL read: the state, the markers, the
    // import record, and the report, which INSTALL rewrites only on a change.
    let mut data_files = vec![];
    for file in StateFile::ALL {
        data_files.push(format!("{}.TXT", file.stem()));
        data_files.push(format!("{}.TMP", file.stem()));
    }
    data_files.extend(
        [
            "SAFE",
            "SAFE.TXT",
            "PROFILE",
            "PROFILE.TXT",
            "IMPORTED.TXT",
            "REPORT.TXT",
            "RELOC.BIN",
            "PLAN.KEY",
        ]
        .map(String::from),
    );
    let data_files = data_files.join(" ");
    // The core files, which a copy leaves without their execute permission.
    let cores = (0..count)
        .map(|index| {
            core_path_of(index, count)
                .rsplit('/')
                .next()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        r#"# The Linux side of the start: the card into RAM, then SCAN.
src=${{1:-{card}}}
# The functions run in the shell of the script, so their variables start
# with an underscore: a function must not change a variable of its caller,
# such as the loop variable of put.
# A line of the boot log: the uptime, then the step.
log() {{ read _t _ </proc/uptime; echo "$_t $*" >>{log}; }}
# The uptime in centiseconds, in $now, without a subshell. An arithmetic
# error stops the script, and a number that starts with 0 is octal, so the
# hundredths get a leading 1.
now() {{ read _t _ </proc/uptime; _s=${{_t%.*}}; _f=${{_t#*.}}; now=$((_s * 100 + 1$_f - 100)); }}
# Release the mount of the loader before the camera mounts the card. If
# the camera tried in the meantime, its mount failed: keep this one for
# it, because an unmount now would stop Linux. Then the camera's mount
# is the card.
release() {{
  [ $src = {card} ] && return
  log release
  # The log so far on the card, because a failed release stops Linux, and
  # FAST.TRY, which the loader removes once this request returns.
  mkdir -p $src/{data_dir} || log mkdir $?
  _n=$(grep -c '' {log})
  cat {log} >>$src/{data_dir}/{boot} || log append $?
  echo $_n >{sent}
  : >$src/{package_dir}/{fast_try}
  if grep -q 'mnt {card}' /var/log/messages && ! grep -q storage1 /proc/mounts; then
    mount --move $src {card}
    log released move
  else
    umount $src
    log released umount
  fi
  src={card}
}}
# The camera's mount of the card.
mounted() {{
  _n=0
  while ! grep -q storage1 /proc/mounts; do
    usleep 200000
    _n=$((_n + 1))
    [ $_n -gt {waits} ] && return 1
  done
  return 0
}}
# On the loader's mount, a step after the deadline is late.
late() {{
  [ $src = {card} ] && return 1
  now
  [ $now -lt $deadline ] && return 1
  log late
  return 0
}}
# A file, or each file of a flat folder, into the folder $2, unless late.
# After a late step, the copy resumes and skips the files that it has.
put() {{
  if [ -d "$1" ]; then
    mkdir -p "$2/${{1##*/}}"
    for _p in "$1"/*; do
      [ -e "$_p" ] || continue
      [ -n "$resume" ] && [ -e "$2/${{1##*/}}/${{_p##*/}}" ] && continue
      late && return 1
      cp "$_p" "$2/${{1##*/}}/"
    done
  else
    [ -n "$resume" ] && [ -e "$2/${{1##*/}}" ] && return 0
    late && return 1
    cp "$1" "$2/"
  fi
  return 0
}}
# The files that SCAN reads, from the card into the RAM copy.
fetch() {{
  mkdir -p {package} {data}
  for _f in PACKAGE.TXT SCAN.SH INSTALL.SH FINISH.SH {cores}; do put $src/{package_dir}/$_f {package} || return 1; done
  chmod 755 {package}/CORE*.BIN
  [ -d $src/{package_dir}/MODULES ] && {{ put $src/{package_dir}/MODULES {package} || return 1; }}
  for _f in {data_files}; do [ -e $src/{data_dir}/$_f ] && {{ put $src/{data_dir}/$_f {data} || return 1; }}; done
  [ -d $src/{data_dir}/MODULES ] && {{ put $src/{data_dir}/MODULES {data} || return 1; }}
  # The state of GRKit 1.2, which INSTALL reads when GRKIT has no state file.
  for _f in STATE.TXT STATE.TMP; do [ -e $src{legacy}/$_f ] && {{ mkdir -p {stage}{legacy}; put $src{legacy}/$_f {stage}{legacy} || return 1; }}; done
  # The plan of an earlier start, and the listing of the folders that keys
  # it, relative to the card root, so that both mounts give one listing.
  [ -e $src/{data_dir}/PLAN.BIN ] && {{ put $src/{data_dir}/PLAN.BIN {package} || return 1; }}
  _dirs=
  for _d in {data_dir}/EFFECTS {package_dir}/EFFECTS {data_dir}/MODULES {package_dir}/MODULES; do [ -d $src/$_d ] && _dirs="$_dirs $_d"; done
  [ -n "$_dirs" ] && (cd $src && {{ echo $_dirs; ls -le $_dirs || ls -l $_dirs; }}) >{data}/LIST.TXT 2>&1
  return 0
}}
if mkdir {stage} 2>/dev/null; then
  mount -t tmpfs -o size={size}m tmpfs {stage}
  log load $src
  # The deadline of the loader's mount: the measured time from the first
  # request to the camera's mount, less a margin.
  window=$(cat $src/{package_dir}/{fast_on} 2>/dev/null)
  [ "$window" -ge 0 ] 2>/dev/null || window={default}
  # The request returns NG after about 5 s while the script runs on, so
  # the release stays well before that, while the loader can still remove
  # FAST.TRY when the request returns.
  [ $window -gt {window_max} ] && window={window_max}
  now
  deadline=$((now + window - {safety}))
  resume=
  # The camera's try to mount the card and its mount, for the log.
  (
    tried=
    n=0
    while [ $n -lt {watches} ]; do
      [ -z "$tried" ] && grep -q 'mnt {card}' /var/log/messages && {{ tried=1; log camera-try; }}
      grep -q storage1 /proc/mounts && {{ log camera-mount; break; }}
      usleep 100000
      n=$((n + 1))
    done
  ) </dev/null >/dev/null 2>&1 &
  if ! fetch; then
    # Late: the rest waits for the camera's mount, as on the slow path.
    release
    mounted || exit 0
    resume=1
    fetch
  fi
  log copied
  if late; then
    release
    mounted || exit 0
  fi
  sh {package}/SCAN.SH
  s=$?
  [ -e {package}/PLAN.BAD ] && log plan-damaged
  if [ $s = {stale} ]; then
    # A new plan: the effect folders, and the effects of GRKit 1.2 when
    # GRKIT has no library. Their copy is long, so it waits for the camera's
    # mount, in a process of its own: the request returns right after the
    # release, and the loader polls the mailbox for the plan.
    release
    (
      mounted || exit 0
      mkdir -p {package}/EFFECTS
      [ -d $src/{package_dir}/EFFECTS ] && cp -r $src/{package_dir}/EFFECTS {package}/
      [ -d $src/{data_dir}/EFFECTS ] && cp -r $src/{data_dir}/EFFECTS {data}/
      if [ ! -e $src/{data_dir}/STATE.TXT ] || [ ! -d $src/{data_dir}/EFFECTS ]; then
        [ -d $src{legacy}/EFFECTS ] && mkdir -p {stage}{legacy} && cp -r $src{legacy}/EFFECTS {stage}{legacy}/
      fi
      log effects
      sh {package}/SCAN.SH
    ) </dev/null >/dev/null 2>&1 &
  fi
fi
release
exit 0
"#,
        boot = BOOT_LOG,
        size = STAGE_SIZE_MB,
        stale = STATUS_PLAN_STALE,
        waits = CARD_WAIT_SECONDS * 5,
        watches = CARD_WAIT_SECONDS * 10,
        default = FAST_DEFAULT_CS,
        window_max = FAST_WINDOW_MAX_CS,
        safety = FAST_SAFETY_CS,
        fast_on = FAST_ON,
        fast_try = FAST_TRY,
    )
}

/// The exit status of INSTALL for a plan that does not fit the start
/// (`runtime/install_status.h`).
const STATUS_PLAN: u32 = 37;

/// `FINISH.SH`: the results to the card after the camera mounts it, in a
/// process of its own, so the request of the loader returns at once.
///
/// It also keeps the fast path safe. After a slow start, it measures the
/// time from the first request to the camera's mount, and turns the fast
/// path on (FAST.ON) when that is at least `FAST_ENABLE_CS`. After a fast start that
/// did not install, that the camera's mount met, or that released its mount
/// less than `FAST_NEAR_CS` before the camera tried its mount, it turns the
/// fast path off (FAST.OFF). An INSTALL that refused a kept plan removes
/// its key, so the next start makes the plan again.
fn finish_script(core: &str) -> String {
    let (log, sent) = boot_log_paths();
    let stage = LINUX_STAGE;
    let card = LINUX_CARD;
    let package = format!("{LINUX_STAGE}{PACKAGE_DIRECTORY}");
    let data = format!("{LINUX_STAGE}{DATA_DIRECTORY}");
    let card_package = format!("{LINUX_CARD}{PACKAGE_DIRECTORY}");
    let card_data = format!("{LINUX_CARD}{DATA_DIRECTORY}");
    format!(
        r#"# After INSTALL: the results to the card, once the camera mounts it.
(
  n=0
  while ! grep -q storage1 /proc/mounts; do
    usleep 200000
    n=$((n + 1))
    [ $n -gt {waits} ] && exit 0
  done
  log() {{ read t _ </proc/uptime; echo "$t $*" >>{log}; }}
  # Seconds with two decimals in centiseconds (LOAD.SH: now).
  cs() {{ _s=${{1%.*}}; _f=${{1#*.}}; echo $((_s * 100 + 1$_f - 100)); }}
  log finish
  for f in INSTALL.OK INSTALL.ERR; do [ -e {package}/$f ] && cp {package}/$f {card_package}/; done
  mkdir -p {card_data}
  # The addresses of a search first, then the plan and its key, for the next start, then the
  # other results: each only on a change. A kept plan without the addresses would search
  # again, and a key without its plan would not keep it.
  [ -e {data}/RELOC.BIN ] && {{ cmp -s {data}/RELOC.BIN {card_data}/RELOC.BIN || cp {data}/RELOC.BIN {card_data}/; }}
  [ -e {package}/PLAN.BIN ] && {{ cmp -s {package}/PLAN.BIN {card_data}/PLAN.BIN || cp {package}/PLAN.BIN {card_data}/; }}
  for f in PLAN.KEY REPORT.TXT; do
    [ -e {data}/$f ] || continue
    cmp -s {data}/$f {card_data}/$f || cp {data}/$f {card_data}/
  done
  # INSTALL refused the plan: the next start makes it again.
  e=
  [ -e {package}/INSTALL.ERR ] && read e _ <{package}/INSTALL.ERR
  [ "$e" = {status_plan} ] && rm -f {card_data}/PLAN.KEY
  {card_package}/{core} --import {card}
  # The fast path: off after a start near the camera's mount or without an
  # install, on after a slow start with time to spare.
  fast=; loaded=; first=; released=; moved=; tried=
  while read t step arg; do
    case $step in
      load) loaded=$t; [ "$arg" = {card} ] || fast=1 ;;
      scan) [ -n "$first" ] || first=$t ;;
      released) released=$t; [ "$arg" = move ] && moved=1 ;;
      camera-try) tried=$t ;;
    esac
  done <{log}
  if [ -n "$fast" ]; then
    near=
    [ -e {package}/INSTALL.OK ] || near=1
    [ -n "$moved" ] && near=1
    [ -n "$tried" ] && [ -n "$released" ] && [ $(($(cs $tried) - $(cs $released))) -lt {near} ] && near=1
    [ -n "$near" ] && {{ : >{card_package}/{fast_off}; log fast-off; }}
  elif [ -e {slow_start} ] && [ -n "$first" ] && [ -e {package}/INSTALL.OK ] && [ ! -e {card_package}/{fast_off} ]; then
    # LOAD.SH started at most one request interval after the camera's mount.
    read a _ <{slow_start}
    w=$(($(cs $loaded) - $(cs $a) - {poll}))
    h=$(($(cs $first) - $(cs $loaded) + 10))
    [ $w -ge {enable} ] && {{ echo $w >{card_package}/{fast_on}; log fast-on $w $h; }}
  fi
  rm -f {card_package}/{fast_try}
  # The rest of the boot log, with the failed mounts of the camera, and only its last lines.
  log done $(grep -c 'fail to mount' /var/log/messages)
  b={card_data}/{boot}
  n=$(cat {sent} 2>/dev/null)
  tail -n +$((${{n:-0}} + 1)) {log} >>$b
  [ $(grep -c '' $b) -gt {lines} ] && tail -n {keep} $b >{stage}/b && cp {stage}/b $b
  umount {stage}
  rm -rf {stage}
) </dev/null >/dev/null 2>&1 &
exit 0
"#,
        boot = BOOT_LOG,
        waits = CARD_WAIT_SECONDS * 5,
        lines = BOOT_LOG_LINES,
        keep = BOOT_LOG_LINES * 3 / 4,
        status_plan = STATUS_PLAN,
        near = FAST_NEAR_CS,
        enable = FAST_ENABLE_CS,
        fast_on = FAST_ON,
        fast_off = FAST_OFF,
        fast_try = FAST_TRY,
        slow_start = LINUX_SLOW_START,
        poll = SLOW_POLL_CS,
    )
}
