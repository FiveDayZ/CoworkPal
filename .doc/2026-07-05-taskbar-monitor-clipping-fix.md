# Code Modification Progress Record

## 1. Modification Time

2026-07-05 14:37:55 +08:00

## 2. Modified Modules / Files

- `src-tauri/src/taskbar_embed.rs`
  - Module: Windows taskbar embedded monitor window.
  - Scope: changed the embedded taskbar monitor height calculation.
- `src/styles/core-ui.css`
  - Module: taskbar monitor text style.
  - Scope: adjusted taskbar monitor text line height.

## 3. New & Optimized Content

- Reused the existing WebView taskbar monitor and Win32 embedding path.
- Changed the taskbar monitor child window to use the full taskbar client height instead of a fixed `28-32px` cap.
- Set the taskbar monitor child window `y` coordinate to `0`, letting the CSS content alignment handle vertical centering.
- Increased taskbar monitor text line height from `1.08` to `1.16` so two-line metric text has enough vertical room.

## 4. Fixed Bugs

- Problem: Taskbar monitor text was clipped at the top and bottom.
  - Recurrence condition: Two-line metric text rendered inside the taskbar monitor while the child window height was capped below the actual taskbar height.
  - Repair scheme: Use the full taskbar client height and loosen text line height.

## 5. Pending Tasks & Optimization Items

- Verify visually on the affected Windows taskbar DPI/theme combination.
- If WebView taskbar rendering still clips text on some Windows builds, replace the taskbar monitor with native Win32 text drawing in a separate iteration.

## 6. Supplementary Remarks

- Verification completed:
  - `cargo check` from `src-tauri`
  - `node_modules\.bin\tsc.CMD --noEmit`
  - `node_modules\.bin\vite.CMD build`
- Development optimization principle followed: fixed the sizing bug in the shared embed calculation instead of adding per-metric CSS workarounds.
