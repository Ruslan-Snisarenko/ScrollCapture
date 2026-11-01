#![windows_subsystem = "windows"]

mod capture;
mod stitcher;
mod input;
mod clipboard;
mod selector;

use std::fs;
use std::path::PathBuf;
use std::ptr::null_mut;
use std::time::Duration;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::commctrl::*;
use winapi::um::minwinbase::SYSTEMTIME;
use winapi::um::shellapi::*;
use winapi::um::sysinfoapi::GetLocalTime;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

const BTN_SCROLL_ID: usize = 101;
const BTN_SNIP_ID: usize = 102;
const BTN_FOLDER_ID: usize = 103;
const BTN_AUTHOR_ID: usize = 104;

const HOTKEY_SCROLL_ID: i32 = 201;
const HOTKEY_SNIP_ID: i32 = 202;
const HOTKEY_SNIP_ALT_ID: i32 = 203;

const WM_TRAYICON: UINT = WM_USER + 10;
const TRAY_MENU_SHOW: usize = 301;
const TRAY_MENU_SCROLL: usize = 302;
const TRAY_MENU_SNIP: usize = 303;
const TRAY_MENU_FOLDER: usize = 304;
const TRAY_MENU_EXIT: usize = 305;

const AUTHOR_URL: &str = "https://t.me/Carist";

// --- Цветовая палитра как в RazerDiamondbackController ---
const COLOR_BG: COLORREF = 0x00181210;         // #101218 (RGB 16, 18, 24 в BGR)
const COLOR_CARD_BG: COLORREF = 0x00221916;    // #161922 (RGB 22, 25, 34 в BGR)
const COLOR_CARD_BORDER: COLORREF = 0x00342622;// #222634 (RGB 34, 38, 52 в BGR)

// Кнопка 1: Снимок с прокруткой (Royal Blue)
const COLOR_BTN_PRIMARY: COLORREF = 0x00EB6325;       // #2563EB (RGB 37, 99, 235 в BGR)
const COLOR_BTN_PRIMARY_HOVER: COLORREF = 0x00F6823B; // #3B82F6 (RGB 59, 130, 246 в BGR)
const COLOR_BTN_PRIMARY_ACTIVE: COLORREF = 0x00D84E1D;// #1D4ED8 (RGB 29, 78, 216 в BGR)

// Кнопка 2: Обычный снимок области (Cyan / Sky Blue)
const COLOR_BTN_SNIP: COLORREF = 0x00C78402;         // #0284C7 (RGB 2, 132, 199 в BGR)
const COLOR_BTN_SNIP_HOVER: COLORREF = 0x00E9A50E;   // #0EA5E9 (RGB 14, 165, 233 в BGR)
const COLOR_BTN_SNIP_ACTIVE: COLORREF = 0x00A16A03;  // #036AA1 (RGB 3, 106, 161 в BGR)

// Кнопка 3: Папка (Dark Slate)
const COLOR_BTN_SEC_BG: COLORREF = 0x002A1E1A;       // #1A1E2A (RGB 26, 30, 42 в BGR)
const COLOR_BTN_SEC_HOVER: COLORREF = 0x003C2B25;    // #252B3C (RGB 37, 43, 60 в BGR)
const COLOR_BTN_SEC_BORDER: COLORREF = 0x0048352E;   // #2E3548 (RGB 46, 53, 72 в BGR)
const COLOR_BTN_SEC_BORDER_HOVER: COLORREF = 0x0060473E; // #3E4760

// Бейдж автора из RazerDiamondbackController
const COLOR_AUTHOR_BG: COLORREF = 0x001E1714;       // #14171E (RGB 20, 23, 30 в BGR)
const COLOR_AUTHOR_BORDER: COLORREF = 0x00382A24;   // #242A38 (RGB 36, 42, 56 в BGR)
const COLOR_AUTHOR_HOVER_BG: COLORREF = 0x002C201A; // #1A202C (RGB 26, 32, 44 в BGR)
const COLOR_AUTHOR_TEXT: COLORREF = 0x00FFAA46;     // #46AAFF (RGB 70, 170, 255 в BGR)

static mut HWND_SCROLL: HWND = null_mut();
static mut HWND_SNIP: HWND = null_mut();
static mut HWND_FOLDER: HWND = null_mut();
static mut HWND_AUTHOR: HWND = null_mut();

static mut IS_SCROLL_HOVERED: bool = false;
static mut IS_SNIP_HOVERED: bool = false;
static mut IS_FOLDER_HOVERED: bool = false;
static mut IS_AUTHOR_HOVERED: bool = false;

static mut OLD_BTN_PROC: WNDPROC = None;

#[allow(dead_code)]
struct TelegramIcon {
    h_dc: HDC,
    h_bmp: HBITMAP,
    width: i32,
    height: i32,
}

static mut TG_ICON: Option<TelegramIcon> = None;

