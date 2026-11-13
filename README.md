# InkNote

**InkNote** là ứng dụng ghi chú viết tay trên **Windows**, viết hoàn toàn bằng ngôn ngữ **Rust** (`eframe` / `egui 0.37.0`), tối ưu cho bảng vẽ **Wacom có lực nhấn (pressure)**.

Dự án kết hợp lõi nét bút mượt mà nguyên bản từ **Rnote**, quy trình chèn/sửa công thức **LaTeX** có xem trước trực tiếp lấy cảm hứng từ **Xournal++**, cùng giao diện trình soạn thảo và màn hình Thư viện (**Library**) trực quan.

---

## Tính năng chính

- **Bút viết nhạy lực nhấn (`Solid`, `Marker`)**:
  - Hỗ trợ lực nhấn bút Wacom, tự động làm mượt đường nét (`ink-stroke-modeler-rs`) và tô hình học sắc nét (`lyon_tessellation`).
  - Chặn lỗi tự vẽ khi rê bút lơ lửng (hover) và lỗi phình nét khi nhấc bút.
  - Nhiều mức độ dày nét bút, hàng chọn màu nhanh và bảng chọn màu mở rộng.
- **Tẩy an toàn (`Standard` & `Stroke`)**:
  - Có 2 chế độ: **Tẩy thường** (xoá từng đoạn nét) và **Tẩy nét** (xoá cả nét chạm vào).
  - **Quy tắc riêng của InkNote**: mặc định bộ lọc chỉ tẩy **nét bút**, **không bao giờ xoá nhầm** công thức LaTeX, hình học, hộp văn bản hay hình ảnh khi quẹt qua (muốn xoá các đối tượng này thì dùng công cụ Lasso để chọn rồi xoá).
- **Chọn vùng (`Lasso`)**:
  - Hỗ trợ 2 kiểu khoanh vùng: **Tự do (Freehand)** và **Hình chữ nhật (Rectangle)**.
  - Bảng công tắc lọc bật/tắt từng loại đối tượng được chọn (nét bút, hình học, hộp văn bản, hình ảnh, công thức LaTeX).
  - Kéo di chuyển, kéo 4 góc để thay đổi kích thước đồng tỷ lệ, và xoá gọn với 1 bước hoàn tác (`Undo`).
- **Công thức LaTeX kiểu Xournal++**:
  - Hộp thoại gõ LaTeX có **Live Preview** chạy ở luồng nền sau ~0,5 giây ngừng gõ; báo lỗi cú pháp rõ ràng nhưng vẫn giữ hình xem trước cũ.
  - Biên dịch an toàn qua `latex -no-shell-escape` → `.dvi` → `dvisvgm --no-fonts` (dự phòng `pdflatex` + `pdftocairo -svg`), luôn có giới hạn thời gian (**timeout**) chống treo app khi gặp vòng lặp vô hạn.
  - Lưu mã nguồn LaTeX (`source`) làm dữ liệu chính kèm bộ nhớ đệm SVG (`svg_cache`): nhấp đúp (**double-click**) vào công thức để mở lại mã nguồn cũ, sửa xong giữ nguyên vị trí; 1 lần sửa = 1 bước `Undo`. Tự động render lại sắc nét khi phóng to.
- **18 loại Hình học & Hộp văn bản tiếng Việt**:
  - **Hình học**: đường thẳng, mũi tên 1–2 chiều, đường cong, chữ nhật, hình tròn/oval, 3 loại tam giác, đa giác 5–8 cạnh, hình thoi, hình bình hành, hình thang... Giữ phím `Ctrl` khi vẽ để căn góc đặc biệt (`0°/30°/45°/60°/90°`) và vẽ hình đều.
  - **Hộp văn bản (Text)**: nạp font hệ thống Windows (`segoeui.ttf`) hiển thị đầy đủ 100% dấu tiếng Việt; tuỳ chỉnh font chữ, cỡ chữ, in đậm, in nghiêng, căn lề, danh sách gạch đầu dòng/đánh số và màu chữ.
- **Chèn & Dán ảnh (`Ctrl+V`)**:
  - Chọn ảnh từ máy (`PNG`, `JPEG`, `BMP`, `WebP`) hoặc bấm nút **Paste Image** / nhấn `Ctrl+V` để dán ảnh trực tiếp từ clipboard.
  - Lưu nguyên byte ảnh gốc trong file `.inknote` và tự động nâng cấp độ nét khi zoom (hỗ trợ giới hạn texture GPU lên tới `8192px` kèm mipmap `Lanczos3`).
