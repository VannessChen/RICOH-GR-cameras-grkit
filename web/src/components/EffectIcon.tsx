import { useCallback } from 'react';
import type { Effect } from '../catalog';

const SIZE = 40;

/** Paints base64 40 x 40 RGBA pixels. A malformed icon leaves the canvas empty. */
function paint(canvas: HTMLCanvasElement | null, icon: string) {
  const context = canvas?.getContext('2d');
  if (!context) {
    return;
  }
  const pixels = Uint8ClampedArray.from(atob(icon), (character) => character.charCodeAt(0));
  if (pixels.length === SIZE * SIZE * 4) {
    context.putImageData(new ImageData(pixels, SIZE, SIZE), 0, 0);
  }
}

/** The ADJ icon of an effect, with the pixels that the camera shows. */
export function EffectIcon({ effect }: { effect: Effect }) {
  const { icon } = effect;
  const draw = useCallback((canvas: HTMLCanvasElement | null) => paint(canvas, icon), [icon]);
  return <canvas ref={draw} className="effect-icon" width={SIZE} height={SIZE} aria-hidden="true" />;
}
