use std::io::Write;
use std::path::Path;

use clutter_runtime::{Diagnostic, Position};

/// Formats a single diagnostic into a human-readable string.
///
/// Output format:
/// ```text
/// error[CLT102] path/to/file.clutter:3:10
///   invalid token value
/// ```
pub fn format_diagnostic(
    label: &str,
    path: &Path,
    code: &str,
    message: &str,
    pos: &Position,
) -> String {
    format!(
        "{label}[{code}] {}:{}:{}\n  {message}",
        path.display(),
        pos.line,
        pos.col
    )
}

/// Writes each diagnostic in `diagnostics` to `out`, one per line.
///
/// Uses `"error"` or `"warning"` as the label based on [`Diagnostic::is_error`].
/// Callers should pass `&mut std::io::stderr()` in production code; tests may
/// pass a `Vec<u8>` buffer.
pub fn print_diagnostics<D: Diagnostic>(diagnostics: &[D], path: &Path, out: &mut impl Write) {
    for d in diagnostics {
        let label = if d.is_error() { "error" } else { "warning" };
        let line = format_diagnostic(label, path, d.code(), d.message(), d.pos());
        writeln!(out, "{line}").expect("write diagnostic");
    }
}
