// Quản lý thư viện ghi chú & thư mục cho màn hình Library (Bước 7.1 - P3).
// Độc lập với egui theo kiến trúc phân tách ở docs/02-architecture.md.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::{Document, DocumentFile, DocumentPage, PaperSettings, page_from_file, unix_secs_now};
use serde::{Deserialize, Serialize};

/// Khu vực hiển thị trên Sidebar trái của màn hình Library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibrarySection {
    Documents,
    Trash,
}

/// Dữ liệu tóm tắt của 1 ghi chú `.inknote` hiển thị thành thẻ trong lưới Library.
#[derive(Clone, Debug)]
pub struct LibraryNoteEntry {
    pub path: PathBuf,
    /// Đường dẫn thư mục cha tương đối so với gốc section (`Documents` hoặc `Trash`).
    pub rel_folder: Vec<String>,
    pub title: String,
    pub modified_at: u64,
    pub is_favorite: bool,
    pub page_count: usize,
    pub paper_settings: PaperSettings,
    pub first_page: Option<DocumentPage>,
}

/// Dữ liệu tóm tắt của 1 thư mục hiển thị cùng lưới với ghi chú.
#[derive(Clone, Debug)]
pub struct LibraryFolderEntry {
    pub path: PathBuf,
    /// Đường dẫn thư mục cha tương đối so với gốc section.
    pub rel_folder: Vec<String>,
    pub name: String,
    pub item_count: usize,
    pub modified_at: u64,
}

/// Mục hiển thị chung trong Grid View (không tách thành 2 khu vực riêng).
#[derive(Clone, Debug)]
pub enum LibraryItem {
    Folder(LibraryFolderEntry),
    Note(LibraryNoteEntry),
}

impl LibraryItem {
    pub fn name(&self) -> &str {
        match self {
            LibraryItem::Folder(f) => &f.name,
            LibraryItem::Note(n) => &n.title,
        }
    }

    pub fn modified_at(&self) -> u64 {
        match self {
            LibraryItem::Folder(f) => f.modified_at,
            LibraryItem::Note(n) => n.modified_at,
        }
    }

    pub fn is_folder(&self) -> bool {
        matches!(self, LibraryItem::Folder(_))
    }
}

/// Lưu trữ thư viện cục bộ tại `%APPDATA%/InkNote/library/` (`Documents/` và `Trash/`).
#[derive(Clone, Debug)]
pub struct LibraryStore {
    root_dir: PathBuf,
}

