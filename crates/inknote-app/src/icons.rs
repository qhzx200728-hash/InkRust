// Vẽ các biểu tượng (icons) vector đồng đều, nét mảnh 1.8px theo phong cách Lucide Icons
// cho Thanh công cụ chính và Thanh tuỳ chọn của InkNote.

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Stroke, pos2, vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolIcon {
    // Nhóm Trái (Navigation Toolbar)
    Home,
    Pages,
    ReadOnly,
    // Nhóm Giữa (Main Tools)
    Lasso,
    Pen,
    Eraser,
    Text,
    Image,
    Shapes,
    Latex,
    // Nhóm Phải (Actions)
    AddPage,
    Export,
    More,
    // Biểu tượng phụ trên Thanh tuỳ chọn công cụ
    SolidBrush,
    MarkerBrush,
    TexturedBrush,
    Sliders,
    Palette,
    Undo,
    Redo,
    Trash,
    // Biểu tượng cho màn hình Library (Bước 7.1 & 7.2)
    Documents,
    BackArrow,
    Search,
    Settings,
    FolderPlus,
    NotePlus,
    Star,
    StarFilled,
    Restore,
    ChevronDown,
}

pub fn draw_icon(painter: &Painter, rect: Rect, icon: ToolIcon, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.42;
    let stroke = Stroke::new(1.75, color);

    match icon {
        ToolIcon::Home => {
            // Mái nhà tam giác + thân nhà
            let roof_top = pos2(c.x, c.y - s * 0.95);
            let roof_l = pos2(c.x - s * 0.95, c.y - s * 0.1);
            let roof_r = pos2(c.x + s * 0.95, c.y - s * 0.1);
            painter.line_segment([roof_l, roof_top], stroke);
            painter.line_segment([roof_top, roof_r], stroke);

            let body = Rect::from_min_max(
                pos2(c.x - s * 0.68, c.y - s * 0.15),
                pos2(c.x + s * 0.68, c.y + s * 0.9),
            );
            painter.line_segment([body.left_top(), body.left_bottom()], stroke);
            painter.line_segment([body.left_bottom(), body.right_bottom()], stroke);
            painter.line_segment([body.right_bottom(), body.right_top()], stroke);
        }
        ToolIcon::Pages => {
            // Hai trang giấy xếp chồng (Thumbnails)
            let back = Rect::from_min_max(
                pos2(c.x - s * 0.45, c.y - s * 0.9),
                pos2(c.x + s * 0.85, c.y + s * 0.6),
            );
            let front = Rect::from_min_max(
                pos2(c.x - s * 0.85, c.y - s * 0.5),
                pos2(c.x + s * 0.45, c.y + s * 0.95),
            );
            painter.rect_stroke(back, 2.0, stroke, egui::StrokeKind::Middle);
            painter.rect_filled(
                front,
                2.0,
                Color32::from_rgba_unmultiplied(24, 90, 188, 220),
            );
            painter.rect_stroke(front, 2.0, stroke, egui::StrokeKind::Middle);
        }
        ToolIcon::ReadOnly => {
            // Biểu tượng con mắt (Read-only / View mode)
            let top_arc = [
                pos2(c.x - s * 0.95, c.y),
                pos2(c.x - s * 0.45, c.y - s * 0.6),
                pos2(c.x + s * 0.45, c.y - s * 0.6),
                pos2(c.x + s * 0.95, c.y),
            ];
            let bot_arc = [
                pos2(c.x - s * 0.95, c.y),
                pos2(c.x - s * 0.45, c.y + s * 0.6),
                pos2(c.x + s * 0.45, c.y + s * 0.6),
                pos2(c.x + s * 0.95, c.y),
            ];
            for w in top_arc.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
            for w in bot_arc.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
            painter.circle_stroke(c, s * 0.28, stroke);
        }
        ToolIcon::Lasso => {
            // Vòng tròn nét đứt + nút thắt dây ở góc dưới
            let n = 10;
            let r = s * 0.82;
            for i in 0..n {
                let a0 = (i as f32 / n as f32) * std::f32::consts::TAU;
                let a1 = a0 + (std::f32::consts::TAU / n as f32) * 0.55;
                let p0 = pos2(c.x + r * a0.cos(), c.y - s * 0.1 + r * 0.78 * a0.sin());
                let p1 = pos2(c.x + r * a1.cos(), c.y - s * 0.1 + r * 0.78 * a1.sin());
                painter.line_segment([p0, p1], stroke);
            }
            painter.circle_stroke(pos2(c.x - s * 0.35, c.y + s * 0.65), s * 0.22, stroke);
        }
        ToolIcon::Pen | ToolIcon::SolidBrush => {
            // Cây bút nghiêng 45 độ
            let tip = pos2(c.x - s * 0.85, c.y + s * 0.85);
            let p1 = pos2(c.x - s * 0.7, c.y + s * 0.25);
            let p2 = pos2(c.x + s * 0.45, c.y - s * 0.9);
            let p3 = pos2(c.x + s * 0.9, c.y - s * 0.45);
            let p4 = pos2(c.x - s * 0.25, c.y + s * 0.7);
            painter.line_segment([tip, p1], stroke);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p4], stroke);
            painter.line_segment([p4, tip], stroke);
            painter.line_segment([p1, p4], stroke);
        }
        ToolIcon::MarkerBrush => {
            // Bút dạ quang bản rộng, đầu vát
            let tip_a = pos2(c.x - s * 0.85, c.y + s * 0.85);
            let tip_b = pos2(c.x - s * 0.35, c.y + s * 0.85);
            let body_l = pos2(c.x - s * 0.65, c.y + s * 0.25);
            let body_r = pos2(c.x - s * 0.1, c.y + s * 0.55);
            let top_l = pos2(c.x + s * 0.25, c.y - s * 0.85);
            let top_r = pos2(c.x + s * 0.85, c.y - s * 0.35);
            painter.line_segment([tip_a, tip_b], Stroke::new(2.4, color));
            painter.line_segment([tip_a, body_l], stroke);
            painter.line_segment([tip_b, body_r], stroke);
            painter.line_segment([body_l, top_l], stroke);
            painter.line_segment([body_r, top_r], stroke);
            painter.line_segment([top_l, top_r], stroke);
        }
        ToolIcon::TexturedBrush => {
            // Cây bút + các hạt chấm xung quanh
            draw_icon(painter, rect, ToolIcon::SolidBrush, color);
            for (dx, dy) in [(-0.75, -0.55), (-0.35, -0.75), (0.65, 0.45), (0.35, 0.75)] {
                painter.circle_filled(pos2(c.x + s * dx, c.y + s * dy), 1.5, color);
            }
        }
        ToolIcon::Eraser => {
            // Cục tẩy nghiêng có vạch chia hai nửa + đường gạch chân
            let a = pos2(c.x - s * 0.85, c.y + s * 0.35);
            let b = pos2(c.x + s * 0.15, c.y - s * 0.75);
            let d = pos2(c.x + s * 0.85, c.y - s * 0.05);
            let e = pos2(c.x - s * 0.15, c.y + s * 0.85);
            painter.line_segment([a, b], stroke);
            painter.line_segment([b, d], stroke);
            painter.line_segment([d, e], stroke);
            painter.line_segment([e, a], stroke);
            painter.line_segment(
                [
                    pos2(c.x - s * 0.35, c.y - s * 0.2),
                    pos2(c.x + s * 0.35, c.y + s * 0.45),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(c.x - s * 0.85, c.y + s * 0.88),
                    pos2(c.x + s * 0.85, c.y + s * 0.88),
                ],
                stroke,
            );
        }
        ToolIcon::Text => {
            // Chữ T trong khung
            let box_r = Rect::from_center_size(c, vec2(s * 1.75, s * 1.75));
            painter.rect_stroke(box_r, 2.5, stroke, egui::StrokeKind::Middle);
            let top_y = c.y - s * 0.48;
            let bot_y = c.y + s * 0.55;
            painter.line_segment(
                [pos2(c.x - s * 0.48, top_y), pos2(c.x + s * 0.48, top_y)],
                stroke,
            );
            painter.line_segment([pos2(c.x, top_y), pos2(c.x, bot_y)], stroke);
        }
        ToolIcon::Image => {
            // Khung ảnh + mặt trời + đường núi
            let box_r = Rect::from_center_size(c, vec2(s * 1.8, s * 1.55));
            painter.rect_stroke(box_r, 2.5, stroke, egui::StrokeKind::Middle);
            painter.circle_filled(pos2(c.x - s * 0.4, c.y - s * 0.28), s * 0.18, color);
            let m1 = pos2(box_r.left() + 2.0, box_r.bottom() - 2.0);
            let m2 = pos2(c.x - s * 0.05, c.y + s * 0.05);
            let m3 = pos2(c.x + s * 0.3, c.y + s * 0.38);
            let m4 = pos2(c.x + s * 0.55, c.y + s * 0.12);
            let m5 = pos2(box_r.right() - 2.0, box_r.bottom() - 2.0);
            painter.line_segment([m1, m2], stroke);
            painter.line_segment([m2, m3], stroke);
            painter.line_segment([m3, m4], stroke);
            painter.line_segment([m4, m5], stroke);
        }
        ToolIcon::Shapes => {
            // Hình vuông + hình tròn giao nhau
            let sq = Rect::from_min_max(
                pos2(c.x - s * 0.88, c.y - s * 0.88),
                pos2(c.x + s * 0.22, c.y + s * 0.22),
            );
            painter.rect_stroke(sq, 1.5, stroke, egui::StrokeKind::Middle);
            painter.circle_stroke(pos2(c.x + s * 0.32, c.y + s * 0.32), s * 0.58, stroke);
        }
        ToolIcon::Latex => {
            // Ký hiệu Sigma (∑) toán học đặc trưng cho LaTeX
            let l = c.x - s * 0.65;
            let r = c.x + s * 0.65;
            let t = c.y - s * 0.78;
            let b = c.y + s * 0.78;
            let mid = pos2(c.x + s * 0.05, c.y);
            painter.line_segment([pos2(r, t + s * 0.22), pos2(r, t)], stroke);
            painter.line_segment([pos2(r, t), pos2(l, t)], stroke);
            painter.line_segment([pos2(l, t), mid], stroke);
            painter.line_segment([mid, pos2(l, b)], stroke);
            painter.line_segment([pos2(l, b), pos2(r, b)], stroke);
            painter.line_segment([pos2(r, b), pos2(r, b - s * 0.22)], stroke);
        }
        ToolIcon::AddPage => {
            // Trang giấy có dấu cộng (+) ở giữa
            let page = Rect::from_center_size(c, vec2(s * 1.45, s * 1.8));
            painter.rect_stroke(page, 2.0, stroke, egui::StrokeKind::Middle);
            painter.line_segment(
                [pos2(c.x - s * 0.38, c.y), pos2(c.x + s * 0.38, c.y)],
                stroke,
            );
            painter.line_segment(
                [pos2(c.x, c.y - s * 0.38), pos2(c.x, c.y + s * 0.38)],
                stroke,
            );
        }
        ToolIcon::Export => {
            // Khay dưới + mũi tên hướng lên (Share / Export)
            let tray_l = pos2(c.x - s * 0.78, c.y + s * 0.15);
            let tray_bl = pos2(c.x - s * 0.78, c.y + s * 0.82);
            let tray_br = pos2(c.x + s * 0.78, c.y + s * 0.82);
            let tray_r = pos2(c.x + s * 0.78, c.y + s * 0.15);
            painter.line_segment([tray_l, tray_bl], stroke);
            painter.line_segment([tray_bl, tray_br], stroke);
            painter.line_segment([tray_br, tray_r], stroke);

            let top = pos2(c.x, c.y - s * 0.85);
            painter.line_segment([pos2(c.x, c.y + s * 0.35), top], stroke);
            painter.line_segment([top, pos2(c.x - s * 0.42, c.y - s * 0.4)], stroke);
            painter.line_segment([top, pos2(c.x + s * 0.42, c.y - s * 0.4)], stroke);
        }
        ToolIcon::More => {
            // Ba chấm tròn ngang (•••)
            for dx in [-0.62, 0.0, 0.62] {
                painter.circle_filled(pos2(c.x + s * dx, c.y), 2.1, color);
            }
        }
        ToolIcon::Sliders => {
            // Ba thanh trượt ngang có nút tròn (cài đặt chi tiết)
            for (dy, knob_x) in [(-0.55, -0.25), (0.0, 0.35), (0.55, -0.1)] {
                let y = c.y + s * dy;
                painter.line_segment([pos2(c.x - s * 0.8, y), pos2(c.x + s * 0.8, y)], stroke);
                painter.circle_filled(pos2(c.x + s * knob_x, y), 2.5, color);
            }
        }
        ToolIcon::Palette => {
            // Vòng tròn bảng màu + 3 chấm nhỏ
            painter.circle_stroke(c, s * 0.85, stroke);
            for (dx, dy) in [(-0.35, -0.25), (0.0, -0.42), (0.35, -0.2)] {
                painter.circle_filled(pos2(c.x + s * dx, c.y + s * dy), 1.7, color);
            }
        }
        ToolIcon::Undo => {
            // Mũi tên vòng quay ngược
            let p_left = pos2(c.x - s * 0.75, c.y - s * 0.15);
            let p_mid = pos2(c.x + s * 0.25, c.y - s * 0.15);
            let p_bot = pos2(c.x + s * 0.65, c.y + s * 0.55);
            painter.line_segment([p_left, p_mid], stroke);
            painter.line_segment([p_mid, p_bot], stroke);
            painter.line_segment(
                [p_left, pos2(p_left.x + s * 0.4, p_left.y - s * 0.38)],
                stroke,
            );
            painter.line_segment(
                [p_left, pos2(p_left.x + s * 0.4, p_left.y + s * 0.38)],
                stroke,
            );
        }
        ToolIcon::Redo => {
            // Mũi tên vòng quay xuôi (đối xứng Undo)
            let p_right = pos2(c.x + s * 0.75, c.y - s * 0.15);
            let p_mid = pos2(c.x - s * 0.25, c.y - s * 0.15);
            let p_bot = pos2(c.x - s * 0.65, c.y + s * 0.55);
            painter.line_segment([p_right, p_mid], stroke);
            painter.line_segment([p_mid, p_bot], stroke);
            painter.line_segment(
                [p_right, pos2(p_right.x - s * 0.4, p_right.y - s * 0.38)],
                stroke,
            );
            painter.line_segment(
                [p_right, pos2(p_right.x - s * 0.4, p_right.y + s * 0.38)],
                stroke,
            );
        }
        ToolIcon::Trash => {
            // Thùng rác nhỏ
            let top_y = c.y - s * 0.55;
            painter.line_segment(
                [pos2(c.x - s * 0.75, top_y), pos2(c.x + s * 0.75, top_y)],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(c.x - s * 0.3, top_y - s * 0.28),
                    pos2(c.x + s * 0.3, top_y - s * 0.28),
                ],
                stroke,
            );
            let body = Rect::from_min_max(
                pos2(c.x - s * 0.55, top_y),
                pos2(c.x + s * 0.55, c.y + s * 0.82),
            );
            painter.rect_stroke(body, 1.5, stroke, egui::StrokeKind::Middle);
        }
        ToolIcon::Documents => {
            // Biểu tượng thư mục tài liệu (Sidebar Library)
            let pts = [
                pos2(c.x - s * 0.85, c.y - s * 0.58),
                pos2(c.x - s * 0.22, c.y - s * 0.58),
                pos2(c.x - s * 0.02, c.y - s * 0.32),
                pos2(c.x + s * 0.85, c.y - s * 0.32),
                pos2(c.x + s * 0.85, c.y + s * 0.65),
                pos2(c.x - s * 0.85, c.y + s * 0.65),
            ];
            painter.add(egui::Shape::closed_line(pts.to_vec(), stroke));
        }
        ToolIcon::BackArrow => {
            // Mũi tên quay lại (←)
            let left = pos2(c.x - s * 0.72, c.y);
            let right = pos2(c.x + s * 0.72, c.y);
            painter.line_segment([left, right], stroke);
            painter.line_segment([left, pos2(left.x + s * 0.52, c.y - s * 0.52)], stroke);
            painter.line_segment([left, pos2(left.x + s * 0.52, c.y + s * 0.52)], stroke);
        }
        ToolIcon::Search => {
            // Kính lúp tìm kiếm
            let lens_c = pos2(c.x - s * 0.14, c.y - s * 0.14);
            let r = s * 0.58;
            painter.circle_stroke(lens_c, r, stroke);
            let handle_start = lens_c + vec2(r * 0.72, r * 0.72);
            let handle_end = pos2(c.x + s * 0.82, c.y + s * 0.82);
            painter.line_segment([handle_start, handle_end], stroke);
        }
        ToolIcon::Settings => {
            // Bánh răng cài đặt (Lucide-style)
            painter.circle_stroke(c, s * 0.34, stroke);
            let r_in = s * 0.66;
            let r_out = s * 0.92;
            for i in 0..8 {
                let ang = (i as f32) * std::f32::consts::FRAC_PI_4;
                let dir = vec2(ang.cos(), ang.sin());
                painter.line_segment([c + dir * r_in, c + dir * r_out], stroke);
            }
            painter.circle_stroke(c, s * 0.64, stroke);
        }
        ToolIcon::FolderPlus => {
            // Biểu tượng thư mục kèm dấu + nhỏ
            let pts = [
                pos2(c.x - s * 0.85, c.y - s * 0.58),
                pos2(c.x - s * 0.22, c.y - s * 0.58),
                pos2(c.x - s * 0.02, c.y - s * 0.32),
                pos2(c.x + s * 0.85, c.y - s * 0.32),
                pos2(c.x + s * 0.85, c.y + s * 0.65),
                pos2(c.x - s * 0.85, c.y + s * 0.65),
            ];
            painter.add(egui::Shape::closed_line(pts.to_vec(), stroke));
            let plus_c = pos2(c.x, c.y + s * 0.16);
            let arm = s * 0.28;
            painter.line_segment(
                [
                    pos2(plus_c.x - arm, plus_c.y),
                    pos2(plus_c.x + arm, plus_c.y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(plus_c.x, plus_c.y - arm),
                    pos2(plus_c.x, plus_c.y + arm),
                ],
                stroke,
            );
        }
        ToolIcon::NotePlus => {
            // Biểu tượng trang giấy kèm dấu + nhỏ
            let page_r = Rect::from_min_max(
                pos2(c.x - s * 0.66, c.y - s * 0.82),
                pos2(c.x + s * 0.66, c.y + s * 0.82),
            );
            painter.rect_stroke(page_r, 2.0, stroke, egui::StrokeKind::Middle);
            let arm = s * 0.3;
            painter.line_segment([pos2(c.x - arm, c.y), pos2(c.x + arm, c.y)], stroke);
            painter.line_segment([pos2(c.x, c.y - arm), pos2(c.x, c.y + arm)], stroke);
        }
        ToolIcon::Star | ToolIcon::StarFilled => {
            let mut pts = Vec::with_capacity(10);
            let outer = s * 0.92;
            let inner = s * 0.42;
            for k in 0..10 {
                let angle = -std::f32::consts::FRAC_PI_2 + (k as f32) * std::f32::consts::PI / 5.0;
                let r = if k % 2 == 0 { outer } else { inner };
                pts.push(pos2(c.x + r * angle.cos(), c.y + r * angle.sin()));
            }
            if matches!(icon, ToolIcon::StarFilled) {
                let mut tris = egui::epaint::Mesh::default();
                let center_idx = tris.vertices.len() as u32;
                tris.vertices.push(egui::epaint::Vertex {
                    pos: c,
                    uv: egui::epaint::WHITE_UV,
                    color,
                });
                for p in &pts {
                    tris.vertices.push(egui::epaint::Vertex {
                        pos: *p,
                        uv: egui::epaint::WHITE_UV,
                        color,
                    });
                }
                for i in 0..10u32 {
                    let next = (i + 1) % 10;
                    tris.indices.extend_from_slice(&[
                        center_idx,
                        center_idx + 1 + i,
                        center_idx + 1 + next,
                    ]);
                }
                painter.add(egui::Shape::mesh(tris));
            } else {
                painter.add(egui::Shape::closed_line(pts, stroke));
            }
        }
        ToolIcon::Restore => {
            // Mũi tên vòng quay ngược chiều kim đồng hồ (Khôi phục từ Trash)
            let r = s * 0.78;
            let segs = 16;
            let a_start = -std::f32::consts::PI * 0.72;
            let a_end = std::f32::consts::PI * 0.85;
            let mut pts = Vec::with_capacity(segs + 1);
            for i in 0..=segs {
                let t = i as f32 / segs as f32;
                let a = a_start + (a_end - a_start) * t;
                pts.push(pos2(c.x + r * a.cos(), c.y + r * a.sin()));
            }
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
            let head = pts[0];
            painter.line_segment([head, pos2(head.x - s * 0.1, head.y - s * 0.45)], stroke);
            painter.line_segment([head, pos2(head.x + s * 0.42, head.y - s * 0.05)], stroke);
        }
        ToolIcon::ChevronDown => {
            let y_top = c.y - s * 0.25;
            let y_bot = c.y + s * 0.32;
            let p_l = pos2(c.x - s * 0.56, y_top);
            let p_m = pos2(c.x, y_bot);
            let p_r = pos2(c.x + s * 0.56, y_top);
            painter.line_segment([p_l, p_m], stroke);
            painter.line_segment([p_m, p_r], stroke);
        }
    }
}

