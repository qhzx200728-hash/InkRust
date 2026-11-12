// Thanh công cụ chính (Navigation Toolbar) và Thanh tuỳ chọn công cụ (Contextual Tool Options Bar)
// theo đặc tả docs/04-ui-spec.md (Ngôn ngữ giao diện: Tiếng Anh).

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};
use inknote_pen::{
    Color as PenColor,
    brushconfig::{BrushConfig, BrushStyle},
    builders::PenPathBuilderType,
    style::PressureCurve,
};

use crate::{
    icons::{ToolIcon, draw_icon},
    theme::{
        EXTENDED_PALETTE, MARKER_QUICK_WIDTHS, QUICK_COLORS, SOLID_QUICK_WIDTHS, Theme,
        pen_color_to_egui, rgb_to_pen_color,
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
    pub clear_clicked: bool,
}

pub struct ToolbarState {
    pub active_tool: ActiveTool,
    pub read_only: bool,
    pub show_thumbnails_hint: bool,
    pub marker_alpha: f64,
}

impl Default for ToolbarState {
    fn default() -> Self {
        Self {
            active_tool: ActiveTool::Pen,
            read_only: false,
            show_thumbnails_hint: false,
            marker_alpha: 0.42,
        }
    }
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

/// Vẽ vạch ngăn cách dọc trên thanh công cụ
fn draw_vertical_divider(ui: &mut Ui, height: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(8.0, height), Sense::hover());
    if ui.is_rect_visible(rect) {
        let x = rect.center().x;
        ui.painter().line_segment(
            [pos2(x, rect.top() + 5.0), pos2(x, rect.bottom() - 5.0)],
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
    let desired = vec2(78.0, 26.0);
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
        let icon_rect = Rect::from_min_size(pos2(rect.left() + 5.0, rect.top() + 4.0), vec2(18.0, 18.0));
        draw_icon(painter, icon_rect, icon, fg);

        painter.text(
            pos2(icon_rect.right() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
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
    let (rect, response) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
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

/// Vẽ chấm màu tròn trên thanh tuỳ chọn hoặc trong Pop-up bảng màu
fn color_swatch_button(ui: &mut Ui, rgb: [u8; 3], selected: bool, radius: f32) -> egui::Response {
    let side = (radius + 4.0) * 2.0;
    let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
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
    let (rect, response) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
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
fn current_width_and_color(
    cfg: &BrushConfig,
    marker_alpha: f64,
) -> (f64, PenColor) {
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
    wacom_status: &str,
) -> ToolbarActions {
    let mut actions = ToolbarActions {
        undo_clicked: false,
        clear_clicked: false,
    };

    // =========================================================================
    // KHU VỰC 1: THANH CÔNG CỤ CHÍNH (Navigation Toolbar - nền xanh dương)
    // =========================================================================
    egui::Panel::top("navigation_toolbar")
        .frame(theme.nav_frame())
        .show(ui, |ui| {
            ui.set_min_height(theme.nav_bar_height - 10.0);
            ui.columns(3, |cols| {
                // --- Nhóm Trái: Home, Pages, Read-only ---
                cols[0].horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if nav_icon_button(ui, theme, ToolIcon::Home, false, "Library (Step 7 - P3)").clicked() {
                        // Sẽ chuyển về màn hình Library ở Bước 7.1
                    }
                    if nav_icon_button(
                        ui,
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
                        ui,
                        theme,
                        ToolIcon::ReadOnly,
                        state.read_only,
                        "Read-Only Mode (Lock canvas drawing)",
                    )
                    .clicked()
                    {
                        state.read_only = !state.read_only;
                    }
                });

                // --- Nhóm Giữa: Các công cụ chính (Lasso, Pen, Eraser, Text, Image, Shapes, LaTeX) ---
                cols[1].with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let tools = [
                            (ActiveTool::Lasso, ToolIcon::Lasso, "Lasso Select (Step 5.2)"),
                            (ActiveTool::Pen, ToolIcon::Pen, "Pen (Rnote Brush)"),
                            (ActiveTool::Eraser, ToolIcon::Eraser, "Eraser - Strokes Only (Step 5.1)"),
                            (ActiveTool::Text, ToolIcon::Text, "Text Tool (Step 5.4)"),
                            (ActiveTool::Image, ToolIcon::Image, "Insert / Paste Image (Step 6.1)"),
                            (ActiveTool::Shapes, ToolIcon::Shapes, "Shapes (Step 5.3)"),
                            (ActiveTool::Latex, ToolIcon::Latex, "Insert LaTeX Formula (Step 4)"),
                        ];
                        for (tool, icon, tip) in tools {
                            let is_sel = !state.read_only && state.active_tool == tool;
                            if nav_icon_button(ui, theme, icon, is_sel, tip).clicked() {
                                state.read_only = false;
                                state.active_tool = tool;
                            }
                        }

                        draw_vertical_divider(ui, 28.0, theme.nav_divider);

                        if nav_icon_button(ui, theme, ToolIcon::Undo, false, "Undo (Ctrl+Z)").clicked() {
                            actions.undo_clicked = true;
                        }
                        if nav_icon_button(ui, theme, ToolIcon::Trash, false, "Clear Page").clicked() {
                            actions.clear_clicked = true;
                        }
                    });
                });

                // --- Nhóm Phải: Add Page, Export, More (•••) ---
                cols[2].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let more_btn = nav_icon_button(
                        ui,
                        theme,
                        ToolIcon::More,
                        false,
                        "More & Paper Settings",
                    );

                    // Pop-up tối bo góc cho menu More (•••)
                    egui::Popup::from_toggle_button_response(&more_btn)
                        .frame(theme.dark_popup_frame(ui.style()))
                        .show(|ui| {
                            ui.set_min_width(270.0);
                            ui.label(
                                RichText::new("Document & Diagnostics")
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
                                        RichText::new("Paper Template: Dotted Paper (Default)")
                                            .size(theme.font_size_body)
                                            .color(theme.tool_text_fg),
                                    );
                                    ui.label(
                                        RichText::new(format!("Stylus: {wacom_status}"))
                                            .size(theme.font_size_small)
                                            .color(theme.tool_text_muted),
                                    );
                                    ui.label(
                                        RichText::new(
                                            "Font Check: Tiếng Việt, ắ ằ ẳ ẵ ặ ơ ư đ Đ",
                                        )
                                        .size(theme.font_size_small)
                                        .color(theme.tool_text_fg),
                                    );
                                });
                        });

                    let _ = nav_icon_button(
                        ui,
                        theme,
                        ToolIcon::Export,
                        false,
                        "Export PDF / Image (Step 7.3 - P3)",
                    );
                    let _ = nav_icon_button(
                        ui,
                        theme,
                        ToolIcon::AddPage,
                        false,
                        "Add Page (Step 3.1 - P1)",
                    );
                });
            });
        });

    // =========================================================================
    // KHU VỰC 2: THANH TUỲ CHỌN CÔNG CỤ (Contextual Tool Options Bar - nền tối bo tròn)
    // =========================================================================
    egui::Panel::top("contextual_tool_options_bar")
        .frame(theme.tool_strip_frame())
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                theme.tool_pill_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
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
            });
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
        // Đảm bảo bút Marker có độ trong suốt (alpha) để tô dạ quang không che chữ
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

            // Nhóm 1: Stroke Builder
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

            // Nhóm 2: Pressure Curve (cho bút Solid)
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
                            let sel = std::mem::discriminant(
                                &brush_config.solid_options.pressure_curve,
                            ) == std::mem::discriminant(&curve);
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

    draw_vertical_divider(ui, 24.0, theme.tool_bar_pill_stroke);

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

    // Nút hiển thị số px hiện tại & mở Pop-up thanh trượt độ dày
    let width_btn = ui
        .add(
            egui::Button::new(
                RichText::new(format!("{cur_width:.1}px"))
                    .size(theme.font_size_small)
                    .color(theme.tool_text_fg),
            )
            .fill(theme.tool_hover_bg)
            .corner_radius(CornerRadius::same(theme.tool_item_rounding)),
        )
        .on_hover_text("Fine-tune Stroke Thickness");

    egui::Popup::from_toggle_button_response(&width_btn)
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

    draw_vertical_divider(ui, 24.0, theme.tool_bar_pill_stroke);

    // 3. Hàng chấm màu nhanh (6 màu) + Nút mở Pop-up Bảng màu mở rộng
    for rgb in QUICK_COLORS {
        let is_sel = same_rgb(cur_color, rgb);
        if color_swatch_button(ui, rgb, is_sel, 8.5).clicked() {
            set_current_color(brush_config, rgb, state.marker_alpha);
        }
    }

    let palette_btn = dark_bar_icon_button(ui, theme, ToolIcon::Palette, "More Colors");
    egui::Popup::from_toggle_button_response(&palette_btn)
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
                                if color_swatch_button(ui, rgb, is_sel, 9.5).clicked() {
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
