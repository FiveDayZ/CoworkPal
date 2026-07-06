//! Read-only queries of system memory pressure via `GlobalMemoryStatusEx`.
//!
//! Used both by the auto-release watcher (to compare against the configured GB
//! threshold) and to report the "released bytes" delta around a cleanup. The
//! function never touches any cleanup APIs — it only samples.

use serde::{Deserialize, Serialize};

/// Snapshot of physical memory pressure at a point in time.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MemoryPressure {
    /// Total physical memory in bytes.
    pub total_bytes: u64,
    /// Available (free + standby) physical memory in bytes.
    pub available_bytes: u64,
    /// Used physical memory = total - available.
    pub used_bytes: u64,
    /// `dwMemoryLoad` — system memory load percentage reported by Windows.
    pub load_percent: u32,
}

impl MemoryPressure {
    /// Used memory expressed in GiB (1 GiB = 1<<30 bytes).
    pub fn used_gib(&self) -> f64 {
        self.used_bytes as f64 / (1u64 << 30) as f64
    }

    /// Delta in bytes (before.available - after.available). A positive value
    /// means available memory grew after cleanup.
    pub fn released_since(before: &MemoryPressure, after: &MemoryPressure) -> i64 {
        after.available_bytes as i64 - before.available_bytes as i64
    }
}

/// Sample current physical memory pressure.
///
/// Returns `None` only when the Windows call itself fails, which is extremely
/// rare (it fails only when the struct size is wrong). Callers may safely
/// treat `None` as "no sample" and skip the cycle.
#[cfg(windows)]
pub fn sample_pressure() -> Option<MemoryPressure> {
    use windows::Win32::System::SystemInformation::{
        GlobalMemoryStatusEx, MEMORYSTATUSEX,
    };

    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };

    // SAFETY: `GlobalMemoryStatusEx` reads system memory state into the
    // caller-provided struct. We pass the correct size in dwLength; the struct
    // is stack-allocated and lives across the call.
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) }.is_ok();
    if !ok {
        return None;
    }

    Some(MemoryPressure {
        total_bytes: status.ullTotalPhys,
        available_bytes: status.ullAvailPhys,
        used_bytes: status.ullTotalPhys.saturating_sub(status.ullAvailPhys),
        load_percent: status.dwMemoryLoad,
    })
}

#[cfg(not(windows))]
pub fn sample_pressure() -> Option<MemoryPressure> {
    // Non-Windows builds are unsupported — auto-release simply never triggers.
    None
}
