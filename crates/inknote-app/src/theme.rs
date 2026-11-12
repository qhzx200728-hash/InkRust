// Tập trung toàn bộ màu sắc, cỡ chữ, khoảng cách, độ bo góc và thiết lập font chữ của InkNote.
// Mọi thành phần giao diện đều đọc từ `Theme` này để dễ chỉnh sửa về sau (theo docs/04-ui-spec.md).

use std::sync::Arc;

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Margin};
use inknote_pen::Color as PenColor;

#[derive(Clone, Debug)]
pub struct Theme {
    // 1. Thanh công cụ chính (Navigation Toolbar)
    pub nav_bar_bg: Color32,
    pub nav_bar_height: f32,
    pub nav_icon_fg: Color32,
    pub nav_icon_muted: Color32,
    pub nav_selected_bg: Color32,
    pub nav_hover_bg: Color32,
    pub nav_divider: Color32,
    pub nav_button_size: [f32; 2],
    pub nav_button_rounding: u8,

    // 2. Thanh tuỳ chọn công cụ (Contextual Tool Options Bar)
    pub tool_strip_bg: Color32,
    pub tool_bar_pill_bg: Color32,
    pub tool_bar_pill_stroke: Color32,
    pub tool_bar_rounding: u8,
    pub tool_row_height: f32,
    pub tool_text_fg: Color32,
    pub tool_text_muted: Color32,
    pub tool_selected_bg: Color32,
    pub tool_hover_bg: Color32,
    pub tool_item_rounding: u8,

    // 3. Bảng Pop-up chi tiết (nền tối, bo góc)
    pub popup_bg: Color32,
    pub popup_stroke: Color32,
    pub popup_rounding: u8,
    pub popup_header_fg: Color32,
    pub popup_section_bg: Color32,

    // 4. Canvas và Trang giấy (Dotted Paper mặc định)
    pub workspace_bg: Color32,
    pub paper_bg: Color32,
    pub paper_border: Color32,
    pub paper_dot_color: Color32,
    pub paper_dot_spacing: f32,
    pub paper_dot_radius: f32,
    pub paper_margin: f32,

    // 5. Cỡ chữ
    pub font_size_title: f32,
    pub font_size_body: f32,
    pub font_size_small: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Thanh công cụ chính nền xanh dương, icon trắng, ô chọn sáng bo góc
            nav_bar_bg: Color32::from_rgb(24, 90, 188),
            nav_bar_height: 40.0,
            nav_icon_fg: Color32::WHITE,
            nav_icon_muted: Color32::from_rgba_unmultiplied(255, 255, 255, 190),
            nav_selected_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 58),
            nav_hover_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 30),
            nav_divider: Color32::from_rgba_unmultiplied(255, 255, 255, 45),
            nav_button_size: [34.0, 30.0],
            nav_button_rounding: 7,

            // Thanh tuỳ chọn công cụ nền tối bo tròn
            tool_strip_bg: Color32::from_rgb(236, 239, 244),
            tool_bar_pill_bg: Color32::from_rgb(34, 36, 42),
            tool_bar_pill_stroke: Color32::from_rgb(56, 59, 68),
            tool_bar_rounding: 18,
            tool_row_height: 26.0,
            tool_text_fg: Color32::from_rgb(242, 244, 248),
            tool_text_muted: Color32::from_rgb(168, 174, 186),
            tool_selected_bg: Color32::from_rgb(56, 116, 203),
            tool_hover_bg: Color32::from_rgb(50, 53, 62),
            tool_item_rounding: 8,

            // Bảng Pop-up nền tối bo góc
            popup_bg: Color32::from_rgb(28, 30, 36),
            popup_stroke: Color32::from_rgb(64, 68, 78),
            popup_rounding: 12,
            popup_header_fg: Color32::WHITE,
            popup_section_bg: Color32::from_rgb(40, 43, 52),

            // Canvas và trang giấy chấm (Dotted Paper)
            workspace_bg: Color32::from_rgb(228, 232, 238),
            paper_bg: Color32::WHITE,
            paper_border: Color32::from_rgb(204, 210, 220),
            paper_dot_color: Color32::from_rgb(198, 205, 216),
            paper_dot_spacing: 24.0,
            paper_dot_radius: 1.25,
            paper_margin: 14.0,

            // Cỡ chữ
            font_size_title: 14.5,
            font_size_body: 13.0,
            font_size_small: 11.5,
        }
    }
}

