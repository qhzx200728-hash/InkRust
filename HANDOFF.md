# HANDOFF.md – Bàn giao phiên làm việc InkNote

## 1. Đang làm gì, tới đâu
Đã hoàn thành và chốt kiểm thử **Bước 2.2: Paper Settings (P1)** (đã gắn tag `step-2.2-ok`). Chuẩn bị bắt tay thực hiện **Bước 2.3: Auto-save (P1)**.

## 2. File đã sửa trong phiên gần nhất
- `Cargo.lock` & `crates/inknote-app/Cargo.toml`: Thêm dependency `serde_json` để lưu/đọc cấu hình giấy mặc định.
- `crates/inknote-core/src/lib.rs`: Thêm `CanvasMode` (Paper/Whiteboard), `PaperSize` (A4/A5), `PaperOrientation` (Portrait/Landscape), mở rộng `PaperSettings`.
- `crates/inknote-app/src/theme.rs`: Thêm màu lưới/dòng kẻ và đường lề hồng pastel (`paper_margin_line_color`).
- `crates/inknote-app/src/icons.rs`: Viết `draw_paper` và `paper_sheet_rect` (Paper có bóng đổ và lề, Whiteboard tràn phẳng không lề).
- `crates/inknote-app/src/toolbar.rs`: Thêm cấu hình giấy vào menu More `...`; gán `PopupCloseBehavior::CloseOnClickOutside` cho pop-up.
- `crates/inknote-app/src/main.rs`: Căn giữa trang (`Ctrl+0`), chặn vẽ lem bàn xám qua `drawable_rect`, clip nét vẽ, lưu cấu hình vào APPDATA.
- `docs/roadmap.md` & `memory.md`: Cập nhật hoàn thành Bước 2.2 (tag `step-2.2-ok`), chuyển mục tiêu sang Bước 2.3.

## 3. Quyết định vừa chốt & lý do
- **Canvas 2 chế độ**: `Paper` (khổ chuẩn A4/A5 có bóng đổ) và `Whiteboard` (bảng trắng tràn màn hình, không có đường lề).
- **Giữ thanh công cụ tối giản**: Gom toàn bộ tuỳ chọn giấy vào menu More `...` thay vì đặt thêm nút lên thanh chính.

## 4. Việc đang dở & Bước tiếp theo
- **Đang dở**: Không có việc dở dang; codebase sạch và pass toàn bộ bài test.
- **Bước tiếp theo (Bước 2.3: Auto-save)**: Với tài liệu đã có file lưu, tự động lưu sau 2–3 giây ngừng vẽ/chỉnh sửa (debounce), ghi ra file `.tmp` rồi atomic rename đè lên file chính, hiển thị trạng thái lưu nhẹ nhàng.

## 5. Cảnh báo & Đã thử không được
- **Pop-up egui**: Luôn phải đặt `close_behavior(PopupCloseBehavior::CloseOnClickOutside)` để pop-up không biến mất khi bấm chuột bên trong.
- **Vẽ lem ra ngoài**: Ở chế độ Paper, phải luôn kiểm tra toạ độ bằng `drawable_rect()` và clip nét bằng `painter.with_clip_rect()`.
