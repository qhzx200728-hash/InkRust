# 00 – Phân tích Rnote và Xournal++

Ngày: 2026-11-12. Đọc từ `reference/rnote` và `reference/xournalpp` (clone `--depth 1`).

## Những gì đã đọc (đã kiểm chứng trong code)

### Rnote (giấy phép GPLv3)
- 4 phần (crate = gói code Rust): `rnote-compose` (nét bút, hình), `rnote-engine` (trang, lưu trữ, các loại bút), `rnote-ui` (giao diện), `rnote-cli`.
- Làm mượt nét: thư viện `ink-stroke-modeler-rs` 0.1.0, dùng trong `rnote-compose/src/builders/penpathmodeledbuilder.rs`. Các tham số (120 mẫu/giây, v.v.) nằm ở hằng `MODELER_PARAMS`.
- Dữ liệu nét bút: `Element` (vị trí + pressure) → `Segment` → `PenPath` (`rnote-compose/src/penpath/`).
- Các loại đối tượng trong `rnote-engine/src/strokes/`: `brushstroke`, `shapestroke`, `textstroke`, `vectorimage`, `bitmapimage`. Mỗi loại là loại riêng → lọc khi tẩy được.
- Vẽ bằng `piet` + `cairo`. Giao diện GTK4.
- `rnote-engine` còn dùng `hayro` (đọc PDF bằng Rust thuần), `usvg`, `librsvg`.
- Có ghi chú trong `rnote-engine/Cargo.toml`: tác giả định bỏ gtk4 về lâu dài.
- Định dạng file: `fileformats/rnoteformat`, và có đọc `xoppformat` (file Xournal++).

### Xournal++ (giấy phép GPLv2)
- Lệnh mặc định: `pdflatex -halt-on-error -interaction=nonstopmode '<file>'` (`LatexSettings.h`).
- `LatexGenerator::templateSub` bơm code người dùng vào file mẫu (template) `.tex`.
- Kết quả là PDF, lưu trong đối tượng `TexImage` cùng chuỗi source (`getText/setText`, `getBinaryData`).
- Khi sửa công thức: `LatexController` gộp "xoá bản cũ + chèn bản mới" thành 1 `GroupUndoAction` → 1 bước undo.
- Có 2 kiểu dialog: nhập trong app (`IntEdLatexDialog`) hoặc dùng editor ngoài (`ExtEdLatexDialog`).

## Kết quả kiểm chứng thêm trong Spike (xem `spike-result.md`)
- `rnote-compose` phụ thuộc `piet-cairo` ⇒ **bắt buộc có cairo**. Không dùng nguyên crate này khi bỏ GTK/cairo.
- `ink-stroke-modeler-rs` thì **tự build được** trên Windows, không cần GTK/cairo.

## Rủi ro

| # | Rủi ro | Mức | Hướng xử lý |
|---|---|---|---|
| 1 | "Clone 100% nét Rnote": phần làm mượt dùng được ngay; phần tính độ dày/đầu nét phải viết lại (vì dính cairo) | Trung bình | Đọc code `penpath/` và `style/smooth/`, viết lại với kurbo/lyon; so sánh bằng mắt với Rnote thật |
| 2 | Pressure Wacom trên Windows tuỳ thư viện giao diện | Cao | Spike bước 0 |
| 3 | Giấy phép: copy code Rnote/Xournal++ ⇒ InkNote phải GPL | Thấp | Đã chấp nhận: đưa toàn bộ source lên GitHub, dùng cá nhân |
| 4 | Xournal++ GPLv2 và Rnote GPLv3 có thể không hợp nhau nếu copy cả hai | Thấp | Chỉ *đọc để hiểu* Xournal++, tự viết lại; chỉ copy từ Rnote khi cần |
| 5 | LaTeX cần TeX Live | Thấp | Chỉ dùng máy của chủ dự án (đã có TeX Live 2026) |
| 6 | Ảnh bị mờ | Trung bình | Lưu ảnh gốc, vẽ lại theo mức zoom |
| 7 | Android + Rust + bút | Cao | Ngoài phạm vi (chỉ Windows) |

