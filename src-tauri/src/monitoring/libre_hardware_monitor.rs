//! Optional high-precision CPU temperature source backed by a running
//! LibreHardwareMonitor (LHM) instance.
//!
//! LHM runs elevated and reads core MSRs / Super IO, then exposes the values
//! over two channels any non-.NET process can consume:
//!
//! 1. **REST API** (`http://127.0.0.1:8085/data.json`) — the stable path on
//!    LHM v0.9.5+, where the legacy WMI provider was removed.
//! 2. **WMI namespace** `root\LibreHardwareMonitor` — the legacy path kept for
//!    older LHM versions or installs with the REST server disabled.
//!
//! Both probes are best-effort: when LHM is not installed / not running / not
//! elevated, every call here returns `None` and the caller falls back to the
//! existing sysinfo / ACPI thermal-zone sources via the `.or()` chain in
//! `sysinfo_adapter.rs`. There is zero cost to a machine without LHM (a local
//! TCP connection-refused resolves in sub-millisecond time).
//!
//! CoreWorkPal itself never requires elevation — it merely *reads* whatever an
//! already-elevated LHM process publishes.

use serde::Deserialize;

/// Result of a successful LHM probe. Only CPU package temperature is surfaced
/// today; the struct is here so adding more fields later stays cheap.
#[derive(Debug, Clone, Default)]
pub struct LibreHardwareMonitorSample {
    pub cpu_package_temperature: Option<f32>,
}

/// Public entry point: tries REST first, then WMI. Returns `None` entirely if
/// neither channel yields a plausible CPU package temperature.
pub fn query_libre_hardware_monitor() -> Option<LibreHardwareMonitorSample> {
    query_via_rest()
        .or_else(query_via_wmi)
        .and_then(|temp| {
            plausible(temp).then_some(LibreHardwareMonitorSample {
                cpu_package_temperature: Some(temp),
            })
        })
}

// ---------------------------------------------------------------------------
// REST path
// ---------------------------------------------------------------------------

/// LHM's `/data.json` endpoint publishes a recursive node tree. Each node has a
/// `Text` label and optionally a `Children` array and/or a `Children` of
/// `Values`-style sensor entries. The exact field names differ slightly across
/// LHM builds, so we model the union and walk defensively.
#[derive(Debug, Deserialize)]
struct LhmRestNode {
    #[serde(default)]
    text: serde_json::Value,
    #[serde(default, rename = "Children")]
    children: Vec<LhmRestNode>,
    /// Sensor reading — present on leaf nodes that report a value.
    #[serde(default)]
    value: serde_json::Value,
    /// LHM REST uses `ImageURL` to encode the sensor kind/hardware family
    /// (e.g. `images_icon/cpu.png`). Used as a hint that a subtree is the CPU.
    #[serde(default, rename = "ImageURL")]
    image_url: serde_json::Value,
}

fn query_via_rest() -> Option<f32> {
    // Short timeout: LHM is always localhost, so anything beyond a moment is
    // really "not running" and we should fall through quickly.
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(800))
        .build()
        .ok()?;

    let body = client
        .get("http://127.0.0.1:8085/data.json")
        .send()
        .ok()?
        .json::<LhmRestNode>()
        .ok()?;

    // Walk the tree collecting (label, value) pairs that look like CPU
    // temperatures. We score "CPU Package" highest, then any temperature whose
    // path contained a cpu-like node.
    let mut best: Option<(f32, u8)> = None;
    walk_for_cpu_temperature(&body, false, &mut best);
    best.map(|(value, _)| value)
}

fn walk_for_cpu_temperature(node: &LhmRestNode, under_cpu: bool, best: &mut Option<(f32, u8)>) {
    let label = as_str(&node.text).to_ascii_lowercase();
    let image = as_str(&node.image_url).to_ascii_lowercase();
    let in_cpu_subtree = under_cpu || label.contains("cpu") || image.contains("cpu");

    // Leaf temperature reading?
    if let Some(value) = parse_value(&node.value) {
        if plausible(value) && (label.contains("temperature") || label.contains("package") || label.contains("tctl") || label.contains("tdie"))
        {
            // "CPU Package" / "Tctl" on the CPU subtree is the gold reading.
            let score = if in_cpu_subtree
                && (label.contains("package") || label.contains("tctl") || label.contains("tdie"))
            {
                3
            } else if in_cpu_subtree {
                2
            } else {
                1
            };
            if best.map(|(_, s)| score > s).unwrap_or(true) {
                *best = Some((value, score));
            }
        }
    }

    for child in &node.children {
        walk_for_cpu_temperature(child, in_cpu_subtree, best);
    }
}

