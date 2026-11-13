// Hộp thoại nhập công thức LaTeX & Live Preview (Bước 4.2):
// - Phong cách Pop-up tối bo góc
// - Debounce ~0,5 giây sau khi ngừng gõ rồi tự biên dịch trên luồng nền
// - Khi gõ sai cú pháp: báo lỗi rõ ràng nhưng vẫn giữ hình xem trước hợp lệ trước đó

use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, ScrollArea, Stroke, TextEdit,
};
use inknote_latex::{compile_latex_to_svg, CompileOptions, LatexError};

use crate::{svg_cache::SvgTextureCache, theme::Theme};

const DIALOG_WIDTH: f32 = 560.0;
const DEBOUNCE_DELAY: Duration = Duration::from_millis(500);
const PREVIEW_TEX_ID: u64 = 999_001;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolCategory {
    All,
    GreekHebrew,
    Delimiters,
    MathsConstructs,
    VariableSized,
    StandardFunctions,
    OperatorsRelations,
    Arrows,
    Accents,
    Miscellaneous,
    LetterStyles,
}

impl SymbolCategory {
    const ALL_CATS: [Self; 11] = [
        Self::All,
        Self::GreekHebrew,
        Self::Delimiters,
        Self::MathsConstructs,
        Self::VariableSized,
        Self::StandardFunctions,
        Self::OperatorsRelations,
        Self::Arrows,
        Self::Accents,
        Self::Miscellaneous,
        Self::LetterStyles,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::GreekHebrew => "Greek/Hebrew Letters",
            Self::Delimiters => "Delimiters",
            Self::MathsConstructs => "Maths Constructs",
            Self::VariableSized => "Variable-sized symbols",
            Self::StandardFunctions => "Standard Functions",
            Self::OperatorsRelations => "Operators and Relations",
            Self::Arrows => "Arrows",
            Self::Accents => "Accents",
            Self::Miscellaneous => "Miscellaneous",
            Self::LetterStyles => "Letter Styles",
        }
    }
}

struct LatexSymbol {
    cat: SymbolCategory,
    label: &'static str,
    tex: &'static str,
}

