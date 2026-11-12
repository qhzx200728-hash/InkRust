# HANDOFF.md – Bàn giao trạng thái cho Model tiếp theo

## 1. Task hiện tại & Tiến độ
Dự án **InkNote** (app ghi chú viết tay trên Windows bằng Rust + `egui 0.37` + bút Wacom) đã hoàn thành **Bước 1.7 (Undo/Redo, tag `step-1.7-ok`)**. Đang ở **Bước 1.8 (Zoom/Pan + test hiệu năng 2000 nét)**: đã code xong zoom/kéo trang, hộp xác nhận Clear Page, phím F9 tạo 2000 nét mẫu và đã hoàn tất tối ưu chống tụt FPS khi viết tay & khi vẽ 2000 nét. Đang chờ người dùng test lại.

## 2. Các file đã tạo/sửa trong phiên này
- `crates/inknote-core/src/lib.rs`: Thêm trường `rev: u64` và hàm `pub fn revision(&self) -> u64` trên `Document` để theo dõi thay đổi trang.
- `crates/inknote-app/src/main.rs`:
  - Tối ưu nét đang viết (`active`): chốt dần theo đợt 16 đoạn (`advance_active_committed`), mỗi khung chỉ vẽ phần đuôi ngắn (< 16 đoạn) + đầu bút modeler.
  - Tối ưu nét tĩnh: gom các nét tĩnh thành 1 `egui::epaint::Mesh` duy nhất và cache lại (`StaticKey`), tái sử dụng khi đang viết nét hoặc không đổi góc nhìn.
  - Dọn dẹp modal xác nhận xoá trang bị lặp.
- `crates/inknote-app/src/view.rs`: Bộ chuyển đổi toạ độ màn hình ↔ toạ độ trang theo zoom/pan.
- `memory.md`, `docs/roadmap.md`: Cập nhật tiến độ.

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

