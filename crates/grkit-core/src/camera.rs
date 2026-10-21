//! Cameras that a package can target.
//!
//! GR IIIx 1.60 runs the RTOS code of GR III 2.10. The two RTOS images differ only
//! in the model name, version, build time, default year, ModeSet numbers, and one
//! byte of the model selection, and none of these bytes is a GRKit patch site or
//! input. A runtime built for `gr3-2.10` is therefore valid on both cameras. The
//! cameras differ only in the ModeSet file that enables SD scripts.
//!
//! A camera lists each firmware release that it can run with the runtime build
//! for it. GR III 2.20 and GR IIIx 1.70 change only the end screen; their code
//! moves by less than the relocation radius, so the installer relocates the
//! `gr3-2.10` runtime to them (docs/architecture.md, Relocation). This was
//! not tested on a camera.
//!
//! Each camera has an HDF model that runs the same firmware and ModeSet file:
//! Ricoh publishes one firmware for both. The RTOS selects the HDF model from a
//! factory adjustment, not from the ModeSet numbers. A package for a camera is
//! therefore also the package for its HDF model.
//!
//! GR IV 1.11 is a different build with its own runtime. It has Image Control,
//! Framing (XPan only, cropped from native 16:9), Guides, Date Stamp, and Grain.
//! It has been tested on a GR IV camera, but not on a GR IV HDF camera.

use anyhow::{bail, Result};

use crate::modules;

/// A firmware version that a camera runs, and the runtime build for it.
#[derive(Debug, PartialEq, Eq)]
pub struct Release {
    pub firmware: &'static str,
    pub target: &'static str,
    /// True until the runtime has been tested on a camera with this release.
    pub experimental: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Camera {
    /// The ID that the CLI, the site, and `grkit verify` use.
    pub id: &'static str,
    pub name: &'static str,
    /// The HDF model, which runs the same firmware and ModeSet file.
    pub hdf: &'static str,
    /// The firmware releases that a GRKit runtime supports, oldest first.
    pub releases: &'static [Release],
    /// The ModeSet command file: the major and minor numbers of the RTOS.
    pub modeset: &'static str,
    /// True until the runtime has been tested on a camera.
    pub experimental: bool,
    /// True when the runtime has Image Control, so a package can have effects.
    pub effects: bool,
    /// The plugins of `plugins::NAMES` that the runtime has. A package cannot
    /// select the others.
    pub plugins: &'static [&'static str],
}

pub const GR3: Camera = Camera {
    id: "gr3",
    name: "GR III",
    hdf: "GR III HDF",
    releases: &[
        Release {
            firmware: "2.10",
            target: "gr3-2.10",
            experimental: false,
        },
        Release {
            firmware: "2.20",
            target: "gr3-2.10",
            experimental: true,
        },
    ],
    modeset: "00078350.588",
    experimental: false,
    effects: true,
    plugins: &modules::NAMES,
};

pub const GR3X: Camera = Camera {
    id: "gr3x",
    name: "GR IIIx",
    hdf: "GR IIIx HDF",
    releases: &[
        Release {
            firmware: "1.60",
            target: "gr3-2.10",
            experimental: false,
        },
        Release {
            firmware: "1.70",
            target: "gr3-2.10",
            experimental: true,
        },
    ],
    modeset: "00078490.609",
    experimental: false,
    effects: true,
    plugins: &modules::NAMES,
};

pub const GR4: Camera = Camera {
    id: "gr4",
    name: "GR IV",
    hdf: "GR IV HDF",
    releases: &[Release {
        firmware: "1.11",
        target: "gr4-1.11",
        experimental: false,
    }],
    modeset: "00078560.636",
    experimental: false,
    effects: true,
    plugins: &modules::NAMES,
};

pub const CAMERAS: [&Camera; 3] = [&GR3, &GR3X, &GR4];

impl Camera {
    /// The runtime builds that the camera runs, in release order, each once.
    pub fn targets(&self) -> Vec<&'static str> {
        let mut targets = Vec::new();
        for release in self.releases {
            if !targets.contains(&release.target) {
                targets.push(release.target);
            }
        }
        targets
    }

    /// Whether a runtime build runs on the camera.
    pub fn runs(&self, target: &str) -> bool {
        self.releases.iter().any(|release| release.target == target)
    }

    /// The firmware versions that run a runtime build, oldest first.
    pub fn firmware_for(&self, target: &str) -> Vec<&'static str> {
        self.releases
            .iter()
            .filter(|release| release.target == target)
            .map(|release| release.firmware)
            .collect()
    }

    /// The camera family: GR IV cameras have their own guide and layout.
    pub fn is_gr4(&self) -> bool {
        self.id == GR4.id
    }
}

pub fn find(id: &str) -> Result<&'static Camera> {
    match CAMERAS.iter().find(|camera| camera.id == id) {
        Some(camera) => Ok(camera),
        None => bail!(
            "Unknown camera {id:?}. Expected one of: {}",
            CAMERAS.map(|camera| camera.id).join(", ")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_gr3_releases_run_the_gr3_210_runtime() {
        assert_eq!(GR3.targets(), ["gr3-2.10"]);
        assert_eq!(GR3.firmware_for("gr3-2.10"), ["2.10", "2.20"]);
        assert_eq!(GR3X.firmware_for("gr3-2.10"), ["1.60", "1.70"]);
        assert!(GR3X.runs("gr3-2.10") && !GR3X.runs("gr4-1.11"));
        assert_eq!(GR4.targets(), ["gr4-1.11"]);
        assert!(CAMERAS
            .iter()
            .all(|camera| camera.releases.iter().all(|release| {
                !release.experimental
                    || camera
                        .releases
                        .iter()
                        .any(|tested| !tested.experimental && tested.target == release.target)
            })));
    }
}
