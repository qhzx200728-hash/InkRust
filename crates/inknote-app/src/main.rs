// Ứng dụng InkNote (Bước 1.6 – Khung giao diện P0 theo docs/04-ui-spec.md):
// - Khu vực 1: Thanh công cụ chính (Navigation Toolbar, nền xanh dương, nút đang chọn có ô nền sáng bo góc)
// - Khu vực 2: Thanh tuỳ chọn công cụ (Contextual Tool Options Bar, nền tối bo tròn + Pop-up tối bo góc)
// - Khu vực 3: Canvas chính với nền giấy chấm (Dotted Paper) và nét bút Rnote (`inknote-pen` + `inknote-render`)

mod autosave;
mod icons;
mod theme;
mod thumbnails;
mod toolbar;
mod view;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui;
use inknote_core::{CanvasMode, Command, Document, DocumentFile, PaperSettings, Stroke};
use inknote_pen::{
    brushconfig::BrushConfig,
    builders::{
        buildable::{Buildable, BuilderCreator, BuilderProgress},
        PenPathBuilderType, PenPathCurvedBuilder, PenPathModeledBuilder, PenPathSimpleBuilder,
    },
    penpath::{Element, Segment},
    style::Composer,
    Constraints, PenEvent, PenPath, Style,
};
use inknote_render::{CairoRenderContext, DrawBatch};
use parry2d_f64::math::Vector2;

use crate::{
    autosave::AutoSaveState,
    icons::{draw_paper, paper_sheet_rect},
    theme::Theme,
    thumbnails::{append_batches_to_mesh, show_thumbnail_panel, ThumbCache, ThumbnailAction},
    toolbar::{show_toolbars, ActiveTool, ToolbarState},
    view::View,
};

const PAGE_GAP: f32 = 28.0;
const PULL_TO_ADD_THRESHOLD: f32 = 150.0;

/// Lưới tam giác của 1 nét (toạ độ TRANG) + hộp bao để bỏ qua nét nằm ngoài màn hình.
pub struct CachedStroke {
    pub batches: Vec<DrawBatch>,
    pub bbox: egui::Rect,
}

impl CachedStroke {
    fn new(batches: Vec<DrawBatch>) -> Self {
        let mut bbox = egui::Rect::NOTHING;
        for b in &batches {
            for &[x, y] in &b.vertices {
                bbox.extend_with(egui::pos2(x, y));
            }
        }
        Self { batches, bbox }
    }
}

struct ActiveStroke {
    path: PenPath,
    style: Style,
    builder: Box<dyn Buildable<Emit = Segment>>,
    // Dựng lưới tăng dần khi đang viết: phần đầu nét đã chốt được giữ lại,
    // mỗi khung chỉ dựng lại phần đuôi ngắn (< 16 đoạn) + đầu bút modeler.
    committed_batches: Vec<DrawBatch>,
    committed_segs: usize,
}

/// Khoá bộ nhớ đệm lưới tĩnh: (revision trang, zoom, pan, vùng giấy, số trang, chế độ).
#[derive(Clone, Copy, PartialEq, Eq)]
struct StaticKey {
    rev: u64,
    zoom_bits: u32,
    pan_x_bits: u32,
    pan_y_bits: u32,
    paper_min_x: u32,
    paper_min_y: u32,
    paper_max_x: u32,
    paper_max_y: u32,
    page_count: usize,
    mode_tag: u8,
}

impl StaticKey {
    fn new(
        rev: u64,
        zoom: f32,
        pan: egui::Vec2,
        paper: egui::Rect,
        page_count: usize,
        mode: CanvasMode,
    ) -> Self {
        Self {
            rev,
            zoom_bits: zoom.to_bits(),
            pan_x_bits: pan.x.to_bits(),
            pan_y_bits: pan.y.to_bits(),
            paper_min_x: paper.min.x.to_bits(),
            paper_min_y: paper.min.y.to_bits(),
            paper_max_x: paper.max.x.to_bits(),
            paper_max_y: paper.max.y.to_bits(),
            page_count,
            mode_tag: match mode {
                CanvasMode::Paper => 0,
                CanvasMode::Whiteboard => 1,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnsavedAction {
    NewNote,
    Open,
    CloseApp,
}

fn default_paper_config_path() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata)
            .join("InkNote")
            .join("default_paper.json")
    } else {
        PathBuf::from(".inknote_default_paper.json")
    }
}

fn default_draft_path() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata)
            .join("InkNote")
            .join("autosave_draft.inknote")
    } else {
        PathBuf::from(".inknote_autosave_draft.inknote")
    }
}

fn load_default_paper_settings() -> PaperSettings {
    let path = default_paper_config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(settings) = serde_json::from_str(&content) {
            return settings;
        }
    }
    PaperSettings::default()
}

fn save_default_paper_settings(settings: &PaperSettings) {
    let path = default_paper_config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(path, json);
    }
}

struct InkNoteApp {
    theme: Theme,
    toolbar_state: ToolbarState,
    brush_config: BrushConfig,
    doc: Document,
    // Cài đặt nền giấy mặc định (lưu nhớ giữa các phiên làm việc và áp dụng cho ghi chú mới)
    default_paper_settings: PaperSettings,
    // Đường dẫn file .inknote hiện tại đang mở/lưu (None nếu là ghi chú mới chưa lưu)
    current_path: Option<PathBuf>,
    confirm_unsaved: Option<UnsavedAction>,
    allow_close: bool,
    autosave: AutoSaveState,
    // Bộ nhớ đệm lưới tam giác của từng nét (khoá = id nét), không thuộc dữ liệu trang
    batch_cache: HashMap<u64, CachedStroke>,
    thumb_cache: ThumbCache,
    view: View,
    confirm_clear: bool,
    confirm_delete_page: Option<usize>,
    paint_ms: f32,
    fps: f32,
    last_frame: Instant,
    active: Option<ActiveStroke>,
    drawing_page_index: Option<usize>,
    static_cache: Option<(StaticKey, Vec<(usize, std::sync::Arc<egui::epaint::Mesh>)>)>,
    render_cx: CairoRenderContext,

    // Kéo quá đáy trang cuối để tự thêm trang mới
    pull_overscroll: f32,
    last_pull_at: Instant,
    pull_cooldown_until: Instant,

    // Quản lý thời gian sự kiện cho ink-stroke-modeler-rs
    base_instant: Instant,
    prev_frame_t: f64,
    last_event_secs: f64,

    // Trạng thái bút cảm ứng / chuột
    touch_active: bool,
    stylus_has_force: bool,
    stylus_mode: bool,
    paper_rect: Option<egui::Rect>,
    cur_pressure: f64,
    last_up_pressure: f64,
    last_source: &'static str,
}

impl InkNoteApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::setup_fonts(&cc.egui_ctx);
        // Ctrl +/-/0 dùng để zoom TRANG, không zoom cả giao diện
        cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let default_paper = load_default_paper_settings();
        let draft_path = default_draft_path();

        let mut doc = Document::new();
        *doc.paper_settings_mut() = default_paper.clone();
        let mut restored_draft = false;
        if draft_path.exists() {
            match DocumentFile::load_from_path(&draft_path) {
                Ok(file) => {
                    doc = Document::from_file(file);
                    restored_draft = true;
                }
                Err(_) => {
                    let _ = std::fs::remove_file(&draft_path);
                }
            }
        }

        let mut autosave = AutoSaveState::with_draft_path(doc.revision(), draft_path);
        if restored_draft {
            autosave.mark_restored_draft(doc.revision());
        }

        let mut app = Self {
            theme: Theme::default(),
            toolbar_state: ToolbarState::default(),
            brush_config: BrushConfig::default(),
            doc,
            default_paper_settings: default_paper,
            current_path: None,
            confirm_unsaved: None,
            allow_close: false,
            autosave,
            batch_cache: HashMap::new(),
            thumb_cache: ThumbCache::default(),
            view: View::default(),
            confirm_clear: false,
            confirm_delete_page: None,
            paint_ms: 0.0,
            fps: 0.0,
            last_frame: Instant::now(),
            active: None,
            drawing_page_index: None,
            static_cache: None,
            render_cx: CairoRenderContext::new(),
            pull_overscroll: 0.0,
            last_pull_at: Instant::now(),
            pull_cooldown_until: Instant::now(),
            base_instant: Instant::now(),
            prev_frame_t: 0.0,
            last_event_secs: 0.0,
            touch_active: false,
            stylus_has_force: false,
            stylus_mode: false,
            paper_rect: None,
            cur_pressure: 0.0,
            last_up_pressure: 0.0,
            last_source: "Ready",
        };
        if restored_draft {
            app.rebuild_batch_cache();
        }
        app
    }
}

fn new_builder(
    builder_type: PenPathBuilderType,
    element: Element,
    now: Instant,
) -> Box<dyn Buildable<Emit = Segment>> {
    match builder_type {
        PenPathBuilderType::Simple => Box::new(PenPathSimpleBuilder::start(element, now)),
        PenPathBuilderType::Curved => Box::new(PenPathCurvedBuilder::start(element, now)),
        PenPathBuilderType::Modeled => Box::new(PenPathModeledBuilder::start(element, now)),
    }
}

impl InkNoteApp {
    fn monotonic_instant(&mut self, target_secs: f64) -> Instant {
        let s = target_secs.max(self.last_event_secs + 0.0005);
        self.last_event_secs = s;
        self.base_instant + Duration::from_secs_f64(s)
    }

