# 03 – Báo cáo Port Code Vẽ Nét từ Rnote sang `inknote-pen` và `inknote-render`

Ngày thực hiện: 2026-11-12
Nguồn gốc: `reference/rnote`
- Commit hash: `83a76367dbd3d6c461c10553c45231799a25ea4b` (2026-11-11)
- Giấy phép: **GPL-3.0-or-later** (GNU General Public License v3.0 or later)

---

## 1. Những gì đã copy nguyên văn 100% từ Rnote

Toàn bộ `crates/rnote-compose/src/` (44 file) và cấu hình bút `brushconfig.rs` từ `crates/rnote-engine/src/pens/pensconfig/brushconfig.rs` đã được copy nguyên văn sang `crates/inknote-pen/src/`:
- **Bộ dựng nét bút (`builders/`)**:
  - `penpathmodeledbuilder.rs` (`PenPathModeledBuilder`, dùng `ink-stroke-modeler-rs` với đúng `MODELER_PARAMS` và dự đoán nét `prediction_buffer`).
  - `penpathcurvedbuilder.rs`, `penpathsimplebuilder.rs`, và tất cả các shape builders.
- **Đường nét (`penpath/`)**: `Element` (`PRESSURE_DEFAULT = 0.5`), `Segment` (`LineTo`, `QuadBezTo`, `CubBezTo`), `PenPath`.
- **Phong cách vẽ (`style/`)**:
  - `smooth/mod.rs` (hàm `compose_lines_variable_width`, nắp bo tròn đầu/cuối hệ số `2/3`, ghép `BezPath` rồi gọi `cx.fill(full_path, ...)`).
  - `smooth/smoothoptions.rs` (`SmoothOptions`, mặc định `stroke_width = 2.0`, `PressureCurve::Linear`).
  - `PressureCurve` (`Const`, `Linear`, `Sqrt`, `Cbrt`, `Pow2`, `Pow3`).
  - `textured/` và `rough/`.
- **Cấu hình bút mặc định (`brushconfig.rs`)**:
  - `SolidOptions` (mặc định `SmoothOptions::default()` -> `stroke_width = 2.0`, `PressureCurve::Linear`).
  - `MarkerOptions` (mặc định `stroke_width = 12.0`, `PressureCurve::Const`).
  - `BrushConfig`, `BrushStyle` (`Marker`, `Solid`, `Textured`).

Đầu mỗi file `.rs` trong `crates/inknote-pen/src/` đều có 3 dòng ghi rõ:
- File gốc trong Rnote
- Commit hash `83a76367dbd3d6c461c10553c45231799a25ea4b`
- Giấy phép `GPL-3.0-or-later`

---

## 2. Cách thay thế Cairo mà không sửa code Rnote

Trong `rnote-compose` gốc, các hàm `draw_styled` nhận `&mut piet_cairo::CairoRenderContext`, và `draw_composed` nhận `&mut impl piet::RenderContext`.
Để **không phải sửa bất kỳ dòng nào** trong các file của `rnote-compose`:
1. Tạo crate `crates/inknote-render` cung cấp `pub struct CairoRenderContext` cài đặt `piet::RenderContext`.
2. Trong `crates/inknote-pen/Cargo.toml`, khai báo:
   ```toml
   piet-cairo = { package = "inknote-render", path = "../inknote-render" }
   ```
   Nhờ đó mọi lệnh gọi `piet_cairo::CairoRenderContext` và `cx.fill(...)` của Rnote giữ nguyên 100%, nhưng bên dưới chạy qua `lyon_tessellation` để chia đường cong `kurbo::BezPath` thành các tam giác cho `egui` vẽ bằng GPU.

---

## 3. Quy tắc tô nét (Fill Rule) của Rnote

Đã kiểm chứng trong code gốc của `piet-cairo` (`lib.rs` dòng 127–145):
- `cx.fill(...)` gọi `self.ctx.set_fill_rule(cairo::FillRule::Winding)`.
- `cx.fill_even_odd(...)` gọi `self.ctx.set_fill_rule(cairo::FillRule::EvenOdd)`.

Trong `lyon_tessellation`, quy tắc `Winding` của Cairo tương ứng chính xác với **`FillRule::NonZero`**.
`inknote-render` đã dùng đúng `FillRule::NonZero` cho `cx.fill(...)` và `FillRule::EvenOdd` cho `cx.fill_even_odd(...)`.

---

## 4. Các chỗ khác với bản gốc Rnote và lý do

| Vị trí | Khác biệt so với Rnote gốc | Lý do |
|---|---|---|
| Đầu mỗi file trong `crates/inknote-pen/src/` | Thêm 3 dòng chú thích nguồn gốc, commit hash, giấy phép GPLv3 | Tuân thủ quy tắc ghi nguồn GPLv3 theo yêu cầu |
| `crates/inknote-pen/src/lib.rs` (cuối file) | Thêm `extern crate self as rnote_compose; pub mod brushconfig; pub mod store { pub mod chrono_comp; }` | Để `brushconfig.rs` (lấy từ `rnote-engine`) giữ nguyên các dòng `use rnote_compose::...` mà không phải sửa |
| `crates/inknote-pen/src/brushconfig.rs` | Đổi `pub(crate) fn` thành `pub fn` ở 3 phương thức (`layer_for_current_options`, `new_style_seeds`, `style_for_current_options`) | Trong Rnote, `brushconfig.rs` nằm cùng crate `rnote-engine`; ở đây tách sang `inknote-pen` nên cần `pub` để `inknote-app` gọi được |
| `crates/inknote-pen/src/store/chrono_comp.rs` | Chỉ giữ `enum StrokeLayer` (50 dòng đầu của `rnote-engine/src/store/chrono_comp.rs`) | `brushconfig.rs` chỉ dùng kiểu `StrokeLayer`; phần còn lại của `chrono_comp.rs` thuộc bộ lưu trữ `StrokeStore` của `rnote-engine` |
| Xử lý sự kiện nhấc bút (`TouchPhase::End`) trên `egui` Windows (`crates/inknote-app/src/main.rs`) | Trong Rnote (`rnote-ui/src/canvas/input.rs` dòng 436–440): nếu là bút (`is_stylus`) thì lấy `event.axis(Pressure)` (khi nhấc bút khỏi mặt bảng cảm ứng là `0.0`), nếu là chuột mới dùng `Element::PRESSURE_DEFAULT` (`0.5`). Trên Windows `winit`/`egui`, sự kiện `TouchPhase::End` báo `force: None`. Do đó trong `inknote-app`, khi đang ở phiên vẽ bằng bút cảm ứng (`is_stylus`) mà `TouchPhase::End` có `force: None`, ta truyền `pressure = 0.0` (đúng áp lực vật lý khi bút rời mặt bảng), còn chuột dùng `Element::PRESSURE_DEFAULT` (`0.5`). | Ở bản spike cũ, `TouchPhase::End` với `force: None` bị gán nhầm thành `0.5`, khiến `ink-stroke-modeler-rs` nội suy điểm cuối nét phình to thành hình giọt nước |