const LATEX_SYMBOLS: &[LatexSymbol] = &[
    // Greek/Hebrew Letters
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "α",
        tex: r"\alpha",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "β",
        tex: r"\beta",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "γ",
        tex: r"\gamma",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "δ",
        tex: r"\delta",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "ε",
        tex: r"\epsilon",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "ζ",
        tex: r"\zeta",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "η",
        tex: r"\eta",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "θ",
        tex: r"\theta",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "λ",
        tex: r"\lambda",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "μ",
        tex: r"\mu",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "π",
        tex: r"\pi",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "ρ",
        tex: r"\rho",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "σ",
        tex: r"\sigma",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "τ",
        tex: r"\tau",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "φ",
        tex: r"\phi",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "ψ",
        tex: r"\psi",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "ω",
        tex: r"\omega",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "Γ",
        tex: r"\Gamma",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "Δ",
        tex: r"\Delta",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "Θ",
        tex: r"\Theta",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "Λ",
        tex: r"\Lambda",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "Σ",
        tex: r"\Sigma",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "Φ",
        tex: r"\Phi",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "Ω",
        tex: r"\Omega",
    },
    LatexSymbol {
        cat: SymbolCategory::GreekHebrew,
        label: "ℵ",
        tex: r"\aleph",
    },
    // Delimiters
    LatexSymbol {
        cat: SymbolCategory::Delimiters,
        label: "( )",
        tex: r"\left(  \right)",
    },
    LatexSymbol {
        cat: SymbolCategory::Delimiters,
        label: "[ ]",
        tex: r"\left[  \right]",
    },
    LatexSymbol {
        cat: SymbolCategory::Delimiters,
        label: "{ }",
        tex: r"\left\{  \right\}",
    },
    LatexSymbol {
        cat: SymbolCategory::Delimiters,
        label: "| |",
        tex: r"\left|  \right|",
    },
    LatexSymbol {
        cat: SymbolCategory::Delimiters,
        label: "‖ ‖",
        tex: r"\left\|  \right\|",
    },
    LatexSymbol {
        cat: SymbolCategory::Delimiters,
        label: "⟨ ⟩",
        tex: r"\langle  \rangle",
    },
    LatexSymbol {
        cat: SymbolCategory::Delimiters,
        label: "⌊ ⌋",
        tex: r"\lfloor  \rfloor",
    },
    LatexSymbol {
        cat: SymbolCategory::Delimiters,
        label: "⌈ ⌉",
        tex: r"\lceil  \rceil",
    },
    // Maths Constructs
    LatexSymbol {
        cat: SymbolCategory::MathsConstructs,
        label: "a/b",
        tex: r"\frac{a}{b}",
    },
    LatexSymbol {
        cat: SymbolCategory::MathsConstructs,
        label: "√x",
        tex: r"\sqrt{x}",
    },
    LatexSymbol {
        cat: SymbolCategory::MathsConstructs,
        label: "ⁿ√x",
        tex: r"\sqrt[n]{x}",
    },
    LatexSymbol {
        cat: SymbolCategory::MathsConstructs,
        label: "xⁿ",
        tex: r"x^{n}",
    },
    LatexSymbol {
        cat: SymbolCategory::MathsConstructs,
        label: "xᵢ",
        tex: r"x_{i}",
    },
    LatexSymbol {
        cat: SymbolCategory::MathsConstructs,
        label: "C(n,k)",
        tex: r"\binom{n}{k}",
    },
    LatexSymbol {
        cat: SymbolCategory::MathsConstructs,
        label: "{cases}",
        tex: r"\begin{cases} a & b \\ c & d \end{cases}",
    },
    LatexSymbol {
        cat: SymbolCategory::MathsConstructs,
        label: "(mat)",
        tex: r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}",
    },
    // Variable-sized symbols
    LatexSymbol {
        cat: SymbolCategory::VariableSized,
        label: "∑",
        tex: r"\sum_{i=1}^{n}",
    },
    LatexSymbol {
        cat: SymbolCategory::VariableSized,
        label: "∏",
        tex: r"\prod_{i=1}^{n}",
    },
    LatexSymbol {
        cat: SymbolCategory::VariableSized,
        label: "∫",
        tex: r"\int_{a}^{b}",
    },
    LatexSymbol {
        cat: SymbolCategory::VariableSized,
        label: "∬",
        tex: r"\iint",
    },
    LatexSymbol {
        cat: SymbolCategory::VariableSized,
        label: "∮",
        tex: r"\oint",
    },
    LatexSymbol {
        cat: SymbolCategory::VariableSized,
        label: "⋃",
        tex: r"\bigcup_{i=1}^{n}",
    },
    LatexSymbol {
        cat: SymbolCategory::VariableSized,
        label: "⋂",
        tex: r"\bigcap_{i=1}^{n}",
    },
    LatexSymbol {
        cat: SymbolCategory::VariableSized,
        label: "lim",
        tex: r"\lim_{x \to \infty}",
    },
    // Standard Functions
    LatexSymbol {
        cat: SymbolCategory::StandardFunctions,
        label: "sin",
        tex: r"\sin",
    },
    LatexSymbol {
        cat: SymbolCategory::StandardFunctions,
        label: "cos",
        tex: r"\cos",
    },
    LatexSymbol {
        cat: SymbolCategory::StandardFunctions,
        label: "tan",
        tex: r"\tan",
    },
    LatexSymbol {
        cat: SymbolCategory::StandardFunctions,
        label: "cot",
        tex: r"\cot",
    },
    LatexSymbol {
        cat: SymbolCategory::StandardFunctions,
        label: "arcsin",
        tex: r"\arcsin",
    },
    LatexSymbol {
        cat: SymbolCategory::StandardFunctions,
        label: "arccos",
        tex: r"\arccos",
    },
    LatexSymbol {
        cat: SymbolCategory::StandardFunctions,
        label: "arctan",
        tex: r"\arctan",
    },
    LatexSymbol {
        cat: SymbolCategory::StandardFunctions,
        label: "ln",
        tex: r"\ln",
    },
    LatexSymbol {
        cat: SymbolCategory::StandardFunctions,
        label: "log",
        tex: r"\log",
    },
    LatexSymbol {
        cat: SymbolCategory::StandardFunctions,
        label: "exp",
        tex: r"\exp",
    },
    LatexSymbol {
        cat: SymbolCategory::StandardFunctions,
        label: "max",
        tex: r"\max",
    },
    LatexSymbol {
        cat: SymbolCategory::StandardFunctions,
        label: "min",
        tex: r"\min",
    },
    LatexSymbol {
        cat: SymbolCategory::StandardFunctions,
        label: "det",
        tex: r"\det",
    },
    LatexSymbol {
        cat: SymbolCategory::StandardFunctions,
        label: "gcd",
        tex: r"\gcd",
    },
    // Operators and Relations
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "±",
        tex: r"\pm",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "∓",
        tex: r"\mp",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "×",
        tex: r"\times",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "÷",
        tex: r"\div",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "·",
        tex: r"\cdot",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "∘",
        tex: r"\circ",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "≤",
        tex: r"\leq",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "≥",
        tex: r"\geq",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "≠",
        tex: r"\neq",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "≈",
        tex: r"\approx",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "≡",
        tex: r"\equiv",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "∼",
        tex: r"\sim",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "∝",
        tex: r"\propto",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "∈",
        tex: r"\in",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "∉",
        tex: r"\notin",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "⊂",
        tex: r"\subset",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "⊆",
        tex: r"\subseteq",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "∪",
        tex: r"\cup",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "∩",
        tex: r"\cap",
    },
    LatexSymbol {
        cat: SymbolCategory::OperatorsRelations,
        label: "∖",
        tex: r"\setminus",
    },
    // Arrows
    LatexSymbol {
        cat: SymbolCategory::Arrows,
        label: "→",
        tex: r"\to",
    },
    LatexSymbol {
        cat: SymbolCategory::Arrows,
        label: "←",
        tex: r"\leftarrow",
    },
    LatexSymbol {
        cat: SymbolCategory::Arrows,
        label: "↔",
        tex: r"\leftrightarrow",
    },
    LatexSymbol {
        cat: SymbolCategory::Arrows,
        label: "⇒",
        tex: r"\Rightarrow",
    },
    LatexSymbol {
        cat: SymbolCategory::Arrows,
        label: "⇐",
        tex: r"\Leftarrow",
    },
    LatexSymbol {
        cat: SymbolCategory::Arrows,
        label: "⇔",
        tex: r"\Leftrightarrow",
    },
    LatexSymbol {
        cat: SymbolCategory::Arrows,
        label: "↦",
        tex: r"\mapsto",
    },
    LatexSymbol {
        cat: SymbolCategory::Arrows,
        label: "↑",
        tex: r"\uparrow",
    },
    LatexSymbol {
        cat: SymbolCategory::Arrows,
        label: "↓",
        tex: r"\downarrow",
    },
    // Accents
    LatexSymbol {
        cat: SymbolCategory::Accents,
        label: "x̂",
        tex: r"\hat{x}",
    },
    LatexSymbol {
        cat: SymbolCategory::Accents,
        label: "x̄",
        tex: r"\bar{x}",
    },
    LatexSymbol {
        cat: SymbolCategory::Accents,
        label: "v⃗",
        tex: r"\vec{v}",
    },
    LatexSymbol {
        cat: SymbolCategory::Accents,
        label: "ẋ",
        tex: r"\dot{x}",
    },
    LatexSymbol {
        cat: SymbolCategory::Accents,
        label: "ẍ",
        tex: r"\ddot{x}",
    },
    LatexSymbol {
        cat: SymbolCategory::Accents,
        label: "x̃",
        tex: r"\tilde{x}",
    },
    LatexSymbol {
        cat: SymbolCategory::Accents,
        label: "AB̂",
        tex: r"\widehat{AB}",
    },
    LatexSymbol {
        cat: SymbolCategory::Accents,
        label: "A̅B̅",
        tex: r"\overline{AB}",
    },
    LatexSymbol {
        cat: SymbolCategory::Accents,
        label: "A̲B̲",
        tex: r"\underline{AB}",
    },
    // Miscellaneous
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "∞",
        tex: r"\infty",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "∂",
        tex: r"\partial",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "∇",
        tex: r"\nabla",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "∀",
        tex: r"\forall",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "∃",
        tex: r"\exists",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "∄",
        tex: r"\nexists",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "∅",
        tex: r"\emptyset",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "∴",
        tex: r"\therefore",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "∵",
        tex: r"\because",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "⋯",
        tex: r"\cdots",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "⋮",
        tex: r"\vdots",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "⋱",
        tex: r"\ddots",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "∠",
        tex: r"\angle",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "△",
        tex: r"\triangle",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "⊥",
        tex: r"\perp",
    },
    LatexSymbol {
        cat: SymbolCategory::Miscellaneous,
        label: "∥",
        tex: r"\parallel",
    },
    // Letter Styles
    LatexSymbol {
        cat: SymbolCategory::LetterStyles,
        label: "ℝ",
        tex: r"\mathbb{R}",
    },
    LatexSymbol {
        cat: SymbolCategory::LetterStyles,
        label: "ℕ",
        tex: r"\mathbb{N}",
    },
    LatexSymbol {
        cat: SymbolCategory::LetterStyles,
        label: "ℤ",
        tex: r"\mathbb{Z}",
    },
    LatexSymbol {
        cat: SymbolCategory::LetterStyles,
        label: "ℚ",
        tex: r"\mathbb{Q}",
    },
    LatexSymbol {
        cat: SymbolCategory::LetterStyles,
        label: "ℂ",
        tex: r"\mathbb{C}",
    },
    LatexSymbol {
        cat: SymbolCategory::LetterStyles,
        label: "𝓛",
        tex: r"\mathcal{L}",
    },
    LatexSymbol {
        cat: SymbolCategory::LetterStyles,
        label: "𝖌",
        tex: r"\mathfrak{g}",
    },
    LatexSymbol {
        cat: SymbolCategory::LetterStyles,
        label: "bf",
        tex: r"\mathbf{v}",
    },
    LatexSymbol {
        cat: SymbolCategory::LetterStyles,
        label: "rm",
        tex: r"\mathrm{d}",
    },
    LatexSymbol {
        cat: SymbolCategory::LetterStyles,
        label: "text",
        tex: r"\text{text}",
    },
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LatexDialogAction {
    None,
    Cancel,
    Confirm { source: String, svg: String },
}

