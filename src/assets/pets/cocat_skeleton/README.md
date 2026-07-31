# CoCat Skeleton Placeholder

This directory reserves the final layered CoCat asset contract.

Phase 0 - Phase 1 uses inline React/SVG placeholder layers in `src/pet/cocat`.
Final art can replace those placeholders by adding transparent, same-canvas PNG
layers that match `manifest.json`.

Required final layers:

- `shadow.png`
- `tail/tail_base.png`
- `body/body_base.png`
- `body/arm_left.png`
- `body/arm_right_wrench.png`
- `body/pouch.png`
- `head/head_base.png`
- `head/ear_left.png`
- `head/ear_right.png`
- `head/goggles.png`
- `eyes/eye_normal.png`
- `vfx/vfx_anchor_placeholder.svg`

All final PNG layers must share the same transparent `160x160` logical canvas
or `320x320` @2x canvas so pivots remain stable.
