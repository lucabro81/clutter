//! Shared types for the entire Clutter compiler pipeline.
//!
//! This crate is the common dependency for all others (`clutter-lexer`,
//! `clutter-parser`, `clutter-analyzer`, `clutter-codegen`). It contains no
//! logic: it only defines the data structures exchanged between pipeline stages.
//!
//! # Structure
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │           clutter-runtime           │
//! │                                     │
//! │  Token/TokenKind  ← used by lexer   │
//! │  AST nodes        ← used by parser  │
//! │  *Error types     ← used by all     │
//! └─────────────────────────────────────┘
//! ```
//!
//! # Error types
//!
//! Each stage produces its own error type, all sharing the same `{ message, pos }`
//! structure for consistency. The `miette` integration (Block 5) will enrich them
//! with structured error codes and multi-token spans.
//!
//! | Type             | Produced by        |
//! |------------------|--------------------|
//! | [`LexError`]     | `clutter-lexer`    |
//! | [`ParseError`]   | `clutter-parser`   |
//! | [`AnalyzerError`]| `clutter-analyzer` |

// ---------------------------------------------------------------------------
// Source position
// ---------------------------------------------------------------------------

/// Position of a token or AST node in the `.clutter` source file.
///
/// Points to the start of the token (first character). Lines and columns are
/// 1-indexed.
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    /// Line number (1-based).
    pub line: usize,
    /// Column number (1-based).
    pub col: usize,
}

// ---------------------------------------------------------------------------
// Token
// ---------------------------------------------------------------------------

/// Category of a token produced by the lexer.
///
/// The lexer categorises every fragment of source into a `TokenKind` before
/// passing the stream to the parser. `Whitespace` tokens are produced but the
/// parser ignores them via `skip_whitespace`; `Unknown` signals an unrecognised
/// character (the lexer also emits a [`LexError`] in that case).
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
    /// Unrecognised character. Accompanied by a [`LexError`] in the error vector.
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

// ---------------------------------------------------------------------------
// Lexer errors
// ---------------------------------------------------------------------------

/// Error produced by the lexer during tokenisation.
///
/// The lexer does not stop at the first error: it continues scanning and
/// accumulates all errors in a `Vec<LexError>` returned alongside the token stream.
#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    /// Human-readable description of the problem (e.g. `"unexpected character '@' in template"`).
    pub message: String,
    /// Position in the source where the error was detected.
    pub pos: Position,
}

// ---------------------------------------------------------------------------
// AST nodes
// ---------------------------------------------------------------------------

/// Value of a component prop.
///
/// A prop can have a string literal value — to be validated against the design
/// system — or a TypeScript expression — evaluated at runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    /// String literal: `gap="md"`. Must be present in the design system.
    StringValue(String),
    /// TypeScript expression: `gap={myVar}`. The identifier name is checked
    /// by the analyzer against bindings declared in the logic block.
    ExpressionValue(String),
}

/// A single `name=value` prop on a component.
#[derive(Debug, Clone, PartialEq)]
pub struct PropNode {
    /// Prop name (e.g. `"gap"`, `"size"`).
    pub name: String,
    /// Prop value (string or expression).
    pub value: PropValue,
    /// Position in the source (first character of the name).
    pub pos: Position,
}

/// A component from the closed vocabulary (e.g. `<Column>`, `<Text />`).
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentNode {
    /// Component name (e.g. `"Column"`, `"Text"`).
    pub name: String,
    /// Props declared on the opening tag.
    pub props: Vec<PropNode>,
    /// Children: present only if the tag is not self-closing.
    pub children: Vec<Node>,
    /// Position of the opening tag in the source.
    pub pos: Position,
}

/// Static text between tags (not an interpolation, not structural whitespace).
#[derive(Debug, Clone, PartialEq)]
pub struct TextNode {
    /// The raw text.
    pub value: String,
    /// Position in the source.
    pub pos: Position,
}

/// Interpolation of a TypeScript expression in the template: `{expr}`.
///
/// The expression name is checked by the analyzer (CLT104) against bindings
/// declared in the logic block.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionNode {
    /// Name of the interpolated identifier (e.g. `"title"`, `"count"`).
    pub value: String,
    /// Position in the source.
    pub pos: Position,
}

/// Conditional node `<if condition={expr}>…</if>` with an optional else branch.
#[derive(Debug, Clone, PartialEq)]
pub struct IfNode {
    /// The condition expression (identifier name).
    pub condition: String,
    /// Children of the `then` branch (between `<if>` and `<else>` or `</if>`).
    pub then_children: Vec<Node>,
    /// Children of the `else` branch, present only if the `<else>` tag is declared.
    pub else_children: Option<Vec<Node>>,
    /// Position of the `<if>` tag in the source.
    pub pos: Position,
}

/// Iteration node `<each collection={expr} as="alias">…</each>`.
#[derive(Debug, Clone, PartialEq)]
pub struct EachNode {
    /// The collection expression (identifier name).
    pub collection: String,
    /// The alias assigned to the current element (local binding for children).
    pub alias: String,
    /// Children of the loop body.
    pub children: Vec<Node>,
    /// Position of the `<each>` tag in the source.
    pub pos: Position,
}

/// A template node: the union of all possible node types.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Closed-vocabulary component (e.g. `<Column>`, `<Text />`).
    Component(ComponentNode),
    /// Static text.
    Text(TextNode),
    /// Expression interpolation `{expr}`.
    Expr(ExpressionNode),
    /// Conditional `<if>`.
    If(IfNode),
    /// Iteration `<each>`.
    Each(EachNode),
}

/// The root of the AST produced by the parser.
///
/// Corresponds to an entire `.clutter` file. The file structure is:
///
/// ```text
/// [TypeScript logic block — opaque to the compiler]
/// ---
/// [template — AST nodes]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramNode {
    /// Raw content of the TypeScript logic block (before `---`).
    /// May be empty if the file starts directly with `---`.
    pub logic_block: String,
    /// Top-level nodes of the template (after `---`).
    pub template: Vec<Node>,
}

// ---------------------------------------------------------------------------
// Parser and analyzer errors
// ---------------------------------------------------------------------------

/// Error produced by the parser during AST construction.
///
/// The parser does not stop at the first error: it applies a recovery strategy
/// (advances to the next prop boundary or tag boundary) and accumulates all
/// errors in a `Vec<ParseError>`.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// Human-readable description of the problem.
    pub message: String,
    /// Position in the source where the error was detected.
    pub pos: Position,
}

/// Semantic error produced by the analyzer.
///
/// The analyzer collects all semantic errors (CLT101–104) into a
/// `Vec<AnalyzerError>` without stopping at the first. An empty list means
/// the file is valid and can proceed to codegen.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzerError {
    /// Human-readable description of the problem, prefixed with the error code
    /// (e.g. `"CLT102: invalid value 'xl2' for prop 'gap' on 'Column'. Valid values: …"`).
    pub message: String,
    /// Position in the source where the error was detected.
    pub pos: Position,
}
