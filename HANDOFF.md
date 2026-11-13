# HANDOFF.md – Bàn giao phiên làm việc InkNote

## 1. Đang làm gì, tới đâu
Đã hoàn thành và chốt kiểm thử **Bước 3.2: Thanh thumbnail trang (P1)** (đã gắn tag `step-3.2-ok`). Chuẩn bị sang **Bước 4.1: Crate `inknote-latex` (P2)**.

## 2. File đã sửa trong phiên gần nhất
- `crates/inknote-core/src/lib.rs`: Thêm `Command::InsertPageBefore`, `Command::DuplicatePage`, `Command::ClearPageAt` kèm Undo/Redo.
- `crates/inknote-app/src/thumbnails.rs`: Tạo thanh thumbnail 2 cột kiểu GoodNotes (`Pages` + `×`, bộ nhớ đệm `ThumbCache` theo từng trang, nút `⌄` mở menu popup `172px`).
- `crates/inknote-app/src/toolbar.rs` & `theme.rs`: Nối nút `ToolIcon::Pages` (`show_thumbnails`) và màu sắc panel thumbnail.
- `crates/inknote-app/src/main.rs`: Tích hợp `egui::Panel::left` (tắt đường kẻ phân cách `show_separator_line(false)`), xử lý đầy đủ thao tác từng trang và xác nhận `Clear Page` / `Delete Page` theo `Option<usize>`.
- `docs/roadmap.md` & `memory.md`: Cập nhật hoàn thành Bước 3.2 (tag `step-3.2-ok`), chuyển mục tiêu sang Bước 4.1.

## 3. Quyết định vừa chốt & lý do
- **Thanh thumbnail kiểu GoodNotes**: Bố cục 2 cột gọn sạch bên trái, không có đường kẻ đen phân cách hay nút Add Page thừa bên trong panel; mỗi trang có menu `⌄` riêng (`172px`) chứa `Add Page Before`, `Add Page After`, `Duplicate`, `Clear Page`, `Delete Page`.

## 4. Việc đang dở & Bước tiếp theo
- **Đang dở**: Không có việc dở dang; codebase sạch và pass toàn bộ bài test (`14` test `inknote-core`, `14` test `inknote-app`).
- **Bước tiếp theo (Bước 4.1: Crate `inknote-latex`)**: Tạo crate `inknote-latex` chạy `latex -no-shell-escape -halt-on-error -interaction=nonstopmode` → DVI → `dvisvgm --no-fonts` (có timeout, không bật `shell-escape`) để render công thức LaTeX sắc nét.

## 5. Cảnh báo & Đã thử không được
- **Vẽ liên tục bằng `request_repaint()` + `AutoNoVsync`**: Làm máy quá tải do đẩy FPS vượt ngưỡng màn hình kể cả lúc không vẽ. Luôn giữ `PresentMode::Fifo` và chỉ gọi `request_repaint_after(...)` khi thực sự có timer chờ.
- **Pop-up egui**: Luôn phải đặt `close_behavior(PopupCloseBehavior::CloseOnClickOutside)` để pop-up không biến mất khi bấm chuột bên trong.
