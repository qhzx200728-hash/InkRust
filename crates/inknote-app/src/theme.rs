// Tập trung toàn bộ màu sắc, cỡ chữ, khoảng cách, độ bo góc và thiết lập font chữ của InkNote.
// Mọi thành phần giao diện đều đọc từ `Theme` này để dễ chỉnh sửa về sau (theo docs/04-ui-spec.md).

use std::sync::Arc;

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Margin};
use inknote_core::TextFontFamily;
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
    pub nav_status_saved_dot: Color32,
    pub nav_status_unsaved_dot: Color32,
    pub nav_status_notice_fg: Color32,

    // 2. Thanh tuỳ chọn công cụ nổi dạng popup (Contextual Tool Options Pill)
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

    // 4. Canvas và Trang giấy (Paper Settings)
    pub workspace_bg: Color32,
    pub paper_bg: Color32,
    pub paper_border: Color32,
    pub paper_dot_color: Color32,
    pub paper_grid_color: Color32,
    pub paper_line_color: Color32,
    pub paper_margin_line_color: Color32,
    pub paper_dot_spacing: f32,
    pub paper_dot_radius: f32,
    pub paper_margin: f32,

    // 5. Thanh thumbnail trang (Page Thumbnails Sidebar - Bước 3.2)
    pub thumb_panel_bg: Color32,
    pub thumb_panel_stroke: Color32,
    pub thumb_panel_width: f32,
    pub thumb_card_bg: Color32,
    pub thumb_card_selected_border: Color32,
    pub thumb_card_border: Color32,
    pub thumb_label_fg: Color32,
    pub thumb_label_active_fg: Color32,

    // 6. Màn hình Library (Bước 7.1 - P3)
    pub lib_sidebar_bg: Color32,
    pub lib_sidebar_stroke: Color32,
    pub lib_sidebar_selected_bg: Color32,
    pub lib_sidebar_hover_bg: Color32,
    pub lib_sidebar_active_fg: Color32,
    pub lib_sidebar_fg: Color32,
    pub lib_main_bg: Color32,
    pub lib_title_fg: Color32,
    pub lib_muted_fg: Color32,
    pub lib_search_bg: Color32,
    pub lib_search_stroke: Color32,
    pub lib_folder_fill: Color32,
    pub lib_folder_tab: Color32,
    pub lib_folder_back: Color32,
    pub lib_card_hover_bg: Color32,
    pub lib_card_rounding: u8,
    pub lib_star_gold: Color32,
    pub lib_danger_fg: Color32,
    pub lib_danger_bg: Color32,

    // 7. Cỡ chữ
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
            nav_status_saved_dot: Color32::from_rgb(74, 222, 128),
            nav_status_unsaved_dot: Color32::from_rgb(248, 113, 113),
            nav_status_notice_fg: Color32::WHITE,

            // Thanh tuỳ chọn công cụ nổi nền tối bo tròn
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

            // Canvas và trang giấy (Paper Settings)
            workspace_bg: Color32::from_rgb(228, 232, 238),
            paper_bg: Color32::WHITE,
            paper_border: Color32::from_rgb(204, 210, 220),
            paper_dot_color: Color32::from_rgb(198, 205, 216),
            paper_grid_color: Color32::from_rgb(222, 227, 235),
            paper_line_color: Color32::from_rgb(218, 224, 232),
            paper_margin_line_color: Color32::from_rgba_unmultiplied(240, 110, 130, 180),
            paper_dot_spacing: 24.0,
            paper_dot_radius: 1.25,
            paper_margin: 14.0,

            // Thanh thumbnail trang (bên trái, bố cục 2 cột kiểu GoodNotes)
            thumb_panel_bg: Color32::from_rgb(245, 246, 249),
            thumb_panel_stroke: Color32::from_rgb(218, 222, 230),
            thumb_panel_width: 280.0,
            thumb_card_bg: Color32::WHITE,
            thumb_card_selected_border: Color32::from_rgb(24, 90, 188),
            thumb_card_border: Color32::from_rgb(204, 210, 220),
            thumb_label_fg: Color32::from_rgb(60, 66, 78),
            thumb_label_active_fg: Color32::from_rgb(24, 90, 188),

            // Màn hình Library (Sidebar xám nhạt, mục chọn xanh nhạt bo góc, nền nội dung trắng)
            lib_sidebar_bg: Color32::from_rgb(245, 246, 249),
            lib_sidebar_stroke: Color32::from_rgb(228, 232, 238),
            lib_sidebar_selected_bg: Color32::from_rgba_unmultiplied(24, 90, 188, 28),
            lib_sidebar_hover_bg: Color32::from_rgba_unmultiplied(15, 23, 42, 10),
            lib_sidebar_active_fg: Color32::from_rgb(24, 90, 188),
            lib_sidebar_fg: Color32::from_rgb(56, 62, 74),
            lib_main_bg: Color32::WHITE,
            lib_title_fg: Color32::from_rgb(20, 24, 33),
            lib_muted_fg: Color32::from_rgb(112, 120, 134),
            lib_search_bg: Color32::from_rgb(241, 243, 247),
            lib_search_stroke: Color32::from_rgb(218, 223, 232),
            lib_folder_fill: Color32::from_rgb(96, 165, 250),
            lib_folder_tab: Color32::from_rgb(59, 130, 246),
            lib_folder_back: Color32::from_rgb(37, 99, 235),
            lib_card_hover_bg: Color32::from_rgba_unmultiplied(24, 90, 188, 12),
            lib_card_rounding: 10,
            lib_star_gold: Color32::from_rgb(238, 28, 37),
            lib_danger_fg: Color32::from_rgb(220, 38, 38),
            lib_danger_bg: Color32::from_rgb(254, 242, 242),

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

    pub fn tool_pill_frame(&self) -> egui::Frame {
        egui::Frame::new()
            .fill(self.tool_bar_pill_bg)
            .stroke(egui::Stroke::new(1.0, self.tool_bar_pill_stroke))
            .corner_radius(CornerRadius::same(self.tool_bar_rounding))
            .inner_margin(Margin::symmetric(14, 5))
            .shadow(egui::Shadow {
                offset: [0, 4],
                blur: 12,
                spread: 0,
                color: Color32::from_rgba_unmultiplied(0, 0, 0, 48),
            })
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

pub fn text_font_family_key(family: TextFontFamily, bold: bool, italic: bool) -> &'static str {
    match (family, bold, italic) {
        (TextFontFamily::Sans, false, false) => "sans_regular",
        (TextFontFamily::Sans, true, false) => "sans_bold",
        (TextFontFamily::Sans, false, true) => "sans_italic",
        (TextFontFamily::Sans, true, true) => "sans_bold_italic",
        (TextFontFamily::Serif, false, false) => "serif_regular",
        (TextFontFamily::Serif, true, false) => "serif_bold",
        (TextFontFamily::Serif, false, true) => "serif_italic",
        (TextFontFamily::Serif, true, true) => "serif_bold_italic",
        (TextFontFamily::Mono, false, false) => "mono_regular",
        (TextFontFamily::Mono, true, false) => "mono_bold",
        (TextFontFamily::Mono, false, true) => "mono_italic",
        (TextFontFamily::Mono, true, true) => "mono_bold_italic",
    }
}

