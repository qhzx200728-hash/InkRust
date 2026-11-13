// Màn hình Thư viện (Library – Bước 7.1 - P3, khớp docs/04-ui-spec.md mục 3).
// Bố cục 2 phần: Sidebar trái (~18%: Logo InkNote, Documents, Trash) + Nội dung chính
// (Thanh trên: nút quay lại, tiêu đề/breadcrumb, + New Folder/+ New Note, ô tìm kiếm có nút X,
//  bánh răng Cài đặt; Lưới Grid View ~5 cột hiển thị chung thư mục & ghi chú kèm thumbnail trang đầu).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Layout, Margin, PopupCloseBehavior, Pos2,
    Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};
use inknote_core::library::{
    LibraryFolderEntry, LibraryItem, LibraryNoteEntry, LibrarySection, LibraryStore,
};
use inknote_core::{CanvasMode, DocumentPage, PaperPattern, PaperSettings, PaperSize};
use inknote_pen::style::Composer;
use inknote_render::CairoRenderContext;

use crate::{
    icons::{ToolIcon, draw_icon, draw_library_brand_logo, draw_library_folder_art},
    image_box::ImageTextureCache,
    svg_cache::SvgTextureCache,
    theme::Theme,
    thumbnails::{append_batches_to_mesh, paint_page_thumbnail_preview},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryAction {
    None,
    OpenNote(PathBuf),
    CreateNote {
        title: String,
        rel_folder: Vec<String>,
    },
    CreateFolder {
        name: String,
        rel_folder: Vec<String>,
    },
    DefaultPaperChanged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreateDialogKind {
    Folder,
    Note,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameDialogMode {
    CreateFolder,
    CreateNote,
    RenameFolder { path: PathBuf },
    RenameNote { path: PathBuf },
}

#[derive(Clone, Debug)]
pub struct CreateModalState {
    pub mode: NameDialogMode,
    pub name: String,
    pub needs_focus: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
struct CardMenuState {
    path: PathBuf,
    display_name: String,
    rel_folder: Vec<String>,
    is_folder: bool,
    is_favorite: bool,
    in_trash: bool,
    anchor: Pos2,
    opened_this_frame: bool,
}

#[derive(Clone, Debug)]
enum TrashConfirmModal {
    DeleteForever {
        path: PathBuf,
        name: String,
        is_folder: bool,
    },
    EmptyTrash {
        count: usize,
    },
}

/// Kết quả tính toán bố cục lưới thẻ Library (`(cols, card_w, col_gap)`).
pub fn compute_grid_layout(avail_w: f32) -> (usize, f32, f32) {
    let col_gap = 24.0;
    let target_min_card = 152.0;
    let cols = (((avail_w + col_gap) / (target_min_card + col_gap)).floor() as usize).clamp(2, 5);
    let card_w = ((avail_w - col_gap * ((cols - 1) as f32)) / (cols as f32)).max(110.0);
    (cols, card_w, col_gap)
}

/// Trạng thái & bộ nhớ đệm hiển thị cho màn hình Library.
pub struct LibraryState {
    pub store: LibraryStore,
    pub section: LibrarySection,
    pub rel_folder: Vec<String>,
    pub search_query: String,
    last_scanned_query: String,
    cached_items: Vec<LibraryItem>,
    note_cache: HashMap<PathBuf, (u64, u64, LibraryNoteEntry)>,
    thumb_meshes: HashMap<(PathBuf, u64, u32), Arc<egui::epaint::Mesh>>,
    svg_cache: SvgTextureCache,
    image_cache: ImageTextureCache,
    needs_refresh: bool,
    pub last_error: Option<String>,
    pub create_dialog: Option<CreateModalState>,
    card_menu: Option<CardMenuState>,
    confirm_trash: Option<TrashConfirmModal>,
    focus_search_requested: bool,
    pub sidebar_width_last: f32,
    pub grid_cols_last: usize,
}

impl LibraryState {
    pub fn new(store: LibraryStore) -> Self {
        Self {
            store,
            section: LibrarySection::Documents,
            rel_folder: Vec::new(),
            search_query: String::new(),
            last_scanned_query: String::new(),
            cached_items: Vec::new(),
            note_cache: HashMap::new(),
            thumb_meshes: HashMap::new(),
            svg_cache: SvgTextureCache::default(),
            image_cache: ImageTextureCache::default(),
            needs_refresh: true,
            last_error: None,
            create_dialog: None,
            card_menu: None,
            confirm_trash: None,
            focus_search_requested: false,
            sidebar_width_last: 216.0,
            grid_cols_last: 5,
        }
    }

    pub fn request_refresh(&mut self) {
        self.needs_refresh = true;
    }

    pub fn invalidate_note_path(&mut self, path: &Path) {
        self.note_cache.remove(path);
        self.thumb_meshes.retain(|(p, _, _), _| p != path);
        self.needs_refresh = true;
    }

    #[cfg(test)]
    pub fn items(&self) -> &[LibraryItem] {
        &self.cached_items
    }

    pub fn open_create_dialog(&mut self, kind: CreateDialogKind) {
        let (mode, default_name) = match kind {
            CreateDialogKind::Folder => (
                NameDialogMode::CreateFolder,
                self.next_suggested_folder_name(),
            ),
            CreateDialogKind::Note => (NameDialogMode::CreateNote, self.next_suggested_note_name()),
        };
        self.card_menu = None;
        self.create_dialog = Some(CreateModalState {
            mode,
            name: default_name,
            needs_focus: true,
            error: None,
        });
    }

    pub fn open_rename_dialog(&mut self, path: PathBuf, current_name: &str, is_folder: bool) {
        let mode = if is_folder {
            NameDialogMode::RenameFolder { path }
        } else {
            NameDialogMode::RenameNote { path }
        };
        self.card_menu = None;
        self.create_dialog = Some(CreateModalState {
            mode,
            name: current_name.to_string(),
            needs_focus: true,
            error: None,
        });
    }

    fn next_suggested_folder_name(&self) -> String {
        let count = self.cached_items.iter().filter(|i| i.is_folder()).count();
        if count == 0 {
            "New Folder".to_string()
        } else {
            format!("Folder {}", count + 1)
        }
    }

    fn next_suggested_note_name(&self) -> String {
        let count = self.cached_items.iter().filter(|i| !i.is_folder()).count();
        if count == 0 {
            "Untitled Note".to_string()
        } else {
            format!("Note {}", count + 1)
        }
    }

    pub fn refresh_now(&mut self) {
        self.needs_refresh = false;
        self.last_scanned_query = self.search_query.clone();
        match self.store.scan_items(
            self.section,
            &self.rel_folder,
            &self.search_query,
            &mut self.note_cache,
        ) {
            Ok(items) => {
                self.cached_items = items;
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to load library: {e}"));
            }
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        ui: &mut Ui,
        theme: &Theme,
        default_paper: &mut PaperSettings,
        render_cx: &mut CairoRenderContext,
    ) -> LibraryAction {
        let mut action = LibraryAction::None;

        if self.search_query != self.last_scanned_query {
            self.needs_refresh = true;
        }
        if self.needs_refresh {
            self.refresh_now();
        }

        // Phím tắt tiện dụng trong Library: Ctrl+F focus ô tìm kiếm; Alt+Left / Backspace quay lại thư mục cha
        if self.create_dialog.is_none() && self.confirm_trash.is_none() {
            let text_focused = ctx.text_edit_focused();
            let (focus_search, go_up) = ctx.input(|i| {
                let focus_f =
                    (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::F);
                let up = !text_focused
                    && (i.key_pressed(egui::Key::Backspace)
                        || (i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft)));
                (focus_f, up)
            });
            if focus_search {
                self.focus_search_requested = true;
            }
            if go_up && self.section == LibrarySection::Documents && !self.rel_folder.is_empty() {
                self.rel_folder.pop();
                self.search_query.clear();
                self.card_menu = None;
                self.needs_refresh = true;
            }
        }

        if self.needs_refresh {
            self.refresh_now();
        }

        // 1. SIDEBAR TRÁI (~18% bề rộng cửa sổ, cố định nền xám rất nhạt)
        let screen_w = ui.max_rect().width().max(640.0);
        let sidebar_w = (screen_w * 0.18).clamp(185.0, 245.0);
        self.sidebar_width_last = sidebar_w;

        let sidebar_frame = egui::Frame::new()
            .fill(theme.lib_sidebar_bg)
            .stroke(Stroke::new(1.0, theme.lib_sidebar_stroke))
            .inner_margin(Margin {
                left: 14,
                right: 14,
                top: 18,
                bottom: 16,
            });

        egui::Panel::left("library_sidebar")
            .resizable(false)
            .show_separator_line(false)
            .exact_size(sidebar_w)
            .frame(sidebar_frame)
            .show(ui, |ui| {
                self.show_sidebar(ui, theme);
            });

        // 2. NỘI DUNG CHÍNH BÊN PHẢI (nền trắng, nhiều khoảng trắng)
        let main_frame = egui::Frame::new()
            .fill(theme.lib_main_bg)
            .inner_margin(Margin::ZERO);

        egui::CentralPanel::default()
            .frame(main_frame)
            .show(ui, |ui| {
                self.show_top_bar(ui, theme, default_paper, &mut action);
                if self.needs_refresh {
                    self.refresh_now();
                }
                self.show_grid(ui, theme, render_cx, &mut action);
            });

        // 3. Menu ngữ cảnh / dropdown trên thẻ (Đổi tên, Nhân bản, Yêu thích, Xoá / Khôi phục)
        self.show_card_menu(ctx, theme);

        // 4. Hộp thoại tạo / đổi tên thư mục & ghi chú (hỗ trợ tiếng Việt đủ dấu & Enter)
        if let Some(dialog_action) = self.show_create_dialog(ctx, theme) {
            action = dialog_action;
        }

        // 5. Hộp thoại xác nhận trước khi xoá vĩnh viễn / dọn sạch Thùng rác
        self.show_trash_confirm_dialog(ctx, theme);

        if self.needs_refresh {
            self.refresh_now();
        }

        action
    }

    fn show_sidebar(&mut self, ui: &mut Ui, theme: &Theme) {
        // Logo + Tên InkNote tự vẽ ở đầu Sidebar
        let (logo_row_rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
        let icon_rect = Rect::from_min_size(logo_row_rect.min + vec2(2.0, 3.0), vec2(28.0, 28.0));
        draw_library_brand_logo(ui.painter(), icon_rect, theme.nav_bar_bg);
        ui.painter().text(
            pos2(icon_rect.right() + 10.0, logo_row_rect.center().y - 1.0),
            Align2::LEFT_CENTER,
            "InkNote",
            FontId::proportional(18.5),
            theme.lib_title_fg,
        );

        ui.add_space(22.0);

        // Hai mục điều hướng chính: Documents & Trash (cách đều nhau)
        if sidebar_nav_row(
            ui,
            theme,
            ToolIcon::Documents,
            "Documents",
            self.section == LibrarySection::Documents,
        )
        .clicked()
        {
            if self.section != LibrarySection::Documents {
                self.section = LibrarySection::Documents;
                self.search_query.clear();
                self.card_menu = None;
                self.needs_refresh = true;
            }
        }

        ui.add_space(6.0);

        let trash_count = self.store.trash_item_count();
        let trash_label = if trash_count > 0 {
            format!("Trash ({trash_count})")
        } else {
            "Trash".to_string()
        };
        if sidebar_nav_row(
            ui,
            theme,
            ToolIcon::Trash,
            &trash_label,
            self.section == LibrarySection::Trash,
        )
        .clicked()
        {
            if self.section != LibrarySection::Trash {
                self.section = LibrarySection::Trash;
                self.search_query.clear();
                self.card_menu = None;
                self.needs_refresh = true;
            }
        }
    }

    fn show_top_bar(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        default_paper: &mut PaperSettings,
        action: &mut LibraryAction,
    ) {
        let bar_frame = egui::Frame::new()
            .fill(theme.lib_main_bg)
            .inner_margin(Margin {
                left: 28,
                right: 28,
                top: 18,
                bottom: 12,
            });
        bar_frame.show(ui, |ui| {
            let row_h = 34.0;
            let (row_rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::hover());

            // Nhóm bên trái: nút quay lại (khi đang ở thư mục con) + tiêu đề khu vực / breadcrumb
            let mut left_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("library_top_left")
                    .max_rect(row_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            left_ui.spacing_mut().item_spacing.x = 8.0;

            let in_subfolder =
                self.section == LibrarySection::Documents && !self.rel_folder.is_empty();
            if in_subfolder {
                let (back_rect, back_resp) =
                    left_ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
                if back_resp.hovered() {
                    left_ui.painter().rect_filled(
                        back_rect,
                        CornerRadius::same(8),
                        theme.lib_search_bg,
                    );
                }
                draw_icon(
                    left_ui.painter(),
                    back_rect.shrink(5.0),
                    ToolIcon::BackArrow,
                    theme.lib_title_fg,
                );
                if back_resp.on_hover_text("Back to parent folder").clicked() {
                    self.rel_folder.pop();
                    self.search_query.clear();
                    self.needs_refresh = true;
                }
            }

            let area_title = match self.section {
                LibrarySection::Trash => "Trash".to_string(),
                LibrarySection::Documents => self
                    .rel_folder
                    .last()
                    .cloned()
                    .unwrap_or_else(|| "Documents".to_string()),
            };
            left_ui.label(
                RichText::new(area_title)
                    .size(21.0)
                    .strong()
                    .color(theme.lib_title_fg),
            );

            if in_subfolder {
                left_ui.add_space(6.0);
                if left_ui
                    .add(
                        egui::Button::new(
                            RichText::new("Documents")
                                .size(theme.font_size_small)
                                .color(theme.lib_muted_fg),
                        )
                        .frame(false),
                    )
                    .clicked()
                {
                    self.rel_folder.clear();
                    self.search_query.clear();
                    self.needs_refresh = true;
                }
                for depth in 0..self.rel_folder.len().saturating_sub(1) {
                    left_ui.label(
                        RichText::new("/")
                            .size(theme.font_size_small)
                            .color(theme.lib_muted_fg),
                    );
                    let seg_name = self.rel_folder[depth].clone();
                    if left_ui
                        .add(
                            egui::Button::new(
                                RichText::new(&seg_name)
                                    .size(theme.font_size_small)
                                    .color(theme.lib_muted_fg),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        self.rel_folder.truncate(depth + 1);
                        self.search_query.clear();
                        self.needs_refresh = true;
                    }
                }
            }

            // Nhóm bên phải: Bánh răng Cài đặt -> Ô tìm kiếm có nút X -> Nút tạo mới
            let mut right_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("library_top_right")
                    .max_rect(row_rect)
                    .layout(Layout::right_to_left(Align::Center)),
            );
            right_ui.spacing_mut().item_spacing.x = 10.0;

            // 1. Bánh răng Cài đặt ngoài ô tìm kiếm, bên phải
            let (gear_rect, gear_resp) =
                right_ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
            if gear_resp.hovered() {
                right_ui.painter().rect_filled(
                    gear_rect,
                    CornerRadius::same(8),
                    theme.lib_search_bg,
                );
            }
            draw_icon(
                right_ui.painter(),
                gear_rect.shrink(6.0),
                ToolIcon::Settings,
                theme.lib_sidebar_fg,
            );
            let gear_hover = gear_resp.on_hover_text("Library Settings");
            egui::Popup::from_toggle_button_response(&gear_hover)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .frame(theme.dark_popup_frame(right_ui.style()))
                .show(|ui| {
                    ui.set_min_width(270.0);
                    ui.label(
                        RichText::new("Library Settings")
                            .size(theme.font_size_title)
                            .strong()
                            .color(theme.popup_header_fg),
                    );
                    ui.add_space(6.0);
                    egui::Frame::new()
                        .fill(theme.popup_section_bg)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(8))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("Storage Folder")
                                    .size(theme.font_size_small)
                                    .color(theme.tool_text_muted),
                            );
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(self.store.root_dir().to_string_lossy().as_ref())
                                    .size(theme.font_size_small)
                                    .color(theme.tool_text_fg),
                            );
                        });
                    ui.add_space(8.0);
                    egui::Frame::new()
                        .fill(theme.popup_section_bg)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(8))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("Default Template for New Notes")
                                    .size(theme.font_size_small)
                                    .color(theme.tool_text_muted),
                            );
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                for (label, pat) in [
                                    ("Plain", PaperPattern::Plain),
                                    ("Dots", PaperPattern::Dotted),
                                    ("Grid", PaperPattern::Grid),
                                    ("Lines", PaperPattern::Lined),
                                ] {
                                    if ui
                                        .selectable_label(default_paper.pattern == pat, label)
                                        .clicked()
                                    {
                                        default_paper.pattern = pat;
                                        *action = LibraryAction::DefaultPaperChanged;
                                    }
                                }
                            });
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                for (label, size) in [("A4", PaperSize::A4), ("A5", PaperSize::A5)]
                                {
                                    if ui
                                        .selectable_label(default_paper.paper_size == size, label)
                                        .clicked()
                                    {
                                        default_paper.paper_size = size;
                                        default_paper.mode = CanvasMode::Paper;
                                        let (w, h) = size.dimensions(default_paper.orientation);
                                        default_paper.page_width = w;
                                        default_paper.page_height = h;
                                        *action = LibraryAction::DefaultPaperChanged;
                                    }
                                }
                            });
                        });
                });

            // 2. Ô tìm kiếm theo tên: chữ nhật bo tròn, có nút X xoá từ khoá
            let search_w = 230.0;
            let search_rect = right_ui
                .allocate_exact_size(vec2(search_w, 32.0), Sense::hover())
                .0;
            right_ui.painter().rect_filled(
                search_rect,
                CornerRadius::same(16),
                theme.lib_search_bg,
            );
            right_ui.painter().rect_stroke(
                search_rect,
                CornerRadius::same(16),
                Stroke::new(1.0, theme.lib_search_stroke),
                StrokeKind::Inside,
            );

            let glass_rect = Rect::from_min_size(
                pos2(search_rect.left() + 8.0, search_rect.center().y - 9.0),
                vec2(18.0, 18.0),
            );
            draw_icon(
                right_ui.painter(),
                glass_rect,
                ToolIcon::Search,
                theme.lib_muted_fg,
            );

            let has_query = !self.search_query.is_empty();
            let clear_w = if has_query { 24.0 } else { 8.0 };
            let input_rect = Rect::from_min_max(
                pos2(glass_rect.right() + 4.0, search_rect.top() + 3.0),
                pos2(
                    search_rect.right() - clear_w - 2.0,
                    search_rect.bottom() - 3.0,
                ),
            );

            let mut search_input_ui = right_ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("library_search_child")
                    .max_rect(input_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            let search_edit = egui::TextEdit::singleline(&mut self.search_query)
                .id_salt("library_search_input")
                .frame(egui::Frame::NONE)
                .desired_width(input_rect.width())
                .hint_text("Search by name...")
                .text_color(theme.lib_title_fg)
                .show(&mut search_input_ui);
            if self.focus_search_requested {
                search_edit.response.request_focus();
                self.focus_search_requested = false;
            }
            if search_edit.response.changed() {
                self.needs_refresh = true;
            }

            if has_query {
                let clear_rect = Rect::from_center_size(
                    pos2(search_rect.right() - 14.0, search_rect.center().y),
                    vec2(20.0, 20.0),
                );
                let clear_resp = right_ui.interact(
                    clear_rect,
                    right_ui.scope_id().with("library_search_clear_x"),
                    Sense::click(),
                );
                if clear_resp.hovered() {
                    right_ui.painter().circle_filled(
                        clear_rect.center(),
                        9.0,
                        Color32::from_rgb(218, 223, 232),
                    );
                }
                draw_small_x(right_ui.painter(), clear_rect.center(), theme.lib_muted_fg);
                if clear_resp
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Clear search")
                    .clicked()
                {
                    self.search_query.clear();
                    self.needs_refresh = true;
                }
            }

            // 3. Các nút tạo Ghi chú mới & Thư mục mới (Documents) hoặc nút Empty Trash (Trash)
            if self.section == LibrarySection::Documents {
                let new_note_btn = pill_action_button(
                    &mut right_ui,
                    theme,
                    "lib_new_note",
                    ToolIcon::NotePlus,
                    "New Note",
                    true,
                );
                if new_note_btn.clicked() {
                    self.open_create_dialog(CreateDialogKind::Note);
                }

                let new_folder_btn = pill_action_button(
                    &mut right_ui,
                    theme,
                    "lib_new_folder",
                    ToolIcon::FolderPlus,
                    "New Folder",
                    false,
                );
                if new_folder_btn.clicked() {
                    self.open_create_dialog(CreateDialogKind::Folder);
                }
            } else {
                let trash_count = self.store.trash_item_count();
                let empty_btn = empty_trash_pill_button(&mut right_ui, theme, trash_count);
                if empty_btn.clicked() && trash_count > 0 {
                    self.card_menu = None;
                    self.confirm_trash = Some(TrashConfirmModal::EmptyTrash { count: trash_count });
                }
            }
        });
    }

    fn show_grid(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        render_cx: &mut CairoRenderContext,
        action: &mut LibraryAction,
    ) {
        if let Some(err) = &self.last_error {
            ui.horizontal(|ui| {
                ui.add_space(28.0);
                ui.label(
                    RichText::new(err)
                        .size(theme.font_size_small)
                        .color(Color32::from_rgb(217, 48, 37)),
                );
            });
        }

        if self.cached_items.is_empty() {
            let full_rect = ui.available_rect_before_wrap();
            let center = full_rect.center();
            let empty_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("library_empty")
                    .max_rect(full_rect),
            );
            let (title, subtitle) = if !self.search_query.trim().is_empty() {
                (
                    format!("No items matching \"{}\"", self.search_query.trim()),
                    "Try another name or click X to clear the search filter.".to_string(),
                )
            } else if self.section == LibrarySection::Trash {
                (
                    "Trash is empty".to_string(),
                    "Deleted notes will appear here.".to_string(),
                )
            } else {
                (
                    "No notes or folders yet".to_string(),
                    "Use + New Note or + New Folder above to get started.".to_string(),
                )
            };
            empty_ui.painter().text(
                pos2(center.x, center.y - 20.0),
                Align2::CENTER_CENTER,
                title,
                FontId::proportional(16.0),
                theme.lib_title_fg,
            );
            empty_ui.painter().text(
                pos2(center.x, center.y + 6.0),
                Align2::CENTER_CENTER,
                subtitle,
                FontId::proportional(theme.font_size_body),
                theme.lib_muted_fg,
            );
            if !self.search_query.trim().is_empty() {
                let btn_rect =
                    Rect::from_center_size(pos2(center.x, center.y + 40.0), vec2(118.0, 30.0));
                let resp = empty_ui.interact(
                    btn_rect,
                    empty_ui.scope_id().with("empty_clear_search"),
                    Sense::click(),
                );
                let bg = if resp.hovered() {
                    Color32::from_rgb(228, 234, 244)
                } else {
                    theme.lib_search_bg
                };
                empty_ui
                    .painter()
                    .rect_filled(btn_rect, CornerRadius::same(8), bg);
                empty_ui.painter().text(
                    btn_rect.center(),
                    Align2::CENTER_CENTER,
                    "Clear search",
                    FontId::proportional(theme.font_size_body),
                    theme.lib_sidebar_active_fg,
                );
                if resp.clicked() {
                    self.search_query.clear();
                    self.needs_refresh = true;
                }
            }
            return;
        }

        let items = self.cached_items.clone();
        let in_search = !self.search_query.trim().is_empty();
        let is_documents = self.section == LibrarySection::Documents;
        let mut clicked_folder: Option<LibraryFolderEntry> = None;
        let mut clicked_note: Option<LibraryNoteEntry> = None;
        let mut open_menu_req: Option<CardMenuState> = None;

        ScrollArea::vertical()
            .id_salt("library_grid_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let side_pad = 28.0;
                let scroll_reserve = 14.0;
                let inner_w = (ui.available_width() - side_pad * 2.0 - scroll_reserve).max(240.0);
                let (cols, card_w, col_gap) = compute_grid_layout(inner_w);
                self.grid_cols_last = cols;

                let slot_h = (card_w * 1.24).round();
                let label_h = 44.0;
                let row_gap = 22.0;

                ui.add_space(10.0);
                for row_start in (0..items.len()).step_by(cols) {
                    ui.horizontal(|ui| {
                        ui.add_space(side_pad);
                        for col in 0..cols {
                            let idx = row_start + col;
                            if idx >= items.len() {
                                break;
                            }
                            if col > 0 {
                                ui.add_space(col_gap);
                            }
                            let item = &items[idx];
                            let card_total_h = slot_h + 8.0 + label_h;
                            let card_rect = ui.allocate_space(vec2(card_w, card_total_h)).1;
                            let card_id = match item {
                                LibraryItem::Folder(f) => {
                                    ui.scope_id().with(("lib_folder", &f.path))
                                }
                                LibraryItem::Note(n) => ui.scope_id().with(("lib_note", &n.path)),
                            };
                            let resp = ui.interact(card_rect, card_id, Sense::click());

                            let mut menu_clicked = false;
                            let mut chevron_rect_opt: Option<Rect> = None;

                            if ui.is_rect_visible(card_rect) {
                                let menu_active_for_card =
                                    self.card_menu.as_ref().is_some_and(|m| {
                                        m.path
                                            == match item {
                                                LibraryItem::Folder(f) => f.path.as_path(),
                                                LibraryItem::Note(n) => n.path.as_path(),
                                            }
                                    });
                                if resp.hovered() || menu_active_for_card {
                                    ui.painter().rect_filled(
                                        card_rect.expand(6.0),
                                        CornerRadius::same(theme.lib_card_rounding),
                                        theme.lib_card_hover_bg,
                                    );
                                }

                                let slot_rect =
                                    Rect::from_min_size(card_rect.min, vec2(card_w, slot_h));
                                let chevron_rect = match item {
                                    LibraryItem::Folder(folder) => {
                                        draw_library_folder_art(ui.painter(), slot_rect, theme);
                                        let sub = if in_search && !folder.rel_folder.is_empty() {
                                            format!(
                                                "{} · {}",
                                                folder.rel_folder.join("/"),
                                                format_item_count(folder.item_count)
                                            )
                                        } else {
                                            format_item_count(folder.item_count)
                                        };
                                        paint_card_caption_with_menu(
                                            ui.painter(),
                                            slot_rect,
                                            card_w,
                                            &folder.name,
                                            &sub,
                                            theme,
                                        )
                                    }
                                    LibraryItem::Note(note) => {
                                        self.paint_note_card_thumbnail(
                                            ui, slot_rect, note, theme, render_cx,
                                        );
                                        let mut meta_parts =
                                            vec![format_relative_time(note.modified_at)];
                                        if note.page_count > 1 {
                                            meta_parts.push(format!("{} pages", note.page_count));
                                        }
                                        if in_search && !note.rel_folder.is_empty() {
                                            meta_parts.push(note.rel_folder.join("/"));
                                        }
                                        paint_card_caption_with_menu(
                                            ui.painter(),
                                            slot_rect,
                                            card_w,
                                            &note.title,
                                            &meta_parts.join(" · "),
                                            theme,
                                        )
                                    }
                                };

                                chevron_rect_opt = Some(chevron_rect);
                                let menu_resp = ui.interact(
                                    chevron_rect,
                                    card_id.with("menu_btn"),
                                    Sense::click(),
                                );
                                if menu_resp.hovered() || menu_active_for_card {
                                    ui.painter().rect_filled(
                                        chevron_rect,
                                        CornerRadius::same(5),
                                        Color32::from_rgba_unmultiplied(24, 90, 188, 28),
                                    );
                                }
                                draw_icon(
                                    ui.painter(),
                                    chevron_rect.shrink(3.5),
                                    ToolIcon::ChevronDown,
                                    if menu_resp.hovered() || menu_active_for_card {
                                        theme.lib_sidebar_active_fg
                                    } else {
                                        theme.lib_muted_fg
                                    },
                                );
                                if menu_resp
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .on_hover_text("More actions")
                                    .clicked()
                                {
                                    menu_clicked = true;
                                }
                            }

                            let right_clicked = resp.secondary_clicked();
                            let card_body_clicked = resp.clicked() && !menu_clicked;

                            if menu_clicked || right_clicked || (card_body_clicked && !is_documents)
                            {
                                let default_anchor = chevron_rect_opt
                                    .map(|r| pos2(r.left() - 60.0, r.bottom() + 4.0))
                                    .unwrap_or_else(|| {
                                        pos2(card_rect.center().x - 75.0, card_rect.bottom())
                                    });
                                let anchor = if right_clicked {
                                    resp.interact_pointer_pos().unwrap_or(default_anchor)
                                } else {
                                    default_anchor
                                };
                                open_menu_req = Some(match item {
                                    LibraryItem::Folder(f) => CardMenuState {
                                        path: f.path.clone(),
                                        display_name: f.name.clone(),
                                        rel_folder: f.rel_folder.clone(),
                                        is_folder: true,
                                        is_favorite: false,
                                        in_trash: !is_documents,
                                        anchor,
                                        opened_this_frame: true,
                                    },
                                    LibraryItem::Note(n) => CardMenuState {
                                        path: n.path.clone(),
                                        display_name: n.title.clone(),
                                        rel_folder: n.rel_folder.clone(),
                                        is_folder: false,
                                        is_favorite: n.is_favorite,
                                        in_trash: !is_documents,
                                        anchor,
                                        opened_this_frame: true,
                                    },
                                });
                            } else if card_body_clicked && is_documents {
                                match item {
                                    LibraryItem::Folder(folder) => {
                                        clicked_folder = Some(folder.clone());
                                    }
                                    LibraryItem::Note(note) => {
                                        clicked_note = Some(note.clone());
                                    }
                                }
                            }
                        }
                    });
                    ui.add_space(row_gap);
                }
                ui.add_space(18.0);
            });

        if let Some(menu) = open_menu_req {
            self.card_menu = Some(menu);
        } else if let Some(folder) = clicked_folder {
            let mut next = folder.rel_folder;
            next.push(folder.name);
            self.rel_folder = next;
            self.search_query.clear();
            self.card_menu = None;
            self.refresh_now();
        } else if let Some(note) = clicked_note {
            self.card_menu = None;
            *action = LibraryAction::OpenNote(note.path);
        }
    }

    fn paint_note_card_thumbnail(
        &mut self,
        ui: &mut Ui,
        slot_rect: Rect,
        note: &LibraryNoteEntry,
        theme: &Theme,
        render_cx: &mut CairoRenderContext,
    ) {
        let aspect = (note.paper_settings.page_height / note.paper_settings.page_width.max(1.0))
            .clamp(0.45, 2.0);
        let mut pw = slot_rect.width();
        let mut ph = pw * aspect;
        if ph > slot_rect.height() {
            ph = slot_rect.height();
            pw = ph / aspect;
        }
        // Căn giữa ngang và tựa đáy vào slot để trang dọc lẫn trang ngang đều thẳng hàng gọn gàng
        let paper_card_rect = Rect::from_min_size(
            pos2(slot_rect.center().x - pw * 0.5, slot_rect.bottom() - ph),
            vec2(pw, ph),
        );

        ui.painter().rect_filled(
            paper_card_rect.translate(vec2(0.0, 2.5)),
            CornerRadius::same(5),
            Color32::from_rgba_unmultiplied(15, 23, 42, 22),
        );

        let scale = pw / note.paper_settings.page_width.max(1.0);
        if let Some(page) = &note.first_page {
            let stroke_mesh = self.build_or_get_note_stroke_mesh(
                &note.path,
                note.modified_at,
                scale,
                page,
                render_cx,
            );
            let note_hash = fnv1a_path(&note.path);
            paint_page_thumbnail_preview(
                ui.ctx(),
                ui.painter(),
                paper_card_rect,
                &note.paper_settings,
                page,
                scale,
                theme,
                stroke_mesh.as_deref(),
                &mut self.image_cache,
                Some((&mut self.svg_cache, Some(note_hash))),
            );
        } else {
            ui.painter()
                .rect_filled(paper_card_rect, CornerRadius::same(4), theme.paper_bg);
        }

        ui.painter().rect_stroke(
            paper_card_rect,
            CornerRadius::same(4),
            Stroke::new(0.85, theme.thumb_card_border),
            StrokeKind::Outside,
        );

        if note.is_favorite {
            let badge_center = pos2(paper_card_rect.right() - 13.0, paper_card_rect.top() + 13.0);
            ui.painter().circle_filled(
                badge_center,
                9.5,
                Color32::from_rgba_unmultiplied(255, 255, 255, 232),
            );
            ui.painter().circle_stroke(
                badge_center,
                9.5,
                Stroke::new(0.75, Color32::from_rgba_unmultiplied(217, 119, 6, 90)),
            );
            draw_icon(
                ui.painter(),
                Rect::from_center_size(badge_center, vec2(12.0, 12.0)),
                ToolIcon::StarFilled,
                theme.lib_star_gold,
            );
        }
    }

    fn build_or_get_note_stroke_mesh(
        &mut self,
        path: &Path,
        modified_at: u64,
        scale: f32,
        page: &DocumentPage,
        render_cx: &mut CairoRenderContext,
    ) -> Option<Arc<egui::epaint::Mesh>> {
        if page.strokes.is_empty() {
            return None;
        }
        let scale_bits = scale.to_bits();
        let key = (path.to_path_buf(), modified_at, scale_bits);
        if let Some(mesh) = self.thumb_meshes.get(&key) {
            return Some(Arc::clone(mesh));
        }

        // Tránh phình bộ nhớ khi thay đổi kích thước cửa sổ hoặc chỉnh sửa liên tiếp
        self.thumb_meshes
            .retain(|(p, m, _), _| p != path || *m == modified_at);

        let mut mesh = egui::epaint::Mesh::default();
        for s in &page.strokes {
            render_cx.clear_batches();
            s.path.draw_composed(render_cx, &s.style);
            let batches = std::mem::take(&mut render_cx.batches);
            append_batches_to_mesh(&batches, Pos2::ZERO, scale, &mut mesh);
        }
        if mesh.indices.is_empty() {
            return None;
        }
        let arc = Arc::new(mesh);
        self.thumb_meshes.insert(key, Arc::clone(&arc));
        Some(arc)
    }

    fn show_card_menu(&mut self, ctx: &egui::Context, theme: &Theme) {
        let Some(menu) = self.card_menu.clone() else {
            return;
        };

        let mut close_menu = false;
        let mut clicked_outside = false;
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close_menu = true;
        }

        let menu_w = 178.0;
        let est_h = if menu.in_trash {
            86.0
        } else if menu.is_folder {
            86.0
        } else {
            154.0
        };
        let screen = ctx.content_rect();
        let clamped_x = menu.anchor.x.clamp(
            screen.left() + 8.0,
            (screen.right() - menu_w - 10.0).max(8.0),
        );
        let clamped_y = menu.anchor.y.clamp(
            screen.top() + 8.0,
            (screen.bottom() - est_h - 10.0).max(8.0),
        );

        #[allow(deprecated)]
        let area_resp = egui::Area::new(egui::Id::new("library_card_context_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(clamped_x, clamped_y))
            .show(ctx, |ui| {
                theme.dark_popup_frame(ui.style()).show(ui, |ui| {
                    ui.set_min_width(menu_w - 20.0);
                    ui.spacing_mut().item_spacing.y = 3.0;

                    if menu.in_trash {
                        if popup_menu_item(
                            ui,
                            theme,
                            ToolIcon::Restore,
                            "Restore",
                            Color32::from_rgb(125, 185, 255),
                        )
                        .clicked()
                        {
                            match self.store.restore_from_trash(&menu.path) {
                                Ok(_) => {
                                    self.note_cache.remove(&menu.path);
                                    self.thumb_meshes.retain(|(p, _, _), _| p != &menu.path);
                                    self.last_error = None;
                                    self.needs_refresh = true;
                                }
                                Err(e) => {
                                    self.last_error = Some(format!("Restore failed: {e}"));
                                }
                            }
                            close_menu = true;
                        }
                        if popup_menu_item(
                            ui,
                            theme,
                            ToolIcon::Trash,
                            "Delete Permanently",
                            Color32::from_rgb(248, 113, 113),
                        )
                        .clicked()
                        {
                            self.confirm_trash = Some(TrashConfirmModal::DeleteForever {
                                path: menu.path.clone(),
                                name: menu.display_name.clone(),
                                is_folder: menu.is_folder,
                            });
                            close_menu = true;
                        }
                    } else {
                        if popup_menu_item(ui, theme, ToolIcon::Text, "Rename", theme.tool_text_fg)
                            .clicked()
                        {
                            self.open_rename_dialog(
                                menu.path.clone(),
                                &menu.display_name,
                                menu.is_folder,
                            );
                            close_menu = true;
                        }

                        if !menu.is_folder {
                            if popup_menu_item(
                                ui,
                                theme,
                                ToolIcon::Pages,
                                "Duplicate",
                                theme.tool_text_fg,
                            )
                            .clicked()
                            {
                                match self.store.duplicate_note(&menu.path) {
                                    Ok(_) => {
                                        self.last_error = None;
                                        self.needs_refresh = true;
                                    }
                                    Err(e) => {
                                        self.last_error =
                                            Some(format!("Could not duplicate note: {e}"));
                                    }
                                }
                                close_menu = true;
                            }

                            let (fav_icon, fav_label, fav_color) = if menu.is_favorite {
                                (ToolIcon::StarFilled, "Remove Favorite", theme.lib_star_gold)
                            } else {
                                (ToolIcon::Star, "Favorite", theme.tool_text_fg)
                            };
                            if popup_menu_item(ui, theme, fav_icon, fav_label, fav_color).clicked()
                            {
                                match self.store.toggle_favorite_note(&menu.path) {
                                    Ok(_) => {
                                        self.note_cache.remove(&menu.path);
                                        self.last_error = None;
                                        self.needs_refresh = true;
                                    }
                                    Err(e) => {
                                        self.last_error =
                                            Some(format!("Could not update favorite: {e}"));
                                    }
                                }
                                close_menu = true;
                            }
                        }

                        ui.add_space(2.0);
                        ui.separator();
                        ui.add_space(2.0);

                        if popup_menu_item(
                            ui,
                            theme,
                            ToolIcon::Trash,
                            "Move to Trash",
                            Color32::from_rgb(248, 113, 113),
                        )
                        .clicked()
                        {
                            match self.store.move_to_trash(&menu.path, &menu.rel_folder) {
                                Ok(_) => {
                                    self.note_cache.remove(&menu.path);
                                    if menu.is_folder {
                                        self.thumb_meshes.clear();
                                        self.note_cache.clear();
                                    } else {
                                        self.thumb_meshes.retain(|(p, _, _), _| p != &menu.path);
                                    }
                                    self.last_error = None;
                                    self.needs_refresh = true;
                                }
                                Err(e) => {
                                    self.last_error = Some(format!("Move to Trash failed: {e}"));
                                }
                            }
                            close_menu = true;
                        }
                    }
                });
            })
            .response;

        if !menu.opened_this_frame {
            let popup_rect = area_resp.rect;
            let press_outside = ctx.input(|i| {
                (i.pointer.primary_pressed() || i.pointer.secondary_pressed())
                    && i.pointer
                        .interact_pos()
                        .is_some_and(|p| !popup_rect.contains(p))
            });
            if press_outside {
                clicked_outside = true;
            }
        }

        if close_menu || clicked_outside {
            self.card_menu = None;
        } else if let Some(m) = self.card_menu.as_mut() {
            m.opened_this_frame = false;
        }
    }

    fn show_trash_confirm_dialog(&mut self, ctx: &egui::Context, theme: &Theme) {
        let Some(confirm) = self.confirm_trash.clone() else {
            return;
        };
        let mut proceed = false;
        let mut cancel = false;

        let (heading, message, btn_label) = match &confirm {
            TrashConfirmModal::DeleteForever {
                name, is_folder, ..
            } => {
                let kind = if *is_folder { "folder" } else { "note" };
                (
                    "Delete Permanently?",
                    format!("Permanently delete {kind} \"{name}\"? This action cannot be undone."),
                    "Delete Permanently",
                )
            }
            TrashConfirmModal::EmptyTrash { count } => (
                "Empty Trash?",
                format!(
                    "Permanently delete all {count} item(s) in Trash? This action cannot be undone."
                ),
                "Empty Trash",
            ),
        };

        #[allow(deprecated)]
        let modal =
            egui::Modal::new(egui::Id::new("library_trash_confirm_modal")).show(ctx, |ui| {
                ui.set_width(350.0);
                ui.heading(heading);
                ui.add_space(6.0);
                ui.label(
                    RichText::new(message)
                        .size(theme.font_size_body)
                        .color(theme.lib_sidebar_fg),
                );
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    cancel = true;
                }
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    let del_btn = egui::Button::new(RichText::new(btn_label).color(Color32::WHITE))
                        .fill(theme.lib_danger_fg);
                    if ui.add(del_btn).clicked() {
                        proceed = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });

        if cancel || modal.should_close() {
            self.confirm_trash = None;
            return;
        }

        if proceed {
            self.confirm_trash = None;
            match confirm {
                TrashConfirmModal::DeleteForever { path, .. } => {
                    match self.store.delete_permanently(&path) {
                        Ok(()) => {
                            self.note_cache.remove(&path);
                            self.thumb_meshes.retain(|(p, _, _), _| p != &path);
                            self.last_error = None;
                            self.needs_refresh = true;
                        }
                        Err(e) => {
                            self.last_error = Some(format!("Delete failed: {e}"));
                        }
                    }
                }
                TrashConfirmModal::EmptyTrash { .. } => match self.store.empty_trash() {
                    Ok(()) => {
                        self.note_cache.clear();
                        self.thumb_meshes.clear();
                        self.last_error = None;
                        self.needs_refresh = true;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Empty Trash failed: {e}"));
                    }
                },
            }
        }
    }

    fn show_create_dialog(&mut self, ctx: &egui::Context, theme: &Theme) -> Option<LibraryAction> {
        let state = self.create_dialog.as_mut()?;
        let mut submit = false;
        let mut cancel = false;

        let (title, hint, ok_label) = match &state.mode {
            NameDialogMode::CreateFolder => ("New Folder", "Enter folder name...", "Create"),
            NameDialogMode::CreateNote => ("New Note", "Enter note title...", "Create"),
            NameDialogMode::RenameFolder { .. } => {
                ("Rename Folder", "Enter new folder name...", "Save")
            }
            NameDialogMode::RenameNote { .. } => ("Rename Note", "Enter new note title...", "Save"),
        };

        #[allow(deprecated)]
        let modal = egui::Modal::new(egui::Id::new("library_create_modal")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.heading(title);
            ui.add_space(8.0);
            let edit = egui::TextEdit::singleline(&mut state.name)
                .id_salt("library_create_name_input")
                .desired_width(320.0)
                .hint_text(hint)
                .show(ui);
            if state.needs_focus {
                edit.response.request_focus();
                state.needs_focus = false;
            }
            if edit.response.changed() {
                state.error = None;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                submit = true;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if let Some(err) = &state.error {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(err)
                        .size(theme.font_size_small)
                        .color(Color32::from_rgb(217, 48, 37)),
                );
            }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button(ok_label).clicked() {
                    submit = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });

        if cancel || modal.should_close() {
            self.create_dialog = None;
            return None;
        }

        if submit {
            let trimmed = state.name.trim().to_string();
            if trimmed.is_empty() {
                state.error = Some("Name cannot be empty.".to_string());
                state.needs_focus = true;
                return None;
            }
            let mode = state.mode.clone();
            let rel_folder = self.rel_folder.clone();
            self.create_dialog = None;
            match mode {
                NameDialogMode::CreateFolder => {
                    return Some(LibraryAction::CreateFolder {
                        name: trimmed,
                        rel_folder,
                    });
                }
                NameDialogMode::CreateNote => {
                    return Some(LibraryAction::CreateNote {
                        title: trimmed,
                        rel_folder,
                    });
                }
                NameDialogMode::RenameFolder { path } => {
                    match self.store.rename_folder(&path, &trimmed) {
                        Ok(_) => {
                            self.note_cache.clear();
                            self.thumb_meshes.clear();
                            self.last_error = None;
                            self.needs_refresh = true;
                        }
                        Err(e) => {
                            self.last_error = Some(format!("Rename folder failed: {e}"));
                        }
                    }
                    return None;
                }
                NameDialogMode::RenameNote { path } => {
                    match self.store.rename_note(&path, &trimmed) {
                        Ok(_) => {
                            self.invalidate_note_path(&path);
                            self.last_error = None;
                        }
                        Err(e) => {
                            self.last_error = Some(format!("Rename note failed: {e}"));
                        }
                    }
                    return None;
                }
            }
        }
        None
    }
}

