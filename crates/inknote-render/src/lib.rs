// Bộ tô nét (RenderContext) cho InkNote, thay thế piet-cairo bằng lyon_tessellation.
// Giữ nguyên tên kiểu `CairoRenderContext` để toàn bộ code Rnote trong `inknote-pen`
// không cần sửa bất kỳ dòng nào.
//
// Quy tắc tô (FillRule):
// Trong `piet-cairo` gốc của Rnote:
// - `cx.fill(...)` gọi `self.ctx.set_fill_rule(cairo::FillRule::Winding)` -> tương ứng `FillRule::NonZero` của lyon.
// - `cx.fill_even_odd(...)` gọi `self.ctx.set_fill_rule(cairo::FillRule::EvenOdd)` -> tương ứng `FillRule::EvenOdd` của lyon.

use std::borrow::Cow;
use std::ops::RangeBounds;

use kurbo::{Affine, PathEl, Point, Rect, Shape, Size};
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, LineCap as LyonCap,
    LineJoin as LyonJoin, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers,
    math::point as lyon_point, path::Path as LyonPath,
};
use piet::{
    Color, Error, FixedGradient, FontFamily, HitTestPoint, HitTestPosition, Image, ImageFormat,
    InterpolationMode, IntoBrush, LineCap, LineJoin, LineMetric, RenderContext, StrokeStyle, Text,
    TextAttribute, TextLayout, TextLayoutBuilder, TextStorage,
};

/// Một mẻ lưới tam giác (mesh) đã chia nhỏ kèm màu RGBA.
#[derive(Debug, Clone)]
pub struct DrawBatch {
    pub vertices: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub color: [u8; 4],
}

#[derive(Debug, Clone)]
pub struct Brush {
    pub color: Color,
}

#[derive(Clone, Debug)]
pub struct RenderImage;
#[derive(Clone, Debug)]
pub struct RenderText;
#[derive(Clone, Debug)]
pub struct RenderTextLayout;
#[derive(Debug)]
pub struct RenderTextLayoutBuilder;

/// Thay thế trực tiếp `piet_cairo::CairoRenderContext` bằng bộ chia tam giác `lyon`.
#[derive(Default)]
pub struct CairoRenderContext {
    pub batches: Vec<DrawBatch>,
    transform_stack: Vec<Affine>,
    current_transform: Affine,
    text: RenderText,
}

impl Default for RenderText {
    fn default() -> Self {
        Self
    }
}

