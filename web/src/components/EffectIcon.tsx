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

/** The smallest size of a large name, about the 4-pixel capitals of `artwork.rs`. */
const MIN_LARGE_SIZE = 5.5;
/** The capital height of the cover font as a share of its size. */
const CAP_HEIGHT = 0.7;

/**
 * Makes a large name smaller until it fits its `data-width`, and keeps its capitals centered on
 * the `data-center` row, as `artwork.rs` shrinks a name without a number. At the smallest size, a
 * name that is still too wide compresses horizontally.
 */
function fitLarge(text: SVGTextElement | null) {
  if (!text) {
    return;
  }
  const measure = () => {
    text.removeAttribute('textLength');
    text.style.removeProperty('font-size');
    const width = Number(text.dataset.width);
    const base = parseFloat(getComputedStyle(text).fontSize);
    const length = text.getComputedTextLength();
    const size = length > width ? Math.max(MIN_LARGE_SIZE, (base * width) / length) : base;
    text.style.fontSize = `${size}px`;
    text.setAttribute('y', String(Number(text.dataset.center) + (size * CAP_HEIGHT) / 2));
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
export function CoverIcon({ cover }: { cover: Cover }) {
  const stripes = cover.template === 'cine';
  const band = cover.template === 'box' && cover.band;
  // The number sits higher above the stripes, so that it keeps a gap to the first one.
  const baseline = stripes ? 30.2 : band ? 32 : 33;
  // The body runs from row 12 to the bottom band, the stripes, or row 37.
  const center = (12 + (stripes ? 32 : band ? 34 : 37)) / 2;
  return (
    <svg className="effect-icon" viewBox="0 0 40 40" aria-hidden="true">
      <rect x="0" y="3" width="40" height="34" fill={BODY} />
      <rect x="0" y="3" width="40" height="9" fill={LIGHT} />
      {band && <rect x="0" y="34" width="40" height="3" fill={LIGHT} />}
      {stripes &&
        [32, 34, 36].map((y) => <rect key={y} x="0" y={y} width="40" height="1" fill={LIGHT} />)}
      {/* A key per text measures it again when it changes, as in the import dialog. */}
      <text key={cover.brand} ref={fit} data-width="34" x="20" y="10.4" className="cover-brand" fill={DARK}>
        {cover.brand}
      </text>
      {cover.number ? (
        <>
          <text key={cover.name} ref={fit} data-width="38" x="20" y="19" className="cover-name" fill="#fff">
            {cover.name}
          </text>
          <text
            key={cover.number}
            ref={fit}
            data-width="36"
            x="20"
            y={baseline}
            className="cover-number"
            fill="#fff"
          >
            {cover.number}
          </text>
        </>
      ) : (
        // Without a number, the name is large and centered on the body, as `artwork.rs` draws it.
        <text
          key={`large ${cover.name} ${center}`}
          ref={fitLarge}
          data-width="38"
          data-center={center}
          x="20"
          y={center + 4}
          className="cover-name-large"
          fill="#fff"
        >
          {cover.name}
        </text>
      )}
    </svg>
  );
}

/** The ADJ icon of an effect. A film cover draws with web fonts, other icons with camera pixels. */
export function EffectIcon({ effect }: { effect: Pick<Effect, 'icon' | 'cover'> }) {
  const { icon, cover } = effect;
  const draw = useCallback((canvas: HTMLCanvasElement | null) => paint(canvas, icon), [icon]);
  if (cover) {
    return <CoverIcon cover={cover} />;
  }
  return <canvas ref={draw} className="effect-icon pixels" width={SIZE} height={SIZE} aria-hidden="true" />;
}
