# Code Modification Progress Record

## 1. Modification Time

2026-07-07 16:30:46 +08:00

## 2. Modified Modules / Files

- `src-tauri/Cargo.toml`
  - Module: Rust dependency feature configuration.
  - Scope: enabled the `windows` crate `Win32_Graphics_Gdi` feature for native taskbar text drawing.
- `src-tauri/Cargo.lock`
  - Module: Rust lockfile.
  - Scope: synchronized the local package version metadata with `Cargo.toml` (`0.2.5`).
- `src-tauri/src/taskbar_embed.rs`
  - Module: Windows taskbar monitor embedding and rendering.
  - Scope: replaced the Windows taskbar monitor WebView rendering path with a native Win32 child window and GDI text drawing path.
- `src-tauri/src/monitoring/mod.rs`
  - Module: hardware sampling pump.
  - Scope: refreshed the native taskbar monitor immediately after each hardware metrics snapshot update.

## 3. New & Optimized Content

- Switched all Windows taskbar monitor rendering to one native Win32/GDI path, without separating Windows 10 and Windows 11 behavior.
- Kept non-Windows behavior on the existing Tauri WebView fallback path.
- Added native two-line taskbar metric cell generation in Rust, matching the existing CPU/GPU/RAM/DISK/NET data format and keeping the maximum visible metrics limit at three.
- Added a transparent native taskbar child window using `WS_CHILD`, `WS_EX_LAYERED`, `SetLayeredWindowAttributes`, and a fixed color key background.
- Added GDI text rendering with Segoe UI semibold, ClearType quality, centered two-row layout, and automatic light/dark text color selection based on sampled taskbar background brightness.
- Preserved the taskbar monitor right-click menu by handling native `WM_CONTEXTMENU` and reusing the existing shared tray menu.
- Updated the hardware snapshot pump so the native taskbar text refreshes at the same cadence as hardware metrics instead of relying only on the previous periodic sync loop.

## 4. Fixed Bugs

- Problem: On Windows 10, the taskbar monitor alternated between a visible light rectangle and a ghosted black text state.
  - Recurrence condition: The Tauri/WebView taskbar monitor window was embedded as a child of the Windows taskbar, and Windows 10 handled WebView transparency/composition inconsistently.
  - Repair scheme: Stop using WebView rendering for Windows taskbar data; render the data with a native Win32 child window and GDI text directly.
- Problem: The taskbar monitor could show stale values between hardware refreshes.
  - Recurrence condition: The taskbar embed sync loop refreshed periodically, independent of the hardware snapshot pump.
  - Repair scheme: Trigger taskbar monitor synchronization immediately after each `hardware:metrics` update.

## 5. Pending Tasks & Optimization Items

- Verify the visual result directly on the affected Windows 10 environment, especially the light/dark taskbar contrast and ClearType clarity.
- If users place the Windows taskbar on the left, right, or top edge, further positioning logic may be needed because the current slot calculation targets the common bottom horizontal taskbar layout.
- If future metric labels become longer, add native ellipsis or adaptive font sizing to keep every cell within its allocated width.

## 6. Supplementary Remarks

- Verification completed:
  - `cargo check` from `src-tauri`
  - `node_modules\.bin\tsc.CMD --noEmit`
  - `node_modules\.bin\vite.CMD build`
- `cargo fmt` was attempted but could not run because `rustfmt` is not installed for the active `stable-x86_64-pc-windows-msvc` toolchain.
- Development optimization principle followed: reused mature native Windows APIs through the existing `windows` crate instead of adding a new dependency or continuing custom WebView transparency workarounds.
