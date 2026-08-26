//! Built-in CPU/GPU temperature probe.
//!
//! Reliable CPU sensors require privileged hardware access on Windows. The
//! bundled .NET helper embeds LibreHardwareMonitorLib, runs elevated after the
//! user enables the feature, and atomically publishes a small local JSON
//! sample. CoworkPal never depends on a separately installed or running app.

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;

const SAMPLE_MAX_AGE_MS: i64 = 10_000;
const SAMPLE_MAX_FUTURE_SKEW_MS: i64 = 30_000;
const LAUNCH_RETRY_INTERVAL: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct IntegratedHardwareMonitorSample {
    pub cpu_temperature_celsius: Option<f32>,
    pub gpu_temperature_celsius: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HelperSample {
    timestamp_ms: i64,
    cpu_temperature_celsius: Option<f32>,
    gpu_temperature_celsius: Option<f32>,
}

pub struct IntegratedHardwareMonitorProbe {
    output_path: PathBuf,
    last_launch_attempt: Option<Instant>,
    launch_suppressed: bool,
    diagnostic_logged: bool,
    error_logged: bool,
    #[cfg(windows)]
    process: Option<windows::Win32::Foundation::HANDLE>,
}

// Windows process handles are safe to wait/close from another thread. Access
// to this probe is additionally serialized by the adapter's Mutex.
#[cfg(windows)]
unsafe impl Send for IntegratedHardwareMonitorProbe {}
#[cfg(windows)]
unsafe impl Sync for IntegratedHardwareMonitorProbe {}

impl IntegratedHardwareMonitorProbe {
    pub fn new() -> Self {
        Self {
            output_path: helper_output_path(),
            last_launch_attempt: None,
            launch_suppressed: false,
            diagnostic_logged: false,
            error_logged: false,
            #[cfg(windows)]
            process: None,
        }
    }

    pub fn sample(&mut self, enabled: bool) -> Option<IntegratedHardwareMonitorSample> {
        if !enabled {
            self.stop();
            return None;
        }

        self.ensure_running();
        self.log_helper_sidecars();
        read_fresh_sample(&self.output_path, crate::models::current_timestamp_ms())
    }

    fn ensure_running(&mut self) {
        #[cfg(windows)]
        {
            if self.helper_is_running() {
                return;
            }
            if let Some(exit_code) = self.close_process_handle() {
                if exit_code != 0 {
                    tracing::warn!(
                        exit_code,
                        "integrated hardware monitor helper exited unexpectedly"
                    );
                }
            }
            if self.launch_suppressed {
                return;
            }

            let now = Instant::now();
            if self
                .last_launch_attempt
                .is_some_and(|attempt| now.duration_since(attempt) < LAUNCH_RETRY_INTERVAL)
            {
                return;
            }
            self.last_launch_attempt = Some(now);
            self.diagnostic_logged = false;
            self.error_logged = false;

            let Some(helper_path) = resolve_helper_path() else {
                tracing::warn!("integrated hardware monitor helper was not found");
                return;
            };
            if let Some(directory) = self.output_path.parent() {
                if let Err(error) = std::fs::create_dir_all(directory) {
                    tracing::warn!(
                        path = %directory.display(),
                        %error,
                        "failed to prepare integrated hardware monitor output directory"
                    );
                }
            }
            let _ = std::fs::remove_file(&self.output_path);

            match launch_elevated(&helper_path, &self.output_path) {
                Ok(process) => {
                    tracing::info!(
                        output_path = %self.output_path.display(),
                        "integrated hardware monitor helper started"
                    );
                    self.process = Some(process);
                }
                Err(LaunchError::UserDeclined) => {
                    tracing::info!("integrated hardware monitor elevation was declined");
                    self.launch_suppressed = true;
                }
                Err(LaunchError::Failed(code)) => {
                    tracing::warn!(
                        "failed to start integrated hardware monitor helper: Windows error {code}"
                    );
                }
            }
        }
    }

    fn stop(&mut self) {
        #[cfg(windows)]
        {
            use windows::Win32::System::Threading::{TerminateProcess, WaitForSingleObject};

            if let Some(process) = self.process.take() {
                if process_is_running(process) {
                    let _ = unsafe { TerminateProcess(process, 0) };
                    let _ = unsafe { WaitForSingleObject(process, 2_000) };
                }
                close_handle(process);
            }
        }
        self.last_launch_attempt = None;
        self.launch_suppressed = false;
        self.diagnostic_logged = false;
        self.error_logged = false;
        let _ = std::fs::remove_file(&self.output_path);
        let _ = std::fs::remove_file(sidecar_path(&self.output_path, ".diagnostic.log"));
        let _ = std::fs::remove_file(sidecar_path(&self.output_path, ".error.log"));
    }

    #[cfg(windows)]
    fn helper_is_running(&self) -> bool {
        self.process.is_some_and(process_is_running)
    }

    #[cfg(windows)]
    fn close_process_handle(&mut self) -> Option<u32> {
        let process = self.process.take()?;
        let exit_code = process_exit_code(process);
        close_handle(process);
        exit_code
    }

    #[cfg(windows)]
    fn log_helper_sidecars(&mut self) {
        if !self.diagnostic_logged {
            if let Ok(diagnostic) =
                std::fs::read_to_string(sidecar_path(&self.output_path, ".diagnostic.log"))
            {
                let diagnostic = diagnostic.trim();
                if !diagnostic.is_empty() {
                    tracing::warn!(
                        diagnostic,
                        "integrated hardware monitor did not report a CPU temperature"
                    );
                    self.diagnostic_logged = true;
                }
            }
        }

        if !self.error_logged {
            if let Ok(error) =
                std::fs::read_to_string(sidecar_path(&self.output_path, ".error.log"))
            {
                let error = error.trim();
                if !error.is_empty() {
                    tracing::warn!(error, "integrated hardware monitor helper failed");
                    self.error_logged = true;
                }
            }
        }
    }
}

impl Drop for IntegratedHardwareMonitorProbe {
    fn drop(&mut self) {
        self.stop();
    }
}

fn read_fresh_sample(path: &Path, now_ms: i64) -> Option<IntegratedHardwareMonitorSample> {
    let content = std::fs::read_to_string(path).ok()?;
    let sample = serde_json::from_str::<HelperSample>(&content).ok()?;
    normalize_sample(sample, now_ms)
}

fn normalize_sample(sample: HelperSample, now_ms: i64) -> Option<IntegratedHardwareMonitorSample> {
    let age_ms = now_ms.saturating_sub(sample.timestamp_ms);
    if !(-SAMPLE_MAX_FUTURE_SKEW_MS..=SAMPLE_MAX_AGE_MS).contains(&age_ms) {
        return None;
    }

    let result = IntegratedHardwareMonitorSample {
        cpu_temperature_celsius: sample
            .cpu_temperature_celsius
            .and_then(sanitize_temperature),
        gpu_temperature_celsius: sample
            .gpu_temperature_celsius
            .and_then(sanitize_temperature),
    };
    (result.cpu_temperature_celsius.is_some() || result.gpu_temperature_celsius.is_some())
        .then_some(result)
}

fn sanitize_temperature(value: f32) -> Option<f32> {
    (value.is_finite() && value > 0.0 && value <= 125.0).then_some(value)
}

fn helper_output_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    #[cfg(windows)]
    let directory = std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
        .join("CoworkPal")
        .join("HardwareMonitor");
    #[cfg(not(windows))]
    let directory = std::env::temp_dir();

    directory.join(format!(
        "coworkpal-hardware-monitor-{}-{nonce}.json",
        std::process::id()
    ))
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    value.into()
}

