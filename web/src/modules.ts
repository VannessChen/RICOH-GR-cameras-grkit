/**
 * The plugins of the ADJ menu and the package rules for them. A plugin that the
 * package omits (`PLUGINS.TXT`, through `Package.omitPlugin`) is not installed
 * and changes no firmware. Image Control also needs effects: without them, the
 * installer does not install it.
 */
export const MODULES = [
  { key: 'imageControl', name: 'Image Control', plugin: 'image-control' },
  { key: 'framing', name: 'Framing', plugin: 'framing' },
  { key: 'guides', name: 'Guides', plugin: 'guides' },
  { key: 'dateStamp', name: 'Date Stamp', plugin: 'date-stamp' },
  { key: 'grain', name: 'Grain', plugin: 'grain' },
] as const;

export type ModuleKey = (typeof MODULES)[number]['key'];

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
 * `available` names the plugins that the camera's runtime has, in menu order.
 * Plugins outside it are neither shown nor omitted.
 */
export function planModules(
  enabled: ReadonlySet<ModuleKey>,
  effectCount: number,
  available: readonly ModuleKey[] = MODULES.map((module) => module.key),
): ModulePlan {
  const offered = MODULES.filter((module) => available.includes(module.key));
  const includeEffects = available.includes('imageControl') && enabled.has('imageControl');
  return {
    includeEffects,
    omitted: offered.flatMap((module) => (enabled.has(module.key) ? [] : [module.plugin])),
    visible: offered
      .filter((module) =>
        module.key === 'imageControl' ? includeEffects && effectCount > 0 : enabled.has(module.key),
      )
      .map((module) => module.key),
  };
}
