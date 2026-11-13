// Crate biên dịch LaTeX -> SVG (qua latex + dvisvgm, dự phòng pdflatex + pdftocairo)
// và rasterize SVG theo độ phân giải mức zoom bằng resvg.
// Quy tắc bảo mật: LUÔN truyền `-no-shell-escape`, có timeout huỷ tiến trình.

use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

static TEMP_SEQ: AtomicU64 = AtomicU64::new(1);

/// Tuỳ chọn biên dịch LaTeX sang SVG.
#[derive(Clone, Debug)]
pub struct CompileOptions {
    /// Thời gian tối đa cho toàn bộ quá trình biên dịch (mặc định 8 giây).
    pub timeout: Duration,
    /// Màu chữ RGB tuỳ chọn (nếu `None` dùng màu mặc định đen).
    pub color_rgb: Option<[u8; 3]>,
    /// Cho phép dùng `pdflatex` + `pdftocairo -svg` nếu thiếu `latex`/`dvisvgm`.
    pub allow_fallback: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(8),
            color_rgb: None,
            allow_fallback: true,
        }
    }
}

/// Kết quả ảnh bitmap đã rasterize từ SVG ở một tỷ lệ phóng to cụ thể.
#[derive(Clone, Debug)]
pub struct RenderedSvg {
    pub width_px: u32,
    pub height_px: u32,
    pub rgba: Vec<u8>,
    pub logical_width: f32,
    pub logical_height: f32,
}

/// Lỗi phát sinh khi biên dịch hoặc hiển thị LaTeX.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LatexError {
    EmptySource,
    Timeout(Duration),
    ToolNotFound(String),
    CompileFailed(String),
    SvgConversionFailed(String),
    SvgParseFailed(String),
    Io(String),
}

impl fmt::Display for LatexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySource => write!(f, "LaTeX source is empty."),
            Self::Timeout(d) => write!(
                f,
                "LaTeX compilation timed out after {:.1}s (process terminated).",
                d.as_secs_f32()
            ),
            Self::ToolNotFound(tool) => write!(
                f,
                "Required TeX tool '{tool}' was not found in PATH. Please install TeX Live."
            ),
            Self::CompileFailed(msg) => write!(f, "LaTeX compile error:\n{msg}"),
            Self::SvgConversionFailed(msg) => write!(f, "SVG conversion failed:\n{msg}"),
            Self::SvgParseFailed(msg) => write!(f, "Failed to parse SVG: {msg}"),
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
        }
    }
}

impl std::error::Error for LatexError {}

struct TempWorkDir {
    path: PathBuf,
}

impl TempWorkDir {
    fn new() -> Result<Self, LatexError> {
        let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("inknote_latex_{}_{}", std::process::id(), seq));
        fs::create_dir_all(&dir).map_err(|e| LatexError::Io(e.to_string()))?;
        Ok(Self { path: dir })
    }
}

impl Drop for TempWorkDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Tạo nội dung file `.tex` hoàn chỉnh hỗ trợ tiếng Việt (`T5` + `utf8`) và toán học (`amsmath, amssymb`).
pub fn build_tex_document(source: &str, color_rgb: Option<[u8; 3]>) -> String {
    let trimmed = source.trim();
    if trimmed.starts_with("\\documentclass") {
        return trimmed.to_string();
    }
    match color_rgb {
        Some([r, g, b]) => format!(
            "\\documentclass[preview,border=2pt]{{standalone}}\n\
             \\usepackage[T5]{{fontenc}}\n\
             \\usepackage[utf8]{{inputenc}}\n\
             \\usepackage{{amsmath,amssymb}}\n\
             \\usepackage{{xcolor}}\n\
             \\begin{{document}}\n\
             {{\\color[RGB]{{{r},{g},{b}}}{trimmed}}}\n\
             \\end{{document}}\n"
        ),
        None => format!(
            "\\documentclass[preview,border=2pt]{{standalone}}\n\
             \\usepackage[T5]{{fontenc}}\n\
             \\usepackage[utf8]{{inputenc}}\n\
             \\usepackage{{amsmath,amssymb}}\n\
             \\begin{{document}}\n\
             {trimmed}\n\
             \\end{{document}}\n"
        ),
    }
}

