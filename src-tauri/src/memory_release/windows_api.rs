//! Windows FFI for the **light** cleanup tier (no elevation required).
//!
//! Two operations run here, mirroring the safe tier from memreduct:
//!
//! 1. `purge_low_priority_standby()` — `NtSetSystemInformation` with
//!    `SystemMemoryListInformation` and command `MemoryPurgeLowPriorityStandbyList`.
//!    Drops only the lowest-priority standby (file cache) pages. These pages
//!    are the system's "least likely to be needed again" cache, so reclaiming
//!    them is essentially zero-impact on perceived performance. Note: this
//!    particular command does **not** require elevation.
//! 2. `empty_current_process_working_set()` — `SetProcessWorkingSetSize(-1, -1)`
//!    on `GetCurrentProcess()`, the documented way to trim CoreCat's own
//!    working set. Pages are paged back in lazily on next access.
//!
//! Both are wrapped in `catch_unwind` by the caller (`mod.rs`) so a panic in
//! the unsafe boundary can never take down the main process (important under
//! `panic = "abort"` — the helper, not the main app, holds the riskier
//! system-wide calls).

// Only built on Windows. The sibling `mod.rs` gates all callers behind
// `#[cfg(windows)]` and provides a `noop` story for non-Windows.

/// `SystemMemoryListInformation` (0x50) — undocument system information class
/// used by memreduct / RAMMap to purge the standby/modified lists.
const SYSTEM_MEMORY_LIST_INFORMATION: u32 = 0x50;

/// Commands passed to `NtSetSystemInformation(SystemMemoryListInformation, ...)`.
/// See phnt's `SYSTEM_MEMORY_LIST_COMMAND` enum.
const MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST: u32 = 3;

#[cfg(windows)]
mod imp {
    use super::{MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST, SYSTEM_MEMORY_LIST_INFORMATION};
    use std::sync::OnceLock;
    use windows::{
        core::w,
        Win32::{
            Foundation::{HANDLE, NTSTATUS},
            System::{
                LibraryLoader::{GetModuleHandleW, GetProcAddress},
                Memory::{SetProcessWorkingSetSizeEx, QUOTA_LIMITS_HARDWS_MIN_DISABLE},
                Threading::GetCurrentProcess,
            },
        },
    };

    /// Function pointer type for `NtSetSystemInformation`. We resolve it
    /// dynamically from ntdll because the `windows` crate does not expose it
    /// (it is an undocumented NT API).
    type NtSetSystemInformationFn = unsafe extern "system" fn(
        system_information_class: u32,
        system_information: *const std::ffi::c_void,
        system_information_length: u32,
    ) -> NTSTATUS;

    /// Resolved pointer to `NtSetSystemInformation`, or `None` if the lookup
    /// failed (e.g. on an exotic Windows build). Cached for the process lifetime.
    fn nt_set_system_information() -> Option<NtSetSystemInformationFn> {
        static FN: OnceLock<Option<NtSetSystemInformationFn>> = OnceLock::new();
        *FN.get_or_init(|| {
            // SAFETY: GetModuleHandleW returns a borrowed HINSTANCE that lives
            // for the process; ntdll.dll is always loaded. GetProcAddress
            // returns the symbol address if present. Both are read-only.
            unsafe {
                let ntdll = GetModuleHandleW(w!("ntdll.dll")).ok()?;
                let addr = GetProcAddress(ntdll, windows::core::s!("NtSetSystemInformation"));
                addr.map(|raw| std::mem::transmute::<_, NtSetSystemInformationFn>(raw))
            }
        })
    }

    /// Drop low-priority standby (file cache) pages. Returns `true` if the call
    /// was made and reported success. Safe to call without elevation.
    ///
    /// On failure the caller simply skips — partial progress from the other
    /// cleanup steps still counts.
    pub(super) fn purge_low_priority_standby() -> bool {
        let Some(nt) = nt_set_system_information() else {
            return false;
        };
        let command: u32 = MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST;
        // SAFETY: We pass the correct class + a 4-byte buffer of the expected
        // enum type. NTSTATUS is checked; failures don't corrupt state.
        let status = unsafe {
            nt(
                SYSTEM_MEMORY_LIST_INFORMATION,
                &command as *const u32 as *const std::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            )
        };
        status.is_ok()
    }

