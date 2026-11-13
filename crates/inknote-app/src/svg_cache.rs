// Bộ nhớ đệm texture SVG theo mức zoom (Bước 4.1 & 4.3) để vẽ công thức LaTeX sắc nét ở mọi độ phóng đại.

use std::collections::HashMap;

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use inknote_latex::{render_svg_to_rgba, LatexError};

pub const LATEX_DISPLAY_SCALE: f32 = 1.6;

struct CachedSvgTexture {
    scale_bucket: u32,
    logical_size: egui::Vec2,
    texture: TextureHandle,
}

#[derive(Default)]
pub struct SvgTextureCache {
    entries: HashMap<u64, CachedSvgTexture>,
}

impl SvgTextureCache {
    /// Lấy hoặc rasterize lại SVG thành texture sắc nét theo `zoom * pixels_per_point` (có supersample 2x).
    pub fn get_or_render(
        &mut self,
        ctx: &egui::Context,
        id: u64,
        svg: &str,
        zoom: f32,
    ) -> Result<(egui::TextureId, egui::Vec2), LatexError> {
        let ppp = ctx.pixels_per_point();
        let target_scale = (zoom * ppp * LATEX_DISPLAY_SCALE * 2.0).clamp(2.0, 12.0);
        // Lượng tử hoá theo bước 0.5x để không rasterize lại liên tục khi zoom mượt
        let bucket = (target_scale * 2.0).round().max(4.0) as u32;

        if let Some(entry) = self.entries.get(&id) {
            if entry.scale_bucket == bucket {
                return Ok((entry.texture.id(), entry.logical_size));
            }
        }

        let render_scale = bucket as f32 / 2.0;
        let rendered = render_svg_to_rgba(svg, render_scale)?;
        let image = ColorImage::from_rgba_unmultiplied(
            [rendered.width_px as usize, rendered.height_px as usize],
            &rendered.rgba,
        );
        let tex_name = format!("latex_svg_{id}_{bucket}");
        let texture = ctx.load_texture(tex_name, image, TextureOptions::LINEAR);
        let logical_size = egui::vec2(rendered.logical_width, rendered.logical_height);
        let tex_id = texture.id();

        self.entries.insert(
            id,
            CachedSvgTexture {
                scale_bucket: bucket,
                logical_size,
                texture,
            },
        );
        Ok((tex_id, logical_size))
    }

    pub fn invalidate(&mut self, id: u64) {
        self.entries.remove(&id);
    }
}
