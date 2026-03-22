# Clutter — CLAUDE.md

## Project

Rust compiler for `.clutter`, a UI markup language with a closed vocabulary that enforces design system compliance at compile time. Output targets: Vue SFC, vanilla JS/HTML.

## Pipeline

```
.clutter file → Lexer → Parser → Analyzer → Codegen → Output
                                     ↑
                               tokens.clutter (design system)
```

Crate map: `clutter-cli` → `clutter-codegen` → `clutter-analyzer` → `clutter-parser` → `clutter-lexer`, all depending on `clutter-runtime` (shared types).

## .clutter file format

```
component MainComponent(props: MainProps) {
    [logic section — TypeScript, treated as opaque by the compiler]
    ----
    [template — JSX-like syntax, closed vocabulary only]
}

component Card(props: CardProps) {
    [logic section]
    ----
    [template]
}
```

- Every component — including the root — is wrapped in `component Name(...) { }`
- `----` (4 dashes) separates logic from template inside each block
- Props signature is opaque TypeScript; compiler does not parse it
- Template props only accept values present in `tokens.clutter` (JSON)
- See `design-doc/clutter-block4a.md` for the full architecture decision record

## Key dependencies

- `clap` 4 — CLI · `miette` 5 — error reporting · `serde_json` 1 — token parsing · `typed-arena` 2 — AST

## TDD

Tests-first. Unit tests live in `src/tests.rs` per crate (`#[cfg(test)] mod tests;` declaration in `lib.rs`). Integration tests in `tests/` use real `.clutter` files from `fixtures/`.

## Todo hygiene

When a todo item is completed during a session, check it off in the relevant `todo/0N-*.md` file **before moving on** — don't batch updates at the end.

After every significant addition, review `todo/00-backlog.md`: some items may have been resolved incidentally (mark or remove them), and new ideas may be worth adding.

## Useful commands

```bash
cargo test -p clutter-lexer  # single crate
cargo test                   # full workspace
cargo check                  # type check only
```

## Current status

Architecture decisions for Block 4 are now locked — see `design-doc/clutter-block4a.md`.

Next immediate steps:
1. **Format migration** — rewrite all fixtures and update lexer/parser/runtime AST types to the new multi-component format (`component Name(...) { }` + `----`)
2. **VocabularyMap** — refactor `clutter-analyzer` internals (replaces `KNOWN_COMPONENTS` + `prop_map`)
3. **Block 4: Codegen** — implement code generator once migration is complete

Completed backlog: unsafe validation (CLT105–107), structured error codes (`clutter-runtime::codes`), `Diagnostic` trait + `DiagnosticCollector` in `clutter-runtime::diagnostics`, `clutter-runtime` split into focused modules. Review `todo/00-backlog.md` for remaining items.

| Block | Status |
|-------|--------|
| Block 1: Lexer   | ✅ complete |
| Block 2: Parser  | ✅ complete |
| Block 3: Analyzer| ✅ complete |
| Block 4: Codegen | ⬜ todo |
| Block 5: CLI     | ⬜ todo |
