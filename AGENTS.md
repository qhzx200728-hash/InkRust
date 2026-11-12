# AGENTS.md – Cẩm nang dự án InkNote

## 1. Giới thiệu dự án
- **InkNote** là ứng dụng ghi chú viết tay trên Windows (tham khảo bố cục GoodNotes), kết hợp nét bút mượt của Rnote và quy trình chèn công thức LaTeX của Xournal++.
- Dành cho chủ dự án (`qHzz`) dùng cá nhân trên máy Windows với bảng vẽ Wacom có lực nhấn (pressure); chủ dự án không biết code, ngân sách 0 đồng, mã nguồn mở theo giấy phép **GPL-3.0-or-later**.

## 2. Công nghệ đang dùng
- **Ngôn ngữ & Công cụ**: Rust `1.99.1` (cài tại `%USERPROFILE%\.cargo\bin`, edition 2024), MSVC Build Tools, Git.
- **Giao diện & Cửa sổ**: `eframe` / `egui 0.37.0` (chạy thuần Rust trên Windows, nhận sự kiện bút Wacom).
- **Nét bút & Hình học**:
  - `inknote-pen`: port nguyên văn từ `rnote-compose` + `BrushConfig` của Rnote, làm mượt bằng `ink-stroke-modeler-rs 0.1.0`, đường cong `kurbo 0.13.1`, hình học `parry2d-f64 0.31.1`, `rough_piet 0.15.0`.
  - `inknote-render`: cài đặt `piet::RenderContext` (`CairoRenderContext`) bằng `lyon_tessellation 1.0.23` (`FillRule::NonZero`).
- **LaTeX**: TeX Live 2026 có sẵn trên máy (`latex -no-shell-escape` → DVI → `dvisvgm --no-fonts` tạo SVG vector; dự phòng `pdflatex` + `pdftocairo -svg`).

## 3. Cấu trúc thư mục
- `Cargo.toml`: Cấu hình workspace Rust.
- `LICENSE`, `NOTICE`: Toàn văn GPL-3.0 và ghi công nguồn Rnote (kèm commit hash), ý tưởng Xournal++, bộ icon.
- `AGENTS.md`, `memory.md`, `GEMINI.md`: Bộ nhớ dự án và quy tắc cho AI.
- `docs/`: Tài liệu bắt buộc đọc trước khi làm:
  - `00-rnote-analysis.md`: Phân tích Rnote & Xournal++.
  - `01-tech-options.md`: Lý do chốt hướng B (`egui`).
  - `02-architecture.md`: Kiến trúc tách crate, cấu trúc file lưu, quy trình LaTeX.
  - `03-port-rnote.md`: Chi tiết port code Rnote sang `inknote-pen` và `inknote-render`.
  - `04-ui-spec.md`: Đặc tả giao diện chi tiết (Note Editor, Library, các công cụ, P0→P3).
  - `roadmap.md`: Lộ trình từng bước nhỏ, cách test thủ công và rollback.
  - `spike-result.md`: Kết quả thử nghiệm kỹ thuật Bước 0.
- `crates/inknote-pen/`: Lõi nét bút Rnote (44 file từ `rnote-compose` + `brushconfig.rs`).
- `crates/inknote-render/`: Bộ chia lưới tam giác `lyon` thay thế Cairo.
- `crates/inknote-app/`: Ứng dụng `egui` (`main.rs`, `theme.rs`, `toolbar.rs`, `icons.rs`).
- *(Dự kiến theo roadmap)*: `crates/inknote-core/` (Bước 1.7/2.1) và `crates/inknote-latex/` (Bước 4.1).
- `reference/`: Mã nguồn tham khảo `rnote` và `xournalpp` (**chỉ đọc**).
- `spike/`: Thí nghiệm Bước 0 (đã xong, bỏ đi, không phải mã sản phẩm).

## 4. Cách chạy, build, test (Windows PowerShell)
Thêm đường dẫn Cargo trước khi chạy lệnh trong phiên PowerShell mới:
```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo check -p inknote-app   # Kiểm tra biên dịch nhanh
cargo test -p inknote-app    # Chạy bộ kiểm thử tự động
cargo fmt -p inknote-app     # Định dạng code
cargo run -p inknote-app     # Chạy ứng dụng InkNote
```

