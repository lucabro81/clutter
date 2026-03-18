#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Structural
    SectionSeparator,
    OpenTag,
    CloseTag,
    SelfCloseTag,
    CloseOpenTag,
    // Props
    Identifier,
    Equals,
    StringLit,
    Expression,
    // Control flow
    IfOpen,
    ElseOpen,
    EachOpen,
    // Content
    Text,
    Whitespace,
    Eof,
    // Logic section
    LogicBlock,
    // Error
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub pos: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub message: String,
    pub pos: Position,
}

// --- AST types ---

#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    StringValue(String),
    ExpressionValue(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropNode {
    pub name: String,
    pub value: PropValue,
    pub pos: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentNode {
    pub name: String,
    pub props: Vec<PropNode>,
    pub children: Vec<Node>,
    pub pos: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextNode {
    pub value: String,
    pub pos: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionNode {
    pub value: String,
    pub pos: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfNode {
    pub condition: String,
    pub then_children: Vec<Node>,
    pub else_children: Option<Vec<Node>>,
    pub pos: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EachNode {
    pub collection: String,
    pub alias: String,
    pub children: Vec<Node>,
    pub pos: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Component(ComponentNode),
    Text(TextNode),
    Expr(ExpressionNode),
    If(IfNode),
    Each(EachNode),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProgramNode {
    pub logic_block: String,
    pub template: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub pos: Position,
}
