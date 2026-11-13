// Logic công cụ Chọn vùng (Lasso – Bước 5.2):
// - 2 chế độ khoanh: Tự do (Freehand) và Hình chữ nhật (Rectangle).
// - Bộ lọc loại đối tượng (Handwriting, LaTeX, Shapes, Text Boxes, Images).
// - Kéo di chuyển, kéo 4 góc đổi kích thước, xoá vùng chọn (1 bước Undo).

use std::collections::{HashMap, HashSet};

use eframe::egui;
use inknote_core::{Document, DocumentPage};

use crate::{
    CachedStroke,
    shapes::{shape_intersects_polygon, shape_intersects_rect, shape_page_bbox},
    toolbar::{LassoFilter, LassoMode},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResizeCorner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}

#[derive(Clone, Debug)]
pub struct LassoSelection {
    pub page_index: usize,
    pub stroke_ids: Vec<u64>,
    pub latex_ids: Vec<u64>,
    pub shape_ids: Vec<u64>,
    pub bbox: egui::Rect,
}

#[derive(Clone, Debug)]
pub enum LassoInteraction {
    DrawingFreehand {
        page_index: usize,
        points: Vec<egui::Pos2>,
    },
    DrawingRect {
        page_index: usize,
        start: egui::Pos2,
        current: egui::Pos2,
    },
    Moving {
        start_page_pos: egui::Pos2,
        delta: egui::Vec2,
    },
    Resizing {
        corner: ResizeCorner,
        anchor: [f64; 2],
        orig_bbox: egui::Rect,
        scale: [f64; 2],
    },
}

#[derive(Default)]
pub struct LassoState {
    pub selection: Option<LassoSelection>,
    pub interaction: Option<LassoInteraction>,
}

impl LassoState {
    pub fn clear(&mut self) {
        self.selection = None;
        self.interaction = None;
    }

    pub fn is_active(&self) -> bool {
        self.interaction.is_some()
    }
}

