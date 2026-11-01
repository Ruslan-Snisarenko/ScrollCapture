use image::{ImageBuffer, Rgb};
use std::ptr::null_mut;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

/// Захватывает заданную прямоугольную область экрана через Windows GDI
pub fn capture_screen_rect(x: i32, y: i32, w: u32, h: u32) -> Option<ImageBuffer<Rgb<u8>, Vec<u8>>> {
    if w == 0 || h == 0 {
        return None;
    }

    unsafe {
        let h_screen_dc = GetDC(null_mut());
        if h_screen_dc.is_null() {
            return None;
        }

        let h_mem_dc = CreateCompatibleDC(h_screen_dc);
        if h_mem_dc.is_null() {
            ReleaseDC(null_mut(), h_screen_dc);
            return None;
        }

        let h_bmp = CreateCompatibleBitmap(h_screen_dc, w as i32, h as i32);
        if h_bmp.is_null() {
            DeleteDC(h_mem_dc);
            ReleaseDC(null_mut(), h_screen_dc);
            return None;
        }

        SelectObject(h_mem_dc, h_bmp as *mut _);

        // Копируем экран в память
        BitBlt(h_mem_dc, 0, 0, w as i32, h as i32, h_screen_dc, x, y, SRCCOPY);

        // Выравнивание строки GDI: кратно 4 байтам
        let row_stride = ((w * 3 + 3) / 4) * 4;
        let buf_size = (row_stride * h) as usize;
        let mut raw_buffer = vec![0u8; buf_size];

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w as i32;
        bmi.bmiHeader.biHeight = -(h as i32); // Сверху вниз (top-down)
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 24;
        bmi.bmiHeader.biCompression = BI_RGB;

        let lines = GetDIBits(
            h_mem_dc,
            h_bmp,
            0,
            h,
            raw_buffer.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // Освобождаем дескрипторы GDI
        DeleteObject(h_bmp as *mut _);
        DeleteDC(h_mem_dc);
        ReleaseDC(null_mut(), h_screen_dc);

        if lines != h as i32 {
            return None;
        }

        // Конвертируем BGR (GDI) в RGB (ImageBuffer)
        let mut img = ImageBuffer::new(w, h);
        for row in 0..h {
            let row_offset = (row * row_stride) as usize;
            for col in 0..w {
                let px_offset = row_offset + (col * 3) as usize;
                let b = raw_buffer[px_offset];
                let g = raw_buffer[px_offset + 1];
                let r = raw_buffer[px_offset + 2];
                img.put_pixel(col, row, Rgb([r, g, b]));
            }
        }

        Some(img)
    }
}
