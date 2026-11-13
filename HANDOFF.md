# HANDOFF.md – Bàn giao phiên làm việc InkNote

## 1. Đang làm gì, tới đâu
Đã hoàn thành và chốt kiểm thử **Bước 3.1: Nhiều trang, cuộn dọc liền mạch & tự lưu bản nháp (P1)** (đã gắn tag `step-3.1-ok`). Chuẩn bị bắt tay thực hiện **Bước 3.2: Thanh thumbnail trang (P1)**.

## 2. File đã sửa trong phiên gần nhất
- `crates/inknote-core/src/lib.rs`: Nâng cấp `Document` nhiều trang (`DocumentPage`), thêm `InsertPageAfter` / `DeletePage` kèm Undo/Redo, lưu/mở `.inknote` nhiều trang.
- `crates/inknote-app/src/autosave.rs`: Tự động lưu bản nháp tạm (`autosave_draft.inknote`) kể cả khi chưa chọn file, khôi phục sau crash và dọn bản nháp khi lưu hoặc chọn Don't Save.
- `crates/inknote-app/src/toolbar.rs` & `theme.rs`: Nối nút Add Page, mục Pages trong menu `...`, hiển thị `(trang/tổng)` giữa tên file và dấu chấm lưu, chữ `"Auto-saved"` màu trắng.
- `crates/inknote-app/src/main.rs`: Cuộn dọc liền mạch, kéo mạnh quá đáy trang cuối để tự thêm trang, `PageUp`/`PageDown`, hộp thoại xác nhận xoá trang, chặn đóng app hỏi lưu khi chưa lưu file chính thức.
- `docs/roadmap.md` & `memory.md`: Cập nhật hoàn thành Bước 3.1 (tag `step-3.1-ok`), chuyển mục tiêu sang Bước 3.2.

## 3. Quyết định vừa chốt & lý do
- **Trạng thái trên Navbar**: Đặt `Tên file` `(X/Y)` + `Chấm xanh/đỏ` + chữ `"Auto-saved"` màu trắng (hiện 3s rồi ẩn) ngay bên phải nút Read-Only.
- **Tạo trang mới & kéo đáy**: Thêm trang chèn ngay sau trang đang viết (`active_page_index + 1`), giữ nguyên cài đặt giấy; kéo mạnh vượt ngưỡng ở đáy trang cuối tự tạo trang mới.
- **Xác nhận an toàn**: Hỏi xác nhận khi bấm xoá trang (vẫn Undo được) và hỏi lưu/xoá nháp khi bấm tắt app lúc chưa lưu file `.inknote`.

## 4. Việc đang dở & Bước tiếp theo
- **Đang dở**: Không có việc dở dang; codebase sạch và pass toàn bộ bài test (`13` test `inknote-core`, `13` test `inknote-app`).
- **Bước tiếp theo (Bước 3.2: Thanh thumbnail trang)**: Bật/tắt cột thumbnail các trang bằng nút **Danh sách trang** (icon hai trang) trên thanh trên, bấm thumbnail để nhảy trang, thêm/xoá trang trực tiếp trên thanh thumbnail.

## 5. Cảnh báo & Đã thử không được
- **Vẽ liên tục bằng `request_repaint()` + `AutoNoVsync`**: Làm máy quá tải do đẩy FPS vượt ngưỡng màn hình kể cả lúc không vẽ. Luôn giữ `PresentMode::Fifo` và chỉ gọi `request_repaint_after(...)` khi thực sự có timer chờ.
- **Pop-up egui**: Luôn phải đặt `close_behavior(PopupCloseBehavior::CloseOnClickOutside)` để pop-up không biến mất khi bấm chuột bên trong.
