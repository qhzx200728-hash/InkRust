// Bộ nhớ đệm texture SVG theo mức zoom (Bước 4.1) để vẽ công thức LaTeX sắc nét ở mọi độ phóng đại.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use eframe::egui::{self, ColorImage, Pos2, Rect, TextureHandle, TextureOptions};
use inknote_latex::{compile_latex_to_svg, render_svg_to_rgba, CompileOptions, LatexError};

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
    /// Lấy hoặc rasterize lại SVG thành texture sắc nét theo `zoom * pixels_per_point`.
    pub fn get_or_render(
        &mut self,
        ctx: &egui::Context,
        id: u64,
        svg: &str,
        zoom: f32,
    ) -> Result<(egui::TextureId, egui::Vec2), LatexError> {
        let ppp = ctx.pixels_per_point();
        let target_scale = (zoom * ppp).clamp(0.5, 8.0);
        // Lượng tử hoá theo bước 0.25x để không rasterize lại liên tục khi zoom mượt
        let bucket = (target_scale * 4.0).round().max(2.0) as u32;

        if let Some(entry) = self.entries.get(&id) {
            if entry.scale_bucket == bucket {
                return Ok((entry.texture.id(), entry.logical_size));
            }
        }

        let render_scale = bucket as f32 / 4.0;
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

/// Trạng thái kiểm tra nhanh Bước 4.1 (render mẫu + test timeout trên luồng nền).
#[derive(Default)]
pub struct LatexSmokeState {
    pub sample_svg: Option<String>,
    pub sample_page_idx: usize,
    pub status_msg: Option<String>,
    pub is_error: bool,
    pub compiling: bool,
    rx: Option<Receiver<Result<String, LatexError>>>,
}

impl LatexSmokeState {
    pub fn start_compile(&mut self, ctx: &egui::Context, source: String, timeout: Duration) {
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.compiling = true;
        self.is_error = false;
        self.status_msg = Some("Compiling LaTeX...".to_string());
        let ctx_clone = ctx.clone();

        thread::spawn(move || {
            let opts = CompileOptions {
                timeout,
                ..Default::default()
            };
            let res = compile_latex_to_svg(&source, &opts);
            let _ = tx.send(res);
            ctx_clone.request_repaint();
        });
    }

    pub fn poll(&mut self, cache: &mut SvgTextureCache) {
        let Some(rx) = self.rx.as_ref() else {
            return;
        };
        if let Ok(res) = rx.try_recv() {
            self.compiling = false;
            self.rx = None;
            match res {
                Ok(svg) => {
                    self.sample_svg = Some(svg);
                    cache.invalidate(1);
                    self.is_error = false;
                    self.status_msg =
                        Some("Rendered SVG! Zoom to 300% to verify sharpness.".to_string());
                }
                Err(err) => {
                    self.is_error = true;
                    self.status_msg = Some(err.to_string());
                }
            }
        }
    }

    /// Vẽ SVG mẫu lên trang giấy ở toạ độ trang `(80, 120)` để kiểm tra độ sắc nét khi zoom.
    pub fn paint_preview(
        &self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        page_origin: Pos2,
        zoom: f32,
        cache: &mut SvgTextureCache,
    ) {
        let Some(svg) = self.sample_svg.as_deref() else {
            return;
        };
        if let Ok((tex_id, logical_size)) = cache.get_or_render(ctx, 1, svg, zoom) {
            // Phóng kích thước hiển thị hợp lý trên trang (1.6x pt -> đơn vị trang)
            let display_size = logical_size * 1.6 * zoom;
            let top_left = page_origin + egui::vec2(80.0 * zoom, 120.0 * zoom);
            let rect = Rect::from_min_size(top_left, display_size);
            painter.image(
                tex_id,
                rect,
                Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
    }
}

