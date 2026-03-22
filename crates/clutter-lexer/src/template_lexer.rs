//! Finite-state lexer for the template section of a `.clutter` component block.

use clutter_runtime::{codes, DiagnosticCollector, LexError, Position, Token, TokenKind};

// ---------------------------------------------------------------------------
// TemplateLexer
// ---------------------------------------------------------------------------

/// Finite-state lexer for the template section of a `.clutter` file.
///
/// Operates on a slice of the source string starting immediately after the `\n`
/// of the `----` separator. Maintains the current position (`line`, `col`) to
/// attach precise [`Position`] values to every emitted token.
///
/// Not instantiated directly from outside: [`super::tokenize`] creates it
/// internally and calls [`TemplateLexer::scan`].
pub(super) struct TemplateLexer {
    /// The template source as a `char` vector (O(1) indexing).
    chars: Vec<char>,
    /// Index of the next character to read in `chars`.
    pos: usize,
    /// Current line number (1-based, already adjusted for the template offset).
    line: usize,
    /// Current column number (1-based).
    col: usize,
    /// Errors accumulated during scanning (drained by `tokenize` at the end).
    pub(super) errors: DiagnosticCollector<LexError>,
}

impl TemplateLexer {
    /// Creates a new `TemplateLexer`.
    ///
    /// `start_line` must be the line number immediately following the `----`
    /// separator in the original file, so that all positions are absolute.
    pub(super) fn new(input: &str, start_line: usize) -> Self {
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
    pub(super) fn scan(&mut self, tokens: &mut Vec<Token>) {
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
                    self.errors.emit(LexError {
                        code: codes::L002,
                        message: format!("unexpected character '{}' in template", c),
                        pos,
                    });
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
                    self.errors.emit(LexError {
                        code: codes::L002,
                        message: format!("unexpected character '{}' in tag", c),
                        pos,
                    });
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
