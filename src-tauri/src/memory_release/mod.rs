//! Memory release module — manual + automatic reclamation of system RAM.
//!
//! Architecture (see `.doc/2026-07-06-memory-release-module.md` for the full
//! design):
//!
//! - **Light tier** (no elevation, used by auto-trigger and as UAC-decline
//!   fallback): drop low-priority standby list + trim CoreCat's own working
//!   set. Runs in-process in `windows_api`.
//! - **Full tier** (elevation required, manual only): the full memreduct-style
//!   sweep — system working set, all standby/modified lists, every process's
//!   working set. Runs inside `coreworkpal-memory-helper.exe`, which the main
//!   process spawns via `ShellExecuteW("runas")`. A UAC prompt appears; if the
//!   user declines, the spawn returns a cancellation and we degrade to the
//!   light tier.
//!
//! Auto-trigger watches system used memory against the configured GB threshold
//! with a 60-second cooldown to avoid thrash.

pub mod sysinfo_query;
pub mod windows_api;

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sysinfo_query::MemoryPressure;
use tokio::sync::Mutex;

use crate::events::MEMORY_RELEASE_COMPLETED;

/// Minimum gap between auto-triggered cleanups. Prevents a burst of triggers
/// when memory hovers right around the threshold.
const AUTO_RELEASE_COOLDOWN: Duration = Duration::from_secs(60);

/// Maximum wall-clock time we wait for the elevated helper to finish. The
/// helper is fast (a few seconds for the full sweep) but a stuck helper should
/// not block the tray menu action forever.
const HELPER_TIMEOUT: Duration = Duration::from_secs(30);

/// What the user asked for, distinguishing the tray path (full, with UAC) from
/// the settings-page "release now" button (also full) and the auto path
/// (light, no UAC).
#[derive(Debug, Clone, Copy)]
pub enum ReleaseKind {
    /// User-initiated: attempt the full tier via an elevated helper; degrade
    /// to the light tier if UAC is declined.
    ManualFull,
    /// Auto-triggered by threshold: light tier only, never elevated.
    AutoLight,
}

/// Result reported back to the frontend via the `memory:release-completed`
/// event and returned from the `trigger_memory_release` command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseResult {
    /// Bytes of available physical memory gained (may be 0 on failure).
    pub released_bytes: u64,
    /// True when the full tier ran elevated. False when we degraded to the
    /// light tier (UAC declined, helper missing, or auto-trigger).
    pub full_tier: bool,
    /// Human-facing note for the notification body.
    pub note: String,
}

impl ReleaseResult {
    pub(crate) fn light(released_bytes: u64) -> Self {
        let note = if released_bytes > 0 {
            format!("已执行轻量清理，释放 {:.1} GB", released_bytes as f64 / bytes_per_gib())
        } else {
            "已执行轻量清理，当前无需释放".to_string()
        };
        Self {
            released_bytes,
            full_tier: false,
            note,
        }
    }

    pub(crate) fn full(released_bytes: u64) -> Self {
        let note = format!(
            "已执行全量清理，释放 {:.1} GB",
            released_bytes as f64 / bytes_per_gib()
        );
        Self {
            released_bytes,
            full_tier: true,
            note,
        }
    }
}

/// Shared state for the auto-release watcher. A single instance lives in
/// `AppState` and is locked briefly on each tick.
#[derive(Debug, Default)]
pub struct MemoryReleaseState {
    last_auto_release: Mutex<Option<Instant>>,
}

impl MemoryReleaseState {
    pub async fn try_acquire(&self) -> Option<AutoReleaseGuard> {
        let mut guard = self.last_auto_release.lock().await;
        let now = Instant::now();
        if let Some(last) = *guard {
            if now.duration_since(last) < AUTO_RELEASE_COOLDOWN {
                return None;
            }
        }
        *guard = Some(now);
        Some(AutoReleaseGuard { _priv: () })
    }
}

/// Returned by `try_acquire` to prove the cooldown was satisfied. Dropping it
/// does nothing — the timestamp is already recorded.
pub struct AutoReleaseGuard {
    _priv: (),
}

fn bytes_per_gib() -> f64 {
    (1u64 << 30) as f64
}

/// Run the light tier (in-process): low-priority standby purge + own working
/// set trim. Returns the released bytes delta measured around the call.
pub fn run_light_clean() -> u64 {
    let before = sysinfo_query::sample_pressure();
    // Each step is independent; failures are silently ignored (partial gain is
    // still a gain). catch_unwind guards the unsafe FFI boundary.
    let _ = std::panic::catch_unwind(|| {
        windows_api::purge_low_priority_standby();
    });
    let _ = std::panic::catch_unwind(|| {
        windows_api::empty_current_process_working_set();
    });
    let after = sysinfo_query::sample_pressure();
    match (before, after) {
        (Some(b), Some(a)) => MemoryPressure::released_since(&b, &a).max(0) as u64,
        _ => 0,
    }
}

