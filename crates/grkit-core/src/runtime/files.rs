//! The runtime files of a package: the startup and loader scripts, the
//! package scripts, the core files, and the files that turn SD scripts on or
//! off.

use super::loader::{core_path, loader_for};
use super::ICON_STRIDE;
use crate::camera::Camera;
use crate::capabilities::StateFile;
use crate::card::{
    DATA_DIRECTORY, FINISH_FILE, INSTALL_FILE, LEGACY_DIRECTORY, LINUX_CARD, LINUX_DIRECTORY,
    LINUX_STAGE, LOADER_FILE, LOAD_FILE, PACKAGE_DIRECTORY, PACKAGE_FOLDER, SCAN_FILE,
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

/// The most seconds that FINISH.SH waits for the camera to mount the card.
const FINISH_WAIT_SECONDS: u32 = 60;

/// The runtime files of a package with the core file of each runtime build of
/// a camera, in release order, for `icon_stride`-byte icon slots. The builds of
/// a camera have the same scripts and plan.
///
/// The loader runs the scripts on Linux (`crate::card`). `LOAD.SH` runs
/// first, on the mount of the card that the loader made, or on the mount of
/// the camera when the camera was first: it copies the package folder and
/// the data folder into the RAM copy of the card, unmounts the card, and
/// runs `SCAN.SH` there. The window with the mount is short, because the
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
    let wrapper = |body: String, mode: &str, exit: &str| {
        format!(
            "{body}echo GR_MODULE_{}_EXIT_$status\nexit {exit}\n",
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

/// `LOAD.SH`: the RAM copy of the card, then SCAN. Its argument is the mount
/// of the card that the loader made, or none for the mount of the camera.
///
/// The copy of a file through the card file system costs about 30 ms on a
/// camera, so the script copies the effect folders only when SCAN needs
/// them for a new plan: it copies the plan of an earlier start with its key
/// and a listing of the folders, and SCAN keeps that plan when the key
/// matches (`installer.c`: plan_key).
fn load_script(count: usize) -> String {
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
        "# The Linux side of the start: the card into RAM, then SCAN.\n\
         src=${{1:-{LINUX_CARD}}}\n\
         if mkdir {LINUX_STAGE} 2>/dev/null; then\n\
         \x20 mount -t tmpfs -o size={STAGE_SIZE_MB}m tmpfs {LINUX_STAGE}\n\
         \x20 mkdir -p {package} {data}\n\
         \x20 for f in PACKAGE.TXT SCAN.SH INSTALL.SH FINISH.SH {cores}; do cp $src/{package_dir}/$f {package}/; done\n\
         \x20 chmod 755 {package}/CORE*.BIN\n\
         \x20 [ -d $src/{package_dir}/MODULES ] && cp -r $src/{package_dir}/MODULES {package}/\n\
         \x20 for f in {data_files}; do [ -e $src/{data_dir}/$f ] && cp $src/{data_dir}/$f {data}/; done\n\
         \x20 [ -d $src/{data_dir}/MODULES ] && cp -r $src/{data_dir}/MODULES {data}/\n\
         \x20 # The state of GRKit 1.2, which INSTALL reads when GRKIT has no state file.\n\
         \x20 for f in STATE.TXT STATE.TMP; do [ -e $src{LEGACY_DIRECTORY}/$f ] && mkdir -p {LINUX_STAGE}{LEGACY_DIRECTORY} && cp $src{LEGACY_DIRECTORY}/$f {LINUX_STAGE}{LEGACY_DIRECTORY}/; done\n\
         \x20 # The plan of an earlier start, and the listing of the folders that keys it.\n\
         \x20 [ -e $src/{data_dir}/PLAN.BIN ] && cp $src/{data_dir}/PLAN.BIN {package}/PLAN.BIN\n\
         \x20 dirs=\n\
         \x20 for d in $src/{data_dir}/EFFECTS $src/{package_dir}/EFFECTS $src/{data_dir}/MODULES $src/{package_dir}/MODULES; do [ -d $d ] && dirs=\"$dirs $d\"; done\n\
         \x20 {{ echo $dirs; ls -le $dirs || ls -l $dirs; }} >{data}/LIST.TXT 2>&1\n\
         \x20 sh {package}/SCAN.SH\n\
         \x20 if [ $? = {stale} ]; then\n\
         \x20   # A new plan: the effect folders, and the effects of GRKit 1.2 when GRKIT has no library.\n\
         \x20   mkdir -p {package}/EFFECTS\n\
         \x20   for d in EFFECTS; do [ -d $src/{package_dir}/$d ] && cp -r $src/{package_dir}/$d {package}/; done\n\
         \x20   for d in EFFECTS; do [ -d $src/{data_dir}/$d ] && cp -r $src/{data_dir}/$d {data}/; done\n\
         \x20   if [ ! -e $src/{data_dir}/STATE.TXT ] || [ ! -d $src/{data_dir}/EFFECTS ]; then\n\
         \x20     [ -d $src{LEGACY_DIRECTORY}/EFFECTS ] && mkdir -p {LINUX_STAGE}{LEGACY_DIRECTORY} && cp -r $src{LEGACY_DIRECTORY}/EFFECTS {LINUX_STAGE}{LEGACY_DIRECTORY}/\n\
         \x20   fi\n\
         \x20   sh {package}/SCAN.SH\n\
         \x20 fi\n\
         fi\n\
         # Release the mount of the loader before the camera mounts the card. If\n\
         # the camera tried in the meantime, its mount failed: keep this one for\n\
         # it, because an unmount now would stop Linux.\n\
         if [ $src != {LINUX_CARD} ]; then\n\
         \x20 if grep -q 'mnt {LINUX_CARD}' /var/log/messages && ! grep -q storage1 /proc/mounts; then\n\
         \x20   mount --move $src {LINUX_CARD}\n\
         \x20 else\n\
         \x20   umount $src\n\
         \x20 fi\n\
         \x20 # A file in the mount point keeps it, so a retry of the loader mounts no more.\n\
         \x20 grep -q \" $src \" /proc/mounts || touch $src/staged\n\
         fi\n\
         exit 0\n",
        data_files = data_files.join(" "),
        stale = STATUS_PLAN_STALE,
    )
}

/// `FINISH.SH`: the results to the card after the camera mounts it, in a
/// process of its own, so the request of the loader returns at once.
fn finish_script(core: &str) -> String {
    let package = format!("{LINUX_STAGE}{PACKAGE_DIRECTORY}");
    let data = format!("{LINUX_STAGE}{DATA_DIRECTORY}");
    let card_package = format!("{LINUX_CARD}{PACKAGE_DIRECTORY}");
    let card_data = format!("{LINUX_CARD}{DATA_DIRECTORY}");
    format!(
        "# After INSTALL: the results to the card, once the camera mounts it.\n\
         (\n\
         \x20 n=0\n\
         \x20 while ! grep -q storage1 /proc/mounts; do\n\
         \x20   usleep 200000\n\
         \x20   n=$((n + 1))\n\
         \x20   [ $n -gt {waits} ] && exit 0\n\
         \x20 done\n\
         \x20 for f in INSTALL.OK INSTALL.ERR; do [ -e {package}/$f ] && cp {package}/$f {card_package}/; done\n\
         \x20 mkdir -p {card_data}\n\
         \x20 # The plan and its key, for the next start, then the other results: each only on a change.\n\
         \x20 [ -e {package}/PLAN.BIN ] && {{ cmp -s {package}/PLAN.BIN {card_data}/PLAN.BIN || cp {package}/PLAN.BIN {card_data}/; }}\n\
         \x20 for f in PLAN.KEY REPORT.TXT RELOC.BIN; do\n\
         \x20   [ -e {data}/$f ] || continue\n\
         \x20   cmp -s {data}/$f {card_data}/$f || cp {data}/$f {card_data}/\n\
         \x20 done\n\
         \x20 {card_package}/{core} --import {LINUX_CARD}\n\
         \x20 umount {LINUX_STAGE}\n\
         \x20 rm -rf {LINUX_STAGE}\n\
         ) </dev/null >/dev/null 2>&1 &\n\
         exit 0\n",
        waits = FINISH_WAIT_SECONDS * 5,
    )
}
