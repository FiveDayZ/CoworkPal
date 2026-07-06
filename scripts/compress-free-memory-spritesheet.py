#!/usr/bin/env python3
"""
Compress Free_Memory sprite-sheet from 720x720 frames (5760x5760 sheet) down
to the project standard 160x160 frames (1280x1280 sheet), matching every other
CoreCat animation (Idle, Celebrate, etc.).

Rewrites both:
  - Free_Memory.webp  (resized 5760x5760 -> 1280x1280)
  - Free_Memory.json  (frame_size, sheet_size, and every frame x/y/w/h scaled)

Idempotent: detects if already at 160x160 and exits without changes.
"""
import json
import sys
from pathlib import Path

from PIL import Image

ASSET_DIR = Path(__file__).resolve().parent.parent / "src" / "assets" / "pets" / "animation"
WEBP = ASSET_DIR / "Free_Memory.webp"
JSON = ASSET_DIR / "Free_Memory.json"

TARGET_FRAME = 160          # standard frame size used by all other animations
GRID = 8                    # 8x8 frames
TARGET_SHEET = TARGET_FRAME * GRID  # 1280


def main() -> int:
    if not WEBP.exists() or not JSON.exists():
        print(f"ERROR: missing asset(s): {WEBP} / {JSON}", file=sys.stderr)
        return 1

    with JSON.open("r", encoding="utf-8") as f:
        meta = json.load(f)

    cur_frame_w = meta["frame_size"]["w"]
    cur_frame_h = meta["frame_size"]["h"]
    cur_sheet_w = meta["sheet_size"]["w"]
    cur_sheet_h = meta["sheet_size"]["h"]

    if cur_frame_w == TARGET_FRAME and cur_frame_h == TARGET_FRAME:
        print(f"Free_Memory already at {TARGET_FRAME}x{TARGET_FRAME} — nothing to do.")
        return 0

    scale = TARGET_FRAME / cur_frame_w
    print(f"Scaling by {scale:.4f}: {cur_frame_w}x{cur_frame_h} frames "
          f"({cur_sheet_w}x{cur_sheet_h} sheet) -> {TARGET_FRAME}x{TARGET_FRAME} "
          f"({TARGET_SHEET}x{TARGET_SHEET} sheet)")

    # Resize the sprite-sheet image with a high-quality filter.
    img = Image.open(WEBP)
    print(f"Loaded {WEBP.name}: {img.size[0]}x{img.size[1]} mode={img.mode}")
    resized = img.resize((TARGET_SHEET, TARGET_SHEET), Image.LANCZOS)
    # WebP lossless keeps pixel art crisp; fall back to quality=90 lossy if the
    # file would be larger than necessary.
    resized.save(WEBP, format="WEBP", quality=90, lossless=False, method=6)
    new_size_kb = WEBP.stat().st_size / 1024
    print(f"Wrote {WEBP.name}: {TARGET_SHEET}x{TARGET_SHEET} ({new_size_kb:.0f} KB)")

    # Rewrite the JSON metadata: scale frame_size, sheet_size, and every
    # frame's x/y/w/h.
    meta["frame_size"] = {"w": TARGET_FRAME, "h": TARGET_FRAME}
    meta["sheet_size"] = {"w": TARGET_SHEET, "h": TARGET_SHEET}
    for frame in meta["frames"]:
        frame["x"] = round(frame["x"] * scale)
        frame["y"] = round(frame["y"] * scale)
        frame["w"] = TARGET_FRAME
        frame["h"] = TARGET_FRAME

    with JSON.open("w", encoding="utf-8") as f:
        json.dump(meta, f, indent=2)
        f.write("\n")
    print(f"Wrote {JSON.name}: {len(meta['frames'])} frames rescaled")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