/// Run the full tier via an elevated self-restart.
///
/// The main executable re-launches *itself* with `ShellExecuteW("runas")` and
/// a `--memory-clean-helper --result-file=<path>` argument pair. Windows shows
/// a UAC prompt; on accept the relaunched process (detected in `main.rs`)
/// performs the full sweep and writes `{"releasedBytes": N}` to the result
/// file, then exits. We wait for that process and read the file.
///
/// Self-restart avoids shipping a second binary — `current_exe()` is always
/// valid in both dev (`target/<profile>/`) and installed runs.
///
/// Returns `ReleaseResult` for the frontend.
pub async fn run_full_clean_via_helper() -> ReleaseResult {
    // Offload the blocking ShellExecuteW + wait to a spawn_blocking task.
    let outcome = tokio::task::spawn_blocking(run_full_via_self_restart_blocking)
        .await
        .unwrap_or_else(|join_error| {
            tracing::error!("memory self-restart task panicked: {join_error}");
            Err(HelperError::TaskPanic)
        });

    match outcome {
        Ok(released_bytes) => ReleaseResult::full(released_bytes),
        Err(HelperError::UserDeclined) => {
            tracing::info!("UAC declined for memory release — degrading to light tier");
            let released = run_light_clean();
            let mut result = ReleaseResult::light(released);
            result.note = format!("{}（未提权）", result.note);
            result
        }
        Err(error) => {
            tracing::warn!("memory self-restart failed ({:?}) — degrading to light tier", error);
            let released = run_light_clean();
            let mut result = ReleaseResult::light(released);
            result.note = format!("{}（清理助手异常，已降级）", result.note);
            result
        }
    }
}

#[derive(Debug)]
enum HelperError {
    /// UAC prompt was declined (user clicked No) or canceled.
    UserDeclined,
    /// Relaunched process exited without writing a usable result.
    NoResult,
    /// Relaunched process did not finish within `HELPER_TIMEOUT`.
    Timeout,
    /// Spawn-blocking task panicked.
    TaskPanic,
}

#[cfg(windows)]
fn run_full_via_self_restart_blocking() -> Result<u64, HelperError> {
    use windows::{
        core::{w, HSTRING, PCWSTR},
        Win32::{
            Foundation::{CloseHandle, GetLastError, WAIT_OBJECT_0},
            System::Threading::{TerminateProcess, WaitForSingleObject},
            UI::{
                Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW},
                WindowsAndMessaging::SW_HIDE,
            },
        },
    };

    let exe_path = std::env::current_exe().map_err(|_| HelperError::NoResult)?;

    // Prepare the result file path the relaunched process will write to.
    let temp_dir = std::env::temp_dir();
    let result_path = temp_dir.join(format!(
        "coreworkpal-memory-release-{}.json",
        std::process::id()
    ));
    let result_path_str = result_path.to_str().ok_or(HelperError::NoResult)?.to_string();

    // `--memory-clean-helper` makes main.rs run the sweep & exit early;
    // `--result-file=<path>` tells it where to write the result JSON.
    let parameters = HSTRING::from(format!(
        "--memory-clean-helper --result-file=\"{}\"",
        result_path_str
    ));
    let file = HSTRING::from(exe_path.as_os_str());

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(parameters.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };

    // SAFETY: ShellExecuteExW re-launches ourselves elevated. We pass a properly
    // sized SHELLEXECUTEINFOW; `file`/`parameters` HSTRINGs outlive the call.
    let launched = unsafe { ShellExecuteExW(&mut info) }.is_ok();

    if !launched {
        // Distinguish UAC-decline (ERROR_CANCELLED = 1223) from genuine failures.
        let last = unsafe { GetLastError() };
        if last.0 == 1223 {
            return Err(HelperError::UserDeclined);
        }
        tracing::warn!("ShellExecuteExW failed with error {}", last.0);
        return Err(HelperError::NoResult);
    }

    let handle = info.hProcess;
    let result = if handle.is_invalid() {
        Err(HelperError::UserDeclined)
    } else {
        // SAFETY: WaitForSingleObject on the returned process handle; it is a
        // real handle that must be closed afterwards. Timeout in ms as u32.
        let wait_ms = HELPER_TIMEOUT.as_millis().min(u32::MAX as u128) as u32;
        let wait = unsafe { WaitForSingleObject(handle, wait_ms) };

        // On timeout the elevated helper is still running. Before closing the
        // handle, terminate the process so it cannot linger as an orphaned
        // elevated process consuming resources (S8). TerminateProcess is safe
        // here: `handle` is a real process handle we own and have not yet freed.
        if wait.0 == 258 {
            // WAIT_TIMEOUT
            let killed = unsafe { TerminateProcess(handle, 1) }.is_ok();
            if killed {
                // Reap the terminated process so it does not become a zombie.
                let _ = unsafe { WaitForSingleObject(handle, 0) };
            }
            tracing::warn!("memory-release helper exceeded timeout and was terminated");
        }

        let _ = unsafe { CloseHandle(handle) };
        if wait == WAIT_OBJECT_0 {
            read_helper_result(&result_path)
        } else if wait.0 == 258 {
            // WAIT_TIMEOUT
            Err(HelperError::Timeout)
        } else {
            Err(HelperError::NoResult)
        }
    };

    // Best-effort cleanup of the result file.
    let _ = std::fs::remove_file(&result_path);
    result
}

