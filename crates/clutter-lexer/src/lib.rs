//! Lexer for `.clutter` source files.
//!
//! First stage of the compilation pipeline:
//!
//! ```text
//! .clutter  →  **Lexer**  →  Parser  →  Analyzer  →  Codegen
//! ```
//!
//! # Structure of a `.clutter` file
//!
//! ```text
//! component Name(props_signature) {
//!     [TypeScript logic block — opaque, treated as a raw string]
//!     ----
//!     [template — JSX-like markup with a closed vocabulary]
//! }
//! ```
//!
//! A file may contain one or more `component` blocks. Each block is wrapped in
//! explicit curly braces; the `----` separator (4 dashes) on its own line is the
//! boundary between the logic block and the template.
//!
//! # Output
//!
//! [`tokenize`] returns `(Vec<Token>, Vec<LexError>)`. The presence of errors does
//! not interrupt tokenisation: the lexer continues and emits a
//! [`TokenKind::Unknown`] token for every unrecognised character, so the parser
//! can collect further errors on the same file.
//!
//! [`TokenKind::Eof`] is **always** the last token in the vector, even when errors
//! are present.
//!
//! # Tokenisation strategy
//!
//! 1. [`component_blocks::find_components`] scans the source line by line,
//!    collecting each `component Name(...) { … }` block.
//!    If no blocks are found, a [`LexError`] is emitted.
//! 2. For each block:
//!    a. A [`TokenKind::ComponentOpen`] token is emitted.
//!    b. [`component_blocks::find_section_separator`] locates `----`; the logic
//!       becomes a [`TokenKind::LogicBlock`] token; `----` becomes
//!       [`TokenKind::SectionSeparator`].
//!    c. The template portion is handed to [`template_lexer::TemplateLexer::scan`].
//!    d. A [`TokenKind::ComponentClose`] token is emitted for the closing `}`.

use clutter_runtime::{codes, LexError, Position, Token, TokenKind};

mod component_blocks;
mod template_lexer;

use component_blocks::{find_components, find_section_separator};
use template_lexer::TemplateLexer;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Tokenises a complete `.clutter` source file.
///
/// # Algorithm
///
/// 1. [`find_components`] collects every `component Name(…) { … }` block.
/// 2. If none found: emits a [`LexError`] (L001) and returns `([Eof], [error])`.
/// 3. For each component block:
///    - Emits [`TokenKind::ComponentOpen`] with name and raw props signature.
///    - [`find_section_separator`] locates `----`; emits `LogicBlock` + `SectionSeparator`.
///    - Delegates template scanning to [`TemplateLexer`].
///    - Emits [`TokenKind::ComponentClose`].
/// 4. Always appends `Eof` at the end of the token vector.
///
/// # Returns
///
/// - `Vec<Token>`: token stream to be passed to the parser. `Eof` is always present.
/// - `Vec<LexError>`: collected errors (may be empty). The presence of errors does
///   not prevent returning partial tokens.
pub fn tokenize(input: &str) -> (Vec<Token>, Vec<LexError>) {
    let mut tokens: Vec<Token> = Vec::new();
    let mut errors: Vec<LexError> = Vec::new();

    let components = find_components(input);

    if components.is_empty() {
        errors.push(LexError {
            code: codes::L001,
            message: "no component blocks found: expected `component Name(…) { … }`".to_string(),
            pos: Position { line: 1, col: 1 },
        });
        tokens.push(Token {
            kind: TokenKind::Eof,
            value: String::new(),
            pos: Position { line: 1, col: 1 },
        });
        return (tokens, errors);
    }

    let mut last_pos = Position { line: 1, col: 1 };

    for comp in components {
        tokens.push(Token {
            kind: TokenKind::ComponentOpen {
                name: comp.name.clone(),
                props_raw: comp.props_raw.clone(),
            },
            value: comp.header_raw.clone(),
            pos: comp.open_pos,
        });

        match find_section_separator(&comp.body, comp.body_start_line) {
            None => {
                errors.push(LexError {
                    code: codes::L001,
                    message: format!(
                        "missing ---- separator in component '{}': \
                         logic and template sections must be separated by ----",
                        comp.name
                    ),
                    pos: comp.open_pos,
                });
                tokens.push(Token {
                    kind: TokenKind::LogicBlock,
                    value: String::new(),
                    pos: Position { line: comp.body_start_line, col: 1 },
                });
            }
            Some((logic, sep_line, template_str)) => {
                tokens.push(Token {
                    kind: TokenKind::LogicBlock,
                    value: logic.to_string(),
                    pos: Position { line: comp.body_start_line, col: 1 },
                });
                tokens.push(Token {
                    kind: TokenKind::SectionSeparator,
                    value: "----".to_string(),
                    pos: Position { line: sep_line, col: 1 },
                });
                let mut lex = TemplateLexer::new(template_str, sep_line + 1);
                lex.scan(&mut tokens);
                last_pos = lex.current_pos();
                errors.extend(lex.errors.into_vec());
            }
        }

        last_pos = comp.close_pos;
        tokens.push(Token {
            kind: TokenKind::ComponentClose,
            value: "}".to_string(),
            pos: comp.close_pos,
        });
    }

    tokens.push(Token { kind: TokenKind::Eof, value: String::new(), pos: last_pos });
    (tokens, errors)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
