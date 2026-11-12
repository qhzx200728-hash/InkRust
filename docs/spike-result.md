# Kết quả Spike (Bước 0)

Ngày: 2026-11-12. Máy: Windows, Rust 1.99.1 (cài mới bằng rustup, không cần winget), MSVC Build Tools có sẵn.
Mã thử nằm trong `spike/` (bỏ đi sau này).

## Tóm tắt
| Bài | Kết quả | Cần bạn test? |
|---|---|---|
| A1. egui đọc pressure | **ĐẠT** (chủ dự án test Wacom 2026-11-12: nguồn đổi sang TOUCH/BUT, force tăng, nét dày/mỏng đúng) | Đã test |
| A2. Tauri 2 | Không cần (A1 đạt) | – |
| A3. Slint | Không cần | – |
| B. `ink-stroke-modeler-rs` | **Build và chạy được trên Windows, không cần GTK/cairo** | Có (xem dưới) |
| B'. `rnote-compose` nguyên crate | **KHÔNG dùng được nếu không có cairo**: `Cargo.toml` của nó phụ thuộc `piet-cairo` | Không |
| C. LaTeX → SVG | **Chạy tốt**, tiếng Việt đúng, ~3,5 giây/lần | Nên nhìn thử file SVG |

---

## A1. egui (`spike/egui-pressure`)

**Cách chạy** (PowerShell):
```powershell
$env:Path="$env:USERPROFILE\.cargo\bin;$env:Path"
cd C:\Users\qhzx\Documents\app-take-note\spike\egui-pressure
cargo run
```
Hoặc nhấp đôi `spike\egui-pressure\target\debug\egui-pressure.exe`.

**Trên màn hình có:** dòng `PRESSURE = 0.xxx` + thanh, dòng chẩn đoán "Nguồn / min / max / số sự kiện touch/bút (có force) / số sự kiện chuột", nút "Xoá hết", ô "Làm mượt kiểu Rnote".

**Cách test thủ công (làm bằng bút Wacom):**
1. Rê bút lơ lửng trên màn hình (chưa chạm). Rồi chạm nhẹ vẽ một đường ngắn: **nét mỏng?** Số PRESSURE nhỏ (~0.1–0.3)?
2. Ấn mạnh vẽ một đường: **nét dày?** Số lên gần 1?
3. Vẽ một nét ấn từ nhẹ tăng dần đến mạnh: nét dày dần?
4. Nhìn dòng "Nguồn": 
   - `TOUCH/BUT (co force)` + số "co force" tăng ⇒ **egui đọc được pressure, TỐT.**
   - `CHUOT ...` và số touch = 0 ⇒ egui chỉ thấy bút như chuột ⇒ **KHÔNG có pressure** (sang Tauri).
   - `TOUCH (khong co force)` ⇒ thấy bút nhưng không có pressure ⇒ cũng coi là lỗi.
5. Tích ô "Làm mượt kiểu Rnote", vẽ đường cong nhanh: nét mượt hơn? Pressure vẫn đổi?
6. Báo lại: dòng "Nguồn", min, max, và các số sự kiện.

Nếu pressure luôn = 0.5 hoặc không đổi, thử: Wacom Tablet Properties → bật **Use Windows Ink**, rồi chạy lại.

**Ghi chú kỹ thuật:** egui 0.37 đổi API so với các ví dụ cũ (`update` → `ui`, `TopBottomPanel` → `Panel`). Đã sửa theo code thật trong `~/.cargo/registry`.

---

## B. `ink-stroke-modeler-rs` (`spike/modeler-test`)

```powershell
cd C:\Users\qhzx\Documents\app-take-note\spike\modeler-test
cargo run
```
**Kết quả đã chạy:** 31 điểm vào (pressure tăng 0 → 1) ⇒ 38 điểm ra mượt, pressure ra tăng dần đúng (0.00 → 0.27 → 0.60 → 1.00). Dùng đúng tham số của Rnote (`penpathmodeledbuilder.rs`). 
**Chạy qua nét thật:** có sẵn trong app egui (ô "Làm mượt kiểu Rnote"). Test thủ công: xem bước 5 ở trên.

**Kết luận:** làm mượt dùng được ngay. Phần *tính độ dày/hình dạng nét* của Rnote nằm trong `rnote-compose` (dính cairo) ⇒ phải đọc và viết lại ở Bước 1.4.

---

## C. LaTeX → SVG (`spike/latex`)

Nội dung thử: công thức `x = (-b ± √(b²-4ac)) / 2a` và câu "Xin chào thế giới, Việt Nam!" (gói `T5` + `utf8`, class `standalone`).

| Cách | Kết quả | Thời gian |
|---|---|---|
| `pdflatex` → PDF | OK | ~2,1 giây |
| `dvisvgm --pdf` (PDF → SVG) | **LỖI**: Ghostscript 10.08 không được hỗ trợ (cần < 10.01 hoặc `mutool`) | – |
| **`latex` → DVI → `dvisvgm --no-fonts`** | **OK**, SVG vector 32 KB | latex ~1,5–1,9 s + dvisvgm ~1,8–2,2 s ≈ **3,5 giây** |
| `pdftocairo -svg` (dự phòng) | OK, 39 KB | ~3,4 giây (chỉ chuyển PDF→SVG, chưa kể pdflatex) |

(Đo khi máy đang build Rust song song nên có thể chậm hơn thực tế.)

**Cách xem thử:** mở `spike\latex\test_dvi.svg` bằng trình duyệt; phóng to xem còn nét không.
**Kết luận:** chọn đường `latex` → `dvisvgm`. ~3,5 s là chậm cho live preview ⇒ cần debounce (đợi ngừng gõ) và chạy luồng nền; bước sau có thể thử tăng tốc (vd. `latex` chỉ nạp gói cần thiết, dùng "format file" đã biên dịch trước).

---

## Kết luận
**Chọn egui (hướng B).** Chưa test kỹ (để bước 1.2-1.4): độ trễ nét, độ giống Rnote khi bật làm mượt, và việc bàn tay chạm màn hình có bị lẫn với bút không.

