// Xuất ghi chú ra PDF nhiều trang (vector) và ảnh PNG độ phân giải cao.
// Hỗ trợ đầy đủ nền giấy (Plain, Dotted, Grid, Lined, Margin), nét bút Rnote (vector Bezier),
// hình học, ảnh chèn gốc, văn bản tiếng Việt và công thức LaTeX (vector SVG).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as FmtWrite};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use image::ImageEncoder;
use inknote_core::{
    CanvasMode, Document, DocumentPage, ImageObject, LatexObject, PaperPattern, PaperSettings,
    ShapeObject, TextAlign, TextFontFamily, TextObject,
};
use inknote_pen::style::Composer;
use inknote_render::{CairoRenderContext, VectorFillRule, VectorLineCap, VectorLineJoin, VectorOp};
use kurbo::{Affine, PathEl};
use resvg::tiny_skia;
use resvg::usvg::{self, tiny_skia_path};

use crate::shapes::{shape_arrow_heads, shape_outline_points};
use crate::theme::Theme;

/// Chế độ xuất tập tin người dùng chọn từ nút Export trên thanh công cụ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportScope {
    PdfAllPages,
    PdfCurrentPage,
    PngCurrentPage,
    PngAllPages,
}

/// Hệ số nhân độ phân giải cho PNG (~288 DPI hoặc ~384 DPI tính trên hệ tọa độ trang 96 DPI).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PngScale {
    X3,
    X4,
}

impl PngScale {
    pub fn factor(self) -> f32 {
        match self {
            Self::X3 => 3.0,
            Self::X4 => 4.0,
        }
    }

    pub fn dpi_label(self) -> &'static str {
        match self {
            Self::X3 => "3x (~300 DPI)",
            Self::X4 => "4x (~400 DPI)",
        }
    }
}

/// Lỗi có thể xảy ra trong quá trình dựng hoặc ghi file PDF/PNG.
#[derive(Debug)]
pub enum ExportError {
    EmptyDocument,
    InvalidPageIndex(usize),
    Io(String),
    Render(String),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDocument => write!(f, "Document has no pages to export."),
            Self::InvalidPageIndex(idx) => write!(f, "Page {} does not exist.", idx + 1),
            Self::Io(msg) => write!(f, "File error: {msg}"),
            Self::Render(msg) => write!(f, "Render error: {msg}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<std::io::Error> for ExportError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

/// Tỉ lệ quy đổi từ đơn vị trang (96 DPI logic) sang điểm PDF chuẩn (72 pt/inch): A4 794x1123 -> 595.5x842.25 pt.
const PDF_PT_PER_PAGE_UNIT: f64 = 0.75;

static EXPORT_FONTDB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();

fn export_font_database() -> Arc<usvg::fontdb::Database> {
    EXPORT_FONTDB
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            let font_paths = [
                r"C:\Windows\Fonts\segoeui.ttf",
                r"C:\Windows\Fonts\segoeuib.ttf",
                r"C:\Windows\Fonts\segoeuii.ttf",
                r"C:\Windows\Fonts\segoeuiz.ttf",
                r"C:\Windows\Fonts\times.ttf",
                r"C:\Windows\Fonts\timesbd.ttf",
                r"C:\Windows\Fonts\timesi.ttf",
                r"C:\Windows\Fonts\timesbi.ttf",
                r"C:\Windows\Fonts\consola.ttf",
                r"C:\Windows\Fonts\consolab.ttf",
                r"C:\Windows\Fonts\consolai.ttf",
                r"C:\Windows\Fonts\consolaz.ttf",
            ];
            for path in font_paths {
                let p = Path::new(path);
                if p.exists() {
                    let _ = db.load_font_file(p);
                }
            }
            if db.is_empty() {
                db.load_system_fonts();
            }
            Arc::new(db)
        })
        .clone()
}

fn make_usvg_options() -> usvg::Options<'static> {
    let mut opts = usvg::Options::default();
    opts.fontdb = export_font_database();
    opts.font_family = "Segoe UI".to_string();
    opts
}

fn page_dimensions(settings: &PaperSettings) -> (f64, f64) {
    (
        (settings.page_width.max(120.0)) as f64,
        (settings.page_height.max(120.0)) as f64,
    )
}

fn collect_stroke_vectors(page: &DocumentPage) -> Vec<VectorOp> {
    let mut cx = CairoRenderContext::new_vector_recorder();
    for s in &page.strokes {
        s.path.draw_composed(&mut cx, &s.style);
    }
    cx.vector_ops
}

fn latex_svg_source(obj: &LatexObject) -> Option<String> {
    if let Some(svg) = obj.svg_cache.as_deref() {
        return Some(svg.to_string());
    }
    let opts = inknote_latex::CompileOptions {
        timeout: Duration::from_secs(5),
        allow_fallback: true,
        color_rgb: None,
    };
    inknote_latex::compile_latex_to_svg(&obj.source, &opts).ok()
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

fn svg_font_family_name(family: TextFontFamily) -> &'static str {
    match family {
        TextFontFamily::Sans => "Segoe UI",
        TextFontFamily::Serif => "Times New Roman",
        TextFontFamily::Mono => "Consolas",
    }
}

fn measure_line_width_svg(
    text: &str,
    family_name: &str,
    font_size: f64,
    bold: bool,
    italic: bool,
    opts: &usvg::Options,
) -> f64 {
    if text.is_empty() {
        return 0.0;
    }
    let weight = if bold { "bold" } else { "normal" };
    let style = if italic { "italic" } else { "normal" };
    let escaped = xml_escape(text);
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"8192\" height=\"512\">\
         <text x=\"0\" y=\"{font_size:.2}\" font-family=\"{family_name}\" font-size=\"{font_size:.2}\" \
         font-weight=\"{weight}\" font-style=\"{style}\" xml:space=\"preserve\">{escaped}</text></svg>"
    );
    usvg::Tree::from_str(&svg, opts)
        .map(|t| t.root().bounding_box().width() as f64)
        .unwrap_or_else(|_| text.chars().count() as f64 * font_size * 0.55)
}

fn wrap_text_lines(obj: &TextObject, opts: &usvg::Options) -> Vec<String> {
    let max_w = obj.size[0].max(24.0);
    let font_size = obj.font_size.clamp(8.0, 120.0);
    let family = svg_font_family_name(obj.font_family);
    let normalized = obj.text.replace("\r\n", "\n");
    let mut lines = Vec::new();

    for raw_para in normalized.split('\n') {
        if raw_para.is_empty() {
            lines.push(String::new());
            continue;
        }
        // Dòng ngắn ước lượng: kiểm tra trực tiếp một lần để tránh chia nhỏ không cần thiết.
        if (raw_para.chars().count() as f64) * font_size * 0.65 <= max_w
            || measure_line_width_svg(raw_para, family, font_size, obj.bold, obj.italic, opts)
                <= max_w
        {
            lines.push(raw_para.to_string());
            continue;
        }

        let mut current = String::new();
        for word in raw_para.split_inclusive(' ') {
            let trimmed_candidate = format!("{current}{word}");
            let check_str = trimmed_candidate.trim_end_matches(' ');
            if current.is_empty() {
                if measure_line_width_svg(check_str, family, font_size, obj.bold, obj.italic, opts)
                    <= max_w
                {
                    current = trimmed_candidate;
                } else {
                    // Cắt theo ký tự nếu 1 từ đơn quá dài vượt bề rộng hộp.
                    let mut chunk = String::new();
                    for ch in word.chars() {
                        let mut next = chunk.clone();
                        next.push(ch);
                        if !chunk.is_empty()
                            && measure_line_width_svg(
                                &next, family, font_size, obj.bold, obj.italic, opts,
                            ) > max_w
                        {
                            lines.push(chunk);
                            chunk = ch.to_string();
                        } else {
                            chunk = next;
                        }
                    }
                    current = chunk;
                }
            } else if measure_line_width_svg(
                check_str, family, font_size, obj.bold, obj.italic, opts,
            ) <= max_w
            {
                current = trimmed_candidate;
            } else {
                lines.push(current.trim_end_matches(' ').to_string());
                current = word.trim_start_matches(' ').to_string();
            }
        }
        if !current.is_empty() {
            lines.push(current.trim_end_matches(' ').to_string());
        }
    }
    lines
}

