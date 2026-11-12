// Ứng dụng InkNote (Bước 1.1 - 1.4): sử dụng trực tiếp code vẽ nét của Rnote từ `inknote-pen`
// và bộ chia tam giác `lyon` từ `inknote-render`.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use eframe::egui;
use inknote_pen::{
    Constraints, PenEvent, PenPath, Style,
    brushconfig::{BrushConfig, BrushStyle},
    builders::{
        PenPathBuilderType, PenPathCurvedBuilder, PenPathModeledBuilder, PenPathSimpleBuilder,
        buildable::{Buildable, BuilderCreator, BuilderProgress},
    },
    penpath::{Element, Segment},
    style::{Composer, PressureCurve},
};
use inknote_render::{CairoRenderContext, DrawBatch};
use parry2d_f64::math::Vector2;

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
    cur_pressure: f64,
    last_up_pressure: f64,
    last_source: &'static str,
}

impl Default for InkNoteApp {
    fn default() -> Self {
        Self {
            brush_config: BrushConfig::default(),
            strokes: Vec::new(),
            active: None,
            render_cx: CairoRenderContext::new(),
            base_instant: Instant::now(),
            prev_frame_t: 0.0,
            last_event_secs: 0.0,
            touch_active: false,
            stylus_has_force: false,
            cur_pressure: 0.0,
            last_up_pressure: 0.0,
            last_source: "(chưa có)",
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

    fn pen_down(&mut self, pos: egui::Pos2, pressure: f64, now: Instant) {
        self.cur_pressure = pressure;
        let element = Element::new(Vector2::new(pos.x as f64, pos.y as f64), pressure);

        if let Some(active) = &mut self.active {
            // Đang trong nét vẽ: giống hệt Rnote (rnote-engine/src/pens/brush.rs),
            // sự kiện di chuyển khi đang nhấn bút vẫn là PenEvent::Down!
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
            // Bắt đầu nét mới
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
                    match phase {
                        egui::TouchPhase::Start => {
                            self.touch_active = true;
                            self.stylus_has_force = force.is_some();
                            let p = match force {
                                Some(f) => (f as f64).clamp(0.0, 1.0),
                                None => Element::PRESSURE_DEFAULT,
                            };
                            self.last_source = if force.is_some() {
                                "BÚT WACOM (có force)"
                            } else {
                                "CẢM ỨNG (không force)"
                            };
                            let now = self.monotonic_instant(ev_secs);
                            self.pen_down(pos, p, now);
                        }
                        egui::TouchPhase::Move => {
                            if force.is_some() {
                                self.stylus_has_force = true;
                            }
                            let p = match force {
                                Some(f) => (f as f64).clamp(0.0, 1.0),
                                None => {
                                    if self.stylus_has_force {
                                        self.cur_pressure
                                    } else {
                                        Element::PRESSURE_DEFAULT
                                    }
                                }
                            };
                            let now = self.monotonic_instant(ev_secs);
                            self.pen_down(pos, p, now);
                        }
                        egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                            // Khi nhấc bút cảm ứng khỏi mặt bảng, nếu hệ điều hành báo force: None
                            // thì áp lực tại thời điểm rời mặt bảng là 0.0 (giống trục Pressure của GDK/Rnote khi nhấc bút),
                            // không phải PRESSURE_DEFAULT (0.5) của chuột.
                            let p = match force {
                                Some(f) => (f as f64).clamp(0.0, 1.0),
                                None => {
                                    if self.stylus_has_force {
                                        0.0
                                    } else {
                                        Element::PRESSURE_DEFAULT
                                    }
                                }
                            };
                            let now = self.monotonic_instant(ev_secs);
                            self.pen_up(pos, p, now);
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
                } if !self.touch_active => {
                    self.last_source = "CHUỘT (PRESSURE_DEFAULT = 0.5)";
                    let now = self.monotonic_instant(ev_secs);
                    if pressed {
                        self.pen_down(pos, Element::PRESSURE_DEFAULT, now);
                    } else if self.active.is_some() {
                        self.pen_up(pos, Element::PRESSURE_DEFAULT, now);
                    }
                }
                egui::Event::PointerMoved(pos) if !self.touch_active => {
                    if self.active.is_some() {
                        let now = self.monotonic_instant(ev_secs);
                        self.pen_down(pos, Element::PRESSURE_DEFAULT, now);
                    }
                }
                _ => {}
            }
        }
        self.prev_frame_t = frame_t;

        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Kiểu bút (BrushStyle):");
                ui.selectable_value(&mut self.brush_config.style, BrushStyle::Solid, "Solid");
                ui.selectable_value(&mut self.brush_config.style, BrushStyle::Marker, "Marker");
                ui.selectable_value(
                    &mut self.brush_config.style,
                    BrushStyle::Textured,
                    "Textured",
                );

                ui.separator();
                ui.label("Bộ dựng nét (Builder):");
                for (b, label) in [
                    (PenPathBuilderType::Modeled, "Modeled (mặc định Rnote)"),
                    (PenPathBuilderType::Curved, "Curved"),
                    (PenPathBuilderType::Simple, "Simple"),
                ] {
                    if ui
                        .selectable_label(
                            std::mem::discriminant(&self.brush_config.builder_type)
                                == std::mem::discriminant(&b),
                            label,
                        )
                        .clicked()
                    {
                        self.brush_config.builder_type = b;
                    }
                }

                ui.separator();
                match self.brush_config.style {
                    BrushStyle::Solid => {
                        ui.label("Cỡ bút Solid:");
                        ui.add(
                            egui::Slider::new(&mut self.brush_config.solid_options.stroke_width, 0.5..=20.0)
                                .text("px"),
                        );
                        ui.label("Đường cong lực (PressureCurve):");
                        egui::ComboBox::from_id_salt("pcurve")
                            .selected_text(format!("{:?}", self.brush_config.solid_options.pressure_curve))
                            .show_ui(ui, |ui| {
                                for c in [
                                    PressureCurve::Const,
                                    PressureCurve::Linear,
                                    PressureCurve::Sqrt,
                                    PressureCurve::Cbrt,
                                    PressureCurve::Pow2,
                                    PressureCurve::Pow3,
                                ] {
                                    if ui
                                        .selectable_label(
                                            std::mem::discriminant(
                                                &self.brush_config.solid_options.pressure_curve,
                                            ) == std::mem::discriminant(&c),
                                            format!("{c:?}"),
                                        )
                                        .clicked()
                                    {
                                        self.brush_config.solid_options.pressure_curve = c;
                                    }
                                }
                            });
                    }
                    BrushStyle::Marker => {
                        ui.label("Cỡ bút Marker:");
                        ui.add(
                            egui::Slider::new(&mut self.brush_config.marker_options.stroke_width, 2.0..=40.0)
                                .text("px"),
                        );
                    }
                    BrushStyle::Textured => {
                        ui.label("Cỡ bút Textured:");
                        ui.add(
                            egui::Slider::new(&mut self.brush_config.textured_options.stroke_width, 1.0..=30.0)
                                .text("px"),
                        );
                    }
                }

                ui.separator();
                if ui.button("Undo (Ctrl+Z)").clicked() {
                    self.strokes.pop();
                }
                if ui.button("Xoá hết").clicked() {
                    self.strokes.clear();
                }
            });

