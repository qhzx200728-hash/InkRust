# 04 – Đặc tả giao diện InkNote (tham khảo phong cách GoodNotes)

> File này là **tài liệu tham khảo giao diện**, KHÔNG phải lệnh "làm hết ngay".
> Người dùng (qHzz) không biết code, làm bằng AI, ngân sách 0 đồng, chỉ chạy trên **Windows + bút Wacom**.

---

## 0. Quy tắc cho AI Agent khi đọc file này

1. **Chưa viết code** từ file này. Việc đầu tiên: cập nhật `docs/roadmap.md` (mục 9), cho người dùng xem và chờ duyệt.
2. Làm theo thứ tự ưu tiên P0 → P1 → P2 → P3 (mục 8). Mỗi lần chỉ 1 bước nhỏ, có cách test thủ công và cách rollback như `roadmap.md`.
3. **Không sao chép tài sản của GoodNotes**: không dùng tên, logo, icon, ảnh chụp, bảng màu nguyên bản. Chỉ lấy **ý tưởng bố cục và cách tổ chức chức năng**. Icon phải tự vẽ hoặc lấy từ bộ icon miễn phí có giấy phép tương thích GPL-3.0 (kiểm tra giấy phép trước khi dùng, ghi nguồn vào `NOTICE`).
4. Công nghệ giữ nguyên: Rust + egui, Windows. Nét bút dùng code đã port từ Rnote (`crates/inknote-pen`). Không đề xuất đổi công nghệ chỉ vì giao diện.
5. Toàn bộ màu, cỡ chữ, khoảng cách, bo góc gom vào **một chỗ duy nhất** (ví dụ `theme.rs` trong `inknote-app`) để chỉnh sau dễ. Giá trị trong file này chỉ là gợi ý ban đầu.
6. Cái gì chưa chắc thì ghi "chưa chắc" và nói cách kiểm tra. Không bịa API của egui hay crate nào, hãy build thử để xác nhận.
7. Cuối mỗi bước: liệt kê tối đa 3 câu hỏi cần người dùng quyết, rồi DỪNG chờ trả lời.

---

## 1. Bức tranh tổng thể

InkNote có **2 màn hình chính**:

| Màn hình | Vai trò |
|---|---|
| **Trình soạn ghi chú (Note Editor)** | Viết, vẽ, chèn nội dung trên trang |
| **Thư viện (Library)** | Quản lý ghi chú, thư mục, thùng rác, tìm kiếm |

Nút Home ở góc trái thanh công cụ của Note Editor đưa người dùng về Library. Mở một ghi chú trong Library thì vào Note Editor.

---

## 2. Note Editor – cấu trúc 3 khu vực

| Khu vực | Vị trí | Vai trò |
|---|---|---|
| Thanh công cụ chính (Navigation Toolbar) | Trên cùng | Công cụ và điều hướng chính |
| Thanh tuỳ chọn công cụ (Tool Options Bar) | Ngay dưới toolbar | Tuỳ chọn của công cụ đang chọn (đổi theo ngữ cảnh) |
| Canvas chính (Main Canvas) | Trung tâm | Nơi viết, vẽ, chèn nội dung |

### 2.1. Thanh công cụ chính

Dải ngang cố định, **chiều cao thấp**, nền xanh dương, icon trắng, cùng 1 trục, kích thước đều. Công cụ đang chọn có **ô nền sáng bo góc** phía sau icon. Các nhóm cách nhau bằng khoảng trống nhỏ hoặc vạch dọc.

Từ trái sang phải:

| Nhóm | Thành phần | Ghi chú cho InkNote |
|---|---|---|
| Trái | **Home** | Về Library |
| Trái | **Danh sách trang** (icon hai trang) | Bật/tắt thanh thumbnail các trang (liên quan roadmap 3.2) |
| Trái | Chế độ chỉ đọc (icon mắt + sách) | **Chưa chắc chức năng.** Đề xuất: khoá vẽ để không chạm nhầm. Để P3, hỏi người dùng |
| Giữa | **Chọn vùng (Lasso)** | |
| Giữa | **Bút** (mặc định đang chọn) | |
| Giữa | **Tẩy** | |
| Giữa | **Văn bản (T)** | |
| Giữa | **Chèn ảnh** | |
| Giữa | **Hình học** | |
| Giữa | **Công thức LaTeX** (MỚI, GoodNotes không có) | Đặt ngay cạnh Chèn ảnh/Hình học. Đây là tính năng quan trọng nhất của InkNote |
| Giữa | Ghi chú dính (Sticky Note) | **Bỏ**, ngoài phạm vi |
| Phải | **Thêm trang** | |
| Phải | Chia sẻ/Xuất | **P3**: chỉ xuất ra PDF hoặc ảnh. Không làm chia sẻ đám mây |
| Phải | **Dấu ba chấm** | Menu chứa chức năng phụ: Paper settings (loại giấy, kẻ ô, lề), đổi tên ghi chú, cài đặt |

