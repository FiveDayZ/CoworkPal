use std::collections::BTreeMap;

use chrono::Local;
use serde::Serialize;

use crate::models::{
    AppSettings, HardwareSnapshot, ModuleUpgradeLevels, WorkshopOrder, WorkshopOrderKind,
    WorkshopState, WORKSHOP_STATE_SCHEMA_VERSION,
};

const BASE_PARTS_PER_MINUTE: f64 = 1.6;
const BASE_INSIGHT_PER_MINUTE: f64 = 0.13;
const ECONOMY_REFERENCE_PARTS_PER_HOUR: f64 = 115.0;
const ECONOMY_REFERENCE_INSIGHT_PER_HOUR: f64 = 6.8;
const MODULE_PARTS_UPGRADE_PARTS_WEIGHT: f64 = 1.15;
const MODULE_PARTS_UPGRADE_INSIGHT_WEIGHT: f64 = 0.75;
const MODULE_PROCESS_UPGRADE_PARTS_WEIGHT: f64 = 0.9;
const MODULE_PROCESS_UPGRADE_INSIGHT_WEIGHT: f64 = 1.2;
const RESOURCE_EPSILON: f64 = 0.001;
const AFFINITY_BONUS_PER_LEVEL: f64 = 0.005;
const AFFINITY_BONUS_MAX: f64 = 0.10;
pub const MAX_WORKSHOP_LEVEL: u32 = 100;
pub const MAX_MODULE_SUB_LEVEL: u32 = 100;
/// Cap a single tick's credited time. This MUST match the work-log's cap
/// (models.rs record_snapshot clamps to 60s) so the workshop's online-seconds
/// tracking and the work-log's active-seconds tracking stay consistent when the
/// sampling pump drops frames — otherwise the two would diverge over time.
const MAX_DELTA_SECONDS: i64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCost {
    pub parts: u64,
    pub insight: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleUpgradeCosts {
    pub parts: Option<ResourceCost>,
    pub process: Option<ResourceCost>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopUpgradeQuotes {
    pub workshop: Option<ResourceCost>,
    pub modules: BTreeMap<String, ModuleUpgradeCosts>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopProductionBreakdown {
    pub parts_per_minute: f64,
    pub insight_per_minute: f64,
    pub parts_activity: f64,
    pub insight_activity: f64,
    pub workshop_multiplier: f64,
    pub parts_module_multiplier: f64,
    pub insight_module_multiplier: f64,
    pub stability_multiplier: f64,
    pub focus_multiplier: f64,
    pub affinity_multiplier: f64,
    pub affinity_title: String,
    pub affinity_tier: String,
    pub next_affinity_level: Option<u32>,
    pub next_affinity_title: Option<String>,
}

pub fn upgrade_quotes(workshop: &WorkshopState) -> WorkshopUpgradeQuotes {
    let modules = ["cpu", "gpu", "ram", "network", "temperature", "disk"]
        .into_iter()
        .map(|module_key| {
            let levels = module_levels(workshop, module_key).expect("known workshop module");
            (
                module_key.to_string(),
                ModuleUpgradeCosts {
                    parts: module_upgrade_cost(levels.parts, "parts", workshop.workshop_level),
                    process: module_upgrade_cost(
                        levels.process,
                        "process",
                        workshop.workshop_level,
                    ),
                },
            )
        })
        .collect();

    WorkshopUpgradeQuotes {
        workshop: workshop_upgrade_cost(workshop.workshop_level),
        modules,
    }
}

pub fn apply_workshop_upgrade(workshop: &mut WorkshopState) -> Result<ResourceCost, String> {
    let cost = workshop_upgrade_cost(workshop.workshop_level)
        .ok_or_else(|| "工坊已达升级上限".to_string())?;
    deduct_resources(workshop, cost)?;
    workshop.workshop_level += 1;
    Ok(cost)
}

pub fn apply_module_upgrade(
    workshop: &mut WorkshopState,
    module_key: &str,
    track: &str,
) -> Result<ResourceCost, String> {
    let current_level = match track {
        "parts" => module_levels(workshop, module_key)?.parts,
        "process" => module_levels(workshop, module_key)?.process,
        _ => return Err(format!("unknown workshop upgrade track: {track}")),
    };
    let cost = module_upgrade_cost(current_level, track, workshop.workshop_level)
        .ok_or_else(|| "该模块已达升级上限".to_string())?;
    deduct_resources(workshop, cost)?;

    let levels = module_levels_mut(workshop, module_key)?;
    match track {
        "parts" => levels.parts += 1,
        "process" => levels.process += 1,
        _ => unreachable!(),
    }
    Ok(cost)
}

fn workshop_upgrade_cost(level: u32) -> Option<ResourceCost> {
    if level >= MAX_WORKSHOP_LEVEL {
        return None;
    }
    let safe_level = level.max(1) as f64;
    let target_hours = (0.85 + safe_level * 0.45 + safe_level.powf(1.55) * 0.42)
        * late_game_cost_multiplier(level.max(1));
    Some(ResourceCost {
        parts: (ECONOMY_REFERENCE_PARTS_PER_HOUR * target_hours).round() as u64,
        insight: (ECONOMY_REFERENCE_INSIGHT_PER_HOUR * target_hours).round() as u64,
    })
}

fn module_upgrade_cost(level: u32, track: &str, workshop_level: u32) -> Option<ResourceCost> {
    if level >= MAX_MODULE_SUB_LEVEL {
        return None;
    }
    let safe_level = level.max(1) as f64;
    let safe_workshop_level = workshop_level.max(1) as f64;
    let target_hours =
        (0.35 + safe_level * 0.28 + safe_level.powf(1.42) * 0.18 + safe_workshop_level * 0.06)
            * late_game_cost_multiplier(level.max(workshop_level).max(1));
    let (parts_weight, insight_weight) = match track {
        "parts" => (
            MODULE_PARTS_UPGRADE_PARTS_WEIGHT,
            MODULE_PARTS_UPGRADE_INSIGHT_WEIGHT,
        ),
        "process" => (
            MODULE_PROCESS_UPGRADE_PARTS_WEIGHT,
            MODULE_PROCESS_UPGRADE_INSIGHT_WEIGHT,
        ),
        _ => return None,
    };
    Some(ResourceCost {
        parts: (ECONOMY_REFERENCE_PARTS_PER_HOUR * target_hours * parts_weight).round() as u64,
        insight: (ECONOMY_REFERENCE_INSIGHT_PER_HOUR * target_hours * insight_weight)
            .round()
            .max(1.0) as u64,
    })
}

fn late_game_cost_multiplier(level: u32) -> f64 {
    if level <= 30 {
        1.0
    } else {
        1.0 + (level.saturating_sub(30) as f64 * 0.008).min(0.55)
    }
}

fn deduct_resources(workshop: &mut WorkshopState, cost: ResourceCost) -> Result<(), String> {
    let parts = cost.parts as f64;
    let insight = cost.insight as f64;
    if !workshop.parts.is_finite()
        || !workshop.insight.is_finite()
        || workshop.parts < 0.0
        || workshop.insight < 0.0
    {
        return Err("工坊资源数据无效".to_string());
    }
    if workshop.parts < parts - RESOURCE_EPSILON || workshop.insight < insight - RESOURCE_EPSILON {
        return Err(format!(
            "资源不足：需要 {} 零件和 {} 灵感",
            cost.parts, cost.insight
        ));
    }
    workshop.parts = (workshop.parts - parts).max(0.0);
    workshop.insight = (workshop.insight - insight).max(0.0);
    Ok(())
}

fn module_levels<'a>(
    workshop: &'a WorkshopState,
    module_key: &str,
) -> Result<&'a ModuleUpgradeLevels, String> {
    match module_key {
        "cpu" => Ok(&workshop.module_levels.cpu),
        "gpu" => Ok(&workshop.module_levels.gpu),
        "ram" => Ok(&workshop.module_levels.ram),
        "network" => Ok(&workshop.module_levels.network),
        "temperature" => Ok(&workshop.module_levels.temperature),
        "disk" => Ok(&workshop.module_levels.disk),
        _ => Err(format!("unknown workshop module: {module_key}")),
    }
}

fn module_levels_mut<'a>(
    workshop: &'a mut WorkshopState,
    module_key: &str,
) -> Result<&'a mut ModuleUpgradeLevels, String> {
    match module_key {
        "cpu" => Ok(&mut workshop.module_levels.cpu),
        "gpu" => Ok(&mut workshop.module_levels.gpu),
        "ram" => Ok(&mut workshop.module_levels.ram),
        "network" => Ok(&mut workshop.module_levels.network),
        "temperature" => Ok(&mut workshop.module_levels.temperature),
        "disk" => Ok(&mut workshop.module_levels.disk),
        _ => Err(format!("unknown workshop module: {module_key}")),
    }
}