- **Trang giấy, Nhiều trang & Tự động lưu (Auto-save)**:
  - Hỗ trợ 2 chế độ khung vẽ: **Paper** (khổ A4/A5 dọc hoặc ngang, mẫu giấy trắng trơn/chấm/kẻ ô/kẻ dòng, tuỳ chỉnh khoảng cách và vạch lề 2 bên) và **Whiteboard** (bảng trắng trải rộng không viền, không đường lề).
  - Cuộn dọc liền mạch nhiều trang, kéo mạnh quá đáy trang cuối để tự thêm trang mới, thanh **Pages** hiển thị thumbnail 2 cột kèm menu thao tác nhanh từng trang (`Add Page Before/After`, `Duplicate`, `Clear Page`, `Delete Page`).
  - Tự động lưu ngầm sau **2,5 giây** bằng cơ chế ghi an toàn (**atomic save**: ghi vào file tạm `.tmp` rồi đổi tên đè lên file chính), kèm bản nháp khôi phục khi tắt app đột ngột.
- **Màn hình Thư viện (Library) & Thùng rác (Trash)**:
  - Bố cục gồm thanh bên (`Documents`, `Trash`) và lưới **5 cột** hiển thị chung thư mục và ghi chú `.inknote` kèm ảnh thu nhỏ trang đầu sắc nét.
  - Hỗ trợ tìm kiếm đệ quy theo tên tiếng Việt, tạo thư mục/ghi chú mới, đánh dấu **Yêu thích (Favorite)**, nhân bản, đổi tên, chuyển vào **Trash**, khôi phục hoặc xoá vĩnh viễn.
- **Xuất file & Chế độ chỉ đọc (Read-Only Mode)**:
  - Xuất ghi chú ra **PDF 1.4 nhiều trang dạng vector** cực nét (giữ đường cong nét bút, SVG LaTeX và chữ tiếng Việt) hoặc xuất từng trang ra **PNG độ phân giải cao** (`3x` / `4x`).
  - Chế độ **Read-Only** khóa mọi thao tác sửa, ẩn thanh công cụ nổi ở giữa dưới để đọc tài liệu thoáng mắt mà vẫn cuộn/phóng to mượt mà.

---

## Cài đặt môi trường trên Windows

Để biên dịch và chạy **InkNote** từ mã nguồn trên Windows, bạn cần chuẩn bị 3 thành phần sau (hoàn toàn miễn phí):

1. **Visual Studio Build Tools (C++ MSVC)**:
   - Tải *Build Tools for Visual Studio* tại trang chủ Microsoft, chọn cài workload **"Desktop development with C++"**.
