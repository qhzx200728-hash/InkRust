# HANDOFF.md – Bàn giao phiên làm việc InkNote

## 1. Đang làm gì, tới đâu
Đã hoàn thành và chốt kiểm thử **Bước 2.3: Auto-save & Hãm FPS theo tần số quét màn hình (P1)** (đã gắn tag `step-2.3-ok`). Chuẩn bị bắt tay thực hiện **Bước 3.1: Nhiều trang (P1)**.

## 2. File đã sửa trong phiên gần nhất
- `crates/inknote-core/src/lib.rs`: Thêm `set_paper_settings`, `mark_saved_at`, nâng cấp `DocumentFile::save_to_path` dùng file tạm cùng thư mục + `sync_all()` + atomic `rename`.
- `crates/inknote-app/src/autosave.rs`: Tạo bộ điều phối `AutoSaveState` chạy luồng nền (debounce 2,5s, `flush_on_exit`, thông báo `"Auto-saved"` hiện 3s rồi tự ẩn).
- `crates/inknote-app/src/theme.rs`: Thêm màu chấm trạng thái lưu (`nav_status_saved_dot`, `nav_status_unsaved_dot`) và màu chữ thông báo trên thanh công cụ chính.
- `crates/inknote-app/src/toolbar.rs`: Vẽ cụm `Tên file + chấm xanh/đỏ + thông báo "Auto-saved" (3s)` ngay bên phải nút Read-Only Mode.
- `crates/inknote-app/src/main.rs`: Tích hợp `AutoSaveState`, bật VSync (`PresentMode::Fifo`) hãm FPS theo tần số quét màn hình (144Hz/60Hz), chuyển sang reactive repaint + `request_repaint_after`.
- `docs/roadmap.md` & `memory.md`: Cập nhật hoàn thành Bước 2.3 (tag `step-2.3-ok`), chuyển mục tiêu sang Bước 3.1.

## 3. Quyết định vừa chốt & lý do
- **Trạng thái lưu trên Navbar**: Đặt `Tên file + Chấm xanh/đỏ + Chữ "Auto-saved" (hiện 3s rồi ẩn)` ngay bên phải nút Read-Only thay vì góc dưới-phải canvas.
- **Hãm FPS theo màn hình**: Dùng `PresentMode::Fifo` + bỏ `ctx.request_repaint()` liên tục (reactive repaint) để tự hãm theo Hz màn hình và giảm tải CPU/GPU khi đứng yên.

## 4. Việc đang dở & Bước tiếp theo
- **Đang dở**: Không có việc dở dang; codebase sạch và pass toàn bộ bài test.
- **Bước tiếp theo (Bước 3.1: Nhiều trang)**: Thêm nhiều trang trong một tài liệu (nút Add Page trên toolbar chính, chuyển trang, xoá trang có Undo/Redo), cập nhật `Document` và định dạng lưu `.inknote` nhiều trang.

## 5. Cảnh báo & Đã thử không được
- **Vẽ liên tục bằng `request_repaint()` + `AutoNoVsync`**: Làm máy quá tải do đẩy FPS vượt ngưỡng màn hình kể cả lúc không vẽ. Luôn giữ `PresentMode::Fifo` và chỉ gọi `request_repaint_after(...)` khi thực sự có timer chờ.
- **Pop-up egui**: Luôn phải đặt `close_behavior(PopupCloseBehavior::CloseOnClickOutside)` để pop-up không biến mất khi bấm chuột bên trong.
