# Backlog — cross-cutting improvements

Ideas that emerged during development and do not belong to a specific block.
To be addressed when the context is mature, not necessarily in order.

---

## Before Block 4

| Item | Detail |
|------|--------|
| ~~Dynamic prop map / custom components~~ | ✅ Decided — see `design-doc/clutter-block4a.md`. Multi-component format (`component Name(...) { }` + `----`), `VocabularyMap` replaces `KNOWN_COMPONENTS` + `prop_map`, `ComponentOpen` token opaque. Custom components recognised but props not validated (deferred). |
| ~~Format migration~~ | ✅ Done — all 12 fixtures migrated, `FileNode`/`ComponentDef` in runtime, `ComponentOpen`/`ComponentClose` tokens in lexer, `parse_file()` in parser, `analyze_file()` + `VocabularyMap` in analyzer. See `todo/04a-format-migration.md`. |
| Richer fixtures | `fixtures/` covers the basic cases. Add fixtures for real edge cases: props with complex expressions, `<each>` nested inside `<if>`, non-empty TypeScript logic blocks. |
| More robust recovery in `parse_props` | Recovery on a malformed prop advances to the next `Whitespace`. It could be more precise: skip to the token that clearly starts the next prop or closes the tag. |

---

## Block 4: Codegen

See `todo/04b-codegen.md`.

| Item | Detail |
|------|--------|
| HTML target (Alpine.js) | Deferred post-POC. The Vue SFC target is the primary output for the POC. Alpine.js also requires TypeScript → JS transpilation (esbuild/tsc), a non-trivial dependency for the Rust binary. |
| Component registry / interface for precompiled components | Built-in components (`Column`, `Row`, etc.) are hardcoded in two places: `VocabularyMap` (analyzer) and the node→HTML mapping (codegen). There is no interface to reference precompiled components from an external library. A future component registry — file-based like `tokens.json`, or a Rust trait — should unify both. Medium priority: required before Clutter can be used with a shared component library. |

---

## Before Block 5

| Item | Detail |
|------|--------|
| Error catalogue | Reference page documenting every error code (L001–L002, P001–P003, CLT101–107, W001–W002): cause, example snippet that triggers it, and suggested fix. |
| Benchmarks with `criterion` | No performance measurements yet. Add a benchmark on the lexer to establish a baseline and catch regressions. |
| Compiler API docs — evaluate | Assess whether a higher-level guide to the public API (`tokenize`, `Parser::new`, `parse_file`, `analyze_file`, `generate_vue`) is needed beyond the existing `///` item docs. |

---

## Block 5: CLI

| Item | Detail |
|------|--------|
| **External `tokens.json`** ⚠️ before POC demo | Currently `DesignTokens` is loaded from an inline string in tests; there is no convention for where the file lives in a real project. The CLI must accept a path to an external `tokens.json` supplied by the consuming project — e.g. `clutter build --tokens tokens.json <file>` or by convention from the project root. The internal fixture file is kept for compiler tests only. This must be resolved before the POC can be shown on a real Vue project. |
| `miette` integration | `LexError`, `ParseError`, and `AnalyzerError` must implement the `miette` `Diagnostic` trait. |
| Multi-token span (`start..end`) | `Position` holds only `{ line, col }` of the start. A `Span { start: Position, end: Position }` would allow underlining text ranges in error messages (`miette` supports this natively). |
| ~~`clutter-diagnostics` module (evaluate)~~ | ✅ Done — `Diagnostic` trait + `DiagnosticCollector<T>` implemented in `clutter-runtime::diagnostics`. All three error types use it; `miette` integration still pending separately. |

---

## Post-POC

| Item | Detail |
|------|--------|
| Vue build plugin | A Vite/Webpack plugin that hooks into the Vue project's build pipeline and invokes the Clutter compiler automatically on `.clutter` files. Required for a seamless DX — without it, the developer must run `clutter build` manually before `vite build`. Architecture: the plugin calls the Clutter CLI (or a Node.js binding) on file change/build, then passes the generated `.vue` files to the bundler as virtual modules or writes them to disk. |

---

## Any time

| Item | Detail |
|------|--------|
| `expect_emit` helper | `expect` currently returns `Result`; callers write `if let Err(e) = … { self.emit(…) }`. An `expect_emit` that emits and returns `Option<Token>` would reduce boilerplate where propagation is not needed. |
| `extract_identifiers` — shallow scan limitation | Known false negatives: destructuring (`const { a, b } = …`), imports (`import foo from …`), type aliases, closure variables. Acceptable for the POC; revisit when fuller TypeScript support is needed. |
