use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Emitter, Manager,
};

use crate::{
    app_state::AppState,
    events::{SETTINGS_UPDATED, UI_NAVIGATE_MAIN},
    models::{AppSettings, AppSettingsPatch},
    taskbar_embed,
    window_manager,
};

const TRAY_ID: &str = "coworkpal";
const MENU_OPEN_MAIN: &str = "open-main";
const MENU_TOGGLE_MINIMAL_MODE: &str = "toggle-minimal-mode";
const MENU_TOGGLE_CAT: &str = "toggle-cat";
const MENU_TOGGLE_MONITOR: &str = "toggle-monitor";
const MENU_TOGGLE_TASKBAR_MONITOR: &str = "toggle-taskbar-monitor";
const MENU_TOGGLE_PRODUCTION: &str = "toggle-production";
const MENU_RELEASE_MEMORY: &str = "release-memory";
const MENU_OPEN_SETTINGS: &str = "open-settings";
const MENU_OPEN_ABOUT: &str = "open-about";
const MENU_QUIT: &str = "quit";

/// Build the shared menu used both by the tray icon and by the taskbar
/// monitor's right-click context menu. Both use the same item ids, so the
/// single `on_menu_event` handler registered on the tray icon dispatches clicks
/// from either source (Tauri's tray menu handler receives *all* menu events,
/// including those from popup menus, per its documentation).
pub async fn build_shared_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let minimal_mode_enabled = if let Some(state) = app.try_state::<AppState>() {
        state.settings.read().await.minimal_mode_enabled
    } else {
        false
    };
    let open_main = MenuItem::with_id(app, MENU_OPEN_MAIN, "打开主界面", true, None::<&str>)?;
    let toggle_minimal_mode = CheckMenuItem::with_id(
        app,
        MENU_TOGGLE_MINIMAL_MODE,
        "极简模式",
        true,
        minimal_mode_enabled,
        None::<&str>,
    )?;
    let toggle_cat = MenuItem::with_id(
        app,
        MENU_TOGGLE_CAT,
        "显示/隐藏 CoCat",
        !minimal_mode_enabled,
        None::<&str>,
    )?;
    let toggle_monitor = MenuItem::with_id(
        app,
        MENU_TOGGLE_MONITOR,
        "显示/隐藏监控条",
        !minimal_mode_enabled,
        None::<&str>,
    )?;
    let toggle_taskbar_monitor = MenuItem::with_id(
        app,
        MENU_TOGGLE_TASKBAR_MONITOR,
        "显示/隐藏任务栏",
        !minimal_mode_enabled,
        None::<&str>,
    )?;
    let toggle_production = MenuItem::with_id(
        app,
        MENU_TOGGLE_PRODUCTION,
        "暂停/继续产出",
        true,
        None::<&str>,
    )?;
    let release_memory = MenuItem::with_id(
        app,
        MENU_RELEASE_MEMORY,
        "释放内存",
        true,
        None::<&str>,
    )?;
    let open_settings = MenuItem::with_id(app, MENU_OPEN_SETTINGS, "设置", true, None::<&str>)?;
    let open_about = MenuItem::with_id(app, MENU_OPEN_ABOUT, "关于", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)?;
    let separator_a = PredefinedMenuItem::separator(app)?;
    let separator_b = PredefinedMenuItem::separator(app)?;

    Menu::with_items(
        app,
        &[
            &open_main,
            &toggle_minimal_mode,
            &separator_a,
            &toggle_cat,
            &toggle_monitor,
            &toggle_taskbar_monitor,
            &toggle_production,
            &release_memory,
            &separator_b,
            &open_settings,
            &open_about,
            &quit,
        ],
    )
}

pub fn setup_tray(app: &App) -> tauri::Result<()> {
    let menu = tauri::async_runtime::block_on(build_shared_menu(app.handle()))?;

    let state = app.state::<AppState>();
    let theme_name = tauri::async_runtime::block_on(async {
        let settings = state.settings.read().await;
        settings.theme_name.clone()
    });

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("CoworkPal")
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            let should_show_main = matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            );

            if should_show_main {
                let app = tray.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = show_main_route(&app, "dashboard").await {
                        tracing::warn!("failed to open main window from tray icon: {error}");
                    }
                });
            }
        });

    builder = builder.icon(create_tray_icon(&theme_name));

    builder.build(app)?;
    Ok(())
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    handle_menu_action(app, event.id().as_ref());
}

pub(crate) fn handle_shared_menu_position(app: &AppHandle, position: usize) {
    let action = match position {
        0 => MENU_OPEN_MAIN,
        1 => MENU_TOGGLE_MINIMAL_MODE,
        3 => MENU_TOGGLE_CAT,
        4 => MENU_TOGGLE_MONITOR,
        5 => MENU_TOGGLE_TASKBAR_MONITOR,
        6 => MENU_TOGGLE_PRODUCTION,
        7 => MENU_RELEASE_MEMORY,
        9 => MENU_OPEN_SETTINGS,
        10 => MENU_OPEN_ABOUT,
        11 => MENU_QUIT,
        _ => return,
    };
    handle_menu_action(app, action);
}