/// Vẽ logo tự thiết kế cho Sidebar màn hình Library (cuốn sổ + nét mực xanh).
pub fn draw_library_brand_logo(painter: &Painter, rect: Rect, accent: Color32) {
    let badge = Rect::from_min_size(rect.min, vec2(rect.height(), rect.height()));
    painter.rect_filled(badge, egui::CornerRadius::same(7), accent);
    let c = badge.center();
    let r = badge.width() * 0.28;
    let pen_stroke = Stroke::new(1.8, Color32::WHITE);
    // Ngòi bút & nét lượn chữ Ink
    painter.line_segment(
        [
            pos2(c.x - r * 0.9, c.y + r * 0.85),
            pos2(c.x + r * 0.75, c.y - r * 0.85),
        ],
        pen_stroke,
    );
    painter.circle_filled(pos2(c.x - r * 0.9, c.y + r * 0.85), 1.6, Color32::WHITE);
    let wave = [
        pos2(c.x - r * 0.4, c.y + r * 0.9),
        pos2(c.x + r * 0.15, c.y + r * 0.55),
        pos2(c.x + r * 0.85, c.y + r * 0.9),
    ];
    painter.line_segment([wave[0], wave[1]], pen_stroke);
    painter.line_segment([wave[1], wave[2]], pen_stroke);
}

