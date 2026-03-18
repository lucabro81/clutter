# Block 1: Lexer

Legend: `[ ]` todo · `[x]` done · `[-]` skipped/deferred

---

## clutter-runtime — shared types

- [ ] `Position { line: usize, col: usize }` — source location carried by every token
- [ ] `TokenKind` enum — all variants:
  - Structural: `SectionSeparator`, `OpenTag`, `CloseTag`, `SelfCloseTag`, `CloseOpenTag`
  - Props: `Identifier`, `Equals`, `StringLit`, `Expression`
  - Control flow: `IfOpen`, `ElseOpen`, `EachOpen`
  - Content: `Text`, `Whitespace`, `Eof`
  - Logic section: `LogicBlock` (opaque TypeScript blob)
  - Error: `Unknown`
- [ ] `Token { kind: TokenKind, value: String, pos: Position }`
- [ ] `LexError { message: String, pos: Position }` — non-fatal, collection continues

---

## clutter-lexer — tests (written BEFORE implementation)

- [ ] Minimal file: `---` only, empty template → `[LogicBlock(""), SectionSeparator, Eof]`
- [ ] Component with no props: `<Column>` → `[OpenTag("Column"), CloseTag, Eof]`
- [ ] Component with string prop: `<Column gap="md">` → correct tokens with positions
- [ ] Component with expression prop: `<Column gap={size}>`
- [ ] Self-closing tag: `<Text />`
- [ ] Closing tag: `</Column>`
- [ ] Nesting: `<Column><Text /></Column>`
- [ ] Logic section with real TypeScript: `const x = 1` before `---`
- [ ] Control flow: `<if condition={x}>`
- [ ] Control flow: `<else>`
- [ ] Control flow: `<each item={items} as="item">`
- [ ] Unrecognized character → `Unknown` token (no panic, lexing continues)
- [ ] File without `---` → `LexError` with clear message
- [ ] Correct line/col on tokens across multiple lines
- [ ] `Eof` is always the last token

---

## clutter-lexer — implementation

- [ ] `LexerState` enum: `Logic`, `Template`, `InTag`, `InString`, `InExpr`
- [ ] `Lexer::new(input: &str)` — constructor, initial state `Logic`
- [ ] `Lexer::tokenize() -> (Vec<Token>, Vec<LexError>)` — main entry point
- [ ] Main loop: advance char by char, trigger state transitions
- [ ] Detect `---` on its own line → emit `SectionSeparator`, switch to `Template`
- [ ] Collect `LogicBlock` (everything before `---`) as opaque token
- [ ] `<` in `Template` → enter `InTag`, collect name → `OpenTag` or control flow keyword
- [ ] `>` → emit `CloseTag`, return to `Template`
- [ ] `/>` → emit `SelfCloseTag`, return to `Template`
- [ ] `</Name>` → emit `CloseOpenTag`
- [ ] `=` in `InTag` → emit `Equals`
- [ ] `"..."` → enter `InString`, collect value, emit `StringLit`
- [ ] `{...}` → enter `InExpr`, collect content, emit `Expression`
- [ ] Static text between tags → emit `Text`
- [ ] Spaces and newlines → emit `Whitespace`
- [ ] Unclassifiable character → emit `Unknown` + push `LexError` (no panic)
- [ ] Track `line`/`col` on every character, increment `line` on `\n`
- [ ] Always emit `Eof` as the final token
