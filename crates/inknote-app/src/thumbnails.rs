// Thanh thumbnail danh sách các trang (Bước 3.2 - P1).
// Hiển thị cột thumbnail bên trái Workspace, cho phép bấm để nhảy trang, thêm trang và xoá trang.

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui::{
    self, pos2, vec2, Align, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText,
    ScrollArea, Sense, Stroke, StrokeKind, Ui,
};
use inknote_core::{CanvasMode, Document, DocumentPage, PaperPattern, PaperSettings};
use inknote_render::DrawBatch;

use crate::{theme::Theme, CachedStroke};

/// Hành động trả về từ Thanh thumbnail trang.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThumbnailAction {
    None,
    SelectPage(usize),
    AddPage,
    DeletePage(usize),
}

struct CachedThumb {
    sig: u64,
    scale_bits: u32,
    mesh: Arc<egui::epaint::Mesh>,
}

/// Bộ nhớ đệm lưới thumbnail cho từng trang (khoá = `page.id`) để giữ hiệu năng mượt khi cuộn.
#[derive(Default)]
pub struct ThumbCache {
    entries: HashMap<u64, CachedThumb>,
}

impl ThumbCache {
    pub fn clear(&mut self) {
        self.entries.clear();
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

/// Vẽ thanh danh sách thumbnail các trang ở cạnh trái workspace.
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

    // Dọn cache nếu số trang giảm sau thao tác xoá
    if thumb_cache.entries.len() > page_count {
        thumb_cache.retain_pages(pages);
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("Pages ({})", page_count))
                .size(theme.font_size_body)
                .strong()
                .color(theme.thumb_label_fg),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .button(RichText::new("+ Add").size(theme.font_size_small))
                .on_hover_text("Add a new page after current page")
                .clicked()
            {
                action = ThumbnailAction::AddPage;
            }
        });
    });
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    let aspect = (settings.page_height / settings.page_width.max(1.0)).clamp(0.45, 2.0);
    ScrollArea::vertical()
        .id_salt("page_thumbnails_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let thumb_w = (ui.available_width() - 16.0).max(96.0);
            let thumb_h = (thumb_w * aspect).round();
            let scale = thumb_w / settings.page_width.max(1.0);

            for (idx, page) in pages.iter().enumerate() {
                let is_active = idx == active_idx;
                ui.vertical_centered(|ui| {
                    let (card_rect, card_resp) =
                        ui.allocate_exact_size(vec2(thumb_w, thumb_h), Sense::click());

                    let mut delete_clicked = false;
                    let can_delete = page_count > 1;
                    let del_rect = Rect::from_center_size(
                        pos2(card_rect.right() - 12.0, card_rect.top() + 12.0),
                        vec2(18.0, 18.0),
                    );
                    let show_del_btn = can_delete
                        && (is_active
                            || card_resp.hovered()
                            || ui.rect_contains_pointer(card_rect));
                    let del_resp = if can_delete {
                        Some(ui.interact(
                            del_rect,
                            ui.scope_id().with(("thumb_del", page.id)),
                            Sense::click(),
                        ))
                    } else {
                        None
                    };
                    if let Some(ref dr) = del_resp {
                        if dr.clicked() {
                            delete_clicked = true;
                        }
                    }

                    if ui.is_rect_visible(card_rect) {
                        // Đổ bóng nhẹ phía dưới card
                        ui.painter().rect_filled(
                            card_rect.translate(vec2(0.0, 1.5)),
                            CornerRadius::same(4),
                            Color32::from_rgba_unmultiplied(0, 0, 0, 20),
                        );

                        draw_thumbnail_paper_bg(ui.painter(), card_rect, settings, scale, theme);

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

                        let border = if is_active {
                            Stroke::new(2.5, theme.thumb_card_selected_border)
                        } else if card_resp.hovered() {
                            Stroke::new(1.5, theme.thumb_card_selected_border.linear_multiply(0.6))
                        } else {
                            Stroke::new(1.0, theme.thumb_card_border)
                        };
                        ui.painter().rect_stroke(
                            card_rect,
                            CornerRadius::same(4),
                            border,
                            StrokeKind::Outside,
                        );

                        if show_del_btn {
                            let is_del_hovered = del_resp.as_ref().is_some_and(|r| r.hovered());
                            let bg = if is_del_hovered {
                                theme.thumb_delete_hover_bg
                            } else {
                                theme.thumb_delete_bg
                            };
                            ui.painter().circle_filled(del_rect.center(), 8.5, bg);
                            ui.painter().text(
                                del_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "×",
                                FontId::proportional(13.0),
                                Color32::WHITE,
                            );
                        }
                    }

                    if let Some(dr) = del_resp {
                        dr.on_hover_text(format!("Delete Page {}", idx + 1));
                    }

                    if delete_clicked {
                        action = ThumbnailAction::DeletePage(idx);
                    } else if card_resp.clicked() {
                        action = ThumbnailAction::SelectPage(idx);
                    }

                    ui.add_space(3.0);
                    let label_color = if is_active {
                        theme.thumb_label_active_fg
                    } else {
                        theme.thumb_label_fg
                    };
                    let mut label = RichText::new(format!("{}", idx + 1))
                        .size(theme.font_size_small)
                        .color(label_color);
                    if is_active {
                        label = label.strong();
                    }
                    ui.label(label);
                });
                ui.add_space(10.0);
            }
        });

    action
}