fn sidebar_nav_row(
    ui: &mut Ui,
    theme: &Theme,
    icon: ToolIcon,
    label: &str,
    selected: bool,
) -> egui::Response {
    let row_w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(row_w, 36.0), Sense::click());
    let bg = if selected {
        theme.lib_sidebar_selected_bg
    } else if resp.hovered() {
        theme.lib_sidebar_hover_bg
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, CornerRadius::same(9), bg);
    }

    let fg = if selected {
        theme.lib_sidebar_active_fg
    } else {
        theme.lib_sidebar_fg
    };
    let icon_rect = Rect::from_min_size(
        pos2(rect.left() + 10.0, rect.center().y - 9.0),
        vec2(18.0, 18.0),
    );
    draw_icon(ui.painter(), icon_rect, icon, fg);
    ui.painter().text(
        pos2(icon_rect.right() + 10.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        fg,
    );
    resp
}

fn pill_action_button(
    ui: &mut Ui,
    theme: &Theme,
    id_salt: &str,
    icon: ToolIcon,
    label: &str,
    primary: bool,
) -> egui::Response {
    let label_galley = ui.painter().layout_no_wrap(
        label.to_string(),
        FontId::proportional(theme.font_size_body),
        Color32::WHITE,
    );
    let w = 12.0 + 16.0 + 6.0 + label_galley.size().x + 12.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let (bg, stroke_col, fg) = if primary {
        let base = if resp.hovered() {
            Color32::from_rgb(20, 78, 166)
        } else {
            theme.nav_bar_bg
        };
        (base, base, Color32::WHITE)
    } else {
        let base = if resp.hovered() {
            theme.lib_search_bg
        } else {
            theme.lib_main_bg
        };
        (base, theme.lib_search_stroke, theme.lib_title_fg)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(16), bg);
    if !primary {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(16),
            Stroke::new(1.0, stroke_col),
            StrokeKind::Inside,
        );
    }
    let _ = id_salt;
    let icon_rect = Rect::from_min_size(
        pos2(rect.left() + 11.0, rect.center().y - 8.0),
        vec2(16.0, 16.0),
    );
    draw_icon(ui.painter(), icon_rect, icon, fg);
    ui.painter().text(
        pos2(icon_rect.right() + 6.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(theme.font_size_body),
        fg,
    );
    resp
}

