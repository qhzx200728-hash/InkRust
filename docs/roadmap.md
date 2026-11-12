# Roadmap InkNote

Quy tắc chung:
- Mỗi bước nhỏ = 1 commit git. **Rollback** chung: `git revert <mã commit>` (hoặc `git reset --hard <mã commit trước>` nếu chưa đẩy lên GitHub). Trước mỗi bước: `git status` phải sạch.
- Mỗi bước xong phải có người dùng test thủ công và báo "OK" mới sang bước sau.
- Pressure: 0 = chạm nhẹ nhất, 1 = ấn mạnh nhất.

## Bước 0 – Spike (đang làm)
- Thử pressure (egui → Tauri → Slint), thử `ink-stroke-modeler-rs`, thử LaTeX → SVG.
- Test: xem `docs/spike-result.md`. Rollback: xoá thư mục `spike/`.

## Bước 1 – Nét bút (tách nhỏ)
| Bước | Việc | Test thủ công | Rollback |
|---|---|---|---|
| 1.1 | Cửa sổ trống + khung dự án (workspace: core/pen/app) | Mở app thấy cửa sổ trắng, đóng được | `git revert` commit 1.1 |
| 1.2 | Vẽ nét bằng bút, độ dày theo pressure (chưa làm mượt) | Vẽ nhẹ → mỏng; vẽ mạnh → dày; số pressure đổi | `git revert` 1.2 |
| 1.3 | Làm mượt qua `ink-stroke-modeler-rs` (tham số như Rnote) | Vẽ đường cong nhanh: nét mượt, không răng cưa; so với Rnote thật cạnh nhau | `git revert` 1.3 (tắt làm mượt, vẫn vẽ được) |
| 1.4 | Hình dạng nét giống Rnote (đầu nét, độ dày, màu, cỡ bút) | Vẽ cùng 1 chữ ký trên Rnote và InkNote, chụp ảnh so sánh | `git revert` 1.4 |
| 1.5 | Undo/redo cho nét | Vẽ 3 nét, Ctrl+Z 3 lần hết, Ctrl+Y 3 lần hiện lại | `git revert` 1.5 |
| 1.6 | Zoom/kéo trang, nét vẫn sắc | Zoom 300% nét không vỡ | `git revert` 1.6 |

## Bước 2 – Trang và file
| Bước | Việc | Test | Rollback |
|---|---|---|---|
| 2.1 | Lưu/mở file | Vẽ, lưu, tắt app, mở lại: còn nguyên | revert |
| 2.2 | Paper settings (kẻ ô, margin 2 bên) | Đổi cỡ ô/lề, xem trang đổi theo | revert |
| 2.3 | Auto-save | Vẽ, tắt app bằng Task Manager, mở lại: mất tối đa vài giây | revert |

## Bước 3 – Nhiều trang, danh sách ghi chú
3.1 nhiều trang + thêm/xoá trang. 3.2 thumbnail. 3.3 danh sách ghi chú (tạo/mở/xoá/đổi tên).
Test: tạo 10 trang, thumbnail đúng nội dung, đóng mở không mất. Rollback: revert.

## Bước 4 – Công cụ
4.1 Eraser (chỉ xoá nét bút). 4.2 Lasso (chọn, kéo, xoá). 4.3 Hình học cơ bản. 4.4 Text.
Test eraser: đặt 1 nét + 1 ảnh + 1 công thức, tẩy qua cả ba: chỉ nét mất. Rollback: revert từng bước.

## Bước 5 – Ảnh
5.1 Chèn từ file. 5.2 Nút paste ảnh (clipboard). 5.3 Giữ nét sắc khi zoom.
Test: chèn ảnh chữ nhỏ, zoom 300% vẫn rõ. Rollback: revert.

## Bước 6 – LaTeX (kiểu Xournal++)
| Bước | Việc | Test | Rollback |
|---|---|---|---|
| 6.1 | Chạy `latex` + `dvisvgm`, hiện SVG (chưa dialog) | Công thức + câu tiếng Việt hiện đúng, sắc nét | revert |
| 6.2 | Dialog + live preview + báo lỗi | Gõ, thấy hình sau ~0,5 giây; gõ sai → báo lỗi, app không đơ | revert |
| 6.3 | Chèn vào trang (1 undo) | Chèn, Ctrl+Z biến mất, Ctrl+Y hiện lại | revert |
| 6.4 | Double-click mở lại code cũ, sửa, giữ vị trí/kích thước, 1 undo | Sửa xong công thức không nhảy chỗ; Ctrl+Z về bản cũ đúng 1 lần | revert |
| 6.5 | Lưu/mở file giữ source LaTeX | Tắt mở lại, double-click vẫn ra code cũ | revert |

## Bước 7 – Hoàn thiện
Dọn lỗi, đóng gói `.exe`, README cho GitHub. Test: chạy trên máy sạch (không có Rust). Rollback: revert.