#[cfg(not(windows))]
fn run_full_via_self_restart_blocking() -> Result<u64, HelperError> {
    Err(HelperError::NoResult)
}

/// Parse the relaunched process's result JSON: `{"releasedBytes": N}`.
fn read_helper_result(path: &std::path::Path) -> Result<u64, HelperError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct HelperOutput {
        #[serde(default)]
        released_bytes: u64,
    }
    let bytes = std::fs::read(path).map_err(|_| HelperError::NoResult)?;
    if bytes.is_empty() {
        return Err(HelperError::NoResult);
    }
    let parsed: HelperOutput = serde_json::from_slice(&bytes).map_err(|error| {
        tracing::warn!("failed to parse helper result: {error}");
        HelperError::NoResult
    })?;
    Ok(parsed.released_bytes)
}

/// Emit the `MEMORY_RELEASE_COMPLETED` event so the frontend can show a toast.
pub fn emit_result(app: &tauri::AppHandle, result: &ReleaseResult) {
    use tauri::Emitter;
    if let Err(error) = app.emit(MEMORY_RELEASE_COMPLETED, result.clone()) {
        tracing::warn!("failed to emit {MEMORY_RELEASE_COMPLETED}: {error}");
    }
}

// ---------------------------------------------------------------------------
// Elevated self-restart entry point
// ---------------------------------------------------------------------------
// When the main executable is relaunched with `--memory-clean-helper`, main.rs
// calls into `run_full_clean_inplace()` before initializing Tauri. This runs
// the full-tier sweep in the already-elevated process and returns the bytes
// freed; main.rs writes that to the result file and exits.

/// Run the full-tier cleanup in the current (already-elevated) process.
/// Enables the needed privileges, samples memory, runs the sweep, and returns
/// the available-memory delta in bytes. Used by the `--memory-clean-helper`
/// self-restart path in `main.rs`.
pub fn run_full_clean_inplace() -> u64 {
    #[cfg(windows)]
    {
        enable_elevated_privileges();
        let before = sysinfo_query::sample_pressure();
        let outcome = windows_api::full_tier::run_full();
        tracing::trace!(?outcome, "full-tier cleanup complete");
        let after = sysinfo_query::sample_pressure();
        match (before, after) {
            (Some(b), Some(a)) => MemoryPressure::released_since(&b, &a).max(0) as u64,
            _ => 0,
        }
    }
    #[cfg(not(windows))]
    {
        0
    }
}

/// Enable the privileges the full-tier sweep needs. Best-effort — failures are
/// non-fatal because not every operation requires every privilege.
#[cfg(windows)]
fn enable_elevated_privileges() {
    use windows::Win32::{
        Foundation::{CloseHandle, LUID},
        Security::{
            AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES,
            SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    for privilege in [
        windows::core::w!("SeIncreaseQuotaPrivilege"),
        windows::core::w!("SeProfileSingleProcessPrivilege"),
    ] {
        // SAFETY: token handle operations on the current process; each call's
        // result is checked and failures just skip that privilege.
        unsafe {
            let mut token = Default::default();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES, &mut token).is_err() {
                continue;
            }
            let mut luid = LUID::default();
            if LookupPrivilegeValueW(None, privilege, &mut luid).is_err() {
                let _ = CloseHandle(token);
                continue;
            }
            let mut tp = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES {
                    Luid: luid,
                    Attributes: SE_PRIVILEGE_ENABLED,
                }],
            };
            let _ = AdjustTokenPrivileges(token, false, Some(&mut tp as *mut TOKEN_PRIVILEGES), 0, None, None);
            let _ = CloseHandle(token);
        }
    }
}
