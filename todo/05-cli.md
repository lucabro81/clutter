# Block 5: CLI

Legend: `[ ]` todo · `[x]` done · `[-]` skipped/deferred

---

## Context

The CLI is the public face of the compiler. It reads `.clutter` files from disk,
runs the full pipeline, writes the generated `.vue` files to disk, and reports
diagnostics to stderr with clear formatting and a correct exit code.

The current `clutter-cli/src/main.rs` is a stub (`println!("Hello, world!")`).
This block builds it out end-to-end.

Pipeline:

```
clutter build <file.clutter>
  └─ read source from disk
  └─ discover tokens.json (walk up dir tree, or explicit --tokens path)
  └─ tokenize  (clutter-lexer)       → Vec<Token>       | LexErrors
  └─ parse     (clutter-parser)      → FileNode          | ParseErrors
  └─ analyze   (clutter-analyzer)    → FileNode (valid)  | AnalyzerErrors + Warnings
  └─ codegen   (clutter-codegen)     → Vec<GeneratedFile>
  └─ write .vue files to --out dir (default: same dir as source)
```

---

## Architecture decisions

### Library split

`main.rs` must stay minimal so the core logic is unit-testable without spawning a
process. Extract a `lib.rs` with a `pub fn run(args: &[String]) -> ExitCode`
function. `main.rs` calls `process::exit(run(&env::args().collect()))`.

Integration tests can call `run()` directly with fake args and a temp dir, or
spawn the binary via `std::process::Command` for full end-to-end tests.

### tokens.json discovery

Two modes:
1. **Auto-discovery** (default): walk up the directory tree from the source file,
   looking for `tokens.json`. Stop at the filesystem root. Emit an error if not
   found.
2. **Explicit path**: `--tokens <path>` overrides discovery entirely.

This mirrors the convention used by tools like `eslint` (`.eslintrc` discovery)
and `cargo` (`Cargo.toml` discovery). The internal compiler fixture file
(`tests/` path) is for compiler tests only and is never consulted by the CLI.

### Error reporting (minimal miette for POC)

Full `miette::Diagnostic` implementation on each error type requires threading
`SourceCode` through every diagnostic — non-trivial and deferred (see backlog).

For the POC, format errors in the CLI as:

```
error[CLT102] src/Card.clutter:4:12
  invalid value 'xl2' for prop 'gap' on 'Column'
  valid values: xs, sm, md, lg, xl, xxl
```

Use `eprintln!` + structured string formatting. Print all errors before exiting.
Use `miette::Report` only as the top-level wrapper for the exit message.

Warnings are printed to stderr with a `warning[W001]` prefix and do not affect
the exit code.

### Exit codes

