# GEMINI.md – Quy tắc riêng cho Antigravity / Gemini

- Đọc `AGENTS.md` và `memory.md` ở đầu mỗi phiên.

## Quy tắc giao tiếp và làm việc
- Trả lời bằng tiếng Việt, ngắn gọn, ít thuật ngữ; nếu phải dùng thuật ngữ kỹ thuật thì giải thích ngắn gọn trong 1 câu.
- Hướng dẫn từng bước rõ ràng, giả định chủ dự án không biết code.
- Không bịa API, tên crate hay phiên bản. Chỗ nào chưa kiểm chứng thì ghi rõ "chưa chắc" và kiểm tra bằng cách đọc code trong `reference/` hoặc chạy `cargo check` / `cargo test` để xác nhận.
- Không đồng ý tuỳ tiện: nếu yêu cầu bất khả thi hoặc có rủi ro cao thì nói thẳng lý do và đề xuất phương án thay thế.
- Mỗi lượt chỉ làm đúng **1 bước nhỏ** trong `docs/roadmap.md` (kèm cách test thủ công và lệnh rollback); không tự ý nhảy bước khi chưa được duyệt.
- Cuối mỗi bước: đặt tối đa **3 câu hỏi** cần người dùng quyết định, rồi **DỪNG** chờ trả lời.
