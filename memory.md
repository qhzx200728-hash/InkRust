# memory.md – Bộ nhớ trạng thái dự án InkNote

## Mục tiêu hiện tại
- Đã hoàn tất và được người dùng xác nhận ổn **Bước 1.6** (Khung giao diện P0 & căn chỉnh chuẩn). Chuẩn bị bắt đầu **Bước 1.7** (Undo/Redo cho nét bút qua hệ thống lệnh Command trong `inknote-core`).

## Đã hoàn thành (kèm tên file đã tạo/sửa)
- **Bước 0 – Spike (Thử nghiệm kỹ thuật)** (`commit fb30599`):
  - Cài Rust `1.99.1`, clone `reference/rnote` (commit `83a76367dbd3d6c461c10553c45231799a25ea4b`) và `reference/xournalpp` (chỉ đọc).
  - Kiểm chứng Wacom pressure (`spike/egui-pressure`), làm mượt `ink-stroke-modeler-rs` (`spike/modeler-test`), chạy `latex` → DVI → `dvisvgm --no-fonts` (`spike/latex`).
  - Tạo tài liệu: `docs/00-rnote-analysis.md`, `docs/01-tech-options.md`, `docs/02-architecture.md`, `docs/roadmap.md`, `docs/spike-result.md`, `GEMINI.md`.
- **Bước 1.1 – 1.4 – Port nguyên văn nét bút Rnote & Sửa lỗi bút Wacom** (`commit a244fad`, `commit f9a4b60`, tag `step-1.4-ok`):
  - Tạo `Cargo.toml`, `crates/inknote-pen/` (copy nguyên văn 44 file từ `rnote-compose/src/` + `brushconfig.rs`), `crates/inknote-render/src/lib.rs` (`lyon_tessellation`, `FillRule::NonZero`), `docs/03-port-rnote.md`.
  - Sửa lỗi phình giọt nước cuối nét và lỗi hover tự vẽ trên bút Wacom trong `crates/inknote-app/src/main.rs`.
- **Bước 1.5 – Giấy phép GPL-3.0** (`commit 4ec3161`, tag `step-1.5-ok`):
  - Tạo `LICENSE` (toàn văn GPL-3.0) và `NOTICE` (ghi nguồn Rnote + commit hash, ý tưởng Xournal++, bộ icon).
- **Bước 1.6 – Khung giao diện P0 & Sửa căn chỉnh UI Alignment** (`commit ed788b5`, `commit 04980fc`, tag `step-1.6-ok` — người dùng đã xác nhận "Đã ổn"):
  - Tạo và cập nhật `crates/inknote-app/src/theme.rs`, `crates/inknote-app/src/icons.rs`, `crates/inknote-app/src/toolbar.rs`, `crates/inknote-app/src/main.rs`.
  - Căn chỉnh chuẩn tâm cửa sổ bằng `show_centered_in_bounds`, đồng bộ chiều cao hàng `tool_row_height = 26.0`, thêm unit test kiểm tra toạ độ ở `1200×800`, `1366×768`, `1600×900` (pass `1/1`).
- **Thiết lập bộ nhớ dự án cho đa model**:
  - Tạo `AGENTS.md`, `memory.md`, `HANDOFF.md` và tinh gọn `GEMINI.md`.

## Đang làm dở
- **Bước 1.7 đã code xong, chờ người dùng test thủ công** (chưa gắn tag step-1.7-ok): tạo crates/inknote-core/src/lib.rs (Document, Command::{AddStroke,ClearPage}, undo/redo, 4 unit test pass); nối vào main.rs (bộ đệm atch_cache), thêm icon/nút Redo (icons.rs, 	oolbar.rs). Không có việc dở ở Bước 1.6 (đã chốt `step-1.6-ok`). Bắt đầu phiên mới là làm ngay **Bước 1.7**.

## Quyết định đã chốt và lý do (không bàn lại)
- **Bỏ hoàn toàn bản cũ (Electron/Flutter/web)**: viết mới 100% bằng Rust cho Windows.
- **Chọn hướng B (`egui 0.37` + Rust thuần)**: đọc được lực nhấn (pressure) bút Wacom trực tiếp trên Windows mà không cần GTK4/Cairo.
- **Giấy phép `GPL-3.0-or-later`**: dùng cá nhân và đẩy toàn bộ source lên GitHub; copy nguyên văn code nét bút từ Rnote (GPLv3, ghi rõ nguồn); chỉ đọc ý tưởng từ Xournal++ (GPLv2) và tự viết lại.
- **Port nguyên văn `rnote-compose` sang `crates/inknote-pen`**: giữ nguyên 100% thuật toán làm mượt, `PressureCurve`, `SmoothOptions`, `BrushConfig`; ánh xạ `piet-cairo = { package = "inknote-render" }` để không sửa code gốc Rnote.
- **Quyết định giao diện (`04-ui-spec.md`)**:
  - Ngôn ngữ UI mặc định: **Tiếng Anh** (nạp font `segoeui.ttf` hỗ trợ 100% dấu tiếng Việt).
  - Giữ nút **Read-Only** và giữ màu **xanh dương** cho Thanh công cụ chính.
  - Quy tắc Tẩy (Eraser): trên thanh có 2 chế độ (Tẩy thường, Tẩy nét) + Pop-up cài đặt nâng cao; mặc định chỉ xoá **nét bút**, không bao giờ xoá LaTeX/hình/ảnh.
