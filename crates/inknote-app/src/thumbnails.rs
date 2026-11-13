// Thanh thumbnail danh sách các trang (Bước 3.2 - P1).
// Bố cục 2 cột kiểu GoodNotes: tiêu đề "Pages" + nút đóng ×, mỗi thẻ có số trang bên trái
// và nút mũi tên ⌄ bên phải mở menu popup thao tác trên trang đó.

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, FontId, PopupCloseBehavior, Pos2, Rect,
    ScrollArea, Sense, Stroke, StrokeKind, Ui, UiBuilder,
};
use inknote_core::{CanvasMode, Document, DocumentPage, PaperPattern, PaperSettings};
use inknote_render::DrawBatch;

use crate::{theme::Theme, CachedStroke};

/// Hành động trả về từ Thanh thumbnail trang.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThumbnailAction {
    None,
    Close,
    SelectPage(usize),
    AddPageBefore(usize),
    AddPageAfter(usize),
    DuplicatePage(usize),
    MovePage { from: usize, to: usize },
    ClearPage(usize),
    DeletePage(usize),
}

struct CachedThumb {
    sig: u64,
    scale_bits: u32,
    mesh: Arc<egui::epaint::Mesh>,
}

struct ThumbDragState {
    source_idx: usize,
    start_pos: Pos2,
    dragging: bool,
    target_idx: usize,
}

/// Bộ nhớ đệm lưới thumbnail cho từng trang (khoá = `page.id`) để giữ hiệu năng mượt khi cuộn.
#[derive(Default)]
pub struct ThumbCache {
    entries: HashMap<u64, CachedThumb>,
    drag: Option<ThumbDragState>,
}

impl ThumbCache {
    pub fn clear(&mut self) {
        self.entries.clear();
        self.drag = None;
    }

    pub fn retain_pages(&mut self, pages: &[DocumentPage]) {
        self.entries
            .retain(|id, _| pages.iter().any(|p| p.id == *id));
    }
}

/// Đưa toàn bộ lưới tam giác từ các batch của 1 nét vào `mesh` theo gốc `origin` và tỷ lệ `scale`.
pub fn append_batches_to_mesh(
    batches: &[DrawBatch],
    origin: Pos2,
    scale: f32,
    mesh: &mut egui::epaint::Mesh,
) {
    for batch in batches {
        if batch.indices.is_empty() || batch.vertices.is_empty() {
            continue;
        }
        let [r, g, b, a] = batch.color;
        let col = Color32::from_rgba_unmultiplied(r, g, b, a);
        let base = mesh.vertices.len() as u32;
        mesh.vertices
            .extend(batch.vertices.iter().map(|&[x, y]| egui::epaint::Vertex {
                pos: pos2(origin.x + x * scale, origin.y + y * scale),
                uv: egui::epaint::WHITE_UV,
                color: col,
            }));
        mesh.indices.extend(batch.indices.iter().map(|i| i + base));
    }
}

fn page_stroke_signature(page: &DocumentPage) -> u64 {
    let mut h = (page.strokes.len() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for s in &page.strokes {
        h = h.rotate_left(7) ^ s.id.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    }
    h
}

fn draw_thumbnail_paper_bg(
    painter: &egui::Painter,
    rect: Rect,
    settings: &PaperSettings,
    scale: f32,
    theme: &Theme,
) {
    painter.rect_filled(rect, CornerRadius::same(4), theme.thumb_card_bg);
    let inner = painter.with_clip_rect(rect);

    let step = (settings.dot_spacing * scale).max(6.0);
    match settings.pattern {
        PaperPattern::Plain => {}
        PaperPattern::Dotted => {
            let mut y = rect.top() + step * 0.6;
            while y < rect.bottom() - 2.0 {
                let mut x = rect.left() + step * 0.6;
                while x < rect.right() - 2.0 {
                    inner.circle_filled(pos2(x, y), 0.7, theme.paper_dot_color);
                    x += step;
                }
                y += step;
            }
        }
        PaperPattern::Grid => {
            let s = Stroke::new(0.5, theme.paper_grid_color);
            let mut x = rect.left() + step;
            while x < rect.right() - 1.0 {
                inner.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], s);
                x += step;
            }
            let mut y = rect.top() + step;
            while y < rect.bottom() - 1.0 {
                inner.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], s);
                y += step;
            }
        }
        PaperPattern::Lined => {
            let s = Stroke::new(0.5, theme.paper_line_color);
            let mut y = rect.top() + step;
            while y < rect.bottom() - 1.0 {
                inner.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], s);
                y += step;
            }
        }
    }

    if settings.mode == CanvasMode::Paper && settings.show_margins {
        let m_stroke = Stroke::new(0.7, theme.paper_margin_line_color);
        let lx = rect.left() + settings.left_margin * scale;
        if lx > rect.left() && lx < rect.right() {
            inner.line_segment([pos2(lx, rect.top()), pos2(lx, rect.bottom())], m_stroke);
        }
        let rx = rect.left() + (settings.page_width - settings.right_margin) * scale;
        if rx > rect.left() && rx < rect.right() {
            inner.line_segment([pos2(rx, rect.top()), pos2(rx, rect.bottom())], m_stroke);
        }
    }
}

