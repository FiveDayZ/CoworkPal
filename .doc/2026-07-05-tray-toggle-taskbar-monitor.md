# Code Modification Progress Record

## 1. Modification Time

2026-07-05 14:17:59 +08:00

## 2. Modified Modules / Files

- `src-tauri/src/tray/mod.rs`
  - Module: Windows tray icon right-click menu.
  - Scope: added a tray menu command for toggling the taskbar monitor display.

## 3. New & Optimized Content

- Added a new tray menu item: `显示/隐藏任务栏`.
- Reused the existing settings patch flow and `show_monitor_data_in_taskbar` setting.
- Reused the existing `taskbar_embed::sync_taskbar_monitor` implementation to immediately show or hide the embedded taskbar monitor after the setting changes.
- Kept the change in the Rust tray module only; no new npm, GitHub, or Rust dependency was introduced.

## 4. Fixed Bugs

- Problem: The tray right-click menu only exposed `显示/隐藏监控条`, but did not provide a quick entry for the taskbar monitor.
  - Recurrence condition: User right-clicks the CoreWorkPal tray icon and wants to show or hide the taskbar monitor without opening Settings.
  - Repair scheme: Add a dedicated tray menu command that toggles `show_monitor_data_in_taskbar` and synchronizes the taskbar monitor window immediately.

## 5. Pending Tasks & Optimization Items

- Verify the menu label wording on the packaged app. If ambiguity remains between the Windows taskbar and the taskbar monitor text, rename it to `显示/隐藏任务栏监控`.
- If users later need state-aware labels, update the tray menu item text dynamically to `显示任务栏监控` / `隐藏任务栏监控`; this was skipped because the existing tray menu uses fixed toggle labels.

## 6. Supplementary Remarks

- Verification completed:
  - `cargo check` from `src-tauri`
- `cargo fmt` was attempted but the current Rust toolchain does not have `rustfmt` installed.
- Development optimization principle followed: reused existing tray menu, settings persistence, event emission, and taskbar synchronization paths instead of adding a new command framework.
