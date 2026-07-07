# Code Modification Progress Record

## 1. Modification Time

2026-07-07 16:12:34 +08:00

## 2. Modified Modules / Files

- `src-tauri/src/taskbar_embed.rs`
  - Module: Windows taskbar embedded monitor window.
  - Scope: added Win32 layered-window color-key transparency for the taskbar monitor child window.
- `src/styles/core-ui.css`
  - Module: taskbar monitor background rendering.
  - Scope: changed the taskbar monitor background from CSS transparent to the fixed color-key background.

## 3. New & Optimized Content

- Reused the existing Tauri WebView taskbar monitor and Win32 embedding path.
- Added `WS_EX_LAYERED` to the embedded taskbar monitor child window.
- Added `SetLayeredWindowAttributes(..., LWA_COLORKEY)` with `rgb(1, 1, 1)` as the transparent key.
- Changed the taskbar monitor page/root background to `rgb(1, 1, 1)` so Win32 can remove the background consistently on Windows 10.
- Kept the current two-line metric display and maximum three-metric taskbar display behavior unchanged.

## 4. Fixed Bugs

- Problem: On Windows 10, the taskbar monitor showed a visible light rectangle/background, while Windows 11 displayed normally.
  - Recurrence condition: The taskbar monitor WebView is re-parented into the Windows 10 taskbar, where normal WebView transparency can fail.
  - Repair scheme: Use a TrafficMonitor-style color-key transparency path: paint a fixed key color in CSS and remove it with Win32 layered-window color-key transparency.

## 5. Pending Tasks & Optimization Items

- Verify visually on the affected Windows 10 taskbar.
- If WebView transparency still behaves inconsistently on some Windows 10 builds, replace the taskbar monitor with native Win32 text drawing in a separate iteration.

## 6. Supplementary Remarks

- Verification completed:
  - `cargo check` from `src-tauri`
  - `node_modules\.bin\tsc.CMD --noEmit`
  - `node_modules\.bin\vite.CMD build`
- Development optimization principle followed: reused the existing taskbar monitor pipeline and added the smallest native transparency fix instead of replacing the rendering subsystem.