fn configure_command(program: &str, args: &[&str], work_dir: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(work_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

enum RunOutcome {
    Success,
    Failed {
        exit_code: Option<i32>,
        output: String,
    },
    TimedOut,
    NotFound(String),
}

fn run_with_timeout(
    program: &str,
    args: &[&str],
    work_dir: &Path,
    deadline: Instant,
) -> Result<RunOutcome, LatexError> {
    let mut cmd = configure_command(program, args, work_dir);
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Ok(RunOutcome::NotFound(program.to_string()));
        }
        Err(e) => return Err(LatexError::Io(format!("{program}: {e}"))),
    };

    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|e| LatexError::Io(e.to_string()))?;
                if output.status.success() {
                    return Ok(RunOutcome::Success);
                }
                let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
                let err_str = String::from_utf8_lossy(&output.stderr);
                if !err_str.trim().is_empty() {
                    if !combined.is_empty() {
                        combined.push('\n');
                    }
                    combined.push_str(&err_str);
                }
                return Ok(RunOutcome::Failed {
                    exit_code: output.status.code(),
                    output: combined,
                });
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Ok(RunOutcome::TimedOut);
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                thread::sleep(remaining.min(Duration::from_millis(25)));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(LatexError::Io(format!("{program}: {e}")));
            }
        }
    }
}

/// Rút gọn log lỗi LaTeX thành các dòng `! ...` và ngữ cảnh ngắn gọn dễ đọc.
fn extract_tex_error(raw: &str, log_path: &Path) -> String {
    let log_text = fs::read_to_string(log_path).unwrap_or_default();
    let haystack = if raw.contains("\n!") || raw.starts_with('!') {
        raw
    } else if !log_text.is_empty() {
        &log_text
    } else {
        raw
    };

    let lines: Vec<&str> = haystack.lines().collect();
    if let Some(pos) = lines.iter().position(|l| l.starts_with('!')) {
        let end = (pos + 4).min(lines.len());
        return lines[pos..end].join("\n");
    }

    let tail: Vec<&str> = lines
        .iter()
        .rev()
        .filter(|l| !l.trim().is_empty())
        .take(5)
        .copied()
        .collect();
    if tail.is_empty() {
        "Unknown LaTeX error.".to_string()
    } else {
        tail.into_iter().rev().collect::<Vec<_>>().join("\n")
    }
}

enum DviOutcome {
    Svg(String),
    MissingTool(String),
    Failed(LatexError),
}

fn compile_via_dvi(
    work_dir: &Path,
    tex_name: &str,
    dvi_name: &str,
    svg_name: &str,
    deadline: Instant,
    timeout: Duration,
) -> DviOutcome {
    let tex_args = [
        "-no-shell-escape",
        "-halt-on-error",
        "-interaction=nonstopmode",
        tex_name,
    ];
    match run_with_timeout("latex", &tex_args, work_dir, deadline) {
        Ok(RunOutcome::Success) => {}
        Ok(RunOutcome::TimedOut) => return DviOutcome::Failed(LatexError::Timeout(timeout)),
        Ok(RunOutcome::NotFound(tool)) => return DviOutcome::MissingTool(tool),
        Ok(RunOutcome::Failed { output, .. }) => {
            let msg = extract_tex_error(&output, &work_dir.join("formula.log"));
            return DviOutcome::Failed(LatexError::CompileFailed(msg));
        }
        Err(e) => return DviOutcome::Failed(e),
    }

    let dvisvgm_args = ["--no-fonts", "-o", svg_name, dvi_name];
    match run_with_timeout("dvisvgm", &dvisvgm_args, work_dir, deadline) {
        Ok(RunOutcome::Success) => {}
        Ok(RunOutcome::TimedOut) => return DviOutcome::Failed(LatexError::Timeout(timeout)),
        Ok(RunOutcome::NotFound(tool)) => return DviOutcome::MissingTool(tool),
        Ok(RunOutcome::Failed { output, exit_code }) => {
            let detail = if output.trim().is_empty() {
                format!("dvisvgm exited with {:?}", exit_code)
            } else {
                output.trim().to_string()
            };
            return DviOutcome::Failed(LatexError::SvgConversionFailed(detail));
        }
        Err(e) => return DviOutcome::Failed(e),
    }

    match fs::read_to_string(work_dir.join(svg_name)) {
        Ok(svg) => DviOutcome::Svg(svg),
        Err(e) => DviOutcome::Failed(LatexError::SvgConversionFailed(e.to_string())),
    }
}

