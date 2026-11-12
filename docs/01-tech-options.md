# 01 – So sánh hướng công nghệ

Trạng thái: **ĐÃ CHỐT egui (hướng B)** ngày 2026-11-12 - pressure Wacom chạy tốt trong spike (xem `spike-result.md`). Tauri/Slint không cần thử.

| | A. Engine Rnote + GTK4 | **B. egui (Rust thuần)** | Tauri 2 (Rust + web canvas) | Slint |
|---|---|---|---|---|
| Nét bút giống Rnote | Giống tuyệt đối | Làm mượt giống (cùng thư viện); độ dày/đầu nét viết lại | Phải viết lại cả vẽ bằng JS/canvas | Viết lại |
| Build trên Windows | Khó (cairo, GTK qua MSYS2/gvsbuild); `rnote-compose` đòi cairo | Dễ (đã build được, chỉ cần MSVC Build Tools) | Dễ vừa (cần WebView2, có sẵn Win10/11; cần Node) | Dễ |
| Pressure Wacom | Chưa thử | **Đang thử** (qua sự kiện touch/force của egui) | Chắc chắn hơn: web có `PointerEvent.pressure` | Chưa thử |
| Phần LaTeX/ảnh/thumbnail | Cairo/Poppler | Tự làm (SVG → vẽ) | Rất dễ: SVG hiển thị sẵn trong web | Tự làm |
| Dễ cho AI viết, bạn đọc lỗi | Khó | Trung bình | Dễ (JS) nhưng trộn 2 ngôn ngữ | Trung bình |
| Rủi ro lớn nhất | Build | Pressure có đọc được không | Không "thuần Rust"; hiệu năng canvas với nhiều nét | Hệ sinh thái nhỏ |

## Đề xuất
1. **Thử egui trước** (bạn đã chọn hướng B). Nếu pressure chạy tốt ⇒ dùng egui.
2. Nếu egui không đọc được pressure ⇒ **Tauri 2** (pressure của web gần như chắc chắn chạy với Wacom). Phần nét bút làm mượt vẫn dùng Rust (`ink-stroke-modeler-rs`) hoặc port sang JS.
3. Slint chỉ thử nếu cả hai đều lỗi.
4. Hướng A chỉ quay lại nếu hướng B không thể đạt chất lượng nét như Rnote.

## Cách kiểm tra "chưa chắc"
- Pressure trong egui: chạy `spike/egui-pressure`, xem số pressure trên màn hình.
- Pressure trong Tauri/Slint: chưa thử, chỉ thử khi cần.


