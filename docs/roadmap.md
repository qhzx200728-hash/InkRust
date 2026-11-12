# Roadmap InkNote

## Quy tắc chung
- Mỗi bước nhỏ = 1 commit git. Trước mỗi bước: `git status` phải sạch.
- **Rollback (hoàn tác):** chỉ dùng `git revert <mã commit>` (không dùng `git reset --hard`).
- **Đánh dấu hoàn thành:** mỗi bước xong phải có người dùng test thủ công và báo **"OK"**, sau đó gắn thẻ `git tag step-X.Y-ok` rồi mới sang bước tiếp theo.
- Thứ tự ưu tiên giao diện theo `docs/04-ui-spec.md`: **P0 (Khung giao diện) → P1 (Công cụ cốt lõi) → P2 (Đối tượng: LaTeX, Hình, Chữ, Ảnh) → P3 (Library)**.

---

## Bước 0 – Spike (Thử nghiệm kỹ thuật)

| Bước | Việc | Trạng thái | Test thủ công | Rollback |
|---|---|---|---|---|
| 0.1 | Thử pressure bút Wacom với `egui`, thử làm mượt `ink-stroke-modeler-rs`, thử `latex` → `dvisvgm` ra SVG tiếng Việt | ✅ Đã xong | Xem `docs/spike-result.md` (nguồn `TOUCH/BUT`, force tăng, nét dày/mỏng đúng; SVG tiếng Việt rõ nét) | `git revert fb30599` |

---

## Bước 1 – Nét bút Rnote & Khung giao diện P0

| Bước | Việc | Trạng thái | Test thủ công | Rollback |
|---|---|---|---|---|
| 1.1 | Cửa sổ trống + khung dự án workspace (`inknote-pen`, `inknote-render`, `inknote-app`) | ✅ Đã xong | Mở app thấy cửa sổ trắng, đóng được | `git revert a244fad` |
| 1.2 | Vẽ nét bằng bút Wacom, độ dày theo pressure, chặn vẽ nhầm khi bút lơ lửng (hover) | ✅ Đã xong | Rê bút lơ lửng không bị vẽ; chạm nhẹ → nét mỏng; ấn mạnh → nét dày | `git revert <commit>` |
| 1.3 | Làm mượt nét qua `PenPathModeledBuilder` (`ink-stroke-modeler-rs`, tham số nguyên bản Rnote) | ✅ Đã xong | Viết liền chữ `"huyen"` có nhiều nét cong gấp không nhấc bút: nét cong mượt, không bị nối thẳng | `git revert <commit>` |
| 1.4 | Port nguyên văn hình dạng nét Rnote (`SmoothOptions`, `PressureCurve`, `Solid`, `Marker`, `Textured`) + tô bằng `lyon` (`FillRule::NonZero`), sửa áp lực lúc nhấc bút | ✅ Đã xong | Viết chữ `"huyen"` so sánh với Rnote thật; cuối nét không còn bị phình giọt nước | `git revert <commit>` |
| 1.5 | **Giấy phép**: tạo file `LICENSE` (toàn văn GPL-3.0) và `NOTICE` ghi rõ nguồn Rnote, commit hash của `reference/rnote` (`83a76367dbd3d6c461c10553c45231799a25ea4b`) và nguồn bộ icon | ✅ Đã xong | Kiểm tra thư mục gốc có đủ `LICENSE` và `NOTICE` đúng thông tin | `git revert 4ec3161` |
| 1.6 | **Khung giao diện P0 (`04-ui-spec.md`)**: tạo `theme.rs` (gom màu, cỡ chữ, khoảng cách, bo góc), nạp font tiếng Việt đủ dấu (`segoeui.ttf`), vẽ Thanh công cụ chính (nền xanh dương, nút đang chọn có ô nền sáng bo góc) + Thanh tuỳ chọn Bút (nền tối bo tròn: kiểu bút `Solid`/`Marker`, độ dày, hàng chấm màu + Pop-up bảng màu) + Canvas chấm (`Dotted Paper`) | ✅ Đã xong | Kiểm tra chữ `"Tiếng Việt, ắ ằ ẳ ẵ ặ ơ ư"` hiển thị đủ dấu; chọn kiểu bút/độ dày/màu trên thanh tuỳ chọn và mở Pop-up bảng màu hoạt động đúng | `git revert 04980fc ed788b5` |
| 1.7 | **Undo/Redo cho nét bút** (qua hệ thống lệnh Command trong `inknote-core`, phím tắt `Ctrl+Z`, `Ctrl+Y` / `Ctrl+Shift+Z`) | ✅ Xong (`step-1.7-ok`) | Vẽ 3 nét, bấm `Ctrl+Z` 3 lần biến mất hết, bấm `Ctrl+Y` 3 lần hiện lại đúng thứ tự | `git revert <commit 1.7>` |
| 1.8 | **Zoom/kéo trang (Pan & Zoom)** + **Test hiệu năng 2000 nét**: zoom/kéo mượt, nét luôn sắc; có nút/lệnh tạo 2000 nét mẫu để kiểm tra tốc độ khung hình | ✅ Code xong, chờ người dùng test | 1) Zoom 300% nét vẫn sắc, không vỡ.<br>2) Bật 2000 nét trên trang, kéo trang và viết thêm nét mới **vẫn mượt, không giật** | `git revert <commit 1.8>` |

