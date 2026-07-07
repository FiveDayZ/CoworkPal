# Code Modification Progress Record

## 1. Modification Time

2026-07-07 16:41:28 +08:00

## 2. Modified Modules / Files

- `src-tauri/src/taskbar_embed.rs`
  - Module: Windows native taskbar monitor rendering.
  - Scope: moved native Win32 taskbar window creation, positioning, hiding, and repaint scheduling onto Tauri's main thread.

## 3. New & Optimized Content

- Reused Tauri's built-in `run_on_main_thread` instead of adding a custom Win32 message loop.
- Scheduled native taskbar render work on the main UI thread so the child window receives paint and input messages normally.
- Added `UpdateWindow` after `InvalidateRect` to force the native taskbar monitor to paint immediately after each data refresh.
- Kept the existing native two-line display, maximum three metrics, and shared right-click menu behavior unchanged.

## 4. Fixed Bugs

- Problem: After switching to native taskbar drawing, no monitoring data appeared on the taskbar.
  - Recurrence condition: The native Win32 child window was created from an async runtime worker thread, which does not own the normal Tauri UI message loop.
  - Repair scheme: Schedule native Win32 window operations through `AppHandle::run_on_main_thread` and force an immediate repaint after invalidation.

## 5. Pending Tasks & Optimization Items

- Verify on the affected Windows 10 machine that the taskbar data is visible and that right-click still opens the shared menu.
- If the taskbar still hides the child window on specific Windows 10 builds, the next fallback should be reverting to a minimal WebView title/text path rather than adding another drawing subsystem.

## 6. Supplementary Remarks

- Verification completed:
  - `cargo check` from `src-tauri`
  - `node_modules\.bin\tsc.CMD --noEmit`
  - `node_modules\.bin\vite.CMD build`
- `cargo fmt` remains unavailable because `rustfmt` is not installed for the active `stable-x86_64-pc-windows-msvc` toolchain.
- Development optimization principle followed: reused Tauri's existing main-thread dispatcher and native Windows repaint API instead of introducing a separate message pump or new dependency.