fn compile_via_pdf_fallback(
    work_dir: &Path,
    tex_name: &str,
    pdf_name: &str,
    svg_name: &str,
    deadline: Instant,
    timeout: Duration,
) -> Result<String, LatexError> {
    let tex_args = [
        "-no-shell-escape",
        "-halt-on-error",
        "-interaction=nonstopmode",
        tex_name,
    ];
    match run_with_timeout("pdflatex", &tex_args, work_dir, deadline)? {
        RunOutcome::Success => {}
        RunOutcome::TimedOut => return Err(LatexError::Timeout(timeout)),
        RunOutcome::NotFound(tool) => return Err(LatexError::ToolNotFound(tool)),
        RunOutcome::Failed { output, .. } => {
            let msg = extract_tex_error(&output, &work_dir.join("formula.log"));
            return Err(LatexError::CompileFailed(msg));
        }
    }

    let cairo_args = ["-svg", pdf_name, svg_name];
    match run_with_timeout("pdftocairo", &cairo_args, work_dir, deadline)? {
        RunOutcome::Success => {}
        RunOutcome::TimedOut => return Err(LatexError::Timeout(timeout)),
        RunOutcome::NotFound(tool) => return Err(LatexError::ToolNotFound(tool)),
        RunOutcome::Failed { output, exit_code } => {
            let detail = if output.trim().is_empty() {
                format!("pdftocairo exited with {:?}", exit_code)
            } else {
                output.trim().to_string()
            };
            return Err(LatexError::SvgConversionFailed(detail));
        }
    }

    fs::read_to_string(work_dir.join(svg_name))
        .map_err(|e| LatexError::SvgConversionFailed(e.to_string()))
}

/// Biên dịch đoạn mã LaTeX thành chuỗi SVG vector.
/// Luôn chạy với `-no-shell-escape` và tự ngắt khi quá `opts.timeout`.
pub fn compile_latex_to_svg(source: &str, opts: &CompileOptions) -> Result<String, LatexError> {
    if source.trim().is_empty() {
        return Err(LatexError::EmptySource);
    }

    let work = TempWorkDir::new()?;
    let tex_name = "formula.tex";
    let dvi_name = "formula.dvi";
    let pdf_name = "formula.pdf";
    let svg_name = "formula.svg";

    let tex_content = build_tex_document(source, opts.color_rgb);
    fs::write(work.path.join(tex_name), tex_content).map_err(|e| LatexError::Io(e.to_string()))?;

    let deadline = Instant::now() + opts.timeout;

    match compile_via_dvi(
        &work.path,
        tex_name,
        dvi_name,
        svg_name,
        deadline,
        opts.timeout,
    ) {
        DviOutcome::Svg(svg) => Ok(svg),
        DviOutcome::Failed(LatexError::SvgConversionFailed(_)) if opts.allow_fallback => {
            compile_via_pdf_fallback(
                &work.path,
                tex_name,
                pdf_name,
                svg_name,
                deadline,
                opts.timeout,
            )
        }
        DviOutcome::Failed(err) => Err(err),
        DviOutcome::MissingTool(missing_tool) => {
            if opts.allow_fallback {
                compile_via_pdf_fallback(
                    &work.path,
                    tex_name,
                    pdf_name,
                    svg_name,
                    deadline,
                    opts.timeout,
                )
            } else {
                Err(LatexError::ToolNotFound(missing_tool))
            }
        }
    }
}