fn parse_value(raw: &serde_json::Value) -> Option<f32> {
    match raw {
        serde_json::Value::String(s) => s.trim().parse::<f32>().ok(),
        serde_json::Value::Number(n) => n.as_f64().map(|v| v as f32),
        _ => None,
    }
}

fn as_str(raw: &serde_json::Value) -> &str {
    raw.as_str().unwrap_or("")
}

// ---------------------------------------------------------------------------
// WMI path (legacy / fallback)
// ---------------------------------------------------------------------------

fn query_via_wmi() -> Option<f32> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // Ask for CPU temperature sensors only; LHM's `Parent` encodes the
        // hardware family (e.g. `/intelcpu/0`, `/amdcpu/0`). ConvertTo-Json so
        // we can parse robustly instead of scraping free-form text.
        let script = "Get-CimInstance -Namespace root/LibreHardwareMonitor -ClassName Sensor | Where-Object { $_.SensorType -eq 'Temperature' -and $_.Parent -like '*cpu*' } | Select-Object Name,Value | ConvertTo-Json -Compress";

        let mut command = std::process::Command::new("powershell.exe");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ]);
        command.creation_flags(CREATE_NO_WINDOW);

        // Bounded: if the root/LibreHardwareMonitor WMI namespace wedges, the
        // powershell call can hang and would otherwise hold the adapter lock.
        let output = crate::process_util::run_command_with_timeout(
            command,
            crate::process_util::DEFAULT_SUBPROCESS_TIMEOUT,
        )?;
        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_wmi_temperature(&stdout)
    }

    #[cfg(not(windows))]
    {
        None
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WmiSensor {
    #[serde(default)]
    name: serde_json::Value,
    #[serde(default)]
    value: serde_json::Value,
}

fn parse_wmi_temperature(stdout: &str) -> Option<f32> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return None;
    }

    // ConvertTo-Json returns a single object when there's one result, array
    // otherwise — normalize to a slice.
    let sensors: Vec<WmiSensor> = if trimmed.starts_with('[') {
        serde_json::from_str(trimmed).ok()?
    } else {
        let one: WmiSensor = serde_json::from_str(trimmed).ok()?;
        vec![one]
    };

    let mut best: Option<(f32, u8)> = None;
    for sensor in sensors {
        let value = match &sensor.value {
            serde_json::Value::String(s) => s.trim().parse::<f32>().ok(),
            serde_json::Value::Number(n) => n.as_f64().map(|v| v as f32),
            _ => None,
        };
        let value = match value {
            Some(v) if plausible(v) => v,
            _ => continue,
        };

        let name = sensor.name.as_str().unwrap_or("").to_ascii_lowercase();
        // "CPU Package" is LHM's headline CPU reading.
        let score = if name.contains("package") {
            3
        } else {
            1
        };
        if best.map(|(_, s)| score > s).unwrap_or(true) {
            best = Some((value, score));
        }
    }

    best.map(|(value, _)| value)
}

fn plausible(value: f32) -> bool {
    value.is_finite() && (-30.0..=125.0).contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_wmi_sensor_object() {
        let json = r#"{"Name":"CPU Package","Value":42.5}"#;
        assert_eq!(parse_wmi_temperature(json), Some(42.5));
    }

    #[test]
    fn parses_wmi_sensor_array_prefers_package() {
        let json = r#"[{"Name":"CPU Core #1","Value":38.0},{"Name":"CPU Package","Value":45.0}]"#;
        assert_eq!(parse_wmi_temperature(json), Some(45.0));
    }

    #[test]
    fn rejects_implausible_wmi_values() {
        // 150°C is outside the plausible range (-30..125), so it must be
        // dropped. (Note: 0°C is plausible and is NOT rejected — an earlier
        // version of this test wrongly expected None for 0, which only
        // "passed" because the parser was entirely broken at the time.)
        let json = r#"{"Name":"CPU Package","Value":150}"#;
        assert_eq!(parse_wmi_temperature(json), None);
    }

    #[test]
    fn empty_stdout_yields_none() {
        assert_eq!(parse_wmi_temperature("   "), None);
    }
}
