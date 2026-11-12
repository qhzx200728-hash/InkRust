// InkNote core (Bước 1.7): dữ liệu trang + hệ thống lệnh (Command) + Undo/Redo.
// Mọi thay đổi nội dung trang PHẢI đi qua `Document::execute` để có thể hoàn tác.

use inknote_pen::{PenPath, Style};

/// Một nét bút đã hoàn thành. `id` duy nhất, dùng để app lưu bộ nhớ đệm hình vẽ.
#[derive(Clone)]
pub struct Stroke {
    pub id: u64,
    pub path: PenPath,
    pub style: Style,
}

/// Lệnh thay đổi nội dung trang.
pub enum Command {
    /// Thêm một nét vào cuối trang.
    AddStroke(Stroke),
    /// Xoá toàn bộ nét trên trang.
    ClearPage,
}

/// Lệnh đã thực thi, kèm dữ liệu cần để hoàn tác.
enum Done {
    AddStroke(Stroke),
    ClearPage(Vec<Stroke>),
}

/// Trang + lịch sử Undo/Redo.
#[derive(Default)]
pub struct Document {
    strokes: Vec<Stroke>,
    undo_stack: Vec<Done>,
    redo_stack: Vec<Done>,
    next_id: u64,
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    /// Cấp id mới cho nét bút.
    pub fn alloc_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    pub fn strokes(&self) -> &[Stroke] {
        &self.strokes
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Thực thi lệnh mới. Xoá nhánh Redo (như mọi trình soạn thảo).
    pub fn execute(&mut self, cmd: Command) {
        let done = match cmd {
            Command::AddStroke(s) => {
                self.strokes.push(s.clone());
                Done::AddStroke(s)
            }
            Command::ClearPage => {
                if self.strokes.is_empty() {
                    return; // không có gì để xoá → không tạo bước undo rỗng
                }
                Done::ClearPage(std::mem::take(&mut self.strokes))
            }
        };
        self.undo_stack.push(done);
        self.redo_stack.clear();
    }

    /// Hoàn tác 1 bước. Trả về `true` nếu có thay đổi.
    pub fn undo(&mut self) -> bool {
        let Some(done) = self.undo_stack.pop() else {
            return false;
        };
        match &done {
            Done::AddStroke(_) => {
                self.strokes.pop();
            }
            Done::ClearPage(removed) => self.strokes = removed.clone(),
        }
        self.redo_stack.push(done);
        true
    }

    /// Làm lại 1 bước. Trả về `true` nếu có thay đổi.
    pub fn redo(&mut self) -> bool {
        let Some(done) = self.redo_stack.pop() else {
            return false;
        };
        match &done {
            Done::AddStroke(s) => self.strokes.push(s.clone()),
            Done::ClearPage(_) => self.strokes.clear(),
        }
        self.undo_stack.push(done);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inknote_pen::penpath::Element;
    use parry2d_f64::math::Vector2;

    fn stroke(doc: &mut Document) -> Stroke {
        let id = doc.alloc_id();
        let path = PenPath::new(Element::new(Vector2::new(0.0, 0.0), 0.5));
        Stroke {
            id,
            path,
            style: Style::default(),
        }
    }

    fn ids(doc: &Document) -> Vec<u64> {
        doc.strokes().iter().map(|s| s.id).collect()
    }

    #[test]
    fn three_strokes_undo_then_redo_in_order() {
        let mut d = Document::new();
        for _ in 0..3 {
            let s = stroke(&mut d);
            d.execute(Command::AddStroke(s));
        }
        assert_eq!(ids(&d), vec![1, 2, 3]);
        for _ in 0..3 {
            assert!(d.undo());
        }
        assert!(ids(&d).is_empty());
        assert!(!d.undo());
        for _ in 0..3 {
            assert!(d.redo());
        }
        assert_eq!(ids(&d), vec![1, 2, 3]);
        assert!(!d.redo());
    }

    #[test]
    fn new_command_clears_redo() {
        let mut d = Document::new();
        let s = stroke(&mut d);
        d.execute(Command::AddStroke(s));
        d.undo();
        assert!(d.can_redo());
        let s = stroke(&mut d);
        d.execute(Command::AddStroke(s));
        assert!(!d.can_redo());
    }

    #[test]
    fn clear_page_is_one_undo_step() {
        let mut d = Document::new();
        for _ in 0..3 {
            let s = stroke(&mut d);
            d.execute(Command::AddStroke(s));
        }
        d.execute(Command::ClearPage);
        assert!(d.strokes().is_empty());
        d.undo();
        assert_eq!(ids(&d), vec![1, 2, 3]);
        d.redo();
        assert!(d.strokes().is_empty());
    }

    #[test]
    fn clear_empty_page_makes_no_step() {
        let mut d = Document::new();
        d.execute(Command::ClearPage);
        assert!(!d.can_undo());
    }
}