fn text_object_to_svg(obj: &TextObject, opts: &usvg::Options) -> Option<String> {
    if obj.text.trim().is_empty() || obj.color[3] == 0 {
        return None;
    }
    let w = obj.size[0].max(24.0);
    let font_size = obj.font_size.clamp(8.0, 120.0);
    let line_h = font_size * 1.28;
    let lines = wrap_text_lines(obj, opts);
    let total_h = (lines.len().max(1) as f64 * line_h).max(obj.size[1]);
    let family = svg_font_family_name(obj.font_family);
    let weight = if obj.bold { "bold" } else { "normal" };
    let style = if obj.italic { "italic" } else { "normal" };
    let (anchor, text_x) = match obj.align {
        TextAlign::Left => ("start", 0.0),
        TextAlign::Center => ("middle", w * 0.5),
        TextAlign::Right => ("end", w),
    };
    let fill_hex = format!(
        "#{:02x}{:02x}{:02x}",
        obj.color[0], obj.color[1], obj.color[2]
    );
    let fill_opacity = obj.color[3] as f64 / 255.0;

    let mut body = String::new();
    for (idx, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        let baseline_y = font_size * 0.94 + idx as f64 * line_h;
        let _ = writeln!(
            body,
            "<text x=\"{text_x:.3}\" y=\"{baseline_y:.3}\" text-anchor=\"{anchor}\" \
             font-family=\"{family}\" font-size=\"{font_size:.3}\" font-weight=\"{weight}\" \
             font-style=\"{style}\" fill=\"{fill_hex}\" fill-opacity=\"{fill_opacity:.4}\" \
             xml:space=\"preserve\">{}</text>",
            xml_escape(line)
        );
    }
    if body.is_empty() {
        return None;
    }
    Some(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.3}\" height=\"{total_h:.3}\" \
         viewBox=\"0 0 {w:.3} {total_h:.3}\">{body}</svg>"
    ))
}

// ============================================================================
// XUẤT PNG ĐỘ PHÂN GIẢI CAO (tiny_skia + resvg + Lanczos3)
// ============================================================================

fn color32_to_skia(c: eframe::egui::Color32) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(c.r(), c.g(), c.b(), c.a())
}

fn rgba_to_paint(rgba: [u8; 4]) -> tiny_skia::Paint<'static> {
    let mut paint = tiny_skia::Paint::default();
    paint.set_color(tiny_skia::Color::from_rgba8(
        rgba[0], rgba[1], rgba[2], rgba[3],
    ));
    paint.anti_alias = true;
    paint
}

fn draw_paper_to_pixmap(
    pixmap: &mut tiny_skia::Pixmap,
    settings: &PaperSettings,
    theme: &Theme,
    scale: f32,
) {
    let (w, h) = page_dimensions(settings);
    let w_f = w as f32;
    let h_f = h as f32;
    pixmap.fill(color32_to_skia(theme.paper_bg));

    let xf = tiny_skia::Transform::from_scale(scale, scale);
    let spacing = settings.dot_spacing.max(0.0);
    if spacing > 0.0 {
        match settings.pattern {
            PaperPattern::Plain => {}
            PaperPattern::Dotted => {
                let mut pb = tiny_skia::PathBuilder::new();
                let mut y = 0.0;
                let r = theme.paper_dot_radius.max(0.6);
                while y <= h_f {
                    let mut x = 0.0;
                    while x <= w_f {
                        pb.push_circle(x, y, r);
                        x += spacing;
                    }
                    y += spacing;
                }
                if let Some(path) = pb.finish() {
                    let c = theme.paper_dot_color;
                    let paint = rgba_to_paint([c.r(), c.g(), c.b(), c.a()]);
                    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, xf, None);
                }
            }
            PaperPattern::Grid => {
                let mut pb = tiny_skia::PathBuilder::new();
                let mut x = 0.0;
                while x <= w_f {
                    pb.move_to(x, 0.0);
                    pb.line_to(x, h_f);
                    x += spacing;
                }
                let mut y = 0.0;
                while y <= h_f {
                    pb.move_to(0.0, y);
                    pb.line_to(w_f, y);
                    y += spacing;
                }
                if let Some(path) = pb.finish() {
                    let c = theme.paper_grid_color;
                    let paint = rgba_to_paint([c.r(), c.g(), c.b(), c.a()]);
                    let stroke = tiny_skia::Stroke {
                        width: 1.0,
                        ..Default::default()
                    };
                    pixmap.stroke_path(&path, &paint, &stroke, xf, None);
                }
            }
            PaperPattern::Lined => {
                let mut pb = tiny_skia::PathBuilder::new();
                let mut y = 0.0;
                while y <= h_f {
                    pb.move_to(0.0, y);
                    pb.line_to(w_f, y);
                    y += spacing;
                }
                if let Some(path) = pb.finish() {
                    let c = theme.paper_line_color;
                    let paint = rgba_to_paint([c.r(), c.g(), c.b(), c.a()]);
                    let stroke = tiny_skia::Stroke {
                        width: 1.0,
                        ..Default::default()
                    };
                    pixmap.stroke_path(&path, &paint, &stroke, xf, None);
                }
            }
        }
    }

    if settings.show_margins && settings.mode == CanvasMode::Paper {
        let mut pb = tiny_skia::PathBuilder::new();
        let left_x = settings.left_margin;
        if left_x >= 0.0 && left_x <= w_f {
            pb.move_to(left_x, 0.0);
            pb.line_to(left_x, h_f);
        }
        let right_x = w_f - settings.right_margin;
        if right_x >= 0.0 && right_x <= w_f {
            pb.move_to(right_x, 0.0);
            pb.line_to(right_x, h_f);
        }
        if let Some(path) = pb.finish() {
            let c = theme.paper_margin_line_color;
            let paint = rgba_to_paint([c.r(), c.g(), c.b(), c.a()]);
            let stroke = tiny_skia::Stroke {
                width: 1.5,
                ..Default::default()
            };
            pixmap.stroke_path(&path, &paint, &stroke, xf, None);
        }
    }
}

fn draw_image_to_pixmap(pixmap: &mut tiny_skia::Pixmap, img: &ImageObject, scale: f32) {
    let Ok(decoded) = image::load_from_memory(&img.bytes) else {
        return;
    };
    let rgba = decoded.to_rgba8();
    let dst_w = (img.size[0] as f32 * scale).round().max(1.0) as u32;
    let dst_h = (img.size[1] as f32 * scale).round().max(1.0) as u32;
    let resized =
        image::imageops::resize(&rgba, dst_w, dst_h, image::imageops::FilterType::Lanczos3);

    // Chuyển unmultiplied RGBA -> premultiplied RGBA cho tiny_skia::PixmapMut
    let mut premul = resized.into_raw();
    for px in premul.chunks_exact_mut(4) {
        let a = px[3] as u16;
        if a < 255 {
            px[0] = ((px[0] as u16 * a + 127) / 255) as u8;
            px[1] = ((px[1] as u16 * a + 127) / 255) as u8;
            px[2] = ((px[2] as u16 * a + 127) / 255) as u8;
        }
    }
    if let Some(src_pm) = tiny_skia::PixmapRef::from_bytes(&premul, dst_w, dst_h) {
        let mut paint = tiny_skia::PixmapPaint::default();
        paint.quality = tiny_skia::FilterQuality::Bilinear;
        let x = (img.pos[0] as f32 * scale).round() as i32;
        let y = (img.pos[1] as f32 * scale).round() as i32;
        pixmap.draw_pixmap(x, y, src_pm, &paint, tiny_skia::Transform::identity(), None);
    }
}

