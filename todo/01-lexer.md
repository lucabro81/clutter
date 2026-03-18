# Block 1: Lexer

Legend: `[ ]` todo · `[x]` done · `[-]` skipped/deferred

---

## clutter-runtime — shared types

- [x] `Position { line: usize, col: usize }` — source location carried by every token
- [x] `TokenKind` enum — all variants:
  - Structural: `SectionSeparator`, `OpenTag`, `CloseTag`, `SelfCloseTag`, `CloseOpenTag`
  - Props: `Identifier`, `Equals`, `StringLit`, `Expression`
  - Control flow: `IfOpen`, `ElseOpen`, `EachOpen`
  - Content: `Text`, `Whitespace`, `Eof`
  - Logic section: `LogicBlock` (opaque TypeScript blob)
  - Error: `Unknown`
- [x] `Token { kind: TokenKind, value: String, pos: Position }`
- [x] `LexError { message: String, pos: Position }` — non-fatal, collection continues

---

## clutter-lexer — tests (written BEFORE implementation)

- [x] Minimal file: `---` only, empty template → `[LogicBlock(""), SectionSeparator, Eof]`
- [x] Component with no props: `<Column>` → `[OpenTag("Column"), CloseTag, Eof]`
- [x] Component with string prop: `<Column gap="md">` → correct tokens with positions
- [x] Component with expression prop: `<Column gap={size}>`
- [x] Self-closing tag: `<Text />`
- [x] Closing tag: `</Column>`
- [x] Nesting: `<Column><Text /></Column>`
- [x] Logic section with real TypeScript: `const x = 1` before `---`
- [x] Control flow: `<if condition={x}>`
- [x] Control flow: `<else>`
- [x] Control flow: `<each item={items} as="item">`
- [x] Unrecognized character → `Unknown` token (no panic, lexing continues)
- [x] File without `---` → `LexError` with clear message
- [x] Correct line/col on tokens across multiple lines
- [x] `Eof` is always the last token

---

## clutter-lexer — implementation

- [x] `find_separator(input) -> Option<(&str, line, byte_offset)>` — pre-scan for `---`
- [x] `pub fn tokenize(input: &str) -> (Vec<Token>, Vec<LexError>)` — public entry point
- [x] Collect `LogicBlock` (everything before `---`) as opaque token
- [x] Detect `---` on its own line → emit `SectionSeparator`, switch to template phase
- [x] `TemplateLexer` struct — char-by-char scan with `line`/`col` tracking
- [x] `<` → collect name → `OpenTag` or control flow keyword (`IfOpen`, `ElseOpen`, `EachOpen`)
- [x] `>` → emit `CloseTag`, return to template
- [x] `/>` → emit `SelfCloseTag`, return to template
- [x] `</Name>` → emit `CloseOpenTag`
- [x] `=` in tag body → emit `Equals`
- [x] `"..."` → collect value, emit `StringLit`
- [x] `{...}` → collect content, emit `Expression`
- [x] Static text between tags → emit `Text`
- [x] Spaces and newlines → emit `Whitespace`
- [x] Unclassifiable character → emit `Unknown` + push `LexError` (no panic)
- [x] Track `line`/`col` on every character, increment `line` on `\n`
- [x] Always emit `Eof` as the final token
