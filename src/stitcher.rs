use image::{ImageBuffer, Rgb};

#[allow(dead_code)]
pub struct StitchResult {
    pub shift_y: u32,
    pub confidence: f32,
}

/// Вычисляет вертикальный сдвиг между двумя кадрами с попиксельной точностью
pub fn find_vertical_shift(
    prev: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    curr: &ImageBuffer<Rgb<u8>, Vec<u8>>,
) -> StitchResult {
    let (w, h) = prev.dimensions();
    if w < 40 || h < 40 {
        return StitchResult { shift_y: 0, confidence: 0.0 };
    }

    // Исключаем правую область полосы прокрутки (6-8%) и левый край
    let x_start = (w as f32 * 0.05) as u32;
    let x_end = (w - (w as f32 * 0.07).max(28.0) as u32).min(w);
    if x_end <= x_start {
        return StitchResult { shift_y: 0, confidence: 0.0 };
    }

    // Шаблон берем из нижней части предыдущего кадра (от 50% до 80% высоты)
    let t_top = (h as f32 * 0.50) as u32;
    let t_bottom = (h as f32 * 0.82) as u32;
    let t_h = t_bottom - t_top;

    // Шаг по X: 2px (для скорости), шаг по Y: 1px (для абсолютной точности склейки)
    let sample_step_x = 2;
    let sample_step_y = 1;

    // Извлекаем яркость (grayscale) шаблона
    let mut template_gray = Vec::with_capacity(((t_h * (x_end - x_start)) / 2) as usize);
    for ty in (t_top..t_bottom).step_by(sample_step_y) {
        for tx in (x_start..x_end).step_by(sample_step_x) {
            let p = prev.get_pixel(tx, ty);
            let gray = ((p[0] as u32 * 54 + p[1] as u32 * 183 + p[2] as u32 * 19) >> 8) as i32;
            template_gray.push(gray);
        }
    }

    let num_samples = template_gray.len() as f32;
    if num_samples == 0.0 {
        return StitchResult { shift_y: 0, confidence: 0.0 };
    }

    let mut best_shift = 0;
    let mut min_diff = f32::MAX;

    // Ищем вертикальный сдвиг от 1px до t_top
    let max_shift = t_top;

    for shift in 1..max_shift {
        let curr_y_start = t_top - shift;
        let mut diff_sum: i64 = 0;
        let mut idx = 0;

        for cy in (curr_y_start..(curr_y_start + t_h)).step_by(sample_step_y) {
            for cx in (x_start..x_end).step_by(sample_step_x) {
                let p = curr.get_pixel(cx, cy);
                let gray = ((p[0] as u32 * 54 + p[1] as u32 * 183 + p[2] as u32 * 19) >> 8) as i32;
                let d = (gray - template_gray[idx]).abs();
                diff_sum += d as i64;
                idx += 1;
            }
        }

        let avg_diff = diff_sum as f32 / num_samples;
        if avg_diff < min_diff {
            min_diff = avg_diff;
            best_shift = shift;
        }
    }

    let confidence = (1.0 - (min_diff / 255.0)).max(0.0);

    StitchResult {
        shift_y: best_shift,
        confidence,
    }
}

/// Бесшовно приклеивает новую полосу пикселей вниз
pub fn append_slice(
    canvas: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    curr: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    shift_y: u32,
) {
    if shift_y == 0 {
        return;
    }

    let (w, h) = curr.dimensions();
    let current_h = canvas.height();
    let new_h = current_h + shift_y;

    let mut new_canvas = ImageBuffer::new(w, new_h);

    // Копируем исходный холст
    for y in 0..current_h {
        for x in 0..w {
            new_canvas.put_pixel(x, y, *canvas.get_pixel(x, y));
        }
    }

    // Приклеиваем нижнюю сдвинутую полосу из curr
    let src_start_y = h - shift_y;
    for dy in 0..shift_y {
        let src_y = src_start_y + dy;
        let dst_y = current_h + dy;
        for x in 0..w {
            new_canvas.put_pixel(x, dst_y, *curr.get_pixel(x, src_y));
        }
    }

    *canvas = new_canvas;
}