            ui.horizontal(|ui| {
                ui.label(format!(
                    "Nguồn: {} | pressure hiện tại: {:.3} | pressure lúc nhấc bút cuối: {:.3} | số nét: {}",
                    self.last_source,
                    self.cur_pressure,
                    self.last_up_pressure,
                    self.strokes.len()
                ));
            });
        });

        // Phím tắt Ctrl+Z để Undo
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
            self.strokes.pop();
        }

        let central_frame = egui::Frame::central_panel(ui.style()).fill(egui::Color32::WHITE);
        egui::CentralPanel::default().frame(central_frame).show(ui, |ui| {
            let painter = ui.painter();

            // 1. Vẽ các nét đã hoàn thành
            for s in &self.strokes {
                Self::paint_batches(painter, &s.batches);
            }

            // 2. Vẽ nét đang kéo (active): phần đã emit + phần dự đoán trong builder (giống Rnote)
            if let Some(active) = &self.active {
                self.render_cx.clear_batches();
                active.path.draw_composed(&mut self.render_cx, &active.style);
                active
                    .builder
                    .draw_styled(&mut self.render_cx, &active.style, 1.0);
                Self::paint_batches(painter, &self.render_cx.batches);
            }
        });

        ctx.request_repaint();
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("InkNote - Nét bút Rnote (GPLv3)")
            .with_inner_size([1100.0, 750.0]),
        ..Default::default()
    };
    eframe::run_native(
        "InkNote - Nét bút Rnote (GPLv3)",
        options,
        Box::new(|_cc| Ok(Box::<InkNoteApp>::default())),
    )
}
