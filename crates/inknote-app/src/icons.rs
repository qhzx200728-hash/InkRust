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
            let body = Rect::from_min_max(
                pos2(c.x - s * 0.55, top_y),
                pos2(c.x + s * 0.55, c.y + s * 0.82),
            );
            painter.rect_stroke(body, 1.5, stroke, egui::StrokeKind::Middle);
        }
    }
}

/// Vẽ chấm lưới cho nền giấy Dotted Paper
pub fn draw_dotted_paper(
    painter: &Painter,
    paper_rect: Rect,
    paper_bg: Color32,
    border_color: Color32,
    dot_color: Color32,
    spacing: f32,
    dot_radius: f32,
) {
    painter.rect_filled(paper_rect, 4.0, paper_bg);
    painter.rect_stroke(
        paper_rect,
        4.0,
        Stroke::new(1.0, border_color),
        egui::StrokeKind::Middle,
    );

    if spacing < 6.0 {
        return;
    }

    let margin = spacing;
    let x_start = paper_rect.left() + margin;
    let x_end = paper_rect.right() - margin;
    let y_start = paper_rect.top() + margin;
    let y_end = paper_rect.bottom() - margin;

    let mut y = y_start;
    while y <= y_end {
        let mut x = x_start;
        while x <= x_end {
            painter.circle_filled(Pos2::new(x, y), dot_radius, dot_color);
            x += spacing;
        }
        y += spacing;
    }
}
