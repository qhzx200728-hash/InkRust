# memory.md – Bộ nhớ trạng thái dự án InkNote

## Mục tiêu hiện tại
- Chuẩn bị thực hiện **Bước 2.3: Auto-save (P1)** – tự động lưu ra file tạm rồi atomic rename đè lên file chính sau 2–3s kể từ khi có thay đổi (nét vẽ, cấu hình giấy, xoá trang) đối với tài liệu đã có đường dẫn lưu.

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
- **Bước 1.6 – Khung giao diện P0 & Sửa căn chỉnh UI Alignment** (`commit ed788b5`, `commit 04980fc`, tag `step-1.6-ok`):
  - Tạo và cập nhật `crates/inknote-app/src/theme.rs`, `crates/inknote-app/src/icons.rs`, `crates/inknote-app/src/toolbar.rs`, `crates/inknote-app/src/main.rs`.
  - Căn chỉnh chuẩn tâm cửa sổ bằng `show_centered_in_bounds`, đồng bộ chiều cao hàng `tool_row_height = 26.0`, thêm unit test kiểm tra toạ độ ở `1200×800`, `1366×768`, `1600×900` (pass `1/1`).
- **Bước 1.7 – Undo/Redo cho nét bút** (`commit a97b545`, tag `step-1.7-ok`):
  - Tạo crate `crates/inknote-core`, cài đặt `Document`, `Command`, hệ thống Undo/Redo cho `AddStroke`, `AddStrokes`, `ClearPage`; tích hợp phím tắt `Ctrl+Z`, `Ctrl+Y` / `Ctrl+Shift+Z` và nút Redo trên toolbar.
- **Bước 1.8 – Zoom/kéo trang Pan & Zoom + Test hiệu năng 2000 nét** (`commit b9ebc47`, `commit be4089e`, tag `step-1.8-ok` — người dùng xác nhận đạt 200 FPS rất mượt):
  - Thêm `view.rs` xử lý toạ độ màn hình ↔ trang, zoom giữ tâm con trỏ, kéo trang bằng chuột giữa hoặc Space + chuột trái.
  - Hộp thoại xác nhận xoá trang an toàn (vẫn hoàn tác được bằng Undo).
  - Tối ưu nét đang viết (`active`): chốt theo đợt 16 đoạn, giữ vững 144–146 FPS khi viết tay.
  - Tối ưu 2000 nét tĩnh: gom 1 `Mesh`, đặt trước dung lượng `reserve` (giảm 57% thời gian build mesh), nâng tolerance Lyon lên 0.5 (giảm 25% số đỉnh thừa).
  - Tối ưu GPU & VSync: chuyển `present_mode` sang `AutoNoVsync`, tắt `debug-assertions` trong profile dev, render khung chỉ 0.84ms (tương đương 1.190 FPS CPU), người dùng test thật đạt 200 FPS.
- **Bước 2.1 – Thiết kế định dạng file & Lưu/Mở file (`Ctrl+S`, `Ctrl+O`)** (`commit 98a04fd`, tag `step-2.1-ok` — người dùng xác nhận test OK):
  - Định dạng file `DocumentFile`: `format_version = 1`, `metadata`, `paper_settings`, `pages` chứa `objects` (`PageObject::BrushStroke`), `first_page_thumbnail`.
  - Cơ chế ghi an toàn (atomic save): ghi vào file tạm `.tmp` rồi đổi tên đè lên file chính, chống hỏng file khi tắt ngang.
  - Phím tắt `Ctrl+S` (Lưu), `Ctrl+Shift+S` (Lưu thành file mới), `Ctrl+O` (Mở file), `Ctrl+N` (Tạo ghi chú mới).
  - Tích hợp 3 nút Save, Open, New trong menu More (icon 3 chấm) trên toolbar.
  - Hộp thoại Modal hỏi lưu thay đổi (`Save changes?`) khi mở file khác hoặc tạo mới lúc có nét vẽ chưa lưu.
  - Cập nhật tiêu đề cửa sổ kèm dấu hoa thị `*` khi tài liệu bị sửa đổi chưa lưu.
