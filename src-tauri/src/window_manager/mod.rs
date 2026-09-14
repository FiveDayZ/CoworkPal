use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

use crate::{
    app_state::AppState,
    events::SETTINGS_UPDATED,
    models::{AppSettings, AppSettingsPatch},
};

pub fn handle_window_event(_window: &tauri::Window, _event: &WindowEvent) {
    // Windows close naturally and get destroyed, preventing app exit via ExitRequested in lib.rs.
}

pub async fn show_window(app: &AppHandle, label: &str, focus: bool) -> Result<(), String> {
    let window = ensure_webview_window(app, label).await?;

    window
        .show()
        .map_err(|error| format!("failed to show {label}: {error}"))?;

    if focus {
        window
            .set_focus()
            .map_err(|error| format!("failed to focus {label}: {error}"))?;
    }

    Ok(())
}

pub async fn hide_window(app: &AppHandle, label: &str) -> Result<(), String> {
    let Some(window) = app.get_webview_window(label) else {
        return if is_lazy_window(label) {
            Ok(())
        } else {
            Err(format!("window '{label}' not found"))
        };
    };

    match label {
        "main" | "pet" | "monitor-bar" | "pet-panel" => window
            .close()
            .map_err(|error| format!("failed to close {label}: {error}")),
        _ => window
            .hide()
            .map_err(|error| format!("failed to hide {label}: {error}")),
    }
}

pub async fn toggle_window(app: &AppHandle, label: &str) -> Result<bool, String> {
    let window = ensure_webview_window(app, label).await?;

    let visible = window
        .is_visible()
        .map_err(|error| format!("failed to read {label} visibility: {error}"))?;

    if visible {
        hide_window(app, label).await?;
        Ok(false)
    } else {
        window
            .show()
            .map_err(|error| format!("failed to show {label}: {error}"))?;
        Ok(true)
    }
}

pub async fn ensure_webview_window(app: &AppHandle, label: &str) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(label) {
        return Ok(window);
    }

    let spec =
        LazyWindowSpec::for_label(label).ok_or_else(|| format!("window '{label}' not found"))?;

    let mut builder = WebviewWindowBuilder::new(app, spec.label, WebviewUrl::App(spec.url.into()))
        .title(spec.title)
        .inner_size(spec.width, spec.height)
        .decorations(spec.decorations)
        .transparent(spec.transparent)
        .resizable(spec.resizable)
        .always_on_top(spec.always_on_top)
        .skip_taskbar(spec.skip_taskbar)
        .visible(false)
        .shadow(false);

    #[cfg(windows)]
    {
        // Tao's native file-drop handler calls OleInitialize (STA) when each window is built.
        // Some Windows drivers initialize the UI thread as MTA first, which makes lazy window
        // creation panic with RPC_E_CHANGED_MODE. CoworkPal uses its own pointer-based window
        // dragging and does not consume native file-drop events.
        builder = builder.drag_and_drop(false);

        if crate::monitoring::webview_software_rendering_recommended() {
            builder = builder.additional_browser_args("--disable-gpu");
        }
    }

    if label == "main" {
        builder = builder.center();
    }

    let window = builder
        .build()
        .map_err(|error| format!("failed to create {label}: {error}"))?;

    if let Some(state) = app.try_state::<AppState>() {
        let scale_factor = window.scale_factor().unwrap_or(1.0);
        let position = {
            let settings = state.settings.read().await;
            match label {
                "pet" => Some((settings.cat_window_x, settings.cat_window_y)),
                "monitor-bar" => Some((settings.monitor_bar_x, settings.monitor_bar_y)),
                "pet-panel" => {
                    let pet_height = 166.0 * scale_factor;
                    let panel_width = 300.0 * scale_factor;
                    let panel_height = 360.0 * scale_factor;
                    let overlap = 10.0 * scale_factor;
                    let x = settings.cat_window_x - panel_width + overlap;
                    let y = settings.cat_window_y - (panel_height - pet_height);
                    Some((x, y))
                }
                _ => None,
            }
        };

        if let Some((x, y)) = position {
            let _ = window.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
        }
    }

    Ok(window)
}

