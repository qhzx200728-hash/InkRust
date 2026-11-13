// Hộp thoại nhập công thức LaTeX & Live Preview (Bước 4.2):
// - Phong cách Pop-up tối bo góc
// - Debounce ~0,5 giây sau khi ngừng gõ rồi tự biên dịch trên luồng nền
// - Khi gõ sai cú pháp: báo lỗi rõ ràng nhưng vẫn giữ hình xem trước hợp lệ trước đó

use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, ScrollArea, Stroke, TextEdit,
};
use inknote_latex::{compile_latex_to_svg, CompileOptions, LatexError};

use crate::{svg_cache::SvgTextureCache, theme::Theme};

const DIALOG_WIDTH: f32 = 520.0;
const DEBOUNCE_DELAY: Duration = Duration::from_millis(500);
const PREVIEW_TEX_ID: u64 = 999_001;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LatexDialogAction {
    None,
    Cancel,
    Confirm { source: String, svg: String },
}

pub struct LatexDialogState {
    pub open: bool,
    pub source: String,
    last_edit_at: Option<Instant>,
    pending_compile: bool,
    compiling: bool,
    latest_seq: u64,
    pub preview_svg: Option<String>,
    pub error_msg: Option<String>,
    rx: Option<Receiver<(u64, Result<String, LatexError>)>>,
}

impl Default for LatexDialogState {
    fn default() -> Self {
        Self {
            open: false,
            source: String::new(),
            last_edit_at: None,
            pending_compile: false,
            compiling: false,
            latest_seq: 0,
            preview_svg: None,
            error_msg: None,
            rx: None,
        }
    }
}

