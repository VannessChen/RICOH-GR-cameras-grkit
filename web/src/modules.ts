import { PLUGINS, type PluginKey } from './generated/plugins';

/**
 * The plugins of the ADJ menu and the package rules for them. A plugin that the
 * package omits (`PLUGINS.TXT`, through `Package.omitPlugin`) is not installed
 * and changes no firmware. A plugin that needs effects (Image Control) is not
 * installed without them. `generated/plugins.ts` comes from the registry in
 * `crates/grkit-core/src/plugins.rs`.
 */
export const MODULES = PLUGINS;

export type ModuleKey = PluginKey;

export const ALL_MODULES: ReadonlySet<ModuleKey> = new Set(MODULES.map((module) => module.key));

export interface ModulePlan {
  /** The package includes the selected effects only when Image Control is on. */
  includeEffects: boolean;
  /** Names for `Package.omitPlugin`. */
  omitted: string[];
  /** The plugins that the camera menu shows, in menu order. */
  visible: ModuleKey[];
}

/**
 * `available` names the plugins that the site offers for the camera, in menu order.
 * Plugins outside it are not shown. Of those, the package omits only the
 * `excluded` plugins, which the runtime has but the camera must not install.
 */
export function planModules(
  enabled: ReadonlySet<ModuleKey>,
  effectCount: number,
  available: readonly ModuleKey[] = MODULES.map((module) => module.key),
  excluded: readonly ModuleKey[] = [],
): ModulePlan {
  const offered = MODULES.filter((module) => available.includes(module.key));
  const includeEffects = offered.some((module) => module.needsEffects && enabled.has(module.key));
  return {
    includeEffects,
    omitted: MODULES.flatMap((module) =>
      excluded.includes(module.key) || (offered.includes(module) && !enabled.has(module.key))
        ? [module.plugin]
        : [],
    ),
    visible: offered
      .filter((module) => enabled.has(module.key) && (!module.needsEffects || effectCount > 0))
      .map((module) => module.key),
  };
}