impl Theme {
    pub fn nav_frame(&self) -> egui::Frame {
        egui::Frame::new()
            .fill(self.nav_bar_bg)
            .inner_margin(Margin::symmetric(12, 5))
    }

    pub fn tool_strip_frame(&self) -> egui::Frame {
        egui::Frame::new()
            .fill(self.tool_strip_bg)
            .inner_margin(Margin::symmetric(12, 6))
    }

    pub fn tool_pill_frame(&self) -> egui::Frame {
        egui::Frame::new()
            .fill(self.tool_bar_pill_bg)
            .stroke(egui::Stroke::new(1.0, self.tool_bar_pill_stroke))
            .corner_radius(CornerRadius::same(self.tool_bar_rounding))
            .inner_margin(Margin::symmetric(14, 5))
    }

    pub fn dark_popup_frame(&self, base_style: &egui::Style) -> egui::Frame {
        egui::Frame::popup(base_style)
            .fill(self.popup_bg)
            .stroke(egui::Stroke::new(1.0, self.popup_stroke))
            .corner_radius(CornerRadius::same(self.popup_rounding))
            .inner_margin(Margin::same(12))
    }
}

/// Các mức độ dày nét nhanh hiển thị trên thanh tuỳ chọn Bút (Solid)
pub const SOLID_QUICK_WIDTHS: [f64; 5] = [1.5, 2.5, 4.0, 6.5, 10.0];

/// Các mức độ dày nét nhanh cho bút dạ quang (Marker)
pub const MARKER_QUICK_WIDTHS: [f64; 5] = [8.0, 12.0, 18.0, 24.0, 32.0];

/// Hàng chấm màu nhanh trên thanh tuỳ chọn Bút
pub const QUICK_COLORS: [[u8; 3]; 6] = [
    [20, 22, 25],   // Black
    [25, 103, 210], // Blue
    [217, 48, 37],  // Red
    [24, 128, 56],  // Green
    [242, 153, 0],  // Amber / Orange
    [147, 52, 230], // Purple
];

/// Bảng màu mở rộng trong Pop-up (3 hàng x 8 cột = 24 màu)
pub const EXTENDED_PALETTE: [[u8; 3]; 24] = [
    // Hàng 1: Đen - Xám - Trắng - Nâu
    [0, 0, 0],
    [55, 65, 81],
    [107, 114, 128],
    [156, 163, 175],
    [209, 213, 219],
    [120, 53, 15],
    [146, 64, 14],
    [180, 83, 9],
    // Hàng 2: Màu đậm (cho bút viết Solid)
    [185, 28, 28],
    [217, 119, 6],
    [202, 138, 4],
    [21, 128, 61],
    [15, 118, 110],
    [29, 78, 216],
    [67, 56, 202],
    [126, 34, 206],
    // Hàng 3: Màu tươi / sáng (hợp cả bút nhấn & dạ quang Marker)
    [239, 68, 68],
    [249, 115, 22],
    [250, 204, 21],
    [74, 222, 128],
    [45, 212, 191],
    [56, 189, 248],
    [129, 140, 248],
    [244, 114, 182],
];

/// Chuyển đổi màu `inknote_pen::Color` sang `egui::Color32`
pub fn pen_color_to_egui(c: PenColor) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (c.r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.b.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.a.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

/// Tạo `inknote_pen::Color` từ RGB `u8` và độ trong suốt `alpha` (`0.0..=1.0`)
pub fn rgb_to_pen_color(rgb: [u8; 3], alpha: f64) -> PenColor {
    PenColor {
        r: rgb[0] as f64 / 255.0,
        g: rgb[1] as f64 / 255.0,
        b: rgb[2] as f64 / 255.0,
        a: alpha.clamp(0.0, 1.0),
    }
}

/// Nạp font hệ thống Windows (`Segoe UI` và `Consolas`) để hiển thị sắc nét và hỗ trợ đầy đủ 100% dấu tiếng Việt.
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        fonts
            .font_data
            .insert("segoeui".to_owned(), Arc::new(FontData::from_owned(bytes)));
        if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
            prop.insert(0, "segoeui".to_owned());
        }
    }

    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\consola.ttf") {
        fonts
            .font_data
            .insert("consolas".to_owned(), Arc::new(FontData::from_owned(bytes)));
        if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
            mono.insert(0, "consolas".to_owned());
        }
    }

    ctx.set_fonts(fonts);
}