impl LatexDialogState {
    /// Mở hộp thoại với nội dung mặc định (hoặc mã cũ khi sửa ở Bước 4.4).
    pub fn open_with(&mut self, initial_source: &str, ctx: &egui::Context) {
        self.open = true;
        self.source = initial_source.to_string();
        self.error_msg = None;
        self.preview_svg = None;
        self.pending_compile = false;
        self.last_edit_at = None;
        if !self.source.trim().is_empty() {
            self.start_compile(ctx);
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.pending_compile = false;
    }

    fn mark_edited(&mut self) {
        self.last_edit_at = Some(Instant::now());
        self.pending_compile = true;
    }

    fn start_compile(&mut self, ctx: &egui::Context) {
        self.pending_compile = false;
        if self.source.trim().is_empty() {
            self.preview_svg = None;
            self.error_msg = None;
            return;
        }

        self.latest_seq = self.latest_seq.wrapping_add(1);
        let seq = self.latest_seq;
        let src = self.source.clone();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.compiling = true;
        let ctx_clone = ctx.clone();

        thread::spawn(move || {
            let res = compile_latex_to_svg(&src, &CompileOptions::default());
            let _ = tx.send((seq, res));
            ctx_clone.request_repaint();
        });
    }

    fn poll_background(&mut self, ctx: &egui::Context, svg_cache: &mut SvgTextureCache) {
        if let Some(rx) = self.rx.as_ref() {
            if let Ok((seq, res)) = rx.try_recv() {
                self.compiling = false;
                self.rx = None;
                if seq == self.latest_seq {
                    match res {
                        Ok(svg) => {
                            self.preview_svg = Some(svg);
                            self.error_msg = None;
                            svg_cache.invalidate(PREVIEW_TEX_ID);
                        }
                        Err(LatexError::EmptySource) => {
                            self.preview_svg = None;
                            self.error_msg = None;
                        }
                        Err(err) => {
                            // Giữ nguyên preview_svg cũ khi có lỗi cú pháp để người dùng dễ đối chiếu
                            self.error_msg = Some(err.to_string());
                        }
                    }
                }
            }
        }

        if self.pending_compile {
            if let Some(edited_at) = self.last_edit_at {
                let elapsed = edited_at.elapsed();
                if elapsed >= DEBOUNCE_DELAY {
                    if !self.compiling {
                        self.start_compile(ctx);
                    }
                } else {
                    ctx.request_repaint_after(
                        (DEBOUNCE_DELAY - elapsed).min(Duration::from_millis(80)),
                    );
                }
            }
        }
    }

    /// Vẽ hộp thoại Pop-up tối bo góc ở giữa màn hình và xử lý sự kiện Live Preview.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        svg_cache: &mut SvgTextureCache,
    ) -> LatexDialogAction {
        if !self.open {
            return LatexDialogAction::None;
        }

        self.poll_background(ctx, svg_cache);

        let mut action = LatexDialogAction::None;
        let esc_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let ctrl_enter = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter));

        // Lớp phủ mờ nhẹ phía sau hộp thoại
        let screen_rect = ctx.content_rect();
        egui::Area::new(egui::Id::new("latex_dialog_backdrop"))
            .order(egui::Order::Middle)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let (r, _) = ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
                ui.painter()
                    .rect_filled(r, CornerRadius::ZERO, Color32::from_black_alpha(90));
            });

        egui::Window::new("latex_editor_dialog")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .fixed_size(egui::vec2(DIALOG_WIDTH, 0.0))
            .frame(
                egui::Frame::new()
                    .fill(theme.tool_bar_pill_bg)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(64, 72, 88)))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(egui::Margin::same(16))
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: Color32::from_rgba_unmultiplied(0, 0, 0, 110),
                    }),
            )
            .show(ctx, |ui| {
                ui.set_width(DIALOG_WIDTH - 32.0);

                // Tiêu đề + nút đóng ×
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("LaTeX Formula")
                            .color(Color32::WHITE)
                            .size(16.0)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("×")
                                        .color(Color32::from_rgb(190, 198, 212))
                                        .size(16.0),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            action = LatexDialogAction::Cancel;
                        }
                        if self.compiling {
                            ui.spinner();
                            ui.label(
                                egui::RichText::new("Rendering...")
                                    .color(Color32::from_rgb(150, 195, 255))
                                    .size(12.0),
                            );
                        } else if self.pending_compile {
                            ui.label(
                                egui::RichText::new("Waiting...")
                                    .color(Color32::from_rgb(160, 168, 184))
                                    .size(12.0),
                            );
                        }
                    });
                });

                ui.add_space(8.0);

                // Thanh chèn nhanh ký hiệu toán học phổ biến
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    let snippets = [
                        (r"\frac{a}{b}", "a/b"),
                        (r"\sqrt{x}", "√x"),
                        (r"\sum_{i=1}^{n}", "∑"),
                        (r"\int_{a}^{b}", "∫"),
                        (r"\pm", "±"),
                        (r"\alpha", "α"),
                        (r"\beta", "β"),
                    ];
                    for (tex, label) in snippets {
                        let btn = egui::Button::new(
                            egui::RichText::new(label)
                                .color(Color32::from_rgb(225, 232, 244))
                                .size(12.0),
                        )
                        .fill(Color32::from_rgb(52, 59, 74))
                        .corner_radius(CornerRadius::same(6));
                        if ui.add(btn).clicked() {
                            if !self.source.is_empty() && !self.source.ends_with(' ') {
                                self.source.push(' ');
                            }
                            self.source.push_str(tex);
                            self.mark_edited();
                        }
                    }
                });

                ui.add_space(8.0);

                // Ô nhập mã LaTeX (nền tối, chữ sáng, font monospace)
                let edit_frame = egui::Frame::new()
                    .fill(Color32::from_rgb(24, 28, 36))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(70, 80, 98)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(8));
                edit_frame.show(ui, |ui| {
                    let resp = ui.add(
                        TextEdit::multiline(&mut self.source)
                            .desired_rows(4)
                            .desired_width(f32::INFINITY)
                            .frame(egui::Frame::NONE)
                            .font(FontId::monospace(14.0))
                            .text_color(Color32::from_rgb(238, 242, 248))
                            .hint_text(
                                "Type LaTeX here, e.g. $x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}$",
                            ),
                    );
                    if resp.changed() {
                        self.mark_edited();
                    }
                });

                ui.add_space(10.0);

                // Khung xem trước SVG (nền trắng để công thức chữ đen đọc rõ nhất)
                ui.label(
                    egui::RichText::new("Live Preview")
                        .color(Color32::from_rgb(180, 188, 204))
                        .size(12.0),
                );
                ui.add_space(4.0);

                let preview_h = 110.0;
                let (preview_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), preview_h),
                    egui::Sense::hover(),
                );
                ui.painter()
                    .rect_filled(preview_rect, CornerRadius::same(8), Color32::WHITE);
                ui.painter().rect_stroke(
                    preview_rect,
                    CornerRadius::same(8),
                    Stroke::new(1.0, Color32::from_rgb(205, 212, 224)),
                    egui::StrokeKind::Inside,
                );

                if let Some(svg) = self.preview_svg.as_deref() {
                    if let Ok((tex_id, logical_size)) =
                        svg_cache.get_or_render(ctx, PREVIEW_TEX_ID, svg, 1.35)
                    {
                        let base_size = logical_size * 1.6;
                        let max_w = (preview_rect.width() - 24.0).max(20.0);
                        let max_h = (preview_rect.height() - 16.0).max(20.0);
                        let fit = (max_w / base_size.x.max(1.0))
                            .min(max_h / base_size.y.max(1.0))
                            .min(1.0);
                        let draw_size = base_size * fit;
                        let img_rect = Rect::from_center_size(preview_rect.center(), draw_size);
                        ui.painter().with_clip_rect(preview_rect).image(
                            tex_id,
                            img_rect,
                            Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                } else {
                    ui.painter().text(
                        preview_rect.center(),
                        Align2::CENTER_CENTER,
                        "Preview will appear here ~0.5s after typing",
                        FontId::proportional(13.0),
                        Color32::from_rgb(140, 148, 162),
                    );
                }

                // Khung báo lỗi rõ ràng khi gõ sai (vẫn giữ hình xem trước cũ ở trên)
                if let Some(err) = &self.error_msg {
                    ui.add_space(8.0);
                    egui::Frame::new()
                        .fill(Color32::from_rgb(66, 30, 34))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(190, 74, 74)))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ScrollArea::vertical()
                                .id_salt("latex_err_scroll")
                                .max_height(68.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(err)
                                            .color(Color32::from_rgb(255, 178, 174))
                                            .monospace()
                                            .size(11.5),
                                    );
                                });
                        });
                }

                ui.add_space(12.0);

                // Hàng nút Cancel / OK
                let can_confirm =
                    self.preview_svg.is_some() && self.error_msg.is_none() && !self.compiling;
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ok_btn = egui::Button::new(
                            egui::RichText::new("OK (Ctrl+Enter)")
                                .color(Color32::WHITE)
                                .size(13.0),
                        )
                        .fill(if can_confirm {
                            theme.nav_bar_bg
                        } else {
                            Color32::from_rgb(65, 74, 90)
                        })
                        .corner_radius(CornerRadius::same(8));
                        if (ui.add_enabled(can_confirm, ok_btn).clicked()
                            || (ctrl_enter && can_confirm))
                            && self.preview_svg.is_some()
                        {
                            action = LatexDialogAction::Confirm {
                                source: self.source.clone(),
                                svg: self.preview_svg.clone().unwrap_or_default(),
                            };
                        }

                        let cancel_btn = egui::Button::new(
                            egui::RichText::new("Cancel")
                                .color(Color32::from_rgb(220, 226, 236))
                                .size(13.0),
                        )
                        .fill(Color32::from_rgb(54, 62, 76))
                        .corner_radius(CornerRadius::same(8));
                        if ui.add(cancel_btn).clicked() {
                            action = LatexDialogAction::Cancel;
                        }
                    });
                });
            });

        if esc_pressed {
            action = LatexDialogAction::Cancel;
        }
        if !matches!(action, LatexDialogAction::None) {
            self.close();
        }
        action
    }
}