    fn page_stride(settings: &PaperSettings) -> f32 {
        settings.page_height + PAGE_GAP
    }

    fn page_origin(&self, workspace: egui::Rect, page_index: usize) -> egui::Pos2 {
        let doc_origin = self.view.origin(workspace);
        let y_offset =
            page_index as f32 * Self::page_stride(self.doc.paper_settings()) * self.view.zoom;
        doc_origin + egui::vec2(0.0, y_offset)
    }

    fn page_screen_rect(&self, workspace: egui::Rect, page_index: usize) -> egui::Rect {
        let origin = self.page_origin(workspace, page_index);
        let settings = self.doc.paper_settings();
        match settings.mode {
            CanvasMode::Paper => paper_sheet_rect(origin, settings, self.view.zoom),
            CanvasMode::Whiteboard => {
                if self.doc.page_count() <= 1 {
                    workspace
                } else {
                    egui::Rect::from_min_max(
                        egui::pos2(workspace.left(), origin.y),
                        egui::pos2(
                            workspace.right(),
                            origin.y + settings.page_height * self.view.zoom,
                        ),
                    )
                }
            }
        }
    }

    /// Tìm trang hợp lệ chứa con trỏ `pos` để bắt đầu nét vẽ.
    fn drawable_page_at(&self, ctx: &egui::Context, pos: egui::Pos2) -> Option<usize> {
        if self.toolbar_state.read_only || self.toolbar_state.active_tool != ActiveTool::Pen {
            return None;
        }
        if egui::Popup::is_any_open(ctx)
            || self.confirm_clear
            || self.confirm_delete_page.is_some()
            || self.confirm_unsaved.is_some()
        {
            return None;
        }
        // Giữ Space để kéo trang bằng chuột/bút thay vì vẽ
        if ctx.input(|i| i.key_down(egui::Key::Space)) {
            return None;
        }
        let workspace = self.paper_rect?;
        if !workspace.contains(pos) {
            return None;
        }
        (0..self.doc.page_count()).find(|&idx| {
            self.page_screen_rect(workspace, idx)
                .intersect(workspace)
                .contains(pos)
        })
    }

    /// Đổi toạ độ màn hình → toạ độ cục bộ của trang đang viết.
    fn to_page_local(&self, pos: egui::Pos2) -> egui::Pos2 {
        match self.paper_rect {
            Some(workspace) => {
                let page_idx = self
                    .drawing_page_index
                    .unwrap_or_else(|| self.doc.active_page_index());
                let origin = self.page_origin(workspace, page_idx);
                ((pos - origin) / self.view.zoom).to_pos2()
            }
            None => pos,
        }
    }

    /// Cuộn khung nhìn đến trang `page_index` (đưa mép trên trang về sát đầu vùng làm việc).
    fn scroll_to_page(&mut self, page_index: usize) {
        let Some(workspace) = self.paper_rect else {
            return;
        };
        let settings = self.doc.paper_settings();
        if settings.mode == CanvasMode::Paper && self.view.pan == egui::Vec2::ZERO {
            self.view.pan.x =
                ((workspace.width() - settings.page_width * self.view.zoom).max(0.0)) * 0.5;
        }
        let top_pad = if settings.mode == CanvasMode::Paper {
            24.0
        } else {
            0.0
        };
        self.view.pan.y =
            top_pad - page_index as f32 * Self::page_stride(settings) * self.view.zoom;
    }

    fn add_page_after_current(&mut self) {
        let after = self.doc.active_page_index();
        self.doc.execute(Command::InsertPageAfter(after));
        let new_idx = self.doc.active_page_index();
        self.scroll_to_page(new_idx);
    }

    fn delete_page_at(&mut self, page_idx: usize) {
        if self.doc.page_count() <= 1 || page_idx >= self.doc.page_count() {
            return;
        }
        self.doc.execute(Command::DeletePage(page_idx));
        let cur = self.doc.active_page_index();
        self.scroll_to_page(cur);
    }

    /// Cập nhật trang đang xem dựa theo trang nằm ở tâm dọc vùng nhìn thấy.
    fn update_active_page_from_viewport(&mut self, workspace: egui::Rect) {
        let center_y = workspace.center().y;
        let mut best_idx = self.doc.active_page_index().min(self.doc.page_count() - 1);
        let mut best_dist = f32::INFINITY;
        for idx in 0..self.doc.page_count() {
            let r = self.page_screen_rect(workspace, idx);
            if r.top() <= center_y && center_y <= r.bottom() {
                best_idx = idx;
                break;
            }
            let dist = if center_y < r.top() {
                r.top() - center_y
            } else {
                center_y - r.bottom()
            };
            if dist < best_dist {
                best_dist = dist;
                best_idx = idx;
            }
        }
        self.doc.set_active_page_index(best_idx);
    }

    /// Kiểm tra và áp dụng lực kéo vượt đáy trang cuối để tự động tạo trang mới.
    /// Trả về `true` nếu đã tự động thêm trang mới.
    fn apply_pan_limits_and_pull_trigger(
        &mut self,
        workspace: egui::Rect,
        dy: f32,
        now: Instant,
    ) -> bool {
        let settings = self.doc.paper_settings();
        if settings.mode != CanvasMode::Paper {
            return false;
        }
        let page_count = self.doc.page_count().max(1);
        let total_h = ((page_count - 1) as f32 * Self::page_stride(settings)
            + settings.page_height)
            * self.view.zoom;
        let max_pan_y = (workspace.height() * 0.45).max(80.0);
        let bottom_keep = 140.0_f32.min(workspace.height() * 0.45);
        let min_pan_y = (workspace.height() - bottom_keep - total_h).min(24.0);

        if dy < 0.0 && self.view.pan.y <= min_pan_y + 1.0 {
            let overflow = (min_pan_y - self.view.pan.y).max(-dy);
            self.view.pan.y = min_pan_y;
            if now >= self.pull_cooldown_until {
                self.pull_overscroll += overflow;
                self.last_pull_at = now;
                if self.pull_overscroll >= PULL_TO_ADD_THRESHOLD {
                    let last = page_count - 1;
                    self.doc.execute(Command::InsertPageAfter(last));
                    self.pull_overscroll = 0.0;
                    self.pull_cooldown_until = now + Duration::from_millis(850);
                    self.scroll_to_page(self.doc.active_page_index());
                    return true;
                }
            }
            return false;
        }

        self.view.pan.y = self.view.pan.y.clamp(min_pan_y, max_pan_y);
        if dy > 0.0 {
            self.pull_overscroll = 0.0;
        }
        false
    }

    /// Tạo `n` nét mẫu rải khắp trang (để test hiệu năng). 1 lần = 1 bước undo.
    fn add_sample_strokes(&mut self, n: usize) {
        let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut list = Vec::with_capacity(n);
        for _ in 0..n {
            let (x0, y0) = (rnd() * 1800.0, rnd() * 1200.0);
            let len = 60.0 + rnd() * 160.0;
            let ang = rnd() * std::f64::consts::TAU;
            let amp = 4.0 + rnd() * 16.0;
            let freq = 2.0 + rnd() * 6.0;
            let point = |t: f64| {
                let along = t * len;
                let wave = amp * (t * freq * std::f64::consts::TAU).sin();
                let x = x0 + along * ang.cos() - wave * ang.sin();
                let y = y0 + along * ang.sin() + wave * ang.cos();
                let pressure = 0.2 + 0.6 * (t * std::f64::consts::PI).sin();
                Element::new(Vector2::new(x, y), pressure)
            };
            self.brush_config.new_style_seeds();
            let style = self.brush_config.style_for_current_options();
            let mut path = PenPath::new(point(0.0));
            // Mật độ điểm mẫu ~5-6px/điểm, sát với thực tế khi viết tay bằng Wacom
            let n_segs = (len / 6.0).clamp(12.0, 36.0) as usize;
            let segs: Vec<Segment> = (1..=n_segs)
                .map(|i| Segment::LineTo {
                    end: point(i as f64 / n_segs as f64),
                })
                .collect();
            path.extend(segs);

            self.render_cx.clear_batches();
            path.draw_composed(&mut self.render_cx, &style);
            let batches = std::mem::take(&mut self.render_cx.batches);
            let id = self.doc.alloc_id();
            self.batch_cache.insert(id, CachedStroke::new(batches));
            list.push(Stroke { id, path, style });
        }
        self.doc.execute(Command::AddStrokes(list));
    }

    /// Có thay đổi hoặc bản nháp chưa lưu thành file `.inknote` chính thức hay không.
    fn needs_save_prompt(&self) -> bool {
        self.doc.is_dirty() || (self.current_path.is_none() && self.autosave.has_draft())
    }

    /// Tái tạo lại bộ nhớ đệm lưới tam giác cho mọi nét trên tất cả trang.
    fn rebuild_batch_cache(&mut self) {
        self.batch_cache.clear();
        for page in self.doc.pages() {
            for s in &page.strokes {
                self.render_cx.clear_batches();
                s.path.draw_composed(&mut self.render_cx, &s.style);
                let batches = std::mem::take(&mut self.render_cx.batches);
                self.batch_cache.insert(s.id, CachedStroke::new(batches));
            }
        }
    }