fn build_or_get_thumb_mesh(
    page: &DocumentPage,
    scale: f32,
    batch_cache: &HashMap<u64, CachedStroke>,
    thumb_cache: &mut ThumbCache,
) -> Option<Arc<egui::epaint::Mesh>> {
    if page.strokes.is_empty() {
        thumb_cache.entries.remove(&page.id);
        return None;
    }

    let sig = page_stroke_signature(page);
    let scale_bits = scale.to_bits();
    if let Some(entry) = thumb_cache.entries.get(&page.id) {
        if entry.sig == sig && entry.scale_bits == scale_bits {
            return Some(Arc::clone(&entry.mesh));
        }
    }

    let mut n_verts = 0;
    let mut n_indices = 0;
    for s in &page.strokes {
        if let Some(c) = batch_cache.get(&s.id) {
            for b in &c.batches {
                n_verts += b.vertices.len();
                n_indices += b.indices.len();
            }
        }
    }
    if n_indices == 0 {
        return None;
    }

    let mut mesh = egui::epaint::Mesh::default();
    mesh.vertices.reserve(n_verts);
    mesh.indices.reserve(n_indices);
    for s in &page.strokes {
        if let Some(c) = batch_cache.get(&s.id) {
            append_batches_to_mesh(&c.batches, Pos2::ZERO, scale, &mut mesh);
        }
    }
    let arc = Arc::new(mesh);
    thumb_cache.entries.insert(
        page.id,
        CachedThumb {
            sig,
            scale_bits,
            mesh: Arc::clone(&arc),
        },
    );
    Some(arc)
}

fn draw_chevron_down(painter: &egui::Painter, center: Pos2, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    painter.line_segment(
        [
            pos2(center.x - 4.0, center.y - 1.5),
            pos2(center.x, center.y + 2.5),
        ],
        stroke,
    );
    painter.line_segment(
        [
            pos2(center.x, center.y + 2.5),
            pos2(center.x + 4.0, center.y - 1.5),
        ],
        stroke,
    );
}

fn draw_close_x(painter: &egui::Painter, center: Pos2, color: Color32) {
    let stroke = Stroke::new(1.6, color);
    let r = 4.5;
    painter.line_segment(
        [
            pos2(center.x - r, center.y - r),
            pos2(center.x + r, center.y + r),
        ],
        stroke,
    );
    painter.line_segment(
        [
            pos2(center.x + r, center.y - r),
            pos2(center.x - r, center.y + r),
        ],
        stroke,
    );
}

const POPUP_MENU_WIDTH: f32 = 172.0;