pub struct LatexDialogState {
    pub open: bool,
    pub source: String,
    pub category: SymbolCategory,
    last_edit_at: Option<Instant>,
    pending_compile: bool,
    compiling: bool,
    latest_seq: u64,
    pub preview_svg: Option<String>,
    pub error_msg: Option<String>,
    rx: Option<Receiver<(u64, Result<String, LatexError>)>>,
}

impl Default for LatexDialogState {
    fn default() -> Self {
        Self {
            open: false,
            source: String::new(),
            category: SymbolCategory::All,
            last_edit_at: None,
            pending_compile: false,
            compiling: false,
            latest_seq: 0,
            preview_svg: None,
            error_msg: None,
            rx: None,
        }
    }
}

impl LatexDialogState {
    /// Mở hộp thoại với nội dung mặc định (hoặc mã cũ khi sửa ở Bước 4.4).
    pub fn open_with(&mut self, initial_source: &str, ctx: &egui::Context) {
        self.open = true;
        self.source = initial_source.to_string();
        self.error_msg = None;
        self.preview_svg = None;
        self.pending_compile = false;
        self.last_edit_at = None;
        if !self.source.trim().is_empty() {
            self.start_compile(ctx);
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.pending_compile = false;
    }

    fn mark_edited(&mut self) {
        self.last_edit_at = Some(Instant::now());
        self.pending_compile = true;
    }

    fn start_compile(&mut self, ctx: &egui::Context) {
        self.pending_compile = false;
        if self.source.trim().is_empty() {
            self.preview_svg = None;
            self.error_msg = None;
            return;
        }

        self.latest_seq = self.latest_seq.wrapping_add(1);
        let seq = self.latest_seq;
        let src = self.source.clone();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.compiling = true;
        let ctx_clone = ctx.clone();

        thread::spawn(move || {
            let res = compile_latex_to_svg(&src, &CompileOptions::default());
            let _ = tx.send((seq, res));
            ctx_clone.request_repaint();
        });
    }

    fn poll_background(&mut self, ctx: &egui::Context, svg_cache: &mut SvgTextureCache) {
        if let Some(rx) = self.rx.as_ref() {
            if let Ok((seq, res)) = rx.try_recv() {
                self.compiling = false;
                self.rx = None;
                if seq == self.latest_seq {
                    match res {
                        Ok(svg) => {
                            self.preview_svg = Some(svg);
                            self.error_msg = None;
                            svg_cache.invalidate(PREVIEW_TEX_ID);
                        }
                        Err(LatexError::EmptySource) => {
                            self.preview_svg = None;
                            self.error_msg = None;
                        }
                        Err(err) => {
                            // Giữ nguyên preview_svg cũ khi có lỗi cú pháp để người dùng dễ đối chiếu
                            self.error_msg = Some(err.to_string());
                        }
                    }
                }
            }
        }

        if self.pending_compile {
            if let Some(edited_at) = self.last_edit_at {
                let elapsed = edited_at.elapsed();
                if elapsed >= DEBOUNCE_DELAY {
                    if !self.compiling {
                        self.start_compile(ctx);
                    }
                } else {
                    ctx.request_repaint_after(
                        (DEBOUNCE_DELAY - elapsed).min(Duration::from_millis(80)),
                    );
                }
            }
        }
    }

    /// Vẽ hộp thoại Pop-up tối bo góc ở giữa màn hình và xử lý sự kiện Live Preview.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        svg_cache: &mut SvgTextureCache,
    ) -> LatexDialogAction {
        if !self.open {
            return LatexDialogAction::None;
        }

        self.poll_background(ctx, svg_cache);

        let mut action = LatexDialogAction::None;
        let esc_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let ctrl_enter = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter));

        // Lớp phủ mờ nhẹ phía sau hộp thoại
        let screen_rect = ctx.content_rect();
        egui::Area::new(egui::Id::new("latex_dialog_backdrop"))
            .order(egui::Order::Middle)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let (r, _) = ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
                ui.painter()
                    .rect_filled(r, CornerRadius::ZERO, Color32::from_black_alpha(90));
            });

        egui::Window::new("latex_editor_dialog")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .fixed_size(egui::vec2(DIALOG_WIDTH, 0.0))
            .frame(
                egui::Frame::new()
                    .fill(theme.tool_bar_pill_bg)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(64, 72, 88)))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(egui::Margin::same(16))
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: Color32::from_rgba_unmultiplied(0, 0, 0, 110),
                    }),
            )
            .show(ctx, |ui| {
                ui.set_width(DIALOG_WIDTH - 32.0);

                // Tiêu đề + nút đóng ×
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("LaTeX Formula")
                            .color(Color32::WHITE)
                            .size(16.0)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("×")
                                        .color(Color32::from_rgb(190, 198, 212))
                                        .size(16.0),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            action = LatexDialogAction::Cancel;
                        }
                        if self.compiling {
                            ui.spinner();
                            ui.label(
                                egui::RichText::new("Rendering...")
                                    .color(Color32::from_rgb(150, 195, 255))
                                    .size(12.0),
                            );
                        } else if self.pending_compile {
                            ui.label(
                                egui::RichText::new("Waiting...")
                                    .color(Color32::from_rgb(160, 168, 184))
                                    .size(12.0),
                            );
                        }
                    });
                });

                ui.add_space(8.0);

                // Bộ chọn nhóm ký hiệu LaTeX (11 nhóm như ảnh mẫu) + lưới nút chèn nhanh
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Symbols:")
                            .color(Color32::from_rgb(180, 188, 204))
                            .size(12.5),
                    );
                    egui::ComboBox::from_id_salt("latex_symbol_category")
                        .width(210.0)
                        .selected_text(self.category.label())
                        .show_ui(ui, |ui| {
                            for cat in SymbolCategory::ALL_CATS {
                                ui.selectable_value(&mut self.category, cat, cat.label());
                            }
                        });
                });

                ui.add_space(6.0);

                egui::Frame::new()
                    .fill(Color32::from_rgb(30, 35, 44))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(56, 64, 78)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(6))
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .id_salt("latex_symbols_scroll")
                            .max_height(88.0)
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                                    for sym in LATEX_SYMBOLS {
                                        if self.category != SymbolCategory::All
                                            && sym.cat != self.category
                                        {
                                            continue;
                                        }
                                        let btn = egui::Button::new(
                                            egui::RichText::new(sym.label)
                                                .color(Color32::from_rgb(232, 238, 248))
                                                .size(13.0),
                                        )
                                        .min_size(egui::vec2(30.0, 26.0))
                                        .fill(Color32::from_rgb(48, 55, 68))
                                        .corner_radius(CornerRadius::same(5));
                                        if ui.add(btn).on_hover_text(sym.tex).clicked() {
                                            if !self.source.is_empty() && !self.source.ends_with(' ')
                                            {
                                                self.source.push(' ');
                                            }
                                            self.source.push_str(sym.tex);
                                            self.mark_edited();
                                        }
                                    }
                                });
                            });
                    });

                ui.add_space(8.0);

                // Ô nhập mã LaTeX (nền tối, chữ sáng, font monospace — không cần gõ $$)
                let edit_frame = egui::Frame::new()
                    .fill(Color32::from_rgb(24, 28, 36))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(70, 80, 98)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(8));
                edit_frame.show(ui, |ui| {
                    let resp = ui.add(
                        TextEdit::multiline(&mut self.source)
                            .desired_rows(4)
                            .desired_width(f32::INFINITY)
                            .frame(egui::Frame::NONE)
                            .font(FontId::monospace(14.0))
                            .text_color(Color32::from_rgb(238, 242, 248))
                            .hint_text(
                                "Type formula directly (no $ needed), e.g. x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}",
                            ),
                    );
                    if resp.changed() {
                        self.mark_edited();
                    }
                });

                ui.add_space(10.0);

                // Khung xem trước SVG (nền trắng để công thức chữ đen đọc rõ nhất)
                ui.label(
                    egui::RichText::new("Live Preview")
                        .color(Color32::from_rgb(180, 188, 204))
                        .size(12.0),
                );
                ui.add_space(4.0);

                let preview_h = 110.0;
                let (preview_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), preview_h),
                    egui::Sense::hover(),
                );
                ui.painter()
                    .rect_filled(preview_rect, CornerRadius::same(8), Color32::WHITE);
                ui.painter().rect_stroke(
                    preview_rect,
                    CornerRadius::same(8),
                    Stroke::new(1.0, Color32::from_rgb(205, 212, 224)),
                    egui::StrokeKind::Inside,
                );

                if let Some(svg) = self.preview_svg.as_deref() {
                    if let Ok((tex_id, logical_size)) =
                        svg_cache.get_or_render(ctx, PREVIEW_TEX_ID, svg, 1.35)
                    {
                        let base_size = logical_size * 1.6;
                        let max_w = (preview_rect.width() - 24.0).max(20.0);
                        let max_h = (preview_rect.height() - 16.0).max(20.0);
                        let fit = (max_w / base_size.x.max(1.0))
                            .min(max_h / base_size.y.max(1.0))
                            .min(1.0);
                        let draw_size = base_size * fit;
                        let img_rect = Rect::from_center_size(preview_rect.center(), draw_size);
                        ui.painter().with_clip_rect(preview_rect).image(
                            tex_id,
                            img_rect,
                            Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                } else {
                    ui.painter().text(
                        preview_rect.center(),
                        Align2::CENTER_CENTER,
                        "Preview will appear here ~0.5s after typing",
                        FontId::proportional(13.0),
                        Color32::from_rgb(140, 148, 162),
                    );
                }

                // Khung báo lỗi rõ ràng khi gõ sai (vẫn giữ hình xem trước cũ ở trên)
                if let Some(err) = &self.error_msg {
                    ui.add_space(8.0);
                    egui::Frame::new()
                        .fill(Color32::from_rgb(66, 30, 34))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(190, 74, 74)))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ScrollArea::vertical()
                                .id_salt("latex_err_scroll")
                                .max_height(68.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(err)
                                            .color(Color32::from_rgb(255, 178, 174))
                                            .monospace()
                                            .size(11.5),
                                    );
                                });
                        });
                }

                ui.add_space(12.0);

                // Hàng nút Cancel / OK
                let can_confirm =
                    self.preview_svg.is_some() && self.error_msg.is_none() && !self.compiling;
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ok_btn = egui::Button::new(
                            egui::RichText::new("OK (Ctrl+Enter)")
                                .color(Color32::WHITE)
                                .size(13.0),
                        )
                        .fill(if can_confirm {
                            theme.nav_bar_bg
                        } else {
                            Color32::from_rgb(65, 74, 90)
                        })
                        .corner_radius(CornerRadius::same(8));
                        if (ui.add_enabled(can_confirm, ok_btn).clicked()
                            || (ctrl_enter && can_confirm))
                            && self.preview_svg.is_some()
                        {
                            action = LatexDialogAction::Confirm {
                                source: self.source.clone(),
                                svg: self.preview_svg.clone().unwrap_or_default(),
                            };
                        }

                        let cancel_btn = egui::Button::new(
                            egui::RichText::new("Cancel")
                                .color(Color32::from_rgb(220, 226, 236))
                                .size(13.0),
                        )
                        .fill(Color32::from_rgb(54, 62, 76))
                        .corner_radius(CornerRadius::same(8));
                        if ui.add(cancel_btn).clicked() {
                            action = LatexDialogAction::Cancel;
                        }
                    });
                });
            });

        if esc_pressed {
            action = LatexDialogAction::Cancel;
        }
        if !matches!(action, LatexDialogAction::None) {
            self.close();
        }
        action
    }
}
