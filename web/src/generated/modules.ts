// Generated from crates/grkit-core/src/modules.rs. Do not edit: change the
// list there, then run
// GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated.

/** The official modules in menu order. `needsEffects`: the module shows only with effects. */
export const OFFICIAL_MODULES = [
  { key: 'imageControl', id: 'IMGCTL', needsEffects: true },
  { key: 'framing', id: 'FRAMING', needsEffects: false },
  { key: 'guides', id: 'GUIDES', needsEffects: false },
  { key: 'dateStamp', id: 'DATE', needsEffects: false },
  { key: 'grain', id: 'GRAIN', needsEffects: false },
] as const;

export type ModuleKey = (typeof OFFICIAL_MODULES)[number]['key'];

/** The official modules that each camera can install, in menu order. */
export const CAMERA_MODULES = {
  gr3: ['imageControl', 'framing', 'guides', 'dateStamp', 'grain'],
  gr3x: ['imageControl', 'framing', 'guides', 'dateStamp', 'grain'],
  gr4: ['imageControl', 'framing', 'guides', 'dateStamp', 'grain'],
} as const satisfies Readonly<Record<string, readonly ModuleKey[]>>;
