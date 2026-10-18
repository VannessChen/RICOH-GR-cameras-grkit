export interface RawCrop {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface RawImage {
  width: number;
  height: number;
  data: Uint16Array;
}

/** Read only the original GR DNG's TIFF directories and default sensor crop. */
export function readRawCrop(bytes: Uint8Array): RawCrop {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const fail = (): never => {
    throw new Error('The DNG has invalid default crop metadata.');
  };
  const check = (offset: number, size: number) => {
    if (!Number.isSafeInteger(offset) || offset < 0 || offset + size > view.byteLength) fail();
  };
  check(0, 8);
  const marker = view.getUint16(0);
  if (marker !== 0x4949 && marker !== 0x4d4d) fail();
  const little = marker === 0x4949;
  const u16 = (offset: number) => {
    check(offset, 2);
    return view.getUint16(offset, little);
  };
  const u32 = (offset: number) => {
    check(offset, 4);
    return view.getUint32(offset, little);
  };
  if (u16(2) !== 42) fail();

  const values = (entry: number, expectedCount?: number): number[] => {
    const type = u16(entry + 2);
    const count = u32(entry + 4);
    const elementSize = type === 3 ? 2 : type === 4 ? 4 : type === 5 ? 8 : 0;
    if (
      !elementSize ||
      count < 1 ||
      count > 16 ||
      (expectedCount !== undefined && count !== expectedCount)
    )
      fail();
    const size = count * elementSize;
    const start = size <= 4 ? entry + 8 : u32(entry + 8);
    check(start, size);
    return Array.from({ length: count }, (_, index) => {
      const offset = start + index * elementSize;
      if (type === 3) return u16(offset);
      if (type === 4) return u32(offset);
      const denominator = u32(offset + 4);
      if (denominator === 0) fail();
      return u32(offset) / denominator;
    });
  };

  const pending = [u32(4)];
  const visited = new Set<number>();
  let crop: RawCrop | undefined;
  while (pending.length) {
    const offset = pending.pop()!;
    if (offset < 8 || visited.has(offset) || visited.size >= 16) fail();
    visited.add(offset);
    const count = u16(offset);
    if (count > 4096) fail();
    check(offset + 2, count * 12 + 4);
    let cfa = false;
    let origin: number[] | undefined;
    let size: number[] | undefined;
    const seenTags = new Set<number>();
    for (let index = 0; index < count; index++) {
      const entry = offset + 2 + index * 12;
      const tag = u16(entry);
      if (![262, 330, 50719, 50720].includes(tag)) continue;
      if (seenTags.has(tag)) fail();
      seenTags.add(tag);
      if (tag === 262) cfa = values(entry, 1)[0] === 32803;
      else if (tag === 330) {
        if (u16(entry + 2) !== 4) fail();
        pending.push(...values(entry));
      } else if (tag === 50719) origin = values(entry, 2);
      else size = values(entry, 2);
    }
    const next = u32(offset + 2 + count * 12);
    if (next) pending.push(next);
    if (cfa) {
      if (!origin || !size || crop) return fail();
      crop = { x: origin[0]!, y: origin[1]!, width: size[0]!, height: size[1]! };
    }
  }
  if (!crop || crop.width <= 0 || crop.height <= 0) return fail();
  return crop;
}

/** Crop the already oriented half-size LibRaw image to the DNG's photo area. */
export function cropRawImage(
  image: RawImage,
  crop: RawCrop,
  activeWidth: number,
  activeHeight: number,
  flip: number,
): RawImage {
  if (
    !Number.isInteger(flip) ||
    flip < 0 ||
    flip > 7 ||
    ![crop.x, crop.y, crop.width, crop.height].every(Number.isFinite) ||
    crop.x < 0 ||
    crop.y < 0 ||
    crop.width <= 0 ||
    crop.height <= 0 ||
    crop.x + crop.width > activeWidth ||
    crop.y + crop.height > activeHeight
  )
    throw new Error('The DNG crop is outside the sensor image.');
  const sensorWidth = Math.ceil(activeWidth / 2);
  const sensorHeight = Math.ceil(activeHeight / 2);
  let x = Math.ceil(crop.x / 2);
  let y = Math.ceil(crop.y / 2);
  let width = Math.floor((crop.x + crop.width) / 2) - x;
  let height = Math.floor((crop.y + crop.height) / 2) - y;
  // LibRaw flip bits reflect source axes before transposing them.
  if (flip & 1) x = sensorWidth - x - width;
  if (flip & 2) y = sensorHeight - y - height;
  if (flip & 4) {
    [x, y] = [y, x];
    [width, height] = [height, width];
  }
  const expectedWidth = flip & 4 ? sensorHeight : sensorWidth;
  const expectedHeight = flip & 4 ? sensorWidth : sensorHeight;
  if (
    image.width !== expectedWidth ||
    image.height !== expectedHeight ||
    width <= 0 ||
    height <= 0 ||
    image.data.length !== image.width * image.height * 3
  )
    throw new Error('The DNG decoded dimensions do not match its sensor crop.');
  const data = new Uint16Array(width * height * 3);
  for (let row = 0; row < height; row++) {
    const start = ((y + row) * image.width + x) * 3;
    data.set(image.data.subarray(start, start + width * 3), row * width * 3);
  }
  return { width, height, data };
}