    /// Trim CoreCat's own working set to the minimum. `SetProcessWorkingSetSize`
    /// with `(SIZE_T)-1, (SIZE_T)-1` is the documented "empty working set"
    /// sentinel (equivalent to `EmptyWorkingSet` from psapi).
    ///
    /// `GetCurrentProcess()` returns a pseudo-handle that never needs closing.
    pub(super) fn empty_current_process_working_set() -> bool {
        // SAFETY: GetCurrentProcess() returns a pseudo-handle (-1) that is
        // always valid for the calling process; no CloseHandle needed.
        let process: HANDLE = unsafe { GetCurrentProcess() };
        // (SIZE_T)-1 = request to empty the working set.
        let sentinel = usize::MAX;
        // SAFETY: documented Win32 API; the -1/-1 sentinel is the documented
        // "trim to minimum" signal. Returns false on failure (e.g. very low
        // memory pressure where there is nothing to trim).
        let result = unsafe {
            SetProcessWorkingSetSizeEx(process, sentinel, sentinel, QUOTA_LIMITS_HARDWS_MIN_DISABLE)
        };
        result.is_ok()
    }
}

#[cfg(not(windows))]
mod imp {
    pub(super) fn purge_low_priority_standby() -> bool {
        false
    }
    pub(super) fn empty_current_process_working_set() -> bool {
        false
    }
}

/// Drop low-priority standby (file cache) pages. No-op returning `false` on
/// non-Windows. Safe to call without elevation.
pub fn purge_low_priority_standby() -> bool {
    imp::purge_low_priority_standby()
}

/// Trim CoreCat's own working set to the minimum. No-op returning `false` on
/// non-Windows.
pub fn empty_current_process_working_set() -> bool {
    imp::empty_current_process_working_set()
}

// ============================================================================
// Full-tier cleanup (used by the elevated helper).
// ============================================================================
// These run only inside `coreworkpal-memory-helper.exe`, never in the main
// process, so a panic here aborts the helper (which writes an error result to
// the result file) without affecting CoreCat itself.

/// Flags the helper uses to perform system-wide cleanup.
#[cfg(windows)]
pub mod full_tier {
    use super::SYSTEM_MEMORY_LIST_INFORMATION;
    use std::sync::OnceLock;
    use windows::{
        core::w,
        Win32::{
            Foundation::{CloseHandle, HANDLE, NTSTATUS},
            System::{
                LibraryLoader::{GetModuleHandleW, GetProcAddress},
                Memory::{SetProcessWorkingSetSizeEx, QUOTA_LIMITS_HARDWS_MIN_DISABLE},
                Threading::{
                    GetCurrentProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
                    PROCESS_SET_QUOTA,
                },
            },
        },
    };

    /// `SystemFileCacheInformation` (0x1A) — used to trim the system file cache
    /// working set (equivalent to RAMMap's "Empty System Working Set").
    const SYSTEM_FILE_CACHE_INFORMATION: u32 = 0x1A;

    const MEMORY_PURGE_STANDBY_LIST: u32 = 2;
    const MEMORY_PURGE_MODIFIED_LIST: u32 = 4;

    type NtSetSystemInformationFn = unsafe extern "system" fn(
        system_information_class: u32,
        system_information: *const std::ffi::c_void,
        system_information_length: u32,
    ) -> NTSTATUS;

    fn nt_set_system_information() -> Option<NtSetSystemInformationFn> {
        static FN: OnceLock<Option<NtSetSystemInformationFn>> = OnceLock::new();
        *FN.get_or_init(|| unsafe {
            let ntdll = GetModuleHandleW(w!("ntdll.dll")).ok()?;
            let addr = GetProcAddress(ntdll, windows::core::s!("NtSetSystemInformation"));
            addr.map(|raw| std::mem::transmute::<_, NtSetSystemInformationFn>(raw))
        })
    }

    /// Result of the full-tier cleanup, reported per-step for diagnostics.
    #[derive(Debug, Default, Clone)]
    pub struct FullTierOutcome {
        pub purged_low_priority_standby: bool,
        pub purged_standby_list: bool,
        pub purged_modified_list: bool,
        pub trimmed_system_file_cache: bool,
        pub processes_trimmed: u32,
        pub processes_failed: u32,
    }

