// SPIKE A1: egui - thu pressure but Wacom tren Windows.
// Ve bang but: net day/mong theo pressure, so pressure hien ngay tren man hinh.
// Co o "Lam muot kieu Rnote" de thu ink-stroke-modeler-rs (bai B).
use eframe::egui;
use ink_stroke_modeler_rs::{ModelerInput, ModelerInputEventType, ModelerParams, StrokeModeler};
use std::time::Instant;

struct Stroke {
    pts: Vec<(egui::Pos2, f32)>, // (vi tri, pressure 0..1)
}

struct App {
    strokes: Vec<Stroke>,
    drawing: bool,
    smooth: bool,
    modeler: StrokeModeler,
    stroke_start: Instant,
    // thong ke chan doan
    cur_pressure: f32,
    min_p: f32,
    max_p: f32,
    touch_events: u64,
    touch_with_force: u64,
    mouse_events: u64,
    last_source: &'static str,
    touch_active: bool,
    now_t: f64,        // gio (giay) cua su kien dang xu ly
    stroke_t0: f64,    // gio bat dau net
    last_t: f64,       // gio diem truoc (de dam bao tang dan)
    prev_frame_t: f64,
    modeler_errs: u64,
    last_err: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            strokes: vec![],
            drawing: false,
            smooth: false,
            modeler: StrokeModeler::default(),
            stroke_start: Instant::now(),
            cur_pressure: 0.0,
            min_p: f32::MAX,
            max_p: 0.0,
            touch_events: 0,
            touch_with_force: 0,
            mouse_events: 0,
            last_source: "(chua co)",
            touch_active: false,
            now_t: 0.0,
            stroke_t0: 0.0,
            last_t: 0.0,
            prev_frame_t: 0.0,
            modeler_errs: 0,
            last_err: String::new(),
        }
    }
}

fn params() -> ModelerParams {
    // Cung tham so voi Rnote
    ModelerParams {
        sampling_min_output_rate: 120.0,
        sampling_end_of_stroke_stopping_distance: 0.01,
        sampling_end_of_stroke_max_iterations: 20,
        sampling_max_outputs_per_call: 200,
        stylus_state_modeler_max_input_samples: 20,
        ..ModelerParams::suggested()
    }
}

impl App {
    fn begin(&mut self, pos: egui::Pos2, p: f32) {
        self.drawing = true;
        self.stroke_start = Instant::now();
        self.stroke_t0 = self.now_t;
        self.last_t = 0.0;
        self.strokes.push(Stroke { pts: vec![] });
        if self.smooth {
            let _ = self.modeler.reset_w_params(params());
        }
        self.feed(pos, p, ModelerInputEventType::Down);
    }

    fn feed(&mut self, pos: egui::Pos2, p: f32, ev: ModelerInputEventType) {
        self.cur_pressure = p;
        self.min_p = self.min_p.min(p);
        self.max_p = self.max_p.max(p);
        let Some(stroke) = self.strokes.last_mut() else { return };
        if self.smooth {
            let input = ModelerInput {
                event_type: ev,
                pos: (pos.x as f64, pos.y as f64),
                time: { let t = (self.now_t - self.stroke_t0).max(self.last_t + 0.001); self.last_t = t; t },
                pressure: p as f64,
            };
            match self.modeler.update(input) {
                Ok(out) => {
                    for r in out {
                        stroke.pts.push((egui::pos2(r.pos.0 as f32, r.pos.1 as f32), r.pressure as f32));
                    }
                }
                Err(e) => {
                    self.modeler_errs += 1;
                    self.last_err = format!("{e:?}");
                }
            }
        } else {
            stroke.pts.push((pos, p));
        }
    }