pub struct ProductionService;

impl ProductionService {
    pub fn apply_tick(
        workshop: &mut WorkshopState,
        settings: &AppSettings,
        snapshot: &HardwareSnapshot,
        now_ms: i64,
        focus_multiplier: f64,
    ) -> bool {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let mut changed = ensure_daily_orders(workshop, snapshot, now_ms, &today);

        if workshop.last_daily_reset_date != today {
            workshop.today_parts = 0.0;
            workshop.today_insight = 0.0;
            workshop.last_daily_reset_date = today;
            changed = true;
        }

        let delta_ms = now_ms.saturating_sub(workshop.last_production_time);
        let delta_seconds = (delta_ms / 1000).clamp(0, MAX_DELTA_SECONDS);

        if delta_seconds == 0 {
            return changed;
        }

        workshop.last_production_time = now_ms;
        workshop.total_online_seconds = workshop
            .total_online_seconds
            .saturating_add(delta_seconds as u64);
        changed = true;

        if settings.is_production_paused {
            return changed;
        }

        let delta_minutes = delta_seconds as f64 / 60.0;
        let output = calculate_output(settings, snapshot, workshop, focus_multiplier);
        let parts_per_minute = output.parts_per_minute;
        let insight_per_minute = output.insight_per_minute;
        let parts_delta = parts_per_minute * delta_minutes;
        let insight_delta = insight_per_minute * delta_minutes;

        workshop.parts += parts_delta;
        workshop.insight += insight_delta;
        workshop.today_parts += parts_delta;
        workshop.today_insight += insight_delta;

        changed
    }
}

#[derive(Debug, Clone, Copy)]
struct ProductionOutput {
    parts_per_minute: f64,
    insight_per_minute: f64,
}