fn kurbo_elements_to_skia_path(elements: &[PathEl]) -> Option<tiny_skia::Path> {
    let mut pb = tiny_skia::PathBuilder::new();
    for &el in elements {
        match el {
            PathEl::MoveTo(p) => pb.move_to(p.x as f32, p.y as f32),
            PathEl::LineTo(p) => pb.line_to(p.x as f32, p.y as f32),
            PathEl::QuadTo(p1, p2) => {
                pb.quad_to(p1.x as f32, p1.y as f32, p2.x as f32, p2.y as f32)
            }
            PathEl::CurveTo(p1, p2, p3) => pb.cubic_to(
                p1.x as f32,
                p1.y as f32,
                p2.x as f32,
                p2.y as f32,
                p3.x as f32,
                p3.y as f32,
            ),
            PathEl::ClosePath => pb.close(),
        }
    }
    pb.finish()
}

fn draw_vector_ops_to_pixmap(pixmap: &mut tiny_skia::Pixmap, ops: &[VectorOp], scale: f32) {
    let xf = tiny_skia::Transform::from_scale(scale, scale);
    for op in ops {
        match op {
            VectorOp::Fill {
                elements,
                color,
                rule,
            } => {
                let Some(path) = kurbo_elements_to_skia_path(elements) else {
                    continue;
                };
                let paint = rgba_to_paint(*color);
                let fill_rule = match rule {
                    VectorFillRule::NonZero => tiny_skia::FillRule::Winding,
                    VectorFillRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
                };
                pixmap.fill_path(&path, &paint, fill_rule, xf, None);
            }
            VectorOp::Stroke {
                elements,
                color,
                width,
                cap,
                join,
                miter_limit,
            } => {
                let Some(path) = kurbo_elements_to_skia_path(elements) else {
                    continue;
                };
                let paint = rgba_to_paint(*color);
                let stroke = tiny_skia::Stroke {
                    width: (*width as f32).max(0.1),
                    miter_limit: *miter_limit as f32,
                    line_cap: match cap {
                        VectorLineCap::Butt => tiny_skia::LineCap::Butt,
                        VectorLineCap::Round => tiny_skia::LineCap::Round,
                        VectorLineCap::Square => tiny_skia::LineCap::Square,
                    },
                    line_join: match join {
                        VectorLineJoin::Miter => tiny_skia::LineJoin::Miter,
                        VectorLineJoin::Round => tiny_skia::LineJoin::Round,
                        VectorLineJoin::Bevel => tiny_skia::LineJoin::Bevel,
                    },
                    dash: None,
                };
                pixmap.stroke_path(&path, &paint, &stroke, xf, None);
            }
        }
    }
}

fn draw_shape_to_pixmap(pixmap: &mut tiny_skia::Pixmap, shape: &ShapeObject, scale: f32) {
    let start = eframe::egui::pos2(shape.start[0] as f32, shape.start[1] as f32);
    let end = eframe::egui::pos2(shape.end[0] as f32, shape.end[1] as f32);
    let (pts, closed) = shape_outline_points(shape.kind, start, end);
    if pts.len() < 2 {
        return;
    }

    let width = (shape.stroke_width as f32).max(0.8);
    let dash = if shape.dashed {
        let dash_len = (width * 3.0).max(8.0);
        let gap_len = (width * 2.0).max(5.5);
        tiny_skia::StrokeDash::new(vec![dash_len, gap_len], 0.0)
    } else {
        None
    };

    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(pts[0].x, pts[0].y);
    for p in &pts[1..] {
        pb.line_to(p.x, p.y);
    }
    if closed {
        pb.close();
    }

    let paint = rgba_to_paint(shape.color);
    let xf = tiny_skia::Transform::from_scale(scale, scale);
    let stroke = tiny_skia::Stroke {
        width,
        line_cap: tiny_skia::LineCap::Round,
        line_join: tiny_skia::LineJoin::Round,
        dash,
        ..Default::default()
    };
    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, &paint, &stroke, xf, None);
    }

    let heads = shape_arrow_heads(shape.kind, start, end, shape.stroke_width);
    if !heads.is_empty() {
        let mut arrow_pb = tiny_skia::PathBuilder::new();
        for [w1, tip, w2] in heads {
            arrow_pb.move_to(w1.x, w1.y);
            arrow_pb.line_to(tip.x, tip.y);
            arrow_pb.line_to(w2.x, w2.y);
        }
        if let Some(arrow_path) = arrow_pb.finish() {
            let arrow_stroke = tiny_skia::Stroke {
                width,
                line_cap: tiny_skia::LineCap::Round,
                line_join: tiny_skia::LineJoin::Round,
                dash: None,
                ..Default::default()
            };
            pixmap.stroke_path(&arrow_path, &paint, &arrow_stroke, xf, None);
        }
    }
}

fn draw_svg_tree_to_pixmap(
    pixmap: &mut tiny_skia::Pixmap,
    tree: &usvg::Tree,
    pos: [f64; 2],
    size: [f64; 2],
    page_scale: f32,
) {
    let svg_size = tree.size();
    let sw = svg_size.width().max(1.0);
    let sh = svg_size.height().max(1.0);
    let sx = (size[0] as f32 / sw) * page_scale;
    let sy = (size[1] as f32 / sh) * page_scale;
    let xf = tiny_skia::Transform::from_translate(
        pos[0] as f32 * page_scale,
        pos[1] as f32 * page_scale,
    )
    .pre_scale(sx, sy);
    resvg::render(tree, xf, &mut pixmap.as_mut());
}

/// Dựng một trang ghi chú thành ảnh PNG chất lượng cao (`scale = 3.0` hoặc `4.0`).
pub fn render_page_to_png_bytes(
    page: &DocumentPage,
    settings: &PaperSettings,
    theme: &Theme,
    scale: PngScale,
) -> Result<Vec<u8>, ExportError> {
    let scale_f = scale.factor();
    let (w, h) = page_dimensions(settings);
    let px_w = (w as f32 * scale_f).ceil().max(1.0) as u32;
    let px_h = (h as f32 * scale_f).ceil().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(px_w, px_h)
        .ok_or_else(|| ExportError::Render("Failed to allocate PNG canvas buffer".to_string()))?;

    // 1. Nền giấy, chấm/ô/dòng kẻ và đường lề
    draw_paper_to_pixmap(&mut pixmap, settings, theme, scale_f);

    // 2. Ảnh chèn (Lanczos3)
    for img in &page.images {
        draw_image_to_pixmap(&mut pixmap, img, scale_f);
    }

    // 3. Nét bút Rnote
    let ops = collect_stroke_vectors(page);
    draw_vector_ops_to_pixmap(&mut pixmap, &ops, scale_f);

    // 4. Hình học
    for shape in &page.shapes {
        draw_shape_to_pixmap(&mut pixmap, shape, scale_f);
    }

    // 5. Văn bản tiếng Việt & 6. Công thức LaTeX
    let usvg_opts = make_usvg_options();
    for txt in &page.text_objects {
        if let Some(svg) = text_object_to_svg(txt, &usvg_opts)
            && let Ok(tree) = usvg::Tree::from_str(&svg, &usvg_opts)
        {
            let tree_sz = tree.size();
            draw_svg_tree_to_pixmap(
                &mut pixmap,
                &tree,
                txt.pos,
                [tree_sz.width() as f64, tree_sz.height() as f64],
                scale_f,
            );
        }
    }
    for latex in &page.latex_objects {
        if let Some(svg) = latex_svg_source(latex)
            && let Ok(tree) = usvg::Tree::from_str(&svg, &usvg_opts)
        {
            draw_svg_tree_to_pixmap(&mut pixmap, &tree, latex.pos, latex.size, scale_f);
        }
    }

    // Chuyển premultiplied RGBA của tiny-skia về straight RGBA8 cho chuẩn PNG.
    let mut rgba = pixmap.take();
    for px in rgba.chunks_exact_mut(4) {
        let a = px[3];
        if a > 0 && a < 255 {
            let af = 255.0 / a as f32;
            px[0] = (px[0] as f32 * af).round().clamp(0.0, 255.0) as u8;
            px[1] = (px[1] as f32 * af).round().clamp(0.0, 255.0) as u8;
            px[2] = (px[2] as f32 * af).round().clamp(0.0, 255.0) as u8;
        }
    }

    let mut out = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut out);
    encoder
        .write_image(&rgba, px_w, px_h, image::ExtendedColorType::Rgba8)
        .map_err(|e| ExportError::Render(e.to_string()))?;
    Ok(out)
}

