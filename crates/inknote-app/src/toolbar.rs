// Thanh công cụ chính (Navigation Toolbar) và Thanh tuỳ chọn công cụ (Contextual Tool Options Bar)
// theo đặc tả docs/04-ui-spec.md (Ngôn ngữ giao diện: Tiếng Anh).

use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, CornerRadius, Layout, PopupCloseBehavior, Pos2, Rect,
    RichText, Sense, Stroke, Ui, UiBuilder, Vec2,
};
use inknote_core::{CanvasMode, PaperOrientation, PaperPattern, PaperSettings, PaperSize};
use inknote_pen::{
    brushconfig::{BrushConfig, BrushStyle},
    builders::PenPathBuilderType,
    style::PressureCurve,
    Color as PenColor,
};

use crate::{
    autosave::FileStatusInfo,
    icons::{draw_icon, ToolIcon},
    theme::{
        pen_color_to_egui, rgb_to_pen_color, Theme, EXTENDED_PALETTE, MARKER_QUICK_WIDTHS,
        QUICK_COLORS, SOLID_QUICK_WIDTHS,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveTool {
    Lasso,
    Pen,
    Eraser,
    Text,
    Image,
    Shapes,
    Latex,
}

impl ActiveTool {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lasso => "Lasso Select",
            Self::Pen => "Pen",
            Self::Eraser => "Eraser",
            Self::Text => "Text",
            Self::Image => "Insert Image",
            Self::Shapes => "Shapes",
            Self::Latex => "LaTeX Formula",
        }
    }
}

pub struct ToolbarActions {
    pub undo_clicked: bool,
    pub redo_clicked: bool,
    pub clear_clicked: bool,
    pub save_clicked: bool,
    pub open_clicked: bool,
    pub new_clicked: bool,
    pub add_page_clicked: bool,
    pub delete_page_clicked: bool,
}

/// Thông tin hình học thực tế của các khu vực trên thanh công cụ (dùng để kiểm chứng alignment)
#[derive(Clone, Copy, Debug)]
pub struct ToolbarLayoutMetrics {
    pub nav_bar_rect: Rect,
    pub nav_left_rect: Rect,
    pub nav_center_rect: Rect,
    pub nav_right_rect: Rect,
    pub tool_strip_rect: Rect,
    pub tool_pill_rect: Rect,
}

impl Default for ToolbarLayoutMetrics {
    fn default() -> Self {
        Self {
            nav_bar_rect: Rect::NOTHING,
            nav_left_rect: Rect::NOTHING,
            nav_center_rect: Rect::NOTHING,
            nav_right_rect: Rect::NOTHING,
            tool_strip_rect: Rect::NOTHING,
            tool_pill_rect: Rect::NOTHING,
        }
    }
}

pub struct ToolbarState {
    pub active_tool: ActiveTool,
    pub read_only: bool,
    pub show_thumbnails_hint: bool,
    pub marker_alpha: f64,
    pub last_metrics: ToolbarLayoutMetrics,
}

impl Default for ToolbarState {
    fn default() -> Self {
        Self {
            active_tool: ActiveTool::Pen,
            read_only: false,
            show_thumbnails_hint: false,
            marker_alpha: 0.42,
            last_metrics: ToolbarLayoutMetrics::default(),
        }
    }
}

/// Đặt một khối widget sao cho tâm của nó trùng với `target_center` (tâm cửa sổ),
/// đồng thời kẹp trong `safe_bounds` để không bao giờ tràn hoặc chồng lấn lên nhóm hai bên.
fn show_centered_in_bounds(
    ui: &mut Ui,
    id_salt: &'static str,
    safe_bounds: Rect,
    target_center: Pos2,
    add_contents: impl FnOnce(&mut Ui),
) -> Rect {
    let id = ui.make_persistent_id(id_salt);
    let size_id = id.with("measured_size");
    let last_size: Option<Vec2> = ui.data(|d| d.get_temp(size_id));

    let child_max_rect = if let Some(size) = last_size {
        let ideal_left = target_center.x - size.x * 0.5;
        let min_left = safe_bounds.left();
        let max_left = (safe_bounds.right() - size.x).max(min_left);
        let left = ideal_left.clamp(min_left, max_left);

        let ideal_top = target_center.y - size.y * 0.5;
        let min_top = safe_bounds.top();
        let max_top = (safe_bounds.bottom() - size.y).max(min_top);
        let top = ideal_top.clamp(min_top, max_top);

        Rect::from_min_size(pos2(left, top), safe_bounds.size())
    } else {
        safe_bounds
    };

    let mut builder = UiBuilder::new()
        .scope_id(id)
        .max_rect(child_max_rect)
        .layout(Layout::left_to_right(Align::Center));
    if last_size.is_none() {
        builder = builder.invisible();
    }

    let inner = ui.scope_builder(builder, add_contents);
    let used_rect = inner.response.rect;
    let measured = used_rect.size();

    if last_size != Some(measured) {
        ui.data_mut(|d| d.insert_temp(size_id, measured));
        ui.ctx()
            .request_discard("toolbar centered group size updated");
    }

    used_rect
}

