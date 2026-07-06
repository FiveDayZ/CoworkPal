use tauri::{AppHandle, Manager};

use crate::models::{AppSettings, MonitorBarMode};
use crate::{app_state::AppState, window_manager};

const TASKBAR_WINDOW_LABEL: &str = "taskbar-monitor";
const TASKBAR_COLUMN_WIDTH: i32 = 112;
const TASKBAR_HORIZONTAL_PADDING: i32 = 28;
const TASKBAR_MIN_WIDTH: i32 = 180;
const TASKBAR_MAX_WIDTH: i32 = 640;

pub async fn sync_taskbar_monitor(app: &AppHandle) {
    let settings = if let Some(state) = app.try_state::<AppState>() {
        Some(state.settings.read().await.clone())
    } else {
        None
    };
    let enabled = settings
        .as_ref()
        .map(|settings| settings.show_monitor_data_in_taskbar)
        .unwrap_or(false);

    if !enabled {
        if let Some(window) = app.get_webview_window(TASKBAR_WINDOW_LABEL) {
            if let Err(error) = window.hide() {
                tracing::warn!("failed to hide taskbar monitor: {error}");
            }
        }
        return;
    }

    let window = match window_manager::ensure_webview_window(app, TASKBAR_WINDOW_LABEL).await {
        Ok(window) => window,
        Err(error) => {
            tracing::warn!("{error}");
            return;
        }
    };

    let desired_width = settings
        .as_ref()
        .map(resolve_taskbar_width)
        .unwrap_or(TASKBAR_MIN_WIDTH);

    if let Err(error) = imp::embed_and_show(&window, desired_width) {
        tracing::warn!("failed to embed taskbar monitor: {error}");
        if let Err(show_error) = window.hide() {
            tracing::warn!("failed to hide unembedded taskbar monitor: {show_error}");
        }
    }
}

fn resolve_taskbar_width(settings: &AppSettings) -> i32 {
    let metric_count = match settings.taskbar_monitor_mode {
        MonitorBarMode::Micro => settings.visible_taskbar_metrics.len().min(2),
        MonitorBarMode::Default | MonitorBarMode::Expanded => {
            settings.visible_taskbar_metrics.len().min(3)
        }
    }
    .max(1) as i32;

    (TASKBAR_HORIZONTAL_PADDING + metric_count * TASKBAR_COLUMN_WIDTH)
        .clamp(TASKBAR_MIN_WIDTH, TASKBAR_MAX_WIDTH)
}

#[cfg(windows)]
mod imp {
    use super::{TASKBAR_MIN_WIDTH, TASKBAR_MAX_WIDTH};
    use tauri::WebviewWindow;
    use windows::{
        core::w,
        Win32::{
            Foundation::{HWND, RECT},
            UI::WindowsAndMessaging::{
                FindWindowExW, FindWindowW, GetClientRect, GetWindowLongPtrW, GetWindowRect,
                SetParent, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_STYLE,
                HWND_TOP, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_SHOWWINDOW,
                WS_CHILD, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
            },
        },
    };

    const TASKBAR_EDGE_PADDING: i32 = 2;
    const NOTIFICATION_AREA_GAP: i32 = 16;

    pub fn embed_and_show(window: &WebviewWindow, desired_width: i32) -> Result<(), String> {
        let child_hwnd = window
            .hwnd()
            .map(|hwnd| HWND(hwnd.0))
            .map_err(|error| format!("failed to get taskbar monitor HWND: {error}"))?;

        unsafe {
            let taskbar_hwnd = FindWindowW(w!("Shell_TrayWnd"), None)
                .map_err(|error| format!("failed to find Shell_TrayWnd: {error}"))?;
            if taskbar_hwnd.0.is_null() {
                return Err("Shell_TrayWnd not found".to_string());
            }

            let style = GetWindowLongPtrW(child_hwnd, GWL_STYLE);
            SetWindowLongPtrW(
                child_hwnd,
                GWL_STYLE,
                ((style as u32 & !WS_POPUP.0) | WS_CHILD.0 | WS_VISIBLE.0) as isize,
            );

            let ex_style = GetWindowLongPtrW(child_hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(
                child_hwnd,
                GWL_EXSTYLE,
                ((ex_style as u32 & !WS_EX_APPWINDOW.0) | WS_EX_TOOLWINDOW.0) as isize,
            );

            SetParent(child_hwnd, Some(taskbar_hwnd))
                .map_err(|error| format!("failed to parent taskbar monitor: {error}"))?;

            let (x, y, width, height) = resolve_taskbar_slot(taskbar_hwnd, desired_width);
            SetWindowPos(
                child_hwnd,
                Some(HWND_TOP),
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_SHOWWINDOW,
            )
            .map_err(|error| format!("failed to position taskbar monitor: {error}"))?;

            let _ = ShowWindow(child_hwnd, SW_SHOW);
        }

        Ok(())
    }

    unsafe fn resolve_taskbar_slot(taskbar_hwnd: HWND, desired_width: i32) -> (i32, i32, i32, i32) {
        let mut client = RECT::default();
        if GetClientRect(taskbar_hwnd, &mut client).is_err() {
            return (0, 0, desired_width.clamp(TASKBAR_MIN_WIDTH, TASKBAR_MAX_WIDTH), 36);
        }

        let taskbar_width = (client.right - client.left).max(TASKBAR_MIN_WIDTH);
        let taskbar_height = (client.bottom - client.top).max(24);
        let height = taskbar_height;
        let tray_left = find_notification_area_left(taskbar_hwnd).unwrap_or(taskbar_width);
        let available_width =
            (tray_left - NOTIFICATION_AREA_GAP - TASKBAR_EDGE_PADDING).max(TASKBAR_MIN_WIDTH);
        let width = desired_width
            .clamp(TASKBAR_MIN_WIDTH, TASKBAR_MAX_WIDTH)
            .min(available_width);
        let x = (tray_left - width - NOTIFICATION_AREA_GAP).max(TASKBAR_EDGE_PADDING);
        let y = 0;

        (x, y, width, height)
    }

    unsafe fn find_notification_area_left(taskbar_hwnd: HWND) -> Option<i32> {
        let tray = FindWindowExW(Some(taskbar_hwnd), None, w!("TrayNotifyWnd"), None).ok()?;
        if tray.0.is_null() {
            return None;
        }

        let mut taskbar_rect = RECT::default();
        let mut tray_rect = RECT::default();
        GetWindowRect(taskbar_hwnd, &mut taskbar_rect).ok()?;
        GetWindowRect(tray, &mut tray_rect).ok()?;

        Some((tray_rect.left - taskbar_rect.left).max(0))
    }

    #[allow(dead_code)]
    pub fn hide(window: &WebviewWindow) -> Result<(), String> {
        let child_hwnd = window
            .hwnd()
            .map(|hwnd| HWND(hwnd.0))
            .map_err(|error| format!("failed to get taskbar monitor HWND: {error}"))?;
        unsafe {
            let _ = ShowWindow(child_hwnd, SW_HIDE);
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    use tauri::WebviewWindow;

    pub fn embed_and_show(window: &WebviewWindow, _desired_width: i32) -> Result<(), String> {
        window
            .show()
            .map_err(|error| format!("failed to show taskbar monitor: {error}"))
    }
}
