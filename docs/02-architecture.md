# 02 – Kiến trúc InkNote (theo hướng B – Rust + egui)

> Cập nhật ngày 2026-11-12 sau khi chốt egui (Bước 0), port nét bút Rnote (`docs/03-port-rnote.md`) và nhận đặc tả giao diện (`docs/04-ui-spec.md`).

## 1. Nguyên tắc cốt lõi
- **Tách "lõi", "vẽ" và "giao diện".** Lõi dữ liệu không phụ thuộc vào cửa sổ hay egui.
- **Mọi thay đổi nội dung trang đều đi qua "lệnh" (Command)** để hỗ trợ Undo/Redo nhất quán (mỗi thao tác thêm/xoá/sửa là đúng 1 bước undo).
- **Dữ liệu gốc là nguồn chính, hình render chỉ là bản lưu tạm (cache).**
- **Toàn bộ thông số giao diện (màu sắc, cỡ chữ, khoảng cách, độ bo góc)** gom vào một file duy nhất (`crates/inknote-app/src/theme.rs`) để dễ chỉnh sửa về sau.

---

## 2. Phân chia trách nhiệm giữa các phần (crate)

| Crate | Trách nhiệm | Không được làm |
|---|---|---|
| `inknote-pen` | Code vẽ nét và hình học port nguyên văn từ `rnote-compose` + `BrushConfig` (`Solid`, `Marker`, `Textured`), làm mượt nét qua `ink-stroke-modeler-rs`, tạo đường cong `kurbo::BezPath` | Không chứa code cửa sổ `egui`, không phụ thuộc GTK/Cairo |
| `inknote-render` | Cài đặt `piet::RenderContext` (`CairoRenderContext`) bằng `lyon_tessellation` (quy tắc `FillRule::NonZero` giống `cairo::FillRule::Winding` của Rnote), biến đường cong thành các mẻ lưới tam giác (`DrawBatch`); hỗ trợ vẽ nền giấy (chấm, kẻ ô, kẻ dòng, lề 2 bên) | Không chứa nút bấm, thanh công cụ hay hộp thoại `egui` |
| `inknote-core` *(tạo ở Bước 1.7 / 2.1)* | Cấu trúc Tài liệu → Trang → Đối tượng (`BrushStroke`, `Shape`, `Text`, `Image`, `Latex`); lịch sử lệnh Undo/Redo; định dạng lưu/mở file (có `format_version` và `first_page_thumbnail`), auto-save; quản lý danh sách ghi chú và Thùng rác (Trash) cho màn hình Library | Không phụ thuộc `egui` |
| `inknote-latex` *(tạo ở Bước 4.1)* | Tạo mẫu `.tex`, chạy `latex` → DVI → `dvisvgm --no-fonts` (hoặc `pdflatex` + `pdftocairo -svg` dự phòng) ở luồng nền, có **timeout** và **cấm `--shell-escape`** (`-no-shell-escape`), trả về chuỗi SVG hoặc thông báo lỗi | Không làm đơ luồng giao diện chính |
| `inknote-app` | Cửa sổ ứng dụng `eframe`/`egui`:<br>• `theme.rs`: bảng màu, cỡ chữ, bo góc, khoảng cách, nạp font tiếng Việt (`segoeui.ttf` / `SystemFontProvider`).<br>• `toolbar.rs`: Thanh công cụ chính (nền xanh dương, nút đang chọn có ô nền sáng bo góc) + Thanh tuỳ chọn ngữ cảnh (nền tối bo tròn) + các bảng Pop-up tối bo góc.<br>• `canvas_view.rs`: Nhận sự kiện bút Wacom / chuột, biến đổi Zoom/Pan, gửi mẻ lưới tam giác từ `inknote-render` xuống `egui::Painter`.<br>• `latex_dialog.rs`: Hộp thoại nhập công thức LaTeX + live preview.<br>• `library_view.rs`: Màn hình Thư viện (Sidebar `Documents`/`Trash`, thanh tìm kiếm theo tên, lưới ~5 cột thẻ thư mục & ghi chú). | Không viết lại thuật toán nét bút của `inknote-pen` |

---

## 3. Cấu trúc dữ liệu và Định dạng file (Bước 2.1)

```
DocumentFile
 ├─ format_version: u32          ← Số phiên bản định dạng file (bắt đầu từ 1)
 ├─ metadata                     ← Tên ghi chú, thời gian tạo/cập nhật, trạng thái yêu thích/thùng rác
 ├─ first_page_thumbnail         ← Ảnh thu nhỏ trang đầu (PNG/WebP bytes) để Library P3 hiển thị tức thì
 ├─ paper_settings               ← Kiểu giấy (Trơn, Chấm, Kẻ ô, Kẻ dòng), cỡ ô, lề trái/phải (bật/tắt, vị trí)
 └─ pages: Vec<Page>
     └─ objects: Vec<PageObject> (mỗi đối tượng có id, vị trí, kích thước, lớp hiển thị)
         ├─ BrushStroke          ← Nét bút (PenPath + Style từ inknote-pen) — CHỈ loại này bị Eraser xoá
         ├─ Shape                ← Hình học cơ bản
         ├─ Text                 ← Hộp văn bản
         ├─ Image                ← Byte ảnh GỐC (PNG/JPEG) + vị trí/kích thước; vẽ lại theo mức zoom (không mờ)
         └─ Latex                ← source: String (dữ liệu chính), pos, size, svg_cache: Option<String>
```