/// Vẽ nút icon trên Thanh công cụ chính (Navigation Toolbar)
fn nav_icon_button(
    ui: &mut Ui,
    theme: &Theme,
    icon: ToolIcon,
    selected: bool,
    tooltip: &str,
) -> egui::Response {
    let size = vec2(theme.nav_button_size[0], theme.nav_button_size[1]);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if selected {
            painter.rect_filled(
                rect,
                CornerRadius::same(theme.nav_button_rounding),
                theme.nav_selected_bg,
            );
        } else if response.hovered() {
            painter.rect_filled(
                rect,
                CornerRadius::same(theme.nav_button_rounding),
                theme.nav_hover_bg,
            );
        }
        let icon_color = if selected || response.hovered() {
            theme.nav_icon_fg
        } else {
            theme.nav_icon_muted
        };
        draw_icon(painter, rect.shrink(5.0), icon, icon_color);
    }

    response.on_hover_text(tooltip)
}

/// Vẽ cụm trạng thái file ngay bên phải nút Read-Only:
/// `Tên File` `(trang/tổng)` - `Dấu chấm xanh/đỏ` - `Thông báo đã lưu (3s rồi ẩn)`.
fn draw_nav_file_status(
    ui: &mut Ui,
    theme: &Theme,
    status: &FileStatusInfo<'_>,
    page_info: (usize, usize),
) {
    ui.add_space(4.0);
    // Tên file co gọn theo độ dài thật, chỉ rút gọn (...) nếu dài quá 110px
    ui.scope(|ui| {
        ui.set_max_width(110.0);
        ui.add(
            egui::Label::new(
                RichText::new(status.file_name)
                    .size(theme.font_size_small)
                    .color(theme.nav_icon_fg),
            )
            .truncate(),
        );
    });

    // Số trang dạng (trang-hien-tai/tong-so-trang) nằm giữa tên file và dấu chấm lưu
    ui.label(
        RichText::new(format!("({}/{})", page_info.0, page_info.1))
            .size(theme.font_size_small)
            .color(theme.nav_icon_fg),
    );

    // Dấu chấm xanh (đã lưu) hoặc đỏ (chưa lưu)
    let (dot_rect, dot_resp) = ui.allocate_exact_size(vec2(12.0, 18.0), Sense::hover());
    if ui.is_rect_visible(dot_rect) {
        let dot_color = if status.is_saved {
            theme.nav_status_saved_dot
        } else {
            theme.nav_status_unsaved_dot
        };
        let c = dot_rect.center();
        ui.painter().circle_filled(c, 4.5, dot_color);
        ui.painter().circle_stroke(
            c,
            4.5,
            Stroke::new(0.8, Color32::from_rgba_unmultiplied(255, 255, 255, 160)),
        );
    }
    let dot_tip = if status.has_error {
        "Auto-save failed"
    } else if status.is_saved {
        "All changes saved"
    } else if status.has_path {
        "Unsaved changes"
    } else {
        "Not saved to file yet (Ctrl+S)"
    };
    dot_resp.on_hover_text(dot_tip);

    // Thông báo đã lưu chỉ hiện trong 3 giây sau mỗi lần tự lưu thành công
    if let Some(notice) = status.autosaved_notice {
        ui.label(
            RichText::new(notice)
                .size(theme.font_size_small)
                .color(theme.nav_status_notice_fg),
        );
    }
}

/// Vẽ vạch ngăn cách dọc giữa các nhóm chức năng, căn giữa theo trục dọc `row_height`
fn draw_vertical_divider(ui: &mut Ui, row_height: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(10.0, row_height), Sense::hover());
    if ui.is_rect_visible(rect) {
        let x = rect.center().x;
        let half_len = (row_height * 0.32).max(6.0);
        let cy = rect.center().y;
        ui.painter().line_segment(
            [pos2(x, cy - half_len), pos2(x, cy + half_len)],
            Stroke::new(1.0, color),
        );
    }
}

/// Vẽ nút chọn kiểu bút (Solid / Marker / Textured) trên Thanh tuỳ chọn tối
fn brush_style_chip(
    ui: &mut Ui,
    theme: &Theme,
    icon: ToolIcon,
    label: &str,
    selected: bool,
) -> egui::Response {
    let desired = vec2(78.0, theme.tool_row_height);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let bg = if selected {
            theme.tool_selected_bg
        } else if response.hovered() {
            theme.tool_hover_bg
        } else {
            Color32::TRANSPARENT
        };
        if bg != Color32::TRANSPARENT {
            painter.rect_filled(rect, CornerRadius::same(theme.tool_item_rounding), bg);
        }

        let fg = if selected {
            Color32::WHITE
        } else {
            theme.tool_text_fg
        };
        let icon_rect =
            Rect::from_center_size(pos2(rect.left() + 14.0, rect.center().y), vec2(18.0, 18.0));
        draw_icon(painter, icon_rect, icon, fg);

        painter.text(
            pos2(icon_rect.right() + 4.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(theme.font_size_small),
            fg,
        );
    }

    response
}