/// Vẽ icon thư mục vector cỡ lớn cho thẻ Grid View trong Library (không sao chép tài sản bên ngoài).
pub fn draw_library_folder_art(painter: &Painter, slot_rect: Rect, theme: &Theme) {
    let fw = (slot_rect.width() * 0.82).min(126.0);
    let fh = fw * 0.74;
    let folder = Rect::from_center_size(slot_rect.center(), vec2(fw, fh));

    // Bóng đổ nhẹ bên dưới thư mục
    painter.rect_filled(
        folder.translate(vec2(0.0, 3.0)),
        egui::CornerRadius::same(10),
        Color32::from_rgba_unmultiplied(15, 23, 42, 22),
    );

    // Gáy trên & tai thư mục (folder tab)
    let tab_w = fw * 0.42;
    let tab_h = fh * 0.20;
    let tab_rect = Rect::from_min_size(folder.min, vec2(tab_w, tab_h + 6.0));
    painter.rect_filled(tab_rect, egui::CornerRadius::same(6), theme.lib_folder_tab);

    let back_rect =
        Rect::from_min_max(pos2(folder.left(), folder.top() + tab_h * 0.42), folder.max);
    painter.rect_filled(
        back_rect,
        egui::CornerRadius::same(8),
        theme.lib_folder_back,
    );

    // Lớp tờ giấy trắng nhú nhẹ bên trong tạo chiều sâu
    let paper_peek = Rect::from_min_max(
        pos2(folder.left() + fw * 0.08, folder.top() + fh * 0.16),
        pos2(folder.right() - fw * 0.08, folder.top() + fh * 0.50),
    );
    painter.rect_filled(paper_peek, egui::CornerRadius::same(4), Color32::WHITE);

    // Mặt trước thư mục bo góc mềm
    let front_rect = Rect::from_min_max(pos2(folder.left(), folder.top() + fh * 0.24), folder.max);
    painter.rect_filled(
        front_rect,
        egui::CornerRadius::same(9),
        theme.lib_folder_fill,
    );
    // Đường highlight mảnh ở mép trên mặt trước
    painter.line_segment(
        [
            pos2(front_rect.left() + 8.0, front_rect.top() + 1.5),
            pos2(front_rect.right() - 8.0, front_rect.top() + 1.5),
        ],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 110)),
    );
}

