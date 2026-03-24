//! End-to-end integration tests for the `clutter` CLI.
//!
//! These tests call `clutter_cli::run()` directly (no subprocess) and verify:
//! - exit codes are correct
//! - output `.vue` files are written and have non-trivial content
//! - generated Vue SFCs contain the expected sections
//!
//! Counterpart unit tests for `run()` itself live in `src/tests.rs`.
//! These tests focus on the *observable output* (file content) rather than
//! just exit codes.

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/clutter-cli
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..").join("..")
}

fn fixture(name: &str) -> PathBuf {
    workspace_root().join("fixtures").join(format!("{name}.clutter"))
}

fn tokens_path() -> PathBuf {
    workspace_root().join("tokens.json")
}

fn temp_out(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clutter_integ_{label}"));
    std::fs::create_dir_all(&dir).expect("create integration temp dir");
    dir
}

fn run(parts: &[&str]) -> i32 {
    let argv: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
    clutter_cli::run(&argv)
}

// ── valid compilation produces readable Vue SFCs ───────────────────────────

#[test]
fn valid_file_produces_vue_sfc_with_required_sections() {
    let out = temp_out("vue_sfc_sections");
    let code = run(&[
        "clutter",
        fixture("valid").to_str().unwrap(),
        "--tokens", tokens_path().to_str().unwrap(),
        "--out", out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);

    let entries: Vec<_> = std::fs::read_dir(&out)
        .expect("read out dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "vue"))
        .collect();
    assert!(!entries.is_empty(), "expected at least one .vue file");

    for entry in entries {
        let content = std::fs::read_to_string(entry.path()).expect("read .vue file");
        assert!(!content.is_empty(), ".vue file should not be empty");
        assert!(content.contains("<template>"), "missing <template> section");
        assert!(content.contains("<script setup"), "missing <script setup> section");
        assert!(content.contains("<style"), "missing <style> section");
    }
}

#[test]
fn multi_component_produces_one_vue_file_per_component_with_correct_names() {
    let out = temp_out("multi_comp_names");
    let code = run(&[
        "clutter",
        fixture("multi_component").to_str().unwrap(),
        "--tokens", tokens_path().to_str().unwrap(),
        "--out", out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);

    let mut vue_files: Vec<_> = std::fs::read_dir(&out)
        .expect("read out dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |x| x == "vue"))
        .collect();
    vue_files.sort();
    assert_eq!(vue_files.len(), 2, "multi_component.clutter has 2 components");

    // Each file should have non-empty template content.
    for path in &vue_files {
        let content = std::fs::read_to_string(path).expect("read .vue file");
        assert!(content.contains("<template>"), "{} missing <template>", path.display());
    }
}

#[test]
fn compile_error_produces_no_output_files() {
    let out = temp_out("no_output_on_error");
    let code = run(&[
        "clutter",
        fixture("invalid_token").to_str().unwrap(),
        "--tokens", tokens_path().to_str().unwrap(),
        "--out", out.to_str().unwrap(),
    ]);
    assert_eq!(code, 1, "compile error should exit 1");

    // Out dir may or may not have been created, but no .vue files should exist.
    if out.exists() {
        let vue_files: Vec<_> = std::fs::read_dir(&out)
            .expect("read out dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |x| x == "vue"))
            .collect();
        assert!(vue_files.is_empty(), "no .vue files should be written on compile error");
    }
}

#[test]
fn out_directory_is_created_if_it_does_not_exist() {
    // Use a unique path (via nanos) guaranteed not to exist yet.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let out = std::env::temp_dir()
        .join(format!("clutter_mkdir_{nanos}"))
        .join("deep")
        .join("nested");
    assert!(!out.exists(), "precondition: freshly generated path should not exist");

    let code = run(&[
        "clutter",
        fixture("valid").to_str().unwrap(),
        "--tokens", tokens_path().to_str().unwrap(),
        "--out", out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    assert!(out.exists(), "output directory should have been created by compile()");
}

// ── output path defaults ───────────────────────────────────────────────────

#[test]
fn output_defaults_to_source_file_directory() {
    // Copy fixture to a temp dir; without --out, .vue files should appear alongside it.
    let dir = temp_out("default_out_dir");
    let source = dir.join("simple_component.clutter");
    std::fs::copy(fixture("simple_component"), &source).expect("copy fixture");

    let code = run(&[
        "clutter",
        source.to_str().unwrap(),
        "--tokens", tokens_path().to_str().unwrap(),
    ]);
    assert_eq!(code, 0);

    let vue_files: Vec<_> = std::fs::read_dir(&dir)
        .expect("read dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "vue"))
        .collect();
    assert!(!vue_files.is_empty(), ".vue file should be written next to the source file");
}