### 2.2. Thanh tuỳ chọn công cụ (Contextual Toolbar)

Thanh ngang **bo tròn, nền tối**, nằm ngay dưới toolbar chính. **Nội dung đổi theo công cụ đang chọn.** Thao tác dùng thường xuyên hiện thẳng trên thanh; thiết lập chi tiết nằm trong **bảng Pop-up** (nền tối, bo góc, có tiêu đề, chia nhóm, dùng thẻ chọn / icon / công tắc / bảng màu).

| Công cụ | Nội dung trên thanh | Pop-up chi tiết |
|---|---|---|
| **Bút** | Kiểu bút (theo kiểu đã port từ Rnote: Solid, Marker; kiểu khác để sau, xem `03-port-rnote.md`), độ dày (nhiều mức, vẽ bằng đường kẻ dày mỏng), hàng chấm màu | Bảng màu mở rộng, chọn màu tuỳ ý |
| **Tẩy** | 2 chế độ: **Tẩy thường** và **Tẩy nét** (xoá cả nét) | **Cài đặt nâng cao**: bộ lọc loại nội dung bị tẩy, tuỳ chọn "Tự động bỏ chọn", nút "Xoá nội dung trang" |
| **Chọn vùng** | 2 cách khoanh: **Tự do** và **Hình chữ nhật** | Danh sách loại đối tượng được chọn, mỗi loại có công tắc bật/tắt: chữ viết tay, ảnh, hình vẽ, mũi tên, hộp văn bản, **công thức LaTeX** (thêm) |
| **Hình học** | Các hình: đường nối, đường cong, chữ nhật, tròn, tam giác, thoi, chữ nhật bo góc; kiểu nét; bảng màu | Kiểu nét, độ dày, màu |
| **Văn bản** | Kiểu chữ, cỡ chữ, in đậm, in nghiêng, căn lề, danh sách; nút mở bảng màu chữ | Bảng màu chữ |
| **Chèn ảnh** | Chọn từ file, **dán ảnh từ clipboard** (nút riêng, theo yêu cầu của người dùng) | |
| **LaTeX** | Mở hộp thoại nhập công thức (xem mục 5) | |

**Quy tắc riêng của InkNote cho Tẩy** (từ yêu cầu gốc): bộ lọc mặc định chỉ gồm **nét bút** (viết, bút chì, dạ quang). **Ảnh, hình, công thức LaTeX không bao giờ bị tẩy lướt qua.** Người dùng muốn xoá chúng thì chọn bằng Lasso rồi xoá.

### 2.3. Canvas chính

- Nền giấy trắng, mặc định có **chấm nhỏ đều nhau (Dotted Paper)**, **có thể tuỳ chỉnh**: trắng trơn, chấm, kẻ ô, kẻ dòng.
- **Margin lines 2 bên** (yêu cầu gốc của người dùng): bật/tắt, chỉnh vị trí. Đặt trong Paper settings.
- Giấy trải gần hết vùng làm việc, **không có đường viền chia ô nội dung cố định**.
- Trên cùng một trang có thể trộn: nét viết, văn bản, hình vẽ, sơ đồ, ảnh, công thức LaTeX.
- Zoom và kéo trang, nét luôn sắc (roadmap 1.6).

### 2.4. Thanh thumbnail trang

Bật bằng nút "Danh sách trang". Hiện cột thumbnail các trang bên trái (hoặc bên phải, chọn khi làm), bấm để nhảy trang, có thêm/xoá trang. Làm ở roadmap 3.1–3.2.

---

## 3. Library – màn hình quản lý ghi chú

### 3.1. Bố cục 2 phần

| Phần | Chiều rộng | Nội dung |
|---|---|---|
| **Sidebar trái** | Khoảng 18% | Tên/logo InkNote (tự vẽ), mục **Documents** (đang chọn), mục **Trash** |
| **Nội dung chính** | Phần còn lại | Thanh trên + lưới ghi chú/thư mục |

