// Hình học cơ bản & mở rộng (Bước 5.3 - P2):
// - Hỗ trợ 18 loại hình: đường thẳng/mũi tên 1-2 chiều, khối cơ bản, tam giác (cân/đều/vuông/vuông cân),
//   đa giác (ngũ/lục/thất/bát giác), và tứ giác (thoi, bình hành, hình thang thường/vuông/cân).
// - Giữ Ctrl khi kéo để căn góc đặc trưng (0°, 30°, 45°, 60°, 90°, ...) hoặc hình đều chuẩn.

use eframe::egui::{
    self, Color32, Painter, Pos2, Rect, Stroke, StrokeKind, epaint::PathShape, pos2, vec2,
};
use inknote_core::{ShapeKind, ShapeObject};

use crate::lasso::LassoInteraction;

/// 6 hình hay dùng hiển thị trực tiếp trên thanh công cụ ngữ cảnh.
pub const QUICK_SHAPE_KINDS: [ShapeKind; 6] = [
    ShapeKind::Line,
    ShapeKind::Arrow,
    ShapeKind::DoubleArrow,
    ShapeKind::Rectangle,
    ShapeKind::Circle,
    ShapeKind::Triangle,
];

/// Toàn bộ 18 loại hình chia nhóm rõ ràng trong Pop-up chọn hình.
pub const SHAPE_GROUPS: [(&str, &[ShapeKind]); 5] = [
    (
        "Lines & Arrows",
        &[ShapeKind::Line, ShapeKind::Arrow, ShapeKind::DoubleArrow],
    ),
    (
        "Basic Shapes",
        &[
            ShapeKind::Rectangle,
            ShapeKind::RoundedRect,
            ShapeKind::Circle,
        ],
    ),
    (
        "Triangles",
        &[
            ShapeKind::Triangle,
            ShapeKind::RightTriangle,
            ShapeKind::RightIsoscelesTriangle,
        ],
    ),
    (
        "Polygons (5–8 sides)",
        &[
            ShapeKind::Pentagon,
            ShapeKind::Hexagon,
            ShapeKind::Heptagon,
            ShapeKind::Octagon,
        ],
    ),
    (
        "Quadrilaterals",
        &[
            ShapeKind::Diamond,
            ShapeKind::Parallelogram,
            ShapeKind::Trapezoid,
            ShapeKind::RightTrapezoid,
            ShapeKind::IsoscelesTrapezoid,
        ],
    ),
];

/// Tên tiếng Anh + gợi ý phím Ctrl cho từng loại hình (hiển thị trong tooltip).
pub fn shape_kind_label(kind: ShapeKind) -> &'static str {
    match kind {
        ShapeKind::Line => "Line (Ctrl: 0° / 30° / 45° / 60° / 90°)",
        ShapeKind::Arrow => "Arrow (Ctrl: 0° / 30° / 45° / 60° / 90°)",
        ShapeKind::DoubleArrow => "Double Arrow (Ctrl: 0° / 30° / 45° / 60° / 90°)",
        ShapeKind::Rectangle => "Rectangle (Ctrl: Square)",
        ShapeKind::RoundedRect => "Rounded Rectangle (Ctrl: Rounded Square)",
        ShapeKind::Circle => "Ellipse / Circle (Ctrl: Perfect Circle)",
        ShapeKind::Triangle => "Isosceles Triangle (Ctrl: Equilateral Triangle)",
        ShapeKind::RightTriangle => "Right Triangle (Ctrl: Right Isosceles)",
        ShapeKind::RightIsoscelesTriangle => "Right Isosceles Triangle (45°-45°-90°)",
        ShapeKind::Diamond => "Diamond / Rhombus (Ctrl: 1:1)",
        ShapeKind::Pentagon => "Pentagon (Ctrl: Regular)",
        ShapeKind::Hexagon => "Hexagon (Ctrl: Regular)",
        ShapeKind::Heptagon => "Heptagon (Ctrl: Regular)",
        ShapeKind::Octagon => "Octagon (Ctrl: Regular)",
        ShapeKind::Parallelogram => "Parallelogram (Ctrl: 1:1)",
        ShapeKind::Trapezoid => "Trapezoid (Ctrl: 1:1)",
        ShapeKind::RightTrapezoid => "Right Trapezoid (Ctrl: 1:1)",
        ShapeKind::IsoscelesTrapezoid => "Isosceles Trapezoid (Ctrl: 1:1)",
    }
}

