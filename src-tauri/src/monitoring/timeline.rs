use crate::models::{AppSettings, HardwareSnapshot, ProcessUsageSnapshot};
use serde::Serialize;
use std::collections::VecDeque;

const WINDOW_MS: i64 = 10 * 60 * 1000;
const MAX_GAP_MS: i64 = 15_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceSample {
    pub timestamp: i64,
    pub values: [Option<f32>; 4],
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceEvent {
    pub metric: usize,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub last_seen_at: i64,
    pub interrupted: bool,
    pub peak: f32,
    pub threshold: f32,
    pub processes: Vec<ProcessUsageSnapshot>,
}

#[derive(Debug, Clone, Default)]
struct Detector {
    since: Option<i64>,
    threshold: Option<f32>,
    peak: f32,
    processes: Vec<ProcessUsageSnapshot>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceHistory {
    pub samples: VecDeque<PerformanceSample>,
    pub events: VecDeque<PerformanceEvent>,
    #[serde(skip)]
    detectors: [Detector; 4],
}

impl PerformanceHistory {
    pub fn record(&mut self, snapshot: &HardwareSnapshot, settings: &AppSettings) {
        let now = snapshot.timestamp;
        let previous = self.samples.back().map(|sample| sample.timestamp);
        if previous.is_some_and(|previous| now <= previous) {
            return;
        }
        let gap = previous.is_some_and(|previous| now - previous > MAX_GAP_MS);
        let values = [
            snapshot.cpu_usage_percent,
            snapshot.memory_usage_percent,
            snapshot.cpu_temperature_celsius,
            snapshot.gpu_temperature_celsius,
        ]
        .map(|value| value.filter(|value| value.is_finite()));
        let thresholds = [
            80.0,
            settings.memory_crowded_threshold,
            settings.cpu_temperature_warning,
            settings.gpu_temperature_warning,
        ];
        self.samples.push_back(PerformanceSample {
            timestamp: now,
            values,
        });
        while self.samples.len() > 600
            || self
                .samples
                .front()
                .is_some_and(|sample| sample.timestamp < now - WINDOW_MS)
        {
            self.samples.pop_front();
        }
        for metric in 0..4 {
            let threshold = thresholds[metric];
            let changed = self.detectors[metric]
                .threshold
                .is_some_and(|old| old != threshold);
            if gap || changed || values[metric].is_none() {
                self.close(metric, previous.unwrap_or(now), true);
                self.detectors[metric].since = None;
            }
            self.detectors[metric].threshold = Some(threshold);
            let Some(value) = values[metric] else {
                continue;
            };
            let active = self
                .events
                .iter()
                .position(|event| event.metric == metric && event.ended_at.is_none());
            if let Some(index) = active {
                if value < threshold - 10.0 {
                    self.close(metric, now, false);
                    self.detectors[metric].since = None;
                } else {
                    let event = &mut self.events[index];
                    event.last_seen_at = now;
                    if value > event.peak {
                        event.peak = value;
                        event.processes = top_processes(snapshot, metric);
                    }
                }
            } else if value >= threshold {
                let detector = &mut self.detectors[metric];
                if detector.since.is_none() || value > detector.peak {
                    detector.peak = value;
                    detector.processes = top_processes(snapshot, metric);
                }
                let since = *detector.since.get_or_insert(now);
                if now - since >= 10_000 {
                    self.events.push_back(PerformanceEvent {
                        metric,
                        started_at: since,
                        ended_at: None,
                        last_seen_at: now,
                        interrupted: false,
                        peak: detector.peak,
                        threshold,
                        processes: detector.processes.clone(),
                    });
                }
            } else {
                self.detectors[metric].since = None;
            }
        }
        self.events
            .retain(|event| event.ended_at.unwrap_or(now) >= now - WINDOW_MS);
        while self.events.len() > 30 {
            self.events.pop_front();
        }
    }

    fn close(&mut self, metric: usize, at: i64, interrupted: bool) {
        if let Some(event) = self
            .events
            .iter_mut()
            .find(|event| event.metric == metric && event.ended_at.is_none())
        {
            event.ended_at = Some(at);
            event.last_seen_at = at;
            event.interrupted = interrupted;
        }
    }
}

fn top_processes(snapshot: &HardwareSnapshot, metric: usize) -> Vec<ProcessUsageSnapshot> {
    let mut processes = snapshot.processes.clone();
    if metric == 1 {
        processes.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    } else {
        processes.sort_by(|a, b| b.cpu_usage_percent.total_cmp(&a.cpu_usage_percent));
    }
    processes.truncate(3);
    processes
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(at: i64, cpu: Option<f32>) -> HardwareSnapshot {
        HardwareSnapshot {
            timestamp: at,
            cpu_usage_percent: cpu,
            ..Default::default()
        }
    }
    #[test]
    fn sustained_pressure_hysteresis_and_recovery() {
        let mut history = PerformanceHistory::default();
        let settings = AppSettings::default();
        for at in [0, 5000] {
            history.record(&sample(at, Some(90.0)), &settings);
        }
        assert!(history.events.is_empty());
        history.record(&sample(10_000, Some(95.0)), &settings);
        history.record(&sample(15_000, Some(75.0)), &settings);
        assert_eq!(history.events.len(), 1);
        assert_eq!(history.events[0].peak, 95.0);
        assert!(history.events[0].ended_at.is_none());
        history.record(&sample(20_000, Some(69.0)), &settings);
        assert_eq!(history.events[0].ended_at, Some(20_000));
    }
    #[test]
    fn confirmation_keeps_initial_peak_and_settings_changes_start_a_new_window() {
        let mut history = PerformanceHistory::default();
        let mut settings = AppSettings::default();
        history.record(&sample(0, Some(99.0)), &settings);
        history.record(&sample(10_000, Some(85.0)), &settings);
        assert_eq!(history.events[0].peak, 99.0);
        settings.memory_crowded_threshold = 60.0;
        let mut memory = sample(15_000, Some(0.0));
        memory.memory_usage_percent = Some(70.0);
        history.record(&memory, &settings);
        settings.memory_crowded_threshold = 50.0;
        memory.timestamp = 20_000;
        history.record(&memory, &settings);
        memory.timestamp = 25_000;
        history.record(&memory, &settings);
        assert_eq!(history.events.len(), 1);
        memory.timestamp = 30_000;
        history.record(&memory, &settings);
        assert_eq!(history.events[1].started_at, 20_000);
    }

    #[test]
    fn missing_samples_and_sleep_never_count_as_sustained_load() {
        let mut history = PerformanceHistory::default();
        let settings = AppSettings::default();
        for at in [0, 60_000, 120_000] {
            history.record(&sample(at, Some(90.0)), &settings);
        }
        assert!(history.events.is_empty());
        history.record(&sample(125_000, None), &settings);
        history.record(&sample(130_000, Some(90.0)), &settings);
        history.record(&sample(140_000, Some(90.0)), &settings);
        history.record(&sample(200_000, Some(90.0)), &settings);
        assert!(history.events[0].interrupted);
        assert_eq!(history.events[0].ended_at, Some(140_000));
    }
    #[test]
    fn history_is_bounded_and_drops_old_events() {
        let mut history = PerformanceHistory::default();
        let settings = AppSettings::default();
        for at in (0..1_000_000).step_by(1000) {
            history.record(
                &sample(at, Some(if at % 20_000 < 15_000 { 90.0 } else { 0.0 })),
                &settings,
            );
        }
        assert!(history.samples.len() <= 600);
        assert!(history.events.len() <= 30);
        history.record(&sample(2_000_000, Some(0.0)), &settings);
        assert!(history.events.is_empty());
        assert_eq!(history.samples.len(), 1);
    }
}