/// Tạo danh sách đường dẫn đích khi xuất một hoặc nhiều trang PNG.
pub fn build_png_output_paths(base_path: &Path, page_count: usize) -> Vec<PathBuf> {
    if page_count <= 1 {
        return vec![ensure_extension(base_path, "png")];
    }
    let parent = base_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = base_path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("inknote_export");
    (1..=page_count)
        .map(|num| parent.join(format!("{stem}_page_{num}.png")))
        .collect()
}

fn ensure_extension(path: &Path, ext: &str) -> PathBuf {
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
    {
        path.to_path_buf()
    } else {
        path.with_extension(ext)
    }
}

/// Xuất danh sách các trang chỉ định ra file PNG (1 trang -> đúng `dest_path`, nhiều trang -> `<stem>_page_N.png`).
pub fn export_pages_to_png_files(
    doc: &Document,
    page_indices: &[usize],
    theme: &Theme,
    scale: PngScale,
    dest_path: &Path,
) -> Result<Vec<PathBuf>, ExportError> {
    if page_indices.is_empty() {
        return Err(ExportError::EmptyDocument);
    }
    let pages = doc.pages();
    for &idx in page_indices {
        if idx >= pages.len() {
            return Err(ExportError::InvalidPageIndex(idx));
        }
    }
    let out_paths = build_png_output_paths(dest_path, page_indices.len());
    let settings = doc.paper_settings();
    for (&page_idx, out_path) in page_indices.iter().zip(&out_paths) {
        let png_bytes = render_page_to_png_bytes(&pages[page_idx], settings, theme, scale)?;
        if let Some(parent) = out_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(out_path, png_bytes)?;
    }
    Ok(out_paths)
}

// ============================================================================
// XUẤT PDF VECTOR NHIỀU TRANG (%PDF-1.4 + FlateDecode + ExtGState + Vector Paths)
// ============================================================================

struct PdfImageStream {
    resource_name: String,
    width: u32,
    height: u32,
    compressed_rgb: Vec<u8>,
    compressed_smask: Option<Vec<u8>>,
}

struct PdfPageStream {
    width_pt: f64,
    height_pt: f64,
    compressed_content: Vec<u8>,
    used_alphas: BTreeSet<u8>,
    images: Vec<PdfImageStream>,
}

struct PdfStreamWriter {
    page_h: f64,
    buf: String,
    current_alpha: u8,
    used_alphas: BTreeSet<u8>,
    images: Vec<PdfImageStream>,
}

impl PdfStreamWriter {
    fn new(page_w: f64, page_h: f64) -> Self {
        let mut writer = Self {
            page_h,
            buf: String::with_capacity(16 * 1024),
            current_alpha: 255,
            used_alphas: BTreeSet::new(),
            images: Vec::new(),
        };
        // Tô nền giấy trắng tinh khôi phủ kín trang PDF
        let _ = writeln!(
            writer.buf,
            "1 1 1 rg 0 0 {:.4} {:.4} re f",
            page_w * PDF_PT_PER_PAGE_UNIT,
            page_h * PDF_PT_PER_PAGE_UNIT
        );
        writer
    }

    #[inline]
    fn px(&self, x: f64) -> f64 {
        x * PDF_PT_PER_PAGE_UNIT
    }

    #[inline]
    fn py(&self, y: f64) -> f64 {
        (self.page_h - y) * PDF_PT_PER_PAGE_UNIT
    }

    fn ensure_alpha(&mut self, alpha: u8) {
        if self.current_alpha != alpha {
            self.current_alpha = alpha;
            self.used_alphas.insert(alpha);
            let _ = writeln!(self.buf, "/GS{alpha} gs");
        }
    }

    fn set_fill_rgba(&mut self, rgba: [u8; 4]) {
        self.ensure_alpha(rgba[3]);
        let _ = writeln!(
            self.buf,
            "{:.4} {:.4} {:.4} rg",
            rgba[0] as f64 / 255.0,
            rgba[1] as f64 / 255.0,
            rgba[2] as f64 / 255.0
        );
    }

    fn set_stroke_rgba(&mut self, rgba: [u8; 4]) {
        self.ensure_alpha(rgba[3]);
        let _ = writeln!(
            self.buf,
            "{:.4} {:.4} {:.4} RG",
            rgba[0] as f64 / 255.0,
            rgba[1] as f64 / 255.0,
            rgba[2] as f64 / 255.0
        );
    }

    fn write_circle_subpath(&mut self, cx: f64, cy: f64, r: f64) {
        const K: f64 = 0.552_284_749_8;
        let x = self.px(cx);
        let y = self.py(cy);
        let r_pt = r * PDF_PT_PER_PAGE_UNIT;
        let kr = r_pt * K;
        let _ = writeln!(
            self.buf,
            "{:.4} {:.4} m \
             {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} c \
             {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} c \
             {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} c \
             {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} c h",
            x + r_pt,
            y,
            x + r_pt,
            y + kr,
            x + kr,
            y + r_pt,
            x,
            y + r_pt,
            x - kr,
            y + r_pt,
            x - r_pt,
            y + kr,
            x - r_pt,
            y,
            x - r_pt,
            y - kr,
            x - kr,
            y - r_pt,
            x,
            y - r_pt,
            x + kr,
            y - r_pt,
            x + r_pt,
            y - kr,
            x + r_pt,
            y,
        );
    }

    fn write_paper(&mut self, settings: &PaperSettings, theme: &Theme, page_w: f64, page_h: f64) {
        let spacing = settings.dot_spacing as f64;
        if spacing > 0.0 {
            match settings.pattern {
                PaperPattern::Plain => {}
                PaperPattern::Dotted => {
                    let c = theme.paper_dot_color;
                    self.set_fill_rgba([c.r(), c.g(), c.b(), c.a()]);
                    let r = theme.paper_dot_radius.max(0.6) as f64;
                    let mut y = 0.0;
                    while y <= page_h {
                        let mut x = 0.0;
                        while x <= page_w {
                            self.write_circle_subpath(x, y, r);
                            x += spacing;
                        }
                        y += spacing;
                    }
                    let _ = writeln!(self.buf, "f");
                }
                PaperPattern::Grid => {
                    let c = theme.paper_grid_color;
                    self.set_stroke_rgba([c.r(), c.g(), c.b(), c.a()]);
                    let _ = writeln!(self.buf, "{:.4} w", 0.9 * PDF_PT_PER_PAGE_UNIT);
                    let mut x = 0.0;
                    while x <= page_w {
                        let _ = writeln!(
                            self.buf,
                            "{:.4} {:.4} m {:.4} {:.4} l",
                            self.px(x),
                            self.py(0.0),
                            self.px(x),
                            self.py(page_h)
                        );
                        x += spacing;
                    }
                    let mut y = 0.0;
                    while y <= page_h {
                        let _ = writeln!(
                            self.buf,
                            "{:.4} {:.4} m {:.4} {:.4} l",
                            self.px(0.0),
                            self.py(y),
                            self.px(page_w),
                            self.py(y)
                        );
                        y += spacing;
                    }
                    let _ = writeln!(self.buf, "S");
                }
                PaperPattern::Lined => {
                    let c = theme.paper_line_color;
                    self.set_stroke_rgba([c.r(), c.g(), c.b(), c.a()]);
                    let _ = writeln!(self.buf, "{:.4} w", 0.9 * PDF_PT_PER_PAGE_UNIT);
                    let mut y = 0.0;
                    while y <= page_h {
                        let _ = writeln!(
                            self.buf,
                            "{:.4} {:.4} m {:.4} {:.4} l",
                            self.px(0.0),
                            self.py(y),
                            self.px(page_w),
                            self.py(y)
                        );
                        y += spacing;
                    }
                    let _ = writeln!(self.buf, "S");
                }
            }
        }

        if settings.show_margins && settings.mode == CanvasMode::Paper {
            let c = theme.paper_margin_line_color;
            self.set_stroke_rgba([c.r(), c.g(), c.b(), c.a()]);
            let _ = writeln!(self.buf, "{:.4} w", 1.3 * PDF_PT_PER_PAGE_UNIT);
            let lx = settings.left_margin as f64;
            if lx >= 0.0 && lx <= page_w {
                let _ = writeln!(
                    self.buf,
                    "{:.4} {:.4} m {:.4} {:.4} l",
                    self.px(lx),
                    self.py(0.0),
                    self.px(lx),
                    self.py(page_h)
                );
            }
            let rx = page_w - settings.right_margin as f64;
            if rx >= 0.0 && rx <= page_w {
                let _ = writeln!(
                    self.buf,
                    "{:.4} {:.4} m {:.4} {:.4} l",
                    self.px(rx),
                    self.py(0.0),
                    self.px(rx),
                    self.py(page_h)
                );
            }
            let _ = writeln!(self.buf, "S");
        }
    }