fn handle_menu_action(app: &AppHandle, action: &str) {
    match action {
        MENU_OPEN_MAIN => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = show_main_route(&app, "dashboard").await {
                    tracing::warn!("failed to open main window from tray: {error}");
                }
            });
        }
        MENU_OPEN_SETTINGS => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = show_main_route(&app, "settings").await {
                    tracing::warn!("failed to open settings from tray: {error}");
                }
            });
        }
        MENU_OPEN_ABOUT => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = show_main_route(&app, "about").await {
                    tracing::warn!("failed to open about from tray: {error}");
                }
            });
        }
        MENU_TOGGLE_MINIMAL_MODE => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = toggle_minimal_mode(app).await {
                    tracing::warn!("failed to toggle minimal mode from tray: {error}");
                }
            });
        }
        MENU_TOGGLE_CAT => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = toggle_cat_visibility(app).await {
                    tracing::warn!("failed to toggle CoCat from tray: {error}");
                }
            });
        }
        MENU_TOGGLE_MONITOR => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = toggle_monitor_visibility(app).await {
                    tracing::warn!("failed to toggle monitor bar from tray: {error}");
                }
            });
        }
        MENU_TOGGLE_TASKBAR_MONITOR => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = toggle_taskbar_monitor_visibility(app).await {
                    tracing::warn!("failed to toggle taskbar monitor from tray: {error}");
                }
            });
        }
        MENU_TOGGLE_PRODUCTION => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = toggle_production(app).await {
                    tracing::warn!("failed to toggle production from tray: {error}");
                }
            });
        }
        MENU_RELEASE_MEMORY => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = release_memory_from_tray(app).await {
                    tracing::warn!("failed to release memory from tray: {error}");
                }
            });
        }
        MENU_QUIT => {
            crate::IS_EXITING.store(true, std::sync::atomic::Ordering::SeqCst);
            app.exit(0);
        }
        _ => {}
    }
}

async fn toggle_minimal_mode(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    {
        let mut settings = state.settings.write().await;
        let previous = settings.clone();
        settings.toggle_minimal_mode();
        if let Err(error) = state.storage.save_settings(&settings) {
            *settings = previous;
            return Err(format!("failed to save minimal mode: {error}"));
        }
    }

    crate::IS_EXITING.store(true, std::sync::atomic::Ordering::SeqCst);
    std::env::set_var("COWORKPAL_RESTART_PENDING", "1");
    app.restart()
}

async fn show_main_route(app: &AppHandle, route: &str) -> Result<(), String> {
    window_manager::show_window(app, "main", true).await?;
    app.emit(UI_NAVIGATE_MAIN, route)
        .map_err(|error| format!("failed to emit {UI_NAVIGATE_MAIN}: {error}"))
}

async fn toggle_cat_visibility(app: AppHandle) -> Result<(), String> {
    let is_cat_visible = window_manager::toggle_window(&app, "pet").await?;
    if !is_cat_visible {
        window_manager::hide_window(&app, "pet-panel").await?;
    }

    apply_settings_patch(
        &app,
        AppSettingsPatch {
            is_cat_visible: Some(is_cat_visible),
            ..Default::default()
        },
    )
    .await
    .map(|_| ())
}

async fn toggle_monitor_visibility(app: AppHandle) -> Result<(), String> {
    let is_monitor_bar_visible = window_manager::toggle_window(&app, "monitor-bar").await?;

    apply_settings_patch(
        &app,
        AppSettingsPatch {
            is_monitor_bar_visible: Some(is_monitor_bar_visible),
            ..Default::default()
        },
    )
    .await
    .map(|_| ())
}

async fn toggle_taskbar_monitor_visibility(app: AppHandle) -> Result<(), String> {
    let show_monitor_data_in_taskbar = {
        let state = app.state::<AppState>();
        let settings = state.settings.read().await;
        !settings.show_monitor_data_in_taskbar
    };

    apply_settings_patch(
        &app,
        AppSettingsPatch {
            show_monitor_data_in_taskbar: Some(show_monitor_data_in_taskbar),
            ..Default::default()
        },
    )
    .await?;

    taskbar_embed::sync_taskbar_monitor(&app).await;
    Ok(())
}

async fn toggle_production(app: AppHandle) -> Result<(), String> {
    let is_production_paused = {
        let state = app.state::<AppState>();
        let settings = state.settings.read().await;
        !settings.is_production_paused
    };

    apply_settings_patch(
        &app,
        AppSettingsPatch {
            is_production_paused: Some(is_production_paused),
            ..Default::default()
        },
    )
    .await
    .map(|_| ())
}

/// Tray "release memory" handler. Respects the `memory_release_enabled` master
/// switch — when off, the menu item is a no-op (a future iteration could also
/// hide/disable the item, but Tauri's static menu doesn't refresh easily).
async fn release_memory_from_tray(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let enabled = state.settings.read().await.memory_release_enabled;
    if !enabled {
        tracing::debug!("memory release disabled in settings — skipping tray action");
        return Ok(());
    }
    crate::commands::perform_release(&state, &app, crate::memory_release::ReleaseKind::ManualFull)
        .await
        .map(|_| ())
}

async fn apply_settings_patch(
    app: &AppHandle,
    patch: AppSettingsPatch,
) -> Result<AppSettings, String> {
    let state = app.state::<AppState>();
    let settings = {
        let mut settings = state.settings.write().await;
        settings.apply_patch(patch);
        state.storage.save_settings(&settings)?;
        settings.clone()
    };

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let icon = create_tray_icon(&settings.theme_name);
        let _ = tray.set_icon(Some(icon));
    }

    app.emit(SETTINGS_UPDATED, settings.clone())
        .map_err(|error| format!("failed to emit {SETTINGS_UPDATED}: {error}"))?;

    Ok(settings)
}

fn create_tray_icon(theme: &str) -> Image<'static> {
    let bytes = match theme {
        "cyber" => include_bytes!("../../../src/assets/icons/tray_icon_blue.png").as_slice(),
        "steampunk" => include_bytes!("../../../src/assets/icons/tray_icon_gold.png").as_slice(),
        _ => include_bytes!("../../../src/assets/icons/tray_icon_orange.png").as_slice(),
    };
    Image::from_bytes(bytes).expect("failed to load tray icon")
}