impl LibraryStore {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    pub fn default_root_dir() -> PathBuf {
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join("InkNote").join("library")
        } else {
            PathBuf::from(".inknote_library")
        }
    }

    pub fn open_default() -> Self {
        let store = Self::new(Self::default_root_dir());
        let _ = store.ensure_dirs();
        store
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn documents_dir(&self) -> PathBuf {
        self.root_dir.join("Documents")
    }

    pub fn trash_dir(&self) -> PathBuf {
        self.root_dir.join("Trash")
    }

    pub fn section_dir(&self, section: LibrarySection) -> PathBuf {
        match section {
            LibrarySection::Documents => self.documents_dir(),
            LibrarySection::Trash => self.trash_dir(),
        }
    }

    pub fn ensure_dirs(&self) -> io::Result<()> {
        fs::create_dir_all(self.documents_dir())?;
        fs::create_dir_all(self.trash_dir())?;
        Ok(())
    }

    /// Giải quyết đường dẫn thư mục hiện tại một cách an toàn bên trong `section_dir`.
    pub fn resolve_folder(&self, section: LibrarySection, rel_folder: &[String]) -> PathBuf {
        let mut dir = self.section_dir(section);
        for seg in rel_folder {
            let safe = sanitize_fs_name(seg, "Folder");
            dir.push(safe);
        }
        dir
    }

    /// Tạo thư mục con mới bên trong `Documents/rel_folder`, tự xử lý trùng tên.
    pub fn create_folder(&self, rel_folder: &[String], raw_name: &str) -> io::Result<String> {
        self.ensure_dirs()?;
        let parent = self.resolve_folder(LibrarySection::Documents, rel_folder);
        fs::create_dir_all(&parent)?;
        let base = sanitize_fs_name(raw_name, "New Folder");
        let path = unique_folder_path(&parent, &base);
        fs::create_dir_all(&path)?;
        Ok(path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&base)
            .to_string())
    }

    /// Tạo ghi chú mới với bộ cài đặt giấy mặc định và lưu sẵn vào thư mục trong `Documents`.
    pub fn create_note_file(
        &self,
        rel_folder: &[String],
        raw_title: &str,
        default_paper: &PaperSettings,
    ) -> io::Result<(PathBuf, Document)> {
        self.ensure_dirs()?;
        let parent = self.resolve_folder(LibrarySection::Documents, rel_folder);
        fs::create_dir_all(&parent)?;
        let display_title = clean_display_title(raw_title, "Untitled Note");
        let file_base = sanitize_fs_name(&display_title, "Untitled Note");
        let path = unique_note_path(&parent, &file_base);

        let mut doc = Document::new();
        *doc.paper_settings_mut() = default_paper.clone();
        doc.metadata_mut().title = display_title;
        doc.to_file().save_to_path(&path)?;
        doc.mark_saved();
        Ok((path, doc))
    }

    /// Lưu ghi chú chưa có file chính thức vào thư mục `Documents/rel_folder` khi quay về Home.
    pub fn save_untitled_into_folder(
        &self,
        rel_folder: &[String],
        doc: &mut Document,
    ) -> io::Result<PathBuf> {
        self.ensure_dirs()?;
        let parent = self.resolve_folder(LibrarySection::Documents, rel_folder);
        fs::create_dir_all(&parent)?;
        let title = clean_display_title(&doc.metadata().title, "Untitled Note");
        doc.metadata_mut().title = title.clone();
        let file_base = sanitize_fs_name(&title, "Untitled Note");
        let path = unique_note_path(&parent, &file_base);
        doc.to_file().save_to_path(&path)?;
        doc.mark_saved();
        Ok(path)
    }

    /// Quét danh sách thư mục và ghi chú (dùng chung bộ nhớ đệm đọc file theo `(mtime, size)`).
    /// Khi `search_query` không rỗng, tìm đệ quy từ thư mục đang mở trở xuống theo tên.
    pub fn scan_items(
        &self,
        section: LibrarySection,
        rel_folder: &[String],
        search_query: &str,
        note_cache: &mut HashMap<PathBuf, (u64, u64, LibraryNoteEntry)>,
    ) -> io::Result<Vec<LibraryItem>> {
        self.ensure_dirs()?;
        let start_dir = self.resolve_folder(section, rel_folder);
        if !start_dir.exists() {
            return Ok(Vec::new());
        }

        let query = search_query.trim().to_lowercase();
        let recursive = section == LibrarySection::Documents && !query.is_empty();
        let mut items = Vec::new();
        collect_dir_items(
            &start_dir, rel_folder, recursive, &query, note_cache, &mut items,
        )?;

        items.sort_by(|a, b| match (a, b) {
            (LibraryItem::Folder(fa), LibraryItem::Folder(fb)) => {
                fa.name.to_lowercase().cmp(&fb.name.to_lowercase())
            }
            (LibraryItem::Folder(_), LibraryItem::Note(_)) => std::cmp::Ordering::Less,
            (LibraryItem::Note(_), LibraryItem::Folder(_)) => std::cmp::Ordering::Greater,
            (LibraryItem::Note(na), LibraryItem::Note(nb)) => nb
                .is_favorite
                .cmp(&na.is_favorite)
                .then_with(|| nb.modified_at.cmp(&na.modified_at))
                .then_with(|| na.title.to_lowercase().cmp(&nb.title.to_lowercase())),
        });

        Ok(items)
    }

    pub fn trash_item_count(&self) -> usize {
        count_visible_children(&self.trash_dir())
    }

    /// Đổi tên ghi chú (cập nhật tiêu đề bên trong `.inknote` và tên file tương ứng).
    pub fn rename_note(&self, path: &Path, raw_title: &str) -> io::Result<PathBuf> {
        let mut file = DocumentFile::load_from_path(path)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let new_title = clean_display_title(raw_title, "Untitled Note");
        file.metadata.title = new_title.clone();
        file.metadata.modified_at = unix_secs_now();

        let parent = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.documents_dir());
        let file_base = sanitize_fs_name(&new_title, "Untitled Note");
        let target_same = path
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s == file_base);
        let dest = if target_same {
            path.to_path_buf()
        } else {
            unique_note_path(&parent, &file_base)
        };

        file.save_to_path(&dest)?;
        if dest != path {
            let _ = fs::remove_file(path);
        }
        Ok(dest)
    }

    /// Đổi tên thư mục (giữ nguyên nội dung bên trong).
    pub fn rename_folder(&self, path: &Path, raw_name: &str) -> io::Result<PathBuf> {
        let new_name = sanitize_fs_name(raw_name, "Folder");
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == new_name)
        {
            return Ok(path.to_path_buf());
        }
        let parent = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.documents_dir());
        let dest = unique_folder_path(&parent, &new_name);
        move_path_cleanly(path, &dest)?;
        Ok(dest)
    }

    /// Nhân bản một ghi chú thành bản sao mới cùng thư mục (không đánh dấu yêu thích sẵn).
    pub fn duplicate_note(&self, path: &Path) -> io::Result<PathBuf> {
        let mut file = DocumentFile::load_from_path(path)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let base_title = clean_display_title(&file.metadata.title, "Untitled Note");
        let copy_title = format!("{base_title} Copy");
        let now = unix_secs_now();
        file.metadata.title = copy_title.clone();
        file.metadata.is_favorite = false;
        file.metadata.created_at = now;
        file.metadata.modified_at = now;

        let parent = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.documents_dir());
        let file_base = sanitize_fs_name(&copy_title, "Untitled Note Copy");
        let dest = unique_note_path(&parent, &file_base);
        file.save_to_path(&dest)?;
        Ok(dest)
    }

    /// Bật/tắt đánh dấu Yêu thích (ngôi sao) trên ghi chú.
    pub fn toggle_favorite_note(&self, path: &Path) -> io::Result<bool> {
        let mut file = DocumentFile::load_from_path(path)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let next = !file.metadata.is_favorite;
        file.metadata.is_favorite = next;
        file.save_to_path(path)?;
        Ok(next)
    }

    /// Chuyển ghi chú hoặc thư mục vào `Trash/` kèm sidecar lưu thư mục gốc để Khôi phục về đúng chỗ.
    pub fn move_to_trash(
        &self,
        path: &Path,
        original_rel_folder: &[String],
    ) -> io::Result<PathBuf> {
        self.ensure_dirs()?;
        if !path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Item does not exist",
            ));
        }
        let trash_dir = self.trash_dir();
        let is_dir = path.is_dir();
        let original_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Item")
            .to_string();

        let dest = if is_dir {
            let base = sanitize_fs_name(&original_name, "Folder");
            unique_folder_path(&trash_dir, &base)
        } else {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled Note");
            let base = sanitize_fs_name(stem, "Untitled Note");
            unique_note_path(&trash_dir, &base)
        };

        move_path_cleanly(path, &dest)?;

        let sidecar = TrashSidecar {
            original_rel_folder: original_rel_folder.to_vec(),
            original_name,
            trashed_at: unix_secs_now(),
        };
        if let Ok(json) = serde_json::to_string(&sidecar) {
            let _ = fs::write(trash_sidecar_path(&dest), json);
        }
        Ok(dest)
    }

    /// Khôi phục ghi chú hoặc thư mục từ `Trash/` về đúng thư mục gốc trong `Documents/`.
    pub fn restore_from_trash(&self, trash_path: &Path) -> io::Result<PathBuf> {
        self.ensure_dirs()?;
        if !trash_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Item in Trash no longer exists",
            ));
        }

        let sidecar_path = trash_sidecar_path(trash_path);
        let sidecar = fs::read_to_string(&sidecar_path)
            .ok()
            .and_then(|s| serde_json::from_str::<TrashSidecar>(&s).ok());

        let target_parent = match &sidecar {
            Some(meta) => self.resolve_folder(LibrarySection::Documents, &meta.original_rel_folder),
            None => self.documents_dir(),
        };
        fs::create_dir_all(&target_parent)?;

        let is_dir = trash_path.is_dir();
        let dest = if is_dir {
            let preferred = sidecar
                .as_ref()
                .map(|m| m.original_name.as_str())
                .or_else(|| trash_path.file_name().and_then(|n| n.to_str()))
                .unwrap_or("Folder");
            let base = sanitize_fs_name(preferred, "Folder");
            unique_folder_path(&target_parent, &base)
        } else {
            let preferred_name = sidecar
                .as_ref()
                .map(|m| m.original_name.as_str())
                .or_else(|| trash_path.file_name().and_then(|n| n.to_str()))
                .unwrap_or("Untitled Note.inknote");
            let stem = Path::new(preferred_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled Note");
            let base = sanitize_fs_name(stem, "Untitled Note");
            unique_note_path(&target_parent, &base)
        };

        move_path_cleanly(trash_path, &dest)?;
        let _ = fs::remove_file(sidecar_path);
        Ok(dest)
    }

    /// Xoá vĩnh viễn một ghi chú hoặc thư mục đang nằm trong `Trash/`.
    pub fn delete_permanently(&self, trash_path: &Path) -> io::Result<()> {
        if trash_path.is_dir() {
            fs::remove_dir_all(trash_path)?;
        } else if trash_path.exists() {
            fs::remove_file(trash_path)?;
        }
        let sidecar_path = trash_sidecar_path(trash_path);
        if sidecar_path.exists() {
            let _ = fs::remove_file(sidecar_path);
        }
        Ok(())
    }

    /// Dọn sạch toàn bộ mục trong `Trash/`.
    pub fn empty_trash(&self) -> io::Result<()> {
        let trash_dir = self.trash_dir();
        if !trash_dir.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&trash_dir)?.flatten() {
            let p = entry.path();
            if p.is_dir() {
                fs::remove_dir_all(&p)?;
            } else {
                fs::remove_file(&p)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TrashSidecar {
    original_rel_folder: Vec<String>,
    original_name: String,
    trashed_at: u64,
}

fn trash_sidecar_path(trash_item_path: &Path) -> PathBuf {
    let parent = trash_item_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let name = trash_item_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("item");
    parent.join(format!(".{name}.trashmeta.json"))
}

fn move_path_cleanly(src: &Path, dest: &Path) -> io::Result<()> {
    if fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    if src.is_dir() {
        copy_dir_recursive(src, dest)?;
        fs::remove_dir_all(src)?;
    } else {
        fs::copy(src, dest)?;
        fs::remove_file(src)?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)?.flatten() {
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn collect_dir_items(
    dir: &Path,
    rel_folder: &[String],
    recursive: bool,
    query_lower: &str,
    note_cache: &mut HashMap<PathBuf, (u64, u64, LibraryNoteEntry)>,
    out: &mut Vec<LibraryItem>,
) -> io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(fname) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if fname.starts_with('.') {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };

        if meta.is_dir() {
            let folder_name = fname.to_string();
            let modified_at = metadata_modified_secs(&meta);
            let matches =
                query_lower.is_empty() || folder_name.to_lowercase().contains(query_lower);
            if matches {
                let item_count = count_visible_children(&path);
                out.push(LibraryItem::Folder(LibraryFolderEntry {
                    path: path.clone(),
                    rel_folder: rel_folder.to_vec(),
                    name: folder_name.clone(),
                    item_count,
                    modified_at,
                }));
            }
            if recursive {
                let mut child_rel = rel_folder.to_vec();
                child_rel.push(folder_name);
                collect_dir_items(&path, &child_rel, true, query_lower, note_cache, out)?;
            }
        } else if meta.is_file() && is_inknote_file(&path) {
            if let Some(mut note_entry) = read_cached_note(&path, rel_folder, &meta, note_cache) {
                let matches =
                    query_lower.is_empty() || note_entry.title.to_lowercase().contains(query_lower);
                if matches {
                    note_entry.rel_folder = rel_folder.to_vec();
                    out.push(LibraryItem::Note(note_entry));
                }
            }
        }
    }
    Ok(())
}

fn read_cached_note(
    path: &Path,
    rel_folder: &[String],
    meta: &fs::Metadata,
    cache: &mut HashMap<PathBuf, (u64, u64, LibraryNoteEntry)>,
) -> Option<LibraryNoteEntry> {
    let mtime = metadata_modified_secs(meta);
    let len = meta.len();

    if let Some((cached_mtime, cached_len, cached)) = cache.get(path) {
        if *cached_mtime == mtime && *cached_len == len {
            let mut copy = cached.clone();
            copy.rel_folder = rel_folder.to_vec();
            return Some(copy);
        }
    }

    let file = DocumentFile::load_from_path(path).ok()?;
    let fallback_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled Note");
    let title = if file.metadata.title.trim().is_empty() {
        fallback_stem.to_string()
    } else {
        file.metadata.title.clone()
    };
    let page_count = file.pages.len().max(1);
    let modified_at = file.metadata.modified_at.max(mtime);
    let first_page = file.pages.into_iter().next().map(|p| page_from_file(p).0);

    let entry = LibraryNoteEntry {
        path: path.to_path_buf(),
        rel_folder: rel_folder.to_vec(),
        title,
        modified_at,
        is_favorite: file.metadata.is_favorite,
        page_count,
        paper_settings: file.paper_settings,
        first_page,
    };
    cache.insert(path.to_path_buf(), (mtime, len, entry.clone()));
    Some(entry)
}

fn is_inknote_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("inknote"))
}