/// Rasterize chuỗi SVG thành buffer RGBA sắc nét theo hệ số `scale` (ví dụ `3.0` cho zoom 300%).
pub fn render_svg_to_rgba(svg: &str, scale: f32) -> Result<RenderedSvg, LatexError> {
    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(svg, &opt)
        .map_err(|e| LatexError::SvgParseFailed(e.to_string()))?;

    let size = tree.size();
    let logical_width = size.width().max(1.0);
    let logical_height = size.height().max(1.0);

    let clamped_scale = scale.clamp(0.25, 16.0);
    // Giới hạn kích thước bitmap tối đa 4096px mỗi chiều để tránh tốn RAM quá mức khi zoom cực đại
    let max_dim = 4096.0_f32;
    let fit_scale = (max_dim / logical_width)
        .min(max_dim / logical_height)
        .min(clamped_scale);

    let width_px = (logical_width * fit_scale).ceil().max(1.0) as u32;
    let height_px = (logical_height * fit_scale).ceil().max(1.0) as u32;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width_px, height_px)
        .ok_or_else(|| LatexError::SvgParseFailed("Failed to allocate pixmap".to_string()))?;

    let sx = width_px as f32 / logical_width;
    let sy = height_px as f32 / logical_height;
    let transform = resvg::tiny_skia::Transform::from_scale(sx, sy);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // resvg/tiny-skia dùng premultiplied RGBA; chuyển sang unmultiplied RGBA cho egui::ColorImage
    let mut rgba = pixmap.take();
    for px in rgba.chunks_exact_mut(4) {
        let a = px[3];
        if a > 0 && a < 255 {
            let af = a as f32 / 255.0;
            px[0] = ((px[0] as f32 / af).round()).min(255.0) as u8;
            px[1] = ((px[1] as f32 / af).round()).min(255.0) as u8;
            px[2] = ((px[2] as f32 / af).round()).min(255.0) as u8;
        }
    }

    Ok(RenderedSvg {
        width_px,
        height_px,
        rgba,
        logical_width,
        logical_height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_formula_and_vietnamese_to_svg_and_rasterize_3x() {
        let src = r"Công thức: $x=\dfrac{-b\pm\sqrt{b^2-4ac}}{2a}$ \quad Tiếng Việt";
        let svg = compile_latex_to_svg(src, &CompileOptions::default())
            .expect("LaTeX formula + Vietnamese should compile to SVG");
        assert!(svg.contains("<svg"), "output must be valid SVG");

        let img_1x = render_svg_to_rgba(&svg, 1.0).expect("1x rasterize");
        let img_3x = render_svg_to_rgba(&svg, 3.0).expect("3x rasterize");
        assert!(img_3x.width_px >= img_1x.width_px * 2);
        assert!(img_3x.height_px >= img_1x.height_px * 2);
        assert!(img_3x.rgba.chunks_exact(4).any(|px| px[3] > 0));
    }

    #[test]
    fn infinite_loop_is_terminated_on_timeout() {
        let src = r"\def\loop{\loop}\loop";
        let opts = CompileOptions {
            timeout: Duration::from_millis(800),
            ..Default::default()
        };
        let started = Instant::now();
        let res = compile_latex_to_svg(src, &opts);
        let elapsed = started.elapsed();

        assert!(
            matches!(res, Err(LatexError::Timeout(_))),
            "expected Timeout error, got: {res:?}"
        );
        assert!(
            elapsed < Duration::from_secs(4),
            "timeout should abort promptly, took {elapsed:?}"
        );
    }

    #[test]
    fn syntax_error_returns_readable_compile_failed() {
        let src = r"$\frac{1}{$";
        let res = compile_latex_to_svg(src, &CompileOptions::default());
        assert!(
            matches!(res, Err(LatexError::CompileFailed(_))),
            "expected CompileFailed, got: {res:?}"
        );
    }
}