unsafe fn init_telegram_icon(screen_dc: HDC) {
    let img = match image::load_from_memory(include_bytes!("telegram_icon.png")) {
        Ok(i) => i.to_rgba8(),
        Err(_) => return,
    };
    let size = 18;
    let resized = image::imageops::resize(&img, size, size, image::imageops::FilterType::Lanczos3);

    let mem_dc = CreateCompatibleDC(screen_dc);
    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = size as i32;
    bmi.bmiHeader.biHeight = -(size as i32);
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    let mut bits: *mut u8 = null_mut();
    let h_bmp = CreateDIBSection(
        mem_dc,
        &bmi,
        DIB_RGB_COLORS,
        &mut bits as *mut _ as *mut *mut _,
        null_mut(),
        0,
    );

    if !h_bmp.is_null() && !bits.is_null() {
        SelectObject(mem_dc, h_bmp as *mut _);
        for y in 0..size {
            for x in 0..size {
                let p = resized.get_pixel(x, y);
                let a = p[3] as f32 / 255.0;
                let pr = (p[0] as f32 * a) as u8;
                let pg = (p[1] as f32 * a) as u8;
                let pb = (p[2] as f32 * a) as u8;
                let offset = ((y * size + x) * 4) as usize;
                *bits.add(offset) = pb;
                *bits.add(offset + 1) = pg;
                *bits.add(offset + 2) = pr;
                *bits.add(offset + 3) = p[3];
            }
        }
        TG_ICON = Some(TelegramIcon {
            h_dc: mem_dc,
            h_bmp,
            width: size as i32,
            height: size as i32,
        });
    }
}

unsafe fn create_hicon_from_rgba(rgba_img: &image::RgbaImage) -> HICON {
    let (w, h) = rgba_img.dimensions();
    let screen_dc = GetDC(null_mut());
    let mem_dc = CreateCompatibleDC(screen_dc);

    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = w as i32;
    bmi.bmiHeader.biHeight = -(h as i32);
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    let mut bits: *mut u8 = null_mut();
    let h_color = CreateDIBSection(
        mem_dc,
        &bmi,
        DIB_RGB_COLORS,
        &mut bits as *mut _ as *mut *mut _,
        null_mut(),
        0,
    );

    if bits.is_null() {
        DeleteDC(mem_dc);
        ReleaseDC(null_mut(), screen_dc);
        return null_mut();
    }

    for y in 0..h {
        for x in 0..w {
            let p = rgba_img.get_pixel(x, y);
            let a = p[3] as f32 / 255.0;
            let pr = (p[0] as f32 * a) as u8;
            let pg = (p[1] as f32 * a) as u8;
            let pb = (p[2] as f32 * a) as u8;
            let offset = ((y * w + x) * 4) as usize;
            *bits.add(offset) = pb;
            *bits.add(offset + 1) = pg;
            *bits.add(offset + 2) = pr;
            *bits.add(offset + 3) = p[3];
        }
    }

    let h_mask = CreateCompatibleBitmap(screen_dc, w as i32, h as i32);

    let mut icon_info: ICONINFO = std::mem::zeroed();
    icon_info.fIcon = 1;
    icon_info.xHotspot = 0;
    icon_info.yHotspot = 0;
    icon_info.hbmMask = h_mask;
    icon_info.hbmColor = h_color;

    let h_icon = CreateIconIndirect(&mut icon_info);

    DeleteObject(h_color as *mut _);
    DeleteObject(h_mask as *mut _);
    DeleteDC(mem_dc);
    ReleaseDC(null_mut(), screen_dc);

    h_icon
}

fn get_exe_dir() -> PathBuf {
    std::env::current_exe()
        .map(|p| p.parent().unwrap_or(&p).to_path_buf())
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn generate_screenshot_path(is_scroll: bool) -> PathBuf {
    let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut st) };

    let prefix = if is_scroll { "ScrollScreenshot" } else { "AreaScreenshot" };
    let filename = format!(
        "{}_{:04}-{:02}-{:02}_{:02}-{:02}-{:02}.png",
        prefix, st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
    );

    let mut dir = get_exe_dir();
    dir.push("Screenshots");
    let _ = fs::create_dir_all(&dir);
    dir.push(filename);
    dir
}

fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn show_tray_balloon(hwnd: HWND, title: &str, msg: &str) {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_INFO;
    nid.dwInfoFlags = NIIF_INFO;

    let w_title = to_wide_null(title);
    let w_msg = to_wide_null(msg);

    let title_len = w_title.len().min(63);
    for i in 0..title_len {
        nid.szInfoTitle[i] = w_title[i];
    }
    let msg_len = w_msg.len().min(255);
    for i in 0..msg_len {
        nid.szInfo[i] = w_msg[i];
    }

    Shell_NotifyIconW(NIM_MODIFY, &mut nid);
}

// 1. Захват с прокруткой
unsafe fn trigger_scroll_capture(hwnd: HWND) {
    let was_visible = IsWindowVisible(hwnd) != 0;
    if was_visible {
        ShowWindow(hwnd, SW_HIDE);
        std::thread::sleep(Duration::from_millis(200));
    }

    if let Some(rect) = selector::select_region() {
        run_scrolling_capture(hwnd, rect);
    }

    if was_visible {
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
    }
}

