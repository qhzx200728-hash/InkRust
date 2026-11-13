// Hình học cơ bản (Bước 5.3 - P2):
// - Quản lý kéo vẽ trực tiếp 8 loại hình (Line, Curve, Rectangle, Circle, Triangle, Diamond, RoundedRect, Arrow).
// - Tính toán đường viền, đường đứt nét, khung bao (bbox), kiểm tra giao cắt Lasso và vẽ vector mượt.

use eframe::egui::{
    self, Color32, Painter, Pos2, Rect, Stroke, StrokeKind, epaint::PathShape, pos2, vec2,
};
use inknote_core::{ShapeKind, ShapeObject};

use crate::lasso::LassoInteraction;

/// Danh sách đầy đủ 8 loại hình học hiển thị trên thanh công cụ ngữ cảnh.
pub const ALL_SHAPE_KINDS: [ShapeKind; 8] = [
    ShapeKind::Line,
    ShapeKind::Curve,
    ShapeKind::Arrow,
    ShapeKind::Rectangle,
    ShapeKind::RoundedRect,
    ShapeKind::Circle,
    ShapeKind::Triangle,
    ShapeKind::Diamond,
];

/// Tên tiếng Anh ngắn gọn của từng loại hình (hiển thị trong tooltip).
pub fn shape_kind_label(kind: ShapeKind) -> &'static str {
    match kind {
        ShapeKind::Line => "Line",
        ShapeKind::Curve => "Curve",
        ShapeKind::Arrow => "Arrow",
        ShapeKind::Rectangle => "Rectangle",
        ShapeKind::RoundedRect => "Rounded Rectangle",
        ShapeKind::Circle => "Circle / Ellipse",
        ShapeKind::Triangle => "Triangle",
        ShapeKind::Diamond => "Diamond",
    }
}

/// Ràng buộc tỷ lệ đều (vuông / tròn / góc 45°) khi người dùng giữ phím Shift lúc kéo vẽ.
pub fn constrain_shape_end(kind: ShapeKind, start: Pos2, pos: Pos2, shift_held: bool) -> Pos2 {
    if !shift_held {
        return pos;
    }
    let d = pos - start;
    match kind {
        ShapeKind::Line | ShapeKind::Curve | ShapeKind::Arrow => {
            let len = d.length();
            if len < 1e-3 {
                return pos;
            }
            let step = std::f32::consts::FRAC_PI_4;
            let snapped = (d.y.atan2(d.x) / step).round() * step;
            start + vec2(snapped.cos(), snapped.sin()) * len
        }
        ShapeKind::Rectangle
        | ShapeKind::RoundedRect
        | ShapeKind::Circle
        | ShapeKind::Triangle
        | ShapeKind::Diamond => {
            let side = d.x.abs().max(d.y.abs());
            let sx = if d.x >= 0.0 { 1.0 } else { -1.0 };
            let sy = if d.y >= 0.0 { 1.0 } else { -1.0 };
            pos2(start.x + side * sx, start.y + side * sy)
        }
    }
}

/// Tính điểm điều khiển mặc định cho đường cong Bézier bậc hai từ 2 đầu mút.
pub fn default_curve_control(start: Pos2, end: Pos2) -> Pos2 {
    let d = end - start;
    let len = d.length();
    if len < 1e-3 {
        return start;
    }
    let mid = start + d * 0.5;
    // Luôn tạo độ cong vồng tự nhiên (hướng lên khi kéo ngang, hướng sang phải khi kéo dọc).
    let mut perp = vec2(d.y / len, -d.x / len);
    if perp.y > 0.0 || (perp.y.abs() < 1e-4 && perp.x < 0.0) {
        perp = -perp;
    }
    let offset = (len * 0.28).clamp(8.0, 96.0);
    mid + perp * offset
}

/// Trạng thái kéo vẽ một hình học đang diễn ra trên trang (chưa thả bút/chuột).
#[derive(Clone, Debug)]
pub struct ActiveShapeDrag {
    pub page_index: usize,
    pub kind: ShapeKind,
    pub start: Pos2,
    pub current: Pos2,
    samples: Vec<Pos2>,
    pub color: [u8; 4],
    pub stroke_width: f64,
    pub dashed: bool,
}

impl ActiveShapeDrag {
    pub fn new(
        page_index: usize,
        kind: ShapeKind,
        start: Pos2,
        color: [u8; 4],
        stroke_width: f64,
        dashed: bool,
    ) -> Self {
        Self {
            page_index,
            kind,
            start,
            current: start,
            samples: vec![start],
            color,
            stroke_width,
            dashed,
        }
    }