    /// Lưu ghi chú (nếu đã có đường dẫn thì lưu thẳng, chưa có thì mở hộp thoại Save As).
    /// Trả về `true` nếu đã lưu thành công.
    fn save_document(&mut self) -> bool {
        if let Some(path) = self.current_path.clone() {
            if let Err(e) = self.doc.to_file().save_to_path(&path) {
                eprintln!("Lỗi khi lưu file {:?}: {e}", path);
                false
            } else {
                self.doc.mark_saved();
                self.autosave.note_manual_save(self.doc.revision());
                true
            }
        } else {
            self.save_document_as()
        }
    }

    /// Mở hộp thoại chọn nơi lưu file .inknote mới. Trả về `true` nếu người dùng chọn lưu và ghi thành công.
    fn save_document_as(&mut self) -> bool {
        let default_name = format!("{}.inknote", self.doc.metadata().title);
        let path = rfd::FileDialog::new()
            .add_filter("InkNote (*.inknote)", &["inknote"])
            .set_file_name(&default_name)
            .save_file();
        if let Some(path) = path {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                self.doc.metadata_mut().title = stem.to_string();
            }
            if let Err(e) = self.doc.to_file().save_to_path(&path) {
                eprintln!("Lỗi khi lưu file {:?}: {e}", path);
                false
            } else {
                self.doc.mark_saved();
                self.current_path = Some(path);
                self.autosave.note_manual_save(self.doc.revision());
                true
            }
        } else {
            false
        }
    }

    /// Nạp dữ liệu từ file .inknote trên đĩa.
    fn load_document_from(&mut self, path: PathBuf) {
        match DocumentFile::load_from_path(&path) {
            Ok(file) => {
                self.doc = Document::from_file(file);
                self.current_path = Some(path);
                self.autosave.reset(self.doc.revision());
                self.thumb_cache.clear();
                self.static_cache = None;
                self.active = None;
                self.drawing_page_index = None;
                self.pull_overscroll = 0.0;
                self.rebuild_batch_cache();
            }
            Err(e) => {
                eprintln!("Lỗi khi mở file: {e}");
            }
        }
    }

    /// Bắt đầu quy trình mở file (hỏi xác nhận nếu có thay đổi chưa lưu).
    fn trigger_open(&mut self) {
        if self.needs_save_prompt() {
            self.confirm_unsaved = Some(UnsavedAction::Open);
        } else if let Some(path) = rfd::FileDialog::new()
            .add_filter("InkNote (*.inknote)", &["inknote"])
            .pick_file()
        {
            self.load_document_from(path);
        }
    }

    /// Bắt đầu quy trình tạo ghi chú mới.
    fn trigger_new(&mut self) {
        if self.needs_save_prompt() {
            self.confirm_unsaved = Some(UnsavedAction::NewNote);
        } else {
            self.new_document();
        }
    }

    /// Khởi tạo tài liệu trống mới (áp dụng PaperSettings mặc định).
    fn new_document(&mut self) {
        self.doc = Document::new();
        *self.doc.paper_settings_mut() = self.default_paper_settings.clone();
        self.current_path = None;
        self.autosave.reset(self.doc.revision());
        self.batch_cache.clear();
        self.thumb_cache.clear();
        self.static_cache = None;
        self.active = None;
        self.drawing_page_index = None;
        self.pull_overscroll = 0.0;
        self.view.pan = egui::Vec2::ZERO;
    }

    fn pen_down(&mut self, pos: egui::Pos2, pressure: f64, now: Instant) {
        self.cur_pressure = pressure;
        let pos = self.to_page_local(pos);
        let element = Element::new(Vector2::new(pos.x as f64, pos.y as f64), pressure);

        if let Some(active) = &mut self.active {
            let res = active.builder.handle_event(
                PenEvent::Down {
                    element,
                    modifier_keys: HashSet::new(),
                },
                now,
                Constraints::default(),
            );
            match res.progress {
                BuilderProgress::InProgress => {}
                BuilderProgress::EmitContinue(segments) => {
                    active.path.extend(segments);
                }
                BuilderProgress::Finished(segments) => {
                    active.path.extend(segments);
                    self.finish_active_stroke();
                    return;
                }
            }
            self.advance_active_committed();
        } else {
            self.brush_config.new_style_seeds();
            let style = self.brush_config.style_for_current_options();
            let path = PenPath::new(element);
            let builder = new_builder(self.brush_config.builder_type, element, now);
            self.active = Some(ActiveStroke {
                path,
                style,
                builder,
                committed_batches: Vec::new(),
                committed_segs: 0,
            });
        }
    }

    /// Chốt phần đầu nét đã vẽ thành lưới tam giác theo từng đợt (16 đoạn/đợt) để không phải vẽ lại từ đầu
    fn advance_active_committed(&mut self) {
        let Some(active) = &mut self.active else {
            return;
        };
        const CHUNK_SIZE: usize = 16;
        let total = active.path.segments.len();
        if total >= active.committed_segs + CHUNK_SIZE {
            let chunk_segs = &active.path.segments[active.committed_segs..total];
            let start = if active.committed_segs == 0 {
                active.path.start
            } else {
                active.path.segments[active.committed_segs - 1].end()
            };
            let chunk_path = PenPath::new_w_segments(start, chunk_segs.to_vec());
            self.render_cx.clear_batches();
            chunk_path.draw_composed(&mut self.render_cx, &active.style);
            active.committed_batches.append(&mut self.render_cx.batches);
            active.committed_segs = total;
        }
    }

    /// Dựng lại lưới tĩnh cho từng trang đang hiển thị trong vùng nhìn thấy (`paper`).
    fn build_static_meshes(
        &self,
        paper: egui::Rect,
    ) -> Vec<(usize, std::sync::Arc<egui::epaint::Mesh>)> {
        let view = self.view;
        let mut result = Vec::new();
        for (page_idx, page) in self.doc.pages().iter().enumerate() {
            let page_rect_screen = self.page_screen_rect(paper, page_idx);
            let cull_rect = page_rect_screen.intersect(paper);
            if !cull_rect.is_positive() {
                continue;
            }
            let origin = self.page_origin(paper, page_idx);
            let mut n_verts = 0;
            let mut n_indices = 0;
            for s in &page.strokes {
                if let Some(c) = self.batch_cache.get(&s.id) {
                    let s_rect = egui::Rect::from_min_max(
                        origin + c.bbox.min.to_vec2() * view.zoom,
                        origin + c.bbox.max.to_vec2() * view.zoom,
                    );
                    if s_rect.intersects(cull_rect) {
                        for b in &c.batches {
                            n_verts += b.vertices.len();
                            n_indices += b.indices.len();
                        }
                    }
                }
            }
            if n_indices == 0 {
                continue;
            }
            let mut mesh = egui::epaint::Mesh::default();
            mesh.vertices.reserve(n_verts);
            mesh.indices.reserve(n_indices);
            for s in &page.strokes {
                let Some(c) = self.batch_cache.get(&s.id) else {
                    continue;
                };
                let s_rect = egui::Rect::from_min_max(
                    origin + c.bbox.min.to_vec2() * view.zoom,
                    origin + c.bbox.max.to_vec2() * view.zoom,
                );
                if !s_rect.intersects(cull_rect) {
                    continue;
                }
                append_batches_to_mesh(&c.batches, origin, view.zoom, &mut mesh);
            }
            if !mesh.is_empty() {
                result.push((page_idx, std::sync::Arc::new(mesh)));
            }
        }
        result
    }

    fn pen_up(&mut self, pos: egui::Pos2, pressure: f64, now: Instant) {
        self.cur_pressure = pressure;
        self.last_up_pressure = pressure;
        let pos = self.to_page_local(pos);
        let element = Element::new(Vector2::new(pos.x as f64, pos.y as f64), pressure);

        if let Some(active) = &mut self.active {
            let res = active.builder.handle_event(
                PenEvent::Up {
                    element,
                    modifier_keys: HashSet::new(),
                },
                now,
                Constraints::default(),
            );
            if let BuilderProgress::EmitContinue(segments) | BuilderProgress::Finished(segments) =
                res.progress
            {
                active.path.extend(segments);
            }
        }
        self.finish_active_stroke();
    }

    fn finish_active_stroke(&mut self) {
        let target_page = self
            .drawing_page_index
            .take()
            .unwrap_or_else(|| self.doc.active_page_index());
        if let Some(active) = self.active.take() {
            self.render_cx.clear_batches();
            active
                .path
                .draw_composed(&mut self.render_cx, &active.style);
            let batches = std::mem::take(&mut self.render_cx.batches);
            let id = self.doc.alloc_id();
            self.batch_cache.insert(id, CachedStroke::new(batches));
            self.doc.set_active_page_index(target_page);
            self.doc.execute(Command::AddStroke(Stroke {
                id,
                path: active.path,
                style: active.style,
            }));
        }
    }

    /// Vẽ lưới (toạ độ trang) lên màn hình: `origin` = gốc trang trên màn hình, `zoom` = tỉ lệ.
    fn paint_batches(
        painter: &egui::Painter,
        batches: &[DrawBatch],
        origin: egui::Pos2,
        zoom: f32,
    ) {
        for batch in batches {
            if batch.indices.is_empty() || batch.vertices.is_empty() {
                continue;
            }
            let [r, g, b, a] = batch.color;
            let col = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
            let mut mesh = egui::epaint::Mesh::default();
            mesh.indices.extend_from_slice(&batch.indices);
            mesh.vertices.reserve(batch.vertices.len());
            for &[x, y] in &batch.vertices {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: egui::pos2(origin.x + x * zoom, origin.y + y * zoom),
                    uv: egui::epaint::WHITE_UV,
                    color: col,
                });
            }
            painter.add(egui::Shape::mesh(mesh));
        }
    }
}

