// Ứng dụng InkNote (Bước 1.6 – Khung giao diện P0 theo docs/04-ui-spec.md):
// - Khu vực 1: Thanh công cụ chính (Navigation Toolbar, nền xanh dương, nút đang chọn có ô nền sáng bo góc)
// - Khu vực 2: Thanh tuỳ chọn công cụ (Contextual Tool Options Bar, nền tối bo tròn + Pop-up tối bo góc)
// - Khu vực 3: Canvas chính với nền giấy chấm (Dotted Paper) và nét bút Rnote (`inknote-pen` + `inknote-render`)

mod icons;
mod theme;
mod toolbar;
mod view;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui;
use inknote_core::{CanvasMode, Command, Document, DocumentFile, PaperSettings, Stroke};
use inknote_pen::{
    Constraints, PenEvent, PenPath, Style,
    brushconfig::BrushConfig,
    builders::{
        PenPathBuilderType, PenPathCurvedBuilder, PenPathModeledBuilder, PenPathSimpleBuilder,
        buildable::{Buildable, BuilderCreator, BuilderProgress},
    },
    penpath::{Element, Segment},
    style::Composer,
};
use inknote_render::{CairoRenderContext, DrawBatch};
use parry2d_f64::math::Vector2;

use crate::{
    icons::{draw_paper, paper_sheet_rect},
    theme::Theme,
    toolbar::{ActiveTool, ToolbarState, show_toolbars},
    view::View,
};

/// Lưới tam giác của 1 nét (toạ độ TRANG) + hộp bao để bỏ qua nét nằm ngoài màn hình.
struct CachedStroke {
    batches: Vec<DrawBatch>,
    bbox: egui::Rect,
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

/// Khoá bộ nhớ đệm lưới tĩnh: (revision trang, zoom, pan, vùng giấy).
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
}