fn is_lazy_window(label: &str) -> bool {
    LazyWindowSpec::for_label(label).is_some()
}

struct LazyWindowSpec {
    label: &'static str,
    title: &'static str,
    url: &'static str,
    width: f64,
    height: f64,
    decorations: bool,
    transparent: bool,
    resizable: bool,
    always_on_top: bool,
    skip_taskbar: bool,
}

impl LazyWindowSpec {
    fn for_label(label: &str) -> Option<Self> {
        match label {
            "main" => Some(Self {
                label: "main",
                title: "CoworkPal",
                url: "/main",
                width: 640.0,
                height: 420.0,
                decorations: false,
                transparent: false,
                resizable: false,
                always_on_top: false,
                skip_taskbar: false,
            }),
            "pet" => Some(Self {
                label: "pet",
                title: "CoCat",
                url: "/pet",
                width: 196.0,
                height: 166.0,
                decorations: false,
                transparent: true,
                resizable: false,
                always_on_top: true,
                skip_taskbar: true,
            }),
            "monitor-bar" => Some(Self {
                label: "monitor-bar",
                title: "CoworkPal Monitor",
                url: "/monitor-bar",
                width: 480.0,
                height: 42.0,
                decorations: false,
                transparent: true,
                resizable: false,
                always_on_top: true,
                skip_taskbar: true,
            }),
            "pet-panel" => Some(Self {
                label: "pet-panel",
                title: "CoworkPal Pet Panel",
                url: "/pet-panel",
                width: 300.0,
                height: 520.0,
                decorations: false,
                transparent: true,
                resizable: false,
                always_on_top: true,
                skip_taskbar: true,
            }),
            "taskbar-monitor" => Some(Self {
                label: "taskbar-monitor",
                title: "CoworkPal Taskbar Monitor",
                url: "/taskbar-monitor",
                width: 520.0,
                height: 36.0,
                decorations: false,
                transparent: true,
                resizable: false,
                always_on_top: true,
                skip_taskbar: true,
            }),
            _ => None,
        }
    }
}

pub async fn save_window_position(
    app: &AppHandle,
    state: &AppState,
    label: &str,
    x: f64,
    y: f64,
) -> Result<AppSettings, String> {
    let settings = {
        let mut settings = state.settings.write().await;
        let mut layout = state.layout.write().await;

        match label {
            "pet" => {
                settings.apply_patch(AppSettingsPatch {
                    cat_window_x: Some(x),
                    cat_window_y: Some(y),
                    ..Default::default()
                });
                layout.cat_window_x = x;
                layout.cat_window_y = y;
            }
            "monitor-bar" => {
                settings.apply_patch(AppSettingsPatch {
                    monitor_bar_x: Some(x),
                    monitor_bar_y: Some(y),
                    ..Default::default()
                });
                layout.monitor_bar_x = x;
                layout.monitor_bar_y = y;
            }
            other => return Err(format!("unsupported window position label: {other}")),
        }

        state.storage.save_settings(&settings)?;
        state.storage.save_layout(&layout)?;
        settings.clone()
    };

    app.emit(SETTINGS_UPDATED, settings.clone())
        .map_err(|error| format!("failed to emit {SETTINGS_UPDATED}: {error}"))?;

    Ok(settings)
}

pub async fn apply_saved_window_positions(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = state.settings.read().await.clone();

    if let Some(window) = app.get_webview_window("pet") {
        if let Err(error) = window.set_position(PhysicalPosition::new(
            settings.cat_window_x.round() as i32,
            settings.cat_window_y.round() as i32,
        )) {
            tracing::warn!("failed to restore pet position: {error}");
        }
    }

    if let Some(window) = app.get_webview_window("monitor-bar") {
        if let Err(error) = window.set_position(PhysicalPosition::new(
            settings.monitor_bar_x.round() as i32,
            settings.monitor_bar_y.round() as i32,
        )) {
            tracing::warn!("failed to restore monitor-bar position: {error}");
        }
    }
}
