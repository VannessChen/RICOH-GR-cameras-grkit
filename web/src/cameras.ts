import { CAMERA_RELEASES } from './generated/cameras';
import { CAMERA_PLUGINS } from './generated/plugins';
import type { ModuleKey } from './modules';

export { PREVIEW_TARGET } from './generated/site';

/** One runtime build of a camera: the package for the firmware versions that it runs. */
export interface Build {
  /** The firmware versions that run this build (`camera.rs` releases of the target). */
  firmware: string;
  /** The site target that holds its runtime. */
  target: string;
}

export interface Camera {
  /** The entry on the site. Two entries can share one package camera. */
  key: string;
  /** The camera of the package (`camera.rs`): it selects the ModeSet file and the guide. */
  id: keyof typeof CAMERA_PLUGINS;
  name: string;
  /**
   * The supported firmware versions (`camera.rs` releases), the runtime builds for them, and the
   * ModeSet file that enables SD scripts (`support`). A camera without it is planned and cannot be
   * selected.
   */
  support?: {
    /** All supported versions, as the camera entry shows them. */
    firmware: string;
    /** One package for each build. With more than one, the user selects the firmware version. */
    builds: readonly Build[];
    modeset: string;
    /** True until the runtime has been tested on a camera with one of the versions. */
    experimental: boolean;
    /** The modules that the site offers, in menu order: the plugins of the runtime
     * (`camera.rs`, through `CAMERA_PLUGINS`) without `excluded`. */
    modules: readonly ModuleKey[];
    /** Plugins of the runtime that the package always omits, so the installer never installs them. */
    excluded?: readonly ModuleKey[];
  };
}

type Releases = Pick<NonNullable<Camera['support']>, 'firmware' | 'builds' | 'modeset' | 'experimental'>;

/**
 * The firmware versions of a camera (`camera.rs`), its runtime builds in release order, its ModeSet
 * file, and whether the entry is experimental: it is while no version has been tested on a camera.
 */
export function releases(id: keyof typeof CAMERA_RELEASES): Releases {
  const camera = CAMERA_RELEASES[id];
  const targets = [...new Set(camera.releases.map((item) => item.target))];
  return {
    firmware: camera.releases.map((item) => item.firmware).join(' / '),
    builds: targets.map((target) => ({
      firmware: camera.releases
        .filter((item) => item.target === target)
        .map((item) => item.firmware)
        .join(' / '),
      target,
    })),
    modeset: camera.modeset,
    experimental: camera.releases.every((item) => item.experimental),
  };
}

/** The build of the selected target, or the only build of a camera with one. */
export function selectedBuild(camera: Camera, target: string | undefined): Build | undefined {
  const builds = camera.support?.builds ?? [];
  return builds.length === 1 ? builds[0] : builds.find((build) => build.target === target);
}

// Each HDF model runs the firmware of its base camera, so it uses the same package.
export const CAMERAS: readonly Camera[] = [
  {
    key: 'gr3',
    id: 'gr3',
    name: 'GR III / GR III HDF',
    // 2.20 changes only the end screen; the installer relocates the 2.10 runtime to it.
    support: { ...releases('gr3'), modules: CAMERA_PLUGINS.gr3 },
  },
  // GR IIIx 1.60 and 1.70 run the GR III 2.10 and 2.20 code, so they use the same runtime.
  {
    key: 'gr3x',
    id: 'gr3x',
    name: 'GR IIIx / GR IIIx HDF',
    support: { ...releases('gr3x'), modules: CAMERA_PLUGINS.gr3x },
  },
  // GR IV 1.11 and 1.20 have a runtime each (`gr4-1.11`, `gr4-1.20`), so the user selects the
  // firmware version.
  {
    key: 'gr4',
    id: 'gr4',
    name: 'GR IV / GR IV HDF',
    support: { ...releases('gr4'), modules: CAMERA_PLUGINS.gr4 },
  },
  // GR IV Monochrome runs the GR IV firmware with its own version: it shows 1.11 as 1.00 and 1.20
  // as 1.10. It uses the GR IV packages and ModeSet file. The effects are made for color sensors,
  // so its packages have no Image Control.
  {
    key: 'gr4m',
    id: 'gr4',
    name: 'GR IV Monochrome',
    support: {
      ...releases('gr4'),
      firmware: '1.00 / 1.10',
      builds: [
        { firmware: '1.00', target: 'gr4-1.11' },
        { firmware: '1.10', target: 'gr4-1.20' },
      ],
      modules: CAMERA_PLUGINS.gr4.filter((key) => key !== 'imageControl'),
      excluded: ['imageControl'],
    },
  },
];

/** The token in the install steps that stands for the camera's ModeSet file. */
export const MODESET_TOKEN = '`{modeset}';
