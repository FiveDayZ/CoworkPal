/**
 * Sprite-sheet frame animation types.
 *
 * Each animation state has a companion JSON file that describes the sprite-sheet
 * layout (frame positions, sizes, and timing). These types mirror that JSON
 * structure and provide the contract for the frame-resolution engine.
 */

/** A single frame inside a sprite-sheet PNG. */
export interface SpriteFrame {
  /** Frame index (0-based). */
  i: number;
  /** X offset in sheet pixels. */
  x: number;
  /** Y offset in sheet pixels. */
  y: number;
  /** Frame width in sheet pixels. */
  w: number;
  /** Frame height in sheet pixels. */
  h: number;
  /** Timestamp in seconds from the start of the animation. */
  t: number;
}

/** Parsed JSON metadata that accompanies a sprite-sheet PNG. */
export interface SpriteSheetMeta {
  version: string;
  frame_size: { w: number; h: number };
  sheet_size: { w: number; h: number };
  frames: SpriteFrame[];
}

/** Pre-computed CSS style values for rendering one sprite frame. */
export interface SpriteFrameStyle {
  backgroundImage: string;
  backgroundPosition: string;
  backgroundSize: string;
  width: number;
  height: number;
}