/// Vẽ nút chọn nhanh độ dày nét bằng hình đường kẻ dày mỏng
fn stroke_width_slot_button(
    ui: &mut Ui,
    theme: &Theme,
    width: f64,
    selected: bool,
    vis_scale: f32,
) -> egui::Response {
    let h = theme.tool_row_height;
    let (rect, response) = ui.allocate_exact_size(vec2(h, h), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if selected {
            painter.rect_filled(
                rect,
                CornerRadius::same(theme.tool_item_rounding),
                theme.tool_selected_bg,
            );
        } else if response.hovered() {
            painter.rect_filled(
                rect,
                CornerRadius::same(theme.tool_item_rounding),
                theme.tool_hover_bg,
            );
        }
        let line_h = ((width as f32) * vis_scale).clamp(1.5, 11.0);
        let c = rect.center();
        painter.line_segment(
            [pos2(rect.left() + 5.0, c.y), pos2(rect.right() - 5.0, c.y)],
            Stroke::new(line_h, Color32::WHITE),
        );
    }
    response.on_hover_text(format!("{width:.1} px"))
}

/// Vẽ nút hiển thị số px độ dày hiện tại (cùng chiều cao `tool_row_height` và căn giữa trục dọc)
fn stroke_width_value_button(ui: &mut Ui, theme: &Theme, width: f64) -> egui::Response {
    let desired = vec2(46.0, theme.tool_row_height);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());
    if ui.is_rect_visible(rect) {
        let bg = if response.hovered() {
            theme.tool_selected_bg
        } else {
            theme.tool_hover_bg
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(theme.tool_item_rounding), bg);
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            format!("{width:.1}px"),
            egui::FontId::proportional(theme.font_size_small),
            theme.tool_text_fg,
        );
    }
    response.on_hover_text("Fine-tune Stroke Thickness")
}

/// Vẽ chấm màu tròn với kích thước ô chứa `box_side` cố định để thẳng hàng trục dọc
fn color_swatch_button(
    ui: &mut Ui,
    rgb: [u8; 3],
    selected: bool,
    radius: f32,
    box_side: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(box_side, box_side), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let center: Pos2 = rect.center();
        let fill = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        painter.circle_filled(center, radius, fill);
        painter.circle_stroke(
            center,
            radius,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 70)),
        );
        if selected {
            painter.circle_stroke(center, radius + 2.5, Stroke::new(1.8, Color32::WHITE));
        } else if response.hovered() {
            painter.circle_stroke(
                center,
                radius + 2.0,
                Stroke::new(1.2, Color32::from_gray(190)),
            );
        }
    }
    response
}

/// Vẽ nút icon nhỏ trong Thanh tuỳ chọn công cụ tối
fn dark_bar_icon_button(
    ui: &mut Ui,
    theme: &Theme,
    icon: ToolIcon,
    tooltip: &str,
) -> egui::Response {
    let h = theme.tool_row_height;
    let (rect, response) = ui.allocate_exact_size(vec2(h, h), Sense::click());
    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(theme.tool_item_rounding),
                theme.tool_hover_bg,
            );
        }
        draw_icon(ui.painter(), rect.shrink(4.0), icon, theme.tool_text_fg);
    }
    response.on_hover_text(tooltip)
}

/// Đọc và cập nhật màu + độ dày của kiểu bút đang chọn trong `BrushConfig`
fn current_width_and_color(cfg: &BrushConfig, marker_alpha: f64) -> (f64, PenColor) {
    match cfg.style {
        BrushStyle::Solid => (
            cfg.solid_options.stroke_width,
            cfg.solid_options.stroke_color.unwrap_or(PenColor::BLACK),
        ),
        BrushStyle::Marker => (
            cfg.marker_options.stroke_width,
            cfg.marker_options
                .stroke_color
                .unwrap_or(rgb_to_pen_color([250, 204, 21], marker_alpha)),
        ),
        BrushStyle::Textured => (
            cfg.textured_options.stroke_width,
            cfg.textured_options.stroke_color.unwrap_or(PenColor::BLACK),
        ),
    }
}

fn set_current_width(cfg: &mut BrushConfig, w: f64) {
    match cfg.style {
        BrushStyle::Solid => cfg.solid_options.stroke_width = w,
        BrushStyle::Marker => cfg.marker_options.stroke_width = w,
        BrushStyle::Textured => cfg.textured_options.stroke_width = w,
    }
}