- **Bước 2.2 – Paper Settings (P1)** (`commit 5c76800`, `ed09af0`, `28c5765`, `414d853`, `0927a84`, tag `step-2.2-ok` — người dùng xác nhận test OK):
  - `crates/inknote-core/src/lib.rs`: Bổ sung `CanvasMode` (Paper/Whiteboard), `PaperSize` (A4/A5), `PaperOrientation` (Portrait/Landscape), mở rộng `PaperSettings`.
  - `crates/inknote-app/src/icons.rs`: Viết hàm vẽ nền `draw_paper` và tính khung tờ giấy `paper_sheet_rect`. Chế độ Paper có viền và đổ bóng mềm (drop shadow), có vạch lề hồng pastel; chế độ Whiteboard trải phẳng tràn màn hình 100% không có vạch lề.
  - `crates/inknote-app/src/toolbar.rs`: Thêm cụm cấu hình Paper Settings vào menu More `...` (Canvas Mode, Paper Format, Template, Spacing, Margins). Cấu hình `PopupCloseBehavior::CloseOnClickOutside` cho tất cả pop-up để không bị tự tắt khi bấm chuột bên trong.
  - `crates/inknote-app/src/main.rs`: Tự động căn giữa tờ giấy A4/A5 khi mở và khi bấm `Ctrl+0`; `drawable_rect` chỉ cho phép vẽ trên tờ giấy ở chế độ Paper (không vẽ lem ra bàn xám); clip nét vẽ gọn gàng trong trang; lưu thiết lập mặc định vào `%APPDATA%/InkNote/default_paper.json`.

## Đang làm dở
- Không có task nào dở dang. Bước 2.2 đã hoàn thành và gắn tag `step-2.2-ok`. Sẵn sàng bắt tay ngay vào Bước 2.3 (Auto-save).

## Quyết định đã chốt và lý do (không bàn lại)
- **Bỏ hoàn toàn bản cũ (Electron/Flutter/web)**: viết mới 100% bằng Rust cho Windows.
- **Chọn hướng B (`egui 0.37` + Rust thuần)**: đọc được lực nhấn (pressure) bút Wacom trực tiếp trên Windows mà không cần GTK4/Cairo.
- **Giấy phép `GPL-3.0-or-later`**: dùng cá nhân và đẩy toàn bộ source lên GitHub; copy nguyên văn code nét bút từ Rnote (GPLv3, ghi rõ nguồn); chỉ đọc ý tưởng từ Xournal++ (GPLv2) và tự viết lại.
- **Port nguyên văn `rnote-compose` sang `crates/inknote-pen`**: giữ nguyên 100% thuật toán làm mượt, `PressureCurve`, `SmoothOptions`, `BrushConfig`; ánh xạ `piet-cairo = { package = "inknote-render" }` để không sửa code gốc Rnote.
- **Quyết định giao diện (`04-ui-spec.md`)**:
  - Ngôn ngữ UI mặc định: **Tiếng Anh** (nạp font `segoeui.ttf` hỗ trợ 100% dấu tiếng Việt).
  - Giữ màu **xanh dương** cho Thanh công cụ chính, giữ navbar tối giản (không thêm nút thừa).
  - Quy tắc Tẩy (Eraser): trên thanh có 2 chế độ (Tẩy thường, Tẩy nét) + Pop-up cài đặt nâng cao; mặc định chỉ xoá **nét bút**, không bao giờ xoá LaTeX/hình/ảnh.
  - 2 chế độ Canvas: `Paper` (trang chuẩn A4/A5 có bóng đổ trên bàn xám) và `Whiteboard` (bảng trắng tràn màn hình, không có đường lề).
  - Tất cả pop-up của `egui` phải có `PopupCloseBehavior::CloseOnClickOutside` để giữ mở khi tương tác.
