# memory.md – Bộ nhớ trạng thái dự án InkNote

## Mục tiêu hiện tại
- Vừa hoàn thành và gắn tag **`step-7.2-ok`** (**Bước 7.2: Quản lý thư mục, Menu thẻ & Thùng rác (Trash – P3)**).
- Đang chuyển sang **Bước 7.3 – Xuất PNG chất lượng cao / PDF nhiều trang cực nét & Chế độ chỉ đọc (P3)** theo `docs/roadmap.md`.

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

- **Bước 2.3 – Auto-save & Hãm FPS theo tần số quét màn hình** (`commit 4f70eb2`, `522f970`, tag `step-2.3-ok` — người dùng xác nhận test OK: 144Hz đạt 144 FPS, 60Hz hạ đúng 60 FPS, 20.000 nét mới tụt về 45 FPS):
  - `crates/inknote-core/src/lib.rs`: Thêm `set_paper_settings` (tăng revision để kích hoạt dirty/auto-save), `mark_saved_at` (chính xác mốc `rev` khi lưu luồng nền), và cải thiện `DocumentFile::save_to_path` dùng file tạm cùng thư mục (`.<name>.tmp`) + `sync_all()` + atomic `std::fs::rename`.
  - `crates/inknote-app/src/autosave.rs`: Cài đặt `AutoSaveState` chạy ở luồng nền sau 2,5 giây debounce (khi tài liệu đã có đường dẫn lưu và không vẽ dở), hỗ trợ `flush_on_exit` khi tắt app bình thường.
  - `crates/inknote-app/src/toolbar.rs` & `theme.rs`: Hiển thị cụm `Tên file + Chấm xanh/đỏ + Thông báo "Auto-saved" (chỉ hiện 3s sau mỗi lần tự lưu rồi tự ẩn)` ngay bên phải nút Read-Only Mode trên thanh công cụ chính.
  - `crates/inknote-app/src/main.rs`: Bật VSync (`PresentMode::Fifo`) để hãm FPS tối đa đúng theo tần số quét màn hình (144Hz / 60Hz) và bỏ `ctx.request_repaint()` liên tục (dùng reactive repaint + `request_repaint_after` cho bộ đếm auto-save) để không gây quá tải máy khi đứng yên.

- **Bước 3.1 – Nhiều trang, cuộn dọc liền mạch & tự lưu bản nháp** (`commit 447d5c6`, `c37d070`, `c75c392`, `8c61230`, tag `step-3.1-ok` — người dùng xác nhận test OK):
  - `crates/inknote-core/src/lib.rs`: Thêm `DocumentPage`, chuyển `Document` sang nhiều trang (`pages: Vec<DocumentPage>`, `active_page_idx`), thêm `Command::InsertPageAfter`, `Command::DeletePage` (kèm Undo/Redo và giữ ít nhất 1 trang cuối), cập nhật lưu/mở `.inknote` nhiều trang.
  - `crates/inknote-app/src/main.rs`: Xếp trang cuộn dọc liền mạch (`PAGE_GAP`), tự cập nhật chỉ số trang khi cuộn, chuyển trang bằng `PageUp`/`PageDown`, kéo mạnh quá đáy trang cuối (`PULL_TO_ADD_THRESHOLD`) tự thêm trang mới sau trang đang viết (giữ nguyên `PaperSettings`), hộp thoại xác nhận xoá trang (`Delete Page X?`), và hộp thoại hỏi lưu khi tắt app (`Save before closing?`) nếu ghi chú có thay đổi chưa lưu thành file `.inknote`.
  - `crates/inknote-app/src/autosave.rs`: Tự động lưu bản nháp tạm `%APPDATA%/InkNote/autosave_draft.inknote` sau 2,5s ngay cả khi chưa chọn file chính thức (chỉ khôi phục nếu app bị tắt ngang; xoá bản nháp khi người dùng chọn `Don't Save` hoặc lưu thành file `.inknote`).
  - `crates/inknote-app/src/toolbar.rs` & `theme.rs`: Hiển thị `(trang-hiện-tại/tổng-số-trang)` giữa tên file và dấu chấm lưu trên Navbar, chữ `"Auto-saved"` màu trắng (`Color32::WHITE`), thêm nút Add Page và mục Pages trong menu More `...`.

