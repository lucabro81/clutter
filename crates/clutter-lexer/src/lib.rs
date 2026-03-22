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
//! 1. [`find_components`] scans the source line by line, collecting each
//!    `component Name(...) { … }` block into a [`ComponentBlock`] value.
//!    If no blocks are found, a [`LexError`] is emitted.
//! 2. For each [`ComponentBlock`]:
//!    a. A [`TokenKind::ComponentOpen`] token is emitted with the component name
//!       and raw props signature.
//!    b. [`find_section_separator`] locates the `----` line inside the block body.
//!       The content before it becomes a [`TokenKind::LogicBlock`] token; the
//!       `----` itself becomes [`TokenKind::SectionSeparator`].
//!    c. The template portion is handed to [`TemplateLexer::scan`], which
//!       recognises tags, props, text, expressions, and whitespace.
//!    d. A [`TokenKind::ComponentClose`] token is emitted for the closing `}`.

use clutter_runtime::{codes, DiagnosticCollector, LexError, Position, Token, TokenKind};

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
// Component block discovery
// ---------------------------------------------------------------------------

/// A single `component Name(props) { … }` block extracted from the source.
struct ComponentBlock {
    /// Component name (e.g. `"MainComponent"`).
    name: String,
    /// Raw props signature between `(` and `)` (e.g. `"props: MainProps"`).
    props_raw: String,
    /// The raw `component Name(…) {` line, stored as the token value.
    header_raw: String,
    /// Source position of the `component` keyword (1-based line, col 1).
    open_pos: Position,
    /// Everything between the opening `{` line and the closing `}` line,
    /// joined with newlines. Includes the `----` separator line.
    body: String,
    /// Absolute 1-based line number of the first line of the body.
    body_start_line: usize,
    /// Source position of the closing `}` (1-based line, col 1).
    close_pos: Position,
}

/// State machine for accumulating an in-progress component block.
struct ActiveComponent {
    name: String,
    props_raw: String,
    header_raw: String,
    open_pos: Position,
    body_lines: Vec<String>,
    body_start_line: usize,
    /// True once `----` has been seen; after this, `}` terminates the block.
    seen_separator: bool,
}

/// Scans `input` line by line and returns all complete `component` blocks found.
///
/// A block starts with a line matching `component Name(…) {` and ends with a
/// line whose trimmed content is `}` — but only after the `----` separator has
/// been seen (so `}` in TypeScript logic does not close the block prematurely).
fn find_components(input: &str) -> Vec<ComponentBlock> {
    let mut result = Vec::new();
    let mut active: Option<ActiveComponent> = None;
    let mut current_line = 1usize;

    for line in input.lines() {
        if let Some(ref mut ac) = active {
            if !ac.seen_separator && line.trim() == "----" {
                ac.seen_separator = true;
                ac.body_lines.push(line.to_string());
            } else if ac.seen_separator && line.trim() == "}" {
                let body = ac.body_lines.join("\n");
                result.push(ComponentBlock {
                    name: ac.name.clone(),
                    props_raw: ac.props_raw.clone(),
                    header_raw: ac.header_raw.clone(),
                    open_pos: ac.open_pos,
                    body,
                    body_start_line: ac.body_start_line,
                    close_pos: Position { line: current_line, col: 1 },
                });
                active = None;
            } else {
                ac.body_lines.push(line.to_string());
            }
        } else if let Some((name, props_raw)) = parse_component_header(line) {
            active = Some(ActiveComponent {
                name,
                props_raw,
                header_raw: line.to_string(),
                open_pos: Position { line: current_line, col: 1 },
                body_lines: Vec::new(),
                body_start_line: current_line + 1,
                seen_separator: false,
            });
        }
        current_line += 1;
    }

    result
}

/// Attempts to parse a `component Name(props) {` header line.
///
/// Returns `Some((name, props_raw))` on success, `None` otherwise.
/// The `props_raw` is the verbatim content between `(` and the last `)`.
fn parse_component_header(line: &str) -> Option<(String, String)> {
    let rest = line.trim().strip_prefix("component ")?;
    let paren_open = rest.find('(')?;
    let name = rest[..paren_open].trim().to_string();
    if name.is_empty() {
        return None;
    }
    let after_name = &rest[paren_open + 1..];
    let paren_close = after_name.rfind(')')?;
    let props_raw = after_name[..paren_close].to_string();
    let after_close = after_name[paren_close + 1..].trim();
    if after_close != "{" {
        return None;
    }
    Some((name, props_raw))
}

// ---------------------------------------------------------------------------
// Section separator search
// ---------------------------------------------------------------------------