| Situation | Exit code |
|-----------|-----------|
| Success (0 or more warnings) | `0` |
| Any lex / parse / analyzer error | `1` |
| I/O error (file not found, can't write) | `1` |
| Missing required argument | `2` (clap default) |

---

## clutter-cli — crate setup

- [ ] Add `lib.rs` alongside `main.rs`; move logic into `pub fn run(args: &[String]) -> i32`
- [ ] `main.rs` calls `std::process::exit(run(&std::env::args().collect::<Vec<_>>()))`
- [ ] Add dependencies to `Cargo.toml`:
  - `clutter-lexer`, `clutter-parser`, `clutter-analyzer`, `clutter-codegen`
  - `clap` (already listed as workspace dep)
  - `miette` (already listed as workspace dep)
- [ ] Add `src/tokens_discovery.rs` submodule
- [ ] Add `src/error_reporter.rs` submodule
- [ ] `#[cfg(test)] mod tests;` in `lib.rs`; create `src/tests.rs`

---

## clutter-cli — argument parsing: tests (written BEFORE implementation)

- [ ] `clutter build` with no file argument → exit code non-zero (clap handles this)
- [ ] `clutter build src/Card.clutter` → `args.file` is `"src/Card.clutter"`
- [ ] `clutter build src/Card.clutter --out dist/` → `args.out` is `Some("dist/")`
- [ ] `clutter build src/Card.clutter --tokens design/tokens.json` → `args.tokens` is `Some("design/tokens.json")`
- [ ] `--target vue` (default) → `args.target` is `Target::Vue`
- [ ] `--target html` → `args.target` is `Target::Html` *(target parsed but html output deferred — see below)*

## clutter-cli — argument parsing: implementation

- [ ] `clap` struct: `struct Cli { file: PathBuf, #[arg(long)] out: Option<PathBuf>, #[arg(long)] tokens: Option<PathBuf>, #[arg(long, default_value = "vue")] target: Target }`
- [ ] `enum Target { Vue, Html }` with `clap::ValueEnum`
- [ ] `--html` target: parse OK, but `run()` returns error "html target not yet implemented"

---

## clutter-cli — tokens.json discovery: tests (written BEFORE implementation)

- [ ] File in same directory as source → `discover_tokens_json` returns that path
- [ ] File in parent directory → returns parent path
- [ ] File in grandparent directory → returns grandparent path
- [ ] No `tokens.json` anywhere in tree → returns `Err` with a descriptive message
- [ ] Explicit `--tokens` path that exists → used directly, no discovery
- [ ] Explicit `--tokens` path that does not exist → `Err` with descriptive message

## clutter-cli — tokens.json discovery: implementation

- [ ] `fn discover_tokens_json(source: &Path) -> Result<PathBuf, String>`
  - Start from `source.parent()`, loop with `.parent()` until root
  - Return `Ok(path)` on first `tokens.json` found
  - Return `Err("tokens.json not found...")` if root reached
- [ ] `fn load_tokens(explicit: Option<&Path>, source: &Path) -> Result<DesignTokens, String>`
  - If explicit is `Some` → load from that path
  - Else → `discover_tokens_json(source)` then load
  - Parse via `DesignTokens::from_str(&content)`

---

## clutter-cli — pipeline orchestration: tests (written BEFORE implementation)

Use a temp directory with real `.clutter` and `tokens.json` files.

- [ ] Valid `.clutter` file → `run()` returns `0`, expected `.vue` files exist on disk
- [ ] Multi-component file → one `.vue` per component, all written to `--out` dir
- [ ] Lex error in source → `run()` returns `1`, stderr contains error code `L001` or `L002`
- [ ] Parse error in source → `run()` returns `1`, stderr contains `P00x` code
- [ ] Analyzer error in source → `run()` returns `1`, stderr contains `CLT10x` code
- [ ] Analyzer warning only → `run()` returns `0`, stderr contains `W00x` code
- [ ] Source file does not exist → `run()` returns `1`, stderr mentions the path
- [ ] Multiple errors → all errors printed, not just the first

## clutter-cli — pipeline orchestration: implementation

- [ ] `fn compile(source: &Path, tokens: &DesignTokens, out_dir: &Path) -> Result<Vec<PathBuf>, ()>`
  1. Read source to `String` (I/O error → print + return `Err`)
  2. `tokenize(&src)` → on lex errors, print all and return `Err`
  3. `Parser::new(tokens).parse_file()` → on parse errors, print all and return `Err`
  4. `analyze_file(&file, tokens)` → on analyzer errors, print all and return `Err`; print warnings regardless
  5. `generate_vue(&file, tokens)` → write each `GeneratedFile` to `out_dir/{name}.vue`
  6. Return `Ok(written_paths)`
- [ ] `fn run(args: &[String]) -> i32` — wires arg parsing + `load_tokens` + `compile` + exit code

---

## clutter-cli — error reporter: tests (written BEFORE implementation)

- [ ] `format_lex_error(path, err)` → string contains path, line, col, code, message
- [ ] `format_parse_error(path, err)` → same fields
- [ ] `format_analyzer_error(path, err)` → same fields
- [ ] `format_analyzer_warning(path, warn)` → contains `warning[W00x]`, path, line, col
- [ ] Error string is printed to stderr (not stdout) — verify via `run()` test capturing stderr

## clutter-cli — error reporter: implementation

- [ ] `fn format_diagnostic(label: &str, path: &Path, code: &str, message: &str, pos: &Position) -> String`
  - `"{label}[{code}] {path}:{line}:{col}\n  {message}"`
- [ ] `fn print_lex_errors(path: &Path, errors: &[LexError])`
- [ ] `fn print_parse_errors(path: &Path, errors: &[ParseError])`
- [ ] `fn print_analyzer_errors(path: &Path, errors: &[AnalyzerError])`
- [ ] `fn print_analyzer_warnings(path: &Path, warnings: &[AnalyzerWarning])`

---

## Integration tests (`tests/integration.rs`)

Full end-to-end: spawn binary via `std::process::Command` or call `run()` directly.

- [ ] `valid.clutter` → exit 0, `MainComponent.vue` written to out dir
- [ ] Multi-component fixture → exit 0, two `.vue` files written
- [ ] `invalid_token.clutter` → exit 1, stderr contains `CLT102`
- [ ] `orphan_else.clutter` → exit 1, stderr contains `P002`
- [ ] Missing `tokens.json` (no file in tree) → exit 1, stderr mentions `tokens.json`
- [ ] Non-existent source file → exit 1, stderr mentions the path
- [ ] Warnings-only file (`unsafe_block.clutter`) → exit 0, stderr contains `W001`

---

## Final check

- [ ] `cargo test` — full workspace green
- [ ] `cargo build --workspace` — zero warnings
- [ ] `cargo build --release` — binary compiles cleanly
- [ ] Manual smoke test: `./target/release/clutter build fixtures/valid.clutter --out /tmp/out` → `MainComponent.vue` created, parseable by Vue tools
- [ ] Mark `Block 5: CLI` row in `CLAUDE.md` status table as ✅ complete
- [ ] Update `todo/00-backlog.md` — review items resolved incidentally
