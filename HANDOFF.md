# HANDOFF – InkNote

## 1. Task & tiến độ
Đang làm InkNote theo `docs/roadmap.md`: vừa xong **Bước 7.3** (xuất PNG 3x/4x, PDF nhiều trang vector, Read-Only Mode), pass test, tag `step-7.3-ok`. Dừng trước **Bước 7.4** để bàn giao.

## 2. File đã sửa (Bước 7.3; Bước 7.1–7.2 đã ghi trong `memory.md`)
- `Cargo.lock`: khóa thêm `resvg` và `flate2` (offline).
- `crates/inknote-app/Cargo.toml`: thêm `resvg = "0.45"`, `flate2 = "1.1"`.
- `crates/inknote-render/src/lib.rs`: thêm `VectorOp`, `CairoRenderContext::new_vector_recorder()`.
- `crates/inknote-app/src/export.rs`: tạo mới renderer PNG (`tiny_skia`, `Lanczos3`) và PDF 1.4 vector (`FlateDecode`, `usvg` outline font).
- `crates/inknote-app/src/shapes.rs`: mở công khai `pub fn shape_arrow_heads`.
- `crates/inknote-app/src/toolbar.rs`: gắn pop-up `ToolIcon::Export`, khóa nút sửa và ẩn thanh nổi khi `read_only`.
- `crates/inknote-app/src/main.rs`: thêm `handle_export_request`, toast, pan/zoom Read-Only, `test_step_7_3_export_pdf_png_and_read_only_mode`.
- `docs/roadmap.md`: đánh dấu xong Bước 7.3.
- `memory.md`: cập nhật trạng thái tới `step-7.3-ok`.
- `HANDOFF.md`: tạo mới tài liệu bàn giao.

## 3. Quyết định & lý do
- Dùng thuần Rust (`resvg`/`usvg` + `flate2`) xuất PDF vector (outline text qua font Windows) và PNG 3x/4x để cực nét, không phụ thuộc tool ngoài.
- Ẩn hoàn toàn thanh nổi tuỳ chọn khi bật Read-Only Mode để đọc thoáng mắt.

## 4. Việc đang dở & bước tiếp theo
- Còn **Bước 7.4 – Hoàn thiện & `README.md` cho GitHub** (`docs/roadmap.md`).
- Làm ngay: thêm `%USERPROFILE%\.cargo\bin` vào `PATH`, chạy `cargo test --offline -p inknote-app`, dọn cảnh báo ở `inknote-app`, viết `README.md` tiếng Việt (cài Rust, TeX Live, cách chạy, GPL-3.0), rà `LICENSE`/`NOTICE`.

## 5. Cảnh báo
- Máy không có `rg` (dùng `Select-String`); Cargo luôn truyền `--offline`.
- Cấm sửa `crates/inknote-pen`, `reference/`; cấm `--shell-escape`; cấm `git reset --hard`.
- Lỗi `egui 0.37`: gọi `ctx.*` trong `ctx.input(...)` gây deadlock; pop-up cần `PopupCloseBehavior::CloseOnClickOutside`; test drag `RawInput` cần nhiều frame.