impl eframe::App for InkNoteApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let (events, frame_t) = ctx.input(|i| (i.events.clone(), i.time));
        let dt = (frame_t - self.prev_frame_t).clamp(0.001, 0.05);
        let n_ev = events.len().max(1) as f64;

        for (k, e) in events.into_iter().enumerate() {
            let ev_secs = frame_t - dt + dt * (k as f64 + 1.0) / n_ev;
            match e {
                egui::Event::Touch {
                    phase, pos, force, ..
                } => {
                    self.stylus_mode = true;
                    match phase {
                        egui::TouchPhase::Start => {
                            let Some(page_idx) = self.drawable_page_at(&ctx, pos) else {
                                continue;
                            };
                            self.doc.set_active_page_index(page_idx);
                            self.drawing_page_index = Some(page_idx);
                            self.touch_active = true;
                            self.stylus_has_force = force.is_some();
                            let p = match force {
                                Some(f) => (f as f64).clamp(0.0, 1.0),
                                None => 0.0,
                            };
                            self.last_source = if force.is_some() {
                                "Wacom Stylus (Down)"
                            } else {
                                "Wacom Stylus (Touch p=0)"
                            };
                            let now = self.monotonic_instant(ev_secs);
                            self.pen_down(pos, p, now);
                        }
                        egui::TouchPhase::Move => {
                            if self.touch_active && self.active.is_some() {
                                if force.is_some() {
                                    self.stylus_has_force = true;
                                }
                                let p = match force {
                                    Some(f) => (f as f64).clamp(0.0, 1.0),
                                    None => 0.0,
                                };
                                self.last_source = "Wacom Stylus (Drawing)";
                                let now = self.monotonic_instant(ev_secs);
                                self.pen_down(pos, p, now);
                            } else {
                                self.last_source = "Wacom Stylus (Hover)";
                            }
                        }
                        egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                            if self.active.is_some() {
                                let p = match force {
                                    Some(f) => (f as f64).clamp(0.0, 1.0),
                                    None => 0.0,
                                };
                                let now = self.monotonic_instant(ev_secs);
                                self.pen_up(pos, p, now);
                            }
                            self.touch_active = false;
                            self.stylus_has_force = false;
                        }
                    }
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    ..
                } if !self.touch_active && !self.stylus_mode => {
                    self.last_source = "Mouse (p=0.5)";
                    let now = self.monotonic_instant(ev_secs);
                    if pressed {
                        if let Some(page_idx) = self.drawable_page_at(&ctx, pos) {
                            self.doc.set_active_page_index(page_idx);
                            self.drawing_page_index = Some(page_idx);
                            self.pen_down(pos, Element::PRESSURE_DEFAULT, now);
                        }
                    } else if !pressed && self.active.is_some() {
                        self.pen_up(pos, Element::PRESSURE_DEFAULT, now);
                    }
                }
                egui::Event::PointerMoved(pos) if !self.touch_active && !self.stylus_mode => {
                    if self.active.is_some() {
                        let now = self.monotonic_instant(ev_secs);
                        self.pen_down(pos, Element::PRESSURE_DEFAULT, now);
                    }
                }
                _ => {}
            }
        }
        self.prev_frame_t = frame_t;

        // Khi người dùng tắt app mà chưa lưu ra file .inknote chính thức -> chặn tắt để hỏi xác nhận
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if self.current_path.is_none() && self.needs_save_prompt() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.confirm_unsaved = Some(UnsavedAction::CloseApp);
            }
        }

        // Cập nhật trạng thái auto-save trước khi vẽ tiêu đề & thanh công cụ để chấm xanh/đỏ & thông báo tức thì
        let autosave_now = Instant::now();
        self.autosave.poll_and_maybe_save(
            &mut self.doc,
            self.current_path.as_ref(),
            self.active.is_some(),
            autosave_now,
        );

        // Cập nhật tiêu đề cửa sổ theo tên file hiện tại và trạng thái sửa đổi (*)
        let doc_title = match &self.current_path {
            Some(p) => p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("untitled.inknote"),
            None => self.doc.metadata().title.as_str(),
        };
        let dirty_mark = if self.needs_save_prompt() { " *" } else { "" };
        let full_title = format!("InkNote - {}{}", doc_title, dirty_mark);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(full_title));

        let file_status = self.autosave.file_status(
            &self.doc,
            doc_title,
            self.current_path.as_ref(),
            autosave_now,
        );

        // Khu vực 1 & 2: Navigation Toolbar + Contextual Tool Options Bar
        let wacom_status = format!(
            "{} | p={:.3} | up_p={:.3} | strokes={}",
            self.last_source,
            self.cur_pressure,
            self.last_up_pressure,
            self.doc.strokes().len()
        );
        let mut paper_settings = self.doc.paper_settings().clone();
        let page_info = (self.doc.active_page_index() + 1, self.doc.page_count());
        let actions = show_toolbars(
            ui,
            &self.theme,
            &mut self.toolbar_state,
            &mut self.brush_config,
            &mut paper_settings,
            &file_status,
            page_info,
            &wacom_status,
        );
        if self.doc.set_paper_settings(paper_settings.clone()) {
            self.default_paper_settings = paper_settings.clone();
            save_default_paper_settings(&paper_settings);
        }
        // Phím tắt: Ctrl+Z = Undo; Ctrl+Y hoặc Ctrl+Shift+Z = Redo
        // Phím tắt tệp: Ctrl+S = Save, Ctrl+Shift+S = Save As, Ctrl+O = Open, Ctrl+N = New
        let (key_undo, key_redo, key_save, key_save_as, key_open, key_new, key_pgup, key_pgdn) =
            ctx.input(|i| {
                let cmd = i.modifiers.command;
                let shift = i.modifiers.shift;
                let z = i.key_pressed(egui::Key::Z);
                (
                    cmd && !shift && z,
                    cmd && ((shift && z) || i.key_pressed(egui::Key::Y)),
                    cmd && !shift && i.key_pressed(egui::Key::S),
                    cmd && shift && i.key_pressed(egui::Key::S),
                    cmd && !shift && i.key_pressed(egui::Key::O),
                    cmd && !shift && i.key_pressed(egui::Key::N),
                    i.key_pressed(egui::Key::PageUp),
                    i.key_pressed(egui::Key::PageDown),
                )
            });
        // Không hoàn tác / thao tác file / thêm trang khi đang kéo nét dở
        if self.active.is_none() {
            if actions.undo_clicked || key_undo {
                self.doc.undo();
            }
            if actions.redo_clicked || key_redo {
                self.doc.redo();
            }
            if actions.clear_clicked {
                self.confirm_clear = true;
            }
            if actions.add_page_clicked {
                self.add_page_after_current();
            }
            if actions.delete_page_clicked && self.doc.page_count() > 1 {
                self.confirm_delete_page = Some(self.doc.active_page_index());
            }
            if key_pgup && self.doc.active_page_index() > 0 {
                let target = self.doc.active_page_index() - 1;
                self.doc.set_active_page_index(target);
                self.scroll_to_page(target);
            }
            if key_pgdn && self.doc.active_page_index() + 1 < self.doc.page_count() {
                let target = self.doc.active_page_index() + 1;
                self.doc.set_active_page_index(target);
                self.scroll_to_page(target);
            }
            if actions.save_clicked || key_save {
                self.save_document();
            }
            if key_save_as {
                self.save_document_as();
            }
            if actions.open_clicked || key_open {
                self.trigger_open();
            }
            if actions.new_clicked || key_new {
                self.trigger_new();
            }
        }

        // Hộp xác nhận xoá sạch nét trên trang (xoá xong vẫn Undo được)
        if self.confirm_clear {
            let mut do_clear = false;
            let mut cancel = false;
            #[allow(deprecated)]
            let modal = egui::Modal::new(egui::Id::new("confirm_clear_page")).show(&ctx, |ui| {
                ui.set_width(300.0);
                ui.heading("Clear this page?");
                ui.add_space(4.0);
                ui.label(
                    "All strokes on this page will be removed. You can undo this with Ctrl+Z.",
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Clear page").clicked() {
                        do_clear = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
            if do_clear {
                self.doc.execute(Command::ClearPage);
            }
            if do_clear || cancel || modal.should_close() {
                self.confirm_clear = false;
            }
        }

        // Hộp xác nhận xoá trang được chọn (xoá xong vẫn Undo được)
        if let Some(target_idx) = self.confirm_delete_page {
            let page_num = target_idx + 1;
            let can_delete = self.doc.page_count() > 1 && target_idx < self.doc.page_count();
            let mut do_delete = false;
            let mut cancel = false;
            #[allow(deprecated)]
            let modal = egui::Modal::new(egui::Id::new("confirm_delete_page")).show(&ctx, |ui| {
                ui.set_width(300.0);
                ui.heading(format!("Delete Page {}?", page_num));
                ui.add_space(4.0);
                ui.label(
                    "This page and all of its strokes will be removed. You can undo this with Ctrl+Z.",
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(can_delete, egui::Button::new("Delete page"))
                        .clicked()
                    {
                        do_delete = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
            if do_delete && can_delete {
                self.delete_page_at(target_idx);
            }
            if do_delete || cancel || !can_delete || modal.should_close() {
                self.confirm_delete_page = None;
            }
        }

        // Hộp xác nhận lưu thay đổi trước khi mở file khác, tạo mới hoặc tắt app khi chưa lưu file
        if let Some(action) = self.confirm_unsaved {
            let mut do_save = false;
            let mut do_discard = false;
            let mut cancel = false;
            let (title, body, save_btn_label) = match action {
                UnsavedAction::CloseApp => (
                    "Save before closing?",
                    "This note hasn't been saved to a file yet. Do you want to save it before deleting the temporary draft and closing?",
                    "Save",
                ),
                UnsavedAction::Open | UnsavedAction::NewNote => (
                    "Save changes?",
                    "You have unsaved changes in this note. Do you want to save before proceeding?",
                    "Save & Continue",
                ),
            };
            #[allow(deprecated)]
            let modal =
                egui::Modal::new(egui::Id::new("confirm_unsaved_changes")).show(&ctx, |ui| {
                    ui.set_width(340.0);
                    ui.heading(title);
                    ui.add_space(4.0);
                    ui.label(body);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button(save_btn_label).clicked() {
                            do_save = true;
                        }
                        if ui.button("Don't Save").clicked() {
                            do_discard = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
            if do_save {
                let saved = self.save_document();
                self.confirm_unsaved = None;
                if saved {
                    match action {
                        UnsavedAction::Open => {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("InkNote (*.inknote)", &["inknote"])
                                .pick_file()
                            {
                                self.load_document_from(path);
                            }
                        }
                        UnsavedAction::NewNote => self.new_document(),
                        UnsavedAction::CloseApp => {
                            self.allow_close = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                }
            } else if do_discard {
                self.confirm_unsaved = None;
                self.autosave.delete_draft();
                match action {
                    UnsavedAction::Open => {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("InkNote (*.inknote)", &["inknote"])
                            .pick_file()
                        {
                            self.load_document_from(path);
                        }
                    }
                    UnsavedAction::NewNote => self.new_document(),
                    UnsavedAction::CloseApp => {
                        self.allow_close = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            } else if cancel || modal.should_close() {
                self.confirm_unsaved = None;
            }
        }

        // FPS khi đang tương tác liên tục (bỏ qua khoảng nghỉ lúc không chạm vào app)
        let now = Instant::now();
        let real_dt = now.duration_since(self.last_frame).as_secs_f32().max(1e-4);
        self.last_frame = now;
        if real_dt < 0.25 {
            self.fps = if self.fps == 0.0 {
                1.0 / real_dt
            } else {
                self.fps * 0.9 + 0.1 / real_dt
            };
        }

        // Zoom / kéo trang & cuộn dọc nhiều trang (+ trigger kéo đáy tạo trang mới)
        if let Some(paper) = self.paper_rect {
            let (hover, zoom_delta, scroll, middle, space_drag, ptr_delta, keys) = ctx.input(|i| {
                let cmd = i.modifiers.command;
                (
                    i.pointer.hover_pos(),
                    i.zoom_delta(),
                    i.smooth_scroll_delta,
                    i.pointer.middle_down(),
                    i.key_down(egui::Key::Space) && i.pointer.primary_down(),
                    i.pointer.delta(),
                    (
                        cmd && i.key_pressed(egui::Key::Num0),
                        cmd && (i.key_pressed(egui::Key::Equals) || i.key_pressed(egui::Key::Plus)),
                        cmd && i.key_pressed(egui::Key::Minus),
                    ),
                )
            });
            let blocked = egui::Popup::is_any_open(&ctx)
                || self.confirm_clear
                || self.confirm_delete_page.is_some()
                || self.confirm_unsaved.is_some();
            let mut scrolled_or_panned = false;
            let mut dy_total = 0.0_f32;
            if !blocked {
                if keys.0 {
                    self.view.zoom = 1.0;
                    if self.doc.paper_settings().mode == CanvasMode::Paper {
                        let centered_x =
                            ((paper.width() - self.doc.paper_settings().page_width).max(0.0)) * 0.5;
                        self.view.pan.x = centered_x;
                    } else {
                        self.view.pan.x = 0.0;
                    }
                    let cur = self.doc.active_page_index();
                    self.scroll_to_page(cur);
                    self.pull_overscroll = 0.0;
                }
                if keys.1 {
                    self.view.zoom_at(paper, paper.center(), 1.25);
                    scrolled_or_panned = true;
                }
                if keys.2 {
                    self.view.zoom_at(paper, paper.center(), 0.8);
                    scrolled_or_panned = true;
                }
                if let Some(h) = hover.filter(|h| paper.contains(*h)) {
                    if zoom_delta != 1.0 {
                        self.view.zoom_at(paper, h, zoom_delta);
                        scrolled_or_panned = true;
                    }
                    if scroll != egui::Vec2::ZERO {
                        self.view.pan += scroll;
                        dy_total += scroll.y;
                        scrolled_or_panned = true;
                    }
                    if (middle || space_drag) && ptr_delta != egui::Vec2::ZERO {
                        self.view.pan += ptr_delta;
                        dy_total += ptr_delta.y;
                        scrolled_or_panned = true;
                    }
                }
            }

            if scrolled_or_panned {
                let added = self.apply_pan_limits_and_pull_trigger(paper, dy_total, now);
                if !added && self.active.is_none() {
                    self.update_active_page_from_viewport(paper);
                }
            } else if self.pull_overscroll > 0.0
                && now.duration_since(self.last_pull_at) > Duration::from_millis(120)
            {
                // Hồi lực kéo về 0 mượt mà khi người dùng ngừng cuộn
                self.pull_overscroll = (self.pull_overscroll - 480.0 * real_dt).max(0.0);
            }
        }

        // F9: thêm 2000 nét mẫu để test hiệu năng
        if self.active.is_none() && ctx.input(|i| i.key_pressed(egui::Key::F9)) {
            self.add_sample_strokes(2000);
        }

        // Thanh thumbnail danh sách trang bên trái (bật/tắt bằng nút Danh sách trang)
        if self.toolbar_state.show_thumbnails {
            let panel_frame = egui::Frame::new()
                .fill(self.theme.thumb_panel_bg)
                .stroke(egui::Stroke::new(1.0, self.theme.thumb_panel_stroke))
                .inner_margin(egui::Margin::symmetric(8, 6));
            let mut thumb_action = ThumbnailAction::None;
            egui::Panel::left("page_thumbnails_sidebar")
                .resizable(false)
                .exact_size(self.theme.thumb_panel_width)
                .frame(panel_frame)
                .show(ui, |ui| {
                    thumb_action = show_thumbnail_panel(
                        ui,
                        &self.theme,
                        &self.doc,
                        &self.batch_cache,
                        &mut self.thumb_cache,
                    );
                });
            if self.active.is_none() {
                match thumb_action {
                    ThumbnailAction::None => {}
                    ThumbnailAction::SelectPage(idx) => {
                        self.doc.set_active_page_index(idx);
                        self.scroll_to_page(idx);
                    }
                    ThumbnailAction::AddPage => {
                        self.add_page_after_current();
                    }
                    ThumbnailAction::DeletePage(idx) => {
                        if self.doc.page_count() > 1 {
                            self.confirm_delete_page = Some(idx);
                        }
                    }
                }
            }
        }

        // Khu vực 3: Main Canvas (trang giấy chấm Dotted Paper)
        let workspace_frame = egui::Frame::central_panel(ui.style()).fill(self.theme.workspace_bg);
        egui::CentralPanel::default()
            .frame(workspace_frame)
            .show(ui, |ui| {
                let avail = ui.max_rect();
                let settings = self.doc.paper_settings();
                let paper_rect = match settings.mode {
                    CanvasMode::Paper => avail.shrink(self.theme.paper_margin),
                    CanvasMode::Whiteboard => avail,
                };
                self.paper_rect = Some(paper_rect);

                let view = self.view;
                let page_count = self.doc.page_count().max(1);

                // 1. Tự động căn giữa trang ở chế độ Paper khi mở ghi chú mới (pan chưa đặt)
                if self.view.pan == egui::Vec2::ZERO && settings.mode == CanvasMode::Paper {
                    let centered_x =
                        ((paper_rect.width() - settings.page_width * view.zoom).max(0.0)) * 0.5;
                    self.view.pan = egui::vec2(centered_x, 24.0);
                }

                // 2. Vẽ nền giấy cho từng trang đang nằm trong khung nhìn
                match settings.mode {
                    CanvasMode::Paper => {
                        for page_idx in 0..page_count {
                            let origin = self.page_origin(paper_rect, page_idx);
                            draw_paper(
                                ui.painter(),
                                paper_rect,
                                origin,
                                &self.theme,
                                settings,
                                view.zoom,
                            );
                        }
                    }
                    CanvasMode::Whiteboard => {
                        let origin0 = self.page_origin(paper_rect, 0);
                        draw_paper(
                            ui.painter(),
                            paper_rect,
                            origin0,
                            &self.theme,
                            settings,
                            view.zoom,
                        );
                        if page_count > 1 {
                            let div_painter = ui.painter().with_clip_rect(paper_rect);
                            for page_idx in 1..page_count {
                                let p_rect = self.page_screen_rect(paper_rect, page_idx);
                                let y = p_rect.top();
                                if y >= paper_rect.top() && y <= paper_rect.bottom() {
                                    div_painter.line_segment(
                                        [
                                            egui::pos2(paper_rect.left(), y),
                                            egui::pos2(paper_rect.right(), y),
                                        ],
                                        egui::Stroke::new(1.0, self.theme.paper_border),
                                    );
                                    div_painter.text(
                                        egui::pos2(paper_rect.left() + 10.0, y + 6.0),
                                        egui::Align2::LEFT_TOP,
                                        format!("Page {}", page_idx + 1),
                                        egui::FontId::proportional(self.theme.font_size_small),
                                        self.theme.tool_text_muted,
                                    );
                                }
                            }
                        }
                    }
                }

                let paint_start = Instant::now();

                // 3. Vẽ các nét bút đã hoàn thành theo từng trang (cắt gọn trong tờ giấy của trang đó)
                let static_key = StaticKey::new(
                    self.doc.revision(),
                    view.zoom,
                    view.pan,
                    paper_rect,
                    page_count,
                    settings.mode,
                );
                let static_meshes = match &self.static_cache {
                    Some((k, meshes)) if *k == static_key => meshes.clone(),
                    _ => {
                        let meshes = self.build_static_meshes(paper_rect);
                        self.static_cache = Some((static_key, meshes.clone()));
                        meshes
                    }
                };
                for (page_idx, mesh) in static_meshes {
                    let clip = self
                        .page_screen_rect(paper_rect, page_idx)
                        .intersect(paper_rect);
                    if clip.is_positive() {
                        ui.painter()
                            .with_clip_rect(clip)
                            .add(egui::Shape::Mesh(mesh));
                    }
                }

                // 4. Vẽ nét bút đang kéo (active) trên đúng trang đang chạm
                if let Some(active) = &self.active {
                    let active_page = self
                        .drawing_page_index
                        .unwrap_or_else(|| self.doc.active_page_index());
                    let active_origin = self.page_origin(paper_rect, active_page);
                    let active_clip = self
                        .page_screen_rect(paper_rect, active_page)
                        .intersect(paper_rect);
                    if active_clip.is_positive() {
                        let painter = ui.painter().with_clip_rect(active_clip);
                        // 4a. Các đoạn nét đã chốt (đã tessellate sẵn)
                        if !active.committed_batches.is_empty() {
                            Self::paint_batches(
                                &painter,
                                &active.committed_batches,
                                active_origin,
                                view.zoom,
                            );
                        }

                        // 4b. Phần đuôi ngắn chưa chốt (< 16 đoạn)
                        if active.path.segments.len() > active.committed_segs {
                            let tail_start = if active.committed_segs == 0 {
                                active.path.start
                            } else {
                                active.path.segments[active.committed_segs - 1].end()
                            };
                            let tail_path = PenPath::new_w_segments(
                                tail_start,
                                active.path.segments[active.committed_segs..].to_vec(),
                            );
                            self.render_cx.clear_batches();
                            tail_path.draw_composed(&mut self.render_cx, &active.style);
                            Self::paint_batches(
                                &painter,
                                &self.render_cx.batches,
                                active_origin,
                                view.zoom,
                            );
                        }

                        // 4c. Đầu bút động và dự đoán nét từ Modeler
                        self.render_cx.clear_batches();
                        active
                            .builder
                            .draw_styled(&mut self.render_cx, &active.style, 1.0);
                        Self::paint_batches(
                            &painter,
                            &self.render_cx.batches,
                            active_origin,
                            view.zoom,
                        );
                    }
                }

                // 5. Chỉ báo kéo quá đáy trang cuối để tự động tạo trang mới (Paper mode)
                if settings.mode == CanvasMode::Paper {
                    let last_sheet = self.page_screen_rect(paper_rect, page_count - 1);
                    if last_sheet.bottom() <= paper_rect.bottom() - 8.0
                        && last_sheet.bottom() >= paper_rect.top()
                    {
                        let hint_pos = egui::pos2(
                            last_sheet.center().x,
                            (last_sheet.bottom() + 26.0).min(paper_rect.bottom() - 28.0),
                        );
                        let hint_painter = ui.painter().with_clip_rect(paper_rect);
                        if self.pull_overscroll > 4.0 {
                            let ratio =
                                (self.pull_overscroll / PULL_TO_ADD_THRESHOLD).clamp(0.0, 1.0);
                            let pill_rect =
                                egui::Rect::from_center_size(hint_pos, egui::vec2(210.0, 26.0));
                            hint_painter.rect_filled(
                                pill_rect,
                                egui::CornerRadius::same(13),
                                self.theme.tool_bar_pill_bg,
                            );
                            let fill_w = (pill_rect.width() * ratio).max(10.0);
                            let fill_rect = egui::Rect::from_min_size(
                                pill_rect.min,
                                egui::vec2(fill_w, pill_rect.height()),
                            );
                            hint_painter.rect_filled(
                                fill_rect,
                                egui::CornerRadius::same(13),
                                self.theme.tool_selected_bg,
                            );
                            hint_painter.text(
                                pill_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "Pull down to add page...",
                                egui::FontId::proportional(self.theme.font_size_small),
                                self.theme.tool_text_fg,
                            );
                        } else {
                            hint_painter.text(
                                hint_pos,
                                egui::Align2::CENTER_CENTER,
                                "Scroll down further to add a new page",
                                egui::FontId::proportional(self.theme.font_size_small),
                                self.theme.tool_text_muted,
                            );
                        }
                    }
                }

                // 6. Thông số góc dưới-trái: Mode, zoom, thời gian dựng khung, FPS
                self.paint_ms =
                    self.paint_ms * 0.9 + paint_start.elapsed().as_secs_f32() * 1000.0 * 0.1;
                let mode_str = match settings.mode {
                    CanvasMode::Paper => format!("Paper ({:?})", settings.paper_size),
                    CanvasMode::Whiteboard => "Whiteboard".to_string(),
                };
                ui.painter().with_clip_rect(paper_rect).text(
                    paper_rect.left_bottom() + egui::vec2(10.0, -8.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!(
                        "Mode: {}  |  Zoom {:.0}%  |  build {:.1} ms  |  {:.0} fps  |  F9: +2000 strokes",
                        mode_str,
                        view.zoom_percent(),
                        self.paint_ms,
                        self.fps
                    ),
                    egui::FontId::proportional(self.theme.font_size_small),
                    self.theme.tool_text_muted,
                );
            });

        // Chỉ hẹn vẽ lại khi có bộ đếm auto-save, thông báo 3s hoặc hiệu ứng hồi lực kéo trang
        if self.pull_overscroll > 0.0 {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else if let Some(delay) =
            self.autosave
                .next_repaint_delay(&self.doc, self.current_path.as_ref(), autosave_now)
        {
            ctx.request_repaint_after(delay);
        }
    }

    fn on_exit(&mut self) {
        self.autosave
            .flush_on_exit(&mut self.doc, self.current_path.as_ref());
    }
}

fn main() -> eframe::Result {
    let mut wgpu_options = eframe::egui_wgpu::WgpuConfiguration::default();
    // VSync (Fifo): hãm tối đa FPS theo đúng tần số quét của màn hình (144Hz -> 144fps, 60Hz -> 60fps)
    wgpu_options.surface.present_mode = eframe::egui_wgpu::wgpu::PresentMode::Fifo;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("InkNote")
            .with_inner_size([1200.0, 800.0]),
        wgpu_options,
        ..Default::default()
    };
    eframe::run_native(
        "InkNote",
        options,
        Box::new(|cc| Ok(Box::new(InkNoteApp::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{vec2, Pos2, RawInput, Rect};
    use inknote_core::{CanvasMode, PaperOrientation, PaperPattern, PaperSize};

    fn new_test_app(ctx: &egui::Context) -> InkNoteApp {
        theme::setup_fonts(ctx);
        InkNoteApp {
            theme: Theme::default(),
            toolbar_state: ToolbarState::default(),
            brush_config: BrushConfig::default(),
            doc: Document::new(),
            default_paper_settings: PaperSettings::default(),
            current_path: None,
            confirm_unsaved: None,
            allow_close: false,
            autosave: AutoSaveState::new(0),
            batch_cache: HashMap::new(),
            thumb_cache: ThumbCache::default(),
            view: View::default(),
            confirm_clear: false,
            confirm_delete_page: None,
            paint_ms: 0.0,
            fps: 0.0,
            last_frame: Instant::now(),
            active: None,
            drawing_page_index: None,
            static_cache: None,
            render_cx: CairoRenderContext::new(),
            pull_overscroll: 0.0,
            last_pull_at: Instant::now(),
            pull_cooldown_until: Instant::now(),
            base_instant: Instant::now(),
            prev_frame_t: 0.0,
            last_event_secs: 0.0,
            touch_active: false,
            stylus_has_force: false,
            stylus_mode: false,
            paper_rect: None,
            cur_pressure: 0.0,
            last_up_pressure: 0.0,
            last_source: "Test",
        }
    }

    #[test]
    fn test_app_save_and_load_roundtrip() {
        let ctx = egui::Context::default();
        let mut app = new_test_app(&ctx);
        let paper = Rect::from_min_size(Pos2::new(20.0, 100.0), vec2(1160.0, 680.0));
        app.paper_rect = Some(paper);
        app.add_sample_strokes(10);
        assert_eq!(app.doc.strokes().len(), 10);
        assert!(app.doc.is_dirty());

        let temp_file = std::env::temp_dir().join("test_app_roundtrip.inknote");
        app.doc.to_file().save_to_path(&temp_file).unwrap();
        app.doc.mark_saved();
        assert!(!app.doc.is_dirty());

        let mut app2 = new_test_app(&ctx);
        app2.load_document_from(temp_file.clone());
        assert_eq!(app2.doc.strokes().len(), 10);
        assert_eq!(app2.batch_cache.len(), 10);
        assert!(!app2.doc.is_dirty());

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_paper_settings_persistence_and_switching() {
        let ctx = egui::Context::default();
        let mut app = new_test_app(&ctx);

        // Đổi sang Grid + bật Margins 2 bên + chuyển sang Paper A5 Landscape
        app.doc.paper_settings_mut().mode = CanvasMode::Paper;
        app.doc.paper_settings_mut().set_paper_size(PaperSize::A5);
        app.doc
            .paper_settings_mut()
            .set_orientation(PaperOrientation::Landscape);
        app.doc.paper_settings_mut().pattern = PaperPattern::Grid;
        app.doc.paper_settings_mut().show_margins = true;
        app.doc.paper_settings_mut().left_margin = 100.0;
        app.doc.paper_settings_mut().right_margin = 120.0;

        // Lưu ra file .inknote
        let temp_file = std::env::temp_dir().join("test_paper_settings.inknote");
        app.doc.to_file().save_to_path(&temp_file).unwrap();

        // Nạp vào app2: kiểu giấy, khổ A5 và lề 2 bên phải còn nguyên vẹn
        let mut app2 = new_test_app(&ctx);
        app2.load_document_from(temp_file.clone());
        assert_eq!(app2.doc.paper_settings().mode, CanvasMode::Paper);
        assert_eq!(app2.doc.paper_settings().paper_size, PaperSize::A5);
        assert_eq!(
            app2.doc.paper_settings().orientation,
            PaperOrientation::Landscape
        );
        assert_eq!(app2.doc.paper_settings().page_width, 840.0);
        assert_eq!(app2.doc.paper_settings().page_height, 594.0);
        assert_eq!(app2.doc.paper_settings().pattern, PaperPattern::Grid);
        assert!(app2.doc.paper_settings().show_margins);
        assert_eq!(app2.doc.paper_settings().left_margin, 100.0);
        assert_eq!(app2.doc.paper_settings().right_margin, 120.0);

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    #[ignore = "benchmark: chạy bằng cargo test -p inknote-app bench -- --ignored --nocapture"]
    fn bench_paint_2000_strokes() {
        let ctx = egui::Context::default();
        let mut app = new_test_app(&ctx);
        let paper = Rect::from_min_size(Pos2::new(20.0, 100.0), vec2(1160.0, 680.0));
        app.paper_rect = Some(paper);
        let t = Instant::now();
        app.add_sample_strokes(2000);
        println!("generate 2000: {:?}", t.elapsed());
        let verts: usize = app
            .batch_cache
            .values()
            .flat_map(|c| c.batches.iter())
            .map(|b| b.vertices.len())
            .sum();
        let tris: usize = app
            .batch_cache
            .values()
            .flat_map(|c| c.batches.iter())
            .map(|b| b.indices.len() / 3)
            .sum();
        println!("total vertices = {verts}, triangles = {tris}");
        for pass in 0..4 {
            let t_build = Instant::now();
            let mut build_dur = Duration::ZERO;
            let mut out = ctx.run_ui(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))),
                    time: Some(pass as f64 * 0.016),
                    ..Default::default()
                },
                |ui| {
                    let painter = ui.painter().with_clip_rect(paper);
                    let t0 = Instant::now();
                    draw_paper(
                        &painter,
                        paper,
                        paper.min,
                        &app.theme,
                        app.doc.paper_settings(),
                        app.view.zoom,
                    );
                    let t_dots = t0.elapsed();
                    let t1 = Instant::now();
                    let static_key = StaticKey::new(
                        app.doc.revision(),
                        app.view.zoom,
                        app.view.pan,
                        paper,
                        app.doc.page_count(),
                        app.doc.paper_settings().mode,
                    );
                    let static_meshes = match &app.static_cache {
                        Some((k, meshes)) if *k == static_key => meshes.clone(),
                        _ => {
                            let meshes = app.build_static_meshes(paper);
                            app.static_cache = Some((static_key, meshes.clone()));
                            meshes
                        }
                    };
                    for (_, mesh) in static_meshes {
                        painter.add(egui::Shape::Mesh(mesh));
                    }
                    build_dur = t1.elapsed();
                    println!("  pass {pass} -> dots: {t_dots:?}, mesh build: {build_dur:?}");
                },
            );
            let t_tess = Instant::now();
            out.textures_delta.clear();
            let prims = ctx.tessellate(out.shapes, out.pixels_per_point);
            println!(
                "pass {pass}: strokes build {:?}, tessellate {:?}, prims {}, total {:?}",
                build_dur,
                t_tess.elapsed(),
                prims.len(),
                t_build.elapsed()
            );
        }
    }

    #[test]
    #[ignore = "benchmark"]
    fn bench_active_stroke() {
        let ctx = egui::Context::default();
        let mut app = new_test_app(&ctx);
        let paper = Rect::from_min_size(Pos2::new(20.0, 100.0), vec2(1160.0, 680.0));
        app.paper_rect = Some(paper);
        let start = Instant::now();
        for k in 0..2000u32 {
            let t = k as f32 * 0.005;
            let pos = Pos2::new(
                300.0 + 200.0 * (t * 2.0).cos() + t * 10.0,
                400.0 + 150.0 * (t * 3.0).sin(),
            );
            let at = start + Duration::from_secs_f64(k as f64 * 0.005);
            app.pen_down(pos, 0.5, at);
            if k % 250 == 249 {
                let a = app.active.as_ref().unwrap();
                let t0 = Instant::now();
                if a.path.segments.len() > a.committed_segs {
                    let tail_start = if a.committed_segs == 0 {
                        a.path.start
                    } else {
                        a.path.segments[a.committed_segs - 1].end()
                    };
                    let tail_path = PenPath::new_w_segments(
                        tail_start,
                        a.path.segments[a.committed_segs..].to_vec(),
                    );
                    app.render_cx.clear_batches();
                    tail_path.draw_composed(&mut app.render_cx, &a.style);
                }
                app.render_cx.clear_batches();
                a.builder.draw_styled(&mut app.render_cx, &a.style, 1.0);
                let live_draw_time = t0.elapsed();
                println!(
                    "events {}: segs {}, committed {}, live_draw_time {:?}",
                    k + 1,
                    a.path.segments.len(),
                    a.committed_segs,
                    live_draw_time
                );
            }
        }
    }

    #[test]
    fn sample_2000_strokes_one_undo_and_fast_culling() {
        let ctx = egui::Context::default();
        let mut app = new_test_app(&ctx);
        app.paper_rect = Some(Rect::from_min_size(
            Pos2::new(20.0, 100.0),
            vec2(1160.0, 680.0),
        ));
        app.add_sample_strokes(2000);
        assert_eq!(app.doc.strokes().len(), 2000);
        // Ở zoom 300%, chỉ 1 phần nhỏ nét nằm trong màn hình
        let paper = app.paper_rect.unwrap();
        app.view.zoom_at(paper, paper.center(), 3.0);
        let visible = app
            .doc
            .strokes()
            .iter()
            .filter(|s| {
                let c = &app.batch_cache[&s.id];
                app.view.rect_to_screen(paper, c.bbox).intersects(paper)
            })
            .count();
        assert!(visible < 2000 / 2, "visible = {visible}");
        app.doc.undo();
        assert!(app.doc.strokes().is_empty());
    }

    #[test]
    fn verify_toolbar_and_canvas_alignment_at_required_resolutions() {
        let resolutions = [(1200.0, 800.0), (1366.0, 768.0), (1600.0, 900.0)];

        for (w, h) in resolutions {
            let ctx = egui::Context::default();
            let mut app = new_test_app(&ctx);

            // Chạy 3 pass để cơ chế đo kích thước tự động (show_centered_in_bounds) hội tụ hoàn toàn
            for pass in 0..3 {
                let raw_input = RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(w, h))),
                    time: Some(pass as f64 * 0.016),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw_input, |ui| {
                    let wacom_status = "Test Stylus";
                    let mut paper_settings = app.doc.paper_settings().clone();
                    let file_status = crate::autosave::FileStatusInfo {
                        file_name: "VeryLongLectureNotesFileName2026.inknote",
                        is_saved: true,
                        has_path: true,
                        is_draft_saved: false,
                        autosaved_notice: Some("Auto-saved"),
                        has_error: false,
                    };
                    let _ = show_toolbars(
                        ui,
                        &app.theme,
                        &mut app.toolbar_state,
                        &mut app.brush_config,
                        &mut paper_settings,
                        &file_status,
                        (1, 1),
                        wacom_status,
                    );
                    let workspace_frame =
                        egui::Frame::central_panel(ui.style()).fill(app.theme.workspace_bg);
                    egui::CentralPanel::default()
                        .frame(workspace_frame)
                        .show(ui, |ui| {
                            let avail = ui.max_rect();
                            app.paper_rect = Some(avail.shrink(app.theme.paper_margin));
                        });
                });
                out.textures_delta.clear();
            }

            let m = app.toolbar_state.last_metrics;
            let win_center_x = w * 0.5;

            // 1. Nhóm Trái căn sát lề trái (margin = 12px)
            assert!(
                (m.nav_left_rect.left() - 12.0).abs() <= 1.5,
                "[{w}x{h}] nav_left_rect.left() = {} (expected ~12.0)",
                m.nav_left_rect.left()
            );

            // 2. Nhóm Phải căn sát lề phải (w - 12px)
            assert!(
                (m.nav_right_rect.right() - (w - 12.0)).abs() <= 1.5,
                "[{w}x{h}] nav_right_rect.right() = {} (expected ~{})",
                m.nav_right_rect.right(),
                w - 12.0
            );

            // 3. Tâm Nhóm Giữa của Navigation Toolbar trùng tâm cửa sổ
            assert!(
                (m.nav_center_rect.center().x - win_center_x).abs() <= 1.0,
                "[{w}x{h}] nav_center_rect.center().x = {} != win_center_x = {}",
                m.nav_center_rect.center().x,
                win_center_x
            );

            // 4. Ba nhóm trên Navigation Toolbar cùng trục dọc và không chồng lấn
            assert!(
                (m.nav_left_rect.center().y - m.nav_center_rect.center().y).abs() <= 1.0
                    && (m.nav_right_rect.center().y - m.nav_center_rect.center().y).abs() <= 1.0,
                "[{w}x{h}] nav groups vertical centers misaligned: L={}, C={}, R={}",
                m.nav_left_rect.center().y,
                m.nav_center_rect.center().y,
                m.nav_right_rect.center().y
            );
            assert!(
                m.nav_left_rect.right() < m.nav_center_rect.left()
                    && m.nav_center_rect.right() < m.nav_right_rect.left(),
                "[{w}x{h}] nav groups overlap: L={:?}, C={:?}, R={:?}",
                m.nav_left_rect,
                m.nav_center_rect,
                m.nav_right_rect
            );

            // 5. Tâm của Thanh tuỳ chọn công cụ dạng pill (Contextual Tool Options) trùng tâm cửa sổ
            assert!(
                (m.tool_pill_rect.center().x - win_center_x).abs() <= 1.0,
                "[{w}x{h}] tool_pill_rect.center().x = {} != win_center_x = {} (pill_rect={:?})",
                m.tool_pill_rect.center().x,
                win_center_x,
                m.tool_pill_rect
            );
            assert!(
                m.tool_pill_rect.width() < w - 40.0 && m.tool_pill_rect.width() > 300.0,
                "[{w}x{h}] unexpected tool_pill_rect width: {}",
                m.tool_pill_rect.width()
            );

            // 6. Canvas giấy chấm bắt đầu ngay dưới 2 thanh công cụ và chiếm diện tích còn lại
            let paper = app.paper_rect.expect("paper_rect must be set");
            assert!(
                paper.top() > m.tool_strip_rect.bottom(),
                "[{w}x{h}] paper_rect.top()={} should be below tool_strip_rect.bottom()={}",
                paper.top(),
                m.tool_strip_rect.bottom()
            );
            assert!(
                (paper.center().x - win_center_x).abs() <= 1.0,
                "[{w}x{h}] paper_rect.center().x = {} != win_center_x = {}",
                paper.center().x,
                win_center_x
            );
        }
    }

    #[test]
    fn test_multipage_scroll_and_add_page_after_current() {
        let ctx = egui::Context::default();
        let mut app = new_test_app(&ctx);
        let paper = Rect::from_min_size(Pos2::new(20.0, 100.0), vec2(1160.0, 680.0));
        app.paper_rect = Some(paper);
        app.scroll_to_page(0);

        // Vẽ 1 nét lên trang 0, rồi thêm 4 trang thành tổng 5 trang (mỗi trang 1 số nét khác nhau)
        app.add_sample_strokes(1);
        for k in 1..=4 {
            app.add_page_after_current();
            assert_eq!(app.doc.active_page_index(), k);
            app.add_sample_strokes(k + 1);
        }
        assert_eq!(app.doc.page_count(), 5);
        for k in 0..5 {
            assert_eq!(app.doc.page_strokes(k).len(), k + 1);
        }

        // Cuộn về trang 1 -> tâm màn hình nhận diện đúng trang 1
        app.scroll_to_page(1);
        app.update_active_page_from_viewport(paper);
        assert_eq!(app.doc.active_page_index(), 1);

        // Bấm Add Page khi đang ở trang 1 -> trang mới chèn ngay sau trang 1 (tại index 2)
        app.add_page_after_current();
        assert_eq!(app.doc.page_count(), 6);
        assert_eq!(app.doc.active_page_index(), 2);
        assert_eq!(app.doc.page_strokes(2).len(), 0);
        assert_eq!(app.doc.page_strokes(3).len(), 3);

        // Xoá trang mới chèn -> về lại 5 trang và Undo khôi phục lại 6 trang
        let active_idx = app.doc.active_page_index();
        app.delete_page_at(active_idx);
        assert_eq!(app.doc.page_count(), 5);
        assert!(app.doc.undo());
        assert_eq!(app.doc.page_count(), 6);
    }

    #[test]
    fn test_thumbnail_panel_render_and_page_selection() {
        let ctx = egui::Context::default();
        let mut app = new_test_app(&ctx);
        app.toolbar_state.show_thumbnails = true;

        // Tạo 10 trang, mỗi trang có số nét khác nhau (trang i có i+1 nét)
        for idx in 0..10 {
            if idx > 0 {
                app.add_page_after_current();
            }
            app.add_sample_strokes(idx + 1);
        }
        assert_eq!(app.doc.page_count(), 10);

        // Chạy UI pass với sidebar thumbnail đang mở -> build cache thumbnail không lỗi
        let mut out = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0))),
                ..Default::default()
            },
            |ui| {
                let panel_frame = egui::Frame::new().fill(app.theme.thumb_panel_bg);
                egui::Panel::left("test_thumb_panel")
                    .resizable(false)
                    .exact_size(app.theme.thumb_panel_width)
                    .frame(panel_frame)
                    .show(ui, |ui| {
                        let act = show_thumbnail_panel(
                            ui,
                            &app.theme,
                            &app.doc,
                            &app.batch_cache,
                            &mut app.thumb_cache,
                        );
                        assert_eq!(act, ThumbnailAction::None);
                    });
                egui::CentralPanel::default().show(ui, |ui| {
                    app.paper_rect = Some(ui.max_rect());
                });
            },
        );
        out.textures_delta.clear();

        // Nhảy tới trang thứ 4 (index 3) như thao tác bấm thumbnail
        let paper = app.paper_rect.unwrap();
        app.doc.set_active_page_index(3);
        app.scroll_to_page(3);
        app.update_active_page_from_viewport(paper);
        assert_eq!(app.doc.active_page_index(), 3);

        // Xoá trang index 7 và kiểm tra số trang giảm còn 9 + Undo khôi phục về 10
        app.delete_page_at(7);
        assert_eq!(app.doc.page_count(), 9);
        assert!(app.doc.undo());
        assert_eq!(app.doc.page_count(), 10);
    }

    #[test]
    fn test_pull_to_add_page_trigger_requires_strong_pull() {
        let ctx = egui::Context::default();
        let mut app = new_test_app(&ctx);
        let paper = Rect::from_min_size(Pos2::new(20.0, 100.0), vec2(1160.0, 680.0));
        app.paper_rect = Some(paper);
        app.doc.paper_settings_mut().pattern = PaperPattern::Grid;

        // Kéo view xuống chạm sát đáy trang 0
        let settings = app.doc.paper_settings();
        let bottom_keep = 140.0_f32.min(paper.height() * 0.45);
        let min_pan_y = (paper.height() - bottom_keep - settings.page_height).min(24.0);
        app.view.pan = vec2(160.0, min_pan_y);
        let t0 = Instant::now();
        app.pull_cooldown_until = t0;

        // Cuộn nhẹ 1 chút -> chưa tạo trang
        assert!(!app.apply_pan_limits_and_pull_trigger(paper, -45.0, t0));
        assert_eq!(app.doc.page_count(), 1);
        assert!(app.pull_overscroll > 0.0 && app.pull_overscroll < PULL_TO_ADD_THRESHOLD);

        // Giật mạnh thêm quá ngưỡng -> tự động thêm trang 2 và giữ nguyên cài đặt giấy
        assert!(app.apply_pan_limits_and_pull_trigger(
            paper,
            -(PULL_TO_ADD_THRESHOLD),
            t0 + Duration::from_millis(40),
        ));
        assert_eq!(app.doc.page_count(), 2);
        assert_eq!(app.doc.active_page_index(), 1);
        assert_eq!(app.doc.paper_settings().pattern, PaperPattern::Grid);
        assert_eq!(app.pull_overscroll, 0.0);

        // Ngay sau đó nếu vẫn còn quán tính cuộn -> cooldown ngăn tạo trang thứ 3 ngoài ý muốn
        assert!(!app.apply_pan_limits_and_pull_trigger(
            paper,
            -200.0,
            t0 + Duration::from_millis(100),
        ));
        assert_eq!(app.doc.page_count(), 2);
    }
}
