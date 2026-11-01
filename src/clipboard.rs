use image::{ImageBuffer, Rgb};
use std::ptr::{null_mut, copy_nonoverlapping};
use winapi::um::winbase::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

/// Копирует изображение в буфер обмена Windows в формате CF_DIB (совместимо со всеми программами)
pub fn copy_image_to_clipboard(img: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> bool {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return false;
    }

    let header_size = std::mem::size_of::<BITMAPINFOHEADER>();
    let row_stride = ((w * 3 + 3) / 4) * 4;
    let data_size = (row_stride * h) as usize;
    let total_size = header_size + data_size;

    unsafe {
        let h_mem = GlobalAlloc(GMEM_MOVEABLE, total_size);
        if h_mem.is_null() {
            return false;
        }

        let p_mem = GlobalLock(h_mem) as *mut u8;
        if p_mem.is_null() {
            GlobalFree(h_mem);
            return false;
        }

        // Заполняем BITMAPINFOHEADER (Bottom-Up)
        let mut bmi: BITMAPINFOHEADER = std::mem::zeroed();
        bmi.biSize = header_size as u32;
        bmi.biWidth = w as i32;
        bmi.biHeight = h as i32; // Bottom-up для максимальной совместимости с CF_DIB
        bmi.biPlanes = 1;
        bmi.biBitCount = 24;
        bmi.biCompression = BI_RGB;
        bmi.biSizeImage = data_size as u32;

        copy_nonoverlapping(&bmi as *const _ as *const u8, p_mem, header_size);

        // Заполняем пиксельные данные BGR снизу вверх
        let p_data = p_mem.add(header_size);
        for row in 0..h {
            let src_y = h - 1 - row; // Снизу вверх
            let dst_offset = (row * row_stride) as usize;
            for col in 0..w {
                let px = img.get_pixel(col, src_y);
                let px_offset = dst_offset + (col * 3) as usize;
                *p_data.add(px_offset) = px[2];     // B
                *p_data.add(px_offset + 1) = px[1]; // G
                *p_data.add(px_offset + 2) = px[0]; // R
            }
        }

        GlobalUnlock(h_mem);

        if OpenClipboard(null_mut()) == 0 {
            GlobalFree(h_mem);
            return false;
        }

        EmptyClipboard();
        let res = SetClipboardData(CF_DIB, h_mem);
        CloseClipboard();

        !res.is_null()
    }
}
