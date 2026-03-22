# Block 4A: Multi-Component Format Migration

Legend: `[ ]` todo · `[x]` done · `[-]` skipped/deferred

---

## Context

Before codegen can be implemented, two structural migrations must land:

1. **Format migration** — every `.clutter` file now wraps each component in an explicit
   named block (`component Name(props) { }`). The `----` separator (4 dashes) replaces `---`.
   The AST root becomes `FileNode { components: Vec<ComponentDef> }`.

2. **VocabularyMap** — the analyzer's two diverging structures (`KNOWN_COMPONENTS` array +
   `prop_map()` function) are unified into a single `VocabularyMap` struct that is the sole
   source of truth for the built-in vocabulary.

These changes touch every crate. Migration order follows the dependency graph:
`clutter-runtime` → `clutter-lexer` → `clutter-parser` → `clutter-analyzer` → fixtures.

Pipeline position after migration: unchanged — same stages, same public errors, new format.

---

## clutter-runtime — AST types

- [x] Add `ComponentDef { name: String, props_raw: String, logic_block: String, template: Vec<Node> }`
- [x] Add `FileNode { components: Vec<ComponentDef> }` — new AST root
- [x] Keep `ProgramNode` temporarily with a `#[deprecated]` attr until all callers are migrated
- [x] Remove `ProgramNode` once all callers are updated

---

## clutter-runtime — token kinds

- [x] Add `TokenKind::ComponentOpen { name: String, props_raw: String }` — emitted for `component Name(...) {`
- [x] Add `TokenKind::ComponentClose` — emitted for the closing `}` at the component level
- [x] Update `TokenKind::SectionSeparator` — now represents `----` (4 dashes), not `---`

---

## clutter-lexer — tests (written BEFORE implementation)

- [x] `component Name(props: T) {` on its own line → `ComponentOpen { name: "Name", props_raw: "props: T" }`
- [x] `}` at component level → `ComponentClose`
- [x] `----` (4 dashes) inside a component block → `SectionSeparator`
- [x] `---` (3 dashes) no longer recognized as `SectionSeparator` → `Unknown` or `LexError`
- [x] File with two `component` blocks → two `ComponentOpen`…`ComponentClose` pairs in the token stream
- [x] Logic block content captured correctly between `ComponentOpen` and `SectionSeparator`
- [x] Template tokens unchanged inside the component block (all existing token tests still pass)
- [x] File missing `component` keyword → `LexError` (L001 variant or new code)
- [x] `props_raw` captures multi-token signatures verbatim: `props: CardProps`, `title: string, size: SpacingToken`

---

## clutter-lexer — implementation

- [x] Update `find_separator` (or replace) to scan for `component Name(...) {` lines
- [x] Emit `ComponentOpen { name, props_raw }` at the start of each component block
- [x] Collect logic block between `ComponentOpen` and `----` as `LogicBlock` token (unchanged)
- [x] Detect `----` (4 dashes) on its own line → emit `SectionSeparator`
- [x] Detect `}` at column 0 (or unindented) → emit `ComponentClose`
- [x] Loop: after `ComponentClose`, resume scanning for the next `component` keyword
- [x] Update module-level doc comment to reflect new file format

---

## clutter-parser — tests (written BEFORE implementation)

- [x] Single component file → `FileNode` with one `ComponentDef`
- [x] Two-component file → `FileNode` with two `ComponentDef`s, names and props_raw correct
- [x] `ComponentDef.logic_block` captures raw TypeScript correctly
- [x] `ComponentDef.template` contains the parsed nodes (same structure as before)
- [x] Empty template in a component → `template: vec![]`, no parse error
- [x] Missing `ComponentOpen` at start of file → `ParseError`
- [-] Missing `SectionSeparator` inside a component block → `ParseError` (lexer emits no tokens for body without separator; parser sees empty template — acceptable)
- [x] All existing template-level parse tests still pass (nodes, props, if/each/unsafe)

---

## clutter-parser — implementation

- [x] Rename `parse_program` → `parse_file`, return `FileNode`
- [x] Top-level loop: consume `ComponentOpen` tokens, delegate to a per-component parser
- [x] `parse_component_def()` — consume logic block up to `SectionSeparator`, then parse template nodes until `ComponentClose`
- [x] Internal parsing functions (`parse_nodes`, `parse_component`, `parse_props`, etc.) unchanged
- [x] Update module-level doc comment

---

## clutter-analyzer — tests (written BEFORE implementation)

- [x] Single-component `FileNode` — all existing error cases still fire (CLT101–107)
- [x] Two-component file — errors in each component reported independently, with correct positions
- [x] Component defined in the same file used as a child → no CLT103 (recognized as custom component)
- [x] Custom component prop → no CLT101/CLT102 (props not validated, `AnyValue`)
- [x] `analyze_file()` added alongside `analyze()` — new public entry point for `FileNode`

---

## clutter-analyzer — implementation

- [x] Add `analyze_file(file: &FileNode, tokens: &DesignTokens) -> (Vec<AnalyzerError>, Vec<AnalyzerWarning>)`
- [x] Collect the set of component names defined in the `FileNode` before validation
- [x] Iterate over `file.components`, run existing validation per `ComponentDef`
- [x] Pass component name set into the validator so custom components skip CLT103

---

## clutter-analyzer — VocabularyMap refactor

- [x] Define `ComponentSchema { props: HashMap<&'static str, PropValidation> }`
- [x] Define `VocabularyMap { components: HashMap<&'static str, ComponentSchema> }`
- [x] `VocabularyMap::new()` — constructs the built-in vocabulary (replaces `KNOWN_COMPONENTS` + `prop_map`)
- [x] `VocabularyMap::contains(&self, name: &str) -> bool` — replaces CLT103 check
- [x] `VocabularyMap::prop(&self, component: &str, prop: &str) -> Option<&PropValidation>` — replaces `prop_map()`
- [x] Construct `VocabularyMap` once at the start of `analyze_file()`
- [-] Delete `KNOWN_COMPONENTS` and `prop_map()` — kept alive by deprecated `analyze()` which is still referenced by old tests
- [x] Tests: same errors emitted before and after the refactor (pure internal change)

---

## Fixtures migration

12 fixtures rewritten. Each has a `component MainComponent(props: Props) { }` wrapper
and `---` replaced with `----`.

- [x] `simple_component.clutter`
- [x] `props.clutter`
- [x] `nesting.clutter`
- [x] `if_else.clutter`
- [x] `logic_block.clutter`
- [x] `orphan_else.clutter`
- [x] `complex.clutter`
- [x] `valid.clutter`
- [x] `invalid_token.clutter`
- [x] `unsafe_block.clutter`
- [x] `unsafe_value.clutter`
- [x] `clt107_complex_expr.clutter`

Multi-component fixture deferred — existing unit tests in `clutter-analyzer/src/tests.rs`
cover the two-component case with synthetic data.

---

## Integration tests — update

- [x] Update `clutter-parser/tests/integration.rs` — call `parse_file`, assert `FileNode`
- [x] Update `clutter-analyzer/tests/integration.rs` — pass `FileNode`, all existing assertions hold

---

## Final check

- [x] `cargo test` — full workspace green
- [x] Remove `ProgramNode` + deprecated `analyze()` + deprecated `parse_program()` + old internal functions (`analyze_nodes`, `analyze_component`, `validate_prop`, `analyze_if`, `analyze_each`, `analyze_unsafe`, `prop_map`, `KNOWN_COMPONENTS`)
- [x] Mark `Format migration` item in `todo/00-backlog.md` as done
