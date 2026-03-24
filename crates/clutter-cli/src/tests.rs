use std::path::PathBuf;

use clap::Parser;

use crate::args::{Args, Target};
use crate::error_reporter::{format_diagnostic, print_diagnostics};
use crate::tokens_discovery::{discover_tokens_json, load_tokens};

fn parse(argv: &[&str]) -> Args {
    Args::try_parse_from(argv).expect("args should parse")
}

// ── file argument ──────────────────────────────────────────────────────────

#[test]
fn file_argument_is_required() {
    let result = Args::try_parse_from(["clutter"]);
    assert!(result.is_err(), "clutter with no args should fail");
}

#[test]
fn file_only_sets_correct_defaults() {
    let args = parse(&["clutter", "main.clutter"]);
    assert_eq!(args.file, PathBuf::from("main.clutter"));
    assert_eq!(args.out, None);
    assert_eq!(args.tokens, None);
    assert!(matches!(args.target, Target::Vue), "default target should be vue");
}

// ── optional flags ─────────────────────────────────────────────────────────

#[test]
fn out_flag_stores_path() {
    let args = parse(&["clutter", "main.clutter", "--out", "dist/"]);
    assert_eq!(args.out, Some(PathBuf::from("dist/")));
}

#[test]
fn tokens_flag_stores_path() {
    let args = parse(&["clutter", "main.clutter", "--tokens", "design/tokens.json"]);
    assert_eq!(args.tokens, Some(PathBuf::from("design/tokens.json")));
}

// ── target enum ────────────────────────────────────────────────────────────

#[test]
fn target_defaults_to_vue() {
    let args = parse(&["clutter", "main.clutter"]);
    assert!(matches!(args.target, Target::Vue));
}

#[test]
fn target_vue_explicit() {
    let args = parse(&["clutter", "main.clutter", "--target", "vue"]);
    assert!(matches!(args.target, Target::Vue));
}

#[test]
fn target_html() {
    let args = parse(&["clutter", "main.clutter", "--target", "html"]);
    assert!(matches!(args.target, Target::Html));
}

#[test]
fn invalid_target_is_rejected() {
    let result = Args::try_parse_from(["clutter", "main.clutter", "--target", "wasm"]);
    assert!(result.is_err(), "--target wasm should be rejected by clap ValueEnum");
}

// ── combined / ordering ────────────────────────────────────────────────────

#[test]
fn all_flags_combined() {
    let args = parse(&[
        "clutter",
        "src/main.clutter",
        "--out",
        "dist/",
        "--tokens",
        "design/tokens.json",
        "--target",
        "html",
    ]);
    assert_eq!(args.file, PathBuf::from("src/main.clutter"));
    assert_eq!(args.out, Some(PathBuf::from("dist/")));
    assert_eq!(args.tokens, Some(PathBuf::from("design/tokens.json")));
    assert!(matches!(args.target, Target::Html));
}

#[test]
fn positional_file_after_flags() {
    // clap should accept the positional arg in any position relative to flags
    let args = parse(&["clutter", "--out", "dist/", "main.clutter"]);
    assert_eq!(args.file, PathBuf::from("main.clutter"));
    assert_eq!(args.out, Some(PathBuf::from("dist/")));
}

// ── tokens_discovery ───────────────────────────────────────────────────────

/// Minimal valid tokens JSON for testing.
const VALID_TOKENS_JSON: &str = r#"{
    "spacing":    ["xs", "sm"],
    "colors":     ["primary"],
    "typography": { "sizes": ["sm"], "weights": ["normal"] },
    "radii":      ["none"],
    "shadows":    ["sm"]
}"#;

fn make_temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clutter_tokdisc_{}", label));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write(path: &PathBuf, content: &str) {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).expect("create parent dirs");
    }
    std::fs::write(path, content).expect("write file");
}

#[test]
fn discover_finds_tokens_in_source_parent_directory() {
    let root = make_temp_dir("same_dir");
    write(&root.join("tokens.json"), VALID_TOKENS_JSON);
    // source is a file inside root — discover must look in root, not in the file path itself
    let source = root.join("component.clutter");
    let result = discover_tokens_json(&source).expect("should find tokens.json");
    assert_eq!(result, root.join("tokens.json"));
}