fn calculate_output(
    settings: &AppSettings,
    snapshot: &HardwareSnapshot,
    workshop: &WorkshopState,
    focus_multiplier: f64,
) -> ProductionOutput {
    let cpu = percent_score(snapshot.cpu_usage_percent);
    let gpu = percent_score(snapshot.gpu_usage_percent);
    let memory = percent_score(snapshot.memory_usage_percent);
    let gpu_memory = ratio_score(
        snapshot.gpu_memory_used_bytes,
        snapshot.gpu_memory_total_bytes,
    );
    let network = throughput_score(
        snapshot.network_download_bytes_per_second.unwrap_or(0.0) as f64
            + snapshot.network_upload_bytes_per_second.unwrap_or(0.0) as f64,
    );
    let disk = throughput_score(
        snapshot.disk_read_bytes_per_second.unwrap_or(0.0) as f64
            + snapshot.disk_write_bytes_per_second.unwrap_or(0.0) as f64,
    );

    let module_levels = &workshop.module_levels;
    let parts_activity = cpu * 0.40 + memory * 0.30 + gpu * 0.20 + gpu_memory * 0.10;
    let insight_activity = network * 0.60 + disk * 0.40;

    let workshop_bonus = 1.0 + workshop.workshop_level.saturating_sub(1) as f64 * 0.12;
    let parts_module_bonus = module_bonus(&[
        (&module_levels.cpu, 0.060, 0.035),
        (&module_levels.gpu, 0.052, 0.030),
        (&module_levels.ram, 0.055, 0.030),
    ]);
    let insight_module_bonus = module_bonus(&[
        (&module_levels.network, 0.052, 0.078),
        (&module_levels.disk, 0.046, 0.066),
    ]);
    let stability = stability_multiplier(settings, snapshot, workshop);
    let focus_multiplier = if focus_multiplier.is_finite() {
        focus_multiplier.clamp(1.0, 1.5)
    } else {
        1.0
    };
    let affinity_multiplier = affinity_multiplier(workshop.cat_affinity_level);

    let parts_per_minute = (BASE_PARTS_PER_MINUTE
        * (0.82 + parts_activity * 1.50)
        * workshop_bonus
        * parts_module_bonus
        * stability
        * focus_multiplier
        * affinity_multiplier)
        .clamp(0.0, 50000.0);
    let insight_per_minute = (BASE_INSIGHT_PER_MINUTE
        * (0.88 + insight_activity * 1.85)
        * workshop_bonus
        * insight_module_bonus
        * stability
        * focus_multiplier
        * affinity_multiplier)
        .clamp(0.0, 5000.0);

    ProductionOutput {
        parts_per_minute,
        insight_per_minute,
    }
}

pub fn production_breakdown(
    settings: &AppSettings,
    snapshot: &HardwareSnapshot,
    workshop: &WorkshopState,
    focus_multiplier: f64,
) -> WorkshopProductionBreakdown {
    let cpu = percent_score(snapshot.cpu_usage_percent);
    let gpu = percent_score(snapshot.gpu_usage_percent);
    let memory = percent_score(snapshot.memory_usage_percent);
    let gpu_memory = ratio_score(
        snapshot.gpu_memory_used_bytes,
        snapshot.gpu_memory_total_bytes,
    );
    let network = throughput_score(
        snapshot.network_download_bytes_per_second.unwrap_or(0.0) as f64
            + snapshot.network_upload_bytes_per_second.unwrap_or(0.0) as f64,
    );
    let disk = throughput_score(
        snapshot.disk_read_bytes_per_second.unwrap_or(0.0) as f64
            + snapshot.disk_write_bytes_per_second.unwrap_or(0.0) as f64,
    );
    let parts_activity = cpu * 0.40 + memory * 0.30 + gpu * 0.20 + gpu_memory * 0.10;
    let insight_activity = network * 0.60 + disk * 0.40;
    let workshop_multiplier = 1.0 + workshop.workshop_level.saturating_sub(1) as f64 * 0.12;
    let parts_module_multiplier = module_bonus(&[
        (&workshop.module_levels.cpu, 0.060, 0.035),
        (&workshop.module_levels.gpu, 0.052, 0.030),
        (&workshop.module_levels.ram, 0.055, 0.030),
    ]);
    let insight_module_multiplier = module_bonus(&[
        (&workshop.module_levels.network, 0.052, 0.078),
        (&workshop.module_levels.disk, 0.046, 0.066),
    ]);
    let stability_multiplier = stability_multiplier(settings, snapshot, workshop);
    let focus_multiplier = if focus_multiplier.is_finite() {
        focus_multiplier.clamp(1.0, 1.5)
    } else {
        1.0
    };
    let affinity_multiplier = affinity_multiplier(workshop.cat_affinity_level);
    let (affinity_title, affinity_tier, next_affinity_level, next_affinity_title) =
        affinity_profile(workshop.cat_affinity_level);
    let output = calculate_output(settings, snapshot, workshop, focus_multiplier);

    WorkshopProductionBreakdown {
        parts_per_minute: output.parts_per_minute,
        insight_per_minute: output.insight_per_minute,
        parts_activity,
        insight_activity,
        workshop_multiplier,
        parts_module_multiplier,
        insight_module_multiplier,
        stability_multiplier,
        focus_multiplier,
        affinity_multiplier,
        affinity_title: affinity_title.to_string(),
        affinity_tier: affinity_tier.to_string(),
        next_affinity_level,
        next_affinity_title: next_affinity_title.map(str::to_string),
    }
}

fn affinity_multiplier(level: u32) -> f64 {
    1.0 + (level.saturating_sub(1) as f64 * AFFINITY_BONUS_PER_LEVEL).min(AFFINITY_BONUS_MAX)
}

fn affinity_profile(
    level: u32,
) -> (
    &'static str,
    &'static str,
    Option<u32>,
    Option<&'static str>,
) {
    match level.max(1) {
        1..=4 => ("初识搭档", "new", Some(5), Some("默契搭档")),
        5..=9 => ("默契搭档", "trusted", Some(10), Some("可靠拍档")),
        10..=19 => ("可靠拍档", "partner", Some(20), Some("最佳拍档")),
        _ => ("最佳拍档", "bonded", None, None),
    }
}

