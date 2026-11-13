# HANDOFF – InkNote

## 1. Task & tiến độ
Đã hoàn thành toàn bộ các bước của `docs/roadmap.md` từ **Bước 0** đến **Bước 7.4** (dọn cảnh báo biên dịch `inknote-app`, viết `README.md` tiếng Việt cho GitHub, rà soát `LICENSE`/`NOTICE`, pass `cargo check --offline` và `cargo test --offline`). Hiện đang chờ người dùng kiểm tra thủ công và xác nhận OK để gắn tag `step-7.4-ok`.

## 2. File đã sửa (Bước 7.4)
- `README.md`: tạo mới tài liệu hướng dẫn tiếng Việt đầy đủ cho GitHub.
- `crates/inknote-app/src/latex_dialog.rs`: thêm `#[allow(deprecated)]` cho `egui::Id::new("latex_dialog_backdrop")`.
- `crates/inknote-app/src/autosave.rs`: gắn `#[cfg(test)]` cho `AutoSaveState::new`.
- `crates/inknote-app/src/view.rs`: gắn `#[cfg(test)]` cho `to_page`, `to_screen`, `rect_to_screen`.
- `crates/inknote-app/src/theme.rs`: thêm `#[allow(dead_code)]` cho `paper_dot_spacing` và `thumb_panel_stroke`.
- `docs/roadmap.md`: đánh dấu xong Bước 7.4 (chờ xác nhận `step-7.4-ok`).
- `memory.md`: cập nhật trạng thái hoàn thành Bước 7.4.
- `HANDOFF.md`: cập nhật trạng thái bàn giao mới nhất.

## 3. Quyết định & lý do
- Dọn cảnh báo bằng `#[cfg(test)]` cho các helper chỉ dùng trong unit test và `#[allow(deprecated)]` / `#[allow(dead_code)]` cho các cấu trúc giữ nguyên hành vi để bảo đảm 0 warning ở `inknote-app` mà không làm thay đổi logic ứng dụng.
- Giữ nguyên hoàn toàn `crates/inknote-pen` (21 warning `missing_docs` của Rnote) và `reference/` theo quy tắc dự án.

## 4. Việc đang dở & bước tiếp theo
- Chờ người dùng kiểm tra `README.md`, `LICENSE`, `NOTICE` và xác nhận OK để gắn tag `step-7.4-ok` (và đẩy lên GitHub nếu muốn).

## 5. Cảnh báo
- Máy không có `rg` (dùng `Select-String`); Cargo luôn truyền `--offline`.
- Cấm sửa `crates/inknote-pen`, `reference/`; cấm `--shell-escape`; cấm `git reset --hard`.
- Lỗi `egui 0.37`: gọi `ctx.*` trong `ctx.input(...)` gây deadlock; pop-up cần `PopupCloseBehavior::CloseOnClickOutside`; test drag `RawInput` cần nhiều frame.
