import { CAMERA_PLUGINS } from './generated/plugins';
import type { ModuleKey } from './modules';

export interface Camera {
  /** The entry on the site. Two entries can share one package camera. */
  key: string;
  /** The camera of the package (`camera.rs`): it selects the ModeSet file and the guide. */
  id: keyof typeof CAMERA_PLUGINS;
  name: string;
  /**
   * The supported firmware versions (`camera.rs` releases of the target), the site target that holds
   * its runtime, and the ModeSet file that enables SD scripts. A camera without it is planned and
   * cannot be selected.
   */
  support?: {
    firmware: string;
    target: string;
    modeset: string;
    /** True until the runtime has been tested on a camera. */
    experimental: boolean;
    /** The modules that the site offers, in menu order: the plugins of the runtime
     * (`camera.rs`, through `CAMERA_PLUGINS`) without `excluded`. */
    modules: readonly ModuleKey[];
    /** Plugins of the runtime that the package always omits, so the installer never installs them. */
    excluded?: readonly ModuleKey[];
    /** Framing offers only XPan, as a crop of native 16:9. */
    xpanOnly?: boolean;
  };
}

// Each HDF model runs the firmware of its base camera, so it uses the same package.
export const CAMERAS: readonly Camera[] = [
  {
    key: 'gr3',
    id: 'gr3',
    name: 'GR III / GR III HDF',
    support: {
      // 2.20 changes only the end screen; the installer relocates the 2.10 runtime to it.
      firmware: '2.10 / 2.20',
      target: 'gr3-2.10',
      modeset: '00078350.588',
      experimental: false,
      modules: CAMERA_PLUGINS.gr3,
    },
  },
  // GR IIIx 1.60 and 1.70 run the GR III 2.10 and 2.20 code, so they use the same runtime.
  {
    key: 'gr3x',
    id: 'gr3x',
    name: 'GR IIIx / GR IIIx HDF',
    support: {
      firmware: '1.60 / 1.70',
      target: 'gr3-2.10',
      modeset: '00078490.609',
      experimental: false,
      modules: CAMERA_PLUGINS.gr3x,
    },
  },
  {
    key: 'gr4',
    id: 'gr4',
    name: 'GR IV / GR IV HDF',
    support: {
      firmware: '1.11',
      target: 'gr4-1.11',
      modeset: '00078560.636',
      experimental: false,
      modules: CAMERA_PLUGINS.gr4,
      xpanOnly: true,
    },
  },
  // GR IV 1.20 has a runtime of its own (`gr4-1.20`), not tested on a camera yet.
  {
    key: 'gr4-120',
    id: 'gr4',
    name: 'GR IV / GR IV HDF',
    support: {
      firmware: '1.20',
      target: 'gr4-1.20',
      modeset: '00078560.636',
      experimental: true,
      modules: CAMERA_PLUGINS.gr4,
      xpanOnly: true,
    },
  },
  // GR IV Monochrome runs the GR IV 1.11 firmware, which it shows as version 1.00, and uses the
  // GR IV package and ModeSet file. The effects are made for color sensors, so its packages have
  // no Image Control.
  {
    key: 'gr4m',
    id: 'gr4',
    name: 'GR IV Monochrome',
    support: {
      firmware: '1.00',
      target: 'gr4-1.11',
      modeset: '00078560.636',
      experimental: true,
      modules: CAMERA_PLUGINS.gr4.filter((key) => key !== 'imageControl'),
      excluded: ['imageControl'],
      xpanOnly: true,
    },
  },
];

/** The token in the install steps that stands for the camera's ModeSet file. */
export const MODESET_TOKEN = '`{modeset}';

/** The target whose effects and previews the site shows. */
export const PREVIEW_TARGET = 'gr3-2.10';