    pub fn update(&mut self, page_pos: Pos2, shift_held: bool) {
        let constrained = constrain_shape_end(self.kind, self.start, page_pos, shift_held);
        self.current = constrained;
        if self.kind == ShapeKind::Curve {
            let should_push = self
                .samples
                .last()
                .is_none_or(|p| p.distance_sq(constrained) >= 25.0);
            if should_push && self.samples.len() < 128 {
                self.samples.push(constrained);
            }
        }
    }

    fn effective_curve_control(&self) -> Pos2 {
        let line = self.current - self.start;
        let len_sq = line.length_sq();
        if len_sq < 4.0 {
            return default_curve_control(self.start, self.current);
        }
        let len = len_sq.sqrt();
        let mut best_dist = 0.0f32;
        let mut best_pt = self.start;
        for &p in &self.samples {
            let v = p - self.start;
            let cross = (line.x * v.y - line.y * v.x).abs() / len;
            let proj = (v.x * line.x + v.y * line.y) / len_sq;
            if proj > 0.08 && proj < 0.92 && cross > best_dist {
                best_dist = cross;
                best_pt = p;
            }
        }
        // Nếu người dùng cố ý uốn cong đường kéo (>= 10 px lệch khỏi đường thẳng),
        // suy ra điểm điều khiển sao cho đường Bézier đi qua đỉnh uốn đó.
        if best_dist >= 10.0 {
            let mid = self.start + line * 0.5;
            pos2(2.0 * best_pt.x - mid.x, 2.0 * best_pt.y - mid.y)
        } else {
            default_curve_control(self.start, self.current)
        }
    }

    pub fn to_preview_object(&self) -> ShapeObject {
        let control = if self.kind == ShapeKind::Curve {
            let c = self.effective_curve_control();
            Some([c.x as f64, c.y as f64])
        } else {
            None
        };
        ShapeObject {
            id: 0,
            kind: self.kind,
            start: [self.start.x as f64, self.start.y as f64],
            end: [self.current.x as f64, self.current.y as f64],
            control,
            color: self.color,
            stroke_width: self.stroke_width,
            dashed: self.dashed,
        }
    }

    pub fn finish(&self, id: u64) -> Option<ShapeObject> {
        if self.start.distance(self.current) < 4.0 {
            return None;
        }
        let mut obj = self.to_preview_object();
        obj.id = id;
        Some(obj)
    }
}

fn rounded_rect_points(rect: Rect) -> Vec<Pos2> {
    let w = rect.width();
    let h = rect.height();
    let r = (w.min(h) * 0.18).min(24.0);
    if r < 1.5 {
        return vec![
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ];
    }
    let corners = [
        (
            pos2(rect.right() - r, rect.top() + r),
            -std::f32::consts::FRAC_PI_2,
            0.0,
        ),
        (
            pos2(rect.right() - r, rect.bottom() - r),
            0.0,
            std::f32::consts::FRAC_PI_2,
        ),
        (
            pos2(rect.left() + r, rect.bottom() - r),
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
        ),
        (
            pos2(rect.left() + r, rect.top() + r),
            std::f32::consts::PI,
            std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
        ),
    ];
    let steps = 7;
    let mut pts = Vec::with_capacity(corners.len() * (steps + 1));
    for (center, a0, a1) in corners {
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let a = a0 + (a1 - a0) * t;
            pts.push(pos2(center.x + r * a.cos(), center.y + r * a.sin()));
        }
    }
    pts
}

fn arrow_head_points(start: Pos2, end: Pos2, stroke_width: f64) -> Option<[Pos2; 3]> {
    let d = end - start;
    let len = d.length();
    if len < 3.0 {
        return None;
    }
    let ux = d.x / len;
    let uy = d.y / len;
    let head_len = ((stroke_width as f32) * 3.2 + 9.0)
        .clamp(10.0, 26.0)
        .min(len * 0.45);
    let angle = 28.0_f32.to_radians();
    let cos = angle.cos();
    let sin = angle.sin();
    // Hai cánh mũi tên đối xứng quanh trục ngược -u
    let w1 = pos2(
        end.x - head_len * (ux * cos - uy * sin),
        end.y - head_len * (ux * sin + uy * cos),
    );
    let w2 = pos2(
        end.x - head_len * (ux * cos + uy * sin),
        end.y - head_len * (-ux * sin + uy * cos),
    );
    Some([w1, end, w2])
}

