//! The folders and files of GRKit on the SD card. A package names a file by
//! its path on the card, the installers on the camera's Linux, and the loader
//! in the RTOS (`C:\`). The three spellings of each folder come from one
//! literal here.
//!
//! On Linux, the installers never read the card itself. At power-on, the
//! camera mounts the card about 2 s after Linux can run commands. The loader
//! runs `LOAD.SH` before that: it mounts the card itself, copies the package
//! folder and the data folder into `LINUX_STAGE`, a folder in RAM, and
//! unmounts the card at once. SCAN and INSTALL then take `LINUX_STAGE` as the
//! card root, and `FINISH.SH` copies their results to the card after the
//! camera mounts it. The installers take the root as an argument, so the
//! paths of this module that start with `/` are relative to that root.

/// The package folder, with `$separator` between its parts.
macro_rules! package_folder {
    ($separator:literal) => {
        concat!("SCRIPT", $separator, "GRKIT")
    };
}
/// The data folder.
macro_rules! data_folder {
    () => {
        "GRKIT"
    };
}
/// The RAM copy of the card on Linux: the root of SCAN and INSTALL.
macro_rules! linux_stage {
    () => {
        "/tmp/gc"
    };
}

/// The card on the camera's Linux, mounted by the camera.
pub const LINUX_CARD: &str = "/media/storage1";
/// The mount of the card that the loader makes before the camera mounts it.
pub const LINUX_MOUNT: &str = "/tmp/m";
/// The copy of `LOAD.SH` that the loader runs: a shell reads a script file
/// while it runs it, and that would keep the mount busy at its unmount.
pub const LINUX_LOAD_COPY: &str = "/tmp/l";
/// The RAM copy of the card on Linux: the root of SCAN and INSTALL.
pub const LINUX_STAGE: &str = linux_stage!();
/// On the slow path, the uptime at which Linux first ran a request of the
/// loader.
pub const LINUX_SLOW_START: &str = "/tmp/t0";
/// The marker files of the fast path in the package folder, so that a new
/// package starts on the slow path again. FAST.ON holds the measured time in
/// centiseconds from the first request to the camera's mount; FAST.OFF turns
/// the fast path off; FAST.TRY is a fast start that has not finished.
pub const FAST_ON: &str = "FAST.ON";
pub const FAST_OFF: &str = "FAST.OFF";
pub const FAST_TRY: &str = "FAST.TRY";
/// The longest card root that an installer accepts, in bytes.
pub const CARD_ROOT_MAX: usize = 32;
const _: () = assert!(LINUX_CARD.len() <= CARD_ROOT_MAX && LINUX_STAGE.len() <= CARD_ROOT_MAX);

/// The package folder on the card: the loader, the core, the package list,
/// the package modules in `MODULES`, and the inbox of the package effects in
/// `EFFECTS`, which SCAN copies into the effects library.
pub const PACKAGE_FOLDER: &str = package_folder!("/");
/// The package folder under a Linux root: the installers, their scripts, the
/// package files, and the files of each boot.
pub const PACKAGE_DIRECTORY: &str = concat!("/", package_folder!("/"));
/// The package folder in the RAM copy of the card, where the loader runs the
/// package scripts.
pub const LINUX_DIRECTORY: &str = concat!(linux_stage!(), "/", package_folder!("/"));
/// The package folder in the RTOS.
pub const RTOS_PACKAGE_FOLDER: &str = concat!("C:\\", package_folder!("\\"));

/// The data folder at the card root, in the RTOS: the state files
/// (`capabilities::StateFile`), `REPORT.TXT`, the effects library in
/// `EFFECTS`, and the modules that a person adds in `MODULES`. No package
/// holds it, so a copy of a new package, by replace or by merge, keeps it.
/// The resident loader writes the state files through the camera's own file
/// system, never from Linux while the camera may write photos.
pub const DATA_FOLDER: &str = concat!("C:\\", data_folder!());
/// The data folder under a Linux root.
pub const DATA_DIRECTORY: &str = concat!("/", data_folder!());
/// The folder of GRKit 1.2 and earlier under a Linux root: its state file and
/// its effects. INSTALL reads them once, when `DATA_FOLDER` has no state file
/// or no effects library.
pub const LEGACY_DIRECTORY: &str = "/SCRIPT/PLUGINS";

/// The files of the package folder on the card.
pub const LOADER_FILE: &str = concat!(package_folder!("/"), "/LOADER.TTL");
pub const CORE_FILE: &str = concat!(package_folder!("/"), "/CORE.BIN");
pub const LOAD_FILE: &str = concat!(package_folder!("/"), "/LOAD.SH");
pub const SCAN_FILE: &str = concat!(package_folder!("/"), "/SCAN.SH");
pub const INSTALL_FILE: &str = concat!(package_folder!("/"), "/INSTALL.SH");
pub const FINISH_FILE: &str = concat!(package_folder!("/"), "/FINISH.SH");
/// The package list (`runtime/install_library.h`): the file name of each
/// effect and module of the package, one on each line. Each package has it,
/// also without effects, so a merge copy of a new package over an old one
/// gives the same result as a replace copy: the installer ignores the
/// package files that the list does not name.
pub const PACKAGE_LIST: &str = concat!(package_folder!("/"), "/PACKAGE.TXT");
/// The inbox of the package effects on the card. SCAN copies a new package
/// effect into the effects library, `GRKIT/EFFECTS`, where a person adds or
/// deletes effect files.
pub const EFFECTS_FOLDER: &str = concat!(package_folder!("/"), "/EFFECTS");
/// The modules of a package (`grm`).
pub const MODULES_FOLDER: &str = concat!(package_folder!("/"), "/MODULES");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_spelling_names_the_same_folder() {
        assert_eq!(PACKAGE_DIRECTORY, format!("/{PACKAGE_FOLDER}"));
        assert_eq!(LINUX_DIRECTORY, format!("{LINUX_STAGE}{PACKAGE_DIRECTORY}"));
        assert_eq!(
            RTOS_PACKAGE_FOLDER,
            format!("C:\\{}", PACKAGE_FOLDER.replace('/', "\\"))
        );
        assert_eq!(
            DATA_DIRECTORY.strip_prefix('/'),
            DATA_FOLDER.strip_prefix("C:\\")
        );
        // The Linux folders are in RAM, apart from the card of the camera.
        for folder in [LINUX_MOUNT, LINUX_STAGE, LINUX_LOAD_COPY] {
            assert!(folder.starts_with("/tmp/"), "{folder}");
        }
        assert!(LINUX_CARD.starts_with("/media/"));
    }
}
