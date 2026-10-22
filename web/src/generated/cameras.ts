// Generated from crates/grkit-core/src/camera.rs. Do not edit: change the
// cameras there, then run
// GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated.

/** The ModeSet file of each camera, and the firmware releases that a
 * runtime build supports, oldest first. `experimental`: not yet tested on
 * a camera with the release. */
export const CAMERA_RELEASES = {
  gr3: {
    modeset: '00078350.588',
    releases: [
      { firmware: '2.10', target: 'gr3-2.10', experimental: false },
      { firmware: '2.20', target: 'gr3-2.10', experimental: true },
    ],
  },
  gr3x: {
    modeset: '00078490.609',
    releases: [
      { firmware: '1.60', target: 'gr3-2.10', experimental: false },
      { firmware: '1.70', target: 'gr3-2.10', experimental: true },
    ],
  },
  gr4: {
    modeset: '00078560.636',
    releases: [
      { firmware: '1.11', target: 'gr4-1.11', experimental: false },
      { firmware: '1.20', target: 'gr4-1.20', experimental: true },
    ],
  },
} as const;