pub fn text_font_family(family: TextFontFamily, bold: bool, italic: bool) -> FontFamily {
    FontFamily::Name(Arc::from(text_font_family_key(family, bold, italic)))
}

fn register_font_variant(
    fonts: &mut FontDefinitions,
    key: &str,
    path: &str,
    fallback_family: FontFamily,
) {
    let mut chain = Vec::new();
    if let Ok(bytes) = std::fs::read(path) {
        fonts
            .font_data
            .insert(key.to_owned(), Arc::new(FontData::from_owned(bytes)));
        chain.push(key.to_owned());
    }
    if let Some(defaults) = fonts.families.get(&fallback_family) {
        for f in defaults {
            if !chain.contains(f) {
                chain.push(f.clone());
            }
        }
    }
    if fonts.font_data.contains_key("segoeui") && !chain.iter().any(|f| f == "segoeui") {
        chain.push("segoeui".to_owned());
    }
    fonts
        .families
        .insert(FontFamily::Name(Arc::from(key)), chain);
}

/// Nạp font hệ thống Windows (`Segoe UI`, `Times New Roman`, `Consolas` đủ Regular/Bold/Italic)
/// để hiển thị sắc nét và hỗ trợ đầy đủ 100% dấu tiếng Việt.
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

    for (key, path, fb) in [
        (
            "sans_regular",
            r"C:\Windows\Fonts\segoeui.ttf",
            FontFamily::Proportional,
        ),
        (
            "sans_bold",
            r"C:\Windows\Fonts\segoeuib.ttf",
            FontFamily::Proportional,
        ),
        (
            "sans_italic",
            r"C:\Windows\Fonts\segoeuii.ttf",
            FontFamily::Proportional,
        ),
        (
            "sans_bold_italic",
            r"C:\Windows\Fonts\segoeuiz.ttf",
            FontFamily::Proportional,
        ),
        (
            "serif_regular",
            r"C:\Windows\Fonts\times.ttf",
            FontFamily::Proportional,
        ),
        (
            "serif_bold",
            r"C:\Windows\Fonts\timesbd.ttf",
            FontFamily::Proportional,
        ),
        (
            "serif_italic",
            r"C:\Windows\Fonts\timesi.ttf",
            FontFamily::Proportional,
        ),
        (
            "serif_bold_italic",
            r"C:\Windows\Fonts\timesbi.ttf",
            FontFamily::Proportional,
        ),
        (
            "mono_regular",
            r"C:\Windows\Fonts\consola.ttf",
            FontFamily::Monospace,
        ),
        (
            "mono_bold",
            r"C:\Windows\Fonts\consolab.ttf",
            FontFamily::Monospace,
        ),
        (
            "mono_italic",
            r"C:\Windows\Fonts\consolai.ttf",
            FontFamily::Monospace,
        ),
        (
            "mono_bold_italic",
            r"C:\Windows\Fonts\consolaz.ttf",
            FontFamily::Monospace,
        ),
    ] {
        register_font_variant(&mut fonts, key, path, fb);
    }

    ctx.set_fonts(fonts);
}