    fn write_image(&mut self, img: &ImageObject) {
        let Ok(decoded) = image::load_from_memory(&img.bytes) else {
            return;
        };
        let rgba = decoded.to_rgba8();
        let (w, h) = rgba.dimensions();
        if w == 0 || h == 0 {
            return;
        }
        let mut rgb_raw = Vec::with_capacity((w * h * 3) as usize);
        let mut alpha_raw = Vec::with_capacity((w * h) as usize);
        let mut has_alpha = false;
        for px in rgba.pixels() {
            rgb_raw.push(px[0]);
            rgb_raw.push(px[1]);
            rgb_raw.push(px[2]);
            alpha_raw.push(px[3]);
            if px[3] < 255 {
                has_alpha = true;
            }
        }
        let Ok(compressed_rgb) = zlib_compress(&rgb_raw) else {
            return;
        };
        let compressed_smask = if has_alpha {
            zlib_compress(&alpha_raw).ok()
        } else {
            None
        };

        let idx = self.images.len() + 1;
        let resource_name = format!("Im{idx}");
        self.images.push(PdfImageStream {
            resource_name: resource_name.clone(),
            width: w,
            height: h,
            compressed_rgb,
            compressed_smask,
        });

        self.ensure_alpha(255);
        let w_pt = img.size[0].max(1.0) * PDF_PT_PER_PAGE_UNIT;
        let h_pt = img.size[1].max(1.0) * PDF_PT_PER_PAGE_UNIT;
        let x_pt = self.px(img.pos[0]);
        let y_bot_pt = self.py(img.pos[1] + img.size[1]);
        let _ = writeln!(
            self.buf,
            "q {w_pt:.4} 0 0 {h_pt:.4} {x_pt:.4} {y_bot_pt:.4} cm /{resource_name} Do Q"
        );
    }

    fn write_kurbo_elements(&mut self, elements: &[PathEl]) {
        let mut cur = (0.0_f64, 0.0_f64);
        for &el in elements {
            match el {
                PathEl::MoveTo(p) => {
                    cur = (p.x, p.y);
                    let _ = writeln!(self.buf, "{:.4} {:.4} m", self.px(p.x), self.py(p.y));
                }
                PathEl::LineTo(p) => {
                    cur = (p.x, p.y);
                    let _ = writeln!(self.buf, "{:.4} {:.4} l", self.px(p.x), self.py(p.y));
                }
                PathEl::QuadTo(q, p) => {
                    // Nâng bậc đường cong bậc 2 lên bậc 3 (công thức chính xác cho PDF `c`)
                    let c1x = cur.0 + (2.0 / 3.0) * (q.x - cur.0);
                    let c1y = cur.1 + (2.0 / 3.0) * (q.y - cur.1);
                    let c2x = p.x + (2.0 / 3.0) * (q.x - p.x);
                    let c2y = p.y + (2.0 / 3.0) * (q.y - p.y);
                    cur = (p.x, p.y);
                    let _ = writeln!(
                        self.buf,
                        "{:.4} {:.4} {:.4} {:.4} {:.4} {:.4} c",
                        self.px(c1x),
                        self.py(c1y),
                        self.px(c2x),
                        self.py(c2y),
                        self.px(p.x),
                        self.py(p.y)
                    );
                }
                PathEl::CurveTo(c1, c2, p) => {
                    cur = (p.x, p.y);
                    let _ = writeln!(
                        self.buf,
                        "{:.4} {:.4} {:.4} {:.4} {:.4} {:.4} c",
                        self.px(c1.x),
                        self.py(c1.y),
                        self.px(c2.x),
                        self.py(c2.y),
                        self.px(p.x),
                        self.py(p.y)
                    );
                }
                PathEl::ClosePath => {
                    let _ = writeln!(self.buf, "h");
                }
            }
        }
    }

    fn write_vector_ops(&mut self, ops: &[VectorOp]) {
        for op in ops {
            match op {
                VectorOp::Fill {
                    elements,
                    color,
                    rule,
                } => {
                    self.set_fill_rgba(*color);
                    self.write_kurbo_elements(elements);
                    let op_cmd = match rule {
                        VectorFillRule::NonZero => "f",
                        VectorFillRule::EvenOdd => "f*",
                    };
                    let _ = writeln!(self.buf, "{op_cmd}");
                }
                VectorOp::Stroke {
                    elements,
                    color,
                    width,
                    cap,
                    join,
                    miter_limit,
                } => {
                    self.set_stroke_rgba(*color);
                    let cap_code = match cap {
                        VectorLineCap::Butt => 0,
                        VectorLineCap::Round => 1,
                        VectorLineCap::Square => 2,
                    };
                    let join_code = match join {
                        VectorLineJoin::Miter => 0,
                        VectorLineJoin::Round => 1,
                        VectorLineJoin::Bevel => 2,
                    };
                    let _ = writeln!(
                        self.buf,
                        "{:.4} w {cap_code} J {join_code} j {:.4} M",
                        (*width * PDF_PT_PER_PAGE_UNIT).max(0.1),
                        miter_limit.max(1.0)
                    );
                    self.write_kurbo_elements(elements);
                    let _ = writeln!(self.buf, "S");
                }
            }
        }
    }

    fn write_shape(&mut self, shape: &ShapeObject) {
        let start = eframe::egui::pos2(shape.start[0] as f32, shape.start[1] as f32);
        let end = eframe::egui::pos2(shape.end[0] as f32, shape.end[1] as f32);
        let (pts, closed) = shape_outline_points(shape.kind, start, end);
        if pts.len() < 2 {
            return;
        }
        let width_pt = (shape.stroke_width.max(0.8)) * PDF_PT_PER_PAGE_UNIT;
        self.set_stroke_rgba(shape.color);
        let _ = writeln!(self.buf, "q {width_pt:.4} w 1 J 1 j");
        if shape.dashed {
            let dash_pt = (shape.stroke_width * 3.0).max(8.0) * PDF_PT_PER_PAGE_UNIT;
            let gap_pt = (shape.stroke_width * 2.0).max(5.5) * PDF_PT_PER_PAGE_UNIT;
            let _ = writeln!(self.buf, "[{dash_pt:.4} {gap_pt:.4}] 0 d");
        }
        let _ = writeln!(
            self.buf,
            "{:.4} {:.4} m",
            self.px(pts[0].x as f64),
            self.py(pts[0].y as f64)
        );
        for p in &pts[1..] {
            let _ = writeln!(
                self.buf,
                "{:.4} {:.4} l",
                self.px(p.x as f64),
                self.py(p.y as f64)
            );
        }
        if closed {
            let _ = writeln!(self.buf, "h");
        }
        let _ = writeln!(self.buf, "S");

        let heads = shape_arrow_heads(shape.kind, start, end, shape.stroke_width);
        if !heads.is_empty() {
            let _ = writeln!(self.buf, "[] 0 d");
            for [w1, tip, w2] in heads {
                let _ = writeln!(
                    self.buf,
                    "{:.4} {:.4} m {:.4} {:.4} l {:.4} {:.4} l",
                    self.px(w1.x as f64),
                    self.py(w1.y as f64),
                    self.px(tip.x as f64),
                    self.py(tip.y as f64),
                    self.px(w2.x as f64),
                    self.py(w2.y as f64),
                );
            }
            let _ = writeln!(self.buf, "S");
        }
        let _ = writeln!(self.buf, "Q");
    }

