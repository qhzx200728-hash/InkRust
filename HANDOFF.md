# HANDOFF.md – Bàn giao phiên làm việc InkNote

## 1. Đang làm gì, tới đâu
Đang thực hiện **Bước 2.2: Paper Settings (P1)**, đã hoàn thành code và vượt qua 14/14 bài test tự động; đang chờ người dùng kiểm thử thực tế để chốt tag `step-2.2-ok`.

## 2. File đã sửa trong phiên này
- `Cargo.lock` & `crates/inknote-app/Cargo.toml`: Thêm dependency `serde_json = "1.0"` để lưu/đọc cấu hình giấy mặc định.
- `crates/inknote-core/src/lib.rs`: Thêm `CanvasMode` (Paper/Whiteboard), `PaperSize` (A4/A5), `PaperOrientation` (Portrait/Landscape), mở rộng `PaperSettings`.
- `crates/inknote-app/src/theme.rs`: Thêm mã màu `paper_grid_color`, `paper_line_color`, `paper_margin_line_color` (hồng pastel).
- `crates/inknote-app/src/icons.rs`: Viết `draw_paper`, `paper_sheet_rect`; Paper có đổ bóng và lề 2 bên, Whiteboard tràn viền phẳng không có lề.
- `crates/inknote-app/src/toolbar.rs`: Bổ sung cài đặt giấy trong menu More; cấu hình `PopupCloseBehavior::CloseOnClickOutside` chống tự đóng pop-up.
- `crates/inknote-app/src/main.rs`: Căn giữa tờ giấy (`Ctrl+0`), chặn vẽ lem ra bàn xám, clip nét trong trang giấy, lưu cấu hình mặc định vào APPDATA.

## 3. Quyết định vừa chốt & lý do
- **2 chế độ Canvas**: `Paper` (trang chuẩn A4/A5 có bóng đổ trên bàn xám) và `Whiteboard` (bảng trắng vô tận tràn màn hình).
- **Whiteboard không được có margin**: Bỏ hoàn toàn đường lề và ẩn mục chỉnh lề khi ở chế độ Whiteboard.
- **Không đặt nút mode trên Navbar**: Giữ navbar tối giản đúng chuẩn P0; gom toàn bộ cấu hình giấy vào menu More `...`.

## 4. Việc đang dở & Bước tiếp theo
- **Chờ test Bước 2.2**: Chạy `cargo run -p inknote-app`, test menu More chỉnh A4/A5, dọc/ngang, Paper/Whiteboard.
- **Khi người dùng báo OK**: Gắn tag `step-2.2-ok`, cập nhật `docs/roadmap.md`, chuyển sang **Bước 2.3: Auto-save** (tự lưu atomic rename sau 2–3s debounce).

## 5. Cảnh báo & Đã thử không được
- **Pop-up egui bị ẩn ngay khi click**: Mặc định là `CloseOnClick`. Phải luôn set `.close_behavior(PopupCloseBehavior::CloseOnClickOutside)`.
- **Chống vẽ lem ra bàn xám**: Kiểm tra toạ độ qua `drawable_rect()` và clip nét bằng `painter.with_clip_rect()`.