Sidebar nền xám rất nhạt, cố định. Mục đang chọn có **nền xanh nhạt bo góc**. Mỗi mục: icon nhỏ bên trái, chữ bên phải, các hàng cách đều nhau.

### 3.2. Thanh phía trên của nội dung chính

- Nút **quay lại** (mũi tên) bên trái, khi đang ở trong một thư mục.
- **Tiêu đề khu vực** (tên thư mục hoặc "Documents").
- **Ô tìm kiếm** bên phải: chữ nhật bo tròn, có nút **X** để xoá từ khoá.
- **Bánh răng Cài đặt** ngoài ô tìm kiếm, bên phải.
- Tìm kiếm **theo tên** là đủ. Tìm theo chữ viết tay: **bỏ**.

### 3.3. Lưới (Grid View)

- Nhiều cột, xếp từ trái sang phải rồi xuống hàng; khoảng **5 mục mỗi hàng** ở cửa sổ rộng (tự giảm cột khi cửa sổ hẹp).
- Khoảng cách ngang/dọc **rộng rãi**, nhiều khoảng trắng; thẻ không viền nổi, **thumbnail là yếu tố nhận diện chính**.
- **Thư mục và ghi chú nằm chung một lưới**, không tách thành hai khu vực.

### 3.4. Thành phần trong mỗi thẻ

| Thành phần | Mô tả | Mức ưu tiên |
|---|---|---|
| Thumbnail | Ảnh trang đầu của ghi chú, hoặc icon thư mục | P3 |
| Tên | Ngay dưới thumbnail | P3 |
| Metadata | Dòng nhỏ: thời gian cập nhật | P3 |
| Yêu thích (ngôi sao ở góc thumbnail) | Đánh dấu ghi chú hay dùng | Sau (tuỳ chọn) |
| Mũi tên dropdown cạnh tên | Mở menu: Đổi tên, Nhân bản, Yêu thích, Chuyển vào Thùng rác. Có thể dùng chuột phải thay thế | P3 |
| Dấu người cộng tác (Collaborator) | **Bỏ**, ngoài phạm vi |

### 3.5. Trash

Danh sách mục đã xoá; mỗi mục có **Khôi phục** và **Xoá vĩnh viễn**; có nút "Dọn thùng rác". Xoá ghi chú từ Library luôn chuyển vào Trash trước.

---

## 4. Phong cách hình ảnh (gợi ý ban đầu, chỉnh trong `theme.rs`)

| Thành phần | Gợi ý |
|---|---|
| Toolbar chính | Nền xanh dương, icon trắng, nút đang chọn có ô nền sáng bo góc |
| Thanh tuỳ chọn + Pop-up | Nền tối, bo tròn, tiêu đề phía trên, chia nhóm rõ |
| Sidebar Library | Nền xám rất nhạt, mục đang chọn nền xanh nhạt bo góc |
| Nội dung Library | Nền trắng, nhiều khoảng trắng |
| Canvas | Trắng, chấm nhỏ đều |
| Icon | Nét đồng đều, cùng kích thước, tự vẽ hoặc bộ icon giấy phép tương thích |
| Chữ | Phông hỗ trợ **tiếng Việt đầy đủ dấu** (kiểm tra thật bằng chữ "Tiếng Việt, ắ ằ ẳ ẵ ặ ơ ư") |
| Ngôn ngữ giao diện | **Tiếng Anh** (đã chốt ngày 2026-11-12; phông chữ vẫn hỗ trợ đầy đủ tiếng Việt để nhập nội dung và tên ghi chú) |
| Chế độ chỉ đọc & Màu chủ đạo | **Giữ nguyên**: có nút Chế độ chỉ đọc (Read-only) ở P3; thanh công cụ chính giữ màu **xanh dương** |

---

## 5. Hộp thoại LaTeX (liên kết `docs/02-architecture.md`)

Giao diện của hộp thoại này đi theo workflow đã chốt trong kế hoạch LaTeX:

- Ô nhập code, **xem trước trực tiếp** (gõ xong chờ khoảng 0,5 giây thì render, chạy nền, app không đơ), báo lỗi rõ khi sai.
- Mở lại công thức bằng **double-click**: hộp thoại hiện đúng code cũ. Sửa xong giữ vị trí/kích thước.
- 1 lần sửa = 1 bước undo.
- Phong cách hộp thoại giống Pop-up (nền tối, bo góc).