    fn write_usvg_path(
        &mut self,
        path: &usvg::Path,
        page_xf: Affine,
        opacity_mul: f32,
        scale_stroke: f64,
    ) {
        if !path.is_visible() {
            return;
        }
        let mut els = Vec::new();
        for seg in path.data().segments() {
            match seg {
                tiny_skia_path::PathSegment::MoveTo(p) => {
                    els.push(PathEl::MoveTo(
                        page_xf * kurbo::Point::new(p.x as f64, p.y as f64),
                    ));
                }
                tiny_skia_path::PathSegment::LineTo(p) => {
                    els.push(PathEl::LineTo(
                        page_xf * kurbo::Point::new(p.x as f64, p.y as f64),
                    ));
                }
                tiny_skia_path::PathSegment::QuadTo(p1, p2) => {
                    els.push(PathEl::QuadTo(
                        page_xf * kurbo::Point::new(p1.x as f64, p1.y as f64),
                        page_xf * kurbo::Point::new(p2.x as f64, p2.y as f64),
                    ));
                }
                tiny_skia_path::PathSegment::CubicTo(p1, p2, p3) => {
                    els.push(PathEl::CurveTo(
                        page_xf * kurbo::Point::new(p1.x as f64, p1.y as f64),
                        page_xf * kurbo::Point::new(p2.x as f64, p2.y as f64),
                        page_xf * kurbo::Point::new(p3.x as f64, p3.y as f64),
                    ));
                }
                tiny_skia_path::PathSegment::Close => els.push(PathEl::ClosePath),
            }
        }
        if els.is_empty() {
            return;
        }

        let draw_fill = |w: &mut Self, fill: &usvg::Fill| {
            let usvg::Paint::Color(c) = fill.paint() else {
                return;
            };
            let a = ((fill.opacity().get() * opacity_mul).clamp(0.0, 1.0) * 255.0).round() as u8;
            if a == 0 {
                return;
            }
            w.set_fill_rgba([c.red, c.green, c.blue, a]);
            w.write_kurbo_elements(&els);
            let cmd = match fill.rule() {
                usvg::FillRule::NonZero => "f",
                usvg::FillRule::EvenOdd => "f*",
            };
            let _ = writeln!(w.buf, "{cmd}");
        };

        let draw_stroke = |w: &mut Self, stroke: &usvg::Stroke| {
            let usvg::Paint::Color(c) = stroke.paint() else {
                return;
            };
            let a = ((stroke.opacity().get() * opacity_mul).clamp(0.0, 1.0) * 255.0).round() as u8;
            if a == 0 {
                return;
            }
            w.set_stroke_rgba([c.red, c.green, c.blue, a]);
            let width_pt =
                (stroke.width().get() as f64 * scale_stroke * PDF_PT_PER_PAGE_UNIT).max(0.1);
            let cap_code = match stroke.linecap() {
                usvg::LineCap::Butt => 0,
                usvg::LineCap::Round => 1,
                usvg::LineCap::Square => 2,
            };
            let join_code = match stroke.linejoin() {
                usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => 0,
                usvg::LineJoin::Round => 1,
                usvg::LineJoin::Bevel => 2,
            };
            let _ = writeln!(
                w.buf,
                "q {width_pt:.4} w {cap_code} J {join_code} j {:.4} M",
                stroke.miterlimit().get().max(1.0)
            );
            if let Some(dash) = stroke.dasharray()
                && !dash.is_empty()
            {
                let dash_str: Vec<String> = dash
                    .iter()
                    .map(|d| format!("{:.4}", *d as f64 * scale_stroke * PDF_PT_PER_PAGE_UNIT))
                    .collect();
                let off = stroke.dashoffset() as f64 * scale_stroke * PDF_PT_PER_PAGE_UNIT;
                let _ = writeln!(w.buf, "[{}] {off:.4} d", dash_str.join(" "));
            }
            w.write_kurbo_elements(&els);
            let _ = writeln!(w.buf, "S Q");
        };

        if path.paint_order() == usvg::PaintOrder::FillAndStroke {
            if let Some(fill) = path.fill() {
                draw_fill(self, fill);
            }
            if let Some(stroke) = path.stroke() {
                draw_stroke(self, stroke);
            }
        } else {
            if let Some(stroke) = path.stroke() {
                draw_stroke(self, stroke);
            }
            if let Some(fill) = path.fill() {
                draw_fill(self, fill);
            }
        }
    }

    fn walk_usvg_group(
        &mut self,
        group: &usvg::Group,
        parent_xf: Affine,
        opacity_mul: f32,
        scale_stroke: f64,
    ) {
        let g = group.transform();
        let group_xf = parent_xf
            * Affine::new([
                g.sx as f64,
                g.ky as f64,
                g.kx as f64,
                g.sy as f64,
                g.tx as f64,
                g.ty as f64,
            ]);
        let op = opacity_mul * group.opacity().get();
        for node in group.children() {
            match node {
                usvg::Node::Group(sub) => {
                    self.walk_usvg_group(sub, group_xf, op, scale_stroke);
                }
                usvg::Node::Path(path) => {
                    self.write_usvg_path(path, group_xf, op, scale_stroke);
                }
                usvg::Node::Text(text) => {
                    self.walk_usvg_group(text.flattened(), group_xf, op, scale_stroke);
                }
                usvg::Node::Image(_) => {}
            }
        }
    }

    fn write_svg_tree(&mut self, tree: &usvg::Tree, pos: [f64; 2], size: [f64; 2]) {
        let svg_size = tree.size();
        let sw = (svg_size.width() as f64).max(1.0);
        let sh = (svg_size.height() as f64).max(1.0);
        let sx = size[0] / sw;
        let sy = size[1] / sh;
        let page_xf = Affine::translate((pos[0], pos[1])) * Affine::scale_non_uniform(sx, sy);
        let avg_scale = ((sx.abs() + sy.abs()) * 0.5).max(0.01);
        self.walk_usvg_group(tree.root(), page_xf, 1.0, avg_scale);
    }

    fn finish(self, page_w: f64) -> Result<PdfPageStream, ExportError> {
        let compressed_content = zlib_compress(self.buf.as_bytes())?;
        Ok(PdfPageStream {
            width_pt: page_w * PDF_PT_PER_PAGE_UNIT,
            height_pt: self.page_h * PDF_PT_PER_PAGE_UNIT,
            compressed_content,
            used_alphas: self.used_alphas,
            images: self.images,
        })
    }
}

fn zlib_compress(data: &[u8]) -> Result<Vec<u8>, ExportError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

fn build_pdf_page_stream(
    page: &DocumentPage,
    settings: &PaperSettings,
    theme: &Theme,
    usvg_opts: &usvg::Options,
) -> Result<PdfPageStream, ExportError> {
    let (page_w, page_h) = page_dimensions(settings);
    let mut writer = PdfStreamWriter::new(page_w, page_h);

    writer.write_paper(settings, theme, page_w, page_h);
    for img in &page.images {
        writer.write_image(img);
    }
    let ops = collect_stroke_vectors(page);
    writer.write_vector_ops(&ops);
    for shape in &page.shapes {
        writer.write_shape(shape);
    }
    for txt in &page.text_objects {
        if let Some(svg) = text_object_to_svg(txt, usvg_opts)
            && let Ok(tree) = usvg::Tree::from_str(&svg, usvg_opts)
        {
            let sz = tree.size();
            writer.write_svg_tree(&tree, txt.pos, [sz.width() as f64, sz.height() as f64]);
        }
    }
    for latex in &page.latex_objects {
        if let Some(svg) = latex_svg_source(latex)
            && let Ok(tree) = usvg::Tree::from_str(&svg, usvg_opts)
        {
            writer.write_svg_tree(&tree, latex.pos, latex.size);
        }
    }

    writer.finish(page_w)
}

