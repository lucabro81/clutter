# Clutter — CLAUDE.md

## Project

Rust compiler for `.clutter`, a UI markup language with a closed vocabulary that enforces design system compliance at compile time. Output targets: Vue SFC, vanilla JS/HTML.

## Pipeline

```
.clutter file → Lexer → Parser → Analyzer → Codegen → Output
                                     ↑
                               tokens.json (design system)
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
- Template props only accept values present in `tokens.json`
- See `ARCHITECTURE.md` for the full architecture decision record
- See `docs/language.md` for the full language reference

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

## Architecture decisions

- **Global CSS**: `clutter.css` is a single global file (no `<style scoped>` per SFC). Closed vocabulary means no component-specific CSS exists — every rule is a utility class, Tailwind-style.
- **CSS variables**: `tokens.json` accepts an optional `"variables"` key mapping CSS custom property names to values. The compiler emits a `:root { }` block at the top of `clutter.css`. Convention: `--{category}-{value}` (e.g. `--spacing-md`, `--color-primary`).
- **Distribution**: GitHub Actions builds binaries on tag `v*` (macOS arm64 + Linux x86_64). A generated `setup.sh` installs the binary, scaffolds a Vue + Vite project, compiles the sample `.clutter` file, and runs `npm install`.
- **Template layout**: `.clutter` sources live in `src/clutter/`, generated Vue components in `src/components/`.
