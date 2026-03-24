pub mod args;
pub mod error_reporter;
pub mod tokens_discovery;

use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;

use clutter_runtime::DesignTokens;

use args::{Args, Target};
use error_reporter::print_diagnostics;
use tokens_discovery::load_tokens;

/// Runs the full compiler pipeline on `source`, writing output files to `out_dir`.
///
/// All diagnostic messages (errors and warnings) are written to `err_out`.
/// Returns `Ok(written_paths)` on success or `Err(())` if compilation failed.
pub fn compile(
    source: &Path,
    tokens: &DesignTokens,
    out_dir: &Path,
    err_out: &mut impl Write,
) -> Result<Vec<PathBuf>, ()> {
    let src = std::fs::read_to_string(source).map_err(|e| {
        writeln!(err_out, "error: cannot read '{}': {e}", source.display()).ok();
    })?;

    let (tok, lex_errors) = clutter_lexer::tokenize(&src);
    if !lex_errors.is_empty() {
        print_diagnostics(&lex_errors, source, err_out);
        return Err(());
    }

    let (file, parse_errors) = clutter_parser::Parser::new(tok).parse_file();
    if !parse_errors.is_empty() {
        print_diagnostics(&parse_errors, source, err_out);
        return Err(());
    }

    let (analyzer_errors, warnings) = clutter_analyzer::analyze_file(&file, tokens);
    print_diagnostics(&warnings, source, err_out);
    if !analyzer_errors.is_empty() {
        print_diagnostics(&analyzer_errors, source, err_out);
        return Err(());
    }

    let generated = clutter_codegen::generate_vue(&file);
    std::fs::create_dir_all(out_dir).map_err(|e| {
        writeln!(err_out, "error: cannot create output dir '{}': {e}", out_dir.display()).ok();
    })?;

    let mut written = Vec::new();
    for output in generated {
        let path = out_dir.join(format!("{}.vue", output.name));
        std::fs::write(&path, &output.content).map_err(|e| {
            writeln!(err_out, "error: cannot write '{}': {e}", path.display()).ok();
        })?;
        written.push(path);
    }

    let css_path = out_dir.join("clutter.css");
    let css_content = clutter_codegen::generate_css(tokens);
    std::fs::write(&css_path, &css_content).map_err(|e| {
        writeln!(err_out, "error: cannot write '{}': {e}", css_path.display()).ok();
    })?;
    written.push(css_path);

    Ok(written)
}

/// Entry point for the CLI. Parses `argv`, runs the pipeline, returns an exit code.
///
/// Exit codes:
/// - `0` — success (warnings are printed but do not fail the build)
/// - `1` — compile error, I/O error, or unsupported target
/// - `2` — invalid/missing CLI arguments
pub fn run(argv: &[String]) -> i32 {
    let args = match Args::try_parse_from(argv) {
        Ok(a) => a,
        Err(e) => {
            // --help and --version are expected exits, not errors.
            if e.kind() == clap::error::ErrorKind::DisplayHelp
                || e.kind() == clap::error::ErrorKind::DisplayVersion
            {
                print!("{e}");
                return 0;
            }
            eprint!("{e}");
            return 2;
        }
    };

    if matches!(args.target, Target::Html) {
        eprintln!("error: html target is not yet implemented");
        return 1;
    }

    let tokens = match load_tokens(args.tokens.as_deref(), &args.file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    let out_dir = args
        .out
        .as_deref()
        .or_else(|| args.file.parent())
        .unwrap_or(Path::new("."))
        .to_path_buf();

    match compile(&args.file, &tokens, &out_dir, &mut std::io::stderr()) {
        Ok(_) => 0,
        Err(()) => 1,
    }
}

#[cfg(test)]
mod tests;