- **Bước 3.2 – Thanh thumbnail trang 2 cột & menu popup từng trang** (`commit 404e3c4`, `0ff62bf`, `e8fad1f`, `3c5fa93`, tag `step-3.2-ok` — người dùng xác nhận test OK):
  - `crates/inknote-app/src/thumbnails.rs`: Bố cục 2 cột kiểu GoodNotes (tiêu đề `Pages` + nút `×`, không có nút Add Page trong panel, không có đường kẻ đen phân cách), bộ nhớ đệm lưới `ThumbCache` riêng cho từng `page.id`, mỗi thẻ có số trang bên trái và nút `⌄` bên phải mở menu popup `172px` (`Add Page Before`, `Add Page After`, `Duplicate`, `Clear Page`, `Delete Page`).
  - `crates/inknote-core/src/lib.rs`: Thêm `Command::InsertPageBefore`, `Command::DuplicatePage`, `Command::ClearPageAt` hỗ trợ đầy đủ Undo/Redo.
  - `crates/inknote-app/src/toolbar.rs`, `theme.rs`, `main.rs`: Đổi `show_thumbnails_hint` → `show_thumbnails`, dùng `egui::Panel::left(...).show_separator_line(false)`, cập nhật `confirm_clear` & `confirm_delete_page` thành `Option<usize>` theo từng trang cụ thể.

- **Bước 4.1 – 4.5 – Công thức LaTeX kiểu Xournal++** (`commit 308eee7`, `d49a24b`, `73c2655`, `bd3cb96`, `ee6399a`, các tag `step-4.1-ok` → `step-4.5-ok` — người dùng xác nhận test OK):
  - `crates/inknote-latex/`, `latex_dialog.rs`, `svg_cache.rs`: Biên dịch `latex -no-shell-escape` → `.dvi` → `dvisvgm --no-fonts` có timeout, hộp thoại Live Preview ~0,5s, chèn/double-click giữ vị trí với 1 bước Undo, re-rasterize sắc nét theo display size khi zoom/kéo to, lưu `source` + `svg_cache` vào `.inknote`.

- **Bước 5.1 – 5.4 – Tẩy, Lasso, 18 Hình học, Văn bản tiếng Việt** (`commit f0ba7fd..2a67ecf`, `c553341`, các tag `step-5.1-ok` → `step-5.4-ok` — người dùng xác nhận test OK):
  - `eraser.rs`: 2 chế độ Standard & Stroke, mặc định chỉ tẩy nét bút (không bao giờ tẩy LaTeX/hình/văn bản/ảnh).
  - `lasso.rs`: Freehand & Rectangle, bộ lọc từng loại đối tượng, di chuyển, kéo 4 góc đồng tỷ lệ, xoá 1 bước Undo.
  - `shapes.rs`: 18 loại hình học + giữ `Ctrl` căn góc đặc biệt (`0°/30°/45°/60°/90°`) & hình đều.
  - `text_box.rs`: Hộp văn bản tiếng Việt đủ dấu (`segoeui.ttf`), font/cỡ/đậm/nghiêng/căn lề/danh sách/màu sắc, Undo/Redo & lưu `.inknote`.

- **Bước 6.1 – Chèn ảnh từ file & Dán ảnh (`Ctrl+V` / nút Paste Image)** (`commit 2b93911`, `73ccdd3`, tag `step-6.1-ok` — người dùng xác nhận test OK):
  - `crates/inknote-core/src/lib.rs`: `ImageObject` (lưu byte ảnh gốc qua base64), `PageObject::Image`, `Command::AddImage`, tích hợp `TransformSelection`/`DeleteSelection`/`DuplicatePage`/Undo/Redo.
  - `crates/inknote-app/src/image_box.rs`: Đọc file PNG/JPEG/BMP/WebP giữ nguyên byte gốc, đọc ảnh clipboard (`arboard`) + mã hoá `ColorImage` (`encode_color_image_to_png`) thành PNG không mất nét, cache texture độ phân giải gốc (`MAX_GPU_TEXTURE_SIDE = 4096`), `TextureOptions::LINEAR`.
  - `crates/inknote-app/src/toolbar.rs`, `lasso.rs`, `main.rs`: Thanh tuỳ chọn có **Open File...**, **Paste Image**, **Delete**, thông báo tự ẩn 3s; vẽ ảnh **bên dưới nét bút**; chọn/di chuyển/kéo 4 góc giữ tỷ lệ; xử lý `egui::Event::PasteImage` và `egui::Event::Paste` cho phím tắt `Ctrl+V`; bảo vệ hoàn toàn trước Tẩy thường/Tẩy nét và hỗ trợ công tắc lọc Lasso.

