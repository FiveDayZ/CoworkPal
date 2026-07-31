/**
 * Sprite-sheet frame resolution engine.
 *
 * Given a `SpriteSheetMeta` and an elapsed time, this module calculates which
 * frame of the sprite-sheet to display and returns ready-to-use CSS style
 * properties (`backgroundPosition`, `backgroundSize`) for rendering.
 */

import type { SpriteFrameStyle, SpriteSheetMeta } from "./spriteSheetTypes";

/** Display size at which the CoCat sprite is rendered (logical px). */
const DISPLAY_SIZE = 160;

/**
 * Resolve the frame index for a given elapsed time.
 *
 * @param meta     Parsed sprite-sheet JSON metadata.
 * @param timeMs   Elapsed time in milliseconds since the animation started.
 * @param loop     Whether the animation should loop. When false, the last frame
 *                 is held once the elapsed time exceeds the total duration.
 * @returns        The 0-based frame index.
 */
export function resolveSpriteFrameIndex(
  meta: SpriteSheetMeta,
  timeMs: number,
  loop: boolean,
): number {
  const { frames } = meta;
  if (frames.length === 0) {
    return 0;
  }

  const totalDurationSec = frames[frames.length - 1].t;
  if (totalDurationSec <= 0) {
    return 0;
  }

  const timeSec = timeMs / 1000;

  // For looping animations, wrap around; for one-shot, clamp to last frame.
  const effectiveTimeSec = loop
    ? timeSec % totalDurationSec
    : Math.min(timeSec, totalDurationSec);

  // Binary-search for the frame whose timestamp is <= effectiveTimeSec.
  let lo = 0;
  let hi = frames.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >>> 1;
    if (frames[mid].t <= effectiveTimeSec) {
      lo = mid;
    } else {
      hi = mid - 1;
    }
  }

  return lo;
}

/**
 * Compute the CSS style properties to render a specific frame of the sprite-sheet.
 *
 * @param meta       Parsed sprite-sheet JSON metadata.
 * @param frameIndex 0-based index of the frame to render.
 * @param sheetUrl   Resolved URL of the sprite-sheet PNG (from Vite import).
 * @returns          A `SpriteFrameStyle` object ready to spread onto a div's style.
 */
export function getSpriteFrameStyle(
  meta: SpriteSheetMeta,
  frameIndex: number,
  sheetUrl: string,
): SpriteFrameStyle {
  const frame = meta.frames[frameIndex] ?? meta.frames[0];
  const frameW = meta.frame_size.w;
  const frameH = meta.frame_size.h;
  const sheetW = meta.sheet_size.w;
  const sheetH = meta.sheet_size.h;

  // Scale from sheet-pixel space to display-pixel space.
  const scaleX = DISPLAY_SIZE / frameW;
  const scaleY = DISPLAY_SIZE / frameH;

  return {
    backgroundImage: `url(${sheetUrl})`,
    backgroundPosition: `${-(frame.x * scaleX)}px ${-(frame.y * scaleY)}px`,
    backgroundSize: `${sheetW * scaleX}px ${sheetH * scaleY}px`,
    width: DISPLAY_SIZE,
    height: DISPLAY_SIZE,
  };
}

/**
 * Get the total duration of a sprite-sheet animation in milliseconds.
 */
export function getSpriteSheetDurationMs(meta: SpriteSheetMeta): number {
  if (meta.frames.length === 0) {
    return 0;
  }

  return meta.frames[meta.frames.length - 1].t * 1000;
}

/**
 * Sample a frame index with low-power throttling.
 *
 * In low-power mode the effective frame rate is reduced by snapping elapsed time
 * to a coarser grid (≈4 fps instead of the native ≈12 fps).
 */
export function resolveSpriteFrameIndexLowPower(
  meta: SpriteSheetMeta,
  timeMs: number,
  loop: boolean,
  lowPowerIntervalMs = 250,
): number {
  const snappedMs =
    Math.floor(timeMs / lowPowerIntervalMs) * lowPowerIntervalMs;
  return resolveSpriteFrameIndex(meta, snappedMs, loop);
}
