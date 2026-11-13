// InkNote core (Bước 1.7 / 2.1): dữ liệu trang, định dạng file .inknote,
// hệ thống lệnh (Command) và Undo/Redo.
// Mọi thay đổi nội dung trang PHẢI đi qua `Document::execute` để có thể hoàn tác.

use inknote_pen::{PenPath, Style, penpath::Segment};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// Số phiên bản định dạng file .inknote hiện tại.
pub const FORMAT_VERSION: u32 = 1;

/// Kiểu nền giấy (Paper Pattern – Bước 2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaperPattern {
    Plain,
    Dotted,
    Grid,
    Lined,
}

impl Default for PaperPattern {
    fn default() -> Self {
        Self::Dotted
    }
}

/// Chế độ hiển thị Canvas:
/// - `Paper`: Trang giấy chuẩn có kích thước cố định (A4, A5), bóng đổ nổi trên nền bàn làm việc.
/// - `Whiteboard`: Bảng trắng vô tận, trải rộng toàn màn hình không giới hạn biên.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CanvasMode {
    #[default]
    Paper,
    Whiteboard,
}

/// Khổ giấy tiêu chuẩn (ISO 216):
/// - A4: 840 x 1188 pt (210 x 297 mm, tỷ lệ 1 : 1.4142)
/// - A5: 594 x 840 pt (148 x 210 mm, bằng một nửa A4)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PaperSize {
    #[default]
    A4,
    A5,
    Custom,
}

impl PaperSize {
    pub fn dimensions(&self, orientation: PaperOrientation) -> (f32, f32) {
        let (mut w, mut h) = match self {
            PaperSize::A4 => (840.0, 1188.0),
            PaperSize::A5 => (594.0, 840.0),
            PaperSize::Custom => (840.0, 1188.0),
        };
        if orientation == PaperOrientation::Landscape {
            std::mem::swap(&mut w, &mut h);
        }
        (w, h)
    }

    pub fn label(&self) -> &'static str {
        match self {
            PaperSize::A4 => "A4",
            PaperSize::A5 => "A5",
            PaperSize::Custom => "Custom",
        }
    }
}

/// Chiều trang giấy: Portrait (Dọc) hoặc Landscape (Ngang)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PaperOrientation {
    #[default]
    Portrait,
    Landscape,
}

impl PaperOrientation {
    pub fn label(&self) -> &'static str {
        match self {
            PaperOrientation::Portrait => "Portrait",
            PaperOrientation::Landscape => "Landscape",
        }
    }
}

fn default_page_width() -> f32 {
    840.0
}

fn default_page_height() -> f32 {
    1188.0
}

/// Cài đặt nền giấy (Paper Settings) theo docs/02-architecture.md mục 3.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaperSettings {
    #[serde(default)]
    pub mode: CanvasMode,
    #[serde(default)]
    pub paper_size: PaperSize,
    #[serde(default)]
    pub orientation: PaperOrientation,
    pub pattern: PaperPattern,
    pub dot_spacing: f32,
    #[serde(default = "default_page_width")]
    pub page_width: f32,
    #[serde(default = "default_page_height")]
    pub page_height: f32,
    pub show_margins: bool,
    pub left_margin: f32,
    pub right_margin: f32,
}

impl PaperSettings {
    pub fn set_paper_size(&mut self, size: PaperSize) {
        self.paper_size = size;
        if size != PaperSize::Custom {
            let (w, h) = size.dimensions(self.orientation);
            self.page_width = w;
            self.page_height = h;
        }
    }

    pub fn set_orientation(&mut self, orientation: PaperOrientation) {
        self.orientation = orientation;
        let (w, h) = self.paper_size.dimensions(orientation);
        self.page_width = w;
        self.page_height = h;
    }
}

impl Default for PaperSettings {
    fn default() -> Self {
        let (w, h) = PaperSize::A4.dimensions(PaperOrientation::Portrait);
        Self {
            mode: CanvasMode::Paper,
            paper_size: PaperSize::A4,
            orientation: PaperOrientation::Portrait,
            pattern: PaperPattern::Dotted,
            dot_spacing: 24.0,
            page_width: w,
            page_height: h,
            show_margins: false,
            left_margin: 80.0,
            right_margin: 80.0,
        }
    }
}

/// Thông tin phụ của ghi chú (Metadata).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: String,
    pub created_at: u64,
    pub modified_at: u64,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            title: "Untitled Note".to_string(),
            created_at: now,
            modified_at: now,
        }
    }
}

/// Một nét bút đã hoàn thành. `id` duy nhất, dùng để app lưu bộ nhớ đệm hình vẽ.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stroke {
    pub id: u64,
    pub path: PenPath,
    pub style: Style,
}

/// Đối tượng công thức LaTeX trên trang (lưu `source` làm dữ liệu gốc + `svg_cache`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LatexObject {
    pub id: u64,
    pub source: String,
    pub pos: [f64; 2],
    pub size: [f64; 2],
    #[serde(default)]
    pub svg_cache: Option<String>,
}

/// Các loại hình học cơ bản & mở rộng (Bước 5.3 - P2, khớp docs/04-ui-spec.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeKind {
    Line,
    Rectangle,
    Circle,
    Triangle,
    RightTriangle,
    RightIsoscelesTriangle,
    Diamond,
    RoundedRect,
    Arrow,
    DoubleArrow,
    Pentagon,
    Hexagon,
    Heptagon,
    Octagon,
    Parallelogram,
    Trapezoid,
    RightTrapezoid,
    IsoscelesTrapezoid,
}

/// Đối tượng hình học vector trên trang.
/// - `start`, `end`: 2 điểm đầu mút (với `Line`/`Arrow`/`DoubleArrow`) hoặc 2 góc đối của khung bao hình.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShapeObject {
    pub id: u64,
    pub kind: ShapeKind,
    pub start: [f64; 2],
    pub end: [f64; 2],
    pub color: [u8; 4],
    pub stroke_width: f64,
    #[serde(default)]
    pub dashed: bool,
}

/// Nhóm phông chữ cho hộp văn bản (Bước 5.4 - P2, khớp docs/04-ui-spec.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextFontFamily {
    #[default]
    Sans,
    Serif,
    Mono,
}

/// Căn lề đoạn văn bản trong hộp văn bản.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Kiểu danh sách của hộp văn bản.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextListMode {
    #[default]
    None,
    Bullet,
    Numbered,
}

/// Đối tượng hộp văn bản (Text Box) trên trang.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextObject {
    pub id: u64,
    pub pos: [f64; 2],
    pub size: [f64; 2],
    pub text: String,
    #[serde(default)]
    pub font_family: TextFontFamily,
    pub font_size: f64,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub align: TextAlign,
    #[serde(default)]
    pub list_mode: TextListMode,
    pub color: [u8; 4],
}

/// Các loại đối tượng trên một trang (khớp docs/02-architecture.md mục 3).
/// Chỉ BrushStroke mới bị Eraser xoá; các loại khác không bị tẩy xoá lướt qua.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PageObject {
    BrushStroke(Stroke),
    Latex(LatexObject),
    Shape(ShapeObject),
    Text(TextObject),
    // Dự phòng mở rộng cho các bước tiếp theo (P2):
    // Image(ImageObject),
}

/// Một trang trong tài liệu.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Page {
    pub id: u64,
    pub objects: Vec<PageObject>,
}

/// Định dạng file `.inknote` hoàn chỉnh lưu xuống đĩa (khớp docs/02-architecture.md mục 3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentFile {
    pub format_version: u32,
    pub metadata: DocumentMetadata,
    pub paper_settings: PaperSettings,
    pub pages: Vec<Page>,
    pub first_page_thumbnail: Option<String>,
}

impl DocumentFile {
    /// Lưu file ra đĩa với cơ chế ghi file tạm rồi đổi tên đè lên file chính (atomic rename).
    pub fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let tmp_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => format!(".{name}.tmp"),
            None => ".inknote_save.tmp".to_string(),
        };
        let tmp_path = path.with_file_name(tmp_name);

        {
            use std::io::Write;
            let mut file = std::fs::File::create(&tmp_path)?;
            file.write_all(json.as_bytes())?;
            file.sync_all()?;
        }

        if let Err(e) = std::fs::rename(&tmp_path, path) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e);
        }
        Ok(())
    }

    /// Đọc và xác thực file `.inknote` từ đĩa.
    pub fn load_from_path(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let file: Self = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        if file.format_version > FORMAT_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "File version {} is newer than supported version {}",
                    file.format_version, FORMAT_VERSION
                ),
            ));
        }
        Ok(file)
    }
}