    fn end(&mut self, pos: egui::Pos2, p: f32) {
        if self.drawing {
            self.feed(pos, p, ModelerInputEventType::Up);
        }
        self.drawing = false;
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // Doc cac su kien tho tu cua so
        let events = ctx.input(|i| i.events.clone());
        let frame_t = ctx.input(|i| i.time);
        let dt = (frame_t - self.prev_frame_t).clamp(0.0, 0.1);
        let n_ev = events.len().max(1) as f64;
        for (k, e) in events.into_iter().enumerate() {
            self.now_t = frame_t - dt + dt * (k as f64 + 1.0) / n_ev;
            match e {
                egui::Event::Touch { phase, pos, force, .. } => {
                    self.touch_events += 1;
                    let p = match force {
                        Some(f) => {
                            self.touch_with_force += 1;
                            f.clamp(0.0, 1.0)
                        }
                        None => 0.5,
                    };
                    self.last_source = if force.is_some() { "TOUCH/BUT (co force)" } else { "TOUCH (khong co force)" };
                    self.touch_active = true;
                    match phase {
                        egui::TouchPhase::Start => self.begin(pos, p),
                        egui::TouchPhase::Move => {
                            if self.drawing {
                                self.feed(pos, p, ModelerInputEventType::Move)
                            }
                        }
                        egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                            self.end(pos, p);
                            self.touch_active = false;
                        }
                    }
                }
                egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, .. }
                    if !self.touch_active =>
                {
                    self.mouse_events += 1;
                    self.last_source = "CHUOT (khong co pressure, gia dinh 0.5)";
                    if pressed {
                        self.begin(pos, 0.5)
                    } else {
                        self.end(pos, 0.5)
                    }
                }
                egui::Event::PointerMoved(pos) if !self.touch_active => {
                    if self.drawing {
                        self.mouse_events += 1;
                        self.last_source = "CHUOT (khong co pressure, gia dinh 0.5)";
                        self.feed(pos, 0.5, ModelerInputEventType::Move);
                    }
                }
                _ => {}
            }
        }

        self.prev_frame_t = frame_t;
        egui::Panel::top("info").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(format!("PRESSURE = {:.3}", self.cur_pressure));
                ui.add(egui::ProgressBar::new(self.cur_pressure).desired_width(200.0));
                if ui.button("Xoa het").clicked() {
                    self.strokes.clear();
                    self.min_p = f32::MAX;
                    self.max_p = 0.0;
                    self.touch_events = 0;
                    self.touch_with_force = 0;
                    self.mouse_events = 0;
                }
                ui.label(format!("loi modeler: {} {}", self.modeler_errs, self.last_err));
                ui.checkbox(&mut self.smooth, "Lam muot kieu Rnote (ink-stroke-modeler)");
            });
            ui.label(format!(
                "Nguon: {} | min={:.3} max={:.3} | su kien touch/but: {} (co force: {}) | su kien chuot: {}",
                self.last_source,
                if self.min_p == f32::MAX { 0.0 } else { self.min_p },
                self.max_p,
                self.touch_events,
                self.touch_with_force,
                self.mouse_events
            ));
        });

        egui::CentralPanel::default().show(ui, |ui| {
            let painter = ui.painter();
            for s in &self.strokes {
                let col = egui::Color32::BLACK;
                for w in s.pts.windows(2) {
                    let (a, pa) = w[0];
                    let (b, pb) = w[1];
                    let width = 1.0 + 14.0 * (pa + pb) * 0.5; // day theo pressure
                    painter.line_segment([a, b], egui::Stroke::new(width, col));
                    painter.circle_filled(b, width * 0.5, col);
                }
                if let Some(&(a, p)) = s.pts.first() {
                    painter.circle_filled(a, (1.0 + 14.0 * p) * 0.5, col);
                }
            }
        });
        ctx.request_repaint();
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native("Spike egui - pressure Wacom", options, Box::new(|_cc| Ok(Box::<App>::default())))
}

#[cfg(test)]
mod ui_check {
    use eframe::egui;

    #[test]
    fn test_ui_spec_capabilities() {
        let ctx = egui::Context::default();
        let vn_sample = "Tiếng Việt, ắ ằ ẳ ẵ ặ ơ ư đ Đ";
        let font_id = egui::FontId::proportional(14.0);

        // 1. Kiểm tra font mặc định của egui (chưa nạp Segoe UI)
        let mut out1 = ctx.run_ui(Default::default(), |ui| {
            ui.fonts_mut(|f| {
                for ch in ['ắ', 'ằ', 'ẳ', 'ẵ', 'ặ', 'ơ', 'ư', 'đ', 'Đ'] {
                    println!("default font has {ch:?}: {}", f.has_glyph(&font_id, ch));
                }
            });
        });
        out1.textures_delta.clear();

        // 2. Nạp font hệ thống Windows (Segoe UI) và kiểm tra lại
        let segoe_bytes = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf").expect("doc segoeui.ttf");
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "segoeui".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(segoe_bytes)),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "segoeui".to_owned());
        ctx.set_fonts(fonts);

        // 3. Chạy thử: thanh trên nền màu bo góc, Pop-up tối bo góc, lưới thumbnail, tiếng Việt đủ dấu
        let mut out2 = ctx.run_ui(Default::default(), |ui| {
            ui.fonts_mut(|f| {
                for ch in ['ắ', 'ằ', 'ẳ', 'ẵ', 'ặ', 'ơ', 'ư', 'đ', 'Đ'] {
                    assert!(f.has_glyph(&font_id, ch), "thieu glyph {ch:?}");
                }
            });

            // (a) Thanh trên nền xanh dương + nút chọn có ô nền sáng bo góc
            let nav_frame = egui::Frame::new()
                .fill(egui::Color32::from_rgb(25, 118, 210))
                .inner_margin(egui::Margin::symmetric(12, 6));
            egui::Panel::top("nav_bar").frame(nav_frame).show(ui, |ui| {
                ui.horizontal(|ui| {
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 50))
                        .corner_radius(egui::CornerRadius::same(8))
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Bút").color(egui::Color32::WHITE));
                        });
                });
            });

            // (b) Thanh tuỳ chọn công cụ bo tròn, nền tối + Pop-up tối bo góc
            egui::CentralPanel::default().show(ui, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(38, 38, 42))
                    .corner_radius(egui::CornerRadius::same(16))
                    .inner_margin(egui::Margin::symmetric(14, 6))
                    .show(ui, |ui| {
                        let btn = ui.button(vn_sample);
                        let popup_frame = egui::Frame::popup(ui.style())
                            .fill(egui::Color32::from_rgb(30, 30, 34))
                            .corner_radius(egui::CornerRadius::same(12));
                        egui::Popup::from_toggle_button_response(&btn)
                            .frame(popup_frame)
                            .show(|ui| {
                                ui.label(vn_sample);
                            });
                    });

                // (c) Lưới thumbnail (5 cột, tự xuống hàng)
                egui::Grid::new("thumb_grid")
                    .spacing(egui::vec2(20.0, 20.0))
                    .show(ui, |ui| {
                        for idx in 0..10 {
                            egui::Frame::new()
                                .fill(egui::Color32::from_gray(245))
                                .corner_radius(egui::CornerRadius::same(8))
                                .inner_margin(egui::Margin::same(8))
                                .show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(120.0, 160.0));
                                    ui.label(format!("Ghi chú #{idx} - {vn_sample}"));
                                });
                            if (idx + 1) % 5 == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
        });
        out2.textures_delta.clear();
    }
}