impl CairoRenderContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear_batches(&mut self) {
        self.batches.clear();
    }

    fn kurbo_to_lyon(&self, shape: impl Shape) -> LyonPath {
        let mut builder = LyonPath::builder();
        let mut subpath_open = false;
        let t = self.current_transform;
        let lp = |p: Point| {
            let tp = t * p;
            lyon_point(tp.x as f32, tp.y as f32)
        };

        for el in shape.path_elements(0.1) {
            match el {
                PathEl::MoveTo(p) => {
                    if subpath_open {
                        builder.end(false);
                    }
                    builder.begin(lp(p));
                    subpath_open = true;
                }
                PathEl::LineTo(p) => {
                    if !subpath_open {
                        builder.begin(lp(p));
                        subpath_open = true;
                    } else {
                        builder.line_to(lp(p));
                    }
                }
                PathEl::QuadTo(p1, p2) => {
                    if !subpath_open {
                        builder.begin(lp(p1));
                        subpath_open = true;
                    }
                    builder.quadratic_bezier_to(lp(p1), lp(p2));
                }
                PathEl::CurveTo(p1, p2, p3) => {
                    if !subpath_open {
                        builder.begin(lp(p1));
                        subpath_open = true;
                    }
                    builder.cubic_bezier_to(lp(p1), lp(p2), lp(p3));
                }
                PathEl::ClosePath => {
                    if subpath_open {
                        builder.end(true);
                        subpath_open = false;
                    }
                }
            }
        }
        if subpath_open {
            builder.end(false);
        }
        builder.build()
    }

    fn fill_with_rule(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>, rule: FillRule) {
        let brush = brush.make_brush(self, || shape.bounding_box()).into_owned();
        let rgba = brush.color.as_rgba8();
        if rgba.3 == 0 {
            return;
        }
        let path = self.kurbo_to_lyon(shape);
        let mut geometry: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
        let mut tessellator = FillTessellator::new();
        let options = FillOptions::default()
            .with_fill_rule(rule)
            .with_tolerance(0.05);

        if tessellator
            .tessellate_path(
                &path,
                &options,
                &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
                    let p = vertex.position();
                    [p.x, p.y]
                }),
            )
            .is_ok()
            && !geometry.indices.is_empty()
        {
            self.batches.push(DrawBatch {
                vertices: geometry.vertices,
                indices: geometry.indices,
                color: [rgba.0, rgba.1, rgba.2, rgba.3],
            });
        }
    }

    fn stroke_with_options(
        &mut self,
        shape: impl Shape,
        brush: &impl IntoBrush<Self>,
        width: f64,
        style: Option<&StrokeStyle>,
    ) {
        let brush = brush.make_brush(self, || shape.bounding_box()).into_owned();
        let rgba = brush.color.as_rgba8();
        if rgba.3 == 0 || width <= 0.0 {
            return;
        }
        let path = self.kurbo_to_lyon(shape);
        let mut geometry: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
        let mut tessellator = StrokeTessellator::new();
        let mut options = StrokeOptions::default()
            .with_line_width(width as f32)
            .with_tolerance(0.05);

        if let Some(st) = style {
            options = options.with_line_cap(match st.line_cap {
                LineCap::Butt => LyonCap::Butt,
                LineCap::Round => LyonCap::Round,
                LineCap::Square => LyonCap::Square,
            });
            options = options.with_line_join(match st.line_join {
                LineJoin::Miter { limit } => {
                    options = options.with_miter_limit(limit as f32);
                    LyonJoin::Miter
                }
                LineJoin::Round => LyonJoin::Round,
                LineJoin::Bevel => LyonJoin::Bevel,
            });
        }

        if tessellator
            .tessellate_path(
                &path,
                &options,
                &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| {
                    let p = vertex.position();
                    [p.x, p.y]
                }),
            )
            .is_ok()
            && !geometry.indices.is_empty()
        {
            self.batches.push(DrawBatch {
                vertices: geometry.vertices,
                indices: geometry.indices,
                color: [rgba.0, rgba.1, rgba.2, rgba.3],
            });
        }
    }
}

impl RenderContext for CairoRenderContext {
    type Brush = Brush;
    type Image = RenderImage;
    type Text = RenderText;
    type TextLayout = RenderTextLayout;

