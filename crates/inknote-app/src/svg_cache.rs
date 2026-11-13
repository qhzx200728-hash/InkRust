// Bộ nhớ đệm texture SVG theo mức zoom (Bước 4.1 & 4.3) để vẽ công thức LaTeX sắc nét ở mọi độ phóng đại.

use std::collections::HashMap;

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use inknote_latex::{
    LatexError, RenderedSvg, compute_clamped_svg_scale, render_svg_for_display_size,
    render_svg_to_rgba,
};

pub const LATEX_DISPLAY_SCALE: f32 = 1.6;

struct CachedSvgTexture {
    svg_hash: u64,
    scale_bucket: u32,
    logical_size: egui::Vec2,
    texture: TextureHandle,
}

fn fnv1a_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in s.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[derive(Default)]
pub struct SvgTextureCache {
    entries: HashMap<u64, CachedSvgTexture>,
}

impl SvgTextureCache {
    fn store_rendered(
        &mut self,
        ctx: &egui::Context,
        id: u64,
        svg_hash: u64,
        bucket: u32,
        rendered: RenderedSvg,
    ) -> (egui::TextureId, egui::Vec2) {
        let image = ColorImage::from_rgba_unmultiplied(
            [rendered.width_px as usize, rendered.height_px as usize],
            &rendered.rgba,
        );
        let tex_name = format!("latex_svg_{id}_{svg_hash}_{bucket}");
        let texture = ctx.load_texture(tex_name, image, TextureOptions::LINEAR);
        let logical_size = egui::vec2(rendered.logical_width, rendered.logical_height);
        let tex_id = texture.id();

        self.entries.insert(
            id,
            CachedSvgTexture {
                svg_hash,
                scale_bucket: bucket,
                logical_size,
                texture,
            },
        );
        (tex_id, logical_size)
    }

    /// Lấy hoặc rasterize lại SVG thành texture sắc nét theo `zoom * pixels_per_point` (có supersample 2x).
    pub fn get_or_render(
        &mut self,
        ctx: &egui::Context,
        id: u64,
        svg: &str,
        zoom: f32,
    ) -> Result<(egui::TextureId, egui::Vec2), LatexError> {
        let ppp = ctx.pixels_per_point();
        let target_scale = (zoom * ppp * LATEX_DISPLAY_SCALE * 2.0).clamp(2.0, 24.0);
        // Lượng tử hoá theo bước 0.5x để không rasterize lại liên tục khi zoom mượt
        let bucket = (target_scale * 2.0).round().max(4.0) as u32;
        let svg_hash = fnv1a_hash(svg);

        if let Some(entry) = self.entries.get(&id) {
            if entry.scale_bucket == bucket && entry.svg_hash == svg_hash {
                return Ok((entry.texture.id(), entry.logical_size));
            }
        }

        let render_scale = bucket as f32 / 2.0;
        let rendered = render_svg_to_rgba(svg, render_scale)?;
        Ok(self.store_rendered(ctx, id, svg_hash, bucket, rendered))
    }

    /// Lấy hoặc rasterize lại SVG theo kích thước hiển thị thực tế trên màn hình (kể cả khi dùng Lasso phóng to).
    pub fn get_or_render_for_display_size(
        &mut self,
        ctx: &egui::Context,
        id: u64,
        svg: &str,
        display_size_screen: egui::Vec2,
    ) -> Result<(egui::TextureId, egui::Vec2), LatexError> {
        let ppp = ctx.pixels_per_point();
        let oversample = (ppp * 2.0).max(2.0);
        let svg_hash = fnv1a_hash(svg);

        if let Some(entry) = self.entries.get(&id) {
            if entry.svg_hash == svg_hash {
                let raw_scale = (display_size_screen.x.max(1.0) / entry.logical_size.x.max(1.0))
                    .max(display_size_screen.y.max(1.0) / entry.logical_size.y.max(1.0))
                    * oversample;
                let fit_scale = compute_clamped_svg_scale(
                    entry.logical_size.x,
                    entry.logical_size.y,
                    raw_scale,
                );
                let bucket = (fit_scale * 2.0).round().max(4.0) as u32;
                if entry.scale_bucket == bucket {
                    return Ok((entry.texture.id(), entry.logical_size));
                }
                let render_scale = bucket as f32 / 2.0;
                let rendered = render_svg_to_rgba(svg, render_scale)?;
                return Ok(self.store_rendered(ctx, id, svg_hash, bucket, rendered));
            }
        }

        let rendered = render_svg_for_display_size(
            svg,
            display_size_screen.x.max(1.0),
            display_size_screen.y.max(1.0),
            oversample,
        )?;
        let raw_scale = (display_size_screen.x.max(1.0) / rendered.logical_width.max(1.0))
            .max(display_size_screen.y.max(1.0) / rendered.logical_height.max(1.0))
            * oversample;
        let fit_scale =
            compute_clamped_svg_scale(rendered.logical_width, rendered.logical_height, raw_scale);
        let bucket = (fit_scale * 2.0).round().max(4.0) as u32;
        Ok(self.store_rendered(ctx, id, svg_hash, bucket, rendered))
    }

    pub fn invalidate(&mut self, id: u64) {
        self.entries.remove(&id);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enlarge_display_size_re_rasterizes_at_higher_resolution() {
        let ctx = egui::Context::default();
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20"><rect width="40" height="20" fill="black"/></svg>"#;
        let mut cache = SvgTextureCache::default();

        let _ = cache
            .get_or_render_for_display_size(&ctx, 1, svg, egui::vec2(64.0, 32.0))
            .unwrap();
        let small_px = cache.entries.get(&1).unwrap().texture.size()[0];

        // Phóng to kích thước hiển thị lên 5x (ví dụ kéo bằng Lasso) -> phải render texture nét hơn hẳn
        let _ = cache
            .get_or_render_for_display_size(&ctx, 1, svg, egui::vec2(320.0, 160.0))
            .unwrap();
        let large_px = cache.entries.get(&1).unwrap().texture.size()[0];

        assert!(
            large_px >= small_px * 3,
            "expected higher resolution after Lasso enlarge: small={small_px}, large={large_px}"
        );
    }
}
