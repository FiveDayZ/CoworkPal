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
#[cfg(windows)]
const TASKBAR_FONT_BYTES: &[u8] = include_bytes!("../fonts/NotoSansCJKsc-CoworkPal.otf");

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

    tracing::info!("opening taskbar context menu");
    let menu = crate::tray::build_shared_menu(&app)
        .await
        .map_err(|error| format!("failed to build taskbar context menu: {error}"))?;
    let popup_handle = menu
        .hpopupmenu()
        .map_err(|error| format!("failed to resolve taskbar menu handle: {error}"))?;
    let dispatch_app = app.clone();

    app.run_on_main_thread(move || unsafe {
        // Keep the Tauri menu alive for the whole native modal menu loop. Dropping it after
        // scheduling this closure invalidates HMENU while TrackPopupMenuEx is still using it.
        let _menu_guard = menu;
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
        tracing::info!(command_id = selected, "taskbar context menu closed");
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
        DEFAULT_DPI, TASKBAR_FONT_BYTES, TASKBAR_FONT_POINT_SIZE, TASKBAR_HORIZONTAL_PADDING,
        TASKBAR_MAX_WIDTH, TASKBAR_MIN_WIDTH,
    };
    use tauri::AppHandle;
    use windows::{
        core::{w, Interface},
        Win32::{
            Foundation::{
                COLORREF, ERROR_SUCCESS, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE,
                WPARAM,
            },
            Graphics::{
                Direct2D::{
                    Common::{D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT},
                    D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory,
                    D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_DRAW_TEXT_OPTIONS_NO_SNAP,
                    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
                    D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT,
                    D2D1_RENDER_TARGET_USAGE_NONE, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
                },
                DirectWrite::{
                    DWriteCreateFactory, IDWriteFactory, IDWriteFactory5, IDWriteFontCollection,
                    IDWriteInMemoryFontFileLoader, IDWriteTextFormat, IDWriteTextLayout,
                    DWRITE_FACTORY_TYPE_ISOLATED, DWRITE_FONT_STRETCH_NORMAL,
                    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
                    DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER,
                    DWRITE_TRIMMING, DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                    DWRITE_WORD_WRAPPING_NO_WRAP,
                },
                Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
                Gdi::{
                    AddFontMemResourceEx, BeginPaint, CreateCompatibleDC, CreateDIBSection,
                    CreateFontW, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint,
                    FillRect, GetDC, GetPixel, InvalidateRect, ReleaseDC, SelectObject, SetBkMode,
                    SetTextColor, UpdateWindow, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO,
                    BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
                    DEFAULT_PITCH, DEFAULT_QUALITY, DIB_RGB_COLORS, DT_CENTER, DT_END_ELLIPSIS,
                    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FF_SWISS, FW_NORMAL, HBITMAP, HDC,
                    HGDIOBJ, OUT_DEFAULT_PRECIS, PAINTSTRUCT, TRANSPARENT,
                },
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
                    ShowWindow, UpdateLayeredWindow, CS_HREDRAW, CS_VREDRAW, GWL_STYLE, HWND_TOP,
                    LWA_COLORKEY, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_SHOWWINDOW, SW_HIDE,
                    SW_SHOW, ULW_ALPHA, WM_CONTEXTMENU, WM_ERASEBKGND, WM_PAINT, WNDCLASSW,
                    WS_CHILD, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
                },
            },
        },
    };
    use windows_numerics::Vector2;

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
    static EMBEDDED_FONT_REGISTERED: OnceLock<bool> = OnceLock::new();
    static DIRECTWRITE_RENDERER: OnceLock<Result<Mutex<DirectWriteRenderer>, String>> =
        OnceLock::new();
    static DIRECTWRITE_FAILURE_LOGGED: OnceLock<()> = OnceLock::new();

    struct DirectWriteRenderer {
        factory: IDWriteFactory,
        font_collection: IDWriteFontCollection,
        target: ID2D1DCRenderTarget,
        _font_loader: IDWriteInMemoryFontFileLoader,
    }

    struct MemorySurface {
        dc: HDC,
        bitmap: HBITMAP,
        old_bitmap: HGDIOBJ,
    }

    impl MemorySurface {
        unsafe fn new(width: i32, height: i32) -> Result<Self, String> {
            let dc = CreateCompatibleDC(None);
            if dc.0.is_null() {
                return Err("failed to create taskbar memory DC".to_string());
            }

            let bitmap_info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap = match CreateDIBSection(
                Some(dc),
                &bitmap_info,
                DIB_RGB_COLORS,
                &mut bits,
                None,
                0,
            ) {
                Ok(bitmap) if !bits.is_null() => bitmap,
                Ok(bitmap) => {
                    let _ = DeleteObject(HGDIOBJ(bitmap.0));
                    let _ = DeleteDC(dc);
                    return Err("taskbar DIB section has no pixel buffer".to_string());
                }
                Err(error) => {
                    let _ = DeleteDC(dc);
                    return Err(format!("failed to create taskbar DIB section: {error}"));
                }
            };
            let old_bitmap = SelectObject(dc, HGDIOBJ(bitmap.0));

            Ok(Self {
                dc,
                bitmap,
                old_bitmap,
            })
        }
    }

    impl Drop for MemorySurface {
        fn drop(&mut self) {
            unsafe {
                if !self.old_bitmap.0.is_null() {
                    let _ = SelectObject(self.dc, self.old_bitmap);
                }
                let _ = DeleteObject(HGDIOBJ(self.bitmap.0));
                let _ = DeleteDC(self.dc);
            }
        }
    }

    impl DirectWriteRenderer {
        unsafe fn new() -> Result<Self, String> {
            let factory5 = DWriteCreateFactory::<IDWriteFactory5>(DWRITE_FACTORY_TYPE_ISOLATED)
                .map_err(|error| format!("failed to create DirectWrite factory: {error}"))?;
            let font_loader = factory5
                .CreateInMemoryFontFileLoader()
                .map_err(|error| format!("failed to create embedded font loader: {error}"))?;
            factory5
                .RegisterFontFileLoader(&font_loader)
                .map_err(|error| format!("failed to register embedded font loader: {error}"))?;
            let font_file = font_loader
                .CreateInMemoryFontFileReference(
                    &factory5,
                    TASKBAR_FONT_BYTES.as_ptr().cast(),
                    TASKBAR_FONT_BYTES.len() as u32,
                    None,
                )
                .map_err(|error| format!("failed to load embedded taskbar font: {error}"))?;
            let font_set_builder = factory5
                .CreateFontSetBuilder()
                .map_err(|error| format!("failed to create taskbar font set: {error}"))?;
            font_set_builder
                .AddFontFile(&font_file)
                .map_err(|error| format!("failed to add embedded taskbar font: {error}"))?;
            let font_set = font_set_builder
                .CreateFontSet()
                .map_err(|error| format!("failed to finalize taskbar font set: {error}"))?;
            let font_collection = factory5
                .CreateFontCollectionFromFontSet(&font_set)
                .map_err(|error| format!("failed to create taskbar font collection: {error}"))?
                .cast::<IDWriteFontCollection>()
                .map_err(|error| format!("failed to use taskbar font collection: {error}"))?;
            let factory = factory5
                .cast::<IDWriteFactory>()
                .map_err(|error| format!("failed to use DirectWrite factory: {error}"))?;

            let d2d_factory =
                D2D1CreateFactory::<ID2D1Factory>(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)
                    .map_err(|error| format!("failed to create Direct2D factory: {error}"))?;
            let properties = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: DEFAULT_DPI as f32,
                dpiY: DEFAULT_DPI as f32,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let target = d2d_factory
                .CreateDCRenderTarget(&properties)
                .map_err(|error| format!("failed to create Direct2D DC target: {error}"))?;

            Ok(Self {
                factory,
                font_collection,
                target,
                _font_loader: font_loader,
            })
        }

        unsafe fn create_text_format(&self, dpi: u32) -> Result<IDWriteTextFormat, String> {
            let font_size = TASKBAR_FONT_POINT_SIZE as f32 * DEFAULT_DPI as f32 / 72.0
                * dpi.max(1) as f32
                / DEFAULT_DPI as f32;
            let format = self
                .factory
                .CreateTextFormat(
                    w!("Noto Sans CJK SC"),
                    &self.font_collection,
                    DWRITE_FONT_WEIGHT_NORMAL,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    font_size,
                    w!("zh-CN"),
                )
                .map_err(|error| format!("failed to create taskbar text format: {error}"))?;
            format
                .SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)
                .map_err(|error| format!("failed to center taskbar text: {error}"))?;
            format
                .SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)
                .map_err(|error| format!("failed to center taskbar text row: {error}"))?;
            format
                .SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)
                .map_err(|error| format!("failed to disable taskbar text wrapping: {error}"))?;
            let ellipsis = self
                .factory
                .CreateEllipsisTrimmingSign(&format)
                .map_err(|error| format!("failed to create taskbar ellipsis: {error}"))?;
            let trimming = DWRITE_TRIMMING {
                granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                delimiter: 0,
                delimiterCount: 0,
            };
            format
                .SetTrimming(&trimming, &ellipsis)
                .map_err(|error| format!("failed to enable taskbar ellipsis: {error}"))?;
            Ok(format)
        }

        unsafe fn create_text_layout(
            &self,
            text: &str,
            width: f32,
            height: f32,
            format: &IDWriteTextFormat,
        ) -> Result<IDWriteTextLayout, String> {
            let text: Vec<u16> = text.encode_utf16().collect();
            self.factory
                .CreateTextLayout(&text, format, width.max(1.0), height.max(1.0))
                .map_err(|error| format!("failed to layout taskbar text: {error}"))
        }

        unsafe fn draw(
            &mut self,
            hwnd: HWND,
            rect: RECT,
            cells: &[(String, String)],
            text_color: COLORREF,
            dpi: u32,
        ) -> Result<(), String> {
            let cell_count = cells.len().max(1) as i32;
            let width = (rect.right - rect.left).max(1);
            let content_width = (width - scale_for_dpi(TASKBAR_HORIZONTAL_PADDING, dpi)).max(1);
            let content_left = rect.right - content_width;
            let column_width = (content_width / cell_count).max(1);
            let (rows_top, middle_y, rows_bottom) = taskbar_row_bounds(rect.top, rect.bottom, dpi);
            let horizontal_inset = scale_for_dpi(2, dpi).max(1);
            let height = (rect.bottom - rect.top).max(1);
            let surface = MemorySurface::new(width, height)?;
            let target_rect = RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            };
            let format = self.create_text_format(dpi)?;
            let mut layouts = Vec::with_capacity(cells.len() * 2);

            for (index, (top, bottom)) in cells.iter().enumerate() {
                let left = content_left + index as i32 * column_width;
                let right = if index == cells.len().saturating_sub(1) {
                    rect.right
                } else {
                    left + column_width
                };
                let text_left = left + horizontal_inset;
                let text_width = (right - horizontal_inset - text_left).max(1) as f32;

                for (text, row_top, row_bottom) in
                    [(top, rows_top, middle_y), (bottom, middle_y, rows_bottom)]
                {
                    if !text.is_empty() {
                        layouts.push((
                            Vector2 {
                                X: text_left as f32,
                                Y: row_top as f32,
                            },
                            self.create_text_layout(
                                text,
                                text_width,
                                (row_bottom - row_top).max(1) as f32,
                                &format,
                            )?,
                        ));
                    }
                }
            }

            self.target
                .BindDC(surface.dc, &target_rect)
                .map_err(|error| format!("failed to bind Direct2D taskbar target: {error}"))?;
            self.target
                .SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            let color = colorref_to_d2d(text_color);
            let brush = self
                .target
                .CreateSolidColorBrush(&color, None)
                .map_err(|error| format!("failed to create taskbar text brush: {error}"))?;

            self.target.BeginDraw();
            self.target.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));
            for (origin, layout) in layouts {
                self.target.DrawTextLayout(
                    origin,
                    &layout,
                    &brush,
                    D2D1_DRAW_TEXT_OPTIONS_CLIP | D2D1_DRAW_TEXT_OPTIONS_NO_SNAP,
                );
            }
            self.target
                .EndDraw(None, None)
                .map_err(|error| format!("failed to draw DirectWrite taskbar text: {error}"))?;

            let source = POINT::default();
            let size = SIZE {
                cx: width,
                cy: height,
            };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            UpdateLayeredWindow(
                hwnd,
                None,
                None,
                Some(&size),
                Some(surface.dc),
                Some(&source),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )
            .map_err(|error| format!("failed to alpha-compose taskbar text: {error}"))
        }
    }

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

        let (cells, text_color) = native_state()
            .lock()
            .map(|state| (state.cells.clone(), state.text_color))
            .unwrap_or_else(|_| (Vec::new(), LIGHT_TEXT));

        let dpi = window_dpi(hwnd);
        if let Err(error) = draw_cells_directwrite(hwnd, rect, &cells, text_color, dpi) {
            if DIRECTWRITE_FAILURE_LOGGED.set(()).is_ok() {
                tracing::warn!(
                    "DirectWrite taskbar rendering unavailable; using GDI fallback: {error}"
                );
            }
            let _ = SetLayeredWindowAttributes(hwnd, COLOR_KEY, 0, LWA_COLORKEY);
            fill_color_key(hdc, &rect);
            draw_cells_gdi(hdc, rect, &cells, text_color, dpi);
        }

        let _ = EndPaint(hwnd, &paint);
    }

    unsafe fn draw_cells_directwrite(
        hwnd: HWND,
        rect: RECT,
        cells: &[(String, String)],
        text_color: COLORREF,
        dpi: u32,
    ) -> Result<(), String> {
        let renderer =
            match DIRECTWRITE_RENDERER.get_or_init(|| DirectWriteRenderer::new().map(Mutex::new)) {
                Ok(renderer) => renderer,
                Err(error) => return Err(error.clone()),
            };
        renderer
            .lock()
            .map_err(|error| format!("failed to lock DirectWrite taskbar renderer: {error}"))?
            .draw(hwnd, rect, cells, text_color, dpi)
    }

    unsafe fn fill_color_key(hdc: HDC, rect: &RECT) {
        let brush = CreateSolidBrush(COLOR_KEY);
        let _ = FillRect(hdc, rect, brush);
        let _ = DeleteObject(HGDIOBJ(brush.0));
    }

    unsafe fn draw_cells_gdi(
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
        let font_face = if ensure_embedded_taskbar_font() {
            w!("Noto Sans CJK SC")
        } else {
            w!("Microsoft YaHei")
        };
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
            DEFAULT_QUALITY,
            (DEFAULT_PITCH.0 | FF_SWISS.0) as u32,
            font_face,
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

    pub(super) fn ensure_embedded_taskbar_font() -> bool {
        *EMBEDDED_FONT_REGISTERED.get_or_init(|| unsafe {
            let font_count = std::cell::UnsafeCell::new(0_u32);
            let handle = AddFontMemResourceEx(
                TASKBAR_FONT_BYTES.as_ptr().cast(),
                TASKBAR_FONT_BYTES.len() as u32,
                None,
                font_count.get(),
            );
            !handle.0.is_null() && *font_count.get() > 0
        })
    }

    #[cfg(test)]
    pub(super) fn verify_directwrite_taskbar_font() -> Result<(), String> {
        unsafe {
            let renderer = match DIRECTWRITE_RENDERER
                .get_or_init(|| DirectWriteRenderer::new().map(Mutex::new))
            {
                Ok(renderer) => renderer,
                Err(error) => return Err(error.clone()),
            };
            let renderer = renderer
                .lock()
                .map_err(|error| format!("failed to lock DirectWrite taskbar renderer: {error}"))?;
            let mut family_index = 0;
            let mut family_exists = windows::core::BOOL::default();
            renderer
                .font_collection
                .FindFamilyName(
                    w!("Noto Sans CJK SC"),
                    &mut family_index,
                    &mut family_exists,
                )
                .map_err(|error| format!("failed to inspect embedded font collection: {error}"))?;
            if !family_exists.as_bool() {
                return Err("embedded font family is missing from DirectWrite collection".into());
            }
            let format = renderer.create_text_format(DEFAULT_DPI)?;
            let _ = renderer.create_text_layout("CPU 42% 温度", 96.0, 16.0, &format)?;
            Ok(())
        }
    }

    fn colorref_to_d2d(color: COLORREF) -> D2D1_COLOR_F {
        D2D1_COLOR_F {
            r: (color.0 & 0xff) as f32 / 255.0,
            g: ((color.0 >> 8) & 0xff) as f32 / 255.0,
            b: ((color.0 >> 16) & 0xff) as f32 / 255.0,
            a: 1.0,
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
    #[cfg(windows)]
    use super::{
        imp::{ensure_embedded_taskbar_font, verify_directwrite_taskbar_font},
        TASKBAR_FONT_BYTES,
    };
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

    #[cfg(windows)]
    #[test]
    fn taskbar_font_is_embedded_and_registers_with_gdi() {
        assert!(TASKBAR_FONT_BYTES.starts_with(b"OTTO"));
        assert!(TASKBAR_FONT_BYTES.len() < 100_000);
        assert!(ensure_embedded_taskbar_font());
    }

    #[cfg(windows)]
    #[test]
    fn taskbar_font_loads_with_directwrite() {
        verify_directwrite_taskbar_font().expect("embedded font should load with DirectWrite");
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
