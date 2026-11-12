# GEMINI.md – Quy tắc cho các phiên AI

## Dự án
InkNote: app ghi chú viết tay cho Windows (clone GoodNotes), viết bằng Rust, dùng bút Wacom có pressure. Chủ dự án KHÔNG biết code (vibe code), ngân sách 0 đồng, chỉ công cụ miễn phí. Đã bỏ hoàn toàn bản cũ (Electron/Flutter/web).

## Đọc trước khi làm
`docs/00-rnote-analysis.md`, `docs/01-tech-options.md`, `docs/02-architecture.md`, `docs/roadmap.md`, `docs/spike-result.md`.

## Quy tắc giao tiếp
- Trả lời tiếng Việt, ngắn gọn, ít thuật ngữ; thuật ngữ phải dùng thì giải thích 1 câu.
- Hướng dẫn từng bước, giả định chủ dự án không biết code.
- KHÔNG bịa API/crate/phiên bản. Chưa kiểm chứng thì ghi "chưa chắc" và nói cách kiểm tra (đọc code trong `reference/`, `cargo add`, chạy thử).
- Không đồng ý tuỳ tiện: yêu cầu bất khả thi/rủi ro thì nói thẳng và đưa phương án thay thế.
- Cuối mỗi bước: tối đa 3 câu hỏi cần quyết định, rồi DỪNG chờ trả lời.
- Chỉ làm đúng 1 bước nhỏ trong `docs/roadmap.md` mỗi lượt; mỗi bước có cách test thủ công và cách rollback. Không tự nhảy bước.

## Quy tắc kỹ thuật
- Môi trường: Windows, PowerShell. Rust ở `%USERPROFILE%\.cargo\bin` (có thể phải thêm vào PATH trong phiên mới). Có MSVC Build Tools, TeX Live 2026, Node.
- `reference/` (Rnote, Xournal++) là **chỉ đọc**, không sửa, không build.
- `spike/` là thí nghiệm bỏ đi, không phải mã sản phẩm.
- Tách lõi và giao diện (xem `docs/02-architecture.md`).
- Eraser chỉ xoá nét bút, không bao giờ xoá LaTeX/hình/ảnh.
- LaTeX: lưu `source` làm dữ liệu chính; double-click mở lại code cũ; live preview; giữ vị trí/kích thước khi sửa; 1 lần sửa = 1 bước undo.
- Ảnh phải lưu bản gốc, vẽ lại theo zoom (không mờ).
- Commit nhỏ, thông điệp tiếng Việt rõ ràng. Không commit `target/`.
- Giấy phép: GPL (sẽ đưa toàn bộ source lên GitHub). Chỉ *đọc* Xournal++ (GPLv2), tự viết lại; được lấy ý tưởng/code từ Rnote (GPLv3) nhưng ghi nguồn.
- Chưa chắc một API thì kiểm tra bằng cách biên dịch thử, không đoán.

