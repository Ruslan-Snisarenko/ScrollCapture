use std::time::Duration;
use winapi::um::winuser::*;

/// Включает поддержку высокого разрешения экрана (Per-Monitor High DPI Aware)
pub fn enable_dpi_awareness() {
    unsafe {
        SetProcessDPIAware();
    }
}

/// Плавная эмуляция прокрутки колеса мыши с микро-шагами для естественной плавной анимации
pub fn smooth_scroll_down(cx: i32, cy: i32, clicks: i32) {
    unsafe {
        SetCursorPos(cx, cy);

        // Прокручиваем каждый щелчок с короткой паузой 30 мс, чтобы анимация браузера была плавной
        for _ in 0..clicks {
            mouse_event(MOUSEEVENTF_WHEEL, 0, 0, (-120i32) as u32, 0);
            std::thread::sleep(Duration::from_millis(30));
        }
    }
}

/// Проверяет, нажата ли в данный момент клавиша Escape
pub fn is_esc_pressed() -> bool {
    unsafe {
        (GetAsyncKeyState(VK_ESCAPE) as u16 & 0x8000) != 0
    }
}