---

## 6. Hành vi chung cần nhớ

- Công cụ đang chọn luôn được **tô nổi** (ô nền sáng bo góc).
- Đổi công cụ thì thanh tuỳ chọn **đổi nội dung theo**; không hiện hết mọi tuỳ chọn cùng lúc.
- Bút Wacom: bấm nút bút / đầu tẩy phía sau bút (nếu có) có thể chuyển công cụ. **Chưa chắc egui đọc được**, để sau, test riêng.
- Phím tắt tối thiểu: Ctrl+Z, Ctrl+Y (hoặc Ctrl+Shift+Z), Ctrl+S.

---

## 7. Ngoài phạm vi (KHÔNG làm, trừ khi người dùng yêu cầu lại)

- Ghi chú dính (Sticky Note), băng dính (tape), người cộng tác (Collaborator), chia sẻ/đồng bộ đám mây.
- Tìm kiếm theo chữ viết tay, nhận diện chữ viết tay.
- Android / iPad / macOS. Chỉ Windows.
- Casio 580, TikZ, compass (đã bỏ từ đầu).

---

## 8. Ưu tiên và liên hệ với roadmap

| Mức | Nội dung | Liên hệ roadmap hiện có |
|---|---|---|
| **P0 – Khung giao diện** | Cửa sổ có toolbar chính + thanh tuỳ chọn cho **Bút** (kiểu, độ dày, màu) + canvas chấm. File `theme.rs` | Sau 1.4; trước 1.5/1.6 (đây là "bước thanh công cụ cơ bản" đã đề xuất) |
| **P1 – Công cụ cốt lõi** | Tẩy (2 chế độ + bộ lọc), Lasso (2 cách khoanh + công tắc loại đối tượng), thanh thumbnail trang, Paper settings | Bước 2, 3, 4.1–4.2 |
| **P2 – Đối tượng** | LaTeX (hộp thoại + toolbar), Text, Hình học, Chèn ảnh + nút dán ảnh | Bước LaTeX, 4.3–4.4, 5 |
| **P3 – Library** | Sidebar Documents/Trash, lưới thư mục + ghi chú, tìm kiếm theo tên, menu mục, thùng rác, xuất PDF/ảnh, chế độ chỉ đọc (nếu muốn) | Bước 3.3 (danh sách ghi chú) mở rộng |

Lý do tách Library ra P3: nét bút, LaTeX và các công cụ là lý do chính làm app; Library chỉ là lớp vỏ quản lý nên làm sau, nhưng **thiết kế file (số phiên bản, kiểu đối tượng, thumbnail trang đầu) ở bước 2.1 phải tính trước** để Library không phải sửa lại file cũ.

---

## 9. Việc AI Agent phải làm lượt này

1. Đọc file này cùng `GEMINI.md`, `docs/02-architecture.md`, `docs/roadmap.md`, `docs/03-port-rnote.md`.
2. Cập nhật `docs/roadmap.md`: thêm các bước giao diện theo mục 8 (đúng thứ tự, mỗi bước nhỏ, có test thủ công và rollback bằng `git revert`; dùng git tag `step-X.Y-ok` sau khi người dùng báo OK).
3. Ghi lại vào `docs/02-architecture.md` (hoặc file mới `docs/05-ui-architecture.md`) cách chia code giao diện giữa `inknote-app`, `inknote-render` và `inknote-pen`: ai lo toolbar, ai lo vẽ canvas, ai lo theme.
4. Kiểm tra bằng cách build thử (chưa làm giao diện hoàn chỉnh): egui có làm được **thanh trên nền màu bo góc, Pop-up tối bo góc, lưới thumbnail, hiển thị tiếng Việt đủ dấu** không. Chỗ nào chưa chắc thì báo.
5. Đề xuất bộ icon miễn phí phù hợp (giấy phép tương thích GPL-3.0). Ghi tên, giấy phép, nguồn.
6. Kết thúc bằng tối đa 3 câu hỏi, ví dụ: (a) ngôn ngữ giao diện mặc định là tiếng Việt hay tiếng Anh? (b) có làm chế độ chỉ đọc không? (c) màu chủ đạo giữ xanh dương hay đổi? Rồi **DỪNG**, chưa viết code giao diện.