fn count_visible_children(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let mut count = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(fname) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if fname.starts_with('.') {
            continue;
        }
        if path.is_dir() || (path.is_file() && is_inknote_file(&path)) {
            count += 1;
        }
    }
    count
}

fn metadata_modified_secs(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn clean_display_title(raw: &str, fallback: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Chuẩn hoá tên thư mục/file an toàn trên Windows (giữ nguyên đầy đủ dấu tiếng Việt).
pub fn sanitize_fs_name(raw: &str, fallback: &str) -> String {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
            out.push('-');
        } else {
            out.push(ch);
        }
    }
    let cleaned = out.trim_matches(|c: char| c == ' ' || c == '.').to_string();
    if cleaned.is_empty() {
        return fallback.to_string();
    }
    let upper = cleaned.to_ascii_uppercase();
    let is_windows_reserved = matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );
    if is_windows_reserved {
        format!("{cleaned}_note")
    } else {
        cleaned
    }
}

fn unique_folder_path(parent: &Path, base: &str) -> PathBuf {
    let candidate = parent.join(base);
    if !candidate.exists() {
        return candidate;
    }
    let mut idx = 2usize;
    loop {
        let next = parent.join(format!("{base} ({idx})"));
        if !next.exists() {
            return next;
        }
        idx += 1;
    }
}