/// Finds the `----` separator inside a component body string.
///
/// `start_line` is the absolute 1-based line number of the first line of `body`,
/// used to compute the absolute line number of the separator.
///
/// Returns `Some((logic_content, sep_line, template_str))` where:
/// - `logic_content`: raw text before `----`, trailing newlines stripped.
/// - `sep_line`: absolute 1-based line number of the `----` line.
/// - `template_str`: the string slice starting immediately after the `\n`
///   that follows `----` (the template content).
///
/// Returns `None` if `----` does not appear as a standalone line in `body`.
fn find_section_separator(body: &str, start_line: usize) -> Option<(&str, usize, &str)> {
    let mut line_start = 0usize;
    let mut current_line = start_line;

    loop {
        match body[line_start..].find('\n') {
            None => {
                let line = &body[line_start..];
                if line == "----" {
                    let logic = body[..line_start].trim_end_matches('\n');
                    return Some((logic, current_line, ""));
                }
                return None;
            }
            Some(offset) => {
                let line_end = line_start + offset;
                let line = &body[line_start..line_end];
                if line == "----" {
                    let logic = body[..line_start].trim_end_matches('\n');
                    let template_start = line_end + 1;
                    return Some((logic, current_line, &body[template_start..]));
                }
                current_line += 1;
                line_start = line_end + 1;
                if line_start > body.len() {
                    return None;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TemplateLexer
// ---------------------------------------------------------------------------

/// Finite-state lexer for the template section of a `.clutter` file.
///
/// Operates on a slice of the source string starting immediately after the `\n`
/// of the `---` separator. Maintains the current position (`line`, `col`) to
/// attach precise [`Position`] values to every emitted token.
///
/// Not instantiated directly from outside: [`tokenize`] creates it internally
/// and calls [`TemplateLexer::scan`].
struct TemplateLexer {
    /// The template source as a `char` vector (O(1) indexing).
    chars: Vec<char>,
    /// Index of the next character to read in `chars`.
    pos: usize,
    /// Current line number (1-based, already adjusted for the template offset).
    line: usize,
    /// Current column number (1-based).
    col: usize,
    /// Errors accumulated during scanning (drained by `tokenize` at the end).
    errors: DiagnosticCollector<LexError>,
}

impl TemplateLexer {
    /// Creates a new `TemplateLexer`.
    ///
    /// `start_line` must be the line number immediately following the `---`
    /// separator in the original file, so that all positions are absolute.
    fn new(input: &str, start_line: usize) -> Self {
        TemplateLexer {
            chars: input.chars().collect(),
            pos: 0,
            line: start_line,
            col: 1,
            errors: DiagnosticCollector::new(),
        }
    }


    /// Returns the [`Position`] of the next character to be read.
    fn current_pos(&self) -> Position {
        Position { line: self.line, col: self.col }
    }

    /// Reads the current character without advancing the cursor.
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// Reads the character `offset` positions ahead of the cursor without advancing.
    ///
    /// Used for two-character lookahead (`/>`) in [`scan_tag_body`].
    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    /// Advances the cursor by one character and updates `line`/`col`.
    ///
    /// Returns the consumed character, or `None` if the end of input has
    /// already been reached.
    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    /// Scans the complete template and accumulates tokens and errors.
    ///
    /// Main loop: dispatches each character to the appropriate handler.
    ///
    /// | Leading character | Action                                                              |
    /// |-------------------|---------------------------------------------------------------------|
    /// | `<`               | [`scan_tag`]                                                        |
    /// | whitespace        | aggregates all spaces/tabs/newlines into a single `Whitespace` token |
    /// | text character    | aggregates characters into a `Text` token via [`is_text_char`]      |
    /// | other             | emits `Unknown` + [`LexError`]                                      |
    fn scan(&mut self, tokens: &mut Vec<Token>) {
        while let Some(ch) = self.peek() {
            match ch {
                '<' => self.scan_tag(tokens),
                ' ' | '\t' | '\n' | '\r' => {
                    let pos = self.current_pos();
                    let mut ws = String::new();
                    while let Some(c) = self.peek() {
                        if matches!(c, ' ' | '\t' | '\n' | '\r') {
                            ws.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    tokens.push(Token { kind: TokenKind::Whitespace, value: ws, pos });
                }
                c if is_text_char(c) => {
                    let pos = self.current_pos();
                    let mut text = String::new();
                    while let Some(c) = self.peek() {
                        if is_text_char(c) {
                            text.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    tokens.push(Token { kind: TokenKind::Text, value: text, pos });
                }
                _ => {
                    let pos = self.current_pos();
                    let c = self.advance().unwrap();
                    tokens.push(Token {
                        kind: TokenKind::Unknown,
                        value: c.to_string(),
                        pos: pos.clone(),
                    });
                    self.errors.emit(LexError { code: codes::L002, message: format!("unexpected character '{}' in template", c), pos });
                }
            }
        }
    }

    /// Scans a tag starting with `<`.
    ///
    /// Handles three cases:
    /// - `</Name>` → [`TokenKind::CloseOpenTag`]
    /// - `<if`, `<else`, `<each` → their respective control-flow tokens
    /// - `<Name` → [`TokenKind::OpenTag`], then delegates props to [`scan_tag_body`]
    fn scan_tag(&mut self, tokens: &mut Vec<Token>) {
        let tag_start = self.current_pos();
        self.advance(); // consume '<'

        // Closing tag: </Name>
        if self.peek() == Some('/') {
            self.advance(); // consume '/'
            let name = self.collect_identifier();
            while matches!(self.peek(), Some(' ') | Some('\t')) {
                self.advance();
            }
            if self.peek() == Some('>') {
                self.advance();
            }
            tokens.push(Token { kind: TokenKind::CloseOpenTag, value: name, pos: tag_start });
            return;
        }

        // Read tag name and emit appropriate token.
        let name = self.collect_identifier();
        let kind = match name.as_str() {
            "if" => TokenKind::IfOpen,
            "else" => TokenKind::ElseOpen,
            "each" => TokenKind::EachOpen,
            "unsafe" => TokenKind::UnsafeOpen,
            _ => TokenKind::OpenTag,
        };
        tokens.push(Token { kind, value: name, pos: tag_start });

        self.scan_tag_body(tokens);
    }

    /// Scans the body of an open tag: props and terminators (`>` or `/>`).
    ///
    /// Iterates skipping whitespace and recognising:
    /// - `>` → [`TokenKind::CloseTag`], end of tag
    /// - `/>` → [`TokenKind::SelfCloseTag`], end of tag
    /// - `=` → [`TokenKind::Equals`]
    /// - `"…"` → [`TokenKind::StringLit`]
    /// - `{…}` → [`TokenKind::Expression`]
    /// - `identifier` → [`TokenKind::Identifier`] (prop name)
    /// - other → [`TokenKind::Unknown`] + [`LexError`]
    fn scan_tag_body(&mut self, tokens: &mut Vec<Token>) {
        loop {
            // Consume whitespace between props.
            while matches!(self.peek(), Some(' ') | Some('\t') | Some('\n') | Some('\r')) {
                self.advance();
            }

            match self.peek() {
                Some('>') => {
                    let pos = self.current_pos();
                    self.advance();
                    tokens.push(Token { kind: TokenKind::CloseTag, value: ">".to_string(), pos });
                    return;
                }
                Some('/') if self.peek_at(1) == Some('>') => {
                    let pos = self.current_pos();
                    self.advance(); // '/'
                    self.advance(); // '>'
                    tokens.push(Token {
                        kind: TokenKind::SelfCloseTag,
                        value: "/>".to_string(),
                        pos,
                    });
                    return;
                }
                Some('=') => {
                    let pos = self.current_pos();
                    self.advance();
                    tokens.push(Token { kind: TokenKind::Equals, value: "=".to_string(), pos });
                }
                Some('"') => {
                    let pos = self.current_pos();
                    self.advance(); // opening '"'
                    let mut value = String::new();
                    loop {
                        match self.peek() {
                            Some('"') => {
                                self.advance();
                                break;
                            }
                            Some(c) => {
                                value.push(c);
                                self.advance();
                            }
                            None => break,
                        }
                    }
                    tokens.push(Token { kind: TokenKind::StringLit, value, pos });
                }
                Some('{') => {
                    let pos = self.current_pos();
                    self.advance(); // '{'
                    let mut value = String::new();
                    loop {
                        match self.peek() {
                            Some('}') => {
                                self.advance();
                                break;
                            }
                            Some(c) => {
                                value.push(c);
                                self.advance();
                            }
                            None => break,
                        }
                    }
                    tokens.push(Token { kind: TokenKind::Expression, value, pos });
                }
                Some(c) if c.is_alphabetic() || c == '_' => {
                    let pos = self.current_pos();
                    let name = self.collect_identifier();
                    tokens.push(Token { kind: TokenKind::Identifier, value: name, pos });
                }
                None => return,
                _ => {
                    let pos = self.current_pos();
                    let c = self.advance().unwrap();
                    tokens.push(Token {
                        kind: TokenKind::Unknown,
                        value: c.to_string(),
                        pos: pos.clone(),
                    });
                    self.errors.emit(LexError { code: codes::L002, message: format!("unexpected character '{}' in tag", c), pos });
                }
            }
        }
    }

    /// Collects an alphanumeric/underscore/hyphen sequence as a name.
    ///
    /// Used for tag names (`Column`, `Text`, `if`) and prop names (`gap`, `as`).
    /// Hyphens are included to support future kebab-case names if needed.
    fn collect_identifier(&mut self) -> String {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        name
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns whether a character can be part of static text in the template.
///
/// Text characters are alphanumeric plus a set of common punctuation.
/// `<`, `{`, spaces, and other special characters are **not** text characters:
/// they terminate the current `Text` token.
fn is_text_char(c: char) -> bool {
    c.is_alphanumeric()
        || matches!(c, '.' | ',' | '!' | '?' | ':' | ';' | '\'' | '(' | ')' | '[' | ']' | '-' | '_')
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
