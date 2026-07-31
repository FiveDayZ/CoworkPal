//! Central registry of Tauri event names.
//!
//! Event names are stringly-typed on both the Rust and TypeScript sides; a typo
//! in either direction silently breaks the frontend wiring. Keeping the names
//! as `const`s here (and mirroring them in `src/services/events/eventNames.ts`)
//! gives a single source of truth to grep against and reduces that risk.

/// Emitted by the hardware snapshot pump with the latest `HardwareMetricsSnapshot`.
pub const HARDWARE_METRICS: &str = "hardware:metrics";

/// Emitted when the work-log report is recomputed (snapshot tick or manual edit).
pub const WORKLOG_UPDATED: &str = "worklog:updated";

/// Emitted when the workshop state changes (tick, upgrade, pause toggle).
pub const WORKSHOP_UPDATED: &str = "workshop:updated";

/// Emitted when app settings are patched (settings command or tray handler).
pub const SETTINGS_UPDATED: &str = "settings:updated";

/// Emitted by the CoCat interaction command to drive the animation one-shot.
pub const COCAT_INTERACTION_STATE: &str = "cocat:interaction-state";

/// Emitted when the pet's resolved state transitions (Idle/Repair/...).
pub const PET_STATE_CHANGED: &str = "pet:state-changed";

/// Emitted when a new achievement unlocks.
pub const ACHIEVEMENT_UNLOCKED: &str = "achievement:unlocked";

/// Emitted to ask the main window to navigate to a given route.
pub const UI_NAVIGATE_MAIN: &str = "ui:navigate-main";

/// Emitted by the updater with incremental download progress.
pub const UPDATE_PROGRESS: &str = "update:progress";

/// Emitted when a focus session starts, is completed, or is abandoned.
/// Payload is the updated `FocusSessionBook` (so the frontend always has the
/// full list + the live active session in one shape).
pub const FOCUS_SESSION_UPDATED: &str = "focus:session-updated";

/// Emitted after any notes/memos mutation (create/update/toggle/delete).
/// Payload is the full updated `NoteBook` so the frontend list stays in sync.
pub const NOTES_UPDATED: &str = "notes:updated";

/// Emitted when a memory release (manual or auto) completes. Payload is the
/// `ReleaseResult` with released bytes and a human-facing note. The frontend
/// uses this to show a toast notification when `enableNotifications` is on.
pub const MEMORY_RELEASE_COMPLETED: &str = "memory:release-completed";