fn empty_trash_pill_button(ui: &mut Ui, theme: &Theme, count: usize) -> egui::Response {
    let label = "Empty Trash";
    let label_galley = ui.painter().layout_no_wrap(
        label.to_string(),
        FontId::proportional(theme.font_size_body),
        Color32::WHITE,
    );
    let w = 12.0 + 16.0 + 6.0 + label_galley.size().x + 12.0;
    let enabled = count > 0;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 32.0), sense);
    let resp = if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp.on_hover_text("Trash is already empty")
    };
    let (bg, stroke_col, fg) = if !enabled {
        (
            theme.lib_search_bg,
            theme.lib_search_stroke,
            theme.lib_muted_fg,
        )
    } else if resp.hovered() {
        (
            Color32::from_rgb(254, 226, 226),
            theme.lib_danger_fg,
            theme.lib_danger_fg,
        )
    } else {
        (
            theme.lib_danger_bg,
            Color32::from_rgb(252, 165, 165),
            theme.lib_danger_fg,
        )
    };
    ui.painter().rect_filled(rect, CornerRadius::same(16), bg);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(16),
        Stroke::new(1.0, stroke_col),
        StrokeKind::Inside,
    );
    let icon_rect = Rect::from_min_size(
        pos2(rect.left() + 11.0, rect.center().y - 8.0),
        vec2(16.0, 16.0),
    );
    draw_icon(ui.painter(), icon_rect, ToolIcon::Trash, fg);
    ui.painter().text(
        pos2(icon_rect.right() + 6.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(theme.font_size_body),
        fg,
    );
    resp
}