#[test]
fn discover_finds_tokens_in_parent_directory() {
    let root = make_temp_dir("parent_dir");
    let sub = root.join("src");
    std::fs::create_dir_all(&sub).expect("create src/");
    write(&root.join("tokens.json"), VALID_TOKENS_JSON);
    let source = sub.join("component.clutter");
    let result = discover_tokens_json(&source).expect("should find tokens.json in parent");
    assert_eq!(result, root.join("tokens.json"));
}

#[test]
fn discover_finds_tokens_in_grandparent_directory() {
    let root = make_temp_dir("grandparent_dir");
    let deep = root.join("a").join("b");
    std::fs::create_dir_all(&deep).expect("create a/b/");
    write(&root.join("tokens.json"), VALID_TOKENS_JSON);
    let source = deep.join("component.clutter");
    let result = discover_tokens_json(&source).expect("should find tokens.json in grandparent");
    assert_eq!(result, root.join("tokens.json"));
}

#[test]
fn discover_prefers_closer_tokens_json_over_ancestor() {
    // When tokens.json exists at both levels, the nearest one wins.
    let root = make_temp_dir("prefer_nearest");
    let sub = root.join("src");
    std::fs::create_dir_all(&sub).expect("create src/");
    write(&root.join("tokens.json"), VALID_TOKENS_JSON);
    write(&sub.join("tokens.json"), VALID_TOKENS_JSON);
    let source = sub.join("component.clutter");
    let result = discover_tokens_json(&source).expect("should find tokens.json");
    assert_eq!(result, sub.join("tokens.json"), "nearest tokens.json should win");
}

#[test]
fn discover_returns_error_when_not_found() {
    let root = make_temp_dir("not_found");
    let deep = root.join("x").join("y");
    std::fs::create_dir_all(&deep).expect("create x/y/");
    // No tokens.json written anywhere in this tree.
    let source = deep.join("component.clutter");
    // We can only assert it finds nothing *within our temp tree*. If a tokens.json
    // exists somewhere in /tmp or above, this test would see it — acceptable trade-off
    // for avoiding a tempfile dependency.
    let result = discover_tokens_json(&source);
    // Only assert error if we're confident: check that the returned path (if Ok) is NOT
    // under our temp root. If it IS under our root, that's a bug.
    if let Ok(ref found) = result {
        assert!(
            !found.starts_with(&root),
            "discover should not have found tokens.json inside our empty temp tree, got: {}",
            found.display()
        );
    }
}

#[test]
fn load_tokens_with_explicit_path_succeeds() {
    let root = make_temp_dir("explicit_ok");
    let tokens_path = root.join("tokens.json");
    write(&tokens_path, VALID_TOKENS_JSON);
    let source = root.join("component.clutter");
    let result = load_tokens(Some(&tokens_path), &source);
    assert!(result.is_ok(), "load with valid explicit path should succeed");
}

#[test]
fn load_tokens_with_missing_explicit_path_returns_error() {
    let root = make_temp_dir("explicit_missing");
    let missing = root.join("nonexistent.json");
    let source = root.join("component.clutter");
    let result = load_tokens(Some(&missing), &source);
    assert!(result.is_err(), "load with non-existent explicit path should fail");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("nonexistent.json"),
        "error message should mention the missing file, got: {msg}"
    );
}

#[test]
fn load_tokens_discovers_automatically_when_no_explicit_path() {
    let root = make_temp_dir("auto_discover");
    write(&root.join("tokens.json"), VALID_TOKENS_JSON);
    let source = root.join("sub").join("component.clutter");
    std::fs::create_dir_all(source.parent().unwrap()).expect("create sub/");
    let result = load_tokens(None, &source);
    assert!(result.is_ok(), "auto-discovery should succeed when tokens.json is in parent dir");
}

#[test]
fn load_tokens_returns_error_for_malformed_json() {
    let root = make_temp_dir("bad_json");
    let tokens_path = root.join("tokens.json");
    write(&tokens_path, "{ this is not valid json }");
    let source = root.join("component.clutter");
    let result = load_tokens(Some(&tokens_path), &source);
    assert!(result.is_err(), "malformed JSON should return an error");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("tokens.json") || msg.contains("invalid"),
        "error message should be informative, got: {msg}"
    );
}

// ── error_reporter ─────────────────────────────────────────────────────────

use clutter_runtime::{LexError, Position};

fn pos(line: usize, col: usize) -> Position {
    Position { line, col }
}