## 5. Quy ước code và giao diện (khớp `docs/04-ui-spec.md` & `docs/02-architecture.md`)
- **Kiến trúc**: Tách biệt lõi dữ liệu (`inknote-pen`, `inknote-core`, `inknote-latex`), bộ vẽ (`inknote-render`) và giao diện (`inknote-app`). Mọi thay đổi nội dung trang phải đi qua hệ thống lệnh (Command) để Undo/Redo.
- **Thứ tự ưu tiên**: **P0 (Khung giao diện) → P1 (Công cụ cốt lõi) → P2 (Đối tượng: LaTeX, Text, Hình học, Ảnh) → P3 (Library)**.
- **Giao diện & Theme**:
  - Toàn bộ màu sắc, cỡ chữ, khoảng cách, bo góc gom vào `crates/inknote-app/src/theme.rs`.
  - Ngôn ngữ giao diện mặc định: **Tiếng Anh**. Nạp font `C:\Windows\Fonts\segoeui.ttf` để hiển thị đầy đủ 100% dấu tiếng Việt (`"Tiếng Việt, ắ ằ ẳ ẵ ặ ơ ư"`).
  - **2 màn hình chính**: **Note Editor** (Thanh công cụ chính nền xanh dương chia 3 nhóm Trái–Giữa–Phải; Thanh tuỳ chọn ngữ cảnh nền tối bo tròn nằm chính giữa kèm Pop-up tối bo góc; Canvas giấy chấm/tuỳ chỉnh có margin lines 2 bên) và **Library** (Sidebar `Documents`/`Trash` + lưới ~5 cột chung thư mục & ghi chú, tìm kiếm theo tên).
- **Quy tắc các công cụ trên Thanh tuỳ chọn & Pop-up (`04-ui-spec.md` mục 2.2 & 5)**:
  - **Bút (Pen)**: Kiểu bút đã port từ Rnote (`Solid`, `Marker`; kiểu khác để sau), nhiều mức độ dày vẽ bằng đường kẻ dày mỏng, hàng chấm màu + Pop-up bảng màu mở rộng/chọn màu tuỳ ý.
  - **Tẩy (Eraser)**: Trên thanh có **2 chế độ** (**Tẩy thường** và **Tẩy nét**). Trong **Pop-up Cài đặt nâng cao** có: bộ lọc loại nội dung bị tẩy, tuỳ chọn "Tự động bỏ chọn", nút "Xoá nội dung trang". **Quy tắc riêng của InkNote**: bộ lọc mặc định chỉ gồm **nét bút**; **ảnh, hình, công thức LaTeX không bao giờ bị tẩy lướt qua** (muốn xoá chúng thì chọn bằng Lasso rồi xoá).
  - **Chọn vùng (Lasso)**: 2 cách khoanh (**Tự do** và **Hình chữ nhật**) + Pop-up danh sách loại đối tượng được chọn có **công tắc bật/tắt** (chữ viết tay, ảnh, hình vẽ, mũi tên, hộp văn bản, **công thức LaTeX**).
  - **Hình học**: Đường nối, đường cong, chữ nhật, tròn, tam giác, thoi, chữ nhật bo góc; kiểu nét, độ dày, bảng màu.
  - **Văn bản (Text)**: Kiểu chữ, cỡ chữ, đậm, nghiêng, căn lề, danh sách, bảng màu chữ.
  - **Chèn ảnh (Image)**: Chọn từ file + **nút riêng dán ảnh từ clipboard**; lưu bản gốc và vẽ lại theo mức zoom (không mờ).
  - **LaTeX**: Hộp thoại phong cách Pop-up tối bo góc; lưu `source` làm dữ liệu chính; **live preview** chạy nền (~0,5s debounce, có timeout, cấm `shell-escape`), báo lỗi rõ khi sai; **double-click** mở lại code cũ, sửa xong giữ nguyên vị trí/kích thước; **1 lần sửa = 1 bước undo**.

## 6. Những điều KHÔNG được làm
- Không sửa, không build thư mục `reference/` (chỉ đọc). Không sao chép code từ Xournal++ (GPLv2); chỉ đọc hiểu quy trình rồi tự viết bằng Rust.
- Không sửa thuật toán gốc trong `crates/inknote-pen` đã copy từ Rnote.
- Không sao chép tên, logo, icon hay tài sản bản quyền của GoodNotes.
- Không bật `--shell-escape` khi gọi LaTeX.
- Không commit thư mục `target/`. Không dùng `git reset --hard` (chỉ dùng `git revert` để rollback). Không tự gắn tag `step-X.Y-ok` khi người dùng chưa xác nhận test OK.

## 7. Quy trình bắt buộc
Bắt đầu mỗi phiên làm việc: đọc `AGENTS.md` và `memory.md`, rồi tóm tắt lại cho người dùng bằng 3 dòng. Kết thúc phiên (hoặc khi người dùng nói 'chốt'): cập nhật `memory.md`.

