// Ứng dụng InkNote (Bước 1.6 – Khung giao diện P0 theo docs/04-ui-spec.md):
// - Khu vực 1: Thanh công cụ chính (Navigation Toolbar, nền xanh dương, nút đang chọn có ô nền sáng bo góc)
// - Khu vực 2: Thanh tuỳ chọn công cụ (Contextual Tool Options Bar, nền tối bo tròn + Pop-up tối bo góc)
// - Khu vực 3: Canvas chính với nền giấy chấm (Dotted Paper) và nét bút Rnote (`inknote-pen` + `inknote-render`)

mod icons;
mod theme;
mod toolbar;

use std::collections::HashSet;
use std::time::{Duration, Instant};

use eframe::egui;
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
    icons::draw_dotted_paper,
    theme::Theme,
    toolbar::{ActiveTool, ToolbarState, show_toolbars},
};

struct FinishedStroke {
    #[allow(dead_code)]
    path: PenPath,
    #[allow(dead_code)]
    style: Style,
    batches: Vec<DrawBatch>,
}

struct ActiveStroke {
    path: PenPath,
    style: Style,
    builder: Box<dyn Buildable<Emit = Segment>>,
}

struct InkNoteApp {
    theme: Theme,
    toolbar_state: ToolbarState,
    brush_config: BrushConfig,
    strokes: Vec<FinishedStroke>,
    active: Option<ActiveStroke>,
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
        Self {
            theme: Theme::default(),
            toolbar_state: ToolbarState::default(),
            brush_config: BrushConfig::default(),
            strokes: Vec::new(),
            active: None,
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
        if egui::Popup::is_any_open(ctx) {
            return false;
        }
        self.paper_rect.is_some_and(|r| r.contains(pos))
    }

    fn pen_down(&mut self, pos: egui::Pos2, pressure: f64, now: Instant) {
        self.cur_pressure = pressure;
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
                }
            }
        } else {
            self.brush_config.new_style_seeds();
            let style = self.brush_config.style_for_current_options();
            let path = PenPath::new(element);
            let builder = new_builder(self.brush_config.builder_type, element, now);
            self.active = Some(ActiveStroke {
                path,
                style,
                builder,
            });
        }
    }

    fn pen_up(&mut self, pos: egui::Pos2, pressure: f64, now: Instant) {
        self.cur_pressure = pressure;
        self.last_up_pressure = pressure;
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
            active.path.draw_composed(&mut self.render_cx, &active.style);
            let batches = std::mem::take(&mut self.render_cx.batches);
            self.strokes.push(FinishedStroke {
                path: active.path,
                style: active.style,
                batches,
            });
        }
    }

    fn paint_batches(painter: &egui::Painter, batches: &[DrawBatch]) {
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
                    pos: egui::pos2(x, y),
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

        // Khu vực 1 & 2: Navigation Toolbar + Contextual Tool Options Bar
        let wacom_status = format!(
            "{} | p={:.3} | up_p={:.3} | strokes={}",
            self.last_source,
            self.cur_pressure,
            self.last_up_pressure,
            self.strokes.len()
        );
        let actions = show_toolbars(
            ui,
            &self.theme,
            &mut self.toolbar_state,
            &mut self.brush_config,
            &wacom_status,
        );
        if actions.undo_clicked {
            self.strokes.pop();
        }
        if actions.clear_clicked {
            self.strokes.clear();
        }

        // Phím tắt Ctrl+Z
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
            self.strokes.pop();
        }

        // Khu vực 3: Main Canvas (trang giấy chấm Dotted Paper)
        let workspace_frame =
            egui::Frame::central_panel(ui.style()).fill(self.theme.workspace_bg);
        egui::CentralPanel::default()
            .frame(workspace_frame)
            .show(ui, |ui| {
                let avail = ui.max_rect();
                let paper_rect = avail.shrink(self.theme.paper_margin);
                self.paper_rect = Some(paper_rect);

                let painter = ui.painter().with_clip_rect(paper_rect);

                // 1. Vẽ nền giấy trắng có chấm nhỏ đều nhau (Dotted Paper)
                draw_dotted_paper(
                    &painter,
                    paper_rect,
                    self.theme.paper_bg,
                    self.theme.paper_border,
                    self.theme.paper_dot_color,
                    self.theme.paper_dot_spacing,
                    self.theme.paper_dot_radius,
                );

                // 2. Vẽ các nét bút đã hoàn thành
                for s in &self.strokes {
                    Self::paint_batches(&painter, &s.batches);
                }

                // 3. Vẽ nét bút đang kéo (active)
                if let Some(active) = &self.active {
                    self.render_cx.clear_batches();
                    active.path.draw_composed(&mut self.render_cx, &active.style);
                    active
                        .builder
                        .draw_styled(&mut self.render_cx, &active.style, 1.0);
                    Self::paint_batches(&painter, &self.render_cx.batches);
                }
            });

        ctx.request_repaint();
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("InkNote")
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "InkNote",
        options,
        Box::new(|cc| Ok(Box::new(InkNoteApp::new(cc)))),
    )
}