fn popup_menu_item(
    ui: &mut Ui,
    theme: &Theme,
    icon: ToolIcon,
    label: &str,
    fg: Color32,
) -> egui::Response {
    let row_w = ui.available_width().max(150.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(row_w, 28.0), Sense::click());
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), theme.tool_hover_bg);
    }
    let icon_rect = Rect::from_min_size(
        pos2(rect.left() + 8.0, rect.center().y - 7.5),
        vec2(15.0, 15.0),
    );
    draw_icon(ui.painter(), icon_rect, icon, fg);
    ui.painter().text(
        pos2(icon_rect.right() + 8.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(theme.font_size_body),
        fg,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn paint_card_caption_with_menu(
    painter: &egui::Painter,
    slot_rect: Rect,
    card_w: f32,
    title: &str,
    meta: &str,
    theme: &Theme,
) -> Rect {
    let title_y = slot_rect.bottom() + 9.0;
    let btn_size = 20.0;
    let gap = 2.0;
    let max_title_w = (card_w - btn_size - gap - 4.0).max(40.0);
    let truncated_title = truncate_with_ellipsis(painter, title, 13.5, max_title_w);
    let title_galley = painter.layout_no_wrap(
        truncated_title.clone(),
        FontId::proportional(13.5),
        theme.lib_title_fg,
    );
    let total_w = title_galley.size().x + gap + btn_size;
    let start_x = (slot_rect.center().x - total_w * 0.5).max(slot_rect.left() + 2.0);
    painter.galley(pos2(start_x, title_y), title_galley, theme.lib_title_fg);

    let chevron_rect = Rect::from_min_size(
        pos2(
            (start_x + (total_w - btn_size)).min(slot_rect.right() - btn_size),
            title_y - 1.5,
        ),
        vec2(btn_size, btn_size),
    );

    let max_meta_w = (card_w - 6.0).max(40.0);
    let truncated_meta = truncate_with_ellipsis(painter, meta, 11.5, max_meta_w);
    painter.text(
        pos2(slot_rect.center().x, title_y + 20.0),
        Align2::CENTER_TOP,
        truncated_meta,
        FontId::proportional(11.5),
        theme.lib_muted_fg,
    );
    chevron_rect
}

fn truncate_with_ellipsis(
    painter: &egui::Painter,
    text: &str,
    font_size: f32,
    max_w: f32,
) -> String {
    let font = FontId::proportional(font_size);
    if painter
        .layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE)
        .size()
        .x
        <= max_w
    {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        let mut cand: String = chars[..mid].iter().collect();
        cand.push('…');
        let w = painter
            .layout_no_wrap(cand, font.clone(), Color32::WHITE)
            .size()
            .x;
        if w <= max_w {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let mut out: String = chars[..lo].iter().collect();
    out.push('…');
    out
}

fn draw_small_x(painter: &egui::Painter, center: Pos2, color: Color32) {
    let arm = 4.0;
    let stroke = Stroke::new(1.5, color);
    painter.line_segment(
        [
            pos2(center.x - arm, center.y - arm),
            pos2(center.x + arm, center.y + arm),
        ],
        stroke,
    );
    painter.line_segment(
        [
            pos2(center.x - arm, center.y + arm),
            pos2(center.x + arm, center.y - arm),
        ],
        stroke,
    );
}

fn format_item_count(count: usize) -> String {
    match count {
        0 => "Empty".to_string(),
        1 => "1 item".to_string(),
        n => format!("{n} items"),
    }
}

fn format_relative_time(ts: u64) -> String {
    if ts == 0 {
        return "Recently".to_string();
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(ts);
    let diff = now.saturating_sub(ts);
    if diff < 60 {
        "Just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86_400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86_400)
    }
}

fn fnv1a_path(path: &Path) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in path.to_string_lossy().as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
