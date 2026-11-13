// Quản lý Auto-save (Bước 2.3): tự động lưu ngầm sau 2,5 giây ngừng chỉnh sửa
// bằng cơ chế atomic rename (ghi file tạm .tmp rồi đổi tên đè lên file chính).

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use inknote_core::Document;

/// Thời gian chờ sau thay đổi cuối cùng trước khi tự động lưu (debounce).
pub const AUTOSAVE_DEBOUNCE: Duration = Duration::from_millis(2500);

/// Thời gian hiển thị thông báo "Auto-saved" sau mỗi lần tự động lưu (3 giây).
pub const AUTOSAVE_NOTICE_DURATION: Duration = Duration::from_secs(3);

struct InFlightSave {
    job_id: u64,
    saved_rev: u64,
    rx: Receiver<Result<(), String>>,
}

/// Thông tin trạng thái file truyền lên Thanh công cụ chính (bên phải nút Read-Only).
#[derive(Clone, Debug)]
pub struct FileStatusInfo<'a> {
    pub file_name: &'a str,
    pub is_saved: bool,
    pub has_path: bool,
    pub autosaved_notice: Option<&'static str>,
    pub has_error: bool,
}

/// Bộ điều phối Auto-save chạy ở luồng nền để không làm khựng nét viết.
pub struct AutoSaveState {
    last_tracked_rev: u64,
    last_change_at: Option<Instant>,
    in_flight: Option<InFlightSave>,
    next_job_id: u64,
    min_valid_job_id: u64,
    last_autosaved_notice_at: Option<Instant>,
    last_error: Option<String>,
}

impl AutoSaveState {
    pub fn new(initial_rev: u64) -> Self {
        Self {
            last_tracked_rev: initial_rev,
            last_change_at: None,
            in_flight: None,
            next_job_id: 1,
            min_valid_job_id: 1,
            last_autosaved_notice_at: None,
            last_error: None,
        }
    }

    /// Đặt lại trạng thái theo dõi khi mở file mới hoặc tạo ghi chú mới.
    pub fn reset(&mut self, current_rev: u64) {
        self.min_valid_job_id = self.next_job_id;
        self.last_tracked_rev = current_rev;
        self.last_change_at = None;
        self.last_autosaved_notice_at = None;
        self.last_error = None;
    }

    /// Ghi nhận người dùng vừa lưu thủ công thành công (không bật thông báo auto-save).
    pub fn note_manual_save(&mut self, current_rev: u64) {
        self.min_valid_job_id = self.next_job_id;
        self.last_tracked_rev = current_rev;
        self.last_change_at = None;
        self.last_autosaved_notice_at = None;
        self.last_error = None;
    }

