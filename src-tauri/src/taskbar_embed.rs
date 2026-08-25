use tauri::{AppHandle, Manager};

use crate::app_state::AppState;
use crate::models::{AppSettings, HardwareSnapshot, MonitorBarMode, MonitorMetric};
#[cfg(not(windows))]
use crate::window_manager;

const TASKBAR_WINDOW_LABEL: &str = "taskbar-monitor";
const DEFAULT_DPI: u32 = 96;
const TASKBAR_COLUMN_WIDTH: i32 = 64;
const TASKBAR_HORIZONTAL_PADDING: i32 = 4;
const TASKBAR_MIN_WIDTH: i32 = 90;
const TASKBAR_MAX_WIDTH: i32 = 320;
const TASKBAR_FONT_POINT_SIZE: i32 = 9;
const TASKBAR_ROW_HEIGHT: i32 = 16;

pub async fn sync_taskbar_monitor(app: &AppHandle) {
    let (settings, snapshot) = if let Some(state) = app.try_state::<AppState>() {
        let settings = state.settings.read().await.clone();
        let snapshot = state.last_snapshot.read().await.clone();
        (Some(settings), snapshot)
    } else {
        (None, None)
    };

    let enabled = settings
        .as_ref()
        .map(|settings| settings.show_monitor_data_in_taskbar)
        .unwrap_or(false);

    if !enabled {
        #[cfg(windows)]
        if let Err(error) = imp::hide_native(app.clone()) {
            tracing::warn!("failed to schedule native taskbar monitor hide: {error}");
        }

        if let Some(window) = app.get_webview_window(TASKBAR_WINDOW_LABEL) {
            if let Err(error) = window.close() {
                tracing::warn!("failed to close taskbar monitor: {error}");
            }
        }
        return;
    }

    #[cfg(windows)]
    {
        if let Some(window) = app.get_webview_window(TASKBAR_WINDOW_LABEL) {
            if let Err(error) = window.close() {
                tracing::warn!("failed to close legacy taskbar monitor window: {error}");
            }
        }

        let Some(settings) = settings.as_ref() else {
            return;
        };
        let snapshot = snapshot.unwrap_or_default();
        let cells = resolve_taskbar_cells(settings, &snapshot);
        let desired_width = resolve_taskbar_width(settings);

        if let Err(error) = imp::embed_and_show_native(app.clone(), cells, desired_width) {
            tracing::warn!("failed to schedule native taskbar monitor: {error}");
        }
        return;
    }

    #[cfg(not(windows))]
    {
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
}

fn resolve_taskbar_width(settings: &AppSettings) -> i32 {
    let metric_count = displayed_taskbar_metrics(settings).len().max(1) as i32;

    (TASKBAR_HORIZONTAL_PADDING + metric_count * TASKBAR_COLUMN_WIDTH)
        .clamp(TASKBAR_MIN_WIDTH, TASKBAR_MAX_WIDTH)
}

fn scale_for_dpi(value: i32, dpi: u32) -> i32 {
    let dpi = if dpi == 0 { DEFAULT_DPI } else { dpi };
    ((i64::from(value) * i64::from(dpi) + i64::from(DEFAULT_DPI / 2)) / i64::from(DEFAULT_DPI))
        as i32
}

fn taskbar_font_height(dpi: u32) -> i32 {
    let dpi = if dpi == 0 { DEFAULT_DPI } else { dpi };
    -((i64::from(TASKBAR_FONT_POINT_SIZE) * i64::from(dpi) + 36) / 72) as i32
}

fn taskbar_row_bounds(top: i32, bottom: i32, dpi: u32) -> (i32, i32, i32) {
    let height = (bottom - top).max(2);
    let row_height = scale_for_dpi(TASKBAR_ROW_HEIGHT, dpi)
        .max(1)
        .min(height / 2);
    let rows_top = top + (height - row_height * 2) / 2;

    (rows_top, rows_top + row_height, rows_top + row_height * 2)
}

fn displayed_taskbar_metrics(settings: &AppSettings) -> Vec<MonitorMetric> {
    let limit = match settings.taskbar_monitor_mode {
        MonitorBarMode::Micro => 2,
        MonitorBarMode::Default | MonitorBarMode::Expanded => 3,
    };

    settings
        .visible_taskbar_metrics
        .iter()
        .take(limit)
        .cloned()
        .collect()
}

fn resolve_taskbar_cells(
    settings: &AppSettings,
    snapshot: &HardwareSnapshot,
) -> Vec<(String, String)> {
    let cells: Vec<(String, String)> = displayed_taskbar_metrics(settings)
        .iter()
        .map(|metric| get_taskbar_metric_cell(metric, snapshot))
        .collect();

    if cells.is_empty() {
        vec![("未选择".to_string(), String::new())]
    } else {
        cells
    }
}

fn get_taskbar_metric_cell(
    metric: &MonitorMetric,
    snapshot: &HardwareSnapshot,
) -> (String, String) {
    match metric {
        MonitorMetric::Cpu => (
            format!("CPU {}", format_percent(snapshot.cpu_usage_percent)),
            format_temperature(snapshot.cpu_temperature_celsius),
        ),
        MonitorMetric::Gpu => (
            format!("GPU {}", format_percent(snapshot.gpu_usage_percent)),
            format_temperature(snapshot.gpu_temperature_celsius),
        ),
        MonitorMetric::Ram => (
            format!("RAM {}", format_percent(snapshot.memory_usage_percent)),
            format_compact_bytes_u64(snapshot.used_memory_bytes),
        ),
        MonitorMetric::Disk => (
            format!(
                "DISK {}",
                format_compact_bytes_f32(snapshot.disk_read_bytes_per_second)
            ),
            format!(
                "W {}",
                format_compact_bytes_f32(snapshot.disk_write_bytes_per_second)
            ),
        ),
        MonitorMetric::Network => (
            format!(
                "NET {}",
                format_compact_bytes_f32(snapshot.network_download_bytes_per_second)
            ),
            format!(
                "U {}",
                format_compact_bytes_f32(snapshot.network_upload_bytes_per_second)
            ),
        ),
    }
}

fn format_percent(value: Option<f32>) -> String {
    value
        .map(|value| format!("{}%", value.clamp(0.0, 100.0).round() as i32))
        .unwrap_or_else(|| "--%".to_string())
}

fn format_temperature(value: Option<f32>) -> String {
    value
        .map(|value| format!("{}°C", value.round() as i32))
        .unwrap_or_else(|| "--°C".to_string())
}

fn format_compact_bytes_f32(value: Option<f32>) -> String {
    value
        .map(|value| format_compact_bytes_value(value.max(0.0) as f64))
        .unwrap_or_else(|| "--".to_string())
}

fn format_compact_bytes_u64(value: Option<u64>) -> String {
    value
        .map(|value| format_compact_bytes_value(value as f64))
        .unwrap_or_else(|| "--".to_string())
}

fn format_compact_bytes_value(mut size: f64) -> String {
    const UNITS: [&str; 4] = ["B", "K", "M", "G"];
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    let digits = if unit_index == 0 || size >= 10.0 {
        0
    } else {
        1
    };
    format!("{size:.digits$}{}", UNITS[unit_index])
}

#[cfg(windows)]
async fn show_native_taskbar_context_menu(app: AppHandle, owner_hwnd: isize) -> Result<(), String> {
    use std::ffi::c_void;

    use tauri::menu::ContextMenu;
    use windows::Win32::{
        Foundation::{HWND, POINT},
        UI::WindowsAndMessaging::{
            GetCursorPos, GetMenuItemID, SetForegroundWindow, TrackPopupMenuEx, HMENU,
            TPM_LEFTALIGN, TPM_RETURNCMD,
        },
    };

    let menu = crate::tray::build_shared_menu(&app)
        .await
        .map_err(|error| format!("failed to build taskbar context menu: {error}"))?;
    let popup_handle = menu
        .hpopupmenu()
        .map_err(|error| format!("failed to resolve taskbar menu handle: {error}"))?;
    let dispatch_app = app.clone();

    app.run_on_main_thread(move || unsafe {
        let owner = HWND(owner_hwnd as *mut c_void);
        let popup = HMENU(popup_handle as *mut c_void);
        let mut cursor = POINT::default();
        if GetCursorPos(&mut cursor).is_err() {
            return;
        }

        let _ = SetForegroundWindow(owner);
        let selected = TrackPopupMenuEx(
            popup,
            (TPM_LEFTALIGN | TPM_RETURNCMD).0,
            cursor.x,
            cursor.y,
            owner,
            None,
        )
        .0 as u32;
        if selected == 0 {
            return;
        }

        for position in 0..12 {
            if GetMenuItemID(popup, position) == selected {
                crate::tray::handle_shared_menu_position(&dispatch_app, position as usize);
                break;
            }
        }
    })
    .map_err(|error| format!("failed to schedule taskbar context menu: {error}"))?;

    Ok(())
}

#[cfg(windows)]
mod imp {
    use std::{
        ffi::c_void,
        sync::{Mutex, OnceLock},
    };

    use super::{
        scale_for_dpi, show_native_taskbar_context_menu, taskbar_font_height, taskbar_row_bounds,
        TASKBAR_HORIZONTAL_PADDING, TASKBAR_MAX_WIDTH, TASKBAR_MIN_WIDTH,
    };
    use tauri::AppHandle;
    use windows::{
        core::w,
        Win32::{
            Foundation::{COLORREF, ERROR_SUCCESS, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
            Graphics::Gdi::{
                BeginPaint, CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint,
                FillRect, GetDC, GetPixel, InvalidateRect, ReleaseDC, SelectObject, SetBkMode,
                SetTextColor, UpdateWindow, CLEARTYPE_NATURAL_QUALITY, CLIP_DEFAULT_PRECIS,
                DEFAULT_CHARSET, DEFAULT_PITCH, DT_CENTER, DT_END_ELLIPSIS, DT_NOPREFIX,
                DT_SINGLELINE, DT_VCENTER, FF_SWISS, FONT_QUALITY, FW_NORMAL, HDC, HGDIOBJ,
                OUT_DEFAULT_PRECIS, PAINTSTRUCT, TRANSPARENT,
            },
            System::{
                LibraryLoader::GetModuleHandleW,
                Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD},
            },
            UI::{
                HiDpi::GetDpiForWindow,
                WindowsAndMessaging::{
                    CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowExW, FindWindowW,
                    GetClientRect, GetWindowLongPtrW, GetWindowRect, IsWindow, RegisterClassW,
                    SetLayeredWindowAttributes, SetParent, SetWindowLongPtrW, SetWindowPos,
                    ShowWindow, CS_HREDRAW, CS_VREDRAW, GWL_STYLE, HWND_TOP, LWA_COLORKEY,
                    SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_SHOWWINDOW, SW_HIDE, SW_SHOW,
                    WM_CONTEXTMENU, WM_ERASEBKGND, WM_PAINT, WNDCLASSW, WS_CHILD, WS_EX_LAYERED,
                    WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
                },
            },
        },
    };

    const TASKBAR_EDGE_PADDING: i32 = 2;
    const NOTIFICATION_AREA_GAP: i32 = 8;
    const COLOR_KEY: COLORREF = COLORREF(0x00010101);
    const DARK_TEXT: COLORREF = COLORREF(0x00141414);
    const LIGHT_TEXT: COLORREF = COLORREF(0x00ffffff);

    #[derive(Default)]
    struct NativeState {
        hwnd: isize,
        menu_owner_hwnd: isize,
        cells: Vec<(String, String)>,
        text_color: COLORREF,
    }

    static NATIVE_STATE: OnceLock<Mutex<NativeState>> = OnceLock::new();
    static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

    pub fn embed_and_show_native(
        app: AppHandle,
        cells: Vec<(String, String)>,
        desired_width: i32,
    ) -> Result<(), String> {
        let _ = APP_HANDLE.set(app.clone());

        app.run_on_main_thread(move || {
            if let Err(error) = embed_and_show_native_now(&cells, desired_width) {
                tracing::warn!("failed to draw native taskbar monitor: {error}");
                hide_native_now();
            }
        })
        .map_err(|error| format!("failed to run native taskbar monitor on main thread: {error}"))
    }

    fn embed_and_show_native_now(
        cells: &[(String, String)],
        desired_width: i32,
    ) -> Result<(), String> {
        unsafe {
            let taskbar_hwnd = FindWindowW(w!("Shell_TrayWnd"), None)
                .map_err(|error| format!("failed to find Shell_TrayWnd: {error}"))?;
            if taskbar_hwnd.0.is_null() {
                return Err("Shell_TrayWnd not found".to_string());
            }

            let hwnd = ensure_native_window(taskbar_hwnd)?;
            let dpi = window_dpi(hwnd);
            let (x, y, width, height) = resolve_taskbar_slot(
                taskbar_hwnd,
                scale_for_dpi(desired_width, dpi),
                scale_for_dpi(TASKBAR_MIN_WIDTH, dpi),
                scale_for_dpi(TASKBAR_MAX_WIDTH, dpi),
            );
            let text_color = resolve_theme_text_color(taskbar_hwnd, x, y, width, height);

            {
                let mut state = native_state().lock().map_err(|error| error.to_string())?;
                state.hwnd = hwnd.0 as isize;
                state.cells = cells.to_vec();
                state.text_color = text_color;
            }

            SetLayeredWindowAttributes(hwnd, COLOR_KEY, 0, LWA_COLORKEY).map_err(|error| {
                format!("failed to set native taskbar transparent key: {error}")
            })?;
            SetWindowPos(
                hwnd,
                Some(HWND_TOP),
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_SHOWWINDOW,
            )
            .map_err(|error| format!("failed to position native taskbar monitor: {error}"))?;

            let _ = InvalidateRect(Some(hwnd), None, true);
            let _ = UpdateWindow(hwnd);
            let _ = ShowWindow(hwnd, SW_SHOW);
        }

        Ok(())
    }

    pub fn hide_native(app: AppHandle) -> Result<(), String> {
        app.run_on_main_thread(hide_native_now).map_err(|error| {
            format!("failed to hide native taskbar monitor on main thread: {error}")
        })
    }

    fn hide_native_now() {
        let Ok(mut state) = native_state().lock() else {
            return;
        };

        if state.hwnd == 0 && state.menu_owner_hwnd == 0 {
            state.cells.clear();
            return;
        }

        unsafe {
            let hwnd = HWND(state.hwnd as *mut c_void);
            if IsWindow(Some(hwnd)).as_bool() {
                let _ = ShowWindow(hwnd, SW_HIDE);
                let _ = DestroyWindow(hwnd);
            }
            let menu_owner = HWND(state.menu_owner_hwnd as *mut c_void);
            if IsWindow(Some(menu_owner)).as_bool() {
                let _ = DestroyWindow(menu_owner);
            }
        }

        state.hwnd = 0;
        state.menu_owner_hwnd = 0;
        state.cells.clear();
    }

    fn native_state() -> &'static Mutex<NativeState> {
        NATIVE_STATE.get_or_init(|| Mutex::new(NativeState::default()))
    }

    unsafe fn ensure_native_window(parent: HWND) -> Result<HWND, String> {
        if let Ok(state) = native_state().lock() {
            if state.hwnd != 0 {
                let hwnd = HWND(state.hwnd as *mut c_void);
                if IsWindow(Some(hwnd)).as_bool() {
                    return Ok(hwnd);
                }
            }
        }

        let module = GetModuleHandleW(None)
            .map_err(|error| format!("failed to get native module handle: {error}"))?;
        let hinstance = HINSTANCE(module.0);
        let window_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(native_taskbar_proc),
            hInstance: hinstance,
            lpszClassName: w!("CoworkPalNativeTaskbarMonitor"),
            ..Default::default()
        };
        let _ = RegisterClassW(&window_class);
        let menu_owner_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(native_taskbar_proc),
            hInstance: hinstance,
            lpszClassName: w!("CoworkPalNativeTaskbarMenuOwner"),
            ..Default::default()
        };
        let _ = RegisterClassW(&menu_owner_class);

        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_LAYERED,
            w!("CoworkPalNativeTaskbarMonitor"),
            w!(""),
            WS_POPUP | WS_VISIBLE,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(hinstance),
            None,
        )
        .map_err(|error| format!("failed to create native taskbar monitor: {error}"))?;

        if hwnd.0.is_null() {
            return Err("native taskbar monitor HWND is null".to_string());
        }

        let menu_owner = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("CoworkPalNativeTaskbarMenuOwner"),
            w!(""),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(hinstance),
            None,
        )
        .map_err(|error| format!("failed to create native taskbar menu owner: {error}"))?;

        if menu_owner.0.is_null() {
            let _ = DestroyWindow(hwnd);
            return Err("native taskbar menu owner HWND is null".to_string());
        }

        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_STYLE,
            ((style as u32 & !WS_POPUP.0) | WS_CHILD.0 | WS_VISIBLE.0) as isize,
        );
        SetParent(hwnd, Some(parent))
            .map_err(|error| format!("failed to parent native taskbar monitor: {error}"))?;

        if let Ok(mut state) = native_state().lock() {
            state.menu_owner_hwnd = menu_owner.0 as isize;
        }

        Ok(hwnd)
    }

    unsafe extern "system" fn native_taskbar_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_ERASEBKGND => LRESULT(1),
            WM_PAINT => {
                paint_native(hwnd);
                LRESULT(0)
            }
            WM_CONTEXTMENU => {
                if let Some(app) = APP_HANDLE.get().cloned() {
                    let owner_hwnd = native_state()
                        .lock()
                        .ok()
                        .map(|state| state.menu_owner_hwnd)
                        .filter(|owner| *owner != 0)
                        .unwrap_or(hwnd.0 as isize);
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = show_native_taskbar_context_menu(app, owner_hwnd).await
                        {
                            tracing::warn!("failed to show native taskbar context menu: {error}");
                        }
                    });
                }
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }

    unsafe fn paint_native(hwnd: HWND) {
        let mut paint = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut paint);
        if hdc.0.is_null() {
            return;
        }

        let mut rect = RECT::default();
        if GetClientRect(hwnd, &mut rect).is_err() {
            let _ = EndPaint(hwnd, &paint);
            return;
        }

        let brush = CreateSolidBrush(COLOR_KEY);
        let _ = FillRect(hdc, &rect, brush);
        let _ = DeleteObject(HGDIOBJ(brush.0));

        let (cells, text_color) = native_state()
            .lock()
            .map(|state| (state.cells.clone(), state.text_color))
            .unwrap_or_else(|_| (Vec::new(), LIGHT_TEXT));

        draw_cells(hdc, rect, &cells, text_color, window_dpi(hwnd));

        let _ = EndPaint(hwnd, &paint);
    }

    unsafe fn draw_cells(
        hdc: HDC,
        rect: RECT,
        cells: &[(String, String)],
        text_color: COLORREF,
        dpi: u32,
    ) {
        let cell_count = cells.len().max(1) as i32;
        let width = (rect.right - rect.left).max(1);
        let content_width = (width - scale_for_dpi(TASKBAR_HORIZONTAL_PADDING, dpi)).max(1);
        let content_left = rect.right - content_width;
        let column_width = (content_width / cell_count).max(1);
        let (rows_top, middle_y, rows_bottom) = taskbar_row_bounds(rect.top, rect.bottom, dpi);
        let horizontal_inset = scale_for_dpi(2, dpi).max(1);
        let font = CreateFontW(
            taskbar_font_height(dpi),
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            FONT_QUALITY(CLEARTYPE_NATURAL_QUALITY as u8),
            (DEFAULT_PITCH.0 | FF_SWISS.0) as u32,
            w!("Microsoft YaHei Light"),
        );
        let old_font = SelectObject(hdc, HGDIOBJ(font.0));
        let _ = SetBkMode(hdc, TRANSPARENT);
        let _ = SetTextColor(hdc, text_color);

        for (index, (top, bottom)) in cells.iter().enumerate() {
            let left = content_left + (index as i32 * column_width);
            let right = if index == cells.len().saturating_sub(1) {
                rect.right
            } else {
                left + column_width
            };

            let top_rect = RECT {
                left: left + horizontal_inset,
                top: rows_top,
                right: right - horizontal_inset,
                bottom: middle_y,
            };
            let bottom_rect = RECT {
                left: left + horizontal_inset,
                top: middle_y,
                right: right - horizontal_inset,
                bottom: rows_bottom,
            };

            let mut top_rect = top_rect;
            let mut bottom_rect = bottom_rect;
            draw_text(hdc, top, &mut top_rect);
            draw_text(hdc, bottom, &mut bottom_rect);
        }

        if !old_font.0.is_null() {
            let _ = SelectObject(hdc, old_font);
        }
        if !font.0.is_null() {
            let _ = DeleteObject(HGDIOBJ(font.0));
        }
    }

    unsafe fn draw_text(hdc: HDC, text: &str, rect: &mut RECT) {
        if text.is_empty() {
            return;
        }

        let mut text: Vec<u16> = text.encode_utf16().collect();
        let _ = DrawTextW(
            hdc,
            &mut text,
            rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
        );
    }

    unsafe fn resolve_theme_text_color(
        taskbar_hwnd: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> COLORREF {
        if let Some(system_uses_light_theme) = read_system_uses_light_theme() {
            return if system_uses_light_theme {
                DARK_TEXT
            } else {
                LIGHT_TEXT
            };
        }

        sample_text_color(
            taskbar_hwnd,
            x + width + (NOTIFICATION_AREA_GAP / 2),
            y + (height / 2),
        )
    }

    unsafe fn read_system_uses_light_theme() -> Option<bool> {
        let mut value: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let status = RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut size),
        );

        if status == ERROR_SUCCESS {
            Some(value != 0)
        } else {
            None
        }
    }

    unsafe fn sample_text_color(taskbar_hwnd: HWND, x: i32, y: i32) -> COLORREF {
        let hdc = GetDC(Some(taskbar_hwnd));
        if hdc.0.is_null() {
            return LIGHT_TEXT;
        }

        let pixel = GetPixel(hdc, x, y).0;
        let _ = ReleaseDC(Some(taskbar_hwnd), hdc);

        if pixel == 0xffffffff {
            return LIGHT_TEXT;
        }

        let red = (pixel & 0xff) as f32;
        let green = ((pixel >> 8) & 0xff) as f32;
        let blue = ((pixel >> 16) & 0xff) as f32;
        let brightness = (red * 0.299) + (green * 0.587) + (blue * 0.114);

        if brightness > 150.0 {
            DARK_TEXT
        } else {
            LIGHT_TEXT
        }
    }

    unsafe fn window_dpi(hwnd: HWND) -> u32 {
        let dpi = GetDpiForWindow(hwnd);
        if dpi == 0 {
            super::DEFAULT_DPI
        } else {
            dpi
        }
    }

    unsafe fn resolve_taskbar_slot(
        taskbar_hwnd: HWND,
        desired_width: i32,
        min_width: i32,
        max_width: i32,
    ) -> (i32, i32, i32, i32) {
        let mut client = RECT::default();
        if GetClientRect(taskbar_hwnd, &mut client).is_err() {
            return (0, 0, desired_width.clamp(min_width, max_width), 36);
        }

        let taskbar_width = (client.right - client.left).max(min_width);
        let taskbar_height = (client.bottom - client.top).max(24);
        let tray_left = find_notification_area_left(taskbar_hwnd).unwrap_or(taskbar_width);
        let available_width =
            (tray_left - NOTIFICATION_AREA_GAP - TASKBAR_EDGE_PADDING).max(min_width);
        let width = desired_width
            .clamp(min_width, max_width)
            .min(available_width);
        let x = (tray_left - width - NOTIFICATION_AREA_GAP).max(TASKBAR_EDGE_PADDING);

        (x, 0, width, taskbar_height)
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
}

#[cfg(test)]
mod tests {
    use super::{scale_for_dpi, taskbar_font_height, taskbar_row_bounds};

    #[test]
    fn taskbar_typography_scales_with_monitor_dpi() {
        assert_eq!(scale_for_dpi(64, 0), 64);
        assert_eq!(scale_for_dpi(64, 96), 64);
        assert_eq!(scale_for_dpi(64, 120), 80);
        assert_eq!(scale_for_dpi(64, 144), 96);
        assert_eq!(scale_for_dpi(64, 192), 128);

        assert_eq!(taskbar_font_height(0), -12);
        assert_eq!(taskbar_font_height(96), -12);
        assert_eq!(taskbar_font_height(120), -15);
        assert_eq!(taskbar_font_height(144), -18);
        assert_eq!(taskbar_font_height(192), -24);

        assert_eq!(taskbar_row_bounds(0, 32, 96), (0, 16, 32));
        assert_eq!(taskbar_row_bounds(0, 60, 120), (10, 30, 50));
        assert_eq!(taskbar_row_bounds(0, 72, 144), (12, 36, 60));
        assert_eq!(taskbar_row_bounds(0, 96, 192), (16, 48, 80));
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
