import { useCallback } from 'react';
import type { Cover, Effect } from '../catalog';

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

/**
 * Compresses a text horizontally when it is wider than its `data-width`. The width is measured
 * again when the web fonts load, because the first measurement can use a fallback font.
 */
function fit(text: SVGTextElement | null) {
  if (!text) {
    return;
  }
  const measure = () => {
    text.removeAttribute('textLength');
    const width = Number(text.dataset.width);
    if (text.getComputedTextLength() > width) {
      text.setAttribute('textLength', String(width));
      text.setAttribute('lengthAdjust', 'spacingAndGlyphs');
    }
  };
  measure();
  void document.fonts?.ready.then(measure);
}

// The shades of the camera icon (`artwork.rs`).
const LIGHT = 'rgb(214 214 214)';
const DARK = 'rgb(40 40 40)';
const BODY = 'rgb(51 51 51)';

/**
 * A film cover in the layout of the camera icon: the brand on a light band, then the film name
 * and a large number on a dark body. A professional film box has a light band at the bottom, and
 * a motion picture film label has three light stripes.
 */
function CoverIcon({ cover }: { cover: Cover }) {
  const stripes = cover.template === 'cine';
  const band = cover.template === 'box' && cover.band;
  // The number sits higher above the stripes, so that it keeps a gap to the first one.
  const baseline = stripes ? 30.2 : band ? 32 : 33;
  return (
    <svg className="effect-icon" viewBox="0 0 40 40" aria-hidden="true">
      <rect x="0" y="3" width="40" height="34" fill={BODY} />
      <rect x="0" y="3" width="40" height="9" fill={LIGHT} />
      {band && <rect x="0" y="34" width="40" height="3" fill={LIGHT} />}
      {stripes &&
        [32, 34, 36].map((y) => <rect key={y} x="0" y={y} width="40" height="1" fill={LIGHT} />)}
      <text ref={fit} data-width="34" x="20" y="10.4" className="cover-brand" fill={DARK}>
        {cover.brand}
      </text>
      <text ref={fit} data-width="38" x="20" y="19" className="cover-name" fill="#fff">
        {cover.name}
      </text>
      <text ref={fit} data-width="36" x="20" y={baseline} className="cover-number" fill="#fff">
        {cover.number}
      </text>
    </svg>
  );
}

/** The ADJ icon of an effect. A film cover draws with web fonts, other icons with camera pixels. */
export function EffectIcon({ effect }: { effect: Effect }) {
  const { icon, cover } = effect;
  const draw = useCallback((canvas: HTMLCanvasElement | null) => paint(canvas, icon), [icon]);
  if (cover) {
    return <CoverIcon cover={cover} />;
  }
  return <canvas ref={draw} className="effect-icon pixels" width={SIZE} height={SIZE} aria-hidden="true" />;
}