/// Tạo danh sách điểm viền (page-local) và cờ `closed` dùng chung cho render, bbox và Lasso.
pub fn shape_outline_points(
    kind: ShapeKind,
    start: Pos2,
    end: Pos2,
    control: Option<Pos2>,
) -> (Vec<Pos2>, bool) {
    let rect = Rect::from_two_pos(start, end);
    match kind {
        ShapeKind::Line | ShapeKind::Arrow => (vec![start, end], false),
        ShapeKind::Curve => {
            let c = control.unwrap_or_else(|| default_curve_control(start, end));
            let steps = 32;
            let mut pts = Vec::with_capacity(steps + 1);
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let omt = 1.0 - t;
                let x = omt * omt * start.x + 2.0 * omt * t * c.x + t * t * end.x;
                let y = omt * omt * start.y + 2.0 * omt * t * c.y + t * t * end.y;
                pts.push(pos2(x, y));
            }
            (pts, false)
        }
        ShapeKind::Rectangle => (
            vec![
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
            ],
            true,
        ),
        ShapeKind::RoundedRect => (rounded_rect_points(rect), true),
        ShapeKind::Circle => {
            let center = rect.center();
            let rx = rect.width() * 0.5;
            let ry = rect.height() * 0.5;
            let steps = 48;
            let mut pts = Vec::with_capacity(steps);
            for i in 0..steps {
                let a = std::f32::consts::TAU * (i as f32 / steps as f32);
                pts.push(pos2(center.x + rx * a.cos(), center.y + ry * a.sin()));
            }
            (pts, true)
        }
        ShapeKind::Triangle => (
            vec![
                pos2(rect.center().x, rect.top()),
                rect.right_bottom(),
                rect.left_bottom(),
            ],
            true,
        ),
        ShapeKind::Diamond => (
            vec![
                pos2(rect.center().x, rect.top()),
                pos2(rect.right(), rect.center().y),
                pos2(rect.center().x, rect.bottom()),
                pos2(rect.left(), rect.center().y),
            ],
            true,
        ),
    }
}

/// Khung bao (page-local) của `ShapeObject` dùng cho chọn vùng Lasso & hiển thị khung chọn.
pub fn shape_page_bbox(shape: &ShapeObject) -> Rect {
    let start = pos2(shape.start[0] as f32, shape.start[1] as f32);
    let end = pos2(shape.end[0] as f32, shape.end[1] as f32);
    let control = shape.control.map(|c| pos2(c[0] as f32, c[1] as f32));
    let (pts, _) = shape_outline_points(shape.kind, start, end, control);
    let mut r = Rect::NOTHING;
    for p in pts {
        r.extend_with(p);
    }
    if shape.kind == ShapeKind::Arrow {
        if let Some([w1, tip, w2]) = arrow_head_points(start, end, shape.stroke_width) {
            r.extend_with(w1);
            r.extend_with(tip);
            r.extend_with(w2);
        }
    }
    r.expand((shape.stroke_width as f32 * 0.5).max(2.0))
}

fn ccw(a: Pos2, b: Pos2, c: Pos2) -> f32 {
    (c.y - a.y) * (b.x - a.x) - (b.y - a.y) * (c.x - a.x)
}