---

## Bước 2 – Trang, Paper Settings (P1) và Lưu File

| Bước | Việc | Trạng thái | Test thủ công | Rollback |
|---|---|---|---|---|
| 2.1 | **Thiết kế định dạng file & Lưu/mở file (`Ctrl+S`)**: file lưu có **số phiên bản (`format_version`)**, danh sách **kiểu đối tượng** (nét bút, hình học, văn bản, ảnh, LaTeX) và **thumbnail trang đầu** (phục vụ màn hình Library P3) | ⏳ Chưa làm | Vẽ vài nét, bấm Lưu, tắt app, mở lại file: nội dung còn nguyên; kiểm tra file có đủ `format_version` và dữ liệu thumbnail trang đầu | `git revert <commit 2.1>` |
| 2.2 | **Paper settings (P1)**: mở từ menu dấu ba chấm trên toolbar; chọn kiểu giấy (trắng trơn, chấm, kẻ ô, kẻ dòng), chỉnh cỡ ô, bật/tắt và chỉnh vị trí **margin lines 2 bên** | ⏳ Chưa làm | Đổi qua lại các kiểu giấy, chỉnh cỡ ô và lề 2 bên: trang giấy cập nhật ngay; lưu và mở lại vẫn giữ cài đặt | `git revert <commit 2.2>` |
| 2.3 | **Auto-save**: tự động lưu ra file tạm rồi đổi tên đè lên file chính sau vài giây kể từ khi có thay đổi | ⏳ Chưa làm | Vẽ thêm nét, đợi vài giây, tắt ngang app bằng Task Manager, mở lại: không mất dữ liệu | `git revert <commit 2.3>` |

---

## Bước 3 – Nhiều trang & Thanh Thumbnail Trang (P1)

| Bước | Việc | Trạng thái | Test thủ công | Rollback |
|---|---|---|---|---|
| 3.1 | **Nhiều trang**: nút **Thêm trang** trên toolbar chính, chuyển trang, xoá trang (có undo) | ⏳ Chưa làm | Tạo 5 trang, vẽ nội dung khác nhau lên từng trang, chuyển qua lại đúng nội dung | `git revert <commit 3.1>` |
| 3.2 | **Thanh thumbnail trang (P1)**: bật/tắt bằng nút **Danh sách trang** (icon hai trang) trên toolbar; hiện cột thumbnail các trang, bấm để nhảy trang, thêm/xoá trang | ⏳ Chưa làm | Tạo 10 trang, mở thanh thumbnail: hình thu nhỏ khớp với nội dung từng trang, bấm vào trang nào nhảy đúng tới trang đó | `git revert <commit 3.2>` |

---

## Bước 4 – Công thức LaTeX kiểu Xournal++ (P2 – Ưu tiên cao)

