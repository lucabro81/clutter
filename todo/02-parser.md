# Block 2: Parser

Legend: `[ ]` todo · `[x]` done · `[-]` skipped/deferred

---

## clutter-runtime — AST types

Add AST types to `clutter-runtime/src/lib.rs` — shared by parser, analyzer, and codegen.

- [x] `PropValue` enum: `StringValue(String)` | `ExpressionValue(String)`
- [x] `PropNode { name: String, value: PropValue, pos: Position }`
- [x] `ComponentNode { name: String, props: Vec<PropNode>, children: Vec<Node>, pos: Position }`
- [x] `TextNode { value: String, pos: Position }`
- [x] `ExpressionNode { value: String, pos: Position }`
- [x] `IfNode { condition: String, then_children: Vec<Node>, else_children: Option<Vec<Node>>, pos: Position }`
- [x] `EachNode { collection: String, alias: String, children: Vec<Node>, pos: Position }`
- [x] `Node` enum: `Component(ComponentNode)` | `Text(TextNode)` | `Expr(ExpressionNode)` | `If(IfNode)` | `Each(EachNode)`
- [x] `ProgramNode { logic_block: String, template: Vec<Node> }`
- [x] `ParseError { message: String, pos: Position }`

---

## clutter-parser — tests (written BEFORE implementation)

Tests construct tokens by hand (without running the Lexer) to test the Parser in isolation.

- [x] Single component, no props → `ProgramNode` containing one `ComponentNode`
- [x] Component with string prop → `PropNode { value: StringValue("md") }`
- [x] Component with expression prop → `PropNode { value: ExpressionValue("size") }`
- [x] Two-level nesting: `<Column><Text /></Column>`
- [x] Deep nesting (3+ levels)
- [x] Self-closing component: `<Text />`
- [x] `<if condition={x}>` without `<else>` → `IfNode { else_children: None }`
- [x] `<if>` with `<else>` → `IfNode { else_children: Some([...]) }`
- [x] `<each collection={items} as="item">`
- [x] Non-empty logic block → `ProgramNode.logic_block` contains the raw TypeScript string
- [x] Unclosed tag → `ParseError`
- [x] Prop without `=` or value → `ParseError`
- [x] `<else>` outside `<if>` → `ParseError` with message `"<else> without matching <if>"`

---

## clutter-parser — implementation

- [x] `struct Parser` with fields `tokens: Vec<Token>` and `pos: usize`
- [x] `Parser::peek() -> &Token` — lookahead-1 without consuming
- [x] `Parser::advance() -> Token` — consume and return the current token
- [x] `Parser::expect(kind: TokenKind) -> Result<Token, ParseError>` — consume or error
- [x] `Parser::skip_whitespace()` — skip `Whitespace` tokens
- [x] `Parser::emit(message, pos)` — centralised error construction
- [x] `parse_program(&mut self) -> (ProgramNode, Vec<ParseError>)` — public entry point
- [x] `parse_nodes(&mut self, allow_else: bool) -> Vec<Node>` — collect nodes until stop token
- [x] `parse_node(&mut self) -> Option<Node>` — dispatcher: pick node type from current token
- [x] `parse_component(&mut self, name: String, pos: Position) -> ComponentNode`
- [x] `parse_props(&mut self) -> Vec<PropNode>` — collect props until `CloseTag` or `SelfCloseTag`
- [x] `parse_prop(&mut self) -> Result<PropNode, ParseError>`
- [x] `parse_if(&mut self, pos: Position) -> IfNode`
- [x] `parse_each(&mut self, pos: Position) -> EachNode`
- [x] Error recovery: on unexpected token advance to next `CloseTag` or `Eof` (panic mode)

---

## clutter-parser — integration tests (lexer → parser)

- [x] `fixtures/simple_component.clutter` → one `ComponentNode`, no children
- [x] `fixtures/props.clutter` → string prop + expression prop
- [x] `fixtures/nesting.clutter` → `Column` > `Text` child
- [x] `fixtures/if_else.clutter` → `IfNode` with both branches
- [x] `fixtures/logic_block.clutter` → `ProgramNode.logic_block` non-empty
- [x] `fixtures/orphan_else.clutter` → parse error, message `"<else> without matching <if>"`
- [x] `fixtures/complex.clutter` → `Column` > `Text` + `if` > `Row` > `each` > `Text`; logic block non-empty