impl StaticKey {
    fn new(rev: u64, zoom: f32, pan: egui::Vec2, paper: egui::Rect) -> Self {
        Self {
            rev,
            zoom_bits: zoom.to_bits(),
            pan_x_bits: pan.x.to_bits(),
            pan_y_bits: pan.y.to_bits(),
            paper_min_x: paper.min.x.to_bits(),
            paper_min_y: paper.min.y.to_bits(),
            paper_max_x: paper.max.x.to_bits(),
            paper_max_y: paper.max.y.to_bits(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnsavedAction {
    NewNote,
    Open,
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
    // Bộ nhớ đệm lưới tam giác của từng nét (khoá = id nét), không thuộc dữ liệu trang
    batch_cache: HashMap<u64, CachedStroke>,
    view: View,
    confirm_clear: bool,
    paint_ms: f32,
    fps: f32,
    last_frame: Instant,
    active: Option<ActiveStroke>,
    static_cache: Option<(StaticKey, std::sync::Arc<egui::epaint::Mesh>)>,
    render_cx: CairoRenderContext,

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
        let mut doc = Document::new();
        *doc.paper_settings_mut() = default_paper.clone();
        Self {
            theme: Theme::default(),
            toolbar_state: ToolbarState::default(),
            brush_config: BrushConfig::default(),
            doc,
            default_paper_settings: default_paper,
            current_path: None,
            confirm_unsaved: None,
            batch_cache: HashMap::new(),
            view: View::default(),
            confirm_clear: false,
            paint_ms: 0.0,
            fps: 0.0,
            last_frame: Instant::now(),
            active: None,
            static_cache: None,
            render_cx: CairoRenderContext::new(),
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
        }
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

    fn can_draw_at(&self, ctx: &egui::Context, pos: egui::Pos2) -> bool {
        if self.toolbar_state.read_only || self.toolbar_state.active_tool != ActiveTool::Pen {
            return false;
        }
        if egui::Popup::is_any_open(ctx) || self.confirm_clear || self.confirm_unsaved.is_some() {
            return false;
        }
        // Giữ Space để kéo trang bằng chuột/bút thay vì vẽ
        if ctx.input(|i| i.key_down(egui::Key::Space)) {
            return false;
        }
        self.drawable_rect().is_some_and(|r| r.contains(pos))
    }

    /// Khung chữ nhật có thể viết vẽ trên màn hình:
    /// - Ở chế độ Paper: chính là diện tích tờ giấy (không vẽ lên mặt bàn xám bên ngoài).
    /// - Ở chế độ Whiteboard: là toàn bộ vùng làm việc.
    fn drawable_rect(&self) -> Option<egui::Rect> {
        let workspace = self.paper_rect?;
        let origin = self.view.origin(workspace);
        match self.doc.paper_settings().mode {
            CanvasMode::Paper => Some(paper_sheet_rect(
                origin,
                self.doc.paper_settings(),
                self.view.zoom,
            )),
            CanvasMode::Whiteboard => Some(workspace),
        }
    }

    /// Đổi toạ độ màn hình → toạ độ trang (theo zoom/kéo hiện tại).
    fn to_page(&self, pos: egui::Pos2) -> egui::Pos2 {
        match self.paper_rect {
            Some(r) => self.view.to_page(r, pos),
            None => pos,
        }
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

    /// Lưu ghi chú (nếu đã có đường dẫn thì lưu thẳng, chưa có thì mở hộp thoại Save As).
    fn save_document(&mut self) {
        if let Some(path) = self.current_path.clone() {
            if let Err(e) = self.doc.to_file().save_to_path(&path) {
                eprintln!("Lỗi khi lưu file {:?}: {e}", path);
            } else {
                self.doc.mark_saved();
            }
        } else {
            self.save_document_as();
        }
    }

    /// Mở hộp thoại chọn nơi lưu file .inknote mới.
    fn save_document_as(&mut self) {
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
            } else {
                self.doc.mark_saved();
                self.current_path = Some(path);
            }
        }
    }

    /// Nạp dữ liệu từ file .inknote trên đĩa.
    fn load_document_from(&mut self, path: PathBuf) {
        match DocumentFile::load_from_path(&path) {
            Ok(file) => {
                self.doc = Document::from_file(file);
                self.current_path = Some(path);
                self.batch_cache.clear();
                self.static_cache = None;
                self.active = None;
                // Tái tạo lại batch_cache cho các nét vừa nạp
                for s in self.doc.strokes() {
                    self.render_cx.clear_batches();
                    s.path.draw_composed(&mut self.render_cx, &s.style);
                    let batches = std::mem::take(&mut self.render_cx.batches);
                    self.batch_cache.insert(s.id, CachedStroke::new(batches));
                }
            }
            Err(e) => {
                eprintln!("Lỗi khi mở file: {e}");
            }
        }
    }

    /// Bắt đầu quy trình mở file (hỏi xác nhận nếu có thay đổi chưa lưu).
    fn trigger_open(&mut self) {
        if self.doc.is_dirty() {
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
        if self.doc.is_dirty() {
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
        self.batch_cache.clear();
        self.static_cache = None;
        self.active = None;
    }

    fn pen_down(&mut self, pos: egui::Pos2, pressure: f64, now: Instant) {
        self.cur_pressure = pressure;
        let pos = self.to_page(pos);
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

    /// Dựng lại lưới tĩnh của các nét đã xong (gộp thành 1 mesh, toạ độ màn hình).
    fn build_static_mesh(&self, paper: egui::Rect) -> egui::epaint::Mesh {
        let view = self.view;
        let origin = view.origin(paper);
        let mut n_verts = 0;
        let mut n_indices = 0;
        for s in self.doc.strokes() {
            if let Some(c) = self.batch_cache.get(&s.id) {
                if view.rect_to_screen(paper, c.bbox).intersects(paper) {
                    for b in &c.batches {
                        n_verts += b.vertices.len();
                        n_indices += b.indices.len();
                    }
                }
            }
        }
        let mut mesh = egui::epaint::Mesh::default();
        mesh.vertices.reserve(n_verts);
        mesh.indices.reserve(n_indices);
        for s in self.doc.strokes() {
            let Some(c) = self.batch_cache.get(&s.id) else {
                continue;
            };
            if !view.rect_to_screen(paper, c.bbox).intersects(paper) {
                continue;
            }
            for batch in &c.batches {
                if batch.indices.is_empty() || batch.vertices.is_empty() {
                    continue;
                }
                let [r, g, b, a] = batch.color;
                let col = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
                let base = mesh.vertices.len() as u32;
                mesh.vertices
                    .extend(batch.vertices.iter().map(|&[x, y]| egui::epaint::Vertex {
                        pos: egui::pos2(origin.x + x * view.zoom, origin.y + y * view.zoom),
                        uv: egui::epaint::WHITE_UV,
                        color: col,
                    }));
                mesh.indices.extend(batch.indices.iter().map(|i| i + base));
            }
        }
        mesh
    }

    fn pen_up(&mut self, pos: egui::Pos2, pressure: f64, now: Instant) {
        self.cur_pressure = pressure;
        self.last_up_pressure = pressure;
        let pos = self.to_page(pos);
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
        if let Some(active) = self.active.take() {
            self.render_cx.clear_batches();
            active
                .path
                .draw_composed(&mut self.render_cx, &active.style);
            let batches = std::mem::take(&mut self.render_cx.batches);
            let id = self.doc.alloc_id();
            self.batch_cache.insert(id, CachedStroke::new(batches));
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
                            if !self.can_draw_at(&ctx, pos) {
                                continue;
                            }
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
                    if pressed && self.can_draw_at(&ctx, pos) {
                        self.pen_down(pos, Element::PRESSURE_DEFAULT, now);
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

        // Cập nhật tiêu đề cửa sổ theo tên file hiện tại và trạng thái sửa đổi (*)
        let doc_title = match &self.current_path {
            Some(p) => p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("untitled.inknote"),
            None => self.doc.metadata().title.as_str(),
        };
        let dirty_mark = if self.doc.is_dirty() { " *" } else { "" };
        let full_title = format!("InkNote - {}{}", doc_title, dirty_mark);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(full_title));

        // Khu vực 1 & 2: Navigation Toolbar + Contextual Tool Options Bar
        let wacom_status = format!(
            "{} | p={:.3} | up_p={:.3} | strokes={}",
            self.last_source,
            self.cur_pressure,
            self.last_up_pressure,
            self.doc.strokes().len()
        );
        let mut paper_settings = self.doc.paper_settings().clone();
        let actions = show_toolbars(
            ui,
            &self.theme,
            &mut self.toolbar_state,
            &mut self.brush_config,
            &mut paper_settings,
            &wacom_status,
        );
        if paper_settings != *self.doc.paper_settings() {
            *self.doc.paper_settings_mut() = paper_settings.clone();
            self.default_paper_settings = paper_settings.clone();
            save_default_paper_settings(&paper_settings);
        }
        // Phím tắt: Ctrl+Z = Undo; Ctrl+Y hoặc Ctrl+Shift+Z = Redo
        // Phím tắt tệp: Ctrl+S = Save, Ctrl+Shift+S = Save As, Ctrl+O = Open, Ctrl+N = New
        let (key_undo, key_redo, key_save, key_save_as, key_open, key_new) = ctx.input(|i| {
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
            )
        });
        // Không hoàn tác / thao tác file khi đang kéo nét dở
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

        // Hộp xác nhận xoá trang (xoá xong vẫn Undo được)
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

        // Hộp xác nhận lưu thay đổi trước khi mở file khác hoặc tạo mới
        if let Some(action) = self.confirm_unsaved {
            let mut do_save = false;
            let mut do_discard = false;
            let mut cancel = false;
            #[allow(deprecated)]
            let modal =
                egui::Modal::new(egui::Id::new("confirm_unsaved_changes")).show(&ctx, |ui| {
                    ui.set_width(320.0);
                    ui.heading("Save changes?");
                    ui.add_space(4.0);
                    ui.label(
                    "You have unsaved changes in this note. Do you want to save before proceeding?",
                );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save & Continue").clicked() {
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
                self.save_document();
                self.confirm_unsaved = None;
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
                }
            } else if do_discard {
                self.confirm_unsaved = None;
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
                }
            } else if cancel || modal.should_close() {
                self.confirm_unsaved = None;
            }
        }

        // Zoom / kéo trang
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
                || self.confirm_unsaved.is_some();
            if !blocked {
                if keys.0 {
                    self.view.zoom = 1.0;
                    if self.doc.paper_settings().mode == CanvasMode::Paper {
                        let centered_x = ((paper.width() - self.doc.paper_settings().page_width).max(0.0)) * 0.5;
                        self.view.pan = egui::vec2(centered_x, 24.0);
                    } else {
                        self.view.pan = egui::Vec2::ZERO;
                    }
                }
                if keys.1 {
                    self.view.zoom_at(paper, paper.center(), 1.25);
                }
                if keys.2 {
                    self.view.zoom_at(paper, paper.center(), 0.8);
                }
                if let Some(h) = hover.filter(|h| paper.contains(*h)) {
                    if zoom_delta != 1.0 {
                        self.view.zoom_at(paper, h, zoom_delta);
                    }
                    self.view.pan += scroll;
                    if middle || space_drag {
                        self.view.pan += ptr_delta;
                    }
                }
            }
        }

        // F9: thêm 2000 nét mẫu để test hiệu năng
        if self.active.is_none() && ctx.input(|i| i.key_pressed(egui::Key::F9)) {
            self.add_sample_strokes(2000);
        }

        // FPS (trung bình trượt)
        let now = Instant::now();
        let real_dt = now.duration_since(self.last_frame).as_secs_f32().max(1e-4);
        self.last_frame = now;
        self.fps = if self.fps == 0.0 {
            1.0 / real_dt
        } else {
            self.fps * 0.9 + 0.1 / real_dt
        };

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
                let origin = view.origin(paper_rect);

                // 1. Tự động căn giữa trang ở chế độ Paper khi mở ghi chú mới (pan chưa đặt)
                if self.view.pan == egui::Vec2::ZERO && settings.mode == CanvasMode::Paper {
                    let centered_x =
                        ((paper_rect.width() - settings.page_width * view.zoom).max(0.0)) * 0.5;
                    self.view.pan = egui::vec2(centered_x, 24.0);
                }

                // 2. Vẽ nền giấy theo PaperSettings (Plain, Dotted, Grid, Lined + Margin lines)
                draw_paper(
                    ui.painter(),
                    paper_rect,
                    origin,
                    &self.theme,
                    settings,
                    view.zoom,
                );

                // 3. Vùng giới hạn nét vẽ (clip_rect):
                // - Paper: Giới hạn nghiêm ngặt trong tờ giấy (không vẽ lem ra bàn xám)
                // - Whiteboard: Tràn toàn bộ vùng làm việc
                let clip_rect = match settings.mode {
                    CanvasMode::Paper => {
                        let sheet = paper_sheet_rect(origin, settings, view.zoom);
                        sheet.intersect(paper_rect)
                    }
                    CanvasMode::Whiteboard => paper_rect,
                };
                let painter = ui.painter().with_clip_rect(clip_rect);
                let paint_start = Instant::now();

                // 4. Vẽ các nét bút đã hoàn thành (gộp 1 Mesh và cache theo StaticKey)
                let static_key =
                    StaticKey::new(self.doc.revision(), view.zoom, view.pan, paper_rect);
                let static_mesh = match &self.static_cache {
                    Some((k, mesh)) if *k == static_key => mesh.clone(),
                    _ => {
                        let mesh = std::sync::Arc::new(self.build_static_mesh(paper_rect));
                        self.static_cache = Some((static_key, mesh.clone()));
                        mesh
                    }
                };
                if !static_mesh.is_empty() {
                    painter.add(egui::Shape::Mesh(static_mesh));
                }

                // 5. Vẽ nét bút đang kéo (active)
                if let Some(active) = &self.active {
                    // 5a. Các đoạn nét đã chốt (đã tessellate sẵn)
                    if !active.committed_batches.is_empty() {
                        Self::paint_batches(&painter, &active.committed_batches, origin, view.zoom);
                    }

                    // 5b. Phần đuôi ngắn chưa chốt (< 16 đoạn)
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
                        Self::paint_batches(&painter, &self.render_cx.batches, origin, view.zoom);
                    }

                    // 5c. Đầu bút động và dự đoán nét từ Modeler
                    self.render_cx.clear_batches();
                    active
                        .builder
                        .draw_styled(&mut self.render_cx, &active.style, 1.0);
                    Self::paint_batches(&painter, &self.render_cx.batches, origin, view.zoom);
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

        ctx.request_repaint();
    }
}

fn main() -> eframe::Result {
    let mut wgpu_options = eframe::egui_wgpu::WgpuConfiguration::default();
    wgpu_options.surface.present_mode = eframe::egui_wgpu::wgpu::PresentMode::AutoNoVsync;

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
    use eframe::egui::{Pos2, RawInput, Rect, vec2};
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
            batch_cache: HashMap::new(),
            view: View::default(),
            confirm_clear: false,
            paint_ms: 0.0,
            fps: 0.0,
            last_frame: Instant::now(),
            active: None,
            static_cache: None,
            render_cx: CairoRenderContext::new(),
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
        app.doc.paper_settings_mut().set_orientation(PaperOrientation::Landscape);
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
        assert_eq!(app2.doc.paper_settings().orientation, PaperOrientation::Landscape);
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
                    let static_key =
                        StaticKey::new(app.doc.revision(), app.view.zoom, app.view.pan, paper);
                    let static_mesh = match &app.static_cache {
                        Some((k, mesh)) if *k == static_key => mesh.clone(),
                        _ => {
                            let mesh = std::sync::Arc::new(app.build_static_mesh(paper));
                            app.static_cache = Some((static_key, mesh.clone()));
                            mesh
                        }
                    };
                    if !static_mesh.is_empty() {
                        painter.add(egui::Shape::Mesh(static_mesh));
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
                    let _ = show_toolbars(
                        ui,
                        &app.theme,
                        &mut app.toolbar_state,
                        &mut app.brush_config,
                        &mut paper_settings,
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
}
