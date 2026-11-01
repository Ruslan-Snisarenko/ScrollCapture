use std::cell::RefCell;
use std::ptr::null_mut;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

#[derive(Clone, Copy, Debug)]
pub struct SelectedRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

struct SelectorState {
    start_x: i32,
    start_y: i32,
    cur_x: i32,
    cur_y: i32,
    is_dragging: bool,
    result: Option<SelectedRect>,
    v_x: i32,
    v_y: i32,
}

thread_local! {
    static STATE: RefCell<SelectorState> = RefCell::new(SelectorState {
        start_x: 0,
        start_y: 0,
        cur_x: 0,
        cur_y: 0,
        is_dragging: false,
        result: None,
        v_x: 0,
        v_y: 0,
    });
}

unsafe extern "system" fn selector_wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_LBUTTONDOWN => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            STATE.with(|s| {
                let mut st = s.borrow_mut();
                st.start_x = x;
                st.start_y = y;
                st.cur_x = x;
                st.cur_y = y;
                st.is_dragging = true;
            });
            SetCapture(hwnd);
            InvalidateRect(hwnd, null_mut(), TRUE);
            0
        }
        WM_MOUSEMOVE => {
            let is_dragging = STATE.with(|s| s.borrow().is_dragging);
            if is_dragging {
                let x = (lparam & 0xFFFF) as i16 as i32;
                let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
                STATE.with(|s| {
                    let mut st = s.borrow_mut();
                    st.cur_x = x;
                    st.cur_y = y;
                });
                InvalidateRect(hwnd, null_mut(), TRUE);
            }
            0
        }
        WM_LBUTTONUP => {
            ReleaseCapture();
            let mut res_rect = None;
            STATE.with(|s| {
                let mut st = s.borrow_mut();
                st.is_dragging = false;

                let min_x = st.start_x.min(st.cur_x);
                let min_y = st.start_y.min(st.cur_y);
                let w = (st.cur_x - st.start_x).abs() as u32;
                let h = (st.cur_y - st.start_y).abs() as u32;

                if w > 30 && h > 30 {
                    let real_x = min_x + st.v_x;
                    let real_y = min_y + st.v_y;
                    res_rect = Some(SelectedRect {
                        x: real_x,
                        y: real_y,
                        width: w,
                        height: h,
                    });
                    st.result = res_rect;
                }
            });
            DestroyWindow(hwnd);
            0
        }
        WM_KEYDOWN => {
            if wparam == VK_ESCAPE as usize {
                STATE.with(|s| s.borrow_mut().result = None);
                DestroyWindow(hwnd);
            }
            0
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);

            STATE.with(|s| {
                let st = s.borrow();
                SetBkMode(hdc, TRANSPARENT as i32);

                if st.is_dragging {
                    let min_x = st.start_x.min(st.cur_x);
                    let min_y = st.start_y.min(st.cur_y);
                    let max_x = st.start_x.max(st.cur_x);
                    let max_y = st.start_y.max(st.cur_y);
                    let w = max_x - min_x;
                    let h = max_y - min_y;

                    // Яркая синяя рамка выделения
                    let pen = CreatePen(PS_SOLID as i32, 2, RGB(37, 99, 235));
                    let old_pen = SelectObject(hdc, pen as *mut _);
                    let brush = GetStockObject(HOLLOW_BRUSH as i32);
                    let old_brush = SelectObject(hdc, brush);

                    Rectangle(hdc, min_x, min_y, max_x, max_y);

                    // Текст с размером
                    SetTextColor(hdc, RGB(255, 255, 255));
                    let info = format!("{} × {} px", w, h);
                    let wide_info: Vec<u16> = info.encode_utf16().chain(std::iter::once(0)).collect();
                    TextOutW(hdc, min_x + 10, min_y + 10, wide_info.as_ptr(), (wide_info.len() - 1) as i32);

                    SelectObject(hdc, old_brush);
                    SelectObject(hdc, old_pen);
                    DeleteObject(pen as *mut _);
                } else {
                    SetTextColor(hdc, RGB(255, 255, 255));
                    let hint = "Выделите область прокрутки мышью | [Esc] — Отмена";
                    let wide_hint: Vec<u16> = hint.encode_utf16().chain(std::iter::once(0)).collect();
                    TextOutW(hdc, 50, 50, wide_hint.as_ptr(), (wide_hint.len() - 1) as i32);
                }
            });

            EndPaint(hwnd, &ps);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn select_region() -> Option<SelectedRect> {
    unsafe {
        let v_x = GetSystemMetrics(76); // SM_XVIRTUALSCREEN
        let v_y = GetSystemMetrics(77); // SM_YVIRTUALSCREEN
        let v_w = GetSystemMetrics(78); // SM_CXVIRTUALSCREEN
        let v_h = GetSystemMetrics(79); // SM_CYVIRTUALSCREEN

        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.v_x = v_x;
            st.v_y = v_y;
            st.result = None;
            st.is_dragging = false;
        });

        let class_name: Vec<u16> = "ScrollCaptureSelector\0".encode_utf16().collect();
        let hinstance = winapi::um::libloaderapi::GetModuleHandleW(null_mut());

        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(selector_wnd_proc);
        wc.hInstance = hinstance;
        wc.lpszClassName = class_name.as_ptr();
        wc.hCursor = LoadCursorW(null_mut(), IDC_CROSS);
        wc.hbrBackground = GetStockObject(BLACK_BRUSH as i32) as *mut _;

        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_LAYERED,
            class_name.as_ptr(),
            null_mut(),
            WS_POPUP,
            v_x,
            v_y,
            v_w,
            v_h,
            null_mut(),
            null_mut(),
            hinstance,
            null_mut(),
        );

        if hwnd.is_null() {
            return None;
        }

        // Полупрозрачность
        SetLayeredWindowAttributes(hwnd, 0, 95, LWA_ALPHA);
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        UnregisterClassW(class_name.as_ptr(), hinstance);

        STATE.with(|s| s.borrow().result)
    }
}
