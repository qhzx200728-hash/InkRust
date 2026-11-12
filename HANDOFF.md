# HANDOFF.md – Bàn giao trạng thái cho Model tiếp theo

## 1. Task hiện tại & Tiến độ
Dự án **InkNote** (app ghi chú viết tay trên Windows bằng Rust + `egui 0.37` + bút Wacom) vừa hoàn thành **Bước 1.6 (Khung giao diện P0 & căn chỉnh chuẩn)** và đã được người dùng xác nhận ổn (`step-1.6-ok`). Phiên tiếp theo bắt đầu ngay **Bước 1.7**.

## 2. Các file đã tạo/sửa trong phiên này
- `LICENSE`, `NOTICE`: Thêm toàn văn GPL-3.0 và ghi công nguồn Rnote (commit `83a76367dbd3d6c461c10553c45231799a25ea4b`), ý tưởng Xournal++, bộ icon.
- `crates/inknote-app/src/theme.rs`: Gom toàn bộ bảng màu, cỡ chữ, bo góc, `tool_row_height = 26.0` và nạp font `C:\Windows\Fonts\segoeui.ttf`.
- `crates/inknote-app/src/icons.rs`: Vẽ toàn bộ icon vector trên toolbar và nền giấy chấm (`draw_dotted_paper`).
- `crates/inknote-app/src/toolbar.rs`: Xây dựng Navigation Toolbar (3 nhóm Trái–Giữa–Phải), Contextual Tool Options Bar (pill tối căn giữa qua `show_centered_in_bounds`) và các Pop-up tối bo góc.
- `crates/inknote-app/src/main.rs`: Kết nối giao diện P0 với bộ vẽ nét Rnote, xử lý sự kiện bút Wacom và thêm unit test kiểm tra căn chỉnh ở 3 độ phân giải (`1200×800`, `1366×768`, `1600×900`).
- `AGENTS.md`, `memory.md`, `GEMINI.md`, `docs/roadmap.md`, `docs/02-architecture.md`: Thiết lập bộ nhớ dự án và lộ trình P0→P3.

## 3. Quyết định đã chốt
- Dùng `egui 0.37` thuần Rust; port nguyên văn `rnote-compose` sang `inknote-pen` + tô lưới bằng `lyon` (`FillRule::NonZero`).
- Giao diện mặc định **Tiếng Anh**, hỗ trợ 100% dấu tiếng Việt; giữ nút Read-Only và thanh công cụ chính màu **xanh dương**.
- Tẩy (Eraser) có 2 chế độ + Pop-up nâng cao, mặc định chỉ xoá nét bút (không bao giờ xoá LaTeX/hình/ảnh).

## 4. Bước tiếp theo cụ thể (Làm ngay)
- **Thực hiện Bước 1.7 (`docs/roadmap.md`)**: Tạo crate `crates/inknote-core`, cài đặt hệ thống lệnh Command cho **Undo/Redo nét bút** (`Ctrl+Z`, `Ctrl+Y` / `Ctrl+Shift+Z` và nút Undo/Redo trên toolbar), kèm cách test thủ công và lệnh rollback `git revert`.

## 5. Cảnh báo quan trọng
- **Không sửa** `reference/` (chỉ đọc) và không sửa thuật toán gốc Rnote trong `crates/inknote-pen`.
- **Wacom trên Windows**: Khi `TouchPhase::End` có `force: None` phải truyền `pressure = 0.0` (nếu truyền `0.5` cuối nét sẽ phình giọt nước); chỉ vẽ ở `TouchPhase::Move` khi `self.touch_active && self.active.is_some()` (tránh lỗi bút hover tự vẽ).
- **Layout `egui`**: Không dùng `ui.columns(3)` hay `ui.vertical_centered` cho thanh công cụ vì sẽ lệch tâm cửa sổ; giữ nguyên hàm `show_centered_in_bounds`.
- Trong headless test `ctx.run_ui`, phải gọi `out.textures_delta.clear()` để tránh panic của `epaint 0.37`.