- **Auto-save (Bước 2.3):** Ghi ra file tạm `.tmp` cùng thư mục rồi đổi tên đè lên file chính (atomic rename) để không hỏng file nếu mất điện hay tắt ngang.

---

## 4. Quy tắc cho Tẩy (Eraser – Bước 5.1)
- Bộ lọc đối tượng của Tẩy **chỉ kiểm tra `PageObject::BrushStroke`** (hỗ trợ 2 chế độ: Tẩy thường và Tẩy cả nét).
- Các đối tượng `Latex`, `Image`, `Shape`, `Text` bị bỏ qua ngay ở đầu hàm kiểm tra va chạm của Tẩy — **không bao giờ bị tẩy xoá**. Muốn xoá chúng thì khoanh bằng Lasso rồi bấm Xoá.

---

## 5. Quy trình LaTeX kiểu Xournal++ (Bước 4)

### 5.1. Đối tượng
`LatexObject { id, source: String, pos: [f64; 2], size: [f64; 2], svg_cache: Option<String> }`
- `source` là dữ liệu chính; `svg_cache` chỉ là bản lưu tạm để mở file nhanh.

### 5.2. Chèn mới & Live Preview
1. Bấm nút **Công thức LaTeX** trên thanh công cụ chính → mở hộp thoại LaTeX (phong cách Pop-up tối bo góc).
2. Người dùng gõ code → đợi ~0,5 giây sau khi ngừng gõ (debounce) → gửi sang luồng nền (`inknote-latex`).
3. Luồng nền chạy lệnh trong thư mục tạm riêng:
   - Đường chính (đã kiểm chứng trong `docs/spike-result.md`): `latex -no-shell-escape -halt-on-error -interaction=nonstopmode` → tạo `.dvi` → `dvisvgm --no-fonts` → tạo `.svg`.
   - Đường dự phòng: `pdflatex -no-shell-escape -halt-on-error -interaction=nonstopmode` → `pdftocairo -svg`.
   - **An toàn & chống treo:** Tuyệt đối **không bật `--shell-escape`** (truyền rõ `-no-shell-escape`), và áp dụng **timeout** (ví dụ 10 giây; nếu vòng lặp vô hạn trong TeX thì tự động dừng tiến trình con và báo quá thời gian).
4. Khi thành công: hiển thị hình xem trước ngay trong hộp thoại. Khi có lỗi cú pháp: hiện thông báo lỗi trích từ `.log`, vẫn giữ hình xem trước hợp lệ gần nhất.
5. Bấm OK → chèn `LatexObject` vào trang (**1 bước undo**).

### 5.3. Sửa công thức (Double-click)
1. Double-click vào công thức trên trang → mở lại hộp thoại với đúng chuỗi `source` cũ.
2. Sửa xong bấm OK → cập nhật `source` và `svg_cache`, **giữ nguyên vị trí và kích thước**; ghi vào lịch sử lệnh thành **đúng 1 bước undo** (Ctrl+Z 1 lần quay về `source` và hình cũ).
3. Bấm Huỷ → không thay đổi gì, không sinh bước undo.

---

## 6. Kết quả kiểm chứng khả năng giao diện của `egui 0.37` (Mục 9.4 của `04-ui-spec.md`)

Đã kiểm tra bằng bài test biên dịch và chạy thật trong `spike/egui-pressure` (`test_ui_spec_capabilities`):
1. **Thanh công cụ nền màu + ô chọn bo góc:** **Làm được** bằng `egui::Panel::top(...).frame(egui::Frame::new().fill(...))` kết hợp `egui::Frame::new().fill(...).corner_radius(...)`.
2. **Thanh tuỳ chọn bo tròn + Pop-up tối bo góc:** **Làm được** bằng `egui::Popup::from_toggle_button_response(&btn).frame(egui::Frame::popup(ui.style()).fill(...).corner_radius(...))`.
3. **Lưới thumbnail (Grid View):** **Làm được** bằng `egui::Grid` hoặc `ui.horizontal_wrapped` với các thẻ `egui::Frame` bo góc.
4. **Tiếng Việt đầy đủ dấu (`"Tiếng Việt, ắ ằ ẳ ẵ ặ ơ ư đ Đ"`):**
   - **Phát hiện quan trọng:** Font mặc định đi kèm của `egui` (`Ubuntu-Light.ttf`) **thiếu** các ký tự tiếng Việt có dấu phụ kép (`'ắ'`, `'ằ'`, `'ẳ'`, `'ẵ'`, `'ặ'` đều trả về `has_glyph == false`).
   - **Cách giải quyết đã kiểm chứng:** Nạp font hệ thống Windows `C:\Windows\Fonts\segoeui.ttf` vào `egui::FontDefinitions` (hoặc bật `egui_system_font_provider::SystemFontProvider`) thì `has_glyph` đạt **true 100%** cho toàn bộ chữ tiếng Việt.
5. **Hiển thị SVG công thức LaTeX trên canvas `egui`:** Cần crate chuyển SVG thành ảnh/texture theo độ phân giải mức zoom hiện tại (dự kiến dùng `resvg` / `usvg` — **chưa kiểm chứng build trong workspace chính**, sẽ kiểm tra bằng `cargo add` và build thử ở Bước 4.1).