pub fn point_in_polygon(pt: egui::Pos2, poly: &[egui::Pos2]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let pi = poly[i];
        let pj = poly[j];
        let intersect = ((pi.y > pt.y) != (pj.y > pt.y))
            && (pt.x < (pj.x - pi.x) * (pt.y - pi.y) / ((pj.y - pi.y) + 1e-9) + pi.x);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn rect_intersects_polygon(r: egui::Rect, poly: &[egui::Pos2]) -> bool {
    let checks = [
        r.center(),
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        r.left_bottom(),
    ];
    if checks.iter().any(|&p| point_in_polygon(p, poly)) {
        return true;
    }
    poly.iter().any(|&p| r.contains(p))
}

pub fn stroke_page_bbox(
    stroke: &inknote_core::Stroke,
    batch_cache: &HashMap<u64, CachedStroke>,
) -> egui::Rect {
    if let Some(c) = batch_cache.get(&stroke.id) {
        if c.bbox.is_positive() {
            return c.bbox;
        }
    }
    let mut r = egui::Rect::NOTHING;
    let s = stroke.path.start.pos;
    r.extend_with(egui::pos2(s.x as f32, s.y as f32));
    for seg in &stroke.path.segments {
        let e = seg.end().pos;
        r.extend_with(egui::pos2(e.x as f32, e.y as f32));
    }
    r.expand((stroke.style.stroke_width() as f32 * 0.5).max(2.0))
}

pub fn compute_selection_bbox(
    page: &DocumentPage,
    stroke_ids: &[u64],
    latex_ids: &[u64],
    shape_ids: &[u64],
    batch_cache: &HashMap<u64, CachedStroke>,
) -> Option<egui::Rect> {
    let s_set: HashSet<u64> = stroke_ids.iter().copied().collect();
    let l_set: HashSet<u64> = latex_ids.iter().copied().collect();
    let sh_set: HashSet<u64> = shape_ids.iter().copied().collect();
    let mut bbox = egui::Rect::NOTHING;

    for s in &page.strokes {
        if s_set.contains(&s.id) {
            bbox = bbox.union(stroke_page_bbox(s, batch_cache));
        }
    }
    for l in &page.latex_objects {
        if l_set.contains(&l.id) {
            let r = egui::Rect::from_min_size(
                egui::pos2(l.pos[0] as f32, l.pos[1] as f32),
                egui::vec2(l.size[0] as f32, l.size[1] as f32),
            );
            bbox = bbox.union(r);
        }
    }
    for sh in &page.shapes {
        if sh_set.contains(&sh.id) {
            bbox = bbox.union(shape_page_bbox(sh));
        }
    }

    if bbox.is_positive()
        || (bbox.width() >= 0.0 && bbox.height() >= 0.0 && bbox != egui::Rect::NOTHING)
    {
        Some(bbox.expand(4.0))
    } else {
        None
    }
}

pub fn select_in_freehand(
    page: &DocumentPage,
    page_index: usize,
    poly: &[egui::Pos2],
    filter: LassoFilter,
    batch_cache: &HashMap<u64, CachedStroke>,
) -> Option<LassoSelection> {
    if poly.len() < 3 {
        return None;
    }
    let mut stroke_ids = Vec::new();
    let mut latex_ids = Vec::new();
    let mut shape_ids = Vec::new();

    if filter.handwriting {
        for s in &page.strokes {
            let start = s.path.start.pos;
            let mut hit = point_in_polygon(egui::pos2(start.x as f32, start.y as f32), poly);
            if !hit {
                for seg in &s.path.segments {
                    let e = seg.end().pos;
                    if point_in_polygon(egui::pos2(e.x as f32, e.y as f32), poly) {
                        hit = true;
                        break;
                    }
                }
            }
            if hit {
                stroke_ids.push(s.id);
            }
        }
    }

    if filter.latex {
        for l in &page.latex_objects {
            let r = egui::Rect::from_min_size(
                egui::pos2(l.pos[0] as f32, l.pos[1] as f32),
                egui::vec2(l.size[0] as f32, l.size[1] as f32),
            );
            if rect_intersects_polygon(r, poly) {
                latex_ids.push(l.id);
            }
        }
    }

    if filter.shapes {
        for sh in &page.shapes {
            if shape_intersects_polygon(sh, poly) {
                shape_ids.push(sh.id);
            }
        }
    }

    let bbox = compute_selection_bbox(page, &stroke_ids, &latex_ids, &shape_ids, batch_cache)?;
    Some(LassoSelection {
        page_index,
        stroke_ids,
        latex_ids,
        shape_ids,
        bbox,
    })
}

pub fn select_in_rect(
    page: &DocumentPage,
    page_index: usize,
    rect: egui::Rect,
    filter: LassoFilter,
    batch_cache: &HashMap<u64, CachedStroke>,
) -> Option<LassoSelection> {
    if rect.width() < 2.0 && rect.height() < 2.0 {
        return None;
    }
    let mut stroke_ids = Vec::new();
    let mut latex_ids = Vec::new();
    let mut shape_ids = Vec::new();

    if filter.handwriting {
        for s in &page.strokes {
            let b = stroke_page_bbox(s, batch_cache);
            if b.intersects(rect) {
                stroke_ids.push(s.id);
            }
        }
    }

    if filter.latex {
        for l in &page.latex_objects {
            let r = egui::Rect::from_min_size(
                egui::pos2(l.pos[0] as f32, l.pos[1] as f32),
                egui::vec2(l.size[0] as f32, l.size[1] as f32),
            );
            if r.intersects(rect) {
                latex_ids.push(l.id);
            }
        }
    }

    if filter.shapes {
        for sh in &page.shapes {
            if shape_intersects_rect(sh, rect) {
                shape_ids.push(sh.id);
            }
        }
    }

    let bbox = compute_selection_bbox(page, &stroke_ids, &latex_ids, &shape_ids, batch_cache)?;
    Some(LassoSelection {
        page_index,
        stroke_ids,
        latex_ids,
        shape_ids,
        bbox,
    })
}

pub fn corner_pos(bbox: egui::Rect, corner: ResizeCorner) -> egui::Pos2 {
    match corner {
        ResizeCorner::TopLeft => bbox.left_top(),
        ResizeCorner::TopRight => bbox.right_top(),
        ResizeCorner::BottomRight => bbox.right_bottom(),
        ResizeCorner::BottomLeft => bbox.left_bottom(),
    }
}

pub fn opposite_anchor(bbox: egui::Rect, corner: ResizeCorner) -> [f64; 2] {
    let p = match corner {
        ResizeCorner::TopLeft => bbox.right_bottom(),
        ResizeCorner::TopRight => bbox.left_bottom(),
        ResizeCorner::BottomRight => bbox.left_top(),
        ResizeCorner::BottomLeft => bbox.right_top(),
    };
    [p.x as f64, p.y as f64]
}

pub fn hit_resize_corner(bbox_screen: egui::Rect, screen_pos: egui::Pos2) -> Option<ResizeCorner> {
    let corners = [
        ResizeCorner::TopLeft,
        ResizeCorner::TopRight,
        ResizeCorner::BottomRight,
        ResizeCorner::BottomLeft,
    ];
    for c in corners {
        let p = corner_pos(bbox_screen, c);
        if p.distance(screen_pos) <= 9.0 {
            return Some(c);
        }
    }
    None
}

pub fn refresh_selection_bbox(
    sel: &mut LassoSelection,
    doc: &Document,
    batch_cache: &HashMap<u64, CachedStroke>,
) -> bool {
    let Some(page) = doc.pages().get(sel.page_index) else {
        return false;
    };
    if let Some(bbox) = compute_selection_bbox(
        page,
        &sel.stroke_ids,
        &sel.latex_ids,
        &sel.shape_ids,
        batch_cache,
    ) {
        sel.bbox = bbox;
        true
    } else {
        false
    }
}

pub fn transformed_bbox(bbox: egui::Rect, interaction: Option<&LassoInteraction>) -> egui::Rect {
    match interaction {
        Some(LassoInteraction::Moving { delta, .. }) => bbox.translate(*delta),
        Some(LassoInteraction::Resizing {
            anchor,
            orig_bbox,
            scale,
            ..
        }) => {
            let ax = anchor[0] as f32;
            let ay = anchor[1] as f32;
            let sx = scale[0] as f32;
            let sy = scale[1] as f32;
            let p0 = egui::pos2(
                ax + (orig_bbox.min.x - ax) * sx,
                ay + (orig_bbox.min.y - ay) * sy,
            );
            let p1 = egui::pos2(
                ax + (orig_bbox.max.x - ax) * sx,
                ay + (orig_bbox.max.y - ay) * sy,
            );
            egui::Rect::from_two_pos(p0, p1)
        }
        _ => bbox,
    }
}

#[allow(dead_code)]
fn _mode_check(_m: LassoMode) {}