/// Dựng các trang chỉ định thành tài liệu PDF 1.4 nhiều trang thuần vector (chữ, nét vẽ, công thức LaTeX).
pub fn render_pages_to_pdf_bytes(
    doc: &Document,
    page_indices: &[usize],
    theme: &Theme,
) -> Result<Vec<u8>, ExportError> {
    if page_indices.is_empty() {
        return Err(ExportError::EmptyDocument);
    }
    let pages = doc.pages();
    for &idx in page_indices {
        if idx >= pages.len() {
            return Err(ExportError::InvalidPageIndex(idx));
        }
    }

    let settings = doc.paper_settings();
    let usvg_opts = make_usvg_options();
    let mut rendered_pages = Vec::with_capacity(page_indices.len());
    let mut all_alphas = BTreeSet::new();
    for &idx in page_indices {
        let stream = build_pdf_page_stream(&pages[idx], settings, theme, &usvg_opts)?;
        all_alphas.extend(stream.used_alphas.iter().copied());
        rendered_pages.push(stream);
    }

    // Lập bảng cấp phát mã đối tượng PDF:
    // 1 = Catalog, 2 = Pages, tiếp theo là các ExtGState dùng chung, rồi từng Page (Page + Content + Image/SMask).
    let mut next_id: usize = 3;
    let mut gs_obj_ids: BTreeMap<u8, usize> = BTreeMap::new();
    for alpha in all_alphas {
        gs_obj_ids.insert(alpha, next_id);
        next_id += 1;
    }

    struct PageObjIds {
        page_id: usize,
        content_id: usize,
        image_ids: Vec<(String, usize, Option<usize>)>,
    }
    let mut page_obj_ids = Vec::with_capacity(rendered_pages.len());
    for p in &rendered_pages {
        let page_id = next_id;
        let content_id = next_id + 1;
        next_id += 2;
        let mut image_ids = Vec::with_capacity(p.images.len());
        for im in &p.images {
            let im_id = next_id;
            next_id += 1;
            let smask_id = im.compressed_smask.as_ref().map(|_| {
                let id = next_id;
                next_id += 1;
                id
            });
            image_ids.push((im.resource_name.clone(), im_id, smask_id));
        }
        page_obj_ids.push(PageObjIds {
            page_id,
            content_id,
            image_ids,
        });
    }

    let total_objs = next_id;
    let mut out: Vec<u8> = Vec::with_capacity(64 * 1024);
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let mut offsets = vec![0usize; total_objs];

    // Obj 1: Catalog
    offsets[1] = out.len();
    out.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // Obj 2: Pages
    offsets[2] = out.len();
    let kids: Vec<String> = page_obj_ids
        .iter()
        .map(|p| format!("{} 0 R", p.page_id))
        .collect();
    let pages_dict = format!(
        "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
        kids.join(" "),
        page_obj_ids.len()
    );
    out.extend_from_slice(pages_dict.as_bytes());

    // ExtGState objects
    for (alpha, obj_id) in &gs_obj_ids {
        offsets[*obj_id] = out.len();
        let a = *alpha as f64 / 255.0;
        let gs = format!("{obj_id} 0 obj\n<< /Type /ExtGState /ca {a:.4} /CA {a:.4} >>\nendobj\n");
        out.extend_from_slice(gs.as_bytes());
    }

    // Per-page objects
    for (page_data, ids) in rendered_pages.iter().zip(&page_obj_ids) {
        let mut resources = String::new();
        if !page_data.used_alphas.is_empty() {
            resources.push_str("/ExtGState << ");
            for a in &page_data.used_alphas {
                if let Some(gs_id) = gs_obj_ids.get(a) {
                    let _ = write!(resources, "/GS{a} {gs_id} 0 R ");
                }
            }
            resources.push_str(">> ");
        }
        if !ids.image_ids.is_empty() {
            resources.push_str("/XObject << ");
            for (name, im_id, _) in &ids.image_ids {
                let _ = write!(resources, "/{name} {im_id} 0 R ");
            }
            resources.push_str(">> ");
        }

        offsets[ids.page_id] = out.len();
        let page_obj = format!(
            "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.4} {:.4}] \
             /Resources << {resources}>> /Contents {} 0 R >>\nendobj\n",
            ids.page_id, page_data.width_pt, page_data.height_pt, ids.content_id
        );
        out.extend_from_slice(page_obj.as_bytes());

        offsets[ids.content_id] = out.len();
        let content_hdr = format!(
            "{} 0 obj\n<< /Length {} /Filter /FlateDecode >>\nstream\n",
            ids.content_id,
            page_data.compressed_content.len()
        );
        out.extend_from_slice(content_hdr.as_bytes());
        out.extend_from_slice(&page_data.compressed_content);
        out.extend_from_slice(b"\nendstream\nendobj\n");

        for (im_stream, (_, im_id, smask_id)) in page_data.images.iter().zip(&ids.image_ids) {
            offsets[*im_id] = out.len();
            let smask_entry = smask_id
                .map(|sid| format!("/SMask {sid} 0 R "))
                .unwrap_or_default();
            let im_hdr = format!(
                "{im_id} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} \
                 /ColorSpace /DeviceRGB /BitsPerComponent 8 {smask_entry}/Filter /FlateDecode \
                 /Length {} >>\nstream\n",
                im_stream.width,
                im_stream.height,
                im_stream.compressed_rgb.len()
            );
            out.extend_from_slice(im_hdr.as_bytes());
            out.extend_from_slice(&im_stream.compressed_rgb);
            out.extend_from_slice(b"\nendstream\nendobj\n");

            if let (Some(sid), Some(smask_bytes)) = (smask_id, &im_stream.compressed_smask) {
                offsets[*sid] = out.len();
                let smask_hdr = format!(
                    "{sid} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} \
                     /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode \
                     /Length {} >>\nstream\n",
                    im_stream.width,
                    im_stream.height,
                    smask_bytes.len()
                );
                out.extend_from_slice(smask_hdr.as_bytes());
                out.extend_from_slice(smask_bytes);
                out.extend_from_slice(b"\nendstream\nendobj\n");
            }
        }
    }

    let xref_offset = out.len();
    let xref_hdr = format!("xref\n0 {total_objs}\n0000000000 65535 f \n");
    out.extend_from_slice(xref_hdr.as_bytes());
    for off in &offsets[1..] {
        let entry = format!("{off:010} 00000 n \n");
        out.extend_from_slice(entry.as_bytes());
    }
    let trailer =
        format!("trailer\n<< /Size {total_objs} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n");
    out.extend_from_slice(trailer.as_bytes());

    Ok(out)
}

