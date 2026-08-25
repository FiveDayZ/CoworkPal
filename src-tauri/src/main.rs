#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Elevated self-restart path: when the main app re-launches itself with
    // `--memory-clean-helper --result-file=<path>` (via ShellExecuteW("runas")),
    // we skip the normal startup (single-instance lock, Tauri init) and instead
    // run the full-tier memory sweep, write the result, and exit. This keeps
    // elevation self-contained — no separate helper binary to ship.
    if try_run_memory_helper_mode() {
        return;
    }

    if !acquire_single_instance() {
        return;
    }

    cowork_pal_lib::run()
}

/// Detect the `--memory-clean-helper` argv flag and, if present, run the
/// full-tier cleanup, write `{"releasedBytes": N}` to the `--result-file`
/// path, and return `true` so `main` exits immediately.
///
/// Returns `false` for the normal app launch (no helper flag).
fn try_run_memory_helper_mode() -> bool {
    let args: Vec<String> = std::env::args().collect();
    if !args.iter().any(|a| a == "--memory-clean-helper") {
        return false;
    }

    let result_file = args.iter().find_map(|a| {
        a.strip_prefix("--result-file=").map(|rest| {
            std::path::PathBuf::from(rest.trim_matches('"'))
        })
    });

    let released = cowork_pal_lib::memory_release::run_full_clean_inplace();
    let payload = format!("{{\"releasedBytes\":{}}}", released);

    if let Some(path) = result_file {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::File::create(&path) {
            use std::io::Write;
            let _ = file.write_all(payload.as_bytes());
            let _ = file.flush();
        }
    } else {
        println!("{}", payload);
    }
    true
}

#[cfg(windows)]
fn acquire_single_instance() -> bool {
    use std::sync::OnceLock;
    use windows::{
        core::w,
        Win32::{
            Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS},
            System::Threading::CreateMutexW,
        },
    };

    static INSTANCE_MUTEX: OnceLock<usize> = OnceLock::new();
    let restart_pending = std::env::var_os("COWORKPAL_RESTART_PENDING").is_some();

    for _ in 0..50 {
        let Ok(handle) =
            (unsafe { CreateMutexW(None, true, w!("Local\\CoworkPal.SingleInstance")) })
        else {
            return true;
        };

        if unsafe { GetLastError() } != ERROR_ALREADY_EXISTS {
            let _ = INSTANCE_MUTEX.set(handle.0 as usize);
            std::env::remove_var("COWORKPAL_RESTART_PENDING");
            return true;
        }

        let _ = unsafe { CloseHandle(handle) };
        if !restart_pending {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    false
}

#[cfg(not(windows))]
fn acquire_single_instance() -> bool {
    true
}