fn constrain_square(start: Pos2, pos: Pos2) -> Pos2 {
    let d = pos - start;
    let side = d.x.abs().max(d.y.abs());
    let sx = if d.x >= 0.0 { 1.0 } else { -1.0 };
    let sy = if d.y >= 0.0 { 1.0 } else { -1.0 };
    pos2(start.x + side * sx, start.y + side * sy)
}

fn snap_characteristic_angle(start: Pos2, pos: Pos2) -> Pos2 {
    const SNAP_DEGREES: [f32; 5] = [0.0, 30.0, 45.0, 60.0, 90.0];
    let d = pos - start;
    let len = d.length();
    if len < 1e-3 {
        return pos;
    }
    let sx = if d.x >= 0.0 { 1.0 } else { -1.0 };
    let sy = if d.y >= 0.0 { 1.0 } else { -1.0 };
    let deg = d.y.abs().atan2(d.x.abs()).to_degrees();

    let mut best_deg = 0.0f32;
    let mut best_diff = f32::MAX;
    for candidate in SNAP_DEGREES {
        let diff = (deg - candidate).abs();
        if diff < best_diff {
            best_diff = diff;
            best_deg = candidate;
        }
    }

    let (cos, sin) = if best_deg == 0.0 {
        (1.0, 0.0)
    } else if best_deg == 90.0 {
        (0.0, 1.0)
    } else {
        let rad = best_deg.to_radians();
        (rad.cos(), rad.sin())
    };
    start + vec2(sx * len * cos, sy * len * sin)
}

/// Ràng buộc góc đặc trưng (`0°/30°/45°/60°/90°`) hoặc tỉ lệ hình chuẩn khi giữ `Ctrl`.
pub fn constrain_shape_end(kind: ShapeKind, start: Pos2, pos: Pos2, ctrl_held: bool) -> Pos2 {
    match kind {
        ShapeKind::RightIsoscelesTriangle => constrain_square(start, pos),
        ShapeKind::Line | ShapeKind::Arrow | ShapeKind::DoubleArrow => {
            if ctrl_held {
                snap_characteristic_angle(start, pos)
            } else {
                pos
            }
        }
        ShapeKind::Triangle => {
            if !ctrl_held {
                return pos;
            }
            // Tam giác đều: chiều cao h = cạnh * sqrt(3) / 2
            let d = pos - start;
            let sx = if d.x >= 0.0 { 1.0 } else { -1.0 };
            let sy = if d.y >= 0.0 { 1.0 } else { -1.0 };
            let k = 3.0_f32.sqrt() * 0.5;
            let side = d.x.abs().max(d.y.abs() / k);
            pos2(start.x + sx * side, start.y + sy * side * k)
        }
        ShapeKind::Rectangle
        | ShapeKind::RoundedRect
        | ShapeKind::Circle
        | ShapeKind::RightTriangle
        | ShapeKind::Diamond
        | ShapeKind::Pentagon
        | ShapeKind::Hexagon
        | ShapeKind::Heptagon
        | ShapeKind::Octagon
        | ShapeKind::Parallelogram
        | ShapeKind::Trapezoid
        | ShapeKind::RightTrapezoid
        | ShapeKind::IsoscelesTrapezoid => {
            if ctrl_held {
                constrain_square(start, pos)
            } else {
                pos
            }
        }
    }
}

