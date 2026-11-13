use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;

use eframe::egui::{
    self, Color32, ColorImage, Context, Painter, Pos2, Rect, Stroke, TextureFilter, TextureHandle,
    TextureOptions, Vec2, pos2, vec2,
};
use image::{DynamicImage, ImageFormat, RgbaImage, imageops::FilterType};
use inknote_core::ImageObject;

/// Upper bound for a single decoded GPU texture side while preserving original `ImageObject.bytes`.
const GPU_MAX_SIDE_CAP: usize = 8192;

/// Trilinear mipmapped filtering keeps high-res text images crisp across all zoom levels (25%–800%).
pub const IMAGE_TEXTURE_OPTIONS: TextureOptions =
    TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear));

/// Validated image payload ready to store inside `ImageObject`.
#[derive(Debug, Clone)]
pub struct LoadedImageData {
    pub bytes: Vec<u8>,
    pub pixel_size: [u32; 2],
}

/// Loads and validates raw encoded image bytes (PNG, JPEG, BMP, WebP, etc.) without modifying them.
pub fn load_image_from_bytes(bytes: Vec<u8>) -> Result<LoadedImageData, String> {
    if bytes.is_empty() {
        return Err("Image file is empty".to_string());
    }
    let decoded = image::load_from_memory(&bytes)
        .map_err(|err| format!("Unsupported or invalid image: {err}"))?;
    let width = decoded.width();
    let height = decoded.height();
    if width == 0 || height == 0 {
        return Err("Image has zero width or height".to_string());
    }
    Ok(LoadedImageData {
        bytes,
        pixel_size: [width, height],
    })
}

/// Reads an image file from disk while keeping its original compressed bytes intact.
pub fn load_image_from_path(path: &Path) -> Result<LoadedImageData, String> {
    let bytes = std::fs::read(path).map_err(|err| format!("Could not read image file: {err}"))?;
    load_image_from_bytes(bytes)
}

/// Encodes raw RGBA8 pixel data (such as clipboard bitmap data) into lossless PNG bytes.
pub fn encode_rgba_to_png(
    width: u32,
    height: u32,
    rgba: Vec<u8>,
) -> Result<LoadedImageData, String> {
    if width == 0 || height == 0 {
        return Err("Clipboard image has zero dimensions".to_string());
    }
    let rgba_img = RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| "Clipboard image buffer size mismatch".to_string())?;
    let mut png_bytes = Vec::new();
    DynamicImage::ImageRgba8(rgba_img)
        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .map_err(|err| format!("Failed to encode clipboard image as PNG: {err}"))?;
    Ok(LoadedImageData {
        bytes: png_bytes,
        pixel_size: [width, height],
    })
}

/// Encodes an `egui::ColorImage` (delivered by `egui::Event::PasteImage` on `Ctrl+V`) into PNG bytes.
pub fn encode_color_image_to_png(color_img: &ColorImage) -> Result<LoadedImageData, String> {
    let width = u32::try_from(color_img.width())
        .map_err(|_| "Pasted image width is too large".to_string())?;
    let height = u32::try_from(color_img.height())
        .map_err(|_| "Pasted image height is too large".to_string())?;
    let mut rgba = Vec::with_capacity(color_img.pixels.len() * 4);
    for px in &color_img.pixels {
        rgba.extend_from_slice(&px.to_srgba_unmultiplied());
    }
    encode_rgba_to_png(width, height, rgba)
}

/// Reads an image from the Windows system clipboard and encodes it as lossless PNG bytes.
pub fn read_image_from_clipboard() -> Result<LoadedImageData, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|err| format!("Could not access clipboard: {err}"))?;
    let img_data = match clipboard.get_image() {
        Ok(data) => data,
        Err(arboard::Error::ContentNotAvailable) => {
            return Err("No image found in clipboard".to_string());
        }
        Err(err) => {
            return Err(format!("Could not read image from clipboard: {err}"));
        }
    };
    let width = u32::try_from(img_data.width)
        .map_err(|_| "Clipboard image width is too large".to_string())?;
    let height = u32::try_from(img_data.height)
        .map_err(|_| "Clipboard image height is too large".to_string())?;
    encode_rgba_to_png(width, height, img_data.bytes.into_owned())
}