pub fn ensure_daily_orders(
    workshop: &mut WorkshopState,
    snapshot: &HardwareSnapshot,
    now_ms: i64,
    today: &str,
) -> bool {
    let mut changed = false;
    if workshop.schema_version < WORKSHOP_STATE_SCHEMA_VERSION {
        workshop.schema_version = WORKSHOP_STATE_SCHEMA_VERSION;
        changed = true;
    }
    if workshop.last_order_refresh_date != today {
        workshop.active_orders.clear();
        workshop.last_order_refresh_date = today.to_string();
        changed = true;
    }
    let level = workshop.workshop_level.max(1) as u64;
    let expires_at = now_ms.saturating_add(24 * 60 * 60 * 1000);
    let event_title = if snapshot.cpu_temperature_celsius.unwrap_or(0.0) >= 75.0
        || snapshot.gpu_temperature_celsius.unwrap_or(0.0) >= 78.0
    {
        "散热告警检修"
    } else if snapshot.memory_usage_percent.unwrap_or(0.0) >= 75.0 {
        "内存压力疏导"
    } else if snapshot.cpu_usage_percent.unwrap_or(0.0) >= 70.0 {
        "高负载稳定测试"
    } else {
        "硬件巡检记录"
    };
    let candidates = [
        WorkshopOrder {
            id: format!("{today}:standard"),
            kind: WorkshopOrderKind::Standard,
            title: "基础校准单".to_string(),
            description: "交付常规零件与灵感，完成今日基础维护。".to_string(),
            required_parts: 55 + level * 8,
            required_insight: 4 + level / 3,
            reward_affinity: 18,
            created_at: now_ms,
            expires_at: None,
        },
        WorkshopOrder {
            id: format!("{today}:timed"),
            kind: WorkshopOrderKind::Timed,
            title: "限时加急工单".to_string(),
            description: "在有效期内完成加急装配，获得更多亲密度。".to_string(),
            required_parts: 90 + level * 12,
            required_insight: 7 + level / 2,
            reward_affinity: 32,
            created_at: now_ms,
            expires_at: Some(expires_at),
        },
        WorkshopOrder {
            id: format!("{today}:hardware"),
            kind: WorkshopOrderKind::HardwareEvent,
            title: event_title.to_string(),
            description: "根据当前硬件状态生成的事件工单。".to_string(),
            required_parts: 70 + level * 10,
            required_insight: 6 + level / 2,
            reward_affinity: 25,
            created_at: now_ms,
            expires_at: Some(expires_at),
        },
    ];
    for order in candidates {
        if !workshop.completed_order_ids.contains(&order.id)
            && workshop
                .active_orders
                .iter()
                .all(|current| current.id != order.id)
        {
            workshop.active_orders.push(order);
            changed = true;
        }
    }
    changed
}

pub fn complete_order(
    workshop: &mut WorkshopState,
    order_id: &str,
    now_ms: i64,
) -> Result<(), String> {
    let order = workshop
        .active_orders
        .iter()
        .find(|order| order.id == order_id)
        .cloned()
        .ok_or_else(|| "工单不存在或已经完成".to_string())?;
    if order
        .expires_at
        .is_some_and(|expires_at| now_ms > expires_at)
    {
        return Err("工单已过期".to_string());
    }
    deduct_resources(
        workshop,
        ResourceCost {
            parts: order.required_parts,
            insight: order.required_insight,
        },
    )?;
    workshop.affinity_experience = workshop
        .affinity_experience
        .saturating_add(order.reward_affinity);
    while workshop.affinity_experience >= 100 {
        workshop.affinity_experience -= 100;
        workshop.cat_affinity_level = workshop.cat_affinity_level.saturating_add(1);
    }
    workshop.completed_order_count = workshop.completed_order_count.saturating_add(1);
    workshop.completed_order_ids.insert(order.id.clone());
    workshop
        .active_orders
        .retain(|current| current.id != order_id);
    Ok(())
}

fn percent_score(value: Option<f32>) -> f64 {
    value.unwrap_or(0.0).clamp(0.0, 100.0) as f64 / 100.0
}

fn ratio_score(used: Option<u64>, total: Option<u64>) -> f64 {
    let Some(total) = total.filter(|value| *value > 0) else {
        return 0.0;
    };
    let used = used.unwrap_or(0);
    (used as f64 / total as f64).clamp(0.0, 1.0)
}

fn throughput_score(bytes_per_second: f64) -> f64 {
    let mib_per_second = (bytes_per_second.max(0.0)) / 1024.0 / 1024.0;
    (mib_per_second.ln_1p() / 64.0_f64.ln_1p()).clamp(0.0, 1.0)
}

fn module_bonus(modules: &[(&ModuleUpgradeLevels, f64, f64)]) -> f64 {
    modules
        .iter()
        .fold(1.0, |bonus, (module, parts_weight, process_weight)| {
            bonus
                + module.parts.max(1).saturating_sub(1) as f64 * parts_weight
                + module.process.max(1).saturating_sub(1) as f64 * process_weight
        })
        .clamp(1.0, 1000.0)
}

fn stability_multiplier(
    settings: &AppSettings,
    snapshot: &HardwareSnapshot,
    workshop: &WorkshopState,
) -> f64 {
    let memory = snapshot
        .memory_usage_percent
        .unwrap_or(0.0)
        .clamp(0.0, 100.0);
    let memory_over = if memory > settings.memory_crowded_threshold {
        (memory - settings.memory_crowded_threshold)
            / (100.0 - settings.memory_crowded_threshold).max(1.0)
    } else {
        0.0
    } as f64;

    let cpu_temp_over = temperature_overage(
        snapshot.cpu_temperature_celsius,
        settings.cpu_temperature_warning,
    );
    let gpu_temp_over = temperature_overage(
        snapshot.gpu_temperature_celsius,
        settings.gpu_temperature_warning,
    );
    let ram_relief = stability_relief(&workshop.module_levels.ram, 0.003, 0.005, 0.55);
    let cooling_relief = stability_relief(&workshop.module_levels.temperature, 0.003, 0.0055, 0.58);
    let memory_penalty_rate = 0.34 * (1.0 - ram_relief).max(0.45);
    let thermal_penalty_rate = 0.24 * (1.0 - cooling_relief).max(0.42);
    let memory_penalty = (memory_over * memory_penalty_rate).clamp(0.0, 0.30);
    let thermal_penalty = ((cpu_temp_over + gpu_temp_over) * thermal_penalty_rate).clamp(0.0, 0.42);

    (1.0 - memory_penalty - thermal_penalty).clamp(0.35, 1.08)
}

