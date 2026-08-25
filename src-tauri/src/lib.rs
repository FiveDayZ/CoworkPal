mod app_state;
pub mod achievements;
mod cloud_sync;
mod commands;
pub mod events;
mod input_activity;
pub mod memory_release;
mod models;
mod monitoring;
mod pet;
mod process_util;
mod storage;
mod suggestions;
mod taskbar_embed;
mod tray;
mod window_manager;
mod workshop;

use app_state::AppState;
use input_activity::start_input_activity_pump;
use monitoring::start_hardware_snapshot_pump;
use tauri::Manager;

pub static IS_EXITING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn run() {
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState::load().map_err(std::io::Error::other)?;
            app.manage(state);
            tray::setup_tray(app)?;
            let corruption_rebuilds = app
                .state::<AppState>()
                .storage
                .take_corruption_rebuilds();
            let launch_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                for (index, file_name) in corruption_rebuilds.into_iter().enumerate() {
                    let now = models::current_timestamp_ms();
                    if let Err(error) = commands::record_internal_achievement_event(
                        &launch_handle,
                        "storage.corruption_rebuilt",
                        format!("storage.corruption_rebuilt:{file_name}:{now}:{index}"),
                        serde_json::json!({
                            "fileName": file_name,
                        }),
                    )
                    .await
                    {
                        tracing::warn!(
                            "failed to record storage.corruption_rebuilt achievement event: {error}"
                        );
                    }
                }
                let now = models::current_timestamp_ms();
                if let Err(error) = commands::record_internal_achievement_event(
                    &launch_handle,
                    "app.launch",
                    format!("app.launch:{now}"),
                    serde_json::json!({
                        "appVersion": env!("CARGO_PKG_VERSION"),
                    }),
                )
                .await
                {
                    tracing::warn!("failed to record app.launch achievement event: {error}");
                }
            });
            start_hardware_snapshot_pump(app.handle().clone());
            start_input_activity_pump(app.handle().clone());
            start_memory_auto_release_pump(app.handle().clone());
            cloud_sync::start_token_request_polling(app.handle().clone());
            cloud_sync::start_auto_backup_polling(app.handle().clone());
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                window_manager::apply_saved_window_positions(&app_handle).await;
                apply_saved_visibility(&app_handle).await;
                loop {
                    taskbar_embed::sync_taskbar_monitor(&app_handle).await;
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            });
            Ok(())
        })
        .on_window_event(window_manager::handle_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::track_achievement_event,
            commands::get_achievement_summary,
            commands::list_achievements,
            commands::get_achievement_detail,
            commands::mark_achievement_notifications_seen,
            commands::get_hardware_snapshot,
            commands::get_app_settings,
            commands::update_app_settings,
            commands::get_workshop_state,
            commands::reward_cocat_interaction,
            commands::get_work_log_report,
            commands::get_daily_work_assessment,
            commands::get_daily_work_assessment_history,
            commands::get_daily_work_assessment_trend,
            commands::toggle_production_paused,
            commands::show_main_window,
            commands::show_main_route,
            commands::hide_main_window,
            commands::show_pet_window,
            commands::hide_pet_window,
            commands::toggle_monitor_bar,
            commands::show_monitor_bar,
            commands::hide_monitor_bar,
            commands::show_pet_panel,
            commands::hide_pet_panel,
            commands::toggle_pet_panel,
            commands::save_window_position,
            commands::exit_app,
            commands::update_workshop_state,
            commands::start_focus_session,
            commands::complete_focus_session,
            commands::abandon_focus_session,
            commands::get_notes,
            commands::create_note,
            commands::update_note,
            commands::toggle_note_pinned,
            commands::toggle_note_archived,
            commands::delete_note,
            commands::export_note,
            commands::import_note,
            commands::get_rhythm_profile,
            commands::get_health_trend,
            commands::get_today_suggestions,
            commands::get_memory_status,
            commands::trigger_memory_release,
            cloud_sync::get_sync_config,
            cloud_sync::update_sync_config,
            cloud_sync::request_access_token,
            cloud_sync::check_access_token_request,
            cloud_sync::upload_user_data,
            cloud_sync::download_user_data,
            commands::show_taskbar_context_menu,
            commands::updater::check_update,
            commands::updater::download_update,
            commands::updater::install_update
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if !IS_EXITING.load(std::sync::atomic::Ordering::SeqCst) {
                    api.prevent_exit();
                }
            }
        });
}

async fn apply_saved_visibility(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let settings = state.settings.read().await.clone();

    let pet_result = if settings.is_cat_visible {
        window_manager::show_window(app, "pet", false).await
    } else {
        window_manager::hide_window(app, "pet").await
    };
    if let Err(error) = pet_result {
        tracing::warn!("failed to apply pet visibility: {error}");
    }

    let monitor_result = if settings.is_monitor_bar_visible {
        window_manager::show_window(app, "monitor-bar", false).await
    } else {
        window_manager::hide_window(app, "monitor-bar").await
    };
    if let Err(error) = monitor_result {
        tracing::warn!("failed to apply monitor-bar visibility: {error}");
    }

    taskbar_embed::sync_taskbar_monitor(app).await;
}

/// Auto-release watcher. Every 10s it samples system memory pressure and, when
/// both `memory_release_enabled` and `memory_auto_release_enabled` are on and
/// used memory exceeds the configured GiB threshold, runs the light tier —
/// but only if the 60s cooldown (held in `AppState.memory_release`) has
/// elapsed. The light tier never elevates, so this loop never disturbs the
/// user with a UAC prompt.
fn start_memory_auto_release_pump(app: tauri::AppHandle) {
    use commands::perform_release;
    use memory_release::{sysinfo_query, ReleaseKind};

    tauri::async_runtime::spawn(async move {
        // A 10s cadence balances responsiveness against overhead — sampling is
        // a single `GlobalMemoryStatusEx` call, well under a millisecond.
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            if IS_EXITING.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }

            let Some(state) = app.try_state::<AppState>() else {
                continue;
            };

            let (enabled, auto_enabled, threshold_gib) = {
                let settings = state.settings.read().await;
                (
                    settings.memory_release_enabled,
                    settings.memory_auto_release_enabled,
                    settings.memory_auto_release_threshold_gib,
                )
            };
            if !enabled || !auto_enabled {
                continue;
            }

            let Some(pressure) = sysinfo_query::sample_pressure() else {
                continue;
            };
            if pressure.used_gib() < threshold_gib {
                continue;
            }

            // Cooldown check — only one auto release per 60s window.
            if state.memory_release.try_acquire().await.is_none() {
                continue;
            }

            tracing::info!(
                "auto memory release triggered: used {:.2} GiB >= threshold {:.2} GiB",
                pressure.used_gib(),
                threshold_gib
            );
            if let Err(error) = perform_release(&state, &app, ReleaseKind::AutoLight).await {
                tracing::warn!("auto memory release failed: {error}");
            }
        }
    });
}
