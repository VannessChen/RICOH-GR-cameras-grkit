//! The runtime files of a package: the startup and loader scripts, the
//! package scripts, the core files, and the files that turn SD scripts on or
//! off.

use super::loader::{core_path, loader_for};
use super::ICON_STRIDE;
use crate::camera::Camera;
use crate::card::{INSTALL_FILE, LINUX_DIRECTORY, LOADER_FILE, PACKAGE_FOLDER, SCAN_FILE};
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

/// The runtime files of a package with the core file of each runtime build of
/// a camera, in release order, for `icon_stride`-byte icon slots. The builds of
/// a camera have the same scripts and plan, so the first core file scans.
///
/// With one core file, `INSTALL.SH` runs it with `--install`. With more, it
/// runs each with `--known` first, which reads one word to find the build. If
/// no core file names the build, it tries each one with `--try`, which runs
/// the relocation, and the last one with `--install`, which writes
/// `INSTALL.ERR` if it also fails. A known build thus installs without a relocation of
/// another build, and an unknown build still gets each runtime.
///
/// After Linux deletes a file in the folder, the camera does not find the next
/// new name that it looks up there, and it keeps that miss. So `SCAN.SH`, the
/// first script of a power-on, looks up each core file before it scans.
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
    let wrapper = |body: String, mode: &str| {
        format!(
            "{body}echo GR_MODULE_{}_EXIT_$status\nexit 0\n",
            mode.to_uppercase()
        )
        .into_bytes()
    };
    let single = |mode: &str| format!("{LINUX_DIRECTORY}/{} --{mode}\nstatus=$?\n", name(0));
    let names = |range: std::ops::Range<usize>| range.map(name).collect::<Vec<_>>().join(" ");
    let (scan, install) = match count {
        1 => (single("scan"), single("install")),
        count => (
            format!(
                "for core in {}; do [ -e {LINUX_DIRECTORY}/$core ]; done\n{}",
                names(1..count),
                single("scan")
            ),
            format!(
                "for core in {all}; do\n\
                 \x20 {LINUX_DIRECTORY}/$core --known\n\
                 \x20 status=$?\n\
                 \x20 case $status in {other}) ;; *) break ;; esac\n\
                 done\n\
                 case $status in {other})\n\
                 \x20 for core in {tried}; do\n\
                 \x20   {LINUX_DIRECTORY}/$core --try\n\
                 \x20   status=$?\n\
                 \x20   case $status in {relocation}) ;; *) break ;; esac\n\
                 \x20 done\n\
                 \x20 case $status in {relocation}) {LINUX_DIRECTORY}/{last} --install; status=$? ;; esac\n\
                 \x20 ;;\n\
                 esac\n",
                all = names(0..count),
                tried = names(0..count - 1),
                last = name(count - 1),
                other = STATUS_OTHER_BUILD,
                relocation = STATUS_RELOCATION,
            ),
        ),
    };
    let mut files = Files::from([
        ("SCRIPT/STARTUP.TTL".into(), startup.into_bytes()),
        (LOADER_FILE.into(), loader_for(icon_stride)),
        (SCAN_FILE.into(), wrapper(scan, "scan")),
        (INSTALL_FILE.into(), wrapper(install, "install")),
    ]);
    for (index, binary) in binaries.iter().enumerate() {
        files.insert(core_path_of(index, count), binary.to_vec());
    }
    files
}