fn stability_relief(
    module: &ModuleUpgradeLevels,
    parts_weight: f64,
    process_weight: f64,
    max_relief: f64,
) -> f64 {
    (module.parts.max(1).saturating_sub(1) as f64 * parts_weight
        + module.process.max(1).saturating_sub(1) as f64 * process_weight)
        .clamp(0.0, max_relief)
}

fn temperature_overage(value: Option<f32>, warning: f32) -> f64 {
    value
        .filter(|temp| *temp > warning)
        .map(|temp| {
            // Degrees above the warning threshold needed to reach full thermal
            // penalty. 30°C is a fixed gradient (not user-configurable): a CPU at
            // warning+30 is running dangerously hot, so that span maps to [0,1].
            const FULL_PENALTY_SPAN_CELSIUS: f32 = 30.0;
            ((temp - warning) / FULL_PENALTY_SPAN_CELSIUS).clamp(0.0, 1.0) as f64
        })
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use crate::models::{
        current_timestamp_ms, AppSettings, HardwareSnapshot, ModuleUpgradeLevels,
        WorkshopModuleLevels, WorkshopState,
    };

    use super::{
        affinity_multiplier, affinity_profile, apply_module_upgrade, apply_workshop_upgrade,
        calculate_output, complete_order, ensure_daily_orders, production_breakdown,
        upgrade_quotes, ProductionService, ResourceCost, ECONOMY_REFERENCE_INSIGHT_PER_HOUR,
        ECONOMY_REFERENCE_PARTS_PER_HOUR,
    };

    #[test]
    fn upgrade_quotes_match_the_released_economy_curve() {
        let quotes = upgrade_quotes(&WorkshopState::default());

        assert_eq!(
            quotes.workshop,
            Some(ResourceCost {
                parts: 198,
                insight: 12,
            })
        );
        assert_eq!(
            quotes.modules["cpu"].parts,
            Some(ResourceCost {
                parts: 115,
                insight: 4,
            })
        );
        assert_eq!(
            quotes.modules["cpu"].process,
            Some(ResourceCost {
                parts: 90,
                insight: 7,
            })
        );
    }

    #[test]
    fn workshop_upgrade_debits_server_calculated_cost_atomically() {
        let mut workshop = WorkshopState::default();

        let cost = apply_workshop_upgrade(&mut workshop).unwrap();

        assert_eq!(cost.parts, 198);
        assert_eq!(cost.insight, 12);
        assert_eq!(workshop.workshop_level, 2);
        assert_close(workshop.parts, 82.0);
        assert_close(workshop.insight, 0.0);

        let parts_before = workshop.parts;
        assert!(apply_workshop_upgrade(&mut workshop).is_err());
        assert_eq!(workshop.workshop_level, 2);
        assert_close(workshop.parts, parts_before);
    }

    #[test]
    fn module_upgrade_validates_module_track_and_resources() {
        let mut workshop = WorkshopState {
            parts: 1_000.0,
            insight: 1_000.0,
            ..Default::default()
        };

        let cost = apply_module_upgrade(&mut workshop, "cpu", "parts").unwrap();

        assert_eq!(cost.parts, 115);
        assert_eq!(cost.insight, 4);
        assert_eq!(workshop.module_levels.cpu.parts, 2);
        assert_close(workshop.parts, 885.0);
        assert_close(workshop.insight, 996.0);

        let parts_before = workshop.parts;
        assert!(apply_module_upgrade(&mut workshop, "unknown", "parts").is_err());
        assert!(apply_module_upgrade(&mut workshop, "cpu", "unknown").is_err());
        assert_close(workshop.parts, parts_before);
    }

    #[test]
    fn upgrade_rejects_non_finite_resource_balances() {
        let mut workshop = WorkshopState {
            parts: f64::NAN,
            insight: 1_000.0,
            ..Default::default()
        };

        assert_eq!(
            apply_workshop_upgrade(&mut workshop).unwrap_err(),
            "工坊资源数据无效"
        );
        assert_eq!(workshop.workshop_level, 1);
        assert!(workshop.parts.is_nan());
    }

    #[test]
    fn reference_economy_simulation_reports_decade_milestones() {
        let mut workshop = WorkshopState {
            parts: 0.0,
            insight: 0.0,
            ..Default::default()
        };
        let mut milestones = Vec::new();

        for day in 1..=365 {
            workshop.parts += ECONOMY_REFERENCE_PARTS_PER_HOUR * 4.0;
            workshop.insight += ECONOMY_REFERENCE_INSIGHT_PER_HOUR * 4.0;
            while apply_workshop_upgrade(&mut workshop).is_ok() {
                if workshop.workshop_level.is_multiple_of(10) {
                    milestones.push((workshop.workshop_level, day));
                }
            }
        }

        assert_eq!(milestones, vec![(10, 20), (20, 106), (30, 286)]);
        assert_eq!(workshop.workshop_level, 33);
    }

    #[test]
    fn production_increases_parts_and_insight_when_not_paused() {
        let settings = AppSettings::default();
        let mut workshop = WorkshopState::default();
        workshop.parts = 0.0;
        workshop.insight = 0.0;
        let start = current_timestamp_ms();
        workshop.last_production_time = start;
        let snapshot = HardwareSnapshot {
            cpu_usage_percent: Some(50.0),
            gpu_usage_percent: Some(35.0),
            memory_usage_percent: Some(50.0),
            disk_read_bytes_per_second: Some(8.0 * 1024.0 * 1024.0),
            disk_write_bytes_per_second: Some(2.0 * 1024.0 * 1024.0),
            network_download_bytes_per_second: Some(3.0 * 1024.0 * 1024.0),
            network_upload_bytes_per_second: Some(512.0 * 1024.0),
            ..Default::default()
        };

        let changed =
            ProductionService::apply_tick(&mut workshop, &settings, &snapshot, start + 10_000, 1.0);

        assert!(changed);
        assert!(workshop.parts > 0.0);
        assert!(workshop.insight > 0.0);
        assert_eq!(workshop.total_online_seconds, 10);
    }

    #[test]
    fn daily_orders_are_deterministic_and_complete_atomically() {
        let mut workshop = WorkshopState {
            parts: 10_000.0,
            insight: 10_000.0,
            ..Default::default()
        };
        let snapshot = HardwareSnapshot {
            memory_usage_percent: Some(82.0),
            ..Default::default()
        };

        assert!(ensure_daily_orders(
            &mut workshop,
            &snapshot,
            1_000,
            "2026-09-14"
        ));
        assert_eq!(workshop.active_orders.len(), 3);
        assert!(!ensure_daily_orders(
            &mut workshop,
            &snapshot,
            2_000,
            "2026-09-14"
        ));
        let order = workshop.active_orders[0].clone();
        let parts_before = workshop.parts;
        complete_order(&mut workshop, &order.id, 2_000).unwrap();

        assert_close(workshop.parts, parts_before - order.required_parts as f64);
        assert_eq!(workshop.affinity_experience, order.reward_affinity);
        assert_eq!(workshop.completed_order_count, 1);
        assert_eq!(workshop.active_orders.len(), 2);
        assert!(complete_order(&mut workshop, &order.id, 2_000).is_err());
    }

    #[test]
    fn production_breakdown_exposes_the_same_final_rates() {
        let settings = AppSettings::default();
        let snapshot = balanced_snapshot();
        let workshop = WorkshopState::default();
        let expected = calculate_output(&settings, &snapshot, &workshop, 1.5);
        let breakdown = production_breakdown(&settings, &snapshot, &workshop, 1.5);

        assert_close(breakdown.parts_per_minute, expected.parts_per_minute);
        assert_close(breakdown.insight_per_minute, expected.insight_per_minute);
        assert_close(breakdown.focus_multiplier, 1.5);
        assert_close(breakdown.affinity_multiplier, 1.0);
        assert_eq!(breakdown.affinity_title, "初识搭档");
        assert_eq!(breakdown.next_affinity_level, Some(5));
    }

    #[test]
    fn affinity_levels_boost_output_with_a_ten_percent_cap() {
        let settings = AppSettings::default();
        let snapshot = balanced_snapshot();
        let base = calculate_output(&settings, &snapshot, &WorkshopState::default(), 1.0);
        let level_eleven = WorkshopState {
            cat_affinity_level: 11,
            ..Default::default()
        };
        let boosted = calculate_output(&settings, &snapshot, &level_eleven, 1.0);
        let capped = WorkshopState {
            cat_affinity_level: 99,
            ..Default::default()
        };
        let capped_output = calculate_output(&settings, &snapshot, &capped, 1.0);

        assert_close(affinity_multiplier(11), 1.05);
        assert_close(boosted.parts_per_minute / base.parts_per_minute, 1.05);
        assert_close(boosted.insight_per_minute / base.insight_per_minute, 1.05);
        assert_close(capped_output.parts_per_minute / base.parts_per_minute, 1.10);
        assert_eq!(affinity_profile(20), ("最佳拍档", "bonded", None, None));
    }

    #[test]
    fn order_affinity_experience_carries_across_level_up() {
        let mut workshop = WorkshopState {
            parts: 10_000.0,
            insight: 10_000.0,
            cat_affinity_level: 4,
            affinity_experience: 90,
            ..Default::default()
        };
        ensure_daily_orders(
            &mut workshop,
            &HardwareSnapshot::default(),
            1_000,
            "2026-09-14",
        );
        let order = workshop.active_orders[0].clone();

        complete_order(&mut workshop, &order.id, 2_000).unwrap();

        assert_eq!(workshop.cat_affinity_level, 5);
        assert_eq!(workshop.affinity_experience, 8);
    }

    #[test]
    fn active_focus_multiplier_boosts_both_resources() {
        let settings = AppSettings::default();
        let snapshot = balanced_snapshot();
        let start = current_timestamp_ms();
        let mut regular = WorkshopState {
            parts: 0.0,
            insight: 0.0,
            last_production_time: start,
            ..Default::default()
        };
        let mut focused = regular.clone();

        ProductionService::apply_tick(&mut regular, &settings, &snapshot, start + 60_000, 1.0);
        ProductionService::apply_tick(&mut focused, &settings, &snapshot, start + 60_000, 1.5);

        assert_close(focused.parts / regular.parts, 1.5);
        assert_close(focused.insight / regular.insight, 1.5);
    }

    #[test]
    fn all_monitor_signals_raise_output_against_idle_baseline() {
        let settings = AppSettings::default();
        let start = current_timestamp_ms();
        let mut idle = WorkshopState::default();
        idle.parts = 0.0;
        idle.insight = 0.0;
        idle.last_production_time = start;
        let mut active = idle.clone();
        let active_snapshot = HardwareSnapshot {
            cpu_usage_percent: Some(72.0),
            gpu_usage_percent: Some(58.0),
            memory_usage_percent: Some(64.0),
            cpu_temperature_celsius: Some(62.0),
            gpu_temperature_celsius: Some(66.0),
            disk_read_bytes_per_second: Some(22.0 * 1024.0 * 1024.0),
            disk_write_bytes_per_second: Some(7.0 * 1024.0 * 1024.0),
            network_download_bytes_per_second: Some(12.0 * 1024.0 * 1024.0),
            network_upload_bytes_per_second: Some(2.0 * 1024.0 * 1024.0),
            gpu_memory_used_bytes: Some(3 * 1024 * 1024 * 1024),
            gpu_memory_total_bytes: Some(8 * 1024 * 1024 * 1024),
            ..Default::default()
        };

        ProductionService::apply_tick(
            &mut idle,
            &settings,
            &HardwareSnapshot::default(),
            start + 10_000,
            1.0,
        );
        ProductionService::apply_tick(
            &mut active,
            &settings,
            &active_snapshot,
            start + 10_000,
            1.0,
        );

        assert!(active.parts > idle.parts);
        assert!(active.insight > idle.insight);
    }

    #[test]
    fn module_upgrades_increase_output() {
        let settings = AppSettings::default();
        let start = current_timestamp_ms();
        let snapshot = HardwareSnapshot {
            cpu_usage_percent: Some(55.0),
            gpu_usage_percent: Some(45.0),
            memory_usage_percent: Some(55.0),
            disk_read_bytes_per_second: Some(6.0 * 1024.0 * 1024.0),
            network_download_bytes_per_second: Some(4.0 * 1024.0 * 1024.0),
            ..Default::default()
        };
        let mut base = WorkshopState::default();
        base.parts = 0.0;
        base.insight = 0.0;
        base.last_production_time = start;
        let mut upgraded = base.clone();
        upgraded.module_levels = WorkshopModuleLevels {
            cpu: ModuleUpgradeLevels {
                parts: 4,
                process: 4,
            },
            gpu: ModuleUpgradeLevels {
                parts: 4,
                process: 4,
            },
            ram: ModuleUpgradeLevels {
                parts: 3,
                process: 3,
            },
            network: ModuleUpgradeLevels {
                parts: 3,
                process: 3,
            },
            temperature: ModuleUpgradeLevels {
                parts: 3,
                process: 3,
            },
            disk: ModuleUpgradeLevels {
                parts: 4,
                process: 4,
            },
        };

        ProductionService::apply_tick(&mut base, &settings, &snapshot, start + 10_000, 1.0);
        ProductionService::apply_tick(&mut upgraded, &settings, &snapshot, start + 10_000, 1.0);

        assert!(upgraded.parts > base.parts);
        assert!(upgraded.insight > base.insight);
    }

    #[test]
    fn workshop_level_bonus_applies_to_parts_and_insight_output() {
        let settings = AppSettings::default();
        let snapshot = balanced_snapshot();
        let mut base = WorkshopState::default();
        base.workshop_level = 1;
        let mut upgraded = base.clone();
        upgraded.workshop_level = 6;

        let base_output = calculate_output(&settings, &snapshot, &base, 1.0);
        let upgraded_output = calculate_output(&settings, &snapshot, &upgraded, 1.0);

        assert_close(
            upgraded_output.parts_per_minute / base_output.parts_per_minute,
            1.60,
        );
        assert_close(
            upgraded_output.insight_per_minute / base_output.insight_per_minute,
            1.60,
        );
    }

    #[test]
    fn production_module_bonus_tracks_apply_to_expected_resources() {
        let settings = AppSettings::default();
        let snapshot = balanced_snapshot();
        assert_parts_module_track_effect(&settings, &snapshot, |levels| &mut levels.cpu.parts);
        assert_parts_module_track_effect(&settings, &snapshot, |levels| &mut levels.cpu.process);
        assert_parts_module_track_effect(&settings, &snapshot, |levels| &mut levels.gpu.parts);
        assert_parts_module_track_effect(&settings, &snapshot, |levels| &mut levels.gpu.process);
        assert_parts_module_track_effect(&settings, &snapshot, |levels| &mut levels.ram.parts);
        assert_parts_module_track_effect(&settings, &snapshot, |levels| &mut levels.ram.process);
        assert_insight_module_track_effect(&settings, &snapshot, |levels| {
            &mut levels.network.parts
        });
        assert_insight_module_track_effect(&settings, &snapshot, |levels| {
            &mut levels.network.process
        });
        assert_insight_module_track_effect(&settings, &snapshot, |levels| &mut levels.disk.parts);
        assert_insight_module_track_effect(&settings, &snapshot, |levels| &mut levels.disk.process);
    }

    #[test]
    fn stability_module_bonus_tracks_apply_to_penalty_scenarios() {
        let mut settings = AppSettings::default();
        settings.memory_crowded_threshold = 70.0;
        settings.cpu_temperature_warning = 70.0;
        settings.gpu_temperature_warning = 70.0;
        let snapshot = HardwareSnapshot {
            cpu_usage_percent: Some(60.0),
            gpu_usage_percent: Some(40.0),
            memory_usage_percent: Some(92.0),
            cpu_temperature_celsius: Some(94.0),
            gpu_temperature_celsius: Some(92.0),
            disk_read_bytes_per_second: Some(8.0 * 1024.0 * 1024.0),
            network_download_bytes_per_second: Some(6.0 * 1024.0 * 1024.0),
            ..Default::default()
        };

        assert_stability_module_track_effect(&settings, &snapshot, |levels| &mut levels.ram.parts);
        assert_stability_module_track_effect(&settings, &snapshot, |levels| {
            &mut levels.ram.process
        });
        assert_stability_module_track_effect(&settings, &snapshot, |levels| {
            &mut levels.temperature.parts
        });
        assert_stability_module_track_effect(&settings, &snapshot, |levels| {
            &mut levels.temperature.process
        });
    }

    #[test]
    fn temperature_module_tracks_reduce_thermal_penalty() {
        let mut settings = AppSettings::default();
        settings.cpu_temperature_warning = 70.0;
        settings.gpu_temperature_warning = 70.0;
        let start = current_timestamp_ms();
        let snapshot = HardwareSnapshot {
            cpu_usage_percent: Some(60.0),
            gpu_usage_percent: Some(40.0),
            memory_usage_percent: Some(58.0),
            cpu_temperature_celsius: Some(94.0),
            gpu_temperature_celsius: Some(92.0),
            disk_read_bytes_per_second: Some(8.0 * 1024.0 * 1024.0),
            network_download_bytes_per_second: Some(6.0 * 1024.0 * 1024.0),
            ..Default::default()
        };
        let mut base = WorkshopState::default();
        base.parts = 0.0;
        base.insight = 0.0;
        base.last_production_time = start;
        let mut upgraded = base.clone();
        upgraded.module_levels.temperature = ModuleUpgradeLevels {
            parts: 20,
            process: 20,
        };

        ProductionService::apply_tick(&mut base, &settings, &snapshot, start + 10_000, 1.0);
        ProductionService::apply_tick(&mut upgraded, &settings, &snapshot, start + 10_000, 1.0);

        assert!(upgraded.parts > base.parts);
        assert!(upgraded.insight > base.insight);
    }

    #[test]
    fn paused_production_updates_time_but_not_resources() {
        let mut settings = AppSettings::default();
        settings.is_production_paused = true;
        let mut workshop = WorkshopState::default();
        let starting_parts = workshop.parts;
        let starting_insight = workshop.insight;
        let start = current_timestamp_ms();
        workshop.last_production_time = start;

        let changed = ProductionService::apply_tick(
            &mut workshop,
            &settings,
            &HardwareSnapshot::default(),
            start + 5_000,
            1.0,
        );

        assert!(changed);
        assert_eq!(workshop.parts, starting_parts);
        assert_eq!(workshop.insight, starting_insight);
        assert_eq!(workshop.total_online_seconds, 5);
    }

    fn balanced_snapshot() -> HardwareSnapshot {
        HardwareSnapshot {
            cpu_usage_percent: Some(55.0),
            gpu_usage_percent: Some(45.0),
            memory_usage_percent: Some(58.0),
            cpu_temperature_celsius: Some(60.0),
            gpu_temperature_celsius: Some(62.0),
            disk_read_bytes_per_second: Some(6.0 * 1024.0 * 1024.0),
            disk_write_bytes_per_second: Some(2.0 * 1024.0 * 1024.0),
            network_download_bytes_per_second: Some(4.0 * 1024.0 * 1024.0),
            network_upload_bytes_per_second: Some(1.0 * 1024.0 * 1024.0),
            gpu_memory_used_bytes: Some(2 * 1024 * 1024 * 1024),
            gpu_memory_total_bytes: Some(8 * 1024 * 1024 * 1024),
            ..Default::default()
        }
    }

    fn assert_parts_module_track_effect(
        settings: &AppSettings,
        snapshot: &HardwareSnapshot,
        upgrade_track: fn(&mut WorkshopModuleLevels) -> &mut u32,
    ) {
        let base = WorkshopState::default();
        let mut upgraded = base.clone();
        *upgrade_track(&mut upgraded.module_levels) = 2;

        let base_output = calculate_output(settings, snapshot, &base, 1.0);
        let upgraded_output = calculate_output(settings, snapshot, &upgraded, 1.0);

        assert!(upgraded_output.parts_per_minute > base_output.parts_per_minute);
        assert_close(
            upgraded_output.insight_per_minute / base_output.insight_per_minute,
            1.0,
        );
    }

    fn assert_insight_module_track_effect(
        settings: &AppSettings,
        snapshot: &HardwareSnapshot,
        upgrade_track: fn(&mut WorkshopModuleLevels) -> &mut u32,
    ) {
        let base = WorkshopState::default();
        let mut upgraded = base.clone();
        *upgrade_track(&mut upgraded.module_levels) = 2;

        let base_output = calculate_output(settings, snapshot, &base, 1.0);
        let upgraded_output = calculate_output(settings, snapshot, &upgraded, 1.0);

        assert_close(
            upgraded_output.parts_per_minute / base_output.parts_per_minute,
            1.0,
        );
        assert!(upgraded_output.insight_per_minute > base_output.insight_per_minute);
    }

    fn assert_stability_module_track_effect(
        settings: &AppSettings,
        snapshot: &HardwareSnapshot,
        upgrade_track: fn(&mut WorkshopModuleLevels) -> &mut u32,
    ) {
        let base = WorkshopState::default();
        let mut upgraded = base.clone();
        *upgrade_track(&mut upgraded.module_levels) = 2;

        let base_output = calculate_output(settings, snapshot, &base, 1.0);
        let upgraded_output = calculate_output(settings, snapshot, &upgraded, 1.0);

        assert!(upgraded_output.parts_per_minute > base_output.parts_per_minute);
        assert!(upgraded_output.insight_per_minute > base_output.insight_per_minute);
    }

    fn assert_close(actual: f64, expected: f64) {
        let delta = (actual - expected).abs();
        assert!(
            delta < 0.000_001,
            "expected {actual} to be close to {expected}, delta {delta}"
        );
    }
}
