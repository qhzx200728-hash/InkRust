// Công cụ Văn bản (Text Box – Bước 5.4 - P2):
// - Tạo mới bằng click (mặc định 280px) hoặc kéo ngang đặt độ rộng khung; nhấp lên hộp có sẵn để sửa tiếp.
// - Hỗ trợ: Kiểu chữ (Sans/Serif/Mono), Cỡ chữ, In đậm, In nghiêng, Căn lề (Left/Center/Right),
//   Danh sách (None / Bullet • / Numbered 1.), Bảng màu chữ; giữ sắc nét theo mức zoom & 100% dấu tiếng Việt.

use eframe::egui::{
    self, Align, Color32, FontId, Painter, Pos2, Rect, pos2, text::LayoutJob, text::TextFormat,
    vec2,
};
use inknote_core::{TextAlign, TextFontFamily, TextListMode, TextObject};

use crate::{lasso::LassoInteraction, theme};

pub const DEFAULT_TEXT_BOX_WIDTH: f64 = 280.0;
pub const MIN_TEXT_BOX_WIDTH: f64 = 80.0;

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyleState {
    pub font_family: TextFontFamily,
    pub font_size: f64,
    pub bold: bool,
    pub italic: bool,
    pub align: TextAlign,
    pub list_mode: TextListMode,
    pub color: [u8; 4],
}

/// Trạng thái hộp văn bản đang mở trình soạn thảo inline ngay trên canvas.
#[derive(Clone, Debug)]
pub struct ActiveTextEdit {
    pub page_index: usize,
    pub original_snapshot: Option<TextObject>,
    pub draft: TextObject,
    pub needs_focus: bool,
    pub area_rect: Rect,
    pub prev_text: String,
    pub cursor_override: Option<usize>,
}

impl ActiveTextEdit {
    pub fn new_blank(
        page_index: usize,
        id: u64,
        pos: [f64; 2],
        width: f64,
        style: TextStyleState,
    ) -> Self {
        let mut initial_text = String::new();
        apply_list_mode_to_text(&mut initial_text, style.list_mode);
        let cursor_override = if initial_text.is_empty() {
            None
        } else {
            Some(initial_text.chars().count())
        };
        let min_h = (style.font_size * 1.6).max(28.0);
        let draft = TextObject {
            id,
            pos,
            size: [width.max(MIN_TEXT_BOX_WIDTH), min_h],
            text: initial_text.clone(),
            font_family: style.font_family,
            font_size: style.font_size,
            bold: style.bold,
            italic: style.italic,
            align: style.align,
            list_mode: style.list_mode,
            color: style.color,
        };
        Self {
            page_index,
            original_snapshot: None,
            draft,
            needs_focus: true,
            area_rect: Rect::NOTHING,
            prev_text: initial_text,
            cursor_override,
        }
    }

    pub fn from_existing(page_index: usize, obj: TextObject) -> Self {
        let prev = obj.text.clone();
        Self {
            page_index,
            original_snapshot: Some(obj.clone()),
            draft: obj,
            needs_focus: true,
            area_rect: Rect::NOTHING,
            prev_text: prev,
            cursor_override: None,
        }
    }
}

pub fn egui_align(align: TextAlign) -> Align {
    match align {
        TextAlign::Left => Align::LEFT,
        TextAlign::Center => Align::Center,
        TextAlign::Right => Align::RIGHT,
    }
}

pub fn egui_align2(align: TextAlign) -> egui::Align2 {
    match align {
        TextAlign::Left => egui::Align2::LEFT_TOP,
        TextAlign::Center => egui::Align2::CENTER_TOP,
        TextAlign::Right => egui::Align2::RIGHT_TOP,
    }
}

/// Gỡ bỏ ký hiệu đầu dòng danh sách (`• ` hoặc `1. `) nếu có trên một dòng.
pub fn strip_list_prefix(line: &str) -> &str {
    if let Some(rest) = line.strip_prefix("• ") {
        return rest;
    }
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && i + 1 < bytes.len() && bytes[i] == b'.' && bytes[i + 1] == b' ' {
        return &line[i + 2..];
    }
    line
}

