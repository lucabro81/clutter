# Backlog — cross-cutting improvements

Ideas that emerged during development and do not belong to a specific block.
To be addressed when the context is mature, not necessarily in order.

---

## Error handling

| Item | Detail | When |
|------|--------|------|
| `emit` in the lexer | The lexer calls `errors.push(LexError { … })` directly at error sites; the parser has a centralised `emit(&mut self, msg, pos)`. Bring the same pattern to the lexer for consistency. | Any time |
| `clutter-diagnostics` module (evaluate) | `LexError`, `ParseError`, and `AnalyzerError` share the same `{ message, pos }` structure. A shared crate/module with a `Diagnostic` trait + `emit` would reduce duplication and simplify the `miette` integration. | Now (Block 3 complete) |
| Structured error codes | Add `code: &'static str` to all error types: `L001` unexpected char, `P001` missing separator, `P002` orphan else, `A101`–`A104` (CLT101–104). Enables testing on codes rather than strings, linkable docs, and selective suppression. | Now (Block 3 complete) |
| Unsafe validation (CLT105/106) — **high priority** | Main selling point of the POC. Lexer/parser support is missing: `<unsafe reason="...">` and `unsafe('val', 'reason')` are not tokenised. Requires a mini parser block (UnsafeBlock + UnsafeValue in the AST), then CLT105/106 in the analyzer. | As soon as possible — unblock before Block 4 |
| Multi-token span (`start..end`) | `Position` holds only the `{ line, col }` of the starting token. A `Span { start: Position, end: Position }` would allow underlining text ranges in error messages (`miette` supports this natively). | When integrating `miette` (Block 5) |

---

## Lexer

| Item | Detail | When |
|------|--------|------|
| `emit` in the lexer | See above. | Any time |
| Tests on exact error messages | Lexer tests only assert the presence of errors, not the text. Align with the parser style (e.g. `assert_eq!(errors[0].message, "…")`). | Before Block 4 |

---

## Parser

| Item | Detail | When |
|------|--------|------|
| `expect_emit` helper | `expect` currently returns `Result`; callers write `if let Err(e) = … { self.emit(…) }`. An `expect_emit` that emits and returns `Option<Token>` would reduce boilerplate where propagation is not needed. | Any time |
| More robust recovery in `parse_props` | Recovery on a malformed prop advances to the next `Whitespace`. It could be more precise: skip to the token that clearly starts the next prop or closes the tag. | Before Block 4 |

---

## Analyzer

| Item | Detail | When |
|------|--------|------|
| Dynamic prop map / custom components | The prop map is hardcoded for the POC. Open questions: where are new built-in components declared? How does a custom component (`component Card(props) {}`) map to token categories? Can the map be loaded from a file or must it always be Rust code? Discuss before Block 4. | Before Block 4 |
| `extract_identifiers` — shallow scan limitation | `extract_identifiers` scans the logic block with `split_whitespace` + previous-token matching. Known false negatives: destructuring (`const { a, b } = …`), imports (`import foo from …`), type aliases, closure variables. Acceptable for the POC. | When fuller TypeScript support is needed |

---

## Documentation

| Item | Detail | When |
|------|--------|------|
| Error catalogue | Write a reference page (or doc module) documenting every error code (L001, P001–P002, CLT101–106): cause, example snippet that triggers it, and suggested fix. Useful for end users and for linking from `miette` diagnostics in Block 5. | Before Block 5 |
| Compiler API docs — evaluate | Assess whether a higher-level guide to the public API (`tokenize`, `Parser::new` + `parse_program`, `analyze`, future `codegen`) is needed beyond the existing `///` item docs. Could be a `docs/` page, a top-level `lib.rs` crate, or just ensuring `cargo doc` output is navigable. Decide scope before Block 5. | Before Block 5 |

---

## Tooling / quality

| Item | Detail | When |
|------|--------|------|
| `miette` integration | Planned for Block 5 (CLI). Will require `LexError`, `ParseError`, and `AnalyzerError` to implement the `miette` `Diagnostic` trait. | Block 5 |
| Richer fixtures | `fixtures/` covers the basic cases. Before Block 4, add fixtures for real edge cases: props with complex expressions, `<each>` nested inside `<if>`, non-empty TypeScript logic blocks. | Before Block 4 |
| Benchmarks with `criterion` | No performance measurements yet. Add a benchmark on the lexer before Block 5 to establish a baseline and catch regressions. | Before Block 5 |
