use std::mem::size_of;

use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1},
        System::Performance::{
            PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData,
            PdhGetFormattedCounterArrayW, PdhGetFormattedCounterValue, PdhOpenQueryW,
            PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE,
            PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
        },
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    },
};

const ERROR_SUCCESS: u32 = 0;

pub fn query_foreground_process_id() -> Option<u32> {
    let window = unsafe { GetForegroundWindow() };
    if window.0.is_null() {
        return None;
    }

    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(window, Some(&mut process_id));
    }
    (process_id > 0).then_some(process_id)
}

#[derive(Debug, Clone, Default)]
pub struct WindowsPerformanceSample {
    pub disk_read_bytes_per_second: Option<f32>,
    pub disk_write_bytes_per_second: Option<f32>,
    pub gpu_usage_percent: Option<f32>,
    pub gpu_memory_used_bytes: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct WindowsGpuInfo {
    pub name: Option<String>,
    pub dedicated_memory_bytes: Option<u64>,
}

pub struct WindowsPerformanceCounters {
    query: PDH_HQUERY,
    disk_read: Option<PDH_HCOUNTER>,
    disk_write: Option<PDH_HCOUNTER>,
    gpu_engine_usage: Option<PDH_HCOUNTER>,
    gpu_dedicated_memory: Option<PDH_HCOUNTER>,
}

unsafe impl Send for WindowsPerformanceCounters {}
unsafe impl Sync for WindowsPerformanceCounters {}

impl WindowsPerformanceCounters {
    pub fn new() -> Option<Self> {
        let mut query = PDH_HQUERY::default();
        let status = unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) };
        if status != ERROR_SUCCESS || query.is_invalid() {
            return None;
        }

        let mut counters = Self {
            query,
            disk_read: None,
            disk_write: None,
            gpu_engine_usage: None,
            gpu_dedicated_memory: None,
        };

        counters.disk_read = counters.add_counter(r"\PhysicalDisk(_Total)\Disk Read Bytes/sec");
        counters.disk_write = counters.add_counter(r"\PhysicalDisk(_Total)\Disk Write Bytes/sec");
        counters.gpu_engine_usage = counters.add_counter(r"\GPU Engine(*)\Utilization Percentage");
        counters.gpu_dedicated_memory =
            counters.add_counter(r"\GPU Adapter Memory(*)\Dedicated Usage");

        unsafe {
            let _ = PdhCollectQueryData(counters.query);
        }

