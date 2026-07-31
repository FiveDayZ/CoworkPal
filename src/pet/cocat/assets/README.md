# CoCat Layered Asset Handoff

This directory is the engineering handoff point for final CoCat PNG/SVG art.
The runtime can already animate independent DOM/SVG placeholder bones; final art
can replace those placeholders by dropping transparent layered assets into
`cocat_skeleton/` with the names listed in `coCatAssetManifest.ts`.

The first transparent SVG batch is already present under `cocat_skeleton/` so
the resource checker can validate file naming, formal asset loading, pivots, and
anchors before final painted art arrives.

## Canvas

- Logical canvas: `160 x 160`.
- High DPI source: export `320 x 320` at 2x when possible.
- Every body part must use the same transparent canvas size.
- Do not crop each part to its local bounds; cropped parts will break pivots.

## Required Parts

- `shadow.png` or `shadow.svg`
- `tail/tail_base.png` or `tail/tail_base.svg`
- `tail/tail_mid.png` or `tail/tail_mid.svg`
- `tail/tail_tip.png` or `tail/tail_tip.svg`
- `body/body_base.png` or `body/body_base.svg`
- `body/arm_left.png` or `body/arm_left.svg`
- `body/arm_right_wrench.png` or `body/arm_right_wrench.svg`
- `body/arm_right_fan.png` or `body/arm_right_fan.svg`
- `body/pouch.png` or `body/pouch.svg`
- `head/head_base.png` or `head/head_base.svg`
- `head/ears_left.png` or `head/ears_left.svg`
- `head/ears_right.png` or `head/ears_right.svg`
- `head/goggles.png` or `head/goggles.svg`
- `eyes/eye_normal.png` or `eyes/eye_normal.svg`
- `eyes/eye_blink.png` or `eyes/eye_blink.svg`
- `eyes/eye_focused.png` or `eyes/eye_focused.svg`
- `eyes/eye_dizzy.png` or `eyes/eye_dizzy.svg`
- `eyes/eye_sleepy.png` or `eyes/eye_sleepy.svg`
- `eyes/eye_glowing.png` or `eyes/eye_glowing.svg`
- `props/ram_box.png` or `props/ram_box.svg`
- `props/wrench_clone.png` or `props/wrench_clone.svg`
- `props/badge_star.png` or `props/badge_star.svg`
- `props/sleep_bubble.png` or `props/sleep_bubble.svg`

## Export Rules

1. Use transparent background only.
2. Keep every moving part as an independent image layer.
3. Do not deliver one flattened full-body image.
4. Leave a small transparent safety margin around edges.
5. Align all eye expression layers exactly.
6. Keep pivots compatible with the manifest coordinates.
7. Prefer PNG for painted parts; SVG is acceptable for vector-style final art,
   simple VFX, or geometric props.
8. Check final art at `80px`, `160px`, and `320px`.

## Replacement Flow

1. Export all parts into `cocat_skeleton/`.
2. Keep file names lowercase with underscores.
3. If a file extension changes permanently, update `coCatAssetManifest.ts`;
   the current checker accepts `.png` manifest paths with same-basename `.svg`
   files as formal assets.
4. Run `corepack pnpm test:cocat`.
5. Run `corepack pnpm build`.
6. Open the dev Pet window and inspect the CoCat Asset panel.

When an asset is missing, the runtime keeps rendering the existing independent
DOM/SVG placeholder layer for that bone.