/// Kiểm tra nội dung văn bản có thực sự rỗng hay không (bỏ qua tiền tố danh sách trống).
pub fn is_text_effectively_empty(text: &str) -> bool {
    text.split('\n')
        .all(|line| strip_list_prefix(line).trim().is_empty())
}

/// Chuyển đổi toàn bộ các dòng trong hộp văn bản sang kiểu danh sách mới.
pub fn apply_list_mode_to_text(text: &mut String, mode: TextListMode) {
    if text.is_empty() {
        match mode {
            TextListMode::None => {}
            TextListMode::Bullet => {
                *text = "• ".to_string();
            }
            TextListMode::Numbered => {
                *text = "1. ".to_string();
            }
        }
        return;
    }
    let lines: Vec<String> = text
        .split('\n')
        .enumerate()
        .map(|(i, line)| {
            let core = strip_list_prefix(line);
            match mode {
                TextListMode::None => core.to_string(),
                TextListMode::Bullet => format!("• {core}"),
                TextListMode::Numbered => format!("{}. {core}", i + 1),
            }
        })
        .collect();
    *text = lines.join("\n");
}

/// Tự động nối tiếp đầu mục danh sách khi nhấn Enter trong `Bullet` / `Numbered`.
/// Trả về vị trí con trỏ (tính theo số ký tự) mới nếu có chèn tiền tố.
pub fn continue_list_on_newline(
    prev_text: &str,
    curr_text: &mut String,
    mode: TextListMode,
    cursor_char_idx: usize,
) -> Option<usize> {
    if mode == TextListMode::None || cursor_char_idx == 0 {
        return None;
    }
    let prev_nl = prev_text.chars().filter(|&c| c == '\n').count();
    let curr_nl = curr_text.chars().filter(|&c| c == '\n').count();
    if curr_nl != prev_nl + 1 {
        return None;
    }
    let byte_pos = curr_text
        .char_indices()
        .nth(cursor_char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(curr_text.len());
    if byte_pos == 0 || !curr_text[..byte_pos].ends_with('\n') {
        return None;
    }

    let before_nl = &curr_text[..byte_pos - 1];
    let prev_line_start = before_nl.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let prev_line = &before_nl[prev_line_start..];
    if strip_list_prefix(prev_line).trim().is_empty() && prev_line != strip_list_prefix(prev_line) {
        // Nhấn Enter trên dòng danh sách trống -> xoá ký hiệu danh sách ở dòng trên
        let prefix_len = prev_line.len() - strip_list_prefix(prev_line).len();
        let removed_chars = prev_line[..prefix_len].chars().count();
        curr_text.drain(prev_line_start..prev_line_start + prefix_len);
        return Some(cursor_char_idx.saturating_sub(removed_chars));
    }

    let line_number = before_nl.split('\n').count() + 1;
    let prefix = match mode {
        TextListMode::None => return None,
        TextListMode::Bullet => "• ".to_string(),
        TextListMode::Numbered => format!("{line_number}. "),
    };
    let added_chars = prefix.chars().count();
    curr_text.insert_str(byte_pos, &prefix);

    if mode == TextListMode::Numbered {
        renumber_lines(curr_text);
    }
    Some(cursor_char_idx + added_chars)
}

fn renumber_lines(text: &mut String) {
    let updated: Vec<String> = text
        .split('\n')
        .enumerate()
        .map(|(i, line)| format!("{}. {}", i + 1, strip_list_prefix(line)))
        .collect();
    *text = updated.join("\n");
}

pub fn build_text_layout_job(obj: &TextObject, font_size_px: f32, wrap_width_px: f32) -> LayoutJob {
    let color =
        Color32::from_rgba_unmultiplied(obj.color[0], obj.color[1], obj.color[2], obj.color[3]);
    let family = theme::text_font_family(obj.font_family, obj.bold, obj.italic);
    let mut job = LayoutJob::single_section(
        obj.text.clone(),
        TextFormat {
            font_id: FontId::new(font_size_px, family),
            color,
            ..Default::default()
        },
    );
    job.wrap.max_width = wrap_width_px.max(16.0);
    job.halign = egui_align(obj.align);
    job
}

/// Đo kích thước tự nhiên của hộp văn bản theo toạ độ trang (`zoom = 1.0`).
pub fn measure_text_page_size(ctx: &egui::Context, obj: &TextObject) -> [f64; 2] {
    let wrap_w = (obj.size[0] as f32).max(MIN_TEXT_BOX_WIDTH as f32);
    let font_sz = (obj.font_size as f32).clamp(8.0, 120.0);
    let job = build_text_layout_job(obj, font_sz, wrap_w);
    let galley = ctx.fonts_mut(|f| f.layout_job(job));
    let min_h = font_sz * 1.45;
    let h = galley.size().y.max(min_h);
    [wrap_w as f64, h as f64]
}

/// Khung bao (page-local) của `TextObject`.
pub fn text_page_bbox(obj: &TextObject) -> Rect {
    Rect::from_min_size(
        pos2(obj.pos[0] as f32, obj.pos[1] as f32),
        vec2(
            (obj.size[0] as f32).max(24.0),
            (obj.size[1] as f32).max((obj.font_size as f32) * 1.35),
        ),
    )
}

/// Vẽ một `TextObject` lên trang theo đúng mức zoom và biến đổi Lasso tạm thời (nếu đang kéo/scale).
pub fn paint_text_object(
    painter: &Painter,
    obj: &TextObject,
    origin: Pos2,
    zoom: f32,
    interaction: Option<&LassoInteraction>,
) {
    let mut p_x = obj.pos[0] as f32;
    let mut p_y = obj.pos[1] as f32;
    let mut w_p = (obj.size[0] as f32).max(24.0);
    let mut font_p = obj.font_size as f32;

    if let Some(inter) = interaction {
        match inter {
            LassoInteraction::Moving { delta, .. } => {
                p_x += delta.x;
                p_y += delta.y;
            }
            LassoInteraction::Resizing { anchor, scale, .. } => {
                let ax = anchor[0] as f32;
                let ay = anchor[1] as f32;
                let sx = scale[0] as f32;
                let sy = scale[1] as f32;
                let x0 = ax + (p_x - ax) * sx;
                let y0 = ay + (p_y - ay) * sy;
                let x1 = ax + (p_x + w_p - ax) * sx;
                p_x = x0.min(x1);
                p_y = y0.min(ay + (p_y + obj.size[1] as f32 - ay) * sy);
                w_p = (x1 - x0).abs().max(48.0);
                if (sy.abs() - 1.0).abs() > 1e-4 {
                    font_p = (font_p * sy.abs()).clamp(8.0, 120.0);
                }
            }
            _ => {}
        }
    }

    let screen_pos = origin + vec2(p_x * zoom, p_y * zoom);
    let wrap_w = (w_p * zoom).max(16.0);
    let font_px = (font_p * zoom).max(4.0);
    let job = build_text_layout_job(obj, font_px, wrap_w);
    let galley = painter.ctx().fonts_mut(|f| f.layout_job(job));
    let box_rect = Rect::from_min_size(
        screen_pos,
        vec2(wrap_w, galley.size().y.max(font_px * 1.45)),
    );
    let galley_pos = egui_align2(obj.align)
        .align_size_within_rect(galley.size(), box_rect)
        .intersect(box_rect)
        .min;
    let draw_pos = galley_pos - vec2(galley.rect.left(), 0.0);
    let color =
        Color32::from_rgba_unmultiplied(obj.color[0], obj.color[1], obj.color[2], obj.color[3]);
    painter.galley(draw_pos, galley, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_text_obj(text: &str, align: TextAlign) -> TextObject {
        TextObject {
            id: 1,
            pos: [100.0, 120.0],
            size: [240.0, 40.0],
            text: text.to_string(),
            font_family: TextFontFamily::Sans,
            font_size: 18.0,
            bold: false,
            italic: false,
            align,
            list_mode: TextListMode::None,
            color: [20, 22, 25, 255],
        }
    }

    #[test]
    fn strip_and_empty_and_apply_list_modes() {
        assert_eq!(strip_list_prefix("• Tiếng Việt"), "Tiếng Việt");
        assert_eq!(strip_list_prefix("1. Một"), "Một");
        assert_eq!(strip_list_prefix("12. Mười hai"), "Mười hai");
        assert_eq!(strip_list_prefix("Không dấu"), "Không dấu");

        assert!(is_text_effectively_empty(""));
        assert!(is_text_effectively_empty("   \n  "));
        assert!(is_text_effectively_empty("• \n• "));
        assert!(is_text_effectively_empty("1. \n2. "));
        assert!(!is_text_effectively_empty("• Có chữ"));
        assert!(!is_text_effectively_empty("1. ă ằ ẳ ẵ ặ ơ ư"));

        let mut text = String::new();
        apply_list_mode_to_text(&mut text, TextListMode::Bullet);
        assert_eq!(text, "• ");

        text = "Dòng 1\nDòng 2".to_string();
        apply_list_mode_to_text(&mut text, TextListMode::Numbered);
        assert_eq!(text, "1. Dòng 1\n2. Dòng 2");
        apply_list_mode_to_text(&mut text, TextListMode::Bullet);
        assert_eq!(text, "• Dòng 1\n• Dòng 2");
        apply_list_mode_to_text(&mut text, TextListMode::None);
        assert_eq!(text, "Dòng 1\nDòng 2");
    }

    #[test]
    fn continue_list_handles_bullet_numbered_and_exit_on_empty_line() {
        let prev = "• Tiếng Việt";
        let mut curr = "• Tiếng Việt\n".to_string();
        let cursor = curr.chars().count();
        let new_c = continue_list_on_newline(prev, &mut curr, TextListMode::Bullet, cursor);
        assert_eq!(curr, "• Tiếng Việt\n• ");
        assert_eq!(new_c, Some(cursor + 2));

        // Enter trên dòng bullet trống -> xoá ký hiệu bullet dòng trên
        let prev2 = "• Tiếng Việt\n• ";
        let mut curr2 = "• Tiếng Việt\n• \n".to_string();
        let cursor2 = curr2.chars().count();
        let new_c2 = continue_list_on_newline(prev2, &mut curr2, TextListMode::Bullet, cursor2);
        assert_eq!(curr2, "• Tiếng Việt\n\n");
        assert_eq!(new_c2, Some(cursor2 - 2));

        // Numbered list nối tiếp đúng số thứ tự
        let prev_n = "1. Một\n2. Hai";
        let mut curr_n = "1. Một\n2. Hai\n".to_string();
        let cursor_n = curr_n.chars().count();
        let new_cn =
            continue_list_on_newline(prev_n, &mut curr_n, TextListMode::Numbered, cursor_n);
        assert_eq!(curr_n, "1. Một\n2. Hai\n3. ");
        assert_eq!(new_cn, Some(cursor_n + 3));
    }

    #[test]
    fn layout_job_alignment_formula_matches_egui_galley() {
        let ctx = egui::Context::default();
        theme::setup_fonts(&ctx);
        let _ = ctx.run_ui(egui::RawInput::default(), |_| {});

        let wrap_w = 240.0_f32;
        let screen_pos = pos2(100.0, 120.0);

        for (align, expected_min_x_in_box, expected_max_x_in_box) in [
            (TextAlign::Left, true, false),
            (TextAlign::Center, false, false),
            (TextAlign::Right, false, true),
        ] {
            let obj = sample_text_obj("Tiếng Việt, ắ ằ ẳ ẵ ặ ơ ư", align);
            let job = build_text_layout_job(&obj, 18.0, wrap_w);
            let galley = ctx.fonts_mut(|f| f.layout_job(job));
            let box_rect = Rect::from_min_size(screen_pos, vec2(wrap_w, galley.size().y.max(26.0)));
            let galley_pos = egui_align2(align)
                .align_size_within_rect(galley.size(), box_rect)
                .intersect(box_rect)
                .min;
            let draw_pos = galley_pos - vec2(galley.rect.left(), 0.0);
            let visual_rect = galley.rect.translate(draw_pos.to_vec2());

            assert!(
                visual_rect.left() >= box_rect.left() - 0.5
                    && visual_rect.right() <= box_rect.right() + 0.5,
                "align={align:?} overflowed box_rect: visual={visual_rect:?}, box={box_rect:?}"
            );
            if expected_min_x_in_box {
                assert!((visual_rect.left() - box_rect.left()).abs() < 1.0);
            } else if expected_max_x_in_box {
                assert!((visual_rect.right() - box_rect.right()).abs() < 1.0);
            } else {
                assert!((visual_rect.center().x - box_rect.center().x).abs() < 1.0);
            }
        }
    }
}
