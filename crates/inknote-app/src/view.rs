// Khung nhìn (View) của trang: zoom + kéo trang (Bước 1.8).
// Nét bút lưu theo toạ độ TRANG; View đổi qua lại giữa toạ độ trang ↔ toạ độ màn hình.

use eframe::egui::{Pos2, Rect, Vec2};

pub const MIN_ZOOM: f32 = 0.2;
pub const MAX_ZOOM: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// 1.0 = 100%
    pub zoom: f32,
    /// Vị trí gốc trang (0,0) so với góc trên-trái của vùng giấy trên màn hình.
    pub pan: Vec2,
}

impl Default for View {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
        }
    }
}

impl View {
    /// Gốc trang trên màn hình.
    pub fn origin(&self, paper: Rect) -> Pos2 {
        paper.min + self.pan
    }

    #[cfg(test)]
    pub fn to_page(&self, paper: Rect, screen: Pos2) -> Pos2 {
        ((screen - self.origin(paper)) / self.zoom).to_pos2()
    }

    #[cfg(test)]
    pub fn to_screen(&self, paper: Rect, page: Pos2) -> Pos2 {
        self.origin(paper) + page.to_vec2() * self.zoom
    }

    #[cfg(test)]
    pub fn rect_to_screen(&self, paper: Rect, page: Rect) -> Rect {
        Rect::from_min_max(
            self.to_screen(paper, page.min),
            self.to_screen(paper, page.max),
        )
    }

    /// Zoom nhân với `factor`, giữ nguyên điểm trang đang nằm dưới `anchor` (toạ độ màn hình).
    pub fn zoom_at(&mut self, paper: Rect, anchor: Pos2, factor: f32) {
        let new_zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let rel = anchor - paper.min;
        self.pan = rel - (rel - self.pan) * (new_zoom / self.zoom);
        self.zoom = new_zoom;
    }

    pub fn zoom_percent(&self) -> f32 {
        self.zoom * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, vec2};

    fn paper() -> Rect {
        Rect::from_min_max(pos2(20.0, 100.0), pos2(1180.0, 780.0))
    }

    #[test]
    fn page_screen_roundtrip() {
        let v = View {
            zoom: 3.0,
            pan: vec2(-50.0, 30.0),
        };
        let p = pos2(123.0, 456.0);
        let back = v.to_page(paper(), v.to_screen(paper(), p));
        assert!((back.x - p.x).abs() < 1e-3 && (back.y - p.y).abs() < 1e-3);
    }

    #[test]
    fn zoom_keeps_point_under_cursor() {
        let mut v = View::default();
        let anchor = pos2(400.0, 300.0);
        let before = v.to_page(paper(), anchor);
        v.zoom_at(paper(), anchor, 3.0);
        assert!((v.zoom - 3.0).abs() < 1e-6);
        let after = v.to_page(paper(), anchor);
        assert!((before.x - after.x).abs() < 1e-3 && (before.y - after.y).abs() < 1e-3);
    }

    #[test]
    fn zoom_is_clamped() {
        let mut v = View::default();
        v.zoom_at(paper(), pos2(100.0, 200.0), 1000.0);
        assert_eq!(v.zoom, MAX_ZOOM);
        v.zoom_at(paper(), pos2(100.0, 200.0), 0.0001);
        assert_eq!(v.zoom, MIN_ZOOM);
    }
}
