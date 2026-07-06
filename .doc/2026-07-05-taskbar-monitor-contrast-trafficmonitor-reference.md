# Code Modification Progress Record

## 1. Modification Time

2026-07-05 14:27:27 +08:00

## 2. Modified Modules / Files

- `src/styles/core-ui.css`
  - Module: taskbar monitor text rendering.
  - Scope: improved text contrast for light and dark Windows taskbar themes.
- `src-tauri/src/taskbar_embed.rs`
  - Module: Windows taskbar embedded monitor window.
  - Scope: adjusted embedded taskbar monitor width and height so two-line text is not clipped.

## 3. New & Optimized Content

- Referenced TrafficMonitor's taskbar display approach: native taskbar window, paint event rendering, text drawing, transparent/background handling, and D2D/GDI fallback.
- Kept the current implementation on the existing Tauri WebView path for this iteration to avoid a large native rewrite.
- Changed taskbar monitor text to dark text by default for light taskbars.
- Added `prefers-color-scheme: dark` handling to switch the taskbar monitor text to light text on dark themes.
- Added subtle opposite-color text shadow to improve readability on mixed or translucent taskbar backgrounds.
- Increased taskbar monitor font weight and line height.
- Increased the embedded taskbar child window height from `22-24px` to `28-32px` to prevent the second line from being clipped.

## 4. Fixed Bugs

- Problem: The taskbar monitor became nearly unreadable on a light Windows taskbar because the text was fixed to white.
  - Recurrence condition: Enable taskbar monitor while Windows taskbar uses a light or translucent light background.
  - Repair scheme: Use dark text by default, switch to light text only in dark color scheme, and add contrast-preserving text shadow.
- Problem: Two-line taskbar monitor text could appear cramped or partially clipped.
  - Recurrence condition: The taskbar monitor child window height was capped to `22-24px`.
  - Repair scheme: Increase the embedded window height cap to `28-32px`.

## 5. Pending Tasks & Optimization Items

- Full TrafficMonitor-style rendering would require replacing the WebView taskbar monitor with a native Win32 child window and direct text painting.
- If WebView transparency remains unstable on Windows 10/11 taskbars, implement the native route in a separate iteration using the existing `windows` crate instead of continuing CSS patches.
- Optional future improvement: detect taskbar background brightness directly through Win32 and choose text color from the actual taskbar background, not only system color scheme.

## 6. Supplementary Remarks

- Reference reviewed:
  - `https://github.com/zhongyang219/TrafficMonitor`
  - `TrafficMonitor/TaskBarDlg.cpp`
- Verification completed:
  - `node_modules\.bin\tsc.CMD --noEmit`
  - `cargo check` from `src-tauri`
  - `node_modules\.bin\vite.CMD build`
- Development optimization principle followed: reused the existing taskbar WebView and Win32 embedding path for the immediate fix; deferred the larger native rewrite until it is actually needed.
