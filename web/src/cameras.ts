import { CAMERA_RELEASES } from './generated/cameras';
import { CAMERA_MODULES } from './generated/modules';
import type { ModuleKey } from './modules';

export { PREVIEW_TARGET } from './generated/site';

export interface Camera {
  /** The entry on the site. Two entries can share one package camera. */
  key: string;
  /** The camera of the package (`camera.rs`): it selects the ModeSet file and the guide. */
  id: keyof typeof CAMERA_MODULES;
  name: string;
  /**
   * The supported firmware versions (`camera.rs` releases), the runtime builds for them, and the
   * ModeSet file that enables SD scripts (`support`). A camera without it is planned and cannot be
   * selected.
   */
  support?: {
    /** All supported versions, as the camera entry shows them. */
    firmware: string;
    /**
     * The site targets of the runtime builds, in release order. The package holds the installer
     * of each, and the camera selects the one for its build at each power-on.
     */
    targets: readonly string[];
    modeset: string;
    /** True until the runtime has been tested on a camera with one of the versions. */
    experimental: boolean;
    /** The modules that the site offers, in menu order: those of the camera
     * (`modules.rs`, through `CAMERA_MODULES`), less any that the entry does not offer. */
    modules: readonly ModuleKey[];
    /** True when the entry offers no Image Control, so its package has no effects. */
    noEffects?: boolean;
  };
}

type Releases = Pick<NonNullable<Camera['support']>, 'firmware' | 'targets' | 'modeset' | 'experimental'>;

/**
 * The firmware versions of a camera (`camera.rs`), each once, its runtime builds in release order,
 * its ModeSet file, and whether the entry is experimental: it is while no version has been tested
 * on a camera. Two builds of one version are two releases with that version.
 */
export function releases(id: keyof typeof CAMERA_RELEASES): Releases {
  const camera = CAMERA_RELEASES[id];
  return {
    firmware: [...new Set(camera.releases.map((item) => item.firmware))].join(' / '),
    targets: [...new Set(camera.releases.map((item) => item.target))],
    modeset: camera.modeset,
    experimental: camera.releases.every((item) => item.experimental),
  };
}

// Each HDF model runs the firmware of its base camera, so it uses the same package.
export const CAMERAS: readonly Camera[] = [
  {
    key: 'gr3',
    id: 'gr3',
    name: 'GR III / GR III HDF',
    // 2.20 changes only the end screen; the installer relocates the 2.10 runtime to it.
    support: { ...releases('gr3'), modules: CAMERA_MODULES.gr3 },
  },
  // GR IIIx 1.60 and 1.70 run the GR III 2.10 and 2.20 code, so they use the same runtime.
  {
    key: 'gr3x',
    id: 'gr3x',
    name: 'GR IIIx / GR IIIx HDF',
    support: { ...releases('gr3x'), modules: CAMERA_MODULES.gr3x },
  },
  // GR IV 1.11 and the two builds of 1.20 have a runtime each. The package holds all three.
  {
    key: 'gr4',
    id: 'gr4',
    name: 'GR IV / GR IV HDF',
    support: { ...releases('gr4'), modules: CAMERA_MODULES.gr4 },
  },
  // GR IV Monochrome runs the GR IV firmware with its own version: it shows 1.11 as 1.00 and 1.20
  // as 1.10. It uses the GR IV package and ModeSet file. The effects are made for color sensors,
  // so its package has no Image Control.
  {
    key: 'gr4m',
    id: 'gr4',
    name: 'GR IV Monochrome',
    support: {
      ...releases('gr4'),
      firmware: '1.00 / 1.10',
      modules: CAMERA_MODULES.gr4.filter((key) => key !== 'filmLook'),
      noEffects: true,
    },
  },
];

/** The token in the install steps that stands for the camera's ModeSet file. */
export const MODESET_TOKEN = '`{modeset}';