    /// `FILE_CACHE_INFORMATION` layout used by `SystemFileCacheInformation`.
    /// We only set `MinimumWorkingSet` / `MaximumWorkingSet` to the sentinel
    /// `(SIZE_T)-1` which requests a trim to minimum.
    #[repr(C)]
    #[derive(Default)]
    struct FileCacheInformation {
        current_size: u32,
        peak_size: u32,
        page_fault_count: u32,
        minimum_working_set: usize,
        maximum_working_set: usize,
        current_size_including_transition_in_pages: u32,
        peak_size_including_transition_in_pages: u32,
        transition_shared_pages: u32,
        transition_shared_pages_peak: u32,
        reserved: [u32; 2],
    }

    fn set_memory_list(command: u32) -> bool {
        let Some(nt) = nt_set_system_information() else {
            return false;
        };
        let value: u32 = command;
        let status = unsafe {
            nt(
                SYSTEM_MEMORY_LIST_INFORMATION,
                &value as *const u32 as *const std::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            )
        };
        status.is_ok()
    }

    pub fn purge_low_priority_standby() -> bool {
        set_memory_list(super::MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST)
    }

    pub fn purge_standby_list() -> bool {
        set_memory_list(MEMORY_PURGE_STANDBY_LIST)
    }

    pub fn purge_modified_list() -> bool {
        set_memory_list(MEMORY_PURGE_MODIFIED_LIST)
    }

    /// Trim the system file cache working set. Requires
    /// `SE_INCREASE_QUOTA_PRIVILEGE`; the helper enables it before calling.
    pub fn trim_system_file_cache() -> bool {
        let Some(nt) = nt_set_system_information() else {
            return false;
        };
        let mut info = FileCacheInformation::default();
        info.minimum_working_set = usize::MAX;
        info.maximum_working_set = usize::MAX;
        let status = unsafe {
            nt(
                SYSTEM_FILE_CACHE_INFORMATION,
                &info as *const FileCacheInformation as *const std::ffi::c_void,
                std::mem::size_of::<FileCacheInformation>() as u32,
            )
        };
        status.is_ok()
    }

    /// Trim the working set of every process the helper can open. Failures
    /// (access denied, process exited) are counted but do not abort the loop —
    /// memreduct applies the same tolerant policy.
    ///
    /// Uses `sysinfo` for process enumeration to avoid hand-rolling
    /// `NtQuerySystemInformation` parsing. The helper links the same crate.
    pub fn trim_all_process_working_sets(outcome: &mut FullTierOutcome) {
        let mut system = sysinfo::System::new_all();
        system.refresh_processes();

        let sentinel = usize::MAX;
        for (_pid, process) in system.processes() {
            // SAFETY: We open each process with limited rights; a failed open
            // (access denied / not found) is recorded but not fatal. The -1/-1
            // sentinel is the documented "trim" request.
            let pid = process.pid().as_u32();
            let handle_result = unsafe {
                OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA,
                    false,
                    pid,
                )
            };
            let Ok(handle) = handle_result else {
                outcome.processes_failed += 1;
                continue;
            };
            let result = unsafe {
                SetProcessWorkingSetSizeEx(handle, sentinel, sentinel, QUOTA_LIMITS_HARDWS_MIN_DISABLE)
            };
            if result.is_ok() {
                outcome.processes_trimmed += 1;
            } else {
                outcome.processes_failed += 1;
            }
            // SAFETY: handle is a real (non-pseudo) handle and must be closed.
            let _ = unsafe { CloseHandle(handle) };
        }

        // Also trim ourselves — pseudo-handle, no close needed.
        let self_handle: HANDLE = unsafe { GetCurrentProcess() };
        let _ = unsafe {
            SetProcessWorkingSetSizeEx(
                self_handle,
                sentinel,
                sentinel,
                QUOTA_LIMITS_HARDWS_MIN_DISABLE,
            )
        };
    }

    /// Run the full-tier sequence and return a per-step outcome.
    pub fn run_full() -> FullTierOutcome {
        let mut outcome = FullTierOutcome::default();
        // Order mirrors memreduct: system-wide lists first, then file cache,
        // then per-process working sets (which may fault pages back from the
        // freshly-purged standby lists, so doing them last is moot but matches
        // the reference).
        outcome.purged_low_priority_standby = purge_low_priority_standby();
        outcome.purged_standby_list = purge_standby_list();
        outcome.purged_modified_list = purge_modified_list();
        outcome.trimmed_system_file_cache = trim_system_file_cache();
        trim_all_process_working_sets(&mut outcome);
        outcome
    }
}
