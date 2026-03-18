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
[logic section — TypeScript, treated as opaque by the compiler]

---

[template — JSX-like syntax, closed vocabulary only]
```

- `---` separator required even if logic section is empty
- Template props only accept values present in `tokens.clutter` (JSON)

## Key dependencies

- `clap` 4 — CLI · `miette` 5 — error reporting · `serde_json` 1 — token parsing · `typed-arena` 2 — AST

## TDD

Tests-first. Every crate has an internal `#[cfg(test)]` module. Integration tests in `tests/` use real `.clutter` files from `fixtures/`.

## Useful commands

```bash
cargo test -p clutter-lexer  # single crate
cargo test                   # full workspace
cargo check                  # type check only
```

## Current status

Working on **Block 1: Lexer**. Resume from `todo/01-lexer.md`.