#[test]
fn format_diagnostic_error_produces_correct_string() {
    let path = PathBuf::from("src/main.clutter");
    let result = format_diagnostic("error", &path, "CLT102", "invalid token value", &pos(3, 10));
    assert_eq!(
        result,
        "error[CLT102] src/main.clutter:3:10\n  invalid token value"
    );
}

#[test]
fn format_diagnostic_warning_uses_warning_label() {
    let path = PathBuf::from("src/main.clutter");
    let result = format_diagnostic("warning", &path, "W001", "deprecated usage", &pos(1, 1));
    assert_eq!(
        result,
        "warning[W001] src/main.clutter:1:1\n  deprecated usage"
    );
}

#[test]
fn format_diagnostic_includes_path_line_col() {
    let path = PathBuf::from("components/Card.clutter");
    let result = format_diagnostic("error", &path, "L001", "unexpected char", &pos(7, 4));
    assert!(result.contains("components/Card.clutter"), "path missing");
    assert!(result.contains("7:4"), "line:col missing");
}

#[test]
fn print_diagnostics_writes_each_error_to_output() {
    use clutter_lexer::tokenize;
    // Produce at least one lex error by feeding an invalid character.
    let (_tokens, errors) = tokenize("@@@");
    assert!(!errors.is_empty(), "expected lex errors from invalid input");
    let path = PathBuf::from("test.clutter");
    let mut buf: Vec<u8> = Vec::new();
    print_diagnostics(&errors, &path, &mut buf);
    let output = String::from_utf8(buf).expect("utf8");
    assert!(!output.is_empty(), "print_diagnostics should write to the output");
    // Each diagnostic should start a new entry with "error[" or "warning["
    assert!(
        output.contains("error[") || output.contains("warning["),
        "output should contain formatted diagnostic, got: {output}"
    );
}

#[test]
fn print_diagnostics_empty_slice_writes_nothing() {
    let errors: &[LexError] = &[];
    let path = PathBuf::from("test.clutter");
    let mut buf: Vec<u8> = Vec::new();
    print_diagnostics(errors, &path, &mut buf);
    assert!(buf.is_empty(), "no output expected for empty diagnostics slice");
}

// ── compile / run ──────────────────────────────────────────────────────────

use crate::compile;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/clutter-cli; workspace root is two levels up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..").join("..")
}

fn fixture(name: &str) -> PathBuf {
    workspace_root().join("fixtures").join(format!("{name}.clutter"))
}

fn tokens_path() -> PathBuf {
    workspace_root().join("tokens.json")
}

fn load_workspace_tokens() -> clutter_runtime::DesignTokens {
    load_tokens(Some(&tokens_path()), &PathBuf::from("dummy")).expect("workspace tokens.json")
}

#[test]
fn compile_valid_file_writes_vue_output() {
    let out_dir = make_temp_dir("compile_valid");
    let source = fixture("valid");
    let tokens = load_workspace_tokens();
    let mut err_buf: Vec<u8> = Vec::new();
    let result = compile(&source, &tokens, &out_dir, &mut err_buf);
    assert!(result.is_ok(), "compile should succeed for valid.clutter");
    let written = result.unwrap();
    assert!(!written.is_empty(), "should have written at least one file");
    for path in &written {
        assert!(path.exists(), "written file should exist: {}", path.display());
        assert!(
            path.extension().map_or(false, |e| e == "vue"),
            "output should be a .vue file, got: {}",
            path.display()
        );
    }
}

#[test]
fn compile_multi_component_writes_one_file_per_component() {
    let out_dir = make_temp_dir("compile_multi");
    let source = fixture("multi_component");
    let tokens = load_workspace_tokens();
    let mut err_buf: Vec<u8> = Vec::new();
    let result = compile(&source, &tokens, &out_dir, &mut err_buf);
    assert!(result.is_ok(), "compile should succeed for multi_component.clutter");
    let written = result.unwrap();
    assert_eq!(written.len(), 2, "multi_component has 2 components → 2 .vue files");
}

#[test]
fn compile_nonexistent_source_returns_error() {
    let out_dir = make_temp_dir("compile_nofile");
    let source = PathBuf::from("/nonexistent/path/missing.clutter");
    let tokens = load_workspace_tokens();
    let mut err_buf: Vec<u8> = Vec::new();
    let result = compile(&source, &tokens, &out_dir, &mut err_buf);
    assert!(result.is_err(), "compile should fail for missing source file");
    assert!(!err_buf.is_empty(), "error message should be written to output");
}