/// Xuất các trang chỉ định thành file PDF tại `dest_path`.
pub fn export_pages_to_pdf_file(
    doc: &Document,
    page_indices: &[usize],
    theme: &Theme,
    dest_path: &Path,
) -> Result<PathBuf, ExportError> {
    let out_path = ensure_extension(dest_path, "pdf");
    let pdf_bytes = render_pages_to_pdf_bytes(doc, page_indices, theme)?;
    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(&out_path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(&pdf_bytes)?;
    writer.flush()?;
    Ok(out_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use inknote_core::{
        Command, Document, ImageObject, LatexObject, ShapeKind, ShapeObject, Stroke, TextAlign,
        TextFontFamily, TextListMode, TextObject,
    };
    use inknote_pen::PenPath;
    use inknote_pen::brushconfig::{BrushConfig, BrushStyle};
    use inknote_pen::penpath::{Element, Segment};
    use parry2d_f64::math::Vector2;

    fn sample_doc_multi_page() -> Document {
        let mut doc = Document::new();

        // Trang 1: nét bút Rnote + hình học + văn bản tiếng Việt + công thức LaTeX SVG + ảnh RGBA trong suốt một phần
        let mut cfg = BrushConfig::default();
        cfg.style = BrushStyle::Solid;
        cfg.solid_options.stroke_width = 3.0;
        let p0 = Element::new(Vector2::new(100.0, 140.0), 0.7);
        let p1 = Element::new(Vector2::new(280.0, 190.0), 0.8);
        let path = PenPath::new_w_segments(p0, vec![Segment::LineTo { end: p1 }]);
        let s_id = doc.alloc_id();
        doc.execute(Command::AddStrokes(vec![Stroke {
            id: s_id,
            path,
            style: cfg.style_for_current_options(),
        }]));

        let sh_id = doc.alloc_id();
        doc.execute(Command::AddShape {
            page_index: 0,
            obj: ShapeObject {
                id: sh_id,
                kind: ShapeKind::Arrow,
                start: [120.0, 260.0],
                end: [360.0, 260.0],
                stroke_width: 3.0,
                color: [0, 102, 204, 255],
                dashed: false,
            },
        });

        let tx_id = doc.alloc_id();
        doc.execute(Command::AddText {
            page_index: 0,
            obj: TextObject {
                id: tx_id,
                pos: [120.0, 320.0],
                size: [360.0, 80.0],
                text: "Xin chào Tiếng Việt: ắ ằ ẳ ẵ ặ ơ ư đ Đ".to_string(),
                font_family: TextFontFamily::Sans,
                font_size: 20.0,
                bold: true,
                italic: false,
                align: TextAlign::Left,
                list_mode: TextListMode::None,
                color: [15, 23, 42, 255],
            },
        });

        let ltx_id = doc.alloc_id();
        let sample_svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"80\" height=\"40\" viewBox=\"0 0 80 40\">\
                          <path d=\"M 4 30 L 35 6 L 76 30 Z\" fill=\"#cc0000\"/></svg>";
        doc.execute(Command::AddLatex {
            page_index: 0,
            obj: LatexObject {
                id: ltx_id,
                pos: [140.0, 440.0],
                size: [160.0, 80.0],
                source: "a^2+b^2=c^2".to_string(),
                svg_cache: Some(sample_svg.to_string()),
            },
        });

        // Tạo ảnh PNG nhỏ 16x16 màu xanh có kênh alpha để kiểm tra SMask + Lanczos3
        let mut rgba = image::RgbaImage::new(16, 16);
        for (x, y, px) in rgba.enumerate_pixels_mut() {
            let alpha = if x < 8 && y < 8 { 128 } else { 255 };
            *px = image::Rgba([24, 144, 255, alpha]);
        }
        let mut png_buf = Vec::new();
        let enc = image::codecs::png::PngEncoder::new(&mut png_buf);
        enc.write_image(&rgba, 16, 16, image::ExtendedColorType::Rgba8)
            .expect("encode test png");
        let im_id = doc.alloc_id();
        doc.execute(Command::AddImage {
            page_index: 0,
            obj: ImageObject {
                id: im_id,
                pos: [450.0, 140.0],
                size: [96.0, 96.0],
                pixel_size: [16, 16],
                bytes: png_buf,
            },
        });

        // Trang 2: nét Highlighter mờ & chữ nhật bo góc nét đứt
        doc.execute(Command::InsertPageAfter(0));
        let mut hl_cfg = BrushConfig::default();
        hl_cfg.style = BrushStyle::Marker;
        hl_cfg.marker_options.stroke_width = 16.0;
        hl_cfg.marker_options.stroke_color = Some(inknote_pen::Color::new(
            250.0 / 255.0,
            204.0 / 255.0,
            21.0 / 255.0,
            0.42,
        ));
        let hp0 = Element::new(Vector2::new(120.0, 200.0), 0.6);
        let hp1 = Element::new(Vector2::new(400.0, 200.0), 0.6);
        let h_path = PenPath::new_w_segments(hp0, vec![Segment::LineTo { end: hp1 }]);
        let hs_id = doc.alloc_id();
        doc.execute(Command::AddStrokes(vec![Stroke {
            id: hs_id,
            path: h_path,
            style: hl_cfg.style_for_current_options(),
        }]));
        let sh2_id = doc.alloc_id();
        doc.execute(Command::AddShape {
            page_index: 1,
            obj: ShapeObject {
                id: sh2_id,
                kind: ShapeKind::RoundedRect,
                start: [140.0, 260.0],
                end: [380.0, 380.0],
                stroke_width: 2.5,
                color: [16, 185, 129, 255],
                dashed: true,
            },
        });

        doc
    }

    #[test]
    fn png_export_is_high_resolution_and_renders_all_layers() {
        let doc = sample_doc_multi_page();
        let theme = Theme::default();
        let png_bytes =
            render_page_to_png_bytes(&doc.pages()[0], doc.paper_settings(), &theme, PngScale::X3)
                .expect("render png");
        assert!(png_bytes.starts_with(b"\x89PNG\r\n\x1a\n"));

        let decoded = image::load_from_memory(&png_bytes).expect("decode exported png");
        let expected_w = (doc.paper_settings().page_width * 3.0).ceil() as u32;
        let expected_h = (doc.paper_settings().page_height * 3.0).ceil() as u32;
        assert_eq!(decoded.width(), expected_w);
        assert_eq!(decoded.height(), expected_h);

        // Kiểm tra điểm nằm trong vùng công thức LaTeX tam giác đỏ (khoảng 180, 480) có pixel đỏ/khác trắng
        let rgba = decoded.to_rgba8();
        let sample = rgba.get_pixel(180 * 3, 480 * 3);
        assert!(
            sample[0] > 180 && sample[1] < 80,
            "expected red LaTeX vector fill at high-DPI sample point, got {:?}",
            sample
        );
    }

    #[test]
    fn pdf_export_generates_valid_multi_page_vector_pdf() {
        let doc = sample_doc_multi_page();
        let theme = Theme::default();
        let pdf_bytes = render_pages_to_pdf_bytes(&doc, &[0, 1], &theme).expect("render pdf");

        assert!(pdf_bytes.starts_with(b"%PDF-1.4\n"));
        assert!(pdf_bytes.ends_with(b"%%EOF\n"));

        let pdf_text = String::from_utf8_lossy(&pdf_bytes);
        let expected_w_pt = doc.paper_settings().page_width as f64 * PDF_PT_PER_PAGE_UNIT;
        let expected_h_pt = doc.paper_settings().page_height as f64 * PDF_PT_PER_PAGE_UNIT;
        let expected_box = format!("/MediaBox [0 0 {expected_w_pt:.4} {expected_h_pt:.4}]");
        assert!(pdf_text.contains("/Type /Catalog"));
        assert!(pdf_text.contains("/Type /Pages"));
        assert!(pdf_text.contains("/Count 2"));
        assert!(
            pdf_text.contains(&expected_box),
            "missing expected {expected_box}"
        );
        assert!(pdf_text.contains("/Filter /FlateDecode"));
        assert!(pdf_text.contains("/Subtype /Image"));
        assert!(pdf_text.contains("/SMask "));
        assert!(pdf_text.contains("/Type /ExtGState"));
        assert!(pdf_text.contains("startxref\n"));
    }

    #[test]
    fn build_png_output_paths_appends_page_numbers_for_multi_page() {
        let single = build_png_output_paths(Path::new(r"C:\notes\calc.png"), 1);
        assert_eq!(single, vec![PathBuf::from(r"C:\notes\calc.png")]);

        let multi = build_png_output_paths(Path::new(r"C:\notes\calc"), 3);
        assert_eq!(
            multi,
            vec![
                PathBuf::from(r"C:\notes\calc_page_1.png"),
                PathBuf::from(r"C:\notes\calc_page_2.png"),
                PathBuf::from(r"C:\notes\calc_page_3.png"),
            ]
        );
    }
}