/// Lệnh thay đổi nội dung tài liệu / trang.
pub enum Command {
    /// Thêm một nét vào cuối trang hiện hành.
    AddStroke(Stroke),
    /// Thêm nhiều nét cùng lúc vào trang hiện hành (1 bước undo).
    AddStrokes(Vec<Stroke>),
    /// Xoá toàn bộ nét trên trang hiện hành (1 bước undo).
    ClearPage,
    /// Xoá toàn bộ nét trên trang tại vị trí `index` (1 bước undo).
    ClearPageAt(usize),
    /// Chèn một trang mới ngay trước vị trí `before_index` (1 bước undo).
    InsertPageBefore(usize),
    /// Chèn một trang mới ngay sau vị trí `after_index` (1 bước undo).
    InsertPageAfter(usize),
    /// Nhân bản trang tại vị trí `index` sang ngay sau nó (1 bước undo).
    DuplicatePage(usize),
    /// Di chuyển trang từ vị trí `from` sang vị trí `to` (1 bước undo).
    MovePage { from: usize, to: usize },
    /// Xoá trang tại vị trí `index` (chỉ xoá khi tài liệu có > 1 trang, 1 bước undo).
    DeletePage(usize),
    /// Chèn một đối tượng LaTeX vào trang tại vị trí `page_index` (1 bước undo).
    AddLatex { page_index: usize, obj: LatexObject },
    /// Cập nhật `source`, `size` và `svg_cache` của `LatexObject` (giữ nguyên vị trí `pos`, 1 bước undo).
    UpdateLatex {
        page_index: usize,
        obj_id: u64,
        new_source: String,
        new_size: [f64; 2],
        new_svg_cache: Option<String>,
    },
    /// Chèn một hình học cơ bản vào trang tại vị trí `page_index` (1 bước undo).
    AddShape { page_index: usize, obj: ShapeObject },
    /// Chèn một hộp văn bản vào trang tại vị trí `page_index` (1 bước undo).
    AddText { page_index: usize, obj: TextObject },
    /// Cập nhật toàn bộ nội dung/định dạng/vị trí hộp văn bản `new_obj` theo `id` (1 bước undo).
    UpdateText {
        page_index: usize,
        new_obj: TextObject,
    },
    /// Áp dụng kết quả tẩy nét bút trên một trang (chỉ tác động lên `strokes`, 1 bước undo).
    EraseStrokes {
        page_index: usize,
        removed: Vec<(usize, Stroke)>,
        added: Vec<Stroke>,
    },
    /// Di chuyển và/hoặc đổi kích thước vùng chọn (nét bút + LaTeX + hình học + văn bản, 1 bước undo).
    TransformSelection {
        page_index: usize,
        stroke_ids: Vec<u64>,
        latex_ids: Vec<u64>,
        shape_ids: Vec<u64>,
        text_ids: Vec<u64>,
        anchor: [f64; 2],
        scale: [f64; 2],
        delta: [f64; 2],
    },
    /// Xoá toàn bộ đối tượng đang chọn (nét bút + LaTeX + hình học + văn bản, 1 bước undo).
    DeleteSelection {
        page_index: usize,
        stroke_ids: Vec<u64>,
        latex_ids: Vec<u64>,
        shape_ids: Vec<u64>,
        text_ids: Vec<u64>,
    },
}

/// Một trang trong bộ nhớ làm việc của `Document`.
#[derive(Clone, Debug)]
pub struct DocumentPage {
    pub id: u64,
    pub strokes: Vec<Stroke>,
    pub latex_objects: Vec<LatexObject>,
    pub shapes: Vec<ShapeObject>,
    pub text_objects: Vec<TextObject>,
}

/// Lệnh đã thực thi, kèm dữ liệu cần để hoàn tác.
enum Done {
    AddStroke {
        page_index: usize,
        stroke: Stroke,
    },
    AddStrokes {
        page_index: usize,
        strokes: Vec<Stroke>,
    },
    ClearPage {
        page_index: usize,
        removed: Vec<Stroke>,
    },
    InsertPage {
        index: usize,
        page: DocumentPage,
        prev_active: usize,
    },
    MovePage {
        from: usize,
        to: usize,
        prev_active: usize,
    },
    DeletePage {
        index: usize,
        page: DocumentPage,
        prev_active: usize,
    },
    AddLatex {
        page_index: usize,
        obj: LatexObject,
    },
    UpdateLatex {
        page_index: usize,
        obj_id: u64,
        old_source: String,
        old_size: [f64; 2],
        old_svg_cache: Option<String>,
        new_source: String,
        new_size: [f64; 2],
        new_svg_cache: Option<String>,
    },
    AddShape {
        page_index: usize,
        obj: ShapeObject,
    },
    AddText {
        page_index: usize,
        obj: TextObject,
    },
    UpdateText {
        page_index: usize,
        old_obj: TextObject,
        new_obj: TextObject,
    },
    EraseStrokes {
        page_index: usize,
        removed: Vec<(usize, Stroke)>,
        added: Vec<Stroke>,
    },
    TransformSelection {
        page_index: usize,
        old_strokes: Vec<(usize, Stroke)>,
        new_strokes: Vec<(usize, Stroke)>,
        old_latex: Vec<(usize, LatexObject)>,
        new_latex: Vec<(usize, LatexObject)>,
        old_shapes: Vec<(usize, ShapeObject)>,
        new_shapes: Vec<(usize, ShapeObject)>,
        old_texts: Vec<(usize, TextObject)>,
        new_texts: Vec<(usize, TextObject)>,
    },
    DeleteSelection {
        page_index: usize,
        removed_strokes: Vec<(usize, Stroke)>,
        removed_latex: Vec<(usize, LatexObject)>,
        removed_shapes: Vec<(usize, ShapeObject)>,
        removed_texts: Vec<(usize, TextObject)>,
    },
}

fn transform_xy(p: [f64; 2], anchor: [f64; 2], scale: [f64; 2], delta: [f64; 2]) -> [f64; 2] {
    [
        anchor[0] + (p[0] - anchor[0]) * scale[0] + delta[0],
        anchor[1] + (p[1] - anchor[1]) * scale[1] + delta[1],
    ]
}

pub fn transform_stroke(stroke: &mut Stroke, anchor: [f64; 2], scale: [f64; 2], delta: [f64; 2]) {
    let start_xy = transform_xy(
        [stroke.path.start.pos.x, stroke.path.start.pos.y],
        anchor,
        scale,
        delta,
    );
    stroke.path.start.pos.x = start_xy[0];
    stroke.path.start.pos.y = start_xy[1];
    for seg in &mut stroke.path.segments {
        match seg {
            Segment::LineTo { end } => {
                let xy = transform_xy([end.pos.x, end.pos.y], anchor, scale, delta);
                end.pos.x = xy[0];
                end.pos.y = xy[1];
            }
            Segment::QuadBezTo { cp, end } => {
                let cp_xy = transform_xy([cp.x, cp.y], anchor, scale, delta);
                cp.x = cp_xy[0];
                cp.y = cp_xy[1];
                let xy = transform_xy([end.pos.x, end.pos.y], anchor, scale, delta);
                end.pos.x = xy[0];
                end.pos.y = xy[1];
            }
            Segment::CubBezTo { cp1, cp2, end } => {
                let c1 = transform_xy([cp1.x, cp1.y], anchor, scale, delta);
                cp1.x = c1[0];
                cp1.y = c1[1];
                let c2 = transform_xy([cp2.x, cp2.y], anchor, scale, delta);
                cp2.x = c2[0];
                cp2.y = c2[1];
                let xy = transform_xy([end.pos.x, end.pos.y], anchor, scale, delta);
                end.pos.x = xy[0];
                end.pos.y = xy[1];
            }
        }
    }
}

pub fn transform_latex(obj: &mut LatexObject, anchor: [f64; 2], scale: [f64; 2], delta: [f64; 2]) {
    let p0 = transform_xy(obj.pos, anchor, scale, delta);
    let p1 = transform_xy(
        [obj.pos[0] + obj.size[0], obj.pos[1] + obj.size[1]],
        anchor,
        scale,
        delta,
    );
    obj.pos = [p0[0].min(p1[0]), p0[1].min(p1[1])];
    obj.size = [
        (p1[0] - p0[0]).abs().max(8.0),
        (p1[1] - p0[1]).abs().max(8.0),
    ];
}

pub fn transform_shape(obj: &mut ShapeObject, anchor: [f64; 2], scale: [f64; 2], delta: [f64; 2]) {
    obj.start = transform_xy(obj.start, anchor, scale, delta);
    obj.end = transform_xy(obj.end, anchor, scale, delta);
}

pub fn transform_text(obj: &mut TextObject, anchor: [f64; 2], scale: [f64; 2], delta: [f64; 2]) {
    let p0 = transform_xy(obj.pos, anchor, scale, delta);
    let p1 = transform_xy(
        [obj.pos[0] + obj.size[0], obj.pos[1] + obj.size[1]],
        anchor,
        scale,
        delta,
    );
    obj.pos = [p0[0].min(p1[0]), p0[1].min(p1[1])];
    obj.size = [
        (p1[0] - p0[0]).abs().max(48.0),
        (p1[1] - p0[1]).abs().max(20.0),
    ];
    let sy = scale[1].abs();
    if (sy - 1.0).abs() > 1e-4 {
        obj.font_size = (obj.font_size * sy).clamp(8.0, 120.0);
    }
}