use crate::theme::Theme;
use inknote_core::{CanvasMode, PaperPattern, PaperSettings};

/// Trả về khung chữ nhật của tờ giấy theo toạ độ màn hình (dành cho chế độ Paper).
pub fn paper_sheet_rect(origin: Pos2, settings: &PaperSettings, zoom: f32) -> Rect {
    Rect::from_min_size(
        origin,
        vec2(settings.page_width * zoom, settings.page_height * zoom),
    )
}

/// Vẽ nền trang giấy (Plain, Dotted, Grid, Lined) kèm đường lề 2 bên (Margin lines).
/// Hỗ trợ 2 chế độ: Paper (Trang giấy chuẩn có bóng đổ) và Whiteboard (Bảng trắng vô tận).
pub fn draw_paper(
    painter: &Painter,
    paper_rect: Rect,
    origin: Pos2,
    theme: &Theme,
    settings: &PaperSettings,
    zoom: f32,
) {
    let (sheet_rect, sheet_painter) = match settings.mode {
        CanvasMode::Paper => {
            let sheet = paper_sheet_rect(origin, settings, zoom);
            if !sheet.intersects(paper_rect) {
                return;
            }
            // Đổ bóng mềm tạo cảm giác tờ giấy thật nằm nổi trên mặt bàn làm việc
            let shadow1 = sheet.translate(vec2(0.0, 4.0));
            painter.rect_filled(shadow1, 4.0, Color32::from_black_alpha(20));
            let shadow2 = sheet.translate(vec2(0.0, 2.0));
            painter.rect_filled(shadow2, 2.0, Color32::from_black_alpha(30));

            // Tờ giấy trắng với viền tinh tế
            painter.rect_filled(sheet, 2.0, theme.paper_bg);
            painter.rect_stroke(
                sheet,
                2.0,
                Stroke::new(1.0, theme.paper_border),
                egui::StrokeKind::Outside,
            );

            let clipped = sheet.intersect(paper_rect);
            (sheet, painter.with_clip_rect(clipped))
        }
        CanvasMode::Whiteboard => {
            // Chế độ bảng trắng vô tận: phủ kín toàn bộ vùng làm việc, không viền, không bo góc
            painter.rect_filled(paper_rect, 0.0, theme.paper_bg);
            (paper_rect, painter.clone())
        }
    };

    let spacing = settings.dot_spacing * zoom;
    if spacing > 0.0 {
        match settings.pattern {
            PaperPattern::Plain => {}
            PaperPattern::Dotted => {
                let mut step = spacing;
                while step < 8.0 {
                    step *= 2.0;
                }
                let first_x = ((sheet_rect.left() - origin.x) / step).ceil();
                let first_y = ((sheet_rect.top() - origin.y) / step).ceil();
                let mut y = origin.y + first_y * step;
                while y <= sheet_rect.bottom() {
                    let mut x = origin.x + first_x * step;
                    while x <= sheet_rect.right() {
                        sheet_painter.circle_filled(
                            Pos2::new(x, y),
                            theme.paper_dot_radius,
                            theme.paper_dot_color,
                        );
                        x += step;
                    }
                    y += step;
                }
            }
            PaperPattern::Grid => {
                let mut step = spacing;
                while step < 8.0 {
                    step *= 2.0;
                }
                let grid_stroke = Stroke::new(1.0, theme.paper_grid_color);
                // Các đường kẻ dọc
                let first_x = ((sheet_rect.left() - origin.x) / step).ceil();
                let mut x = origin.x + first_x * step;
                while x <= sheet_rect.right() {
                    sheet_painter.line_segment(
                        [
                            Pos2::new(x, sheet_rect.top()),
                            Pos2::new(x, sheet_rect.bottom()),
                        ],
                        grid_stroke,
                    );
                    x += step;
                }
                // Các đường kẻ ngang
                let first_y = ((sheet_rect.top() - origin.y) / step).ceil();
                let mut y = origin.y + first_y * step;
                while y <= sheet_rect.bottom() {
                    sheet_painter.line_segment(
                        [
                            Pos2::new(sheet_rect.left(), y),
                            Pos2::new(sheet_rect.right(), y),
                        ],
                        grid_stroke,
                    );
                    y += step;
                }
            }
            PaperPattern::Lined => {
                let mut step = spacing;
                while step < 8.0 {
                    step *= 2.0;
                }
                let line_stroke = Stroke::new(1.0, theme.paper_line_color);
                // Các dòng kẻ ngang
                let first_y = ((sheet_rect.top() - origin.y) / step).ceil();
                let mut y = origin.y + first_y * step;
                while y <= sheet_rect.bottom() {
                    sheet_painter.line_segment(
                        [
                            Pos2::new(sheet_rect.left(), y),
                            Pos2::new(sheet_rect.right(), y),
                        ],
                        line_stroke,
                    );
                    y += step;
                }
            }
        }
    }

    // 2. Vẽ đường lề 2 bên (Margin Lines màu đỏ nhạt / hồng pastel) - chỉ dành riêng cho chế độ Paper, Whiteboard không có lề
    if settings.show_margins && settings.mode == CanvasMode::Paper {
        let margin_stroke = Stroke::new(1.5, theme.paper_margin_line_color);
        // Đường lề trái
        let left_x = origin.x + settings.left_margin * zoom;
        if left_x >= sheet_rect.left() && left_x <= sheet_rect.right() {
            sheet_painter.line_segment(
                [
                    Pos2::new(left_x, sheet_rect.top()),
                    Pos2::new(left_x, sheet_rect.bottom()),
                ],
                margin_stroke,
            );
        }
        // Đường lề phải
        let right_x = origin.x + (settings.page_width - settings.right_margin) * zoom;
        if right_x >= sheet_rect.left() && right_x <= sheet_rect.right() {
            sheet_painter.line_segment(
                [
                    Pos2::new(right_x, sheet_rect.top()),
                    Pos2::new(right_x, sheet_rect.bottom()),
                ],
                margin_stroke,
            );
        }
    }
}
