use crate::position::Position;

/// Category of a token produced by the lexer.
///
/// The lexer categorises every fragment of source into a `TokenKind` before
/// passing the stream to the parser. `Whitespace` tokens are produced but the
/// parser ignores them via `skip_whitespace`; `Unknown` signals an unrecognised
/// character (the lexer also emits a [`crate::LexError`] in that case).
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // --- Structural ---
    /// The `---` separator between the logic block and the template.
    SectionSeparator,
    /// `<Name` followed by `>`: opens a tag that may have children.
    OpenTag,
    /// `>`: closes an open tag (non-self-closing).
    CloseTag,
    /// `/>`: closes a tag with no children.
    SelfCloseTag,
    /// `</Name>`: closes a previously opened tag.
    CloseOpenTag,

    // --- Props ---
    /// Prop name: an alphanumeric/underscore/hyphen sequence before `=`.
    Identifier,
    /// The `=` character between a prop name and its value.
    Equals,
    /// String prop value: content between `"..."`.
    StringLit,
    /// Expression prop value or template interpolation: content between `{...}`.
    Expression,

    // --- Control flow ---
    /// `<if` tag: introduces a conditional. Props are read normally.
    IfOpen,
    /// `<else` tag: alternative branch of an `<if>`.
    ElseOpen,
    /// `<each` tag: introduces a loop. Props: `collection={expr} as="alias"`.
    EachOpen,
    /// `<unsafe` tag: escape hatch for complex template logic or off-design prop values.
    /// Requires a mandatory non-empty `reason` attribute.
    UnsafeOpen,

    // --- Content ---
    /// Static text between tags (non-whitespace).
    Text,
    /// Sequence of spaces, tabs, or newlines between template elements.
    Whitespace,
    /// Marks the end of the token stream. Always the last token emitted.
    Eof,

    // --- Logic section ---
    /// Raw content of the TypeScript logic block (before `---`).
    /// The compiler treats it as opaque: passed through unchanged to codegen.
    LogicBlock,

    // --- Error ---
    /// Unrecognised character. Accompanied by a [`crate::LexError`] in the error vector.
    Unknown,
}

/// A single token produced by the lexer.
///
/// Each token carries its [`TokenKind`], the original text extracted from the
/// source, and the [`Position`] of its first character.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Token category.
    pub kind: TokenKind,
    /// Raw text from the source (e.g. `"Column"`, `"md"`, `"---"`).
    pub value: String,
    /// Position in the source (first character of the token).
    pub pos: Position,
}