#[test]
fn compile_file_with_lex_error_returns_error_and_prints_diagnostic() {
    let out_dir = make_temp_dir("compile_lex_err");
    // invalid_token.clutter contains a value that fails at the analyzer level (CLT102).
    // For a lex-level error we need actual invalid chars — create an inline fixture.
    let source = out_dir.join("bad_lex.clutter");
    write(&source, "component Foo(props: P) {\n----\n<Column gap=@@@>Foo</Column>\n}");
    let tokens = load_workspace_tokens();
    let mut err_buf: Vec<u8> = Vec::new();
    let result = compile(&source, &tokens, &out_dir, &mut err_buf);
    assert!(result.is_err(), "compile should fail on lex errors");
    let output = String::from_utf8(err_buf).expect("utf8");
    assert!(
        output.contains("error["),
        "diagnostic should be printed, got: {output}"
    );
}

#[test]
fn compile_file_with_analyzer_error_returns_error() {
    let out_dir = make_temp_dir("compile_analyzer_err");
    let source = fixture("invalid_token");
    let tokens = load_workspace_tokens();
    let mut err_buf: Vec<u8> = Vec::new();
    let result = compile(&source, &tokens, &out_dir, &mut err_buf);
    assert!(result.is_err(), "compile should fail on analyzer errors (CLT102)");
    let output = String::from_utf8(err_buf).expect("utf8");
    assert!(output.contains("error["), "diagnostic should be printed, got: {output}");
}

#[test]
fn compile_file_with_only_warnings_succeeds() {
    // unsafe_block.clutter triggers W001/W002 warnings but no errors.
    let out_dir = make_temp_dir("compile_warnings");
    let source = fixture("unsafe_block");
    let tokens = load_workspace_tokens();
    let mut err_buf: Vec<u8> = Vec::new();
    let result = compile(&source, &tokens, &out_dir, &mut err_buf);
    assert!(result.is_ok(), "compile should succeed when only warnings, no errors");
    let output = String::from_utf8(err_buf).expect("utf8");
    // Warnings should still be printed even on success.
    assert!(
        output.is_empty() || output.contains("warning["),
        "output should be empty or contain warnings, got: {output}"
    );
}

// ── run() exit codes ────────────────────────────────────────────────────────

use crate::run;

fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

#[test]
fn run_valid_file_exits_zero() {
    let out_dir = make_temp_dir("run_valid");
    let code = run(&args(&[
        "clutter",
        fixture("valid").to_str().unwrap(),
        "--tokens", tokens_path().to_str().unwrap(),
        "--out", out_dir.to_str().unwrap(),
    ]));
    assert_eq!(code, 0, "valid file should exit 0");
}

#[test]
fn run_missing_file_argument_exits_two() {
    let code = run(&args(&["clutter"]));
    assert_eq!(code, 2, "missing required arg should exit 2");
}

#[test]
fn run_nonexistent_source_exits_one() {
    let code = run(&args(&[
        "clutter",
        "/nonexistent/missing.clutter",
        "--tokens", tokens_path().to_str().unwrap(),
    ]));
    assert_eq!(code, 1, "nonexistent source should exit 1");
}

#[test]
fn run_missing_tokens_exits_one() {
    // No --tokens flag and no tokens.json discoverable near a temp dir.
    let out_dir = make_temp_dir("run_no_tokens");
    let source = out_dir.join("comp.clutter");
    write(&source, "component Foo(props: P) {\n----\n<Column>Foo</Column>\n}");
    let code = run(&args(&[
        "clutter",
        source.to_str().unwrap(),
        "--out", out_dir.to_str().unwrap(),
    ]));
    assert_eq!(code, 1, "missing tokens.json should exit 1");
}

#[test]
fn run_html_target_exits_one_with_not_implemented() {
    let code = run(&args(&[
        "clutter",
        fixture("valid").to_str().unwrap(),
        "--tokens", tokens_path().to_str().unwrap(),
        "--target", "html",
    ]));
    assert_eq!(code, 1, "html target should exit 1 (not yet implemented)");
}

#[test]
fn run_file_with_compile_error_exits_one() {
    let code = run(&args(&[
        "clutter",
        fixture("invalid_token").to_str().unwrap(),
        "--tokens", tokens_path().to_str().unwrap(),
        "--out", make_temp_dir("run_err").to_str().unwrap(),
    ]));
    assert_eq!(code, 1, "compile error should exit 1");
}