| Bước | Việc | Trạng thái | Test thủ công | Rollback |
|---|---|---|---|---|
| 4.1 | **Crate `inknote-latex`**: chọn đường chạy theo `docs/spike-result.md` (`latex -no-shell-escape -halt-on-error -interaction=nonstopmode` → DVI → `dvisvgm --no-fonts`, dự phòng `pdflatex` + `pdftocairo -svg`), **không bật `shell-escape`**, có **timeout** (quá giờ tự huỷ tiến trình), vẽ SVG sắc nét lên canvas | ⏳ Chưa làm | 1) Render công thức + chữ tiếng Việt hiện đúng, zoom 300% vẫn sắc nét.<br>2) Thử đoạn TeX lặp vô hạn: tự ngắt đúng timeout, app không treo | `git revert <commit 4.1>` |
| 4.2 | **Hộp thoại LaTeX & Live Preview**: mở từ nút LaTeX trên toolbar (phong cách Pop-up tối bo góc); gõ xong chờ ~0,5 giây tự render ở luồng nền; báo lỗi rõ khi gõ sai mà vẫn giữ hình xem trước cũ | ⏳ Chưa làm | Gõ công thức → thấy hình xem trước sau ~0,5 giây, lúc đang chạy vẫn gõ tiếp được không đơ; gõ sai cú pháp → hiện báo lỗi rõ ràng | `git revert <commit 4.2>` |
| 4.3 | **Chèn đối tượng LaTeX vào trang**: bấm OK trong hộp thoại để đặt công thức lên trang; lưu `source` làm dữ liệu chính; **1 bước undo** | ⏳ Chưa làm | Chèn công thức, bấm `Ctrl+Z` 1 lần công thức biến mất, bấm `Ctrl+Y` hiện lại | `git revert <commit 4.3>` |
| 4.4 | **Double-click mở lại & Sửa công thức**: double-click vào công thức trên trang mở lại hộp thoại với đúng `source` cũ; sửa xong **giữ nguyên vị trí và kích thước**; **1 lần sửa = 1 bước undo** | ⏳ Chưa làm | Double-click công thức đã chèn → hiện đúng code cũ; sửa nội dung rồi bấm OK → không nhảy vị trí; bấm `Ctrl+Z` đúng 1 lần quay về công thức trước khi sửa | `git revert <commit 4.4>` |
| 4.5 | **Lưu/mở file giữ `source` LaTeX**: lưu cả `source` và `svg_cache` vào file; khi mở lại vẫn sửa tiếp được | ⏳ Chưa làm | Lưu file có công thức LaTeX, tắt app, mở lại, double-click vào công thức vẫn hiện đúng code LaTeX cũ | `git revert <commit 4.5>` |

---

## Bước 5 – Công cụ: Tẩy, Lasso, Hình học, Văn bản (P1 & P2)

| Bước | Việc | Trạng thái | Test thủ công | Rollback |
|---|---|---|---|---|
| 5.1 | **Tẩy (Eraser – P1)**: thanh tuỳ chọn có 2 chế độ (**Tẩy thường** và **Tẩy nét**) + Pop-up cài đặt nâng cao; **chỉ xoá nét bút, không bao giờ xoá LaTeX/hình/ảnh** | ⏳ Chưa làm | Đặt 1 nét bút đè lên/cạnh 1 công thức LaTeX, dùng cả 2 chế độ tẩy chà qua cả hai: **chỉ nét bút biến mất, công thức LaTeX còn nguyên vẹn**; `Ctrl+Z` khôi phục lại nét bút | `git revert <commit 5.1>` |
| 5.2 | **Chọn vùng (Lasso – P1)**: thanh tuỳ chọn có 2 cách khoanh (**Tự do** và **Hình chữ nhật**) + Pop-up công tắc lọc loại đối tượng (nét bút, ảnh, hình vẽ, hộp văn bản, **công thức LaTeX**); kéo di chuyển, đổi kích thước, xoá (1 bước undo) | ⏳ Chưa làm | Thử khoanh chọn nét bút + LaTeX, kéo sang chỗ khác; thử tắt công tắc "công thức LaTeX" trong Pop-up rồi khoanh lại: chỉ nét bút được chọn | `git revert <commit 5.2>` |
| 5.3 | **Hình học cơ bản (P2)**: thanh tuỳ chọn chọn hình (đường nối, đường cong, chữ nhật, tròn, tam giác, thoi, chữ nhật bo góc, mũi tên), kiểu nét, độ dày, màu; vẽ và undo/redo | ⏳ Chưa làm | Vẽ các hình học, dùng tẩy quẹt qua hình học + nét bút: chỉ nét bút mất, hình học không bị tẩy xoá | `git revert <commit 5.3>` |
| 5.4 | **Văn bản (Text – P2)**: tạo/sửa hộp văn bản tiếng Việt đủ dấu; thanh tuỳ chọn chỉnh cỡ chữ, đậm, nghiêng, căn lề, màu chữ; undo/redo | ⏳ Chưa làm | Gõ `"Tiếng Việt, ắ ằ ẳ ẵ ặ ơ ư"`, đổi cỡ/màu chữ; dùng tẩy quẹt qua không bị xoá; `Ctrl+Z` hoàn tác đúng | `git revert <commit 5.4>` |