// 2. Обычный снимок выделенной области
unsafe fn trigger_regular_snip(hwnd: HWND) {
    let was_visible = IsWindowVisible(hwnd) != 0;
    if was_visible {
        ShowWindow(hwnd, SW_HIDE);
        std::thread::sleep(Duration::from_millis(180));
    }

    if let Some(rect) = selector::select_region() {
        if let Some(img) = capture::capture_screen_rect(rect.x, rect.y, rect.width, rect.height) {
            let save_path = generate_screenshot_path(false);
            let _ = img.save(&save_path);
            let _clip_ok = clipboard::copy_image_to_clipboard(&img);

            MessageBeep(MB_OK);

            let (w, h) = img.dimensions();
            let balloon_msg = format!("{} × {} px скопировано в буфер (Ctrl+V) и сохранено в Screenshots", w, h);
            show_tray_balloon(hwnd, "✂ Снимок области готов!", &balloon_msg);
        }
    }

    if was_visible {
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
    }
}

unsafe fn run_scrolling_capture(hwnd: HWND, rect: selector::SelectedRect) {
    let center_x = rect.x + (rect.width as i32 / 2);
    let center_y = rect.y + (rect.height as i32 / 2);

    let scroll_clicks = 2;
    let delay_ms = 280;
    let max_scrolls = 70;

    let first_frame = match capture::capture_screen_rect(rect.x, rect.y, rect.width, rect.height) {
        Some(img) => img,
        None => {
            let msg = to_wide_null("Не удалось сделать начальный снимок выбранной области.");
            let title = to_wide_null("Ошибка захвата");
            MessageBoxW(hwnd, msg.as_ptr(), title.as_ptr(), MB_ICONERROR);
            return;
        }
    };

    let mut canvas = first_frame.clone();
    let mut prev_frame = first_frame;
    let mut frames_count = 1;
    let mut consecutive_no_motion = 0;

    for _step in 1..=max_scrolls {
        if input::is_esc_pressed() {
            break;
        }

        input::smooth_scroll_down(center_x, center_y, scroll_clicks);

        let mut waited = 0;
        while waited < delay_ms {
            if input::is_esc_pressed() {
                break;
            }
            std::thread::sleep(Duration::from_millis(30));
            waited += 30;
        }

        if input::is_esc_pressed() {
            break;
        }

        let curr_frame = match capture::capture_screen_rect(rect.x, rect.y, rect.width, rect.height) {
            Some(img) => img,
            None => break,
        };

        let res = stitcher::find_vertical_shift(&prev_frame, &curr_frame);

        if res.shift_y <= 1 {
            consecutive_no_motion += 1;
            if consecutive_no_motion >= 2 {
                break;
            }
            continue;
        } else {
            consecutive_no_motion = 0;
        }

        let shift = res.shift_y.min(rect.height - 8);
        stitcher::append_slice(&mut canvas, &curr_frame, shift);
        prev_frame = curr_frame;
        frames_count += 1;
    }

    let save_path = generate_screenshot_path(true);
    let save_str = save_path.to_string_lossy().to_string();

    let save_ok = canvas.save(&save_path).is_ok();
    let clip_ok = clipboard::copy_image_to_clipboard(&canvas);

    let (w, h) = canvas.dimensions();

    let result_msg = format!(
        "Снимок экрана успешно создан!\n\n\
        📐 Разрешение: {} × {} px\n\
        🎞 Склеено кадров: {}\n\
        📋 Скопировано в буфер: {}\n\
        💾 Сохранено: {}\n\n\
        👤 Автор: @Carist (Telegram)\n\n\
        Открыть изображение сейчас?",
        w, h, frames_count,
        if clip_ok { "Да (Ctrl + V)" } else { "Нет" },
        save_str
    );

    let wide_msg = to_wide_null(&result_msg);
    let wide_title = to_wide_null("ScrollCapture • Готово");

    let res = MessageBoxW(
        hwnd,
        wide_msg.as_ptr(),
        wide_title.as_ptr(),
        MB_YESNO | MB_ICONINFORMATION,
    );

    if res == IDYES && save_ok {
        let wide_path = to_wide_null(&save_str);
        let open_op = to_wide_null("open");
        ShellExecuteW(null_mut(), open_op.as_ptr(), wide_path.as_ptr(), null_mut(), null_mut(), SW_SHOWNORMAL);
    }
}