fn segments_intersect(a: Pos2, b: Pos2, c: Pos2, d: Pos2) -> bool {
    let d1 = ccw(a, c, d);
    let d2 = ccw(b, c, d);
    let d3 = ccw(a, b, c);
    let d4 = ccw(a, b, d);
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

fn iter_shape_edges(shape: &ShapeObject) -> Vec<[Pos2; 2]> {
    let start = pos2(shape.start[0] as f32, shape.start[1] as f32);
    let end = pos2(shape.end[0] as f32, shape.end[1] as f32);
    let control = shape.control.map(|c| pos2(c[0] as f32, c[1] as f32));
    let (pts, closed) = shape_outline_points(shape.kind, start, end, control);
    let mut edges = Vec::with_capacity(pts.len() + 3);
    for w in pts.windows(2) {
        edges.push([w[0], w[1]]);
    }
    if closed && pts.len() >= 2 {
        edges.push([pts[pts.len() - 1], pts[0]]);
    }
    if shape.kind == ShapeKind::Arrow {
        if let Some([w1, tip, w2]) = arrow_head_points(start, end, shape.stroke_width) {
            edges.push([w1, tip]);
            edges.push([tip, w2]);
        }
    }
    edges
}

/// Kiểm tra hình có chạm hoặc nằm trong đa giác Lasso tự do hay không.
pub fn shape_intersects_polygon(shape: &ShapeObject, poly: &[Pos2]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let edges = iter_shape_edges(shape);
    for &[a, b] in &edges {
        if crate::lasso::point_in_polygon(a, poly) || crate::lasso::point_in_polygon(b, poly) {
            return true;
        }
        for i in 0..poly.len() {
            let j = (i + 1) % poly.len();
            if segments_intersect(a, b, poly[i], poly[j]) {
                return true;
            }
        }
    }
    // Hỗ trợ trường hợp Lasso nằm lọt hoàn toàn bên trong một hình lớn (hoặc tâm hình nằm trong đa giác)
    let bbox = shape_page_bbox(shape);
    poly.iter().any(|&p| bbox.contains(p)) && {
        // Kiểm tra tâm đối với hình đóng hoặc đỉnh gần nhất
        crate::lasso::point_in_polygon(bbox.center(), poly)
            || edges
                .iter()
                .any(|&[a, b]| crate::lasso::point_in_polygon(a + (b - a) * 0.5, poly))
    }
}

/// Kiểm tra hình có giao với hình chữ nhật chọn Lasso hay không.
pub fn shape_intersects_rect(shape: &ShapeObject, rect: Rect) -> bool {
    if !shape_page_bbox(shape).intersects(rect) {
        return false;
    }
    let edges = iter_shape_edges(shape);
    let rect_edges = [
        [rect.left_top(), rect.right_top()],
        [rect.right_top(), rect.right_bottom()],
        [rect.right_bottom(), rect.left_bottom()],
        [rect.left_bottom(), rect.left_top()],
    ];
    for &[a, b] in &edges {
        if rect.contains(a) || rect.contains(b) {
            return true;
        }
        if rect_edges
            .iter()
            .any(|&[c, d]| segments_intersect(a, b, c, d))
        {
            return true;
        }
    }
    false
}

/// Vẽ đường đa giác/gấp khúc nét đứt liên tục qua các góc.
fn stroke_dashed_polyline(
    painter: &Painter,
    pts: &[Pos2],
    closed: bool,
    dash_len: f32,
    gap_len: f32,
    stroke: Stroke,
) {
    if pts.len() < 2 {
        return;
    }
    let dash_len = dash_len.max(4.0);
    let gap_len = gap_len.max(3.0);
    let mut in_dash = true;
    let mut rem = dash_len;

    let edge_count = if closed { pts.len() } else { pts.len() - 1 };
    for idx in 0..edge_count {
        let a = pts[idx];
        let b = pts[(idx + 1) % pts.len()];
        let seg = b - a;
        let seg_len = seg.length();
        if seg_len < 1e-4 {
            continue;
        }
        let dir = seg / seg_len;
        let mut cursor = 0.0f32;
        while cursor < seg_len {
            let take = rem.min(seg_len - cursor);
            if in_dash && take > 0.25 {
                let p0 = a + dir * cursor;
                let p1 = a + dir * (cursor + take);
                painter.line_segment([p0, p1], stroke);
            }
            cursor += take;
            rem -= take;
            if rem <= 1e-3 {
                in_dash = !in_dash;
                rem = if in_dash { dash_len } else { gap_len };
            }
        }
    }
}

fn transform_point(pt: Pos2, interaction: Option<&LassoInteraction>) -> Pos2 {
    match interaction {
        Some(LassoInteraction::Moving { delta, .. }) => pt + *delta,
        Some(LassoInteraction::Resizing { anchor, scale, .. }) => {
            let ax = anchor[0] as f32;
            let ay = anchor[1] as f32;
            let sx = scale[0] as f32;
            let sy = scale[1] as f32;
            pos2(ax + (pt.x - ax) * sx, ay + (pt.y - ay) * sy)
        }
        _ => pt,
    }
}

/// Vẽ một `ShapeObject` lên `painter` theo hệ toạ độ trang (`origin`, `zoom`), có áp dụng biến đổi Lasso trực tiếp nếu đang kéo.
pub fn paint_shape(
    painter: &Painter,
    shape: &ShapeObject,
    origin: Pos2,
    zoom: f32,
    interaction: Option<&LassoInteraction>,
) {
    let start = transform_point(
        pos2(shape.start[0] as f32, shape.start[1] as f32),
        interaction,
    );
    let end = transform_point(pos2(shape.end[0] as f32, shape.end[1] as f32), interaction);
    let control = shape
        .control
        .map(|c| transform_point(pos2(c[0] as f32, c[1] as f32), interaction));

    let color = Color32::from_rgba_unmultiplied(
        shape.color[0],
        shape.color[1],
        shape.color[2],
        shape.color[3],
    );
    let screen_width = (shape.stroke_width as f32 * zoom).max(0.8);
    let stroke = Stroke::new(screen_width, color);
    let to_screen = |p: Pos2| origin + p.to_vec2() * zoom;

    let (page_pts, closed) = shape_outline_points(shape.kind, start, end, control);
    let screen_pts: Vec<Pos2> = page_pts.into_iter().map(to_screen).collect();

    if shape.dashed {
        stroke_dashed_polyline(
            painter,
            &screen_pts,
            closed,
            (screen_width * 3.0).max(8.0),
            (screen_width * 2.0).max(5.5),
            stroke,
        );
    } else {
        match shape.kind {
            ShapeKind::Rectangle => {
                let rect = Rect::from_two_pos(to_screen(start), to_screen(end));
                painter.rect_stroke(rect, 0.0, stroke, StrokeKind::Middle);
            }
            ShapeKind::RoundedRect => {
                let rect = Rect::from_two_pos(to_screen(start), to_screen(end));
                let r = (rect.width().min(rect.height()) * 0.18).min(24.0 * zoom);
                painter.rect_stroke(rect, r, stroke, StrokeKind::Middle);
            }
            _ => {
                if screen_pts.len() >= 2 {
                    let path = if closed {
                        PathShape::closed_line(screen_pts, stroke)
                    } else {
                        PathShape::line(screen_pts, stroke)
                    };
                    painter.add(egui::Shape::Path(path));
                }
            }
        }
    }

    if shape.kind == ShapeKind::Arrow {
        if let Some([w1, tip, w2]) = arrow_head_points(start, end, shape.stroke_width) {
            painter.add(egui::Shape::Path(PathShape::line(
                vec![to_screen(w1), to_screen(tip), to_screen(w2)],
                stroke,
            )));
        }
    }
}

/// Vẽ icon thu nhỏ sắc nét cho từng loại hình trên Thanh tuỳ chọn ngữ cảnh.
pub fn draw_shape_kind_icon(painter: &Painter, rect: Rect, kind: ShapeKind, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.36;
    let stroke = Stroke::new(1.6, color);

    match kind {
        ShapeKind::Line => {
            painter.line_segment([pos2(c.x - s, c.y + s), pos2(c.x + s, c.y - s)], stroke);
        }
        ShapeKind::Curve => {
            let pts = [
                pos2(c.x - s, c.y + s * 0.55),
                pos2(c.x - s * 0.35, c.y - s * 0.75),
                pos2(c.x + s * 0.35, c.y - s * 0.75),
                pos2(c.x + s, c.y + s * 0.55),
            ];
            painter.add(egui::Shape::Path(PathShape::line(pts.to_vec(), stroke)));
        }
        ShapeKind::Arrow => {
            let p0 = pos2(c.x - s, c.y + s * 0.85);
            let p1 = pos2(c.x + s, c.y - s * 0.85);
            painter.line_segment([p0, p1], stroke);
            painter.line_segment([p1, pos2(p1.x - s * 0.72, p1.y)], stroke);
            painter.line_segment([p1, pos2(p1.x, p1.y + s * 0.72)], stroke);
        }
        ShapeKind::Rectangle => {
            let r = Rect::from_center_size(c, vec2(s * 1.85, s * 1.45));
            painter.rect_stroke(r, 1.0, stroke, StrokeKind::Middle);
        }
        ShapeKind::RoundedRect => {
            let r = Rect::from_center_size(c, vec2(s * 1.85, s * 1.45));
            painter.rect_stroke(r, 3.5, stroke, StrokeKind::Middle);
        }
        ShapeKind::Circle => {
            painter.circle_stroke(c, s * 0.88, stroke);
        }
        ShapeKind::Triangle => {
            let pts = vec![
                pos2(c.x, c.y - s * 0.88),
                pos2(c.x + s * 0.92, c.y + s * 0.8),
                pos2(c.x - s * 0.92, c.y + s * 0.8),
            ];
            painter.add(egui::Shape::Path(PathShape::closed_line(pts, stroke)));
        }
        ShapeKind::Diamond => {
            let pts = vec![
                pos2(c.x, c.y - s * 0.95),
                pos2(c.x + s * 0.95, c.y),
                pos2(c.x, c.y + s * 0.95),
                pos2(c.x - s * 0.95, c.y),
            ];
            painter.add(egui::Shape::Path(PathShape::closed_line(pts, stroke)));
        }
    }
}
