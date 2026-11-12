# 02 – Kiến trúc InkNote (bản nháp, theo hướng B)

> Bản nháp: có thể đổi sau khi Spike xong (đặc biệt phần giao diện: egui hoặc Tauri).

## Nguyên tắc
- **Tách "lõi" và "giao diện".** Lõi (`inknote-core`) chỉ chứa dữ liệu và logic, không biết gì về cửa sổ. Nhờ đó đổi giao diện (egui → Tauri) không phải viết lại lõi.
- **Mọi thay đổi đi qua "lệnh" (command)** để có undo/redo: mỗi lệnh biết cách làm và cách hoàn tác.
- **Dữ liệu chính là nguồn gốc, hình chỉ là bản lưu tạm** (cache).

## Các phần (crate)
| Phần | Việc |
|---|---|
| `inknote-core` | Tài liệu → trang → đối tượng; lệnh undo/redo; lưu/mở file |
| `inknote-pen` | Nhận điểm bút (vị trí + pressure) → làm mượt (`ink-stroke-modeler-rs`) → tạo nét |
| `inknote-latex` | Chạy `pdflatex`/`latex` + `dvisvgm`, trả về SVG |
| `inknote-app` | Cửa sổ, vẽ, dialog, thanh công cụ (egui) |

## Dữ liệu
```
Tài liệu
 ├─ cài đặt giấy (kiểu kẻ, cỡ ô, lề trái/phải)
 └─ Trang[]
     └─ Đối tượng[]  (mỗi cái có id, vị trí, kích thước)
         ├─ Nét bút   (điểm + pressure + kiểu bút + màu)   ← CHỈ loại này bị Eraser xoá
         ├─ Hình học
         ├─ Chữ
         ├─ Ảnh       (byte ảnh GỐC + vị trí; vẽ lại theo zoom → không mờ)
         └─ LaTeX     (source = dữ liệu chính; svg = bản lưu tạm)
```
- File lưu: một file duy nhất (dự kiến JSON nén, hoặc SQLite — chưa chốt).
- Auto-save: ghi ra file tạm, rồi đổi tên (an toàn nếu máy tắt đột ngột). Lưu mỗi vài giây sau thay đổi.
- Thumbnail: vẽ trang thu nhỏ, lưu cache theo "phiên bản trang".

## Eraser
Chỉ duyệt đối tượng loại "Nét bút". Các loại khác bỏ qua ngay ở đầu hàm.

## LaTeX (cực kỳ quan trọng) – theo cách Xournal++

### Đối tượng
`LatexObject { id, source: String, pos, size, svg_cache: Option<String> }`

### Chèn mới
1. Người dùng bấm nút LaTeX → mở dialog trống.
2. Gõ code → chờ ~0,5 giây sau khi ngừng gõ → chạy ở **luồng nền** (không đơ app).
3. Live preview: hiện hình khi thành công, hiện lỗi (dòng từ log) khi thất bại; bản cũ vẫn hiển thị khi lỗi.
4. Bấm OK → chèn đối tượng. **1 bước undo.**

### Sửa
1. Double-click (hoặc double-tap) vào đối tượng LaTeX → mở dialog với `source` cũ.
2. Sửa xong bấm OK → **thay `source` và `svg`, giữ nguyên vị trí và kích thước.**
   Quy tắc kích thước (đề xuất): giữ nguyên *chiều rộng*, chiều cao theo tỉ lệ mới (chốt sau).
3. **1 lần sửa = 1 lệnh undo** (lưu `source` cũ + `svg` cũ để hoàn tác).
4. Bấm Huỷ → không đổi gì, không có bước undo.

### Cách render (đã đo trong Spike, xem `spike-result.md`)
- Mẫu file `.tex` (template): `standalone` + `preview`, `amsmath`, `amssymb`, gói tiếng Việt (`T5`/`vietnam`) → chạy `latex` ra DVI → `dvisvgm --no-fonts` ra SVG vector.
- Lý do dùng DVI thay vì PDF: `dvisvgm` đọc PDF cần Ghostscript < 10.01 hoặc `mutool`, máy hiện có Ghostscript 10.08 nên **không chạy**. Phương án dự phòng: `pdftocairo -svg` (chậm hơn ~2×).
- Thư mục tạm riêng cho mỗi lần render; xoá sau khi xong.
- Hiển thị SVG: dùng `resvg`/`usvg` (chưa chắc, sẽ kiểm tra) hoặc để webview hiển thị nếu dùng Tauri.

### Lưu file
Lưu `source` + `svg_cache`. Nếu mở file mà thiếu `svg_cache` → render lại từ `source`.

