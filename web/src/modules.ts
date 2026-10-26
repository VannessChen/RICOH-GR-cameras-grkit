import { OFFICIAL_MODULES, type ModuleKey } from './generated/modules';

/**
 * The official modules of the ADJ menu and the package rules for them. The package holds the
 * module file of each selected module (`Package.addModule`), and the camera installs only those.
 * A module that needs effects (Image Control) shows nothing without them, so the package leaves
 * it out then. `generated/modules.ts` comes from `crates/grkit-core/src/modules.rs`.
 */
export const MODULES = OFFICIAL_MODULES;

export type { ModuleKey };

export const ALL_MODULES: ReadonlySet<ModuleKey> = new Set(MODULES.map((module) => module.key));

export interface ModulePlan {
  /** The package includes the selected effects only when Image Control is on. */
  includeEffects: boolean;
  /** The IDs of the module files that the package holds (`Package.addModule`), in menu order. */
  modules: string[];
  /** The modules that the camera menu shows, in menu order. */
  visible: ModuleKey[];
}

/**
 * `available` names the modules that the site offers for the camera, in menu order. The package
 * holds the enabled ones among them that show something.
 */
export function planModules(
  enabled: ReadonlySet<ModuleKey>,
  effectCount: number,
  available: readonly ModuleKey[] = MODULES.map((module) => module.key),
): ModulePlan {
  const offered = MODULES.filter((module) => available.includes(module.key));
  const includeEffects = offered.some((module) => module.needsEffects && enabled.has(module.key));
  const shown = offered.filter(
    (module) => enabled.has(module.key) && (!module.needsEffects || effectCount > 0),
  );
  return {
    includeEffects,
    modules: shown.map((module) => module.id),
    visible: shown.map((module) => module.key),
  };
}