/// Computes a proportional initial size in page points so newly inserted images fit neatly on the page.
pub fn compute_default_image_page_size(pixel_size: [u32; 2], page_w: f64, page_h: f64) -> [f64; 2] {
    let pw = (pixel_size[0].max(1)) as f64;
    let ph = (pixel_size[1].max(1)) as f64;
    let max_w = (page_w * 0.65).max(48.0);
    let max_h = (page_h * 0.55).max(48.0);

    let mut scale = (max_w / pw).min(max_h / ph).min(1.0);
    let longest_side = pw.max(ph);
    if longest_side * scale < 72.0 {
        let min_target = 72.0_f64.min(max_w).min(max_h);
        scale = (min_target / longest_side).min(max_w / pw).min(max_h / ph);
    }

    [(pw * scale).max(16.0), (ph * scale).max(16.0)]
}

/// Returns the bounding rectangle of an image in page-local coordinates.
pub fn image_page_bbox(obj: &ImageObject) -> Rect {
    Rect::from_min_size(
        pos2(obj.pos[0] as f32, obj.pos[1] as f32),
        vec2(obj.size[0].max(1.0) as f32, obj.size[1].max(1.0) as f32),
    )
}

/// Maps an image's page-local rect from `old_bbox` to `new_bbox` during move/resize preview.
pub fn transformed_image_page_rect(obj: &ImageObject, old_bbox: Rect, new_bbox: Rect) -> Rect {
    let cur = image_page_bbox(obj);
    let ow = old_bbox.width();
    let oh = old_bbox.height();
    if ow <= 1e-4 || oh <= 1e-4 {
        let delta = new_bbox.min - old_bbox.min;
        return cur.translate(delta);
    }
    let nx0 = (cur.min.x - old_bbox.min.x) / ow;
    let ny0 = (cur.min.y - old_bbox.min.y) / oh;
    let nx1 = (cur.max.x - old_bbox.min.x) / ow;
    let ny1 = (cur.max.y - old_bbox.min.y) / oh;
    Rect::from_min_max(
        pos2(
            new_bbox.min.x + nx0 * new_bbox.width(),
            new_bbox.min.y + ny0 * new_bbox.height(),
        ),
        pos2(
            new_bbox.min.x + nx1 * new_bbox.width(),
            new_bbox.min.y + ny1 * new_bbox.height(),
        ),
    )
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

enum CachedEntry {
    Ready {
        texture: TextureHandle,
        max_side_used: usize,
        was_clamped: bool,
    },
    Failed,
}

/// Cache of full-resolution mipmapped GPU textures decoded lazily from `ImageObject.bytes`.
#[derive(Default)]
pub struct ImageTextureCache {
    entries: HashMap<(u64, usize), CachedEntry>,
}

fn resolve_gpu_max_side(ctx: &Context) -> usize {
    ctx.input(|i| i.max_texture_side)
        .clamp(512, GPU_MAX_SIDE_CAP)
}

/// Computes the GPU texture dimensions preserving aspect ratio within `gpu_max_side`.
pub fn effective_gpu_texture_size(native: [u32; 2], gpu_max_side: usize) -> [usize; 2] {
    let w = native[0].max(1) as f64;
    let h = native[1].max(1) as f64;
    let max_side = gpu_max_side.max(1) as f64;
    if w <= max_side && h <= max_side {
        return [w as usize, h as usize];
    }
    let scale = (max_side / w).min(max_side / h);
    [
        (w * scale).round().max(1.0) as usize,
        (h * scale).round().max(1.0) as usize,
    ]
}

impl ImageTextureCache {
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn get_or_load(&mut self, ctx: &Context, bytes: &[u8]) -> Option<&TextureHandle> {
        let key = (fnv1a_64(bytes), bytes.len());
        let gpu_max_side = resolve_gpu_max_side(ctx);
        let needs_decode = match self.entries.get(&key) {
            None => true,
            Some(CachedEntry::Ready {
                max_side_used,
                was_clamped,
                ..
            }) => *was_clamped && gpu_max_side > *max_side_used,
            Some(CachedEntry::Failed) => false,
        };
        if needs_decode {
            let entry = match decode_to_texture(ctx, key.0, bytes, gpu_max_side) {
                Ok((texture, was_clamped)) => CachedEntry::Ready {
                    texture,
                    max_side_used: gpu_max_side,
                    was_clamped,
                },
                Err(_) => CachedEntry::Failed,
            };
            self.entries.insert(key, entry);
        }
        match self.entries.get(&key) {
            Some(CachedEntry::Ready { texture, .. }) => Some(texture),
            _ => None,
        }
    }
}

fn decode_to_texture(
    ctx: &Context,
    hash: u64,
    bytes: &[u8],
    gpu_max_side: usize,
) -> Result<(TextureHandle, bool), String> {
    let mut img = image::load_from_memory(bytes)
        .map_err(|err| format!("Failed to decode cached image: {err}"))?;
    let [tw, th] = effective_gpu_texture_size([img.width(), img.height()], gpu_max_side);
    let was_clamped = tw as u32 != img.width() || th as u32 != img.height();
    if was_clamped {
        img = img.resize_exact(tw as u32, th as u32, FilterType::Lanczos3);
    }
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color_image = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    let name = format!(
        "inknote-img-{hash:016x}-{}-{}x{}",
        bytes.len(),
        size[0],
        size[1]
    );
    Ok((
        ctx.load_texture(name, color_image, IMAGE_TEXTURE_OPTIONS),
        was_clamped,
    ))
}

/// Paints an `ImageObject` onto the canvas painter in page coordinates scaled by `zoom`.
pub fn paint_image_object(
    ctx: &Context,
    painter: &Painter,
    cache: &mut ImageTextureCache,
    obj: &ImageObject,
    page_origin: Pos2,
    zoom: f32,
    override_page_rect: Option<Rect>,
) {
    let page_rect = override_page_rect.unwrap_or_else(|| image_page_bbox(obj));
    let screen_min = page_origin + Vec2::new(page_rect.min.x * zoom, page_rect.min.y * zoom);
    let screen_size = Vec2::new(
        (page_rect.width() * zoom).max(1.0),
        (page_rect.height() * zoom).max(1.0),
    );
    let screen_rect = Rect::from_min_size(screen_min, screen_size);

    if let Some(tex) = cache.get_or_load(ctx, &obj.bytes) {
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        painter.image(tex.id(), screen_rect, uv, Color32::WHITE);
    } else {
        painter.rect_filled(screen_rect, 2.0, Color32::from_gray(236));
        painter.rect_stroke(
            screen_rect,
            2.0,
            Stroke::new(1.0, Color32::from_rgb(180, 72, 72)),
            egui::StrokeKind::Inside,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_rgba_to_png_and_load_roundtrip() {
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        let encoded = encode_rgba_to_png(2, 2, rgba).expect("encode png");
        assert_eq!(encoded.pixel_size, [2, 2]);
        assert!(encoded.bytes.starts_with(b"\x89PNG\r\n\x1a\n"));

        let reloaded = load_image_from_bytes(encoded.bytes).expect("reload png");
        assert_eq!(reloaded.pixel_size, [2, 2]);
    }

    #[test]
    fn default_image_page_size_preserves_aspect_ratio_and_fits_page() {
        let size = compute_default_image_page_size([1920, 1080], 595.0, 842.0);
        assert!(size[0] <= 595.0 * 0.65 + 1e-6);
        assert!(size[1] <= 842.0 * 0.55 + 1e-6);
        let expected_ratio = 1920.0 / 1080.0;
        let actual_ratio = size[0] / size[1];
        assert!((actual_ratio - expected_ratio).abs() < 1e-6);

        let tiny = compute_default_image_page_size([12, 6], 595.0, 842.0);
        assert!((tiny[0] - 72.0).abs() < 1e-6);
        assert!((tiny[1] - 36.0).abs() < 1e-6);
    }

    #[test]
    fn image_texture_cache_loads_full_resolution_and_transforms_rect() {
        let ctx = Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |_| {});

        let rgba = vec![200u8; 4 * 2 * 4];
        let encoded = encode_rgba_to_png(4, 2, rgba).expect("encode 4x2 png");
        let obj = ImageObject {
            id: 7,
            pos: [50.0, 100.0],
            size: [200.0, 100.0],
            pixel_size: encoded.pixel_size,
            bytes: encoded.bytes,
        };

        let mut cache = ImageTextureCache::default();
        let tex = cache
            .get_or_load(&ctx, &obj.bytes)
            .expect("texture handle should load");
        assert_eq!(tex.size(), [4, 2]);

        let old_bbox = image_page_bbox(&obj);
        let new_bbox = Rect::from_min_size(pos2(80.0, 120.0), vec2(400.0, 200.0));
        let transformed = transformed_image_page_rect(&obj, old_bbox, new_bbox);
        assert!((transformed.min.x - 80.0).abs() < 1e-4);
        assert!((transformed.min.y - 120.0).abs() < 1e-4);
        assert!((transformed.width() - 400.0).abs() < 1e-4);
        assert!((transformed.height() - 200.0).abs() < 1e-4);
    }

    #[test]
    fn encode_color_image_to_png_roundtrips_pixels() {
        let color_img = ColorImage::from_rgba_unmultiplied(
            [3, 2],
            &[
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 10, 20, 30, 128, 40, 50, 60, 255,
                255, 255, 255, 255,
            ],
        );
        let encoded = encode_color_image_to_png(&color_img).expect("encode ColorImage");
        assert_eq!(encoded.pixel_size, [3, 2]);
        assert!(encoded.bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        let decoded = image::load_from_memory(&encoded.bytes).expect("decode PNG");
        assert_eq!(decoded.width(), 3);
        assert_eq!(decoded.height(), 2);
    }

    #[test]
    fn gpu_texture_respects_max_side_uses_mipmaps_and_upgrades_when_limit_grows() {
        assert_eq!(
            IMAGE_TEXTURE_OPTIONS.mipmap_mode,
            Some(TextureFilter::Linear)
        );
        assert_eq!(effective_gpu_texture_size([1920, 1080], 4096), [1920, 1080]);
        assert_eq!(effective_gpu_texture_size([6000, 3000], 2048), [2048, 1024]);

        let ctx = Context::default();
        let _ = ctx.run_ui(
            egui::RawInput {
                max_texture_side: Some(1024),
                ..Default::default()
            },
            |_| {},
        );
        let rgba = vec![140u8; 1600 * 800 * 4];
        let encoded = encode_rgba_to_png(1600, 800, rgba).expect("encode 1600x800 png");
        let orig_bytes = encoded.bytes.clone();

        let mut cache = ImageTextureCache::default();
        let tex_clamped = cache
            .get_or_load(&ctx, &encoded.bytes)
            .expect("clamped texture");
        assert_eq!(tex_clamped.size(), [1024, 512]);
        assert_eq!(encoded.bytes, orig_bytes);

        let _ = ctx.run_ui(
            egui::RawInput {
                max_texture_side: Some(4096),
                ..Default::default()
            },
            |_| {},
        );
        let tex_full = cache
            .get_or_load(&ctx, &encoded.bytes)
            .expect("upgraded full-res texture");
        assert_eq!(tex_full.size(), [1600, 800]);
        assert_eq!(encoded.bytes, orig_bytes);
    }
}
