use std::path::PathBuf;

use clap::Parser;

use crate::args::{Args, Target};

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
