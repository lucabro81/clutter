# Block 2: Parser

Legend: `[ ]` todo · `[x]` done · `[-]` skipped/deferred

---

## clutter-runtime — AST types

Add AST types to `clutter-runtime/src/lib.rs` — shared by parser, analyzer, and codegen.

- [ ] `PropValue` enum: `StringValue(String)` | `ExpressionValue(String)`
- [ ] `PropNode { name: String, value: PropValue, pos: Position }`
- [ ] `ComponentNode { name: String, props: Vec<PropNode>, children: Vec<Node>, pos: Position }`
- [ ] `TextNode { value: String, pos: Position }`
- [ ] `ExpressionNode { value: String, pos: Position }`
- [ ] `IfNode { condition: String, then_children: Vec<Node>, else_children: Option<Vec<Node>>, pos: Position }`
- [ ] `EachNode { collection: String, alias: String, children: Vec<Node>, pos: Position }`
- [ ] `Node` enum: `Component(ComponentNode)` | `Text(TextNode)` | `Expr(ExpressionNode)` | `If(IfNode)` | `Each(EachNode)`
- [ ] `ProgramNode { logic_block: String, template: Vec<Node> }`
- [ ] `ParseError { message: String, pos: Position }`

---

## clutter-parser — tests (written BEFORE implementation)

Tests construct tokens by hand (without running the Lexer) to test the Parser in isolation.

- [ ] Single component, no props → `ProgramNode` containing one `ComponentNode`
- [ ] Component with string prop → `PropNode { value: StringValue("md") }`
- [ ] Component with expression prop → `PropNode { value: ExpressionValue("size") }`
- [ ] Two-level nesting: `<Column><Text /></Column>`
- [ ] Deep nesting (3+ levels)
- [ ] Self-closing component: `<Text />`
- [ ] `<if condition={x}>` without `<else>` → `IfNode { else_children: None }`
- [ ] `<if>` with `<else>` → `IfNode { else_children: Some([...]) }`
- [ ] `<each collection={items} as="item">`
- [ ] Non-empty logic block → `ProgramNode.logic_block` contains the raw TypeScript string
- [ ] Unclosed tag → `ParseError`
- [ ] Prop without `=` or value → `ParseError`

---

## clutter-parser — implementation

- [ ] `struct Parser` with fields `tokens: Vec<Token>` and `pos: usize`
- [ ] `Parser::peek() -> &Token` — lookahead-1 without consuming
- [ ] `Parser::advance() -> Token` — consume and return the current token
- [ ] `Parser::expect(kind: TokenKind) -> Result<Token, ParseError>` — consume or error
- [ ] `Parser::skip_whitespace()` — skip `Whitespace` tokens
- [ ] `parse_program(&mut self) -> (ProgramNode, Vec<ParseError>)` — public entry point
- [ ] `parse_nodes(&mut self) -> Vec<Node>` — collect nodes until `CloseOpenTag` or `Eof`
- [ ] `parse_node(&mut self) -> Option<Node>` — dispatcher: pick node type from current token
- [ ] `parse_component(&mut self, name: String, pos: Position) -> ComponentNode`
- [ ] `parse_props(&mut self) -> Vec<PropNode>` — collect props until `CloseTag` or `SelfCloseTag`
- [ ] `parse_prop(&mut self) -> Result<PropNode, ParseError>`
- [ ] `parse_if(&mut self, pos: Position) -> IfNode`
- [ ] `parse_each(&mut self, pos: Position) -> EachNode`
- [ ] Error recovery: on unexpected token advance to next `CloseTag` or `Eof` (panic mode)