        Some(counters)
    }

    pub fn sample(&mut self) -> WindowsPerformanceSample {
        let status = unsafe { PdhCollectQueryData(self.query) };
        if status != ERROR_SUCCESS {
            return WindowsPerformanceSample::default();
        }

        let gpu_usage_percent = self
            .gpu_engine_usage
            .and_then(|counter| self.formatted_array_sum(counter, Some("engtype_3D")))
            .or_else(|| {
                self.gpu_engine_usage
                    .and_then(|counter| self.formatted_array_sum(counter, None))
            })
            .map(|value| value.clamp(0.0, 100.0));

        WindowsPerformanceSample {
            disk_read_bytes_per_second: self.disk_read.and_then(|counter| {
                self.formatted_counter_value(counter)
                    .map(|value| value.max(0.0) as f32)
            }),
            disk_write_bytes_per_second: self.disk_write.and_then(|counter| {
                self.formatted_counter_value(counter)
                    .map(|value| value.max(0.0) as f32)
            }),
            gpu_usage_percent: gpu_usage_percent.map(|value| value as f32),
            gpu_memory_used_bytes: self
                .gpu_dedicated_memory
                .and_then(|counter| self.formatted_array_sum(counter, None))
                .map(|value| value.max(0.0) as u64),
        }
    }

    fn add_counter(&self, path: &str) -> Option<PDH_HCOUNTER> {
        let wide = to_wide(path);
        let mut counter = PDH_HCOUNTER::default();
        let status =
            unsafe { PdhAddEnglishCounterW(self.query, PCWSTR(wide.as_ptr()), 0, &mut counter) };

        if status == ERROR_SUCCESS && !counter.is_invalid() {
            Some(counter)
        } else {
            None
        }
    }

    fn formatted_counter_value(&self, counter: PDH_HCOUNTER) -> Option<f64> {
        let mut value = PDH_FMT_COUNTERVALUE::default();
        let status =
            unsafe { PdhGetFormattedCounterValue(counter, PDH_FMT_DOUBLE, None, &mut value) };

        if status != ERROR_SUCCESS || !is_valid_counter_status(value.CStatus) {
            return None;
        }

        Some(unsafe { value.Anonymous.doubleValue })
    }

    fn formatted_array_sum(&self, counter: PDH_HCOUNTER, name_filter: Option<&str>) -> Option<f64> {
        let values = self.formatted_counter_array(counter, name_filter)?;
        let sum = values.into_iter().sum::<f64>();
        Some(sum)
    }

    fn formatted_counter_array(
        &self,
        counter: PDH_HCOUNTER,
        name_filter: Option<&str>,
    ) -> Option<Vec<f64>> {
        let mut buffer_size = 0;
        let mut item_count = 0;
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                None,
            )
        };

        if status != PDH_MORE_DATA || buffer_size == 0 || item_count == 0 {
            return None;
        }

        let item_capacity = (buffer_size as usize + size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>() - 1)
            / size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        let mut buffer = vec![PDH_FMT_COUNTERVALUE_ITEM_W::default(); item_capacity.max(1)];
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                Some(buffer.as_mut_ptr()),
            )
        };

        if status != ERROR_SUCCESS {
            return None;
        }

        let filter = name_filter.map(str::to_ascii_lowercase);
        let mut values = Vec::new();
        for item in buffer.iter().take(item_count as usize) {
            if !is_valid_counter_status(item.FmtValue.CStatus) {
                continue;
            }

            if let Some(filter) = &filter {
                let name = pwstr_to_string(item.szName).to_ascii_lowercase();
                if !name.contains(filter) {
                    continue;
                }
            }

            let value = unsafe { item.FmtValue.Anonymous.doubleValue };
            if value.is_finite() {
                values.push(value);
            }
        }

        if values.is_empty() {
            None
        } else {
            Some(values)
        }
    }
}

impl Drop for WindowsPerformanceCounters {
    fn drop(&mut self) {
        if !self.query.is_invalid() {
            unsafe {
                let _ = PdhCloseQuery(self.query);
            }
        }
    }
}

pub fn query_primary_gpu_info() -> WindowsGpuInfo {
    unsafe {
        let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else {
            return WindowsGpuInfo::default();
        };

        let mut best = WindowsGpuInfo::default();
        let mut index = 0;
        while let Ok(adapter) = factory.EnumAdapters1(index) {
            if let Ok(desc) = adapter.GetDesc1() {
                let name = wide_array_to_string(&desc.Description);
                let dedicated_memory_bytes = desc.DedicatedVideoMemory as u64;
                let current_best = best.dedicated_memory_bytes.unwrap_or(0);

                if best.name.is_none() || dedicated_memory_bytes > current_best {
                    best = WindowsGpuInfo {
                        name: (!name.is_empty()).then_some(name),
                        dedicated_memory_bytes: (dedicated_memory_bytes > 0)
                            .then_some(dedicated_memory_bytes),
                    };
                }
            }
            index += 1;
        }

        best
    }
}

fn is_valid_counter_status(status: u32) -> bool {
    status == PDH_CSTATUS_VALID_DATA || status == PDH_CSTATUS_NEW_DATA
}

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_array_to_string(value: &[u16]) -> String {
    let len = value
        .iter()
        .position(|item| *item == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..len]).trim().to_string()
}

fn pwstr_to_string(value: PWSTR) -> String {
    if value.0.is_null() {
        return String::new();
    }

    let mut len = 0usize;
    unsafe {
        while *value.0.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(value.0, len))
    }
}