2. **Rust (`1.99.1` trở lên, edition `2024`)**:
   - Tải và cài đặt qua [rustup.rs](https://rustup.rs/).
   - Sau khi cài xong, bộ công cụ Cargo sẽ nằm tại `%USERPROFILE%\.cargo\bin`.
3. **TeX Live (tuỳ chọn — bắt buộc nếu muốn gõ và biên dịch công thức LaTeX)**:
   - Cài **TeX Live** (ví dụ bản TeX Live 2026) và đảm bảo các lệnh `latex`, `dvisvgm` (hoặc dự phòng `pdflatex`, `pdftocairo`) đã có trong biến môi trường `PATH` của Windows.

---

## Cách chạy, kiểm thử và đóng gói (Windows PowerShell)

Mở **PowerShell** tại thư mục gốc của dự án và thêm đường dẫn Cargo vào phiên làm việc hiện tại:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
```

Các lệnh thường dùng:

```powershell
# Kiểm tra biên dịch nhanh toàn bộ ứng dụng
cargo check -p inknote-app

# Chạy bộ kiểm thử tự động
cargo test -p inknote-app

# Chạy trực tiếp ứng dụng InkNote (chế độ phát triển đã bật opt-level = 2 cho mượt)
cargo run -p inknote-app

# Đóng gói file chạy độc lập (.exe) để dùng hàng ngày
cargo build --release -p inknote-app
```

Sau khi chạy `cargo build --release -p inknote-app`, file thực thi sẽ được tạo tại:

```text
target\release\inknote-app.exe
```

*(Mẹo: nếu máy đang không kết nối Internet nhưng đã tải sẵn các thư viện trong `Cargo.lock`, bạn có thể thêm cờ `--offline` vào các lệnh `cargo` ở trên).*

---

## Phím tắt thông dụng

| Phím tắt / Thao tác | Chức năng |
|---|---|
| `Ctrl + N` | Tạo ghi chú mới |
| `Ctrl + O` | Mở file `.inknote` |
| `Ctrl + S` | Lưu file hiện tại |
| `Ctrl + Shift + S` | Lưu thành file mới (`Save As`) |
| `Ctrl + Z` | Hoàn tác (`Undo`) |
| `Ctrl + Y` hoặc `Ctrl + Shift + Z` | Làm lại (`Redo`) |
| `Ctrl + V` | Dán ảnh trực tiếp từ clipboard |
| `Ctrl + 0` | Đặt lại mức zoom 100% và căn giữa trang |
| `Ctrl + +` / `Ctrl + -` (hoặc cuộn chuột + `Ctrl`) | Phóng to / thu nhỏ trang |
| Giữ `Space` + kéo chuột trái (hoặc kéo chuột giữa) | Di chuyển / kéo trang (`Pan`) |
| `PageUp` / `PageDown` | Chuyển lên trang trước / xuống trang sau |
| `Delete` / `Backspace` | Xoá các đối tượng đang được chọn bằng Lasso |
| Giữ `Ctrl` khi vẽ hình học | Căn góc đặc biệt (`0°/30°/45°/60°/90°`) & vẽ hình đều |

---

## Cấu trúc thư mục

- `crates/inknote-pen/`: Lõi xử lý nét bút, đường cong và làm mượt được port nguyên bản từ `rnote-compose` của Rnote.
- `crates/inknote-render/`: Bộ dựng hình học bằng `lyon_tessellation` thay thế cho Cairo, hỗ trợ vẽ `egui` và ghi lại đường cong vector cho xuất PDF.
- `crates/inknote-core/`: Mô hình dữ liệu tài liệu nhiều trang (`.inknote`), hệ thống lệnh `Command` cho `Undo`/`Redo`, định dạng file và quản lý Thư viện (`LibraryStore`).
- `crates/inknote-latex/`: Bộ biên dịch LaTeX an toàn (cấm `--shell-escape`, có timeout) và rasterize SVG theo độ phân giải.
- `crates/inknote-app/`: Ứng dụng giao diện `eframe` / `egui`, thanh công cụ, canvas, các hộp thoại, auto-save và xuất PDF/PNG.
- `docs/`: Các tài liệu phân tích kiến trúc, đặc tả giao diện và lộ trình phát triển (`00-rnote-analysis.md` → `04-ui-spec.md`, `roadmap.md`, `spike-result.md`).
- `LICENSE` và `NOTICE`: Toàn văn giấy phép GPL-3.0 và ghi công chi tiết các dự án mã nguồn mở tham chiếu.

---

## Giấy phép & Ghi công (License & Attribution)

**InkNote** được phát hành theo giấy phép **GNU General Public License v3.0 or later (`GPL-3.0-or-later`)**. Xem toàn văn tại tệp [LICENSE](LICENSE) và chi tiết bản quyền bên thứ ba tại [NOTICE](NOTICE).

- **Rnote** ([github.com/flxzt/rnote](https://github.com/flxzt/rnote)): Mã nguồn xử lý nét bút, đường cong và cấu hình bút trong `crates/inknote-pen/` được port nguyên bản từ Rnote của tác giả Felix Zwettler (`flxzt`) và các cộng tác viên (commit tham chiếu: `83a76367dbd3d6c461c10553c45231799a25ea4b`, giấy phép `GPL-3.0-or-later`).
- **Xournal++** ([github.com/xournalpp/xournalpp](https://github.com/xournalpp/xournalpp)): Tham khảo ý tưởng quy trình chèn và chỉnh sửa công thức LaTeX (lưu `source` làm dữ liệu chính, live preview, nhấp đúp mở lại mã cũ, gộp sửa thành 1 bước hoàn tác). InkNote **chỉ đọc để hiểu quy trình** và tự viết lại mới hoàn toàn bằng Rust, không sao chép mã nguồn GPLv2 của Xournal++.
- **Lucide Icons** (`ISC License`) và **Phosphor Icons** (`MIT License`): Nguồn tham khảo cho các biểu tượng giao diện vector.
