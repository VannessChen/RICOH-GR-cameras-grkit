import { CAMERA_PLUGINS } from './generated/plugins';
import type { ModuleKey } from './modules';

export interface Camera {
  /** The camera of the package (`camera.rs`): it selects the ModeSet file and the guide. */
  id: keyof typeof CAMERA_PLUGINS;
  name: string;
  /**
   * The supported firmware version, the site target that holds its runtime, and the ModeSet file
   * that enables SD scripts. A camera without it is planned and cannot be selected.
   */
  support?: {
    firmware: string;
    target: string;
    modeset: string;
    /** True until the runtime has been tested on a camera. */
    experimental: boolean;
    /** The modules that the runtime has, in menu order (`camera.rs`, through `CAMERA_PLUGINS`). */
    modules: readonly ModuleKey[];
    /** Framing offers only XPan, as a crop of native 16:9. */
    xpanOnly?: boolean;
  };
}

// Each HDF model runs the firmware of its base camera, so it uses the same package.
export const CAMERAS: readonly Camera[] = [
  {
    id: 'gr3',
    name: 'GR III / GR III HDF',
    support: {
      firmware: '2.10',
      target: 'gr3-2.10',
      modeset: '00078350.588',
      experimental: false,
      modules: CAMERA_PLUGINS.gr3,
    },
  },
  // GR IIIx 1.60 runs the GR III 2.10 code, so it uses the same runtime.
  {
    id: 'gr3x',
    name: 'GR IIIx / GR IIIx HDF',
    support: {
      firmware: '1.60',
      target: 'gr3-2.10',
      modeset: '00078490.609',
      experimental: false,
      modules: CAMERA_PLUGINS.gr3x,
    },
  },
  {
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
];

/** The token in the install steps that stands for the camera's ModeSet file. */
export const MODESET_TOKEN = '`{modeset}';

/** The target whose effects and previews the site shows. */
export const PREVIEW_TARGET = 'gr3-2.10';
