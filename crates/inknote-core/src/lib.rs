// InkNote core (Bước 1.7 / 2.1): dữ liệu trang, định dạng file .inknote,
// hệ thống lệnh (Command) và Undo/Redo.
// Mọi thay đổi nội dung trang PHẢI đi qua `Document::execute` để có thể hoàn tác.

use inknote_pen::{PenPath, Style};
use serde::{Deserialize, Serialize};
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

/// Các loại đối tượng trên một trang (khớp docs/02-architecture.md mục 3).
/// Chỉ BrushStroke mới bị Eraser xoá; các loại khác không bị tẩy xoá lướt qua.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PageObject {
    BrushStroke(Stroke),
    // Dự phòng mở rộng cho các bước tiếp theo (P2):
    // Shape(ShapeObject),
    // Text(TextObject),
    // Image(ImageObject),
    // Latex(LatexObject),
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

/// Lệnh thay đổi nội dung trang.
pub enum Command {
    /// Thêm một nét vào cuối trang.
    AddStroke(Stroke),
    /// Thêm nhiều nét cùng lúc (1 bước undo).
    AddStrokes(Vec<Stroke>),
    /// Xoá toàn bộ nét trên trang.
    ClearPage,
}

/// Lệnh đã thực thi, kèm dữ liệu cần để hoàn tác.
enum Done {
    AddStroke(Stroke),
    AddStrokes(Vec<Stroke>),
    ClearPage(Vec<Stroke>),
}

/// Trang + lịch sử Undo/Redo + trạng thái lưu file.
#[derive(Default)]
pub struct Document {
    metadata: DocumentMetadata,
    paper_settings: PaperSettings,
    strokes: Vec<Stroke>,
    undo_stack: Vec<Done>,
    redo_stack: Vec<Done>,
    next_id: u64,
    rev: u64,
    saved_rev: u64,
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

    pub fn strokes(&self) -> &[Stroke] {
        &self.strokes
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
        let done = match cmd {
            Command::AddStroke(s) => {
                self.strokes.push(s.clone());
                Done::AddStroke(s)
            }
            Command::AddStrokes(list) => {
                if list.is_empty() {
                    return;
                }
                self.strokes.extend(list.iter().cloned());
                Done::AddStrokes(list)
            }
            Command::ClearPage => {
                if self.strokes.is_empty() {
                    return; // không có gì để xoá → không tạo bước undo rỗng
                }
                Done::ClearPage(std::mem::take(&mut self.strokes))
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
            Done::AddStroke(_) => {
                self.strokes.pop();
            }
            Done::AddStrokes(list) => {
                let keep = self.strokes.len().saturating_sub(list.len());
                self.strokes.truncate(keep);
            }
            Done::ClearPage(removed) => self.strokes = removed.clone(),
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
            Done::AddStroke(s) => self.strokes.push(s.clone()),
            Done::AddStrokes(list) => self.strokes.extend(list.iter().cloned()),
            Done::ClearPage(_) => self.strokes.clear(),
        }
        self.undo_stack.push(done);
        self.rev += 1;
        true
    }

    /// Xuất dữ liệu sang cấu trúc `DocumentFile` để lưu đĩa.
    pub fn to_file(&self) -> DocumentFile {
        let page_objects = self
            .strokes
            .iter()
            .cloned()
            .map(PageObject::BrushStroke)
            .collect();
        DocumentFile {
            format_version: FORMAT_VERSION,
            metadata: self.metadata.clone(),
            paper_settings: self.paper_settings.clone(),
            pages: vec![Page {
                id: 1,
                objects: page_objects,
            }],
            first_page_thumbnail: None,
        }
    }

    /// Nạp dữ liệu từ `DocumentFile` đã đọc từ đĩa.
    pub fn from_file(file: DocumentFile) -> Self {
        let mut strokes = Vec::new();
        let mut max_id = 0u64;
        for page in file.pages {
            for obj in page.objects {
                match obj {
                    PageObject::BrushStroke(s) => {
                        if s.id > max_id {
                            max_id = s.id;
                        }
                        strokes.push(s);
                    }
                }
            }
        }
        Self {
            metadata: file.metadata,
            paper_settings: file.paper_settings,
            strokes,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            next_id: max_id,
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
}