- **Quy tắc Git & Rollback**: Mỗi bước nhỏ = 1 commit; chỉ dùng `git revert` (không dùng `git reset --hard`); gắn tag `step-X.Y-ok` sau khi người dùng báo OK.

## Đã thử nhưng không được, và vì sao
- **Dùng trực tiếp crate `rnote-compose` gốc**: phụ thuộc cứng vào `piet-cairo` (đòi C `cairo`). Đã xử lý bằng `inknote-pen` + `inknote-render`.
- **Chuyển PDF sang SVG bằng `dvisvgm --pdf`**: lỗi vì Ghostscript `10.08` trong TeX Live 2026 không tương thích. Đã đổi sang `latex` → `.dvi` → `dvisvgm --no-fonts`.
- **Dùng font mặc định của `egui` (`Ubuntu-Light.ttf`)**: thiếu dấu phụ kép tiếng Việt (`ắ ằ ẳ ẵ ặ`). Đã xử lý bằng nạp `C:\Windows\Fonts\segoeui.ttf`.
- **Gán `pressure = 0.5` khi `TouchPhase::End` có `force: None`**: làm cuối nét bút Wacom phình giọt nước. Đã sửa bằng truyền `pressure = 0.0` khi nhấc bút.
- **Vẽ ngay ở `TouchPhase::Move` khi chưa có `TouchPhase::Start`**: làm bút Wacom hover đã tự vẽ. Đã chặn bằng kiểm tra `self.touch_active && self.active.is_some()`.
- **Căn giữa bằng `ui.columns(3)` và `ui.vertical_centered`**: `ui.horizontal` bên trong luôn bắt đầu từ mép trái vùng chứa nên lệch tâm. Đã thay bằng `show_centered_in_bounds`.

## Lỗi/vấn đề đang tồn tại
- 21 cảnh báo `missing_docs` trong `crates/inknote-pen` (do giữ nguyên code gốc Rnote) và 1 cảnh báo `unused_assignments` nhỏ trong `crates/inknote-render/src/lib.rs` (sẽ dọn ở Bước 7.4).
- Nút bấm phụ trên thân bút / đầu tẩy phía sau bút Wacom: chưa kiểm tra `egui` có nhận riêng được không (để sau).

## Việc tiếp theo (xếp theo thứ tự ưu tiên)
1. **Bước 1.7**: Tạo crate `crates/inknote-core`, chuyển danh sách nét bút sang cấu trúc trang + lịch sử lệnh Command để hỗ trợ **Undo/Redo** (`Ctrl+Z`, `Ctrl+Y` / `Ctrl+Shift+Z`, và nút Undo trên toolbar).
2. **Bước 1.8**: **Zoom/kéo trang (Pan & Zoom)** + **Test hiệu năng 2000 nét** vẫn mượt.
3. **Bước 2 (2.1 → 2.3)**: Định dạng file (`format_version`, kiểu đối tượng, `first_page_thumbnail`), lưu/mở file (`Ctrl+S`), Paper Settings (P1), Auto-save.
4. **Bước 3 (3.1 → 3.2)**: Nhiều trang & Thanh thumbnail trang (P1).
5. **Bước 4 (4.1 → 4.5)**: Công thức LaTeX kiểu Xournal++ (`inknote-latex`, Live Preview, chèn/double-click sửa, lưu file).
6. **Bước 5 (5.1 → 5.4)**: Tẩy (2 chế độ + Pop-up nâng cao, chỉ xoá nét bút), Lasso, Hình học, Văn bản.
7. **Bước 6 (6.1 → 6.2)**: Chèn ảnh + nút dán ảnh từ clipboard, giữ ảnh sắc nét theo zoom.
8. **Bước 7 (7.1 → 7.4)**: Màn hình Library (P3), Trash, xuất PDF/Ảnh, `README.md`.

## Cập nhật lần cuối
- **Ngày**: 2026-11-12
- **Model đã dùng**: Gemini (Antigravity IDE)
