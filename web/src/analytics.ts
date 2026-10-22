import type { Camera } from './cameras';
import type { Language } from './i18n';
import type { ModuleKey, ModulePlan } from './modules';

type Props = Record<string, string>;

declare global {
  interface Window {
    /** Set only when the build adds Plausible (PLAUSIBLE_HOST). */
    plausible?: (event: string, options: { props: Props }) => void;
  }
}

/** Sends a custom event to Plausible. Without Plausible, it does nothing. */
function track(event: string, props: Props): void {
  window.plausible?.(event, { props });
}

/**
 * The properties of a GRKit download: the camera, the effect count, the page language, and
 * `on` or `off` for each module that the site offers for the camera. A module is `on` only
 * when the package installs it, so Image Control without effects is `off`.
 */
export function downloadProps(
  camera: Camera,
  plan: ModulePlan,
  effectCount: number,
  language: Language,
): Props {
  const props: Props = { camera: camera.key, effects: String(effectCount), language };
  for (const key of camera.support?.modules ?? ([] as readonly ModuleKey[])) {
    props[key] = plan.visible.includes(key) ? 'on' : 'off';
  }
  return props;
}

/** Records a GRKit package that the page saved. */
export function trackDownload(
  camera: Camera,
  plan: ModulePlan,
  effectCount: number,
  language: Language,
): void {
  track('Download', downloadProps(camera, plan, effectCount, language));
}

/** Records a saved package that turns SD scripts off. */
export function trackScriptsOff(camera: Camera): void {
  track('Scripts Off', { camera: camera.key });
}
