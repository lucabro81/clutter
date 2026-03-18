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