fn set_current_color(cfg: &mut BrushConfig, rgb: [u8; 3], marker_alpha: f64) {
    match cfg.style {
        BrushStyle::Solid => {
            cfg.solid_options.stroke_color = Some(rgb_to_pen_color(rgb, 1.0));
        }
        BrushStyle::Marker => {
            cfg.marker_options.stroke_color = Some(rgb_to_pen_color(rgb, marker_alpha));
        }
        BrushStyle::Textured => {
            cfg.textured_options.stroke_color = Some(rgb_to_pen_color(rgb, 1.0));
        }
    }
}

fn same_rgb(c: PenColor, rgb: [u8; 3]) -> bool {
    let r = (c.r.clamp(0.0, 1.0) * 255.0).round() as i32;
    let g = (c.g.clamp(0.0, 1.0) * 255.0).round() as i32;
    let b = (c.b.clamp(0.0, 1.0) * 255.0).round() as i32;
    (r - rgb[0] as i32).abs() <= 2
        && (g - rgb[1] as i32).abs() <= 2
        && (b - rgb[2] as i32).abs() <= 2
}

/// Hiển thị Khu vực 1 (Navigation Toolbar) và Khu vực 2 (Contextual Tool Options Bar)
pub fn show_toolbars(
    ui: &mut Ui,
    theme: &Theme,
    state: &mut ToolbarState,
    brush_config: &mut BrushConfig,
    paper_settings: &mut PaperSettings,
    file_status: &FileStatusInfo<'_>,
    page_info: (usize, usize),
    wacom_status: &str,
) -> ToolbarActions {
    let mut actions = ToolbarActions {
        undo_clicked: false,
        redo_clicked: false,
        clear_clicked: false,
        save_clicked: false,
        open_clicked: false,
        new_clicked: false,
        add_page_clicked: false,
        delete_page_clicked: false,
    };

    // =========================================================================
    // KHU VỰC 1: THANH CÔNG CỤ CHÍNH (Navigation Toolbar - nền xanh dương)
    // =========================================================================
    egui::Panel::top("navigation_toolbar")
        .frame(theme.nav_frame())
        .show(ui, |ui| {
            let nav_row_h = theme.nav_button_size[1].max(theme.nav_bar_height - 10.0);
            let (bar_rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), nav_row_h), Sense::hover());

            // 1. Nhóm Trái: luôn căn trái trong `bar_rect`
            let mut left_ui = ui.new_child(
                UiBuilder::new()
                    .id_salt("nav_left_group")
                    .max_rect(bar_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            left_ui.spacing_mut().item_spacing.x = 4.0;
            if nav_icon_button(
                &mut left_ui,
                theme,
                ToolIcon::Home,
                false,
                "Library (Step 7 - P3)",
            )
            .clicked()
            {
                // Sẽ chuyển về màn hình Library ở Bước 7.1
            }
            if nav_icon_button(
                &mut left_ui,
                theme,
                ToolIcon::Pages,
                state.show_thumbnails_hint,
                "Page Thumbnails (Step 3.2 - P1)",
            )
            .clicked()
            {
                state.show_thumbnails_hint = !state.show_thumbnails_hint;
            }
            if nav_icon_button(
                &mut left_ui,
                theme,
                ToolIcon::ReadOnly,
                state.read_only,
                "Read-Only Mode (Lock canvas drawing)",
            )
            .clicked()
            {
                state.read_only = !state.read_only;
            }
            draw_nav_file_status(&mut left_ui, theme, file_status, page_info);
            let left_rect = left_ui.min_rect();

            // 2. Nhóm Phải: luôn căn phải trong `bar_rect`
            let mut right_ui = ui.new_child(
                UiBuilder::new()
                    .id_salt("nav_right_group")
                    .max_rect(bar_rect)
                    .layout(Layout::right_to_left(Align::Center)),
            );
            right_ui.spacing_mut().item_spacing.x = 4.0;
            let more_btn = nav_icon_button(
                &mut right_ui,
                theme,
                ToolIcon::More,
                false,
                "More & Paper Settings",
            );

            egui::Popup::from_toggle_button_response(&more_btn)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .frame(theme.dark_popup_frame(right_ui.style()))
                .show(|ui| {
                    ui.set_min_width(270.0);
                    ui.label(
                        RichText::new("File")
                            .size(theme.font_size_title)
                            .strong()
                            .color(theme.popup_header_fg),
                    );
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(theme.popup_section_bg)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui.button("Save (Ctrl+S)").clicked() {
                                    actions.save_clicked = true;
                                }
                                if ui.button("Open (Ctrl+O)").clicked() {
                                    actions.open_clicked = true;
                                }
                                if ui.button("New (Ctrl+N)").clicked() {
                                    actions.new_clicked = true;
                                }
                            });
                        });
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Pages ({} / {})", page_info.0, page_info.1))
                            .size(theme.font_size_title)
                            .strong()
                            .color(theme.popup_header_fg),
                    );
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(theme.popup_section_bg)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui.button("+ Add Page After").clicked() {
                                    actions.add_page_clicked = true;
                                }
                                let can_delete = page_info.1 > 1;
                                if ui
                                    .add_enabled(
                                        can_delete,
                                        egui::Button::new("Delete Current Page"),
                                    )
                                    .on_disabled_hover_text("A note must have at least 1 page")
                                    .clicked()
                                {
                                    actions.delete_page_clicked = true;
                                    ui.close();
                                }
                            });
                        });
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Paper Settings (P1)")
                            .size(theme.font_size_title)
                            .strong()
                            .color(theme.popup_header_fg),
                    );
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(theme.popup_section_bg)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            // 0. Chế độ hiển thị: Paper vs Whiteboard
                            ui.label(
                                RichText::new("Canvas Mode")
                                    .size(theme.font_size_small)
                                    .color(theme.tool_text_muted),
                            );
                            ui.horizontal(|ui| {
                                ui.selectable_value(
                                    &mut paper_settings.mode,
                                    CanvasMode::Paper,
                                    "📄 Paper (Trang chuẩn)",
                                );
                                ui.selectable_value(
                                    &mut paper_settings.mode,
                                    CanvasMode::Whiteboard,
                                    "🔲 Whiteboard (Bảng trắng)",
                                );
                            });

                            // 1. Khổ giấy tiêu chuẩn (A4 / A5) khi ở chế độ Paper
                            if paper_settings.mode == CanvasMode::Paper {
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new("Paper Format")
                                        .size(theme.font_size_small)
                                        .color(theme.tool_text_muted),
                                );
                                ui.horizontal(|ui| {
                                    if ui
                                        .selectable_label(
                                            paper_settings.paper_size == PaperSize::A4,
                                            "A4 Paper",
                                        )
                                        .on_hover_text("Khổ A4 tiêu chuẩn (210 × 297 mm)")
                                        .clicked()
                                    {
                                        paper_settings.set_paper_size(PaperSize::A4);
                                    }
                                    if ui
                                        .selectable_label(
                                            paper_settings.paper_size == PaperSize::A5,
                                            "A5 Paper",
                                        )
                                        .on_hover_text("Khổ A5 tiêu chuẩn (148 × 210 mm)")
                                        .clicked()
                                    {
                                        paper_settings.set_paper_size(PaperSize::A5);
                                    }
                                });
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    if ui
                                        .selectable_label(
                                            paper_settings.orientation
                                                == PaperOrientation::Portrait,
                                            "Portrait (Dọc)",
                                        )
                                        .clicked()
                                    {
                                        paper_settings.set_orientation(PaperOrientation::Portrait);
                                    }
                                    if ui
                                        .selectable_label(
                                            paper_settings.orientation
                                                == PaperOrientation::Landscape,
                                            "Landscape (Ngang)",
                                        )
                                        .clicked()
                                    {
                                        paper_settings.set_orientation(PaperOrientation::Landscape);
                                    }
                                });
                            }
                            ui.add_space(6.0);

                            // 2. Template chọn kiểu giấy
                            ui.label(
                                RichText::new("Template")
                                    .size(theme.font_size_small)
                                    .color(theme.tool_text_muted),
                            );
                            ui.horizontal(|ui| {
                                ui.selectable_value(
                                    &mut paper_settings.pattern,
                                    PaperPattern::Plain,
                                    "Plain",
                                );
                                ui.selectable_value(
                                    &mut paper_settings.pattern,
                                    PaperPattern::Dotted,
                                    "Dotted",
                                );
                                ui.selectable_value(
                                    &mut paper_settings.pattern,
                                    PaperPattern::Grid,
                                    "Grid",
                                );
                                ui.selectable_value(
                                    &mut paper_settings.pattern,
                                    PaperPattern::Lined,
                                    "Lined",
                                );
                            });

                            // 2. Cỡ ô / khoảng cách
                            if paper_settings.pattern != PaperPattern::Plain {
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("Spacing:")
                                            .size(theme.font_size_small)
                                            .color(theme.tool_text_fg),
                                    );
                                    ui.add(
                                        egui::Slider::new(
                                            &mut paper_settings.dot_spacing,
                                            14.0..=56.0,
                                        )
                                        .suffix(" pt"),
                                    );
                                });
                            }

                            // 3. Đường lề 2 bên (chỉ áp dụng cho chế độ Paper, Whiteboard không có lề)
                            if paper_settings.mode == CanvasMode::Paper {
                                ui.add_space(6.0);
                                ui.checkbox(&mut paper_settings.show_margins, "Margin lines 2 bên");
                                if paper_settings.show_margins {
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("Left:")
                                                .size(theme.font_size_small)
                                                .color(theme.tool_text_fg),
                                        );
                                        ui.add(
                                            egui::Slider::new(
                                                &mut paper_settings.left_margin,
                                                20.0..=240.0,
                                            )
                                            .suffix(" pt"),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("Right:")
                                                .size(theme.font_size_small)
                                                .color(theme.tool_text_fg),
                                        );
                                        ui.add(
                                            egui::Slider::new(
                                                &mut paper_settings.right_margin,
                                                20.0..=240.0,
                                            )
                                            .suffix(" pt"),
                                        );
                                    });
                                }
                            }
                        });
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Diagnostics")
                            .size(theme.font_size_title)
                            .strong()
                            .color(theme.popup_header_fg),
                    );
                    ui.add_space(6.0);
                    egui::Frame::new()
                        .fill(theme.popup_section_bg)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("Stylus: {wacom_status}"))
                                    .size(theme.font_size_small)
                                    .color(theme.tool_text_muted),
                            );
                            ui.label(
                                RichText::new("Font Check: Tiếng Việt, ắ ằ ẳ ẵ ặ ơ ư đ Đ")
                                    .size(theme.font_size_small)
                                    .color(theme.tool_text_fg),
                            );
                        });
                });

            let _ = nav_icon_button(
                &mut right_ui,
                theme,
                ToolIcon::Export,
                false,
                "Export PDF / Image (Step 7.3 - P3)",
            );
            if nav_icon_button(
                &mut right_ui,
                theme,
                ToolIcon::AddPage,
                false,
                "Add Page After Current",
            )
            .clicked()
            {
                actions.add_page_clicked = true;
            }
            let right_rect = right_ui.min_rect();

            // 3. Nhóm Giữa: tâm trùng với tâm cửa sổ (`bar_rect.center()`),
            // đồng thời giới hạn trong khoảng giữa Nhóm Trái và Nhóm Phải để không chồng lấn.
            let min_gap = 8.0;
            let safe_left = (left_rect.right() + min_gap).min(bar_rect.right());
            let safe_right = (right_rect.left() - min_gap).max(safe_left);
            let safe_center_bounds = Rect::from_min_max(
                pos2(safe_left, bar_rect.top()),
                pos2(safe_right, bar_rect.bottom()),
            );

            let center_rect = show_centered_in_bounds(
                ui,
                "nav_center_group",
                safe_center_bounds,
                bar_rect.center(),
                |ui| {
                    ui.set_min_height(nav_row_h);
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let tools = [
                        (
                            ActiveTool::Lasso,
                            ToolIcon::Lasso,
                            "Lasso Select (Step 5.2)",
                        ),
                        (ActiveTool::Pen, ToolIcon::Pen, "Pen (Rnote Brush)"),
                        (
                            ActiveTool::Eraser,
                            ToolIcon::Eraser,
                            "Eraser - Strokes Only (Step 5.1)",
                        ),
                        (ActiveTool::Text, ToolIcon::Text, "Text Tool (Step 5.4)"),
                        (
                            ActiveTool::Image,
                            ToolIcon::Image,
                            "Insert / Paste Image (Step 6.1)",
                        ),
                        (ActiveTool::Shapes, ToolIcon::Shapes, "Shapes (Step 5.3)"),
                        (
                            ActiveTool::Latex,
                            ToolIcon::Latex,
                            "Insert LaTeX Formula (Step 4)",
                        ),
                    ];
                    for (tool, icon, tip) in tools {
                        let is_sel = !state.read_only && state.active_tool == tool;
                        if nav_icon_button(ui, theme, icon, is_sel, tip).clicked() {
                            state.read_only = false;
                            state.active_tool = tool;
                        }
                    }

                    draw_vertical_divider(ui, nav_row_h, theme.nav_divider);

                    if nav_icon_button(ui, theme, ToolIcon::Undo, false, "Undo (Ctrl+Z)").clicked()
                    {
                        actions.undo_clicked = true;
                    }
                    if nav_icon_button(
                        ui,
                        theme,
                        ToolIcon::Redo,
                        false,
                        "Redo (Ctrl+Y / Ctrl+Shift+Z)",
                    )
                    .clicked()
                    {
                        actions.redo_clicked = true;
                    }
                    if nav_icon_button(ui, theme, ToolIcon::Trash, false, "Clear Page").clicked() {
                        actions.clear_clicked = true;
                    }
                },
            );

            state.last_metrics.nav_bar_rect = bar_rect;
            state.last_metrics.nav_left_rect = left_rect;
            state.last_metrics.nav_center_rect = center_rect;
            state.last_metrics.nav_right_rect = right_rect;
        });

    // =========================================================================
    // KHU VỰC 2: THANH TUỲ CHỌN CÔNG CỤ (Contextual Tool Options Bar - nền tối bo tròn)
    // =========================================================================
    egui::Panel::top("contextual_tool_options_bar")
        .frame(theme.tool_strip_frame())
        .show(ui, |ui| {
            let pill_outer_h = theme.tool_row_height + 10.0; // inner_margin.y = 5 * 2
            let (strip_rect, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), pill_outer_h),
                Sense::hover(),
            );
            let mut pill_rect = Rect::NOTHING;

            let _ = show_centered_in_bounds(
                ui,
                "contextual_options_pill_aligned",
                strip_rect,
                strip_rect.center(),
                |ui| {
                    let inner = theme.tool_pill_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.set_min_height(theme.tool_row_height);
                            ui.spacing_mut().item_spacing.x = 6.0;

                            if state.read_only {
                                ui.label(
                                    RichText::new("Read-Only Mode is active — click the eye icon or any tool to resume drawing")
                                        .size(theme.font_size_body)
                                        .color(theme.tool_text_fg),
                                );
                                return;
                            }

                            match state.active_tool {
                                ActiveTool::Pen => {
                                    show_pen_contextual_options(ui, theme, state, brush_config);
                                }
                                other => {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} — Contextual options will be enabled in its Roadmap step",
                                            other.label()
                                        ))
                                        .size(theme.font_size_body)
                                        .color(theme.tool_text_fg),
                                    );
                                }
                            }
                        });
                    });
                    pill_rect = inner.response.rect;
                },
            );

            state.last_metrics.tool_strip_rect = strip_rect;
            state.last_metrics.tool_pill_rect = pill_rect;
        });

    actions
}

