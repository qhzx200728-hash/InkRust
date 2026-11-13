// Logic công cụ Tẩy (Bước 5.1):
// - EraserMode::Standard: tẩy từng đoạn nét đi qua vùng tẩy (chia tách nét nếu tẩy ở giữa).
// - EraserMode::Stroke: chạm bất kỳ đâu trên nét là xoá cả nét đó.
// - Không bao giờ tác động lên LaTeX, hình vẽ hay ảnh (chỉ xử lý `strokes`).

use std::collections::{HashMap, HashSet};

use eframe::egui;
use inknote_core::{Command, Document, Stroke};
use inknote_pen::{PenPath, style::Composer};
use inknote_render::CairoRenderContext;
use parry2d_f64::bounding_volume::Aabb;
use parry2d_f64::math::Vector2;

use crate::{CachedStroke, toolbar::EraserMode};

/// Trạng thái một lượt kéo tẩy đang diễn ra trên một trang.
pub struct ActiveEraser {
    pub page_index: usize,
    pub last_page_pos: egui::Pos2,
    pub original_strokes: Vec<Stroke>,
    pub working_strokes: Vec<Stroke>,
    pub temp_batches: HashMap<u64, CachedStroke>,
    next_temp_id: u64,
}

impl ActiveEraser {
    pub fn new(page_index: usize, start_page_pos: egui::Pos2, strokes: Vec<Stroke>) -> Self {
        Self {
            page_index,
            last_page_pos: start_page_pos,
            working_strokes: strokes.clone(),
            original_strokes: strokes,
            temp_batches: HashMap::new(),
            next_temp_id: u64::MAX / 2,
        }
    }

    fn alloc_temp_id(&mut self) -> u64 {
        self.next_temp_id = self.next_temp_id.wrapping_add(1);
        self.next_temp_id
    }
}

fn dist_sq_point_to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let abx = b[0] - a[0];
    let aby = b[1] - a[1];
    let apx = p[0] - a[0];
    let apy = p[1] - a[1];
    let len_sq = abx * abx + aby * aby;
    if len_sq <= 1e-9 {
        return apx * apx + apy * apy;
    }
    let t = ((apx * abx + apy * aby) / len_sq).clamp(0.0, 1.0);
    let cx = a[0] + t * abx;
    let cy = a[1] + t * aby;
    let dx = p[0] - cx;
    let dy = p[1] - cy;
    dx * dx + dy * dy
}

fn stroke_half_width(stroke: &Stroke) -> f64 {
    (stroke.style.stroke_width() * 0.5).max(1.0)
}

fn build_sweep_aabb(from: [f64; 2], to: [f64; 2], pad: f64) -> Aabb {
    let min_x = from[0].min(to[0]) - pad;
    let min_y = from[1].min(to[1]) - pad;
    let max_x = from[0].max(to[0]) + pad;
    let max_y = from[1].max(to[1]) + pad;
    Aabb::new(Vector2::new(min_x, min_y), Vector2::new(max_x, max_y))
}

/// Quét tẩy dọc đoạn `from -> to` (toạ độ trang) và cập nhật `eraser.working_strokes`.
pub fn apply_eraser_sweep(
    eraser: &mut ActiveEraser,
    from: egui::Pos2,
    to: egui::Pos2,
    radius_page: f32,
    mode: EraserMode,
    render_cx: &mut CairoRenderContext,
) {
    let from_p = [from.x as f64, from.y as f64];
    let to_p = [to.x as f64, to.y as f64];
    let r = (radius_page as f64).max(2.0);
    let sweep_aabb = build_sweep_aabb(from_p, to_p, r + 4.0);

    let prev_strokes = std::mem::take(&mut eraser.working_strokes);
    let mut next_strokes = Vec::with_capacity(prev_strokes.len());

    for stroke in prev_strokes {
        let hit_r = r + stroke_half_width(&stroke);
        if stroke.path.hittest(&sweep_aabb, hit_r).is_empty() {
            next_strokes.push(stroke);
            continue;
        }

        match mode {
            EraserMode::Stroke => {
                // Tẩy nét: chạm vào bất kỳ đoạn nào -> xoá cả nét
                eraser.temp_batches.remove(&stroke.id);
            }
            EraserMode::Standard => {
                // Tẩy thường: chỉ xoá các điểm/đoạn nằm trong bán kính tẩy
                let hit_r_sq = hit_r * hit_r;
                let elements = stroke.path.clone().into_elements();
                if elements.len() <= 1 {
                    let keep = elements.first().is_some_and(|el| {
                        dist_sq_point_to_segment([el.pos.x, el.pos.y], from_p, to_p) > hit_r_sq
                    });
                    if keep {
                        next_strokes.push(stroke);
                    } else {
                        eraser.temp_batches.remove(&stroke.id);
                    }
                    continue;
                }

                let mut runs: Vec<Vec<inknote_pen::penpath::Element>> = Vec::new();
                let mut current: Vec<inknote_pen::penpath::Element> = Vec::new();
                let mut any_erased = false;

                for el in &elements {
                    let erased =
                        dist_sq_point_to_segment([el.pos.x, el.pos.y], from_p, to_p) <= hit_r_sq;
                    if erased {
                        any_erased = true;
                        if current.len() >= 2 {
                            runs.push(std::mem::take(&mut current));
                        } else {
                            current.clear();
                        }
                    } else {
                        current.push(*el);
                    }
                }
                if current.len() >= 2 {
                    runs.push(current);
                }

                if !any_erased {
                    next_strokes.push(stroke);
                    continue;
                }

                eraser.temp_batches.remove(&stroke.id);
                for run in runs {
                    if let Some(path) = PenPath::try_from_elements(run) {
                        let temp_id = eraser.alloc_temp_id();
                        render_cx.clear_batches();
                        path.draw_composed(render_cx, &stroke.style);
                        let batches = std::mem::take(&mut render_cx.batches);
                        eraser
                            .temp_batches
                            .insert(temp_id, CachedStroke::new(batches));
                        next_strokes.push(Stroke {
                            id: temp_id,
                            path,
                            style: stroke.style.clone(),
                        });
                    }
                }
            }
        }
    }

    eraser.working_strokes = next_strokes;
    eraser.last_page_pos = to;
}

/// Chuyển kết quả cuối lượt kéo tẩy thành 1 `Command::EraseStrokes` duy nhất (1 bước Undo).
pub fn finish_eraser_to_command(eraser: ActiveEraser, doc: &mut Document) -> Option<Command> {
    let surviving_ids: HashSet<u64> = eraser.working_strokes.iter().map(|s| s.id).collect();
    let original_ids: HashSet<u64> = eraser.original_strokes.iter().map(|s| s.id).collect();

    let removed: Vec<(usize, Stroke)> = eraser
        .original_strokes
        .into_iter()
        .enumerate()
        .filter(|(_, s)| !surviving_ids.contains(&s.id))
        .collect();

    let added: Vec<Stroke> = eraser
        .working_strokes
        .into_iter()
        .filter(|s| !original_ids.contains(&s.id))
        .map(|mut s| {
            s.id = doc.alloc_id();
            s
        })
        .collect();

    if removed.is_empty() && added.is_empty() {
        return None;
    }

    Some(Command::EraseStrokes {
        page_index: eraser.page_index,
        removed,
        added,
    })
}