/// Tài liệu nhiều trang + lịch sử Undo/Redo + trạng thái lưu file.
pub struct Document {
    metadata: DocumentMetadata,
    paper_settings: PaperSettings,
    pages: Vec<DocumentPage>,
    active_page_idx: usize,
    undo_stack: Vec<Done>,
    redo_stack: Vec<Done>,
    next_id: u64,
    next_page_id: u64,
    rev: u64,
    saved_rev: u64,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            metadata: DocumentMetadata::default(),
            paper_settings: PaperSettings::default(),
            pages: vec![DocumentPage {
                id: 1,
                strokes: Vec::new(),
                latex_objects: Vec::new(),
                shapes: Vec::new(),
                text_objects: Vec::new(),
            }],
            active_page_idx: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            next_id: 0,
            next_page_id: 1,
            rev: 0,
            saved_rev: 0,
        }
    }
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn metadata(&self) -> &DocumentMetadata {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut DocumentMetadata {
        &mut self.metadata
    }

    pub fn paper_settings(&self) -> &PaperSettings {
        &self.paper_settings
    }

    pub fn paper_settings_mut(&mut self) -> &mut PaperSettings {
        &mut self.paper_settings
    }

    /// Cập nhật cài đặt nền giấy; nếu có thay đổi sẽ tăng `revision` để đánh dấu cần lưu.
    pub fn set_paper_settings(&mut self, settings: PaperSettings) -> bool {
        if self.paper_settings == settings {
            return false;
        }
        self.paper_settings = settings;
        self.rev += 1;
        true
    }

    /// Cấp id mới cho nét bút.
    pub fn alloc_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn alloc_page_id(&mut self) -> u64 {
        self.next_page_id += 1;
        self.next_page_id
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn active_page_index(&self) -> usize {
        self.active_page_idx
    }

    pub fn set_active_page_index(&mut self, index: usize) -> bool {
        if self.pages.is_empty() {
            return false;
        }
        let clamped = index.min(self.pages.len() - 1);
        if self.active_page_idx == clamped {
            return false;
        }
        self.active_page_idx = clamped;
        true
    }

    pub fn pages(&self) -> &[DocumentPage] {
        &self.pages
    }

    pub fn page_strokes(&self, page_index: usize) -> &[Stroke] {
        self.pages
            .get(page_index)
            .map(|p| p.strokes.as_slice())
            .unwrap_or(&[])
    }

    pub fn strokes(&self) -> &[Stroke] {
        self.page_strokes(self.active_page_idx)
    }

    /// Số đổi mỗi khi nội dung trang thay đổi (execute/undo/redo) — dùng để làm mới bộ nhớ đệm hình vẽ.
    pub fn revision(&self) -> u64 {
        self.rev
    }

    /// Kiểm tra xem tài liệu có thay đổi chưa được lưu xuống đĩa hay không.
    pub fn is_dirty(&self) -> bool {
        self.rev != self.saved_rev
    }

    /// Đánh dấu đã lưu thành công ở revision hiện tại.
    pub fn mark_saved(&mut self) {
        self.mark_saved_at(self.rev);
    }

    /// Đánh dấu đã lưu thành công tại một mốc `rev` cụ thể (hữu ích khi lưu trên luồng nền).
    pub fn mark_saved_at(&mut self, rev: u64) {
        let clamped = rev.min(self.rev);
        if clamped > self.saved_rev {
            self.saved_rev = clamped;
        }
        self.metadata.modified_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Thực thi lệnh mới. Xoá nhánh Redo (như mọi trình soạn thảo).
    pub fn execute(&mut self, cmd: Command) {
        let page_index = self.active_page_idx;
        let done = match cmd {
            Command::AddStroke(s) => {
                self.pages[page_index].strokes.push(s.clone());
                Done::AddStroke {
                    page_index,
                    stroke: s,
                }
            }
            Command::AddStrokes(list) => {
                if list.is_empty() {
                    return;
                }
                self.pages[page_index].strokes.extend(list.iter().cloned());
                Done::AddStrokes {
                    page_index,
                    strokes: list,
                }
            }
            Command::ClearPage => {
                if self.pages[page_index].strokes.is_empty() {
                    return; // không có gì để xoá → không tạo bước undo rỗng
                }
                let removed = std::mem::take(&mut self.pages[page_index].strokes);
                Done::ClearPage {
                    page_index,
                    removed,
                }
            }
            Command::ClearPageAt(idx) => {
                let target_idx = idx.min(self.pages.len().saturating_sub(1));
                if self.pages[target_idx].strokes.is_empty() {
                    return;
                }
                let removed = std::mem::take(&mut self.pages[target_idx].strokes);
                Done::ClearPage {
                    page_index: target_idx,
                    removed,
                }
            }
            Command::InsertPageBefore(before_idx) => {
                let insert_idx = before_idx.min(self.pages.len().saturating_sub(1));
                let page_id = self.alloc_page_id();
                let page = DocumentPage {
                    id: page_id,
                    strokes: Vec::new(),
                    latex_objects: Vec::new(),
                    shapes: Vec::new(),
                    text_objects: Vec::new(),
                };
                let prev_active = self.active_page_idx;
                self.pages.insert(insert_idx, page.clone());
                self.active_page_idx = insert_idx;
                Done::InsertPage {
                    index: insert_idx,
                    page,
                    prev_active,
                }
            }
            Command::InsertPageAfter(after_idx) => {
                let insert_idx = (after_idx.min(self.pages.len().saturating_sub(1))) + 1;
                let page_id = self.alloc_page_id();
                let page = DocumentPage {
                    id: page_id,
                    strokes: Vec::new(),
                    latex_objects: Vec::new(),
                    shapes: Vec::new(),
                    text_objects: Vec::new(),
                };
                let prev_active = self.active_page_idx;
                self.pages.insert(insert_idx, page.clone());
                self.active_page_idx = insert_idx;
                Done::InsertPage {
                    index: insert_idx,
                    page,
                    prev_active,
                }
            }
            Command::DuplicatePage(src_idx) => {
                let src = src_idx.min(self.pages.len().saturating_sub(1));
                let insert_idx = src + 1;
                let page_id = self.alloc_page_id();
                let src_strokes = self.pages[src].strokes.clone();
                let cloned_strokes: Vec<Stroke> = src_strokes
                    .into_iter()
                    .map(|s| Stroke {
                        id: self.alloc_id(),
                        path: s.path,
                        style: s.style,
                    })
                    .collect();
                let src_latex = self.pages[src].latex_objects.clone();
                let cloned_latex: Vec<LatexObject> = src_latex
                    .into_iter()
                    .map(|mut l| {
                        l.id = self.alloc_id();
                        l
                    })
                    .collect();
                let src_shapes = self.pages[src].shapes.clone();
                let cloned_shapes: Vec<ShapeObject> = src_shapes
                    .into_iter()
                    .map(|mut sh| {
                        sh.id = self.alloc_id();
                        sh
                    })
                    .collect();
                let src_texts = self.pages[src].text_objects.clone();
                let cloned_texts: Vec<TextObject> = src_texts
                    .into_iter()
                    .map(|mut txt| {
                        txt.id = self.alloc_id();
                        txt
                    })
                    .collect();
                let page = DocumentPage {
                    id: page_id,
                    strokes: cloned_strokes,
                    latex_objects: cloned_latex,
                    shapes: cloned_shapes,
                    text_objects: cloned_texts,
                };
                let prev_active = self.active_page_idx;
                self.pages.insert(insert_idx, page.clone());
                self.active_page_idx = insert_idx;
                Done::InsertPage {
                    index: insert_idx,
                    page,
                    prev_active,
                }
            }
            Command::MovePage { from, to } => {
                if self.pages.len() <= 1 {
                    return;
                }
                let from_idx = from.min(self.pages.len() - 1);
                let to_idx = to.min(self.pages.len() - 1);
                if from_idx == to_idx {
                    return;
                }
                let prev_active = self.active_page_idx;
                let page = self.pages.remove(from_idx);
                self.pages.insert(to_idx, page);
                self.active_page_idx = to_idx;
                Done::MovePage {
                    from: from_idx,
                    to: to_idx,
                    prev_active,
                }
            }
            Command::DeletePage(idx) => {
                if self.pages.len() <= 1 {
                    return; // luôn giữ ít nhất 1 trang trong tài liệu
                }
                let del_idx = idx.min(self.pages.len() - 1);
                let prev_active = self.active_page_idx;
                let page = self.pages.remove(del_idx);
                self.active_page_idx = del_idx.min(self.pages.len() - 1);
                Done::DeletePage {
                    index: del_idx,
                    page,
                    prev_active,
                }
            }
            Command::AddLatex { page_index, obj } => {
                let target_idx = page_index.min(self.pages.len().saturating_sub(1));
                self.pages[target_idx].latex_objects.push(obj.clone());
                self.active_page_idx = target_idx;
                Done::AddLatex {
                    page_index: target_idx,
                    obj,
                }
            }
            Command::UpdateLatex {
                page_index,
                obj_id,
                new_source,
                new_size,
                new_svg_cache,
            } => {
                let target_idx = page_index.min(self.pages.len().saturating_sub(1));
                let Some(obj) = self.pages[target_idx]
                    .latex_objects
                    .iter_mut()
                    .find(|o| o.id == obj_id)
                else {
                    return;
                };
                if obj.source == new_source
                    && obj.size == new_size
                    && obj.svg_cache == new_svg_cache
                {
                    return;
                }
                let old_source = std::mem::replace(&mut obj.source, new_source.clone());
                let old_size = std::mem::replace(&mut obj.size, new_size);
                let old_svg_cache = std::mem::replace(&mut obj.svg_cache, new_svg_cache.clone());
                self.active_page_idx = target_idx;
                Done::UpdateLatex {
                    page_index: target_idx,
                    obj_id,
                    old_source,
                    old_size,
                    old_svg_cache,
                    new_source,
                    new_size,
                    new_svg_cache,
                }
            }
            Command::AddShape { page_index, obj } => {
                let target_idx = page_index.min(self.pages.len().saturating_sub(1));
                self.pages[target_idx].shapes.push(obj.clone());
                self.active_page_idx = target_idx;
                Done::AddShape {
                    page_index: target_idx,
                    obj,
                }
            }
            Command::AddText { page_index, obj } => {
                let target_idx = page_index.min(self.pages.len().saturating_sub(1));
                self.pages[target_idx].text_objects.push(obj.clone());
                self.active_page_idx = target_idx;
                Done::AddText {
                    page_index: target_idx,
                    obj,
                }
            }
            Command::UpdateText {
                page_index,
                new_obj,
            } => {
                let target_idx = page_index.min(self.pages.len().saturating_sub(1));
                let Some(obj) = self.pages[target_idx]
                    .text_objects
                    .iter_mut()
                    .find(|o| o.id == new_obj.id)
                else {
                    return;
                };
                if *obj == new_obj {
                    return;
                }
                let old_obj = obj.clone();
                *obj = new_obj.clone();
                self.active_page_idx = target_idx;
                Done::UpdateText {
                    page_index: target_idx,
                    old_obj,
                    new_obj,
                }
            }
            Command::EraseStrokes {
                page_index,
                mut removed,
                added,
            } => {
                let target_idx = page_index.min(self.pages.len().saturating_sub(1));
                if removed.is_empty() && added.is_empty() {
                    return;
                }
                removed.sort_by_key(|(idx, _)| *idx);
                let strokes = &mut self.pages[target_idx].strokes;
                for &(idx, _) in removed.iter().rev() {
                    if idx < strokes.len() {
                        strokes.remove(idx);
                    }
                }
                strokes.extend(added.clone());
                self.active_page_idx = target_idx;
                Done::EraseStrokes {
                    page_index: target_idx,
                    removed,
                    added,
                }
            }
            Command::TransformSelection {
                page_index,
                stroke_ids,
                latex_ids,
                shape_ids,
                text_ids,
                anchor,
                scale,
                delta,
            } => {
                let target_idx = page_index.min(self.pages.len().saturating_sub(1));
                let s_set: HashSet<u64> = stroke_ids.into_iter().collect();
                let l_set: HashSet<u64> = latex_ids.into_iter().collect();
                let sh_set: HashSet<u64> = shape_ids.into_iter().collect();
                let txt_set: HashSet<u64> = text_ids.into_iter().collect();
                let mut old_strokes = Vec::new();
                let mut new_strokes = Vec::new();
                let mut old_latex = Vec::new();
                let mut new_latex = Vec::new();
                let mut old_shapes = Vec::new();
                let mut new_shapes = Vec::new();
                let mut old_texts = Vec::new();
                let mut new_texts = Vec::new();

                let page = &mut self.pages[target_idx];
                for (idx, s) in page.strokes.iter_mut().enumerate() {
                    if s_set.contains(&s.id) {
                        old_strokes.push((idx, s.clone()));
                        transform_stroke(s, anchor, scale, delta);
                        new_strokes.push((idx, s.clone()));
                    }
                }
                for (idx, l) in page.latex_objects.iter_mut().enumerate() {
                    if l_set.contains(&l.id) {
                        old_latex.push((idx, l.clone()));
                        transform_latex(l, anchor, scale, delta);
                        new_latex.push((idx, l.clone()));
                    }
                }
                for (idx, sh) in page.shapes.iter_mut().enumerate() {
                    if sh_set.contains(&sh.id) {
                        old_shapes.push((idx, sh.clone()));
                        transform_shape(sh, anchor, scale, delta);
                        new_shapes.push((idx, sh.clone()));
                    }
                }
                for (idx, txt) in page.text_objects.iter_mut().enumerate() {
                    if txt_set.contains(&txt.id) {
                        old_texts.push((idx, txt.clone()));
                        transform_text(txt, anchor, scale, delta);
                        new_texts.push((idx, txt.clone()));
                    }
                }
                if old_strokes.is_empty()
                    && old_latex.is_empty()
                    && old_shapes.is_empty()
                    && old_texts.is_empty()
                {
                    return;
                }
                self.active_page_idx = target_idx;
                Done::TransformSelection {
                    page_index: target_idx,
                    old_strokes,
                    new_strokes,
                    old_latex,
                    new_latex,
                    old_shapes,
                    new_shapes,
                    old_texts,
                    new_texts,
                }
            }
            Command::DeleteSelection {
                page_index,
                stroke_ids,
                latex_ids,
                shape_ids,
                text_ids,
            } => {
                let target_idx = page_index.min(self.pages.len().saturating_sub(1));
                let s_set: HashSet<u64> = stroke_ids.into_iter().collect();
                let l_set: HashSet<u64> = latex_ids.into_iter().collect();
                let sh_set: HashSet<u64> = shape_ids.into_iter().collect();
                let txt_set: HashSet<u64> = text_ids.into_iter().collect();
                let page = &mut self.pages[target_idx];

                let removed_strokes: Vec<(usize, Stroke)> = page
                    .strokes
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s_set.contains(&s.id))
                    .map(|(i, s)| (i, s.clone()))
                    .collect();
                let removed_latex: Vec<(usize, LatexObject)> = page
                    .latex_objects
                    .iter()
                    .enumerate()
                    .filter(|(_, l)| l_set.contains(&l.id))
                    .map(|(i, l)| (i, l.clone()))
                    .collect();
                let removed_shapes: Vec<(usize, ShapeObject)> = page
                    .shapes
                    .iter()
                    .enumerate()
                    .filter(|(_, sh)| sh_set.contains(&sh.id))
                    .map(|(i, sh)| (i, sh.clone()))
                    .collect();
                let removed_texts: Vec<(usize, TextObject)> = page
                    .text_objects
                    .iter()
                    .enumerate()
                    .filter(|(_, txt)| txt_set.contains(&txt.id))
                    .map(|(i, txt)| (i, txt.clone()))
                    .collect();

                if removed_strokes.is_empty()
                    && removed_latex.is_empty()
                    && removed_shapes.is_empty()
                    && removed_texts.is_empty()
                {
                    return;
                }
                for &(idx, _) in removed_strokes.iter().rev() {
                    page.strokes.remove(idx);
                }
                for &(idx, _) in removed_latex.iter().rev() {
                    page.latex_objects.remove(idx);
                }
                for &(idx, _) in removed_shapes.iter().rev() {
                    page.shapes.remove(idx);
                }
                for &(idx, _) in removed_texts.iter().rev() {
                    page.text_objects.remove(idx);
                }
                self.active_page_idx = target_idx;
                Done::DeleteSelection {
                    page_index: target_idx,
                    removed_strokes,
                    removed_latex,
                    removed_shapes,
                    removed_texts,
                }
            }
        };
        self.undo_stack.push(done);
        self.redo_stack.clear();
        self.rev += 1;
    }

    /// Hoàn tác 1 bước. Trả về `true` nếu có thay đổi.
    pub fn undo(&mut self) -> bool {
        let Some(done) = self.undo_stack.pop() else {
            return false;
        };
        match &done {
            Done::AddStroke { page_index, .. } => {
                self.pages[*page_index].strokes.pop();
                self.active_page_idx = *page_index;
            }
            Done::AddStrokes {
                page_index,
                strokes,
            } => {
                let target = &mut self.pages[*page_index].strokes;
                let keep = target.len().saturating_sub(strokes.len());
                target.truncate(keep);
                self.active_page_idx = *page_index;
            }
            Done::ClearPage {
                page_index,
                removed,
            } => {
                self.pages[*page_index].strokes = removed.clone();
                self.active_page_idx = *page_index;
            }
            Done::InsertPage {
                index, prev_active, ..
            } => {
                self.pages.remove(*index);
                self.active_page_idx = (*prev_active).min(self.pages.len().saturating_sub(1));
            }
            Done::MovePage {
                from,
                to,
                prev_active,
            } => {
                let page = self.pages.remove(*to);
                self.pages.insert(*from, page);
                self.active_page_idx = (*prev_active).min(self.pages.len().saturating_sub(1));
            }
            Done::DeletePage {
                index,
                page,
                prev_active,
            } => {
                self.pages.insert(*index, page.clone());
                self.active_page_idx = (*prev_active).min(self.pages.len().saturating_sub(1));
            }
            Done::AddLatex { page_index, .. } => {
                self.pages[*page_index].latex_objects.pop();
                self.active_page_idx = *page_index;
            }
            Done::AddShape { page_index, .. } => {
                self.pages[*page_index].shapes.pop();
                self.active_page_idx = *page_index;
            }
            Done::AddText { page_index, .. } => {
                self.pages[*page_index].text_objects.pop();
                self.active_page_idx = *page_index;
            }
            Done::UpdateText {
                page_index,
                old_obj,
                ..
            } => {
                if let Some(obj) = self.pages[*page_index]
                    .text_objects
                    .iter_mut()
                    .find(|o| o.id == old_obj.id)
                {
                    *obj = old_obj.clone();
                }
                self.active_page_idx = *page_index;
            }
            Done::UpdateLatex {
                page_index,
                obj_id,
                old_source,
                old_size,
                old_svg_cache,
                ..
            } => {
                if let Some(obj) = self.pages[*page_index]
                    .latex_objects
                    .iter_mut()
                    .find(|o| o.id == *obj_id)
                {
                    obj.source = old_source.clone();
                    obj.size = *old_size;
                    obj.svg_cache = old_svg_cache.clone();
                }
                self.active_page_idx = *page_index;
            }
            Done::EraseStrokes {
                page_index,
                removed,
                added,
            } => {
                let strokes = &mut self.pages[*page_index].strokes;
                let keep = strokes.len().saturating_sub(added.len());
                strokes.truncate(keep);
                for (idx, stroke) in removed {
                    let insert_at = (*idx).min(strokes.len());
                    strokes.insert(insert_at, stroke.clone());
                }
                self.active_page_idx = *page_index;
            }
            Done::TransformSelection {
                page_index,
                old_strokes,
                old_latex,
                old_shapes,
                old_texts,
                ..
            } => {
                let page = &mut self.pages[*page_index];
                for (idx, s) in old_strokes {
                    if *idx < page.strokes.len() {
                        page.strokes[*idx] = s.clone();
                    }
                }
                for (idx, l) in old_latex {
                    if *idx < page.latex_objects.len() {
                        page.latex_objects[*idx] = l.clone();
                    }
                }
                for (idx, sh) in old_shapes {
                    if *idx < page.shapes.len() {
                        page.shapes[*idx] = sh.clone();
                    }
                }
                for (idx, txt) in old_texts {
                    if *idx < page.text_objects.len() {
                        page.text_objects[*idx] = txt.clone();
                    }
                }
                self.active_page_idx = *page_index;
            }
            Done::DeleteSelection {
                page_index,
                removed_strokes,
                removed_latex,
                removed_shapes,
                removed_texts,
            } => {
                let page = &mut self.pages[*page_index];
                for (idx, s) in removed_strokes {
                    let at = (*idx).min(page.strokes.len());
                    page.strokes.insert(at, s.clone());
                }
                for (idx, l) in removed_latex {
                    let at = (*idx).min(page.latex_objects.len());
                    page.latex_objects.insert(at, l.clone());
                }
                for (idx, sh) in removed_shapes {
                    let at = (*idx).min(page.shapes.len());
                    page.shapes.insert(at, sh.clone());
                }
                for (idx, txt) in removed_texts {
                    let at = (*idx).min(page.text_objects.len());
                    page.text_objects.insert(at, txt.clone());
                }
                self.active_page_idx = *page_index;
            }
        }
        self.redo_stack.push(done);
        self.rev += 1;
        true
    }

    /// Làm lại 1 bước. Trả về `true` nếu có thay đổi.
    pub fn redo(&mut self) -> bool {
        let Some(done) = self.redo_stack.pop() else {
            return false;
        };
        match &done {
            Done::AddStroke { page_index, stroke } => {
                self.pages[*page_index].strokes.push(stroke.clone());
                self.active_page_idx = *page_index;
            }
            Done::AddStrokes {
                page_index,
                strokes,
            } => {
                self.pages[*page_index]
                    .strokes
                    .extend(strokes.iter().cloned());
                self.active_page_idx = *page_index;
            }
            Done::ClearPage { page_index, .. } => {
                self.pages[*page_index].strokes.clear();
                self.active_page_idx = *page_index;
            }
            Done::InsertPage { index, page, .. } => {
                self.pages.insert(*index, page.clone());
                self.active_page_idx = *index;
            }
            Done::MovePage { from, to, .. } => {
                let page = self.pages.remove(*from);
                self.pages.insert(*to, page);
                self.active_page_idx = *to;
            }
            Done::DeletePage { index, .. } => {
                self.pages.remove(*index);
                self.active_page_idx = (*index).min(self.pages.len().saturating_sub(1));
            }
            Done::AddLatex { page_index, obj } => {
                self.pages[*page_index].latex_objects.push(obj.clone());
                self.active_page_idx = *page_index;
            }
            Done::AddShape { page_index, obj } => {
                self.pages[*page_index].shapes.push(obj.clone());
                self.active_page_idx = *page_index;
            }
            Done::AddText { page_index, obj } => {
                self.pages[*page_index].text_objects.push(obj.clone());
                self.active_page_idx = *page_index;
            }
            Done::UpdateText {
                page_index,
                new_obj,
                ..
            } => {
                if let Some(obj) = self.pages[*page_index]
                    .text_objects
                    .iter_mut()
                    .find(|o| o.id == new_obj.id)
                {
                    *obj = new_obj.clone();
                }
                self.active_page_idx = *page_index;
            }
            Done::UpdateLatex {
                page_index,
                obj_id,
                new_source,
                new_size,
                new_svg_cache,
                ..
            } => {
                if let Some(obj) = self.pages[*page_index]
                    .latex_objects
                    .iter_mut()
                    .find(|o| o.id == *obj_id)
                {
                    obj.source = new_source.clone();
                    obj.size = *new_size;
                    obj.svg_cache = new_svg_cache.clone();
                }
                self.active_page_idx = *page_index;
            }
            Done::EraseStrokes {
                page_index,
                removed,
                added,
            } => {
                let strokes = &mut self.pages[*page_index].strokes;
                for &(idx, _) in removed.iter().rev() {
                    if idx < strokes.len() {
                        strokes.remove(idx);
                    }
                }
                strokes.extend(added.clone());
                self.active_page_idx = *page_index;
            }
            Done::TransformSelection {
                page_index,
                new_strokes,
                new_latex,
                new_shapes,
                new_texts,
                ..
            } => {
                let page = &mut self.pages[*page_index];
                for (idx, s) in new_strokes {
                    if *idx < page.strokes.len() {
                        page.strokes[*idx] = s.clone();
                    }
                }
                for (idx, l) in new_latex {
                    if *idx < page.latex_objects.len() {
                        page.latex_objects[*idx] = l.clone();
                    }
                }
                for (idx, sh) in new_shapes {
                    if *idx < page.shapes.len() {
                        page.shapes[*idx] = sh.clone();
                    }
                }
                for (idx, txt) in new_texts {
                    if *idx < page.text_objects.len() {
                        page.text_objects[*idx] = txt.clone();
                    }
                }
                self.active_page_idx = *page_index;
            }
            Done::DeleteSelection {
                page_index,
                removed_strokes,
                removed_latex,
                removed_shapes,
                removed_texts,
            } => {
                let page = &mut self.pages[*page_index];
                for &(idx, _) in removed_strokes.iter().rev() {
                    if idx < page.strokes.len() {
                        page.strokes.remove(idx);
                    }
                }
                for &(idx, _) in removed_latex.iter().rev() {
                    if idx < page.latex_objects.len() {
                        page.latex_objects.remove(idx);
                    }
                }
                for &(idx, _) in removed_shapes.iter().rev() {
                    if idx < page.shapes.len() {
                        page.shapes.remove(idx);
                    }
                }
                for &(idx, _) in removed_texts.iter().rev() {
                    if idx < page.text_objects.len() {
                        page.text_objects.remove(idx);
                    }
                }
                self.active_page_idx = *page_index;
            }
        }
        self.undo_stack.push(done);
        self.rev += 1;
        true
    }

    /// Xuất dữ liệu sang cấu trúc `DocumentFile` để lưu đĩa.
    pub fn to_file(&self) -> DocumentFile {
        let pages = self
            .pages
            .iter()
            .map(|p| {
                let mut objects: Vec<PageObject> = p
                    .strokes
                    .iter()
                    .cloned()
                    .map(PageObject::BrushStroke)
                    .collect();
                objects.extend(p.latex_objects.iter().cloned().map(PageObject::Latex));
                objects.extend(p.shapes.iter().cloned().map(PageObject::Shape));
                objects.extend(p.text_objects.iter().cloned().map(PageObject::Text));
                Page { id: p.id, objects }
            })
            .collect();
        DocumentFile {
            format_version: FORMAT_VERSION,
            metadata: self.metadata.clone(),
            paper_settings: self.paper_settings.clone(),
            pages,
            first_page_thumbnail: None,
        }
    }

    /// Nạp dữ liệu từ `DocumentFile` đã đọc từ đĩa.
    pub fn from_file(file: DocumentFile) -> Self {
        let mut pages = Vec::with_capacity(file.pages.len().max(1));
        let mut max_id = 0u64;
        let mut max_page_id = 0u64;
        for page in file.pages {
            if page.id > max_page_id {
                max_page_id = page.id;
            }
            let mut strokes = Vec::new();
            let mut latex_objects = Vec::new();
            let mut shapes = Vec::new();
            let mut text_objects = Vec::new();
            for obj in page.objects {
                match obj {
                    PageObject::BrushStroke(s) => {
                        if s.id > max_id {
                            max_id = s.id;
                        }
                        strokes.push(s);
                    }
                    PageObject::Latex(l) => {
                        if l.id > max_id {
                            max_id = l.id;
                        }
                        latex_objects.push(l);
                    }
                    PageObject::Shape(sh) => {
                        if sh.id > max_id {
                            max_id = sh.id;
                        }
                        shapes.push(sh);
                    }
                    PageObject::Text(txt) => {
                        if txt.id > max_id {
                            max_id = txt.id;
                        }
                        text_objects.push(txt);
                    }
                }
            }
            pages.push(DocumentPage {
                id: page.id,
                strokes,
                latex_objects,
                shapes,
                text_objects,
            });
        }
        if pages.is_empty() {
            max_page_id = max_page_id.max(1);
            pages.push(DocumentPage {
                id: 1,
                strokes: Vec::new(),
                latex_objects: Vec::new(),
                shapes: Vec::new(),
                text_objects: Vec::new(),
            });
        }
        Self {
            metadata: file.metadata,
            paper_settings: file.paper_settings,
            pages,
            active_page_idx: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            next_id: max_id,
            next_page_id: max_page_id,
            rev: 1,
            saved_rev: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inknote_pen::penpath::Element;
    use parry2d_f64::math::Vector2;

    fn stroke(doc: &mut Document) -> Stroke {
        let id = doc.alloc_id();
        let path = PenPath::new(Element::new(Vector2::new(0.0, 0.0), 0.5));
        Stroke {
            id,
            path,
            style: Style::default(),
        }
    }

    fn ids(doc: &Document) -> Vec<u64> {
        doc.strokes().iter().map(|s| s.id).collect()
    }

    #[test]
    fn three_strokes_undo_then_redo_in_order() {
        let mut d = Document::new();
        for _ in 0..3 {
            let s = stroke(&mut d);
            d.execute(Command::AddStroke(s));
        }
        assert_eq!(ids(&d), vec![1, 2, 3]);
        for _ in 0..3 {
            assert!(d.undo());
        }
        assert!(ids(&d).is_empty());
        assert!(!d.undo());
        for _ in 0..3 {
            assert!(d.redo());
        }
        assert_eq!(ids(&d), vec![1, 2, 3]);
        assert!(!d.redo());
    }

    #[test]
    fn new_command_clears_redo() {
        let mut d = Document::new();
        let s = stroke(&mut d);
        d.execute(Command::AddStroke(s));
        d.undo();
        assert!(d.can_redo());
        let s = stroke(&mut d);
        d.execute(Command::AddStroke(s));
        assert!(!d.can_redo());
    }

    #[test]
    fn clear_page_is_one_undo_step() {
        let mut d = Document::new();
        for _ in 0..3 {
            let s = stroke(&mut d);
            d.execute(Command::AddStroke(s));
        }
        d.execute(Command::ClearPage);
        assert!(d.strokes().is_empty());
        d.undo();
        assert_eq!(ids(&d), vec![1, 2, 3]);
        d.redo();
        assert!(d.strokes().is_empty());
    }

    #[test]
    fn add_many_strokes_is_one_undo_step() {
        let mut d = Document::new();
        let s = stroke(&mut d);
        d.execute(Command::AddStroke(s));
        let batch: Vec<Stroke> = (0..5).map(|_| stroke(&mut d)).collect();
        d.execute(Command::AddStrokes(batch));
        assert_eq!(d.strokes().len(), 6);
        d.undo();
        assert_eq!(ids(&d), vec![1]);
        d.redo();
        assert_eq!(d.strokes().len(), 6);
    }

    #[test]
    fn clear_empty_page_makes_no_step() {
        let mut d = Document::new();
        d.execute(Command::ClearPage);
        assert!(!d.can_undo());
    }

    #[test]
    fn document_save_and_load_roundtrip() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_roundtrip.inknote");

        let mut d = Document::new();
        d.metadata_mut().title = "My Test Note".to_string();
        d.paper_settings_mut().pattern = PaperPattern::Grid;
        for _ in 0..5 {
            let s = stroke(&mut d);
            d.execute(Command::AddStroke(s));
        }
        assert!(d.is_dirty());

        let doc_file = d.to_file();
        assert_eq!(doc_file.format_version, FORMAT_VERSION);
        assert_eq!(doc_file.metadata.title, "My Test Note");
        assert_eq!(doc_file.pages[0].objects.len(), 5);

        doc_file.save_to_path(&file_path).expect("save failed");
        d.mark_saved();
        assert!(!d.is_dirty());

        let loaded_file = DocumentFile::load_from_path(&file_path).expect("load failed");
        assert_eq!(loaded_file.format_version, FORMAT_VERSION);
        assert_eq!(loaded_file.metadata.title, "My Test Note");
        assert_eq!(loaded_file.paper_settings.pattern, PaperPattern::Grid);

        let mut loaded_doc = Document::from_file(loaded_file);
        assert_eq!(loaded_doc.strokes().len(), 5);
        assert!(!loaded_doc.is_dirty());
        assert_eq!(loaded_doc.alloc_id(), 6);

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn rejects_newer_format_version() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_newer_version.inknote");

        let mut file = Document::new().to_file();
        file.format_version = 999;
        file.save_to_path(&file_path).unwrap();

        let res = DocumentFile::load_from_path(&file_path);
        assert!(res.is_err());
        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn paper_settings_change_marks_dirty() {
        let mut d = Document::new();
        assert!(!d.is_dirty());
        let mut ps = d.paper_settings().clone();
        assert!(!d.set_paper_settings(ps.clone()));
        assert!(!d.is_dirty());

        ps.pattern = PaperPattern::Lined;
        assert!(d.set_paper_settings(ps));
        assert!(d.is_dirty());
    }

    #[test]
    fn mark_saved_at_preserves_dirty_when_newer_revision_exists() {
        let mut d = Document::new();
        let s1 = stroke(&mut d);
        d.execute(Command::AddStroke(s1));
        let rev1 = d.revision();

        let s2 = stroke(&mut d);
        d.execute(Command::AddStroke(s2));
        let rev2 = d.revision();

        d.mark_saved_at(rev1);
        assert!(d.is_dirty());

        d.mark_saved_at(rev2);
        assert!(!d.is_dirty());
    }

    #[test]
    fn save_to_path_atomic_overwrite_leaves_no_tmp_file() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_atomic_overwrite.inknote");
        let tmp_path = temp_dir.join(".test_atomic_overwrite.inknote.tmp");
        let _ = std::fs::remove_file(&file_path);
        let _ = std::fs::remove_file(&tmp_path);

        let mut d = Document::new();
        let s1 = stroke(&mut d);
        d.execute(Command::AddStroke(s1));
        d.to_file().save_to_path(&file_path).unwrap();
        assert!(file_path.exists());
        assert!(!tmp_path.exists());

        let s2 = stroke(&mut d);
        d.execute(Command::AddStroke(s2));
        d.to_file().save_to_path(&file_path).unwrap();
        assert!(file_path.exists());
        assert!(!tmp_path.exists());

        let loaded = DocumentFile::load_from_path(&file_path).unwrap();
        assert_eq!(loaded.pages[0].objects.len(), 2);

        let _ = std::fs::remove_file(&file_path);
    }

    #[test]
    fn insert_page_after_active_and_undo_redo() {
        let mut d = Document::new();
        assert_eq!(d.page_count(), 1);
        assert_eq!(d.active_page_index(), 0);

        let s1 = stroke(&mut d);
        d.execute(Command::AddStroke(s1));

        // Thêm trang sau trang 0 -> trang mới là index 1
        d.execute(Command::InsertPageAfter(0));
        assert_eq!(d.page_count(), 2);
        assert_eq!(d.active_page_index(), 1);

        let s2 = stroke(&mut d);
        d.execute(Command::AddStroke(s2));

        // Quay lại trang 0, chèn thêm trang mới -> nằm giữa tại index 1
        d.set_active_page_index(0);
        d.execute(Command::InsertPageAfter(0));
        assert_eq!(d.page_count(), 3);
        assert_eq!(d.active_page_index(), 1);
        assert!(d.page_strokes(0).len() == 1);
        assert!(d.page_strokes(1).is_empty());
        assert!(d.page_strokes(2).len() == 1);

        // Undo việc chèn ở giữa -> còn 2 trang, quay về trang 0
        assert!(d.undo());
        assert_eq!(d.page_count(), 2);
        assert_eq!(d.active_page_index(), 0);
        assert_eq!(d.page_strokes(1).len(), 1);

        // Redo lại -> lại có 3 trang
        assert!(d.redo());
        assert_eq!(d.page_count(), 3);
        assert_eq!(d.active_page_index(), 1);
    }

    #[test]
    fn delete_page_with_undo_restores_strokes_and_keeps_last_page() {
        let mut d = Document::new();
        // Không thể xoá khi chỉ còn 1 trang
        d.execute(Command::DeletePage(0));
        assert_eq!(d.page_count(), 1);
        assert!(!d.can_undo());

        let s1 = stroke(&mut d);
        d.execute(Command::AddStroke(s1));
        d.execute(Command::InsertPageAfter(0));
        let s2 = stroke(&mut d);
        let s3 = stroke(&mut d);
        d.execute(Command::AddStroke(s2));
        d.execute(Command::AddStroke(s3));

        // Xoá trang 1 (có 2 nét)
        d.execute(Command::DeletePage(1));
        assert_eq!(d.page_count(), 1);
        assert_eq!(d.active_page_index(), 0);

        // Undo khôi phục đúng trang 1 và cả 2 nét của nó
        assert!(d.undo());
        assert_eq!(d.page_count(), 2);
        assert_eq!(d.active_page_index(), 1);
        assert_eq!(d.page_strokes(1).len(), 2);

        // Redo xoá lại
        assert!(d.redo());
        assert_eq!(d.page_count(), 1);
    }

    #[test]
    fn multi_page_save_and_load_roundtrip() {
        let temp_file = std::env::temp_dir().join("test_multipage_roundtrip.inknote");
        let _ = std::fs::remove_file(&temp_file);

        let mut d = Document::new();
        let s0 = stroke(&mut d);
        d.execute(Command::AddStroke(s0));

        for page_idx in 0..4 {
            d.execute(Command::InsertPageAfter(page_idx));
            let s = stroke(&mut d);
            d.execute(Command::AddStroke(s));
        }
        assert_eq!(d.page_count(), 5);

        d.to_file().save_to_path(&temp_file).unwrap();
        let loaded = Document::from_file(DocumentFile::load_from_path(&temp_file).unwrap());
        assert_eq!(loaded.page_count(), 5);
        for i in 0..5 {
            assert_eq!(loaded.page_strokes(i).len(), 1);
        }

        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn insert_page_before_duplicate_and_clear_page_at_undo_redo() {
        let mut d = Document::new();
        let s1 = stroke(&mut d);
        d.execute(Command::AddStroke(s1));

        // Nhân bản trang 0 -> trang 1 có 1 nét mới (id khác)
        d.execute(Command::DuplicatePage(0));
        assert_eq!(d.page_count(), 2);
        assert_eq!(d.active_page_index(), 1);
        assert_eq!(d.page_strokes(1).len(), 1);
        assert_ne!(d.page_strokes(0)[0].id, d.page_strokes(1)[0].id);

        // Chèn trang mới trước trang 1 -> trang trống ở index 1, trang nhân bản dời sang index 2
        d.execute(Command::InsertPageBefore(1));
        assert_eq!(d.page_count(), 3);
        assert_eq!(d.active_page_index(), 1);
        assert_eq!(d.page_strokes(1).len(), 0);
        assert_eq!(d.page_strokes(2).len(), 1);

        // Clear trang index 2 rồi Undo
        d.execute(Command::ClearPageAt(2));
        assert_eq!(d.page_strokes(2).len(), 0);
        assert!(d.undo());
        assert_eq!(d.page_strokes(2).len(), 1);

        // Di chuyển trang index 2 lên đầu (index 0) rồi Undo/Redo
        let moved_id = d.pages()[2].id;
        d.execute(Command::MovePage { from: 2, to: 0 });
        assert_eq!(d.pages()[0].id, moved_id);
        assert_eq!(d.active_page_index(), 0);
        assert!(d.undo());
        assert_eq!(d.pages()[2].id, moved_id);
        assert!(d.redo());
        assert_eq!(d.pages()[0].id, moved_id);
    }

    #[test]
    fn add_latex_is_one_undo_step() {
        let mut d = Document::new();
        let id = d.alloc_id();
        let obj = LatexObject {
            id,
            source: r"x^2 + y^2 = r^2".to_string(),
            pos: [120.0, 240.0],
            size: [80.0, 24.0],
            svg_cache: Some("<svg></svg>".to_string()),
        };
        d.execute(Command::AddLatex {
            page_index: 0,
            obj: obj.clone(),
        });
        assert_eq!(d.pages()[0].latex_objects.len(), 1);
        assert_eq!(d.pages()[0].latex_objects[0].source, obj.source);

        assert!(d.undo());
        assert_eq!(d.pages()[0].latex_objects.len(), 0);

        assert!(d.redo());
        assert_eq!(d.pages()[0].latex_objects.len(), 1);
        assert_eq!(d.pages()[0].latex_objects[0].pos, [120.0, 240.0]);
    }

    #[test]
    fn update_latex_preserves_pos_updates_size_and_is_one_undo_step() {
        let mut d = Document::new();
        let id = d.alloc_id();
        d.execute(Command::AddLatex {
            page_index: 0,
            obj: LatexObject {
                id,
                source: "a + b".to_string(),
                pos: [150.0, 260.0],
                size: [90.0, 30.0],
                svg_cache: Some("<svg>old</svg>".to_string()),
            },
        });

        d.execute(Command::UpdateLatex {
            page_index: 0,
            obj_id: id,
            new_source: "a^2 + b^2 + c^2".to_string(),
            new_size: [180.0, 36.0],
            new_svg_cache: Some("<svg>new</svg>".to_string()),
        });
        let updated = &d.pages()[0].latex_objects[0];
        assert_eq!(updated.source, "a^2 + b^2 + c^2");
        assert_eq!(updated.pos, [150.0, 260.0]);
        assert_eq!(updated.size, [180.0, 36.0]);

        assert!(d.undo());
        let reverted = &d.pages()[0].latex_objects[0];
        assert_eq!(reverted.source, "a + b");
        assert_eq!(reverted.svg_cache.as_deref(), Some("<svg>old</svg>"));
        assert_eq!(reverted.pos, [150.0, 260.0]);
        assert_eq!(reverted.size, [90.0, 30.0]);

        assert!(d.redo());
        let redone = &d.pages()[0].latex_objects[0];
        assert_eq!(redone.source, "a^2 + b^2 + c^2");
        assert_eq!(redone.size, [180.0, 36.0]);
    }

    #[test]
    fn latex_save_and_load_roundtrip() {
        let temp_file = std::env::temp_dir().join("test_latex_roundtrip.inknote");
        let _ = std::fs::remove_file(&temp_file);

        let mut d = Document::new();
        let id = d.alloc_id();
        let obj = LatexObject {
            id,
            source: r"\int_0^1 x^2 \, dx = \frac{1}{3}".to_string(),
            pos: [210.0, 340.0],
            size: [195.0, 48.0],
            svg_cache: Some(r#"<svg width="100" height="30"></svg>"#.to_string()),
        };
        d.execute(Command::AddLatex {
            page_index: 0,
            obj: obj.clone(),
        });

        d.to_file().save_to_path(&temp_file).unwrap();
        let mut loaded = Document::from_file(DocumentFile::load_from_path(&temp_file).unwrap());
        assert_eq!(loaded.pages()[0].latex_objects.len(), 1);
        let loaded_obj = &loaded.pages()[0].latex_objects[0];
        assert_eq!(loaded_obj.id, id);
        assert_eq!(loaded_obj.source, obj.source);
        assert_eq!(loaded_obj.pos, obj.pos);
        assert_eq!(loaded_obj.size, obj.size);
        assert_eq!(loaded_obj.svg_cache, obj.svg_cache);

        // ID mới cấp sau khi nạp không được trùng với id của LatexObject đã lưu
        let next = loaded.alloc_id();
        assert!(next > id);

        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn erase_strokes_is_one_undo_step_and_never_touches_latex() {
        let mut d = Document::new();
        let s1 = stroke(&mut d);
        let s2 = stroke(&mut d);
        let s3 = stroke(&mut d);
        d.execute(Command::AddStrokes(vec![
            s1.clone(),
            s2.clone(),
            s3.clone(),
        ]));

        let latex_id = d.alloc_id();
        d.execute(Command::AddLatex {
            page_index: 0,
            obj: LatexObject {
                id: latex_id,
                source: "E = mc^2".to_string(),
                pos: [100.0, 100.0],
                size: [80.0, 30.0],
                svg_cache: None,
            },
        });

        let split_piece = stroke(&mut d);
        d.execute(Command::EraseStrokes {
            page_index: 0,
            removed: vec![(0, s1.clone()), (2, s3.clone())],
            added: vec![split_piece.clone()],
        });

        assert_eq!(d.page_strokes(0).len(), 2);
        assert_eq!(d.page_strokes(0)[0].id, s2.id);
        assert_eq!(d.page_strokes(0)[1].id, split_piece.id);
        assert_eq!(d.pages()[0].latex_objects.len(), 1);
        assert_eq!(d.pages()[0].latex_objects[0].id, latex_id);

        assert!(d.undo());
        assert_eq!(d.page_strokes(0).len(), 3);
        assert_eq!(d.page_strokes(0)[0].id, s1.id);
        assert_eq!(d.page_strokes(0)[1].id, s2.id);
        assert_eq!(d.page_strokes(0)[2].id, s3.id);

        assert!(d.redo());
        assert_eq!(d.page_strokes(0).len(), 2);
        assert_eq!(d.pages()[0].latex_objects.len(), 1);
    }

    #[test]
    fn lasso_transform_and_delete_are_one_undo_step() {
        let mut d = Document::new();
        let s1 = stroke(&mut d);
        let s1_id = s1.id;
        let orig_x = s1.path.start.pos.x;
        d.execute(Command::AddStroke(s1));

        let l_id = d.alloc_id();
        d.execute(Command::AddLatex {
            page_index: 0,
            obj: LatexObject {
                id: l_id,
                source: "x^2".to_string(),
                pos: [100.0, 100.0],
                size: [40.0, 20.0],
                svg_cache: None,
            },
        });

        // Di chuyển + scale đồng thời cả nét bút và LaTeX = 1 bước Undo
        d.execute(Command::TransformSelection {
            page_index: 0,
            stroke_ids: vec![s1_id],
            latex_ids: vec![l_id],
            shape_ids: vec![],
            text_ids: vec![],
            anchor: [0.0, 0.0],
            scale: [2.0, 2.0],
            delta: [50.0, 30.0],
        });
        assert!((d.page_strokes(0)[0].path.start.pos.x - (orig_x * 2.0 + 50.0)).abs() < 1e-6);
        assert_eq!(d.pages()[0].latex_objects[0].pos, [250.0, 230.0]);
        assert_eq!(d.pages()[0].latex_objects[0].size, [80.0, 40.0]);

        assert!(d.undo());
        assert!((d.page_strokes(0)[0].path.start.pos.x - orig_x).abs() < 1e-6);
        assert_eq!(d.pages()[0].latex_objects[0].pos, [100.0, 100.0]);
        assert_eq!(d.pages()[0].latex_objects[0].size, [40.0, 20.0]);

        assert!(d.redo());
        assert_eq!(d.pages()[0].latex_objects[0].pos, [250.0, 230.0]);

        // Xoá vùng chọn (cả nét bút + LaTeX) = 1 bước Undo
        d.execute(Command::DeleteSelection {
            page_index: 0,
            stroke_ids: vec![s1_id],
            latex_ids: vec![l_id],
            shape_ids: vec![],
            text_ids: vec![],
        });
        assert_eq!(d.page_strokes(0).len(), 0);
        assert_eq!(d.pages()[0].latex_objects.len(), 0);

        assert!(d.undo());
        assert_eq!(d.page_strokes(0).len(), 1);
        assert_eq!(d.pages()[0].latex_objects.len(), 1);
    }

    #[test]
    fn shape_add_undo_redo_transform_delete_and_file_roundtrip() {
        let mut d = Document::new();
        let sh_id = d.alloc_id();
        let sh = ShapeObject {
            id: sh_id,
            kind: ShapeKind::Arrow,
            start: [40.0, 80.0],
            end: [200.0, 80.0],
            color: [25, 103, 210, 255],
            stroke_width: 3.0,
            dashed: true,
        };
        d.execute(Command::AddShape {
            page_index: 0,
            obj: sh.clone(),
        });
        assert_eq!(d.pages()[0].shapes.len(), 1);
        assert_eq!(d.pages()[0].shapes[0], sh);

        assert!(d.undo());
        assert!(d.pages()[0].shapes.is_empty());
        assert!(d.redo());
        assert_eq!(d.pages()[0].shapes.len(), 1);

        // Chuyển đổi + xoá qua Lasso (TransformSelection / DeleteSelection)
        d.execute(Command::TransformSelection {
            page_index: 0,
            stroke_ids: vec![],
            latex_ids: vec![],
            shape_ids: vec![sh_id],
            text_ids: vec![],
            anchor: [0.0, 0.0],
            scale: [1.5, 1.0],
            delta: [10.0, 20.0],
        });
        assert_eq!(d.pages()[0].shapes[0].start, [70.0, 100.0]);
        assert_eq!(d.pages()[0].shapes[0].end, [310.0, 100.0]);

        assert!(d.undo());
        assert_eq!(d.pages()[0].shapes[0], sh);
        assert!(d.redo());

        // Kiểm tra lưu & mở lại file vẫn giữ nguyên ShapeObject
        let file = d.to_file();
        let json = serde_json::to_string(&file).unwrap();
        let parsed: DocumentFile = serde_json::from_str(&json).unwrap();
        let loaded = Document::from_file(parsed);
        assert_eq!(loaded.pages()[0].shapes.len(), 1);
        assert_eq!(loaded.pages()[0].shapes[0].kind, ShapeKind::Arrow);
        assert!(loaded.pages()[0].shapes[0].dashed);

        // Eraser (EraseStrokes) không bao giờ chạm tới shapes
        let s = stroke(&mut d);
        d.execute(Command::AddStroke(s));
        let idx = 0;
        let rem = d.pages()[0].strokes[idx].clone();
        d.execute(Command::EraseStrokes {
            page_index: 0,
            removed: vec![(idx, rem)],
            added: vec![],
        });
        assert!(d.pages()[0].strokes.is_empty());
        assert_eq!(d.pages()[0].shapes.len(), 1);

        // DeleteSelection xoá hình và undo khôi phục lại
        d.execute(Command::DeleteSelection {
            page_index: 0,
            stroke_ids: vec![],
            latex_ids: vec![],
            shape_ids: vec![sh_id],
            text_ids: vec![],
        });
        assert!(d.pages()[0].shapes.is_empty());
        assert!(d.undo());
        assert_eq!(d.pages()[0].shapes.len(), 1);
    }

    #[test]
    fn text_add_update_undo_redo_transform_delete_and_file_roundtrip() {
        let mut d = Document::new();
        let txt_id = d.alloc_id();
        let original = TextObject {
            id: txt_id,
            pos: [60.0, 90.0],
            size: [260.0, 48.0],
            text: "Tiếng Việt, ắ ằ ẳ ẵ ặ ơ ư".to_string(),
            font_family: TextFontFamily::Sans,
            font_size: 16.0,
            bold: true,
            italic: false,
            align: TextAlign::Left,
            list_mode: TextListMode::None,
            color: [20, 22, 25, 255],
        };
        d.execute(Command::AddText {
            page_index: 0,
            obj: original.clone(),
        });
        assert_eq!(d.pages()[0].text_objects.len(), 1);
        assert_eq!(d.pages()[0].text_objects[0], original);

        assert!(d.undo());
        assert!(d.pages()[0].text_objects.is_empty());
        assert!(d.redo());
        assert_eq!(d.pages()[0].text_objects[0], original);

        // Sửa nội dung & kiểu chữ trong 1 bước Undo
        let mut updated = original.clone();
        updated.text = "• Tiếng Việt, ắ ằ ẳ ẵ ặ ơ ư\n• Dòng thứ hai".to_string();
        updated.font_family = TextFontFamily::Serif;
        updated.font_size = 20.0;
        updated.italic = true;
        updated.align = TextAlign::Center;
        updated.list_mode = TextListMode::Bullet;
        updated.color = [25, 103, 210, 255];
        d.execute(Command::UpdateText {
            page_index: 0,
            new_obj: updated.clone(),
        });
        assert_eq!(d.pages()[0].text_objects[0], updated);
        assert!(d.undo());
        assert_eq!(d.pages()[0].text_objects[0], original);
        assert!(d.redo());
        assert_eq!(d.pages()[0].text_objects[0], updated);

        // Lasso transform (scale cả chiều rộng và cỡ chữ) + Undo
        d.execute(Command::TransformSelection {
            page_index: 0,
            stroke_ids: vec![],
            latex_ids: vec![],
            shape_ids: vec![],
            text_ids: vec![txt_id],
            anchor: [0.0, 0.0],
            scale: [1.5, 1.25],
            delta: [10.0, 20.0],
        });
        assert_eq!(d.pages()[0].text_objects[0].pos, [100.0, 132.5]);
        assert_eq!(d.pages()[0].text_objects[0].size, [390.0, 60.0]);
        assert!((d.pages()[0].text_objects[0].font_size - 25.0).abs() < 1e-6);
        assert!(d.undo());
        assert_eq!(d.pages()[0].text_objects[0], updated);
        assert!(d.redo());

        // Lưu & mở lại .inknote giữ nguyên tiếng Việt + định dạng
        let file = d.to_file();
        let json = serde_json::to_string(&file).unwrap();
        let parsed: DocumentFile = serde_json::from_str(&json).unwrap();
        let loaded = Document::from_file(parsed);
        assert_eq!(loaded.pages()[0].text_objects.len(), 1);
        assert_eq!(
            loaded.pages()[0].text_objects[0],
            d.pages()[0].text_objects[0]
        );

        // Tẩy (EraseStrokes) không bao giờ chạm tới text_objects
        let s = stroke(&mut d);
        d.execute(Command::AddStroke(s));
        let rem = d.pages()[0].strokes[0].clone();
        d.execute(Command::EraseStrokes {
            page_index: 0,
            removed: vec![(0, rem)],
            added: vec![],
        });
        assert!(d.pages()[0].strokes.is_empty());
        assert_eq!(d.pages()[0].text_objects.len(), 1);

        // DeleteSelection xoá hộp văn bản và undo khôi phục lại
        d.execute(Command::DeleteSelection {
            page_index: 0,
            stroke_ids: vec![],
            latex_ids: vec![],
            shape_ids: vec![],
            text_ids: vec![txt_id],
        });
        assert!(d.pages()[0].text_objects.is_empty());
        assert!(d.undo());
        assert_eq!(d.pages()[0].text_objects.len(), 1);
    }
}