/// Nội dung Thanh tuỳ chọn khi đang chọn công cụ Bút (Pen)
fn show_pen_contextual_options(
    ui: &mut Ui,
    theme: &Theme,
    state: &mut ToolbarState,
    brush_config: &mut BrushConfig,
) {
    let row_h = theme.tool_row_height;

    // 1. Nhóm chọn Kiểu bút (Solid / Marker / Textured)
    if brush_style_chip(
        ui,
        theme,
        ToolIcon::SolidBrush,
        "Solid",
        brush_config.style == BrushStyle::Solid,
    )
    .clicked()
    {
        brush_config.style = BrushStyle::Solid;
    }

    if brush_style_chip(
        ui,
        theme,
        ToolIcon::MarkerBrush,
        "Marker",
        brush_config.style == BrushStyle::Marker,
    )
    .clicked()
    {
        brush_config.style = BrushStyle::Marker;
        let (_, c) = current_width_and_color(brush_config, state.marker_alpha);
        if c.a >= 0.95 {
            brush_config.marker_options.stroke_color = Some(PenColor {
                r: 0.98,
                g: 0.80,
                b: 0.08,
                a: state.marker_alpha,
            });
        }
    }

    if brush_style_chip(
        ui,
        theme,
        ToolIcon::TexturedBrush,
        "Textured",
        brush_config.style == BrushStyle::Textured,
    )
    .clicked()
    {
        brush_config.style = BrushStyle::Textured;
    }

    // Nút mở Pop-up chi tiết Pen Settings (Builder & Pressure Curve)
    let pen_settings_btn = dark_bar_icon_button(
        ui,
        theme,
        ToolIcon::Sliders,
        "Pen Settings (Stroke Builder & Pressure Curve)",
    );
    egui::Popup::from_toggle_button_response(&pen_settings_btn)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .frame(theme.dark_popup_frame(ui.style()))
        .show(|ui| {
            ui.set_min_width(240.0);
            ui.label(
                RichText::new("Pen Settings")
                    .size(theme.font_size_title)
                    .strong()
                    .color(theme.popup_header_fg),
            );
            ui.add_space(6.0);

            egui::Frame::new()
                .fill(theme.popup_section_bg)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Stroke Builder")
                            .size(theme.font_size_small)
                            .color(theme.tool_text_muted),
                    );
                    ui.horizontal(|ui| {
                        for (b, name) in [
                            (PenPathBuilderType::Modeled, "Modeled"),
                            (PenPathBuilderType::Curved, "Curved"),
                            (PenPathBuilderType::Simple, "Simple"),
                        ] {
                            let sel = std::mem::discriminant(&brush_config.builder_type)
                                == std::mem::discriminant(&b);
                            if ui
                                .selectable_label(
                                    sel,
                                    RichText::new(name).color(theme.tool_text_fg),
                                )
                                .clicked()
                            {
                                brush_config.builder_type = b;
                            }
                        }
                    });
                });

            ui.add_space(6.0);

            egui::Frame::new()
                .fill(theme.popup_section_bg)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Pressure Curve (Solid Pen)")
                            .size(theme.font_size_small)
                            .color(theme.tool_text_muted),
                    );
                    ui.horizontal_wrapped(|ui| {
                        for (curve, label) in [
                            (PressureCurve::Linear, "Linear"),
                            (PressureCurve::Sqrt, "Sqrt"),
                            (PressureCurve::Cbrt, "Cbrt"),
                            (PressureCurve::Pow2, "Pow2"),
                            (PressureCurve::Pow3, "Pow3"),
                            (PressureCurve::Const, "Const"),
                        ] {
                            let sel =
                                std::mem::discriminant(&brush_config.solid_options.pressure_curve)
                                    == std::mem::discriminant(&curve);
                            if ui
                                .selectable_label(
                                    sel,
                                    RichText::new(label).color(theme.tool_text_fg),
                                )
                                .clicked()
                            {
                                brush_config.solid_options.pressure_curve = curve;
                            }
                        }
                    });
                });
        });

    draw_vertical_divider(ui, row_h, theme.tool_bar_pill_stroke);

    // 2. Nhóm Độ dày nét bút (5 mức nhanh + Pop-up thanh trượt)
    let (cur_width, cur_color) = current_width_and_color(brush_config, state.marker_alpha);
    let (presets, vis_scale): ([f64; 5], f32) = match brush_config.style {
        BrushStyle::Marker => (MARKER_QUICK_WIDTHS, 0.32),
        BrushStyle::Solid | BrushStyle::Textured => (SOLID_QUICK_WIDTHS, 1.0),
    };

    for w in presets {
        let is_sel = (cur_width - w).abs() < 0.25;
        if stroke_width_slot_button(ui, theme, w, is_sel, vis_scale).clicked() {
            set_current_width(brush_config, w);
        }
    }

    // Nút hiển thị số px hiện tại & mở Pop-up thanh trượt độ dày (cao đúng `row_h`)
    let width_btn = stroke_width_value_button(ui, theme, cur_width);

    egui::Popup::from_toggle_button_response(&width_btn)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .frame(theme.dark_popup_frame(ui.style()))
        .show(|ui| {
            ui.set_min_width(210.0);
            ui.label(
                RichText::new("Stroke Thickness")
                    .size(theme.font_size_title)
                    .strong()
                    .color(theme.popup_header_fg),
            );
            ui.add_space(6.0);
            let mut w = cur_width;
            let range = match brush_config.style {
                BrushStyle::Marker => 4.0..=48.0,
                BrushStyle::Solid | BrushStyle::Textured => 0.5..=24.0,
            };
            if ui
                .add(egui::Slider::new(&mut w, range).text("px"))
                .changed()
            {
                set_current_width(brush_config, w);
            }
        });

    draw_vertical_divider(ui, row_h, theme.tool_bar_pill_stroke);

    // 3. Hàng chấm màu nhanh (6 màu) + Nút mở Pop-up Bảng màu mở rộng (tất cả cao đúng `row_h`)
    for rgb in QUICK_COLORS {
        let is_sel = same_rgb(cur_color, rgb);
        if color_swatch_button(ui, rgb, is_sel, 8.5, row_h).clicked() {
            set_current_color(brush_config, rgb, state.marker_alpha);
        }
    }

    let palette_btn = dark_bar_icon_button(ui, theme, ToolIcon::Palette, "More Colors");
    egui::Popup::from_toggle_button_response(&palette_btn)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .frame(theme.dark_popup_frame(ui.style()))
        .show(|ui| {
            ui.set_min_width(245.0);
            ui.label(
                RichText::new("Stroke Color")
                    .size(theme.font_size_title)
                    .strong()
                    .color(theme.popup_header_fg),
            );
            ui.add_space(6.0);

            egui::Frame::new()
                .fill(theme.popup_section_bg)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    egui::Grid::new("extended_color_grid")
                        .spacing(vec2(6.0, 6.0))
                        .show(ui, |ui| {
                            for (idx, rgb) in EXTENDED_PALETTE.iter().copied().enumerate() {
                                let is_sel = same_rgb(cur_color, rgb);
                                if color_swatch_button(ui, rgb, is_sel, 9.5, 26.0).clicked() {
                                    set_current_color(brush_config, rgb, state.marker_alpha);
                                }
                                if (idx + 1) % 8 == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Custom:")
                        .size(theme.font_size_small)
                        .color(theme.tool_text_muted),
                );
                let mut egui_col = pen_color_to_egui(cur_color);
                if ui.color_edit_button_srgba(&mut egui_col).changed() {
                    let [r, g, b, a] = egui_col.to_srgba_unmultiplied();
                    let alpha = if brush_config.style == BrushStyle::Marker {
                        state.marker_alpha
                    } else {
                        (a as f64 / 255.0).max(0.1)
                    };
                    set_current_color(brush_config, [r, g, b], alpha);
                }
            });

            if brush_config.style == BrushStyle::Marker {
                ui.add_space(4.0);
                let mut alpha_pct = (state.marker_alpha * 100.0) as f32;
                if ui
                    .add(egui::Slider::new(&mut alpha_pct, 15.0..=85.0).text("Opacity %"))
                    .changed()
                {
                    state.marker_alpha = (alpha_pct as f64 / 100.0).clamp(0.15, 0.85);
                    if let Some(mut c) = brush_config.marker_options.stroke_color {
                        c.a = state.marker_alpha;
                        brush_config.marker_options.stroke_color = Some(c);
                    }
                }
            }
        });
}