    fn status(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn solid_brush(&mut self, color: Color) -> Self::Brush {
        Brush { color }
    }

    fn gradient(&mut self, _gradient: impl Into<FixedGradient>) -> Result<Self::Brush, Error> {
        Ok(Brush {
            color: Color::BLACK,
        })
    }

    fn clear(&mut self, _region: impl Into<Option<Rect>>, _color: Color) {
        self.batches.clear();
    }

    fn stroke(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>, width: f64) {
        self.stroke_with_options(shape, brush, width, None);
    }

    fn stroke_styled(
        &mut self,
        shape: impl Shape,
        brush: &impl IntoBrush<Self>,
        width: f64,
        style: &StrokeStyle,
    ) {
        self.stroke_with_options(shape, brush, width, Some(style));
    }

    fn fill(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
        // Đúng quy tắc của piet-cairo: cairo::FillRule::Winding == FillRule::NonZero
        self.fill_with_rule(shape, brush, FillRule::NonZero);
    }

    fn fill_even_odd(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
        self.fill_with_rule(shape, brush, FillRule::EvenOdd);
    }

    fn clip(&mut self, _shape: impl Shape) {}

    fn text(&mut self) -> &mut Self::Text {
        &mut self.text
    }

    fn draw_text(&mut self, _layout: &Self::TextLayout, _pos: impl Into<Point>) {}

    fn save(&mut self) -> Result<(), Error> {
        self.transform_stack.push(self.current_transform);
        Ok(())
    }

    fn restore(&mut self) -> Result<(), Error> {
        if let Some(t) = self.transform_stack.pop() {
            self.current_transform = t;
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn transform(&mut self, transform: Affine) {
        self.current_transform *= transform;
    }

    fn capture_image_area(&mut self, _src_rect: impl Into<Rect>) -> Result<Self::Image, Error> {
        Ok(RenderImage)
    }

    fn make_image_with_stride(
        &mut self,
        _width: usize,
        _height: usize,
        _stride: usize,
        _buf: &[u8],
        _format: ImageFormat,
    ) -> Result<Self::Image, Error> {
        Ok(RenderImage)
    }

    fn draw_image(
        &mut self,
        _image: &Self::Image,
        _dst_rect: impl Into<Rect>,
        _interp: InterpolationMode,
    ) {
    }

    fn draw_image_area(
        &mut self,
        _image: &Self::Image,
        _src_rect: impl Into<Rect>,
        _dst_rect: impl Into<Rect>,
        _interp: InterpolationMode,
    ) {
    }

    fn blurred_rect(&mut self, _rect: Rect, _blur_radius: f64, _brush: &impl IntoBrush<Self>) {}

    fn current_transform(&self) -> Affine {
        self.current_transform
    }
}

impl IntoBrush<CairoRenderContext> for Brush {
    fn make_brush<'b>(
        &'b self,
        _piet: &mut CairoRenderContext,
        _bbox: impl FnOnce() -> Rect,
    ) -> Cow<'b, Brush> {
        Cow::Borrowed(self)
    }
}

impl Image for RenderImage {
    fn size(&self) -> Size {
        Size::ZERO
    }
}

impl Text for RenderText {
    type TextLayout = RenderTextLayout;
    type TextLayoutBuilder = RenderTextLayoutBuilder;

    fn load_font(&mut self, _data: &[u8]) -> Result<FontFamily, Error> {
        Ok(FontFamily::default())
    }

    fn new_text_layout(&mut self, _text: impl TextStorage) -> Self::TextLayoutBuilder {
        RenderTextLayoutBuilder
    }

    fn font_family(&mut self, _family_name: &str) -> Option<FontFamily> {
        Some(FontFamily::default())
    }
}

impl TextLayoutBuilder for RenderTextLayoutBuilder {
    type Out = RenderTextLayout;

    fn max_width(self, _width: f64) -> Self {
        self
    }

    fn alignment(self, _alignment: piet::TextAlignment) -> Self {
        self
    }

    fn default_attribute(self, _attribute: impl Into<TextAttribute>) -> Self {
        self
    }

    fn range_attribute(
        self,
        _range: impl RangeBounds<usize>,
        _attribute: impl Into<TextAttribute>,
    ) -> Self {
        self
    }

    fn build(self) -> Result<Self::Out, Error> {
        Ok(RenderTextLayout)
    }
}

impl TextLayout for RenderTextLayout {
    fn size(&self) -> Size {
        Size::ZERO
    }

    fn trailing_whitespace_width(&self) -> f64 {
        0.0
    }

    fn image_bounds(&self) -> Rect {
        Rect::ZERO
    }

    fn line_text(&self, _line_number: usize) -> Option<&str> {
        None
    }

    fn line_metric(&self, _line_number: usize) -> Option<LineMetric> {
        None
    }

    fn line_count(&self) -> usize {
        0
    }

    fn hit_test_point(&self, _point: Point) -> HitTestPoint {
        HitTestPoint::default()
    }

    fn hit_test_text_position(&self, _text_position: usize) -> HitTestPosition {
        HitTestPosition::default()
    }

    fn text(&self) -> &str {
        ""
    }
}