---

## Bước 6 – Chèn Ảnh (P2)

| Bước | Việc | Trạng thái | Test thủ công | Rollback |
|---|---|---|---|---|
| 6.1 | **Chèn ảnh từ file & Nút dán ảnh (Paste)**: thanh tuỳ chọn Chèn ảnh có nút chọn file và **nút riêng Dán ảnh từ clipboard**; kéo di chuyển/đổi cỡ ảnh (1 bước undo) | ⏳ Chưa làm | Copy một ảnh vào clipboard, bấm nút Dán ảnh → ảnh hiện lên trang; thử chèn thêm 1 ảnh từ file | `git revert <commit 6.1>` |
| 6.2 | **Giữ ảnh sắc nét theo mức zoom & Kiểm tra tổng hợp với Tẩy**: lưu byte ảnh gốc, vẽ lại theo mức zoom; kiểm tra Tẩy không xoá ảnh | ⏳ Chưa làm | 1) Chèn ảnh có chữ nhỏ, zoom 300% ảnh vẫn sắc nét, không mờ.<br>2) Đặt nét bút + LaTeX + Ảnh, dùng tẩy quẹt qua cả ba: chỉ nét bút mất | `git revert <commit 6.2>` |

---

## Bước 7 – Màn hình Thư viện (Library – P3) & Hoàn thiện

| Bước | Việc | Trạng thái | Test thủ công | Rollback |
|---|---|---|---|---|
| 7.1 | **Giao diện Library (P3)**: bấm nút **Home** từ Note Editor về Library; bố cục Sidebar trái (`Documents`, `Trash`) + Thanh trên (quay lại, tiêu đề, ô tìm kiếm theo tên có nút `X`) + Lưới (Grid View ~5 cột) hiển thị chung thư mục và ghi chú kèm thumbnail trang đầu | ⏳ Chưa làm | Tạo vài ghi chú và thư mục, bấm Home về Library: thấy lưới thẻ có đúng thumbnail trang đầu, gõ tìm kiếm theo tên lọc đúng thẻ, bấm `X` xoá tìm kiếm | `git revert <commit 7.1>` |
| 7.2 | **Quản lý thư mục, Menu thẻ & Thùng rác (Trash – P3)**: menu trên mỗi thẻ (Đổi tên, Nhân bản, Yêu thích, Chuyển vào Thùng rác); màn hình `Trash` có Khôi phục, Xoá vĩnh viễn, Dọn thùng rác | ⏳ Chưa làm | Xoá 1 ghi chú → vào `Trash` thấy ghi chú đó → bấm Khôi phục → quay lại `Documents` còn nguyên; thử Xoá vĩnh viễn | `git revert <commit 7.2>` |
| 7.3 | **Xuất PDF/Ảnh & Chế độ chỉ đọc (P3)**: nút Chia sẻ/Xuất trên toolbar xuất trang/ghi chú ra file PDF hoặc PNG; nút khoá vẽ (Chế độ chỉ đọc, nếu người dùng chọn làm) | ⏳ Chưa làm | Xuất ghi chú có nét bút + LaTeX + ảnh ra PDF/PNG, mở file xuất ra kiểm tra rõ nét | `git revert <commit 7.3>` |
| 7.4 | **Hoàn thiện & `README.md` cho GitHub**: dọn cảnh báo/lỗi, viết `README.md` tiếng Việt hướng dẫn cài đặt môi trường (Rust, TeX Live), cách chạy và giấy phép GPL-3.0 | ⏳ Chưa làm | Đọc kiểm tra `README.md`, `LICENSE`, `NOTICE` và `git status` sạch sẽ trước khi đẩy lên GitHub | `git revert <commit 7.4>` |