#[cfg(windows)]
fn resolve_helper_path() -> Option<PathBuf> {
    const PACKAGED_NAME: &str = "coworkpal-hardware-monitor.exe";
    const DEVELOPMENT_NAME: &str = "coworkpal-hardware-monitor-x86_64-pc-windows-msvc.exe";

    let adjacent = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join(PACKAGED_NAME)));
    let development = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(DEVELOPMENT_NAME);

    adjacent
        .into_iter()
        .chain(std::iter::once(development))
        .find(|path| path.is_file())
}

#[cfg(windows)]
#[derive(Debug)]
enum LaunchError {
    UserDeclined,
    Failed(u32),
}

#[cfg(windows)]
fn launch_elevated(
    helper_path: &Path,
    output_path: &Path,
) -> Result<windows::Win32::Foundation::HANDLE, LaunchError> {
    use windows::{
        core::{w, HSTRING, PCWSTR},
        Win32::{
            Foundation::GetLastError,
            UI::{
                Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW},
                WindowsAndMessaging::SW_HIDE,
            },
        },
    };

    let file = HSTRING::from(helper_path.as_os_str());
    let parameters = HSTRING::from(format!(
        "--parent-pid={} --output=\"{}\"",
        std::process::id(),
        output_path.display()
    ));
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(parameters.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };

    if unsafe { ShellExecuteExW(&mut info) }.is_err() {
        let code = unsafe { GetLastError() }.0;
        return if code == 1223 {
            Err(LaunchError::UserDeclined)
        } else {
            Err(LaunchError::Failed(code))
        };
    }
    if info.hProcess.is_invalid() {
        return Err(LaunchError::Failed(0));
    }
    Ok(info.hProcess)
}