    /// Kiểm tra kết quả luồng nền và kích hoạt auto-save nếu đã quá thời gian debounce.
    pub fn poll_and_maybe_save(
        &mut self,
        doc: &mut Document,
        current_path: Option<&PathBuf>,
        is_drawing: bool,
        now: Instant,
    ) {
        self.poll_in_flight(doc, now);

        let rev = doc.revision();
        if rev != self.last_tracked_rev {
            self.last_tracked_rev = rev;
            self.last_change_at = Some(now);
            self.last_autosaved_notice_at = None;
            self.last_error = None;
        }

        let Some(path) = current_path else {
            return;
        };
        if !doc.is_dirty() || is_drawing || self.in_flight.is_some() {
            return;
        }

        let changed_at = *self.last_change_at.get_or_insert(now);
        if now.duration_since(changed_at) < AUTOSAVE_DEBOUNCE {
            return;
        }

        let job_id = self.next_job_id;
        self.next_job_id += 1;
        let snapshot = doc.to_file();
        let path_clone = path.clone();
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let res = snapshot
                .save_to_path(&path_clone)
                .map_err(|e| e.to_string());
            let _ = tx.send(res);
        });

        self.in_flight = Some(InFlightSave {
            job_id,
            saved_rev: rev,
            rx,
        });
    }

    fn poll_in_flight(&mut self, doc: &mut Document, now: Instant) {
        let Some(job) = &self.in_flight else {
            return;
        };
        match job.rx.try_recv() {
            Ok(result) => {
                let job = self.in_flight.take().unwrap();
                if job.job_id < self.min_valid_job_id {
                    return;
                }
                match result {
                    Ok(()) => {
                        doc.mark_saved_at(job.saved_rev);
                        self.last_error = None;
                        if !doc.is_dirty() {
                            self.last_change_at = None;
                            // Chỉ hiện thông báo "Auto-saved" 3s khi mọi thay đổi đã được tự lưu sạch
                            self.last_autosaved_notice_at = Some(now);
                        }
                    }
                    Err(err) => {
                        eprintln!("Auto-save error: {err}");
                        self.last_error = Some(err);
                        // Đợi đủ 1 chu kỳ debounce nữa mới thử lại, tránh ghi dồn dập khi gặp lỗi
                        self.last_change_at = Some(now);
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.in_flight = None;
                self.last_change_at = Some(now);
            }
        }
    }

    /// Đợi luồng nền đang chạy (nếu có) và lưu nốt thay đổi cuối trước khi đóng app.
    pub fn flush_on_exit(&mut self, doc: &mut Document, current_path: Option<&PathBuf>) {
        if let Some(job) = self.in_flight.take() {
            if let Ok(Ok(())) = job.rx.recv() {
                if job.job_id >= self.min_valid_job_id {
                    doc.mark_saved_at(job.saved_rev);
                }
            }
        }

        if let Some(path) = current_path {
            if doc.is_dirty() {
                if let Err(e) = doc.to_file().save_to_path(path) {
                    eprintln!("Auto-save on exit error {:?}: {e}", path);
                } else {
                    doc.mark_saved();
                }
            }
        }
    }

    /// Thông báo đã lưu chỉ hiện trong 3 giây sau mỗi lần tự lưu thành công.
    pub fn autosaved_notice(&self, doc: &Document, now: Instant) -> Option<&'static str> {
        if doc.is_dirty() {
            return None;
        }
        let saved_at = self.last_autosaved_notice_at?;
        if now.saturating_duration_since(saved_at) < AUTOSAVE_NOTICE_DURATION {
            Some("Auto-saved")
        } else {
            None
        }
    }

    /// Tạo gói thông tin trạng thái để vẽ lên thanh công cụ chính.
    pub fn file_status<'a>(
        &self,
        doc: &Document,
        file_name: &'a str,
        current_path: Option<&PathBuf>,
        now: Instant,
    ) -> FileStatusInfo<'a> {
        let has_path = current_path.is_some();
        let is_saved = has_path && !doc.is_dirty();
        FileStatusInfo {
            file_name,
            is_saved,
            has_path,
            autosaved_notice: self.autosaved_notice(doc, now),
            has_error: self.last_error.is_some(),
        }
    }

    /// Thời gian cần hẹn vẽ lại UI cho bộ đếm auto-save hoặc tự ẩn thông báo sau 3s.
    pub fn next_repaint_delay(
        &self,
        doc: &Document,
        current_path: Option<&PathBuf>,
        now: Instant,
    ) -> Option<Duration> {
        if self.in_flight.is_some() {
            return Some(Duration::from_millis(50));
        }
        if current_path.is_some() && doc.is_dirty() {
            if let Some(changed_at) = self.last_change_at {
                let elapsed = now.saturating_duration_since(changed_at);
                let rem = AUTOSAVE_DEBOUNCE.saturating_sub(elapsed);
                return Some(rem.max(Duration::from_millis(20)));
            }
        }
        if !doc.is_dirty() {
            if let Some(saved_at) = self.last_autosaved_notice_at {
                let elapsed = now.saturating_duration_since(saved_at);
                if elapsed < AUTOSAVE_NOTICE_DURATION {
                    return Some(
                        (AUTOSAVE_NOTICE_DURATION - elapsed).max(Duration::from_millis(20)),
                    );
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inknote_core::{Command, DocumentFile, PaperPattern, Stroke};
    use inknote_pen::{PenPath, Style, penpath::Element};
    use parry2d_f64::math::Vector2;

    fn add_test_stroke(doc: &mut Document) {
        let id = doc.alloc_id();
        let path = PenPath::new(Element::new(Vector2::new(10.0, 20.0), 0.5));
        doc.execute(Command::AddStroke(Stroke {
            id,
            path,
            style: Style::default(),
        }));
    }

    fn wait_until_idle(state: &mut AutoSaveState, doc: &mut Document, path: &PathBuf, t: Instant) {
        for _ in 0..100 {
            state.poll_and_maybe_save(doc, Some(path), false, t);
            if state.in_flight.is_none() {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("autosave background thread did not finish in time");
    }

    #[test]
    fn autosave_triggers_after_debounce_and_shows_notice_for_3s() {
        let temp_file = std::env::temp_dir().join("test_autosave_debounce.inknote");
        let _ = std::fs::remove_file(&temp_file);

        let mut doc = Document::new();
        doc.to_file().save_to_path(&temp_file).unwrap();
        doc.mark_saved();

        let t0 = Instant::now();
        let mut state = AutoSaveState::new(doc.revision());

        // Mới mở file: chưa tự lưu lần nào nên không hiện thông báo "Auto-saved"
        assert_eq!(state.autosaved_notice(&doc, t0), None);

        add_test_stroke(&mut doc);
        assert!(doc.is_dirty());

        // Chưa đủ 2.5s -> chưa kích hoạt lưu
        state.poll_and_maybe_save(&mut doc, Some(&temp_file), false, t0);
        assert!(state.in_flight.is_none());
        assert!(doc.is_dirty());

        state.poll_and_maybe_save(
            &mut doc,
            Some(&temp_file),
            false,
            t0 + Duration::from_millis(2000),
        );
        assert!(state.in_flight.is_none());
        assert!(doc.is_dirty());

        // Đủ 2.5s -> tự động lưu ra file
        let t_save = t0 + Duration::from_millis(2600);
        state.poll_and_maybe_save(&mut doc, Some(&temp_file), false, t_save);
        assert!(state.in_flight.is_some());

        wait_until_idle(&mut state, &mut doc, &temp_file, t_save);
        assert!(!doc.is_dirty());

        // Trong 3 giây sau khi tự lưu: hiện "Auto-saved" và chấm xanh
        let status_1s = state.file_status(
            &doc,
            "test_autosave_debounce.inknote",
            Some(&temp_file),
            t_save + Duration::from_secs(1),
        );
        assert!(status_1s.is_saved);
        assert_eq!(status_1s.autosaved_notice, Some("Auto-saved"));

        // Sau 3 giây: thông báo tự ẩn đi, chấm vẫn xanh
        let status_4s = state.file_status(
            &doc,
            "test_autosave_debounce.inknote",
            Some(&temp_file),
            t_save + Duration::from_secs(4),
        );
        assert!(status_4s.is_saved);
        assert_eq!(status_4s.autosaved_notice, None);

        let loaded = DocumentFile::load_from_path(&temp_file).unwrap();
        assert_eq!(loaded.pages[0].objects.len(), 1);

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn autosave_waits_while_drawing_and_ignores_unsaved_untitled_note() {
        let temp_file = std::env::temp_dir().join("test_autosave_drawing.inknote");
        let _ = std::fs::remove_file(&temp_file);

        let mut doc = Document::new();
        let t0 = Instant::now();
        let mut state = AutoSaveState::new(doc.revision());

        add_test_stroke(&mut doc);

        // Chưa có đường dẫn lưu -> chấm đỏ (is_saved == false) và không auto-save
        state.poll_and_maybe_save(&mut doc, None, false, t0);
        state.poll_and_maybe_save(&mut doc, None, false, t0 + Duration::from_secs(5));
        assert!(state.in_flight.is_none());
        assert!(doc.is_dirty());
        assert!(!state.file_status(&doc, "Untitled Note", None, t0).is_saved);

        // Có đường dẫn nhưng đang vẽ dở -> hoãn lưu
        doc.to_file().save_to_path(&temp_file).unwrap();
        state.poll_and_maybe_save(&mut doc, Some(&temp_file), true, t0 + Duration::from_secs(6));
        assert!(state.in_flight.is_none());

        // Nhấc bút -> lưu ngay vì đã quá debounce
        let t_done = t0 + Duration::from_secs(6);
        state.poll_and_maybe_save(&mut doc, Some(&temp_file), false, t_done);
        assert!(state.in_flight.is_some());

        wait_until_idle(&mut state, &mut doc, &temp_file, t_done);
        assert!(!doc.is_dirty());

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn autosave_persists_paper_settings_changes() {
        let temp_file = std::env::temp_dir().join("test_autosave_paper.inknote");
        let _ = std::fs::remove_file(&temp_file);

        let mut doc = Document::new();
        doc.to_file().save_to_path(&temp_file).unwrap();
        doc.mark_saved();

        let t0 = Instant::now();
        let mut state = AutoSaveState::new(doc.revision());

        let mut ps = doc.paper_settings().clone();
        ps.pattern = PaperPattern::Grid;
        assert!(doc.set_paper_settings(ps));

        state.poll_and_maybe_save(&mut doc, Some(&temp_file), false, t0);
        let t_save = t0 + Duration::from_secs(3);
        state.poll_and_maybe_save(&mut doc, Some(&temp_file), false, t_save);
        wait_until_idle(&mut state, &mut doc, &temp_file, t_save);

        assert!(!doc.is_dirty());
        let loaded = DocumentFile::load_from_path(&temp_file).unwrap();
        assert_eq!(loaded.paper_settings.pattern, PaperPattern::Grid);

        let _ = std::fs::remove_file(temp_file);
    }
}
