// Spike B: chay thu ink-stroke-modeler-rs (thu vien lam muot net cua Rnote), KHONG can GTK/cairo.
use ink_stroke_modeler_rs::{ModelerInput, ModelerInputEventType, ModelerParams, StrokeModeler};

fn main() {
    // Cung tham so voi Rnote (reference/rnote/.../penpathmodeledbuilder.rs)
    let params = ModelerParams {
        sampling_min_output_rate: 120.0,
        sampling_end_of_stroke_stopping_distance: 0.01,
        sampling_end_of_stroke_max_iterations: 20,
        sampling_max_outputs_per_call: 200,
        stylus_state_modeler_max_input_samples: 20,
        ..ModelerParams::suggested()
    };
    let mut m = StrokeModeler::default();
    m.reset_w_params(params).expect("reset that bai");

    let n = 30;
    let mut total_out = 0;
    for i in 0..=n {
        let t = i as f64 * 0.008; // ~125 mau/giay
        let x = 10.0 + i as f64 * 3.0;
        let y = 50.0 + (i as f64 * 0.3).sin() * 10.0;
        let p = (i as f64 / n as f64).clamp(0.0, 1.0); // ap luc tang dan 0 -> 1
        let ev = if i == 0 {
            ModelerInputEventType::Down
        } else if i == n {
            ModelerInputEventType::Up
        } else {
            ModelerInputEventType::Move
        };
        let out = m
            .update(ModelerInput { event_type: ev, pos: (x, y), time: t, pressure: p })
            .expect("update that bai");
        total_out += out.len();
        if i % 10 == 0 || i == n {
            println!("vao #{i}: pressure={p:.2} -> {} diem ra", out.len());
            if let Some(last) = out.last() {
                println!("   diem cuoi: pos=({:.1},{:.1}) pressure={:.2}", last.pos.0, last.pos.1, last.pressure);
            }
        }
    }
    println!("OK: {} diem vao -> {} diem lam muot", n + 1, total_out);
}