/// Trạng thái kéo vẽ một hình học đang diễn ra trên trang (chưa thả bút/chuột).
#[derive(Clone, Debug)]
pub struct ActiveShapeDrag {
    pub page_index: usize,
    pub kind: ShapeKind,
    pub start: Pos2,
    raw_current: Pos2,
    pub current: Pos2,
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
            raw_current: start,
            current: start,
            color,
            stroke_width,
            dashed,
        }
    }

    pub fn update(&mut self, page_pos: Pos2, ctrl_held: bool) {
        self.raw_current = page_pos;
        self.reapply_constraint(ctrl_held);
    }

    pub fn reapply_constraint(&mut self, ctrl_held: bool) {
        self.current = constrain_shape_end(self.kind, self.start, self.raw_current, ctrl_held);
    }

    pub fn to_preview_object(&self) -> ShapeObject {
        ShapeObject {
            id: 0,
            kind: self.kind,
            start: [self.start.x as f64, self.start.y as f64],
            end: [self.current.x as f64, self.current.y as f64],
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

fn regular_polygon_points(rect: Rect, sides: usize, start_angle: f32) -> Vec<Pos2> {
    let center = rect.center();
    let rx = rect.width() * 0.5;
    let ry = rect.height() * 0.5;
    (0..sides)
        .map(|i| {
            let a = start_angle + std::f32::consts::TAU * (i as f32 / sides as f32);
            pos2(center.x + rx * a.cos(), center.y + ry * a.sin())
        })
        .collect()
}

fn arrow_head_points(
    from: Pos2,
    tip: Pos2,
    stroke_width: f64,
    is_double: bool,
) -> Option<[Pos2; 3]> {
    let d = tip - from;
    let len = d.length();
    if len < 3.0 {
        return None;
    }
    let ux = d.x / len;
    let uy = d.y / len;
    let max_ratio = if is_double { 0.34 } else { 0.45 };
    let head_len = ((stroke_width as f32) * 3.2 + 9.0)
        .clamp(10.0, 26.0)
        .min(len * max_ratio);
    let angle = 28.0_f32.to_radians();
    let cos = angle.cos();
    let sin = angle.sin();
    let w1 = pos2(
        tip.x - head_len * (ux * cos - uy * sin),
        tip.y - head_len * (ux * sin + uy * cos),
    );
    let w2 = pos2(
        tip.x - head_len * (ux * cos + uy * sin),
        tip.y - head_len * (-ux * sin + uy * cos),
    );
    Some([w1, tip, w2])
}

pub fn shape_arrow_heads(
    kind: ShapeKind,
    start: Pos2,
    end: Pos2,
    stroke_width: f64,
) -> Vec<[Pos2; 3]> {
    let mut heads = Vec::new();
    match kind {
        ShapeKind::Arrow => {
            if let Some(h) = arrow_head_points(start, end, stroke_width, false) {
                heads.push(h);
            }
        }
        ShapeKind::DoubleArrow => {
            if let Some(h) = arrow_head_points(start, end, stroke_width, true) {
                heads.push(h);
            }
            if let Some(h) = arrow_head_points(end, start, stroke_width, true) {
                heads.push(h);
            }
        }
        _ => {}
    }
    heads
}

/// Tạo danh sách điểm viền (page-local) và cờ `closed` dùng chung cho render, bbox và Lasso.
pub fn shape_outline_points(kind: ShapeKind, start: Pos2, end: Pos2) -> (Vec<Pos2>, bool) {
    let rect = Rect::from_two_pos(start, end);
    let w = rect.width();
    match kind {
        ShapeKind::Line | ShapeKind::Arrow | ShapeKind::DoubleArrow => (vec![start, end], false),
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
        ShapeKind::Circle => (regular_polygon_points(rect, 64, 0.0), true),
        ShapeKind::Triangle => (
            vec![
                pos2(rect.center().x, rect.top()),
                rect.right_bottom(),
                rect.left_bottom(),
            ],
            true,
        ),
        ShapeKind::RightTriangle | ShapeKind::RightIsoscelesTriangle => (
            vec![rect.left_top(), rect.right_bottom(), rect.left_bottom()],
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
        ShapeKind::Pentagon => (
            regular_polygon_points(rect, 5, -std::f32::consts::FRAC_PI_2),
            true,
        ),
        ShapeKind::Hexagon => (
            regular_polygon_points(rect, 6, -std::f32::consts::FRAC_PI_2),
            true,
        ),
        ShapeKind::Heptagon => (
            regular_polygon_points(rect, 7, -std::f32::consts::FRAC_PI_2),
            true,
        ),
        ShapeKind::Octagon => (
            // Xoay lệch 22.5° để đỉnh trên & đáy nằm ngang cân đối
            regular_polygon_points(
                rect,
                8,
                -std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_8,
            ),
            true,
        ),
        ShapeKind::Parallelogram => {
            let skew = w * 0.22;
            vec![
                pos2(rect.left() + skew, rect.top()),
                rect.right_top(),
                pos2(rect.right() - skew, rect.bottom()),
                rect.left_bottom(),
            ]
            .into_pair_closed()
        }
        ShapeKind::Trapezoid => vec![
            pos2(rect.left() + w * 0.12, rect.top()),
            pos2(rect.right() - w * 0.40, rect.top()),
            rect.right_bottom(),
            rect.left_bottom(),
        ]
        .into_pair_closed(),
        ShapeKind::RightTrapezoid => vec![
            rect.left_top(),
            pos2(rect.right() - w * 0.30, rect.top()),
            rect.right_bottom(),
            rect.left_bottom(),
        ]
        .into_pair_closed(),
        ShapeKind::IsoscelesTrapezoid => vec![
            pos2(rect.left() + w * 0.22, rect.top()),
            pos2(rect.right() - w * 0.22, rect.top()),
            rect.right_bottom(),
            rect.left_bottom(),
        ]
        .into_pair_closed(),
    }
}

trait ClosedPolygonExt {
    fn into_pair_closed(self) -> (Vec<Pos2>, bool);
}

impl ClosedPolygonExt for Vec<Pos2> {
    fn into_pair_closed(self) -> (Vec<Pos2>, bool) {
        (self, true)
    }
}

/// Khung bao (page-local) của `ShapeObject` dùng cho chọn vùng Lasso & hiển thị khung chọn.
pub fn shape_page_bbox(shape: &ShapeObject) -> Rect {
    let start = pos2(shape.start[0] as f32, shape.start[1] as f32);
    let end = pos2(shape.end[0] as f32, shape.end[1] as f32);
    let (pts, _) = shape_outline_points(shape.kind, start, end);
    let mut r = Rect::NOTHING;
    for p in pts {
        r.extend_with(p);
    }
    for [w1, tip, w2] in shape_arrow_heads(shape.kind, start, end, shape.stroke_width) {
        r.extend_with(w1);
        r.extend_with(tip);
        r.extend_with(w2);
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
    let (pts, closed) = shape_outline_points(shape.kind, start, end);
    let mut edges = Vec::with_capacity(pts.len() + 6);
    for w in pts.windows(2) {
        edges.push([w[0], w[1]]);
    }
    if closed && pts.len() >= 2 {
        edges.push([pts[pts.len() - 1], pts[0]]);
    }
    for [w1, tip, w2] in shape_arrow_heads(shape.kind, start, end, shape.stroke_width) {
        edges.push([w1, tip]);
        edges.push([tip, w2]);
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
    let bbox = shape_page_bbox(shape);
    poly.iter().any(|&p| bbox.contains(p))
        && (crate::lasso::point_in_polygon(bbox.center(), poly)
            || edges
                .iter()
                .any(|&[a, b]| crate::lasso::point_in_polygon(a + (b - a) * 0.5, poly)))
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

    let color = Color32::from_rgba_unmultiplied(
        shape.color[0],
        shape.color[1],
        shape.color[2],
        shape.color[3],
    );
    let screen_width = (shape.stroke_width as f32 * zoom).max(0.8);
    let stroke = Stroke::new(screen_width, color);
    let to_screen = |p: Pos2| origin + p.to_vec2() * zoom;

    let (page_pts, closed) = shape_outline_points(shape.kind, start, end);
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

    for [w1, tip, w2] in shape_arrow_heads(shape.kind, start, end, shape.stroke_width) {
        painter.add(egui::Shape::Path(PathShape::line(
            vec![to_screen(w1), to_screen(tip), to_screen(w2)],
            stroke,
        )));
    }
}

fn draw_right_angle_mark(painter: &Painter, corner: Pos2, size: f32, color: Color32) {
    let s = Stroke::new(1.1, color);
    painter.line_segment(
        [
            pos2(corner.x, corner.y - size),
            pos2(corner.x + size, corner.y - size),
        ],
        s,
    );
    painter.line_segment(
        [
            pos2(corner.x + size, corner.y - size),
            pos2(corner.x + size, corner.y),
        ],
        s,
    );
}

/// Vẽ icon thu nhỏ sắc nét cho từng loại hình trên Thanh tuỳ chọn ngữ cảnh & Pop-up.
pub fn draw_shape_kind_icon(painter: &Painter, rect: Rect, kind: ShapeKind, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.37;
    let stroke = Stroke::new(1.55, color);

    match kind {
        ShapeKind::Line => {
            painter.line_segment([pos2(c.x - s, c.y + s), pos2(c.x + s, c.y - s)], stroke);
        }
        ShapeKind::Arrow => {
            let p0 = pos2(c.x - s, c.y + s * 0.85);
            let p1 = pos2(c.x + s, c.y - s * 0.85);
            painter.line_segment([p0, p1], stroke);
            painter.line_segment([p1, pos2(p1.x - s * 0.7, p1.y)], stroke);
            painter.line_segment([p1, pos2(p1.x, p1.y + s * 0.7)], stroke);
        }
        ShapeKind::DoubleArrow => {
            let p0 = pos2(c.x - s * 0.92, c.y + s * 0.85);
            let p1 = pos2(c.x + s * 0.92, c.y - s * 0.85);
            painter.line_segment([p0, p1], stroke);
            // Đầu mũi tên phải trên
            painter.line_segment([p1, pos2(p1.x - s * 0.65, p1.y)], stroke);
            painter.line_segment([p1, pos2(p1.x, p1.y + s * 0.65)], stroke);
            // Đầu mũi tên trái dưới
            painter.line_segment([p0, pos2(p0.x + s * 0.65, p0.y)], stroke);
            painter.line_segment([p0, pos2(p0.x, p0.y - s * 0.65)], stroke);
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
        _ => {
            let box_size = match kind {
                ShapeKind::RightTriangle => vec2(s * 2.0, s * 1.28),
                ShapeKind::RightIsoscelesTriangle => vec2(s * 1.62, s * 1.62),
                ShapeKind::Triangle => vec2(s * 1.85, s * 1.60),
                ShapeKind::Pentagon
                | ShapeKind::Hexagon
                | ShapeKind::Heptagon
                | ShapeKind::Octagon => vec2(s * 1.88, s * 1.88),
                _ => vec2(s * 1.92, s * 1.42),
            };
            let icon_box = Rect::from_center_size(c, box_size);
            let (pts, _) = shape_outline_points(kind, icon_box.left_top(), icon_box.right_bottom());
            painter.add(egui::Shape::Path(PathShape::closed_line(pts, stroke)));

            if matches!(
                kind,
                ShapeKind::RightTriangle
                    | ShapeKind::RightIsoscelesTriangle
                    | ShapeKind::RightTrapezoid
            ) {
                draw_right_angle_mark(painter, icon_box.left_bottom(), s * 0.42, color);
            }
        }
    }
}