fn popup_menu_item(ui: &mut Ui, label: &str, text_color: Color32, enabled: bool) -> bool {
    let desired = vec2(POPUP_MENU_WIDTH, 30.0);
    let (rect, resp) = ui.allocate_exact_size(
        desired,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if ui.is_rect_visible(rect) {
        if enabled && resp.hovered() {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(6),
                Color32::from_rgb(238, 241, 246),
            );
        }
        let col = if enabled {
            text_color
        } else {
            Color32::from_rgb(175, 180, 190)
        };
        ui.painter().text(
            pos2(rect.left() + 10.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(13.5),
            col,
        );
    }
    enabled && resp.clicked()
}

/// Vẽ thanh danh sách thumbnail các trang ở cạnh trái workspace (2 cột kiểu GoodNotes).
pub fn show_thumbnail_panel(
    ui: &mut Ui,
    theme: &Theme,
    doc: &Document,
    batch_cache: &HashMap<u64, CachedStroke>,
    thumb_cache: &mut ThumbCache,
) -> ThumbnailAction {
    let mut action = ThumbnailAction::None;
    let pages = doc.pages();
    let page_count = pages.len();
    let active_idx = doc.active_page_index();
    let settings = doc.paper_settings();

    if thumb_cache.entries.len() > page_count {
        thumb_cache.retain_pages(pages);
    }

    // Header: chữ "Pages" căn giữa + nút × đóng bảng ở góc phải (không có nút Add Page)
    ui.add_space(4.0);
    let (header_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
    ui.painter().text(
        header_rect.center(),
        Align2::CENTER_CENTER,
        "Pages",
        FontId::proportional(15.0),
        Color32::from_rgb(28, 32, 40),
    );
    let close_rect = Rect::from_center_size(
        pos2(header_rect.right() - 14.0, header_rect.center().y),
        vec2(24.0, 24.0),
    );
    let close_resp = ui.interact(
        close_rect,
        ui.scope_id().with("close_pages_panel"),
        Sense::click(),
    );
    if close_resp.hovered() {
        ui.painter().rect_filled(
            close_rect,
            CornerRadius::same(6),
            Color32::from_rgb(228, 232, 238),
        );
    }
    draw_close_x(
        ui.painter(),
        close_rect.center(),
        Color32::from_rgb(56, 62, 74),
    );
    if close_resp.clicked() {
        action = ThumbnailAction::Close;
    }

    ui.add_space(10.0);

    let aspect = (settings.page_height / settings.page_width.max(1.0)).clamp(0.45, 2.0);
    let mut rendered_cards: Vec<(usize, Rect)> = Vec::with_capacity(page_count);
    let mut scroll_rect = Rect::NOTHING;

    ScrollArea::vertical()
        .id_salt("page_thumbnails_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            scroll_rect = ui.clip_rect();
            let side_pad = 6.0;
            // Chừa khoảng trống bên phải cho thanh cuộn nằm hẳn ra ngoài thẻ thumbnail
            let scroll_reserve = 16.0;
            let col_gap = 16.0;
            let row_gap = 14.0;
            let avail_w = ui.available_width();
            let card_w = ((avail_w - side_pad - scroll_reserve - col_gap) * 0.5).max(76.0);
            let thumb_h = (card_w * aspect).round();
            let scale = card_w / settings.page_width.max(1.0);
            let footer_h = 24.0;

            ui.add_space(4.0);
            for row_start in (0..page_count).step_by(2) {
                ui.horizontal(|ui| {
                    ui.add_space(side_pad);
                    for col in 0..2 {
                        let idx = row_start + col;
                        if idx >= page_count {
                            break;
                        }
                        if col > 0 {
                            ui.add_space(col_gap);
                        }
                        let page = &pages[idx];
                        let is_active = idx == active_idx;

                        let cell_rect = ui.allocate_space(vec2(card_w, thumb_h + 4.0 + footer_h)).1;
                        let card_rect = Rect::from_min_size(cell_rect.min, vec2(card_w, thumb_h));
                        let footer_rect = Rect::from_min_size(
                            pos2(cell_rect.left(), card_rect.bottom() + 4.0),
                            vec2(card_w, footer_h),
                        );
                        rendered_cards.push((idx, card_rect));

                        let card_resp = ui.interact(
                            card_rect,
                            ui.scope_id().with(("thumb_card", page.id)),
                            Sense::click_and_drag(),
                        );

                        if page_count > 1 && card_resp.drag_started_by(egui::PointerButton::Primary)
                        {
                            if let Some(pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                                thumb_cache.drag = Some(ThumbDragState {
                                    source_idx: idx,
                                    start_pos: pos,
                                    dragging: false,
                                    target_idx: idx,
                                });
                            }
                        }

                        let is_drag_source = thumb_cache
                            .drag
                            .as_ref()
                            .is_some_and(|d| d.dragging && d.source_idx == idx);

                        if ui.is_rect_visible(card_rect) {
                            ui.painter().rect_filled(
                                card_rect.translate(vec2(0.0, 2.0)),
                                CornerRadius::same(6),
                                Color32::from_rgba_unmultiplied(15, 23, 42, 18),
                            );
                            draw_thumbnail_paper_bg(
                                ui.painter(),
                                card_rect,
                                settings,
                                scale,
                                theme,
                            );

                            if let Some(cached_mesh) =
                                build_or_get_thumb_mesh(page, scale, batch_cache, thumb_cache)
                            {
                                let mut translated = (*cached_mesh).clone();
                                let offset = card_rect.min.to_vec2();
                                for v in &mut translated.vertices {
                                    v.pos += offset;
                                }
                                ui.painter()
                                    .with_clip_rect(card_rect)
                                    .add(egui::Shape::mesh(translated));
                            }

                            if is_drag_source {
                                ui.painter().rect_filled(
                                    card_rect,
                                    CornerRadius::same(6),
                                    Color32::from_rgba_unmultiplied(245, 246, 249, 150),
                                );
                            }

                            let border = if is_active {
                                Stroke::new(2.2, theme.thumb_card_selected_border)
                            } else if card_resp.hovered() {
                                Stroke::new(
                                    1.5,
                                    theme.thumb_card_selected_border.linear_multiply(0.6),
                                )
                            } else {
                                Stroke::new(1.0, theme.thumb_card_border)
                            };
                            ui.painter().rect_stroke(
                                card_rect,
                                CornerRadius::same(6),
                                border,
                                StrokeKind::Inside,
                            );
                        }

                        if card_resp.clicked() {
                            action = ThumbnailAction::SelectPage(idx);
                        }

                        // Hàng chân thẻ: số trang căn trái + nút mũi tên xuống ⌄ căn phải
                        let num_color = if is_active {
                            theme.thumb_label_active_fg
                        } else {
                            theme.thumb_label_fg
                        };
                        ui.painter().text(
                            pos2(footer_rect.left() + 2.0, footer_rect.center().y),
                            Align2::LEFT_CENTER,
                            format!("{}", idx + 1),
                            FontId::proportional(12.5),
                            num_color,
                        );

                        let menu_btn_rect = Rect::from_center_size(
                            pos2(footer_rect.right() - 10.0, footer_rect.center().y),
                            vec2(22.0, 20.0),
                        );
                        let menu_btn_resp = ui
                            .scope_builder(UiBuilder::new().max_rect(menu_btn_rect), |ui| {
                                let (r, resp) =
                                    ui.allocate_exact_size(menu_btn_rect.size(), Sense::click());
                                if ui.is_rect_visible(r) {
                                    if resp.hovered() {
                                        ui.painter().rect_filled(
                                            r,
                                            CornerRadius::same(5),
                                            Color32::from_rgb(228, 232, 238),
                                        );
                                    }
                                    draw_chevron_down(
                                        ui.painter(),
                                        r.center(),
                                        Color32::from_rgb(56, 62, 74),
                                    );
                                }
                                resp
                            })
                            .inner;

                        let popup_frame = egui::Frame::new()
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(220, 224, 232)))
                            .corner_radius(CornerRadius::same(12))
                            .inner_margin(egui::Margin::symmetric(6, 6))
                            .shadow(egui::Shadow {
                                offset: [0, 4],
                                blur: 14,
                                spread: 0,
                                color: Color32::from_rgba_unmultiplied(0, 0, 0, 38),
                            });

                        egui::Popup::from_toggle_button_response(&menu_btn_resp)
                            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                            .frame(popup_frame)
                            .show(|ui| {
                                ui.set_width(POPUP_MENU_WIDTH);
                                let normal_fg = Color32::from_rgb(32, 36, 44);
                                let danger_fg = Color32::from_rgb(225, 76, 56);

                                if popup_menu_item(ui, "Add Page Before", normal_fg, true) {
                                    action = ThumbnailAction::AddPageBefore(idx);
                                    ui.close();
                                }
                                if popup_menu_item(ui, "Add Page After", normal_fg, true) {
                                    action = ThumbnailAction::AddPageAfter(idx);
                                    ui.close();
                                }
                                if popup_menu_item(ui, "Duplicate", normal_fg, true) {
                                    action = ThumbnailAction::DuplicatePage(idx);
                                    ui.close();
                                }

                                ui.add_space(4.0);
                                let (sep_rect, _) = ui.allocate_exact_size(
                                    vec2(POPUP_MENU_WIDTH, 1.0),
                                    Sense::hover(),
                                );
                                ui.painter().line_segment(
                                    [
                                        pos2(sep_rect.left() + 8.0, sep_rect.center().y),
                                        pos2(sep_rect.right() - 8.0, sep_rect.center().y),
                                    ],
                                    Stroke::new(1.0, Color32::from_rgb(236, 239, 245)),
                                );
                                ui.add_space(4.0);

                                if popup_menu_item(ui, "Clear Page", danger_fg, true) {
                                    action = ThumbnailAction::ClearPage(idx);
                                    ui.close();
                                }
                                if popup_menu_item(ui, "Delete Page", danger_fg, page_count > 1) {
                                    action = ThumbnailAction::DeletePage(idx);
                                    ui.close();
                                }
                            });
                    }
                });
                ui.add_space(row_gap);
            }
        });

    // Xử lý kéo-thả sắp xếp thứ tự trang
    if thumb_cache.drag.is_some() {
        let (primary_down, cur_pos) = ui
            .ctx()
            .input(|i| (i.pointer.primary_down(), i.pointer.interact_pos()));

        if primary_down {
            if let (Some(drag), Some(pos)) = (thumb_cache.drag.as_mut(), cur_pos) {
                if !drag.dragging && pos.distance(drag.start_pos) > 6.0 {
                    drag.dragging = true;
                }
                if drag.dragging && !rendered_cards.is_empty() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    let &(hover_idx, hover_rect) = rendered_cards
                        .iter()
                        .min_by(|a, b| {
                            let da = a.1.center().distance_sq(pos);
                            let db = b.1.center().distance_sq(pos);
                            da.total_cmp(&db)
                        })
                        .unwrap();
                    let insert_before = pos.x < hover_rect.center().x;
                    let raw_slot = if insert_before {
                        hover_idx
                    } else {
                        hover_idx + 1
                    };
                    let target_idx = if raw_slot > drag.source_idx {
                        raw_slot - 1
                    } else {
                        raw_slot
                    }
                    .min(page_count.saturating_sub(1));
                    drag.target_idx = target_idx;

                    if target_idx != drag.source_idx {
                        let bar_x = if insert_before {
                            hover_rect.left() - 8.0
                        } else {
                            hover_rect.right() + 8.0
                        };
                        let bar_painter = ui.painter().with_clip_rect(scroll_rect);
                        let col = theme.thumb_card_selected_border;
                        bar_painter.line_segment(
                            [
                                pos2(bar_x, hover_rect.top() - 2.0),
                                pos2(bar_x, hover_rect.bottom() + 2.0),
                            ],
                            Stroke::new(3.0, col),
                        );
                        bar_painter.circle_filled(pos2(bar_x, hover_rect.top() - 2.0), 3.5, col);
                        bar_painter.circle_filled(pos2(bar_x, hover_rect.bottom() + 2.0), 3.5, col);
                    }

                    // Nhãn nổi đi theo con trỏ chuột khi đang kéo trang
                    let overlay = ui.ctx().layer_painter(egui::LayerId::new(
                        egui::Order::Tooltip,
                        ui.scope_id().with("thumb_drag_badge"),
                    ));
                    let badge_rect = Rect::from_min_size(pos + vec2(14.0, 14.0), vec2(74.0, 24.0));
                    overlay.rect_filled(
                        badge_rect,
                        CornerRadius::same(6),
                        theme.thumb_card_selected_border,
                    );
                    overlay.text(
                        badge_rect.center(),
                        Align2::CENTER_CENTER,
                        format!("Page {}", drag.source_idx + 1),
                        FontId::proportional(12.0),
                        Color32::WHITE,
                    );
                }
            }
        } else if let Some(drag) = thumb_cache.drag.take() {
            if drag.dragging && drag.target_idx != drag.source_idx {
                action = ThumbnailAction::MovePage {
                    from: drag.source_idx,
                    to: drag.target_idx,
                };
            }
        }
    }

    action
}