#[cfg(windows)]
fn process_is_running(process: windows::Win32::Foundation::HANDLE) -> bool {
    use windows::Win32::{Foundation::WAIT_TIMEOUT, System::Threading::WaitForSingleObject};
    (unsafe { WaitForSingleObject(process, 0) }) == WAIT_TIMEOUT
}

#[cfg(windows)]
fn process_exit_code(process: windows::Win32::Foundation::HANDLE) -> Option<u32> {
    use windows::Win32::System::Threading::GetExitCodeProcess;

    let mut exit_code = 0;
    unsafe { GetExitCodeProcess(process, &mut exit_code) }
        .is_ok()
        .then_some(exit_code)
}

#[cfg(windows)]
fn close_handle(handle: windows::Win32::Foundation::HANDLE) {
    use windows::Win32::Foundation::CloseHandle;
    let _ = unsafe { CloseHandle(handle) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_fresh_plausible_sample() {
        let sample = HelperSample {
            timestamp_ms: 100_000,
            cpu_temperature_celsius: Some(57.25),
            gpu_temperature_celsius: Some(63.0),
        };
        assert_eq!(
            normalize_sample(sample, 105_000),
            Some(IntegratedHardwareMonitorSample {
                cpu_temperature_celsius: Some(57.25),
                gpu_temperature_celsius: Some(63.0),
            })
        );
    }

    #[test]
    fn rejects_stale_and_zero_sensor_values() {
        let stale = HelperSample {
            timestamp_ms: 1,
            cpu_temperature_celsius: Some(55.0),
            gpu_temperature_celsius: None,
        };
        assert_eq!(normalize_sample(stale, 100_000), None);

        let zero = HelperSample {
            timestamp_ms: 100_000,
            cpu_temperature_celsius: Some(0.0),
            gpu_temperature_celsius: None,
        };
        assert_eq!(normalize_sample(zero, 100_000), None);
    }

    #[cfg(windows)]
    #[test]
    fn helper_output_uses_shared_program_data_directory() {
        let output_path = helper_output_path();
        let expected_directory = std::env::var_os("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
            .join("CoworkPal")
            .join("HardwareMonitor");

        assert_eq!(output_path.parent(), Some(expected_directory.as_path()));
        assert!(output_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(
                |name| name.starts_with("coworkpal-hardware-monitor-") && name.ends_with(".json")
            ));
    }
}