// Процедура для кнопок: убирает пунктирные рамки и плавно подсвечивает при наведении
unsafe extern "system" fn button_subclass_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_SETCURSOR => {
            SetCursor(LoadCursorW(null_mut(), IDC_HAND));
            return 1;
        }
        WM_SETFOCUS => {
            return 0;
        }
        WM_MOUSEMOVE => {
            let is_target = if hwnd == HWND_SCROLL {
                if !IS_SCROLL_HOVERED {
                    IS_SCROLL_HOVERED = true;
                    true
                } else { false }
            } else if hwnd == HWND_SNIP {
                if !IS_SNIP_HOVERED {
                    IS_SNIP_HOVERED = true;
                    true
                } else { false }
            } else if hwnd == HWND_FOLDER {
                if !IS_FOLDER_HOVERED {
                    IS_FOLDER_HOVERED = true;
                    true
                } else { false }
            } else if hwnd == HWND_AUTHOR {
                if !IS_AUTHOR_HOVERED {
                    IS_AUTHOR_HOVERED = true;
                    true
                } else { false }
            } else {
                false
            };

            if is_target {
                let mut tme: TRACKMOUSEEVENT = std::mem::zeroed();
                tme.cbSize = std::mem::size_of::<TRACKMOUSEEVENT>() as u32;
                tme.dwFlags = TME_LEAVE;
                tme.hwndTrack = hwnd;
                TrackMouseEvent(&mut tme);
                InvalidateRect(hwnd, null_mut(), TRUE);
            }
        }
        WM_MOUSELEAVE => {
            if hwnd == HWND_SCROLL {
                IS_SCROLL_HOVERED = false;
            } else if hwnd == HWND_SNIP {
                IS_SNIP_HOVERED = false;
            } else if hwnd == HWND_FOLDER {
                IS_FOLDER_HOVERED = false;
            } else if hwnd == HWND_AUTHOR {
                IS_AUTHOR_HOVERED = false;
            }
            InvalidateRect(hwnd, null_mut(), TRUE);
        }
        _ => {}
    }
    CallWindowProcW(OLD_BTN_PROC, hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn main_wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            let event = lparam as UINT;
            if event == WM_LBUTTONUP || event == WM_LBUTTONDBLCLK {
                ShowWindow(hwnd, SW_SHOW);
                SetForegroundWindow(hwnd);
            } else if event == WM_RBUTTONUP {
                let mut pt: POINT = std::mem::zeroed();
                GetCursorPos(&mut pt);

                let hmenu = CreatePopupMenu();
                AppendMenuW(hmenu, MF_STRING, TRAY_MENU_SHOW, to_wide_null("📜 Открыть окно ScrollCapture").as_ptr());
                AppendMenuW(hmenu, MF_SEPARATOR, 0, null_mut());
                AppendMenuW(hmenu, MF_STRING, TRAY_MENU_SCROLL, to_wide_null("📜 Снимок с прокруткой (Ctrl+Shift+S)").as_ptr());
                AppendMenuW(hmenu, MF_STRING, TRAY_MENU_SNIP, to_wide_null("✂ Обычный снимок области (Ctrl+Shift+X)").as_ptr());
                AppendMenuW(hmenu, MF_STRING, TRAY_MENU_FOLDER, to_wide_null("📁 Папка со снимками").as_ptr());
                AppendMenuW(hmenu, MF_SEPARATOR, 0, null_mut());
                AppendMenuW(hmenu, MF_STRING, TRAY_MENU_EXIT, to_wide_null("❌ Выход из программы").as_ptr());

                SetForegroundWindow(hwnd);
                TrackPopupMenu(hmenu, TPM_RIGHTALIGN | TPM_BOTTOMALIGN, pt.x, pt.y, 0, hwnd, null_mut());
                DestroyMenu(hmenu);
            }
            0
        }
        WM_HOTKEY => {
            let id = wparam as i32;
            if id == HOTKEY_SCROLL_ID {
                trigger_scroll_capture(hwnd);
            } else if id == HOTKEY_SNIP_ID || id == HOTKEY_SNIP_ALT_ID {
                trigger_regular_snip(hwnd);
            }
            0
        }
        WM_SYSCOMMAND => {
            if (wparam & 0xFFF0) == SC_MINIMIZE {
                ShowWindow(hwnd, SW_HIDE);
                show_tray_balloon(hwnd, "ScrollCapture свернут в трей", "Используйте Ctrl+Shift+S (скролл) или Ctrl+Shift+X (обычный снимок)");
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
            show_tray_balloon(hwnd, "ScrollCapture работает в трее", "Программа слушает горячие клавиши в фоне. Кликните правой кнопкой по иконке для выхода.");
            0
        }
        WM_CTLCOLORBTN => {
            static mut BG_BRUSH: HBRUSH = null_mut();
            if BG_BRUSH.is_null() {
                BG_BRUSH = CreateSolidBrush(COLOR_BG);
            }
            BG_BRUSH as LRESULT
        }
        WM_DRAWITEM => {
            let dis = lparam as *const DRAWITEMSTRUCT;
            if !dis.is_null() {
                let item = &*dis;
                let is_selected = (item.itemState & ODS_SELECTED) != 0;
                let hdc = item.hDC;
                let rc = item.rcItem;

                let fill_brush = CreateSolidBrush(COLOR_BG);
                FillRect(hdc, &rc, fill_brush);
                DeleteObject(fill_brush as *mut _);

                if item.CtlID == BTN_SCROLL_ID as u32 {
                    // --- Кнопка 1: Снимок с прокруткой (Royal Blue) ---
                    let font = CreateFontW(
                        14, 0, 0, 0, FW_BOLD as i32, 0, 0, 0,
                        DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
                        CLEARTYPE_QUALITY, DEFAULT_PITCH | FF_DONTCARE,
                        to_wide_null("Segoe UI").as_ptr(),
                    );
                    let old_font = SelectObject(hdc, font as *mut _);

                    let bg_color = if is_selected {
                        COLOR_BTN_PRIMARY_ACTIVE
                    } else if IS_SCROLL_HOVERED {
                        COLOR_BTN_PRIMARY_HOVER
                    } else {
                        COLOR_BTN_PRIMARY
                    };

                    let brush = CreateSolidBrush(bg_color);
                    let old_brush = SelectObject(hdc, brush as *mut _);
                    let pen = CreatePen(PS_SOLID as i32, 1, bg_color);
                    let old_pen = SelectObject(hdc, pen as *mut _);

                    RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, 16, 16);

                    SelectObject(hdc, old_brush);
                    DeleteObject(brush as *mut _);
                    SelectObject(hdc, old_pen);
                    DeleteObject(pen as *mut _);

                    SetBkMode(hdc, TRANSPARENT as i32);
                    SetTextColor(hdc, RGB(255, 255, 255));
                    let text = to_wide_null("📜  С ПРОКРУТКОЙ");
                    let mut r = rc;
                    DrawTextW(hdc, text.as_ptr(), -1, &mut r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                    SelectObject(hdc, old_font);
                    DeleteObject(font as *mut _);
                } else if item.CtlID == BTN_SNIP_ID as u32 {
                    // --- Кнопка 2: Обычный снимок области (Cyan / Sky Blue) ---
                    let font = CreateFontW(
                        14, 0, 0, 0, FW_BOLD as i32, 0, 0, 0,
                        DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
                        CLEARTYPE_QUALITY, DEFAULT_PITCH | FF_DONTCARE,
                        to_wide_null("Segoe UI").as_ptr(),
                    );
                    let old_font = SelectObject(hdc, font as *mut _);

                    let bg_color = if is_selected {
                        COLOR_BTN_SNIP_ACTIVE
                    } else if IS_SNIP_HOVERED {
                        COLOR_BTN_SNIP_HOVER
                    } else {
                        COLOR_BTN_SNIP
                    };

                    let brush = CreateSolidBrush(bg_color);
                    let old_brush = SelectObject(hdc, brush as *mut _);
                    let pen = CreatePen(PS_SOLID as i32, 1, bg_color);
                    let old_pen = SelectObject(hdc, pen as *mut _);

                    RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, 16, 16);

                    SelectObject(hdc, old_brush);
                    DeleteObject(brush as *mut _);
                    SelectObject(hdc, old_pen);
                    DeleteObject(pen as *mut _);

                    SetBkMode(hdc, TRANSPARENT as i32);
                    SetTextColor(hdc, RGB(255, 255, 255));
                    let text = to_wide_null("✂  ОБЫЧНЫЙ СНИМОК");
                    let mut r = rc;
                    DrawTextW(hdc, text.as_ptr(), -1, &mut r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                    SelectObject(hdc, old_font);
                    DeleteObject(font as *mut _);
                } else if item.CtlID == BTN_FOLDER_ID as u32 {
                    // --- Кнопка 3: Папка ---
                    let font = CreateFontW(
                        13, 0, 0, 0, FW_SEMIBOLD as i32, 0, 0, 0,
                        DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
                        CLEARTYPE_QUALITY, DEFAULT_PITCH | FF_DONTCARE,
                        to_wide_null("Segoe UI").as_ptr(),
                    );
                    let old_font = SelectObject(hdc, font as *mut _);

                    let (bg_color, border_color) = if is_selected {
                        (COLOR_BTN_SEC_HOVER, COLOR_BTN_SEC_BORDER_HOVER)
                    } else if IS_FOLDER_HOVERED {
                        (COLOR_BTN_SEC_HOVER, COLOR_BTN_SEC_BORDER_HOVER)
                    } else {
                        (COLOR_BTN_SEC_BG, COLOR_BTN_SEC_BORDER)
                    };

                    let brush = CreateSolidBrush(bg_color);
                    let old_brush = SelectObject(hdc, brush as *mut _);
                    let pen = CreatePen(PS_SOLID as i32, 1, border_color);
                    let old_pen = SelectObject(hdc, pen as *mut _);

                    RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, 16, 16);

                    SelectObject(hdc, old_brush);
                    DeleteObject(brush as *mut _);
                    SelectObject(hdc, old_pen);
                    DeleteObject(pen as *mut _);

                    SetBkMode(hdc, TRANSPARENT as i32);
                    SetTextColor(hdc, RGB(180, 186, 204));
                    let text = to_wide_null("📁  Папка");
                    let mut r = rc;
                    DrawTextW(hdc, text.as_ptr(), -1, &mut r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                    SelectObject(hdc, old_font);
                    DeleteObject(font as *mut _);
                } else if item.CtlID == BTN_AUTHOR_ID as u32 {
                    // --- Кнопка 4: Авторский бейдж Telegram ---
                    let font = CreateFontW(
                        14, 0, 0, 0, FW_SEMIBOLD as i32, 0, 0, 0,
                        DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
                        CLEARTYPE_QUALITY, DEFAULT_PITCH | FF_DONTCARE,
                        to_wide_null("Segoe UI").as_ptr(),
                    );
                    let old_font = SelectObject(hdc, font as *mut _);

                    let (bg_color, border_color) = if IS_AUTHOR_HOVERED || is_selected {
                        (COLOR_AUTHOR_HOVER_BG, COLOR_AUTHOR_TEXT)
                    } else {
                        (COLOR_AUTHOR_BG, COLOR_AUTHOR_BORDER)
                    };

                    let brush = CreateSolidBrush(bg_color);
                    let old_brush = SelectObject(hdc, brush as *mut _);
                    let pen = CreatePen(PS_SOLID as i32, 1, border_color);
                    let old_pen = SelectObject(hdc, pen as *mut _);

                    RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, 16, 16);

                    SelectObject(hdc, old_brush);
                    DeleteObject(brush as *mut _);
                    SelectObject(hdc, old_pen);
                    DeleteObject(pen as *mut _);

                    if let Some(ref icon) = TG_ICON {
                        let icon_x = rc.left + 12;
                        let icon_y = rc.top + (rc.bottom - rc.top - icon.height) / 2;
                        let blend = BLENDFUNCTION {
                            BlendOp: AC_SRC_OVER as u8,
                            BlendFlags: 0,
                            SourceConstantAlpha: 255,
                            AlphaFormat: AC_SRC_ALPHA as u8,
                        };
                        AlphaBlend(
                            hdc,
                            icon_x,
                            icon_y,
                            icon.width,
                            icon.height,
                            icon.h_dc,
                            0,
                            0,
                            icon.width,
                            icon.height,
                            blend,
                        );
                    }

                    SetBkMode(hdc, TRANSPARENT as i32);
                    SetTextColor(hdc, RGB(70, 170, 255));
                    let text = to_wide_null("@Carist");
                    let mut text_rc = RECT {
                        left: rc.left + 35,
                        top: rc.top,
                        right: rc.right - 8,
                        bottom: rc.bottom,
                    };
                    DrawTextW(hdc, text.as_ptr(), -1, &mut text_rc, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

                    SelectObject(hdc, old_font);
                    DeleteObject(font as *mut _);
                }
            }
            1
        }
        WM_COMMAND => {
            let id = wparam as usize & 0xFFFF;
            if id == BTN_SCROLL_ID || id == TRAY_MENU_SCROLL {
                trigger_scroll_capture(hwnd);
            } else if id == BTN_SNIP_ID || id == TRAY_MENU_SNIP {
                trigger_regular_snip(hwnd);
            } else if id == BTN_FOLDER_ID || id == TRAY_MENU_FOLDER {
                let mut dir = get_exe_dir();
                dir.push("Screenshots");
                let _ = fs::create_dir_all(&dir);
                let wide_dir = to_wide_null(&dir.to_string_lossy());
                let open_op = to_wide_null("explore");
                ShellExecuteW(null_mut(), open_op.as_ptr(), wide_dir.as_ptr(), null_mut(), null_mut(), SW_SHOWNORMAL);
            } else if id == BTN_AUTHOR_ID {
                let wide_url = to_wide_null(AUTHOR_URL);
                let open_op = to_wide_null("open");
                ShellExecuteW(null_mut(), open_op.as_ptr(), wide_url.as_ptr(), null_mut(), null_mut(), SW_SHOWNORMAL);
            } else if id == TRAY_MENU_SHOW {
                ShowWindow(hwnd, SW_SHOW);
                SetForegroundWindow(hwnd);
            } else if id == TRAY_MENU_EXIT {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);

            // Фон
            let bg_brush = CreateSolidBrush(COLOR_BG);
            FillRect(hdc, &ps.rcPaint, bg_brush);
            DeleteObject(bg_brush as *mut _);

            SetBkMode(hdc, TRANSPARENT as i32);

            // Карточка настроек и хоткеев
            let card_brush = CreateSolidBrush(COLOR_CARD_BG);
            let card_pen = CreatePen(PS_SOLID as i32, 1, COLOR_CARD_BORDER);
            let old_brush = SelectObject(hdc, card_brush as *mut _);
            let old_pen = SelectObject(hdc, card_pen as *mut _);

            RoundRect(hdc, 20, 142, 530, 264, 12, 12);

            SelectObject(hdc, old_brush);
            DeleteObject(card_brush as *mut _);
            SelectObject(hdc, old_pen);
            DeleteObject(card_pen as *mut _);

            // Заголовок окна
            let title_font = CreateFontW(
                22, 0, 0, 0, FW_BOLD as i32, 0, 0, 0,
                DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY, DEFAULT_PITCH | FF_DONTCARE,
                to_wide_null("Segoe UI").as_ptr(),
            );
            let old_font = SelectObject(hdc, title_font as *mut _);

            SetTextColor(hdc, RGB(70, 170, 255));
            let blue_dot = to_wide_null("● ");
            TextOutW(hdc, 22, 18, blue_dot.as_ptr(), (blue_dot.len() - 1) as i32);

            SetTextColor(hdc, RGB(240, 244, 255));
            let title = to_wide_null("ScrollCapture");
            TextOutW(hdc, 44, 18, title.as_ptr(), (title.len() - 1) as i32);

            let sub_font = CreateFontW(
                13, 0, 0, 0, FW_NORMAL as i32, 0, 0, 0,
                DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY, DEFAULT_PITCH | FF_DONTCARE,
                to_wide_null("Segoe UI").as_ptr(),
            );
            SelectObject(hdc, sub_font as *mut _);
            DeleteObject(title_font as *mut _);

            SetTextColor(hdc, RGB(140, 146, 168));
            let subtitle = to_wide_null("Скриншоты с прокруткой и выделенной области • Трей и фоновые хоткеи");
            TextOutW(hdc, 44, 46, subtitle.as_ptr(), (subtitle.len() - 1) as i32);

            // Блок горячих клавиш
            let hk_header_font = CreateFontW(
                13, 0, 0, 0, FW_BOLD as i32, 0, 0, 0,
                DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY, DEFAULT_PITCH | FF_DONTCARE,
                to_wide_null("Segoe UI").as_ptr(),
            );
            SelectObject(hdc, hk_header_font as *mut _);
            DeleteObject(sub_font as *mut _);

            SetTextColor(hdc, RGB(70, 170, 255));
            let hk_title = to_wide_null("⌨  Горячие клавиши (работают в трее и любом окне):");
            TextOutW(hdc, 34, 152, hk_title.as_ptr(), (hk_title.len() - 1) as i32);

            let hk_body_font = CreateFontW(
                13, 0, 0, 0, FW_NORMAL as i32, 0, 0, 0,
                DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY, DEFAULT_PITCH | FF_DONTCARE,
                to_wide_null("Segoe UI").as_ptr(),
            );
            SelectObject(hdc, hk_body_font as *mut _);
            DeleteObject(hk_header_font as *mut _);

            // Хоткей 1: Ctrl + Shift + S
            SetTextColor(hdc, RGB(255, 255, 255));
            let k1 = to_wide_null("[Ctrl + Shift + S]");
            TextOutW(hdc, 34, 174, k1.as_ptr(), (k1.len() - 1) as i32);

            SetTextColor(hdc, RGB(180, 186, 204));
            let d1 = to_wide_null("— Снимок с автопрокруткой (длинный)");
            TextOutW(hdc, 160, 174, d1.as_ptr(), (d1.len() - 1) as i32);

            // Хоткей 2: Ctrl + Shift + X
            SetTextColor(hdc, RGB(255, 255, 255));
            let k2 = to_wide_null("[Ctrl + Shift + X]");
            TextOutW(hdc, 34, 196, k2.as_ptr(), (k2.len() - 1) as i32);

            SetTextColor(hdc, RGB(180, 186, 204));
            let d2 = to_wide_null("— Обычный быстрый снимок области");
            TextOutW(hdc, 160, 196, d2.as_ptr(), (d2.len() - 1) as i32);

            // Хоткей 3: Esc
            SetTextColor(hdc, RGB(255, 255, 255));
            let k3 = to_wide_null("[Esc]");
            TextOutW(hdc, 34, 218, k3.as_ptr(), (k3.len() - 1) as i32);

            SetTextColor(hdc, RGB(180, 186, 204));
            let d3 = to_wide_null("— Остановить автоскролл и сохранить / Отмена");
            TextOutW(hdc, 82, 218, d3.as_ptr(), (d3.len() - 1) as i32);

            // Подсказка трея
            SetTextColor(hdc, RGB(56, 189, 248));
            let d4 = to_wide_null("💡 При сворачивании или закрытии окно прячется в трей");
            TextOutW(hdc, 34, 240, d4.as_ptr(), (d4.len() - 1) as i32);

            // Футер слева
            let footer_font = CreateFontW(
                12, 0, 0, 0, FW_NORMAL as i32, 0, 0, 0,
                DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY, DEFAULT_PITCH | FF_DONTCARE,
                to_wide_null("Segoe UI").as_ptr(),
            );
            SelectObject(hdc, footer_font as *mut _);
            DeleteObject(hk_body_font as *mut _);

            SetTextColor(hdc, RGB(100, 108, 128));
            let footer_text = to_wide_null("ScrollCapture v1.3 • Rust Native Edition");
            TextOutW(hdc, 22, 282, footer_text.as_ptr(), (footer_text.len() - 1) as i32);

            SelectObject(hdc, old_font);
            DeleteObject(footer_font as *mut _);

            EndPaint(hwnd, &ps);
            0
        }
        WM_DESTROY => {
            UnregisterHotKey(hwnd, HOTKEY_SCROLL_ID);
            UnregisterHotKey(hwnd, HOTKEY_SNIP_ID);
            UnregisterHotKey(hwnd, HOTKEY_SNIP_ALT_ID);

            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1;
            Shell_NotifyIconW(NIM_DELETE, &mut nid);

            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn main() {
    input::enable_dpi_awareness();

    unsafe {
        let class_name = to_wide_null("ScrollCaptureMainWindow");
        let hinstance = winapi::um::libloaderapi::GetModuleHandleW(null_mut());

        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(main_wnd_proc);
        wc.hInstance = hinstance;
        wc.lpszClassName = class_name.as_ptr();
        wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
        let bg_brush = CreateSolidBrush(COLOR_BG);
        wc.hbrBackground = bg_brush as *mut _;

        RegisterClassW(&wc);

        let window_title = to_wide_null("ScrollCapture • by @Carist");
        let w = 570;
        let h = 360;
        let sw = GetSystemMetrics(SM_CXSCREEN);
        let sh = GetSystemMetrics(SM_CYSCREEN);
        let x = (sw - w) / 2;
        let y = (sh - h) / 2;

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            x,
            y,
            w,
            h,
            null_mut(),
            null_mut(),
            hinstance,
            null_mut(),
        );

        if hwnd.is_null() {
            return;
        }

        // Регистрируем глобальные системные горячие клавиши
        // 1. Снимок с прокруткой: Ctrl + Shift + S
        RegisterHotKey(hwnd, HOTKEY_SCROLL_ID, (MOD_CONTROL | MOD_SHIFT) as UINT, 0x53);
        // 2. Обычный снимок области: Ctrl + Shift + X и Ctrl + Shift + A
        RegisterHotKey(hwnd, HOTKEY_SNIP_ID, (MOD_CONTROL | MOD_SHIFT) as UINT, 0x58);
        RegisterHotKey(hwnd, HOTKEY_SNIP_ALT_ID, (MOD_CONTROL | MOD_SHIFT) as UINT, 0x41);

        // Загружаем и устанавливаем иконки
        let mut h_small = null_mut();
        if let Ok(raw_icon) = image::load_from_memory(include_bytes!("../app_icon.png")) {
            let rgba = raw_icon.to_rgba8();
            let icon_32 = image::imageops::resize(&rgba, 32, 32, image::imageops::FilterType::Lanczos3);
            let icon_16 = image::imageops::resize(&rgba, 16, 16, image::imageops::FilterType::Lanczos3);
            let h_big = create_hicon_from_rgba(&icon_32);
            h_small = create_hicon_from_rgba(&icon_16);
            if !h_big.is_null() {
                SendMessageW(hwnd, WM_SETICON, ICON_BIG as WPARAM, h_big as LPARAM);
            }
            if !h_small.is_null() {
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL as WPARAM, h_small as LPARAM);
            }
        }

        // Создаем иконку в системном трее Windows
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = h_small;
        let tip = to_wide_null("ScrollCapture • by @Carist");
        for i in 0..tip.len().min(127) {
            nid.szTip[i] = tip[i];
        }
        Shell_NotifyIconW(NIM_ADD, &mut nid);

        // Инициализируем иконку Telegram
        let screen_dc = GetDC(hwnd);
        init_telegram_icon(screen_dc);
        ReleaseDC(hwnd, screen_dc);

        let btn_class = to_wide_null("BUTTON");

        // --- Кнопка 1: "С ПРОКРУТКОЙ" (Royal Blue) ---
        let btn_scroll_text = to_wide_null("📜  С ПРОКРУТКОЙ");
        HWND_SCROLL = CreateWindowExW(
            0,
            btn_class.as_ptr(),
            btn_scroll_text.as_ptr(),
            WS_VISIBLE | WS_CHILD | BS_OWNERDRAW,
            20,
            74,
            220,
            52,
            hwnd,
            BTN_SCROLL_ID as HMENU,
            hinstance,
            null_mut(),
        );

        // --- Кнопка 2: "ОБЫЧНЫЙ СНИМОК" (Cyan / Sky Blue) ---
        let btn_snip_text = to_wide_null("✂  ОБЫЧНЫЙ СНИМОК");
        HWND_SNIP = CreateWindowExW(
            0,
            btn_class.as_ptr(),
            btn_snip_text.as_ptr(),
            WS_VISIBLE | WS_CHILD | BS_OWNERDRAW,
            250,
            74,
            180,
            52,
            hwnd,
            BTN_SNIP_ID as HMENU,
            hinstance,
            null_mut(),
        );

        // --- Кнопка 3: "Папка" (Dark Slate) ---
        let folder_text = to_wide_null("📁  Папка");
        HWND_FOLDER = CreateWindowExW(
            0,
            btn_class.as_ptr(),
            folder_text.as_ptr(),
            WS_VISIBLE | WS_CHILD | BS_OWNERDRAW,
            440,
            74,
            90,
            52,
            hwnd,
            BTN_FOLDER_ID as HMENU,
            hinstance,
            null_mut(),
        );

        // --- Кнопка 4: Бейдж автора с официальной иконкой Telegram ---
        let author_text = to_wide_null("✈  @Carist");
        HWND_AUTHOR = CreateWindowExW(
            0,
            btn_class.as_ptr(),
            author_text.as_ptr(),
            WS_VISIBLE | WS_CHILD | BS_OWNERDRAW,
            420,
            276,
            110,
            32,
            hwnd,
            BTN_AUTHOR_ID as HMENU,
            hinstance,
            null_mut(),
        );

        // Сабклассинг кнопок для идеальных скруглений и hover-подсветки
        let subclass_fn = button_subclass_proc as *const () as usize as isize;
        OLD_BTN_PROC = Some(std::mem::transmute(SetWindowLongPtrW(
            HWND_SCROLL,
            GWLP_WNDPROC,
            subclass_fn,
        )));
        SetWindowLongPtrW(HWND_SNIP, GWLP_WNDPROC, subclass_fn);
        SetWindowLongPtrW(HWND_FOLDER, GWLP_WNDPROC, subclass_fn);
        SetWindowLongPtrW(HWND_AUTHOR, GWLP_WNDPROC, subclass_fn);

        // Всплывающая подсказка ToolTip для автора
        let tooltip_class = to_wide_null("tooltips_class32");
        let hwnd_tip = CreateWindowExW(
            WS_EX_TOPMOST,
            tooltip_class.as_ptr(),
            null_mut(),
            WS_POPUP | TTS_NOPREFIX | TTS_ALWAYSTIP,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            hwnd,
            null_mut(),
            hinstance,
            null_mut(),
        );

        if !hwnd_tip.is_null() {
            let mut ti: TOOLINFOW = std::mem::zeroed();
            ti.cbSize = std::mem::size_of::<TOOLINFOW>() as u32;
            ti.uFlags = TTF_SUBCLASS | TTF_IDISHWND;
            ti.hwnd = hwnd;
            ti.uId = HWND_AUTHOR as usize;
            let tip_text = to_wide_null("Открыть Telegram автора: https://t.me/Carist");
            ti.lpszText = tip_text.as_ptr() as *mut _;
            SendMessageW(hwnd_tip, TTM_ADDTOOLW, 0, &ti as *const _ as LPARAM);
        }

        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