- **Bước 6.2 – Giữ ảnh sắc nét theo mức zoom & Kiểm tra tổng hợp với Tẩy** (`commit 64246ad`, tag `step-6.2-ok` — người dùng xác nhận test OK):
  - `crates/inknote-app/src/image_box.rs`: Lưu nguyên `ImageObject.bytes`, đọc động giới hạn GPU `max_texture_side` (`clamp(512, 8192)`), bật mipmap `TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear))`, dùng `FilterType::Lanczos3` cho ảnh quá cỡ và tự nâng cấp texture nếu giới hạn GPU tăng.
  - `crates/inknote-app/src/thumbnails.rs` & `main.rs`: Hiển thị ảnh trang trong thanh thumbnail Pages (bên dưới nét bút) và kiểm thử tích hợp ở zoom 300% xác nhận cả `Standard` và `Stroke` Eraser chỉ xoá nét bút, giữ nguyên LaTeX/Hình/Text/Ảnh.

- **Bước 7.1 – Giao diện Library (P3)** (`commit cf8f320`, `1aa3dff`, tag `step-7.1-ok` — người dùng xác nhận test OK):
  - `crates/inknote-core/src/library.rs` & `lib.rs`: Thêm `LibraryStore` quản lý thư mục & file `.inknote` trong `Documents`/`Trash`, cache quét theo `(mtime, size)`, tìm kiếm đệ quy theo tên tiếng Việt đủ dấu, tạo thư mục/ghi chú, lưu ghi chú chưa đặt tên khi về Home; bổ sung `DocumentMetadata.is_favorite`.
  - `crates/inknote-app/src/library_view.rs`, `theme.rs`, `icons.rs`, `thumbnails.rs`, `toolbar.rs`, `main.rs`: Giao diện Library (Sidebar ~18% `Documents`/`Trash` + logo InkNote; thanh trên `←`, breadcrumb, `+ New Folder`, `+ New Note`, ô tìm kiếm có `X`, bánh răng `⚙` giấy mặc định; lưới ~5 cột chung thư mục & ghi chú kèm thumbnail trang đầu sắc nét); nút Home về Library và mở thẳng vào Library khi khởi động bình thường.

- **Bước 7.2 – Quản lý thư mục, Menu thẻ & Thùng rác (Trash – P3)** (`commit eae5dc4`, `c4a9313`, `9ac0a61`, tag `step-7.2-ok` — người dùng xác nhận test OK):
  - `crates/inknote-core/src/library.rs`: Thêm `rename_note`, `rename_folder`, `duplicate_note`, `toggle_favorite_note`, `move_to_trash` (kèm sidecar `.trashmeta.json` nhớ thư mục gốc), `restore_from_trash`, `delete_permanently`, `empty_trash`, sắp xếp ghi chú Yêu thích lên đầu.
  - `crates/inknote-app/src/library_view.rs`, `icons.rs`, `theme.rs`, `main.rs`: Thêm nút `⌄` cạnh tên thẻ + chuột phải mở menu tối bo góc co gọn theo nội dung (`Rename`, `Duplicate`, `Favorite` với icon bookmark đỏ giống GoodNotes, `Move to Trash`), mục `Trash (N)` + nút `Empty Trash` và hộp thoại xác nhận trước khi xoá vĩnh viễn.

## Đang làm dở
- Chuẩn bị thực hiện **Bước 7.3 – Xuất PDF/Ảnh & Chế độ chỉ đọc (P3)**. Đã gắn tag `step-7.2-ok` và đẩy lên GitHub `origin/main`.

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
  - Trạng thái lưu trên Navbar: `Tên file` `(trang/tổng)` + `chấm xanh/đỏ` + chữ `"Auto-saved"` màu trắng (3s) ngay bên phải nút Read-Only; tự lưu bản nháp tạm khi chưa chọn file và hỏi xác nhận lúc tắt app.
- **Chuyển từ `AutoNoVsync` + `request_repaint()` liên tục sang VSync (`PresentMode::Fifo`) + reactive repaint**: Không ép render tối đa liên tục lúc đứng yên gây nóng máy/quá tải; tự động hãm FPS đúng bằng tần số quét màn hình (144Hz → max 144 FPS, 60Hz → max 60 FPS).
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
1. **Bước 7 (7.1 → 7.4)**: Màn hình Library (P3), Trash, xuất PDF/Ảnh, `README.md`.

## Cập nhật lần cuối
- **Ngày**: 2026-11-13
- **Model đã dùng**: Gemini (Antigravity IDE)
