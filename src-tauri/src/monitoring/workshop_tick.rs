use crate::models::{WorkshopOrder, WorkshopState};
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopTick {
    revision: u64,
    parts: f64,
    insight: f64,
    last_production_time: i64,
    total_online_seconds: u64,
    today_parts: f64,
    today_insight: f64,
    last_daily_reset_date: String,
    active_orders: Vec<WorkshopOrder>,
    last_order_refresh_date: String,
}

impl From<&WorkshopState> for WorkshopTick {
    fn from(value: &WorkshopState) -> Self {
        Self {
            revision: value.revision,
            parts: value.parts,
            insight: value.insight,
            last_production_time: value.last_production_time,
            total_online_seconds: value.total_online_seconds,
            today_parts: value.today_parts,
            today_insight: value.today_insight,
            last_daily_reset_date: value.last_daily_reset_date.clone(),
            active_orders: value.active_orders.clone(),
            last_order_refresh_date: value.last_order_refresh_date.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tick_never_transports_reward_ledger_or_completed_order_history() {
        let state = WorkshopState::default();
        let tick = serde_json::to_value(WorkshopTick::from(&state)).unwrap();
        assert_eq!(tick["parts"], state.parts);
        assert!(tick.get("rewardReceipts").is_none());
        assert!(tick.get("completedOrderIds").is_none());
        assert!(tick.get("activeOrders").is_some());
    }
}