- **Quy tắc Git & Rollback**: Mỗi bước nhỏ = 1 commit; chỉ dùng `git revert` (không dùng `git reset --hard`); gắn tag `step-X.Y-ok` sau khi người dùng báo OK.

## Đã thử nhưng không được, và vì sao
- **Dùng trực tiếp crate `rnote-compose` gốc**: phụ thuộc cứng vào `piet-cairo` (đòi C `cairo`). Đã xử lý bằng `inknote-pen` + `inknote-render`.
- **Chuyển PDF sang SVG bằng `dvisvgm --pdf`**: lỗi vì Ghostscript `10.08` trong TeX Live 2026 không tương thích. Đã đổi sang `latex` → `.dvi` → `dvisvgm --no-fonts`.
- **Dùng font mặc định của `egui` (`Ubuntu-Light.ttf`)**: thiếu dấu phụ kép tiếng Việt (`ắ ằ ẳ ẵ ặ`). Đã xử lý bằng nạp `C:\Windows\Fonts\segoeui.ttf`.
- **Gán `pressure = 0.5` khi `TouchPhase::End` có `force: None`**: làm cuối nét bút Wacom phình giọt nước. Đã sửa bằng truyền `pressure = 0.0` khi nhấc bút.
- **Vẽ ngay ở `TouchPhase::Move` khi chưa có `TouchPhase::Start`**: làm bút Wacom hover đã tự vẽ. Đã chặn bằng kiểm tra `self.touch_active && self.active.is_some()`.
- **Căn giữa bằng `ui.columns(3)` và `ui.vertical_centered`**: `ui.horizontal` bên trong luôn bắt đầu từ mép trái vùng chứa nên lệch tâm. Đã thay bằng `show_centered_in_bounds`.
- **Đặt nút chuyển Canvas Mode ngay trên thanh Navbar**: Người dùng chê thừa và rối thanh công cụ; đã gỡ bỏ và gom toàn bộ vào menu More `...`.
- **Dùng pop-up mặc định của `egui` (`CloseOnClick`)**: Vừa bấm nút hoặc kéo slider bên trong là pop-up tự biến mất. Đã sửa bằng `PopupCloseBehavior::CloseOnClickOutside`.
- **Vẽ đường lề ở chế độ Whiteboard**: Làm mất tính chất bảng trắng vô tận; đã sửa để Whiteboard hoàn toàn không có đường lề và tràn phẳng viền.

## Lỗi/vấn đề đang tồn tại
- 21 cảnh báo `missing_docs` trong `crates/inknote-pen` (do giữ nguyên code gốc Rnote).
- Nút bấm phụ trên thân bút / đầu tẩy phía sau bút Wacom: chưa kiểm tra `egui` có nhận riêng được không (để sau).

## Việc tiếp theo (xếp theo thứ tự ưu tiên)
1. **Bước 2.3: Auto-save (P1)**: Tự động lưu sau 2–3s debounce khi có thay đổi (vẽ, xoá, đổi template/khổ giấy), dùng atomic save (.tmp rename).
2. **Bước 3 (3.1 → 3.2)**: Nhiều trang & Thanh thumbnail trang (P1).
3. **Bước 4 (4.1 → 4.5)**: Công thức LaTeX kiểu Xournal++ (`inknote-latex`, Live Preview, chèn/double-click sửa, lưu file).
4. **Bước 5 (5.1 → 5.4)**: Tẩy (2 chế độ + Pop-up nâng cao, chỉ xoá nét bút), Lasso, Hình học, Văn bản.
5. **Bước 6 (6.1 → 6.2)**: Chèn ảnh + nút dán ảnh từ clipboard, giữ ảnh sắc nét theo zoom.
6. **Bước 7 (7.1 → 7.4)**: Màn hình Library (P3), Trash, xuất PDF/Ảnh, `README.md`.

## Cập nhật lần cuối
- **Ngày**: 2026-11-13
- **Model đã dùng**: Gemini (Antigravity IDE)