fn unique_note_path(parent: &Path, base: &str) -> PathBuf {
    let candidate = parent.join(format!("{base}.inknote"));
    if !candidate.exists() {
        return candidate;
    }
    let mut idx = 2usize;
    loop {
        let next = parent.join(format!("{base} ({idx}).inknote"));
        if !next.exists() {
            return next;
        }
        idx += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Stroke};
    use inknote_pen::{PenPath, Style, penpath::Element};
    use parry2d_f64::math::Vector2;

    struct TempDirGuard(PathBuf);

    impl TempDirGuard {
        fn new(prefix: &str) -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let dir = std::env::temp_dir().join(format!("{prefix}_{unique}"));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn sanitize_keeps_vietnamese_and_strips_forbidden_chars() {
        assert_eq!(
            sanitize_fs_name("Toán Giải Tích: ắ ằ ẳ ẵ ặ / ơ ư?", "Note"),
            "Toán Giải Tích- ắ ằ ẳ ẵ ặ - ơ ư-"
        );
        assert_eq!(sanitize_fs_name("   ...  ", "Fallback"), "Fallback");
        assert_eq!(sanitize_fs_name("CON", "Fallback"), "CON_note");
    }

    #[test]
    fn create_folders_notes_and_search_recursively() {
        let guard = TempDirGuard::new("inknote_lib_test");
        let store = LibraryStore::new(guard.0.clone());
        store.ensure_dirs().unwrap();

        let f1 = store.create_folder(&[], "Toán cao cấp").unwrap();
        assert_eq!(f1, "Toán cao cấp");
        // Duplicate folder name auto-increments
        let f1_dup = store.create_folder(&[], "Toán cao cấp").unwrap();
        assert_eq!(f1_dup, "Toán cao cấp (2)");

        let paper = PaperSettings::default();
        let (n1_path, mut n1_doc) = store
            .create_note_file(&[], "Ghi chú Đại số", &paper)
            .unwrap();
        let sid = n1_doc.alloc_id();
        n1_doc.execute(Command::AddStroke(Stroke {
            id: sid,
            path: PenPath::new(Element::new(Vector2::new(20.0, 20.0), 0.5)),
            style: Style::default(),
        }));
        n1_doc.to_file().save_to_path(&n1_path).unwrap();

        store
            .create_note_file(&[f1.clone()], "Giải tích chương 1", &paper)
            .unwrap();

        // Ignored non-inknote temp files
        fs::write(store.documents_dir().join(".hidden.tmp"), b"tmp").unwrap();

        let mut cache = HashMap::new();
        let root_items = store
            .scan_items(LibrarySection::Documents, &[], "", &mut cache)
            .unwrap();
        // 2 folders + 1 root note
        assert_eq!(root_items.len(), 3);
        assert!(root_items[0].is_folder());
        assert!(root_items[1].is_folder());
        assert!(!root_items[2].is_folder());
        if let LibraryItem::Note(n) = &root_items[2] {
            assert_eq!(n.title, "Ghi chú Đại số");
            assert_eq!(n.first_page.as_ref().unwrap().strokes.len(), 1);
        } else {
            panic!("expected note");
        }

        // Recursive search matches Vietnamese note inside subfolder
        let found = store
            .scan_items(LibrarySection::Documents, &[], "giải tích", &mut cache)
            .unwrap();
        assert_eq!(found.len(), 1);
        match &found[0] {
            LibraryItem::Note(n) => {
                assert_eq!(n.title, "Giải tích chương 1");
                assert_eq!(n.rel_folder, vec!["Toán cao cấp".to_string()]);
            }
            _ => panic!("expected matching note"),
        }

        // Trash starts empty
        let trash_items = store
            .scan_items(LibrarySection::Trash, &[], "", &mut cache)
            .unwrap();
        assert!(trash_items.is_empty());
    }

    #[test]
    fn card_actions_and_trash_restore_delete_empty_roundtrip() {
        let guard = TempDirGuard::new("inknote_lib_step72");
        let store = LibraryStore::new(guard.0.clone());
        store.ensure_dirs().unwrap();
        let paper = PaperSettings::default();

        let sub = store.create_folder(&[], "Học tập").unwrap();
        let (note_path, _doc) = store
            .create_note_file(&[sub.clone()], "Đại số tuyến tính", &paper)
            .unwrap();

        // Rename note (title & file stem update, Vietnamese kept)
        let renamed_path = store.rename_note(&note_path, "Giải tích nâng cao").unwrap();
        assert!(!note_path.exists());
        assert!(renamed_path.exists());
        let loaded = DocumentFile::load_from_path(&renamed_path).unwrap();
        assert_eq!(loaded.metadata.title, "Giải tích nâng cao");

        // Duplicate note
        let dup_path = store.duplicate_note(&renamed_path).unwrap();
        assert!(dup_path.exists());
        assert_ne!(dup_path, renamed_path);
        let dup_loaded = DocumentFile::load_from_path(&dup_path).unwrap();
        assert_eq!(dup_loaded.metadata.title, "Giải tích nâng cao Copy");

        // Toggle favorite
        assert!(store.toggle_favorite_note(&renamed_path).unwrap());
        assert!(!store.toggle_favorite_note(&renamed_path).unwrap());
        assert!(store.toggle_favorite_note(&renamed_path).unwrap());

        // Move note in subfolder -> Trash -> check Trash -> Restore back to subfolder
        let trashed_note = store
            .move_to_trash(&renamed_path, std::slice::from_ref(&sub))
            .unwrap();
        assert!(!renamed_path.exists());
        assert!(trashed_note.exists());
        assert_eq!(store.trash_item_count(), 1);

        let mut cache = HashMap::new();
        let trash_list = store
            .scan_items(LibrarySection::Trash, &[], "", &mut cache)
            .unwrap();
        assert_eq!(trash_list.len(), 1);
        assert_eq!(trash_list[0].name(), "Giải tích nâng cao");

        let restored_path = store.restore_from_trash(&trashed_note).unwrap();
        assert!(!trashed_note.exists());
        assert!(restored_path.exists());
        assert_eq!(
            restored_path.parent().unwrap(),
            store.resolve_folder(LibrarySection::Documents, std::slice::from_ref(&sub))
        );
        assert_eq!(store.trash_item_count(), 0);

        // Move folder + note -> Trash -> Delete Permanently & Empty Trash
        let folder_path =
            store.resolve_folder(LibrarySection::Documents, std::slice::from_ref(&sub));
        let renamed_folder = store.rename_folder(&folder_path, "Toán 12").unwrap();
        assert!(renamed_folder.ends_with("Toán 12"));

        let trashed_folder = store.move_to_trash(&renamed_folder, &[]).unwrap();
        assert!(!renamed_folder.exists());
        assert_eq!(store.trash_item_count(), 1);

        store.delete_permanently(&trashed_folder).unwrap();
        assert!(!trashed_folder.exists());
        assert_eq!(store.trash_item_count(), 0);

        // Create two more notes, trash them, and Empty Trash
        let (a_path, _) = store.create_note_file(&[], "Temp A", &paper).unwrap();
        let (b_path, _) = store.create_note_file(&[], "Temp B", &paper).unwrap();
        store.move_to_trash(&a_path, &[]).unwrap();
        store.move_to_trash(&b_path, &[]).unwrap();
        assert_eq!(store.trash_item_count(), 2);

        store.empty_trash().unwrap();
        assert_eq!(store.trash_item_count(), 0);
    }
}
