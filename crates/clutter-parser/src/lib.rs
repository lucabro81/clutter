//! Parser for the Clutter compiler: from the token stream to the AST.
//!
//! This crate receives the [`Token`] stream produced by `clutter-lexer` and
//! constructs a [`FileNode`] — the root of the AST — ready for semantic analysis.
//!
//! # Structure of a `.clutter` file
//!
//! ```text
//! component Name(props: T) {
//!     [TypeScript logic block — opaque]    ← TokenKind::LogicBlock
//!     ----                                  ← TokenKind::SectionSeparator
//!     [template — JSX-like nodes]           ← sequence of tags / text / expressions
//! }
//! ```
//!
//! The parser processes tokens in order and builds the tree recursively: each
//! open tag (`<Name`) starts parsing the corresponding node, which collects
//! props and then recurses into children until the closing tag (`</Name>`).
//!
//! # Error recovery strategy
//!
//! The parser does not stop at the first error. When it encounters an unexpected
//! token:
//!
//! - **At prop level** (`parse_prop` returns `Err`): skips tokens until the next
//!   *prop boundary* (whitespace, `>`, `/>`, EOF) and continues with the next prop.
//! - **At node level** (unexpected token in the template sequence): skips tokens
//!   until the next *tag boundary* (`>`, `</…>`, EOF) and continues.
//! - **Orphan `<else>`** (outside an `<if>`): emits a specific error and consumes
//!   the entire `<else>…</else>` block before resuming.
//!
//! All errors are collected in a `Vec<ParseError>` returned alongside the
//! partially constructed [`FileNode`].
//!
//! # Usage
//!
//! ```rust,ignore
//! use clutter_lexer::tokenize;
//! use clutter_parser::Parser;
//!
//! let src = "component Main(props: P) {\n----\n<Text value={x} />\n}";
//! let (tokens, _lex_errors) = tokenize(src);
//! let (file, parse_errors) = Parser::new(tokens).parse_file();
//! ```

use clutter_runtime::{
    codes, ComponentDef, ComponentNode, DiagnosticCollector, EachNode, EventBinding,
    ExpressionNode, FileNode, IfNode, Node, ParseError, Position, PropNode, PropValue,
    TextNode, Token, TokenKind, UnsafeNode,
};

/// Clutter template parser.
///
/// Consumes a [`Token`] stream (produced by `clutter-lexer`) and constructs the
/// corresponding [`FileNode`]. The internal state is a cursor over the token
/// vector (`pos`) and an error accumulator (`errors`).
///
/// Create a `Parser` with [`Parser::new`] and start parsing with
/// [`Parser::parse_file`].
/// Attempts to parse an `unsafe('value', 'reason')` string literal.
///
/// Returns `Some((value, reason))` when the string starts with `unsafe(` and
/// ends with `)`. The `reason` field is `""` when only one argument is present.
/// Returns `None` if the string does not look like an `unsafe(...)` call at all.
fn parse_unsafe_call(s: &str) -> Option<(String, String)> {
    let inner = s.strip_prefix("unsafe(")?.strip_suffix(')')?;
    // Split on the first comma to separate value and reason.
    let (raw_value, raw_reason) = match inner.split_once(',') {
        Some((v, r)) => (v, r),
        None => (inner, ""),
    };
    let value = raw_value.trim().trim_matches('\'').to_string();
    let reason = raw_reason.trim().trim_matches('\'').to_string();
    Some((value, reason))
}

pub struct Parser {
    /// The complete token stream produced by the lexer.
    tokens: Vec<Token>,
    /// Index of the current token (cursor).
    pos: usize,
    /// Errors accumulated during parsing (error recovery).
    errors: DiagnosticCollector<ParseError>,
}

impl Parser {
    /// Creates a new `Parser` from a token stream.
    ///
    /// The vector must end with a [`TokenKind::Eof`] token; the lexer always
    /// guarantees this. Without a trailing `Eof`, `peek`/`advance` could go
    /// out of bounds.
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0, errors: DiagnosticCollector::new() }
    }

    /// Returns a reference to the current token without consuming it.
    ///
    /// If the cursor is already on the last token (`Eof`), always returns that
    /// token — never an out-of-bounds access.
    fn peek(&self) -> &Token {
        // Always safe: tokenize always ends with Eof
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    /// Consumes the current token and advances the cursor.
    ///
    /// Returns the consumed token. If the cursor is already on `Eof`, returns it
    /// without advancing further (cursor stays on the last token).
    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    /// Consumes the current token only if it has the expected `kind`.
    ///
    /// # Returns
    ///
    /// - `Ok(token)` if the current token matches `kind`.
    /// - `Err(ParseError)` if the current token differs from `kind`; the cursor
    ///   does **not** advance and the error describes the mismatch.
    fn expect(&mut self, kind: TokenKind) -> Result<Token, ParseError> {
        let tok = self.peek().clone();
        if tok.kind == kind {
            Ok(self.advance())
        } else {
            Err(ParseError {
                code: codes::P001,
                message: format!("expected {:?}, found {:?}", kind, tok.kind),
                pos: tok.pos,
            })
        }
    }

    /// Skips all consecutive [`TokenKind::Whitespace`] tokens.
    ///
    /// Called systematically before every significant `peek` to ignore structural
    /// whitespace between tags and props.
    fn skip_whitespace(&mut self) {
        while self.peek().kind == TokenKind::Whitespace {
            self.advance();
        }
    }

    /// Parses a `.clutter` file in the new multi-component format.
    ///
    /// Loops over the token stream collecting `component Name(…) { … }` blocks.
    /// Each block is parsed by [`Self::parse_component_def`] into a [`ComponentDef`].
    ///
    /// # Returns
    ///
    /// A pair `(FileNode, Vec<ParseError>)`:
    /// - `FileNode` contains all [`ComponentDef`]s declared in the file, in order.
    /// - `Vec<ParseError>` collects all errors; parsing continues on error.
    pub fn parse_file(&mut self) -> (FileNode, Vec<ParseError>) {
        let mut components = Vec::new();

        loop {
            self.skip_whitespace();
            if self.peek().kind == TokenKind::Eof {
                break;
            }
            let tok = self.peek().clone();
            match tok.kind {
                TokenKind::ComponentOpen { ref name, ref props_raw } => {
                    let name = name.clone();
                    let props_raw = props_raw.clone();
                    self.advance();
                    components.push(self.parse_component_def(name, props_raw));
                }
                _ => {
                    self.errors.emit(ParseError {
                        code: codes::P001,
                        message: format!("expected `component` block, found {:?}", tok.kind),
                        pos: tok.pos,
                    });
                    self.advance();
                }
            }
        }

        let errors = std::mem::take(&mut self.errors).into_vec();
        (FileNode { components }, errors)
    }

    /// Parses the body of a single component block after the `ComponentOpen` has
    /// been consumed.
    ///
    /// Reads `LogicBlock`, `SectionSeparator`, template nodes, and `ComponentClose`.
    fn parse_component_def(&mut self, name: String, props_raw: String) -> ComponentDef {
        let logic_block = if self.peek().kind == TokenKind::LogicBlock {
            self.advance().value
        } else {
            String::new()
        };

        if self.peek().kind == TokenKind::SectionSeparator {
            self.advance();
        } else {
            self.errors.emit(ParseError {
                code: codes::P001,
                message: format!("expected ---- separator in component '{}'", name),
                pos: self.peek().pos,
            });
        }

        self.skip_whitespace();
        let template = self.parse_nodes(false);

        if self.peek().kind == TokenKind::ComponentClose {
            self.advance();
        } else {
            self.errors.emit(ParseError {
                code: codes::P001,
                message: format!("expected `}}` to close component '{}'", name),
                pos: self.peek().pos,
            });
        }

        ComponentDef { name, props_raw, logic_block, template }
    }

    /// Collects a sequence of template nodes until a stop condition is met.
    ///
    /// Calls [`Self::parse_node`] in a loop until one of the stop tokens is seen:
    /// - [`TokenKind::CloseOpenTag`] (`</…>`) — end of the current child block.
    /// - [`TokenKind::Eof`] — end of file.
    /// - [`TokenKind::ElseOpen`] — only when `allow_else = true` (then-branch of `<if>`).
    ///
    /// # Parameter `allow_else`
    ///
    /// - `true`: the `ElseOpen` token stops the loop *without consuming it*. Used
    ///   by [`Self::parse_if`] to delimit the `then` branch.
    /// - `false`: `ElseOpen` is not a valid stop token; if encountered it is passed
    ///   to [`Self::parse_node`], which treats it as an orphan `<else>` and emits
    ///   an error.
    fn parse_nodes(&mut self, allow_else: bool) -> Vec<Node> {
        let mut nodes = Vec::new();
        loop {
            self.skip_whitespace();
            let stop = match self.peek().kind {
                TokenKind::CloseOpenTag | TokenKind::Eof | TokenKind::ComponentClose => true,
                TokenKind::ElseOpen => allow_else,
                _ => false,
            };
            if stop {
                break;
            }
            if let Some(node) = self.parse_node() {
                nodes.push(node);
            }
        }
        nodes
    }

    /// Recognises and delegates parsing of the current template node.
    ///
    /// Inspects the current token with `peek` and dispatches:
    ///
    /// | Token             | Action                                                |
    /// |-------------------|-------------------------------------------------------|
    /// | `OpenTag`         | Advances, calls [`Self::parse_component`]             |
    /// | `IfOpen`          | Advances, calls [`Self::parse_if`]                    |
    /// | `EachOpen`        | Advances, calls [`Self::parse_each`]                  |
    /// | `Text`            | Constructs a [`TextNode`]                             |
    /// | `Expression`      | Constructs an [`ExpressionNode`]                      |
    /// | `Whitespace`      | Consumes and returns `None` (ignored)                 |
    /// | `ElseOpen`        | Orphan `<else>` — error, consumes up to `</else>`     |
    /// | other             | Unexpected token — error, advances to tag boundary    |
    ///
    /// # Returns
    ///
    /// `Some(Node)` if the token produces a node, `None` if it is ignored
    /// (whitespace) or if error recovery does not yield a valid node.
    fn parse_node(&mut self) -> Option<Node> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::OpenTag => {
                let name = self.advance().value;
                Some(Node::Component(self.parse_component(name, tok.pos)))
            }
            TokenKind::IfOpen => {
                self.advance();
                Some(Node::If(self.parse_if(tok.pos)))
            }
            TokenKind::EachOpen => {
                self.advance();
                Some(Node::Each(self.parse_each(tok.pos)))
            }
            TokenKind::UnsafeOpen => {
                self.advance();
                Some(Node::Unsafe(self.parse_unsafe(tok.pos)))
            }
            TokenKind::Text => {
                let t = self.advance();
                Some(Node::Text(TextNode { value: t.value, pos: t.pos }))
            }
            TokenKind::Expression => {
                let t = self.advance();
                Some(Node::Expr(ExpressionNode { value: t.value, pos: t.pos }))
            }
            TokenKind::Whitespace => {
                self.advance();
                None
            }
            // ElseOpen only reaches parse_node when allow_else=false, i.e. always outside <if>
            TokenKind::ElseOpen => {
                self.errors.emit(ParseError {
                    code: codes::P002,
                    message: "<else> without matching <if>".to_string(),
                    pos: tok.pos,
                });
                while !matches!(self.peek().kind, TokenKind::CloseOpenTag | TokenKind::Eof) {
                    self.advance();
                }
                if self.peek().kind == TokenKind::CloseOpenTag {
                    self.advance();
                }
                None
            }
            _ => {
                self.errors.emit(ParseError {
                    code: codes::P001,
                    message: format!("unexpected token in template: {:?}", tok.kind),
                    pos: tok.pos.clone(),
                });
                while !matches!(
                    self.peek().kind,
                    TokenKind::CloseTag | TokenKind::CloseOpenTag | TokenKind::Eof
                ) {
                    self.advance();
                }
                None
            }
        }
    }

    /// Parses a component whose `OpenTag` has already been identified.
    ///
    /// Called by [`Self::parse_node`] after the `OpenTag` has been consumed and
    /// the name extracted. Parsing steps:
    ///
    /// 1. Collects props with [`Self::parse_props`].
    /// 2. If the next token is `SelfCloseTag` (`/>`): returns immediately with a
    ///    `ComponentNode` that has no children.
    /// 3. Otherwise expects `CloseTag` (`>`), collects children with
    ///    [`Self::parse_nodes`], then expects `CloseOpenTag` (`</Name>`).
    ///
    /// Errors on missing `CloseTag` and `CloseOpenTag` are emitted and parsing
    /// continues on a best-effort basis.
    fn parse_component(&mut self, name: String, pos: Position) -> ComponentNode {
        let (props, events) = self.parse_props();
        self.skip_whitespace();

        if self.peek().kind == TokenKind::SelfCloseTag {
            self.advance();
            return ComponentNode { name, props, events, children: vec![], pos };
        }

        if let Err(e) = self.expect(TokenKind::CloseTag) {
            self.errors.emit(e);
        }

        let children = self.parse_nodes(false);

        // Verify the closing tag exists AND matches the opening tag name.
        // Checking the kind alone (CloseOpenTag) is insufficient: </Row> satisfies
        // the kind check even when the opening tag was <Column>.
        let close_tok = self.peek().clone();
        if close_tok.kind == TokenKind::CloseOpenTag {
            if close_tok.value != name {
                self.errors.emit(ParseError {
                    code: codes::P001,
                    message: format!(
                        "expected </{}>, found </{}>",
                        name, close_tok.value
                    ),
                    pos: close_tok.pos,
                });
            }
            self.advance();
        } else if let Err(e) = self.expect(TokenKind::CloseOpenTag) {
            self.errors.emit(e);
        }

        ComponentNode { name, props, events, children, pos }
    }

    /// Collects all props of a tag until the end-of-props marker.
    ///
    /// Calls [`Self::parse_prop`] in a loop; stops when it sees `CloseTag`,
    /// `SelfCloseTag`, or `Eof`.
    ///
    /// # Error recovery
    ///
    /// If `parse_prop` returns an error, the error is emitted and the cursor
    /// advances to the next *prop boundary* (whitespace, `>`, `/>`, EOF), after
    /// which the loop resumes with the next prop.
    fn parse_props(&mut self) -> (Vec<PropNode>, Vec<EventBinding>) {
        let mut props = Vec::new();
        let mut events = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek().kind {
                TokenKind::CloseTag | TokenKind::SelfCloseTag | TokenKind::Eof => break,
                TokenKind::EventName => match self.parse_event() {
                    Ok(ev) => events.push(ev),
                    Err(e) => {
                        self.errors.emit(e);
                        while !matches!(
                            self.peek().kind,
                            TokenKind::Whitespace
                                | TokenKind::CloseTag
                                | TokenKind::SelfCloseTag
                                | TokenKind::Eof
                        ) {
                            self.advance();
                        }
                    }
                },
                _ => match self.parse_prop() {
                    Ok(prop) => props.push(prop),
                    Err(e) => {
                        self.errors.emit(e);
                        // recovery: skip to next prop boundary
                        while !matches!(
                            self.peek().kind,
                            TokenKind::Whitespace
                                | TokenKind::CloseTag
                                | TokenKind::SelfCloseTag
                                | TokenKind::Eof
                        ) {
                            self.advance();
                        }
                    }
                },
            }
        }
        (props, events)
    }

    /// Parses a single `@event={handler}` event binding.
    ///
    /// Expected token sequence: `EventName Equals Expression`
    fn parse_event(&mut self) -> Result<EventBinding, ParseError> {
        let name_tok = self.expect(TokenKind::EventName)?;
        self.skip_whitespace();
        self.expect(TokenKind::Equals)?;
        self.skip_whitespace();
        let val_tok = self.peek().clone();
        if val_tok.kind != TokenKind::Expression {
            return Err(ParseError {
                code: codes::P001,
                message: format!(
                    "expected expression for event handler, found {:?}",
                    val_tok.kind
                ),
                pos: val_tok.pos,
            });
        }
        self.advance();
        Ok(EventBinding { name: name_tok.value, handler: val_tok.value, pos: name_tok.pos })
    }

    /// Parses a single `name=value` prop.
    ///
    /// Expected token sequence:
    /// ```text
    /// Identifier  Equals  ( StringLit | Expression )
    /// ```
    ///
    /// # Returns
    ///
    /// - `Ok(PropNode)` with `name`, `value` (`StringValue` or `ExpressionValue`),
    ///   and the position of the `Identifier` token.
    /// - `Err(ParseError)` if any expected token is missing or has a different kind.
    ///   In this case the cursor stops at the unexpected token; the caller is
    ///   responsible for error recovery.
    fn parse_prop(&mut self) -> Result<PropNode, ParseError> {
        let name_tok = self.expect(TokenKind::Identifier)?;
        self.skip_whitespace();
        self.expect(TokenKind::Equals)?;
        self.skip_whitespace();

        let val_tok = self.peek().clone();
        let value = match val_tok.kind {
            TokenKind::StringLit => {
                self.advance();
                if let Some((value, reason)) = parse_unsafe_call(&val_tok.value) {
                    PropValue::UnsafeValue { value, reason }
                } else {
                    PropValue::StringValue(val_tok.value)
                }
            }
            TokenKind::Expression => {
                self.advance();
                PropValue::ExpressionValue(val_tok.value)
            }
            _ => {
                return Err(ParseError {
                    code: codes::P001,
                    message: format!("expected string or expression, found {:?}", val_tok.kind),
                    pos: val_tok.pos,
                })
            }
        };

        Ok(PropNode { name: name_tok.value, value, pos: name_tok.pos })
    }

    /// Parses a conditional node `<if condition={expr}>`.
    ///
    /// Called by [`Self::parse_node`] after the `IfOpen` token has been consumed.
    /// Parsing steps:
    ///
    /// 1. Reads the `condition={expr}` prop via [`Self::parse_prop`].
    /// 2. Consumes `CloseTag` (`>`).
    /// 3. Collects `then`-branch children with `parse_nodes(allow_else=true)`:
    ///    the loop stops without consuming `ElseOpen`.
    /// 4. If the next token is `ElseOpen`: consumes `<else>`, collects `else`-branch
    ///    children, consumes `</else>`. Sets `else_children`.
    /// 5. Consumes `</if>`.
    ///
    /// # Parameter `pos`
    ///
    /// Position of the original `<if` token, passed by the caller before the
    /// token was consumed.
    fn parse_if(&mut self, pos: Position) -> IfNode {
        // expect: condition={expr}
        self.skip_whitespace();
        let condition = match self.parse_prop() {
            Ok(prop) => match prop.value {
                PropValue::ExpressionValue(v) => v,
                PropValue::StringValue(v) => v,
                PropValue::UnsafeValue { value, .. } => value,
            },
            Err(e) => {
                self.errors.emit(e);
                String::new()
            }
        };

        self.skip_whitespace();
        if let Err(e) = self.expect(TokenKind::CloseTag) {
            self.errors.emit(e);
        }

        // then-branch: stop on ElseOpen
        let then_children = self.parse_nodes(true);

        // optional else-branch
        let else_children = if self.peek().kind == TokenKind::ElseOpen {
            self.advance(); // consume <else
            self.skip_whitespace();
            if let Err(e) = self.expect(TokenKind::CloseTag) {
                self.errors.emit(e);
            }
            let nodes = self.parse_nodes(false);
            if let Err(e) = self.expect(TokenKind::CloseOpenTag) { // </else>
                self.errors.emit(e);
            }
            Some(nodes)
        } else {
            None
        };

        self.skip_whitespace();
        if let Err(e) = self.expect(TokenKind::CloseOpenTag) { // </if>
            self.errors.emit(e);
        }

        IfNode { condition, then_children, else_children, pos }
    }

    /// Parses an iteration node `<each collection={expr} as="alias">`.
    ///
    /// Called by [`Self::parse_node`] after the `EachOpen` token has been consumed.
    /// Parsing steps:
    ///
    /// 1. Reads the first prop (`collection={expr}`) via [`Self::parse_prop`].
    /// 2. Reads the second prop (`as="alias"`) via [`Self::parse_prop`].
    /// 3. Consumes `CloseTag` (`>`).
    /// 4. Collects loop-body children with [`Self::parse_nodes`].
    /// 5. Consumes `</each>`.
    ///
    /// The alias read here is a local identifier: the analyzer will add `alias`
    /// to the in-scope identifier set before validating children (CLT104 rule).
    ///
    /// # Parameter `pos`
    ///
    /// Position of the original `<each` token, passed by the caller before the
    /// token was consumed.
    fn parse_each(&mut self, pos: Position) -> EachNode {
        // expect: collection={expr} as="alias"
        self.skip_whitespace();
        let collection = match self.parse_prop() {
            Ok(prop) => match prop.value {
                PropValue::ExpressionValue(v) => v,
                PropValue::StringValue(v) => v,
                PropValue::UnsafeValue { value, .. } => value,
            },
            Err(e) => {
                self.errors.emit(e);
                String::new()
            }
        };

        self.skip_whitespace();
        let alias = match self.parse_prop() {
            Ok(prop) => match prop.value {
                PropValue::StringValue(v) => v,
                PropValue::ExpressionValue(v) => v,
                PropValue::UnsafeValue { value, .. } => value,
            },
            Err(e) => {
                self.errors.emit(e);
                String::new()
            }
        };

        self.skip_whitespace();
        if let Err(e) = self.expect(TokenKind::CloseTag) {
            self.errors.emit(e);
        }

        let children = self.parse_nodes(false);

        if let Err(e) = self.expect(TokenKind::CloseOpenTag) {
            self.errors.emit(e);
        }

        EachNode { collection, alias, children, pos }
    }

    /// Parses an unsafe escape-hatch block `<unsafe reason="...">`.
    ///
    /// Called by [`Self::parse_node`] after the `UnsafeOpen` token has been consumed.
    /// Parsing steps:
    ///
    /// 1. Expects the `reason` identifier, `=`, and a `StringLit` value.
    ///    If the `reason` attr is missing, emits an error and stores `reason = ""`.
    /// 2. Consumes `CloseTag` (`>`).
    /// 3. Collects children with [`Self::parse_nodes`].
    /// 4. Consumes `</unsafe>`.
    ///
    /// Whether `reason` is non-empty is validated by the analyzer (CLT105).
    fn parse_unsafe(&mut self, pos: Position) -> UnsafeNode {
        self.skip_whitespace();

        // Expect `reason="..."`. Emit an error and use "" if missing.
        let reason = if self.peek().kind == TokenKind::Identifier
            && self.peek().value == "reason"
        {
            self.advance(); // consume `reason`
            self.skip_whitespace();
            if let Err(e) = self.expect(TokenKind::Equals) {
                self.errors.emit(e);
            }
            self.skip_whitespace();
            match self.expect(TokenKind::StringLit) {
                Ok(t) => t.value,
                Err(e) => {
                    self.errors.emit(e);
                    String::new()
                }
            }
        } else {
            self.errors.emit(ParseError {
                code: codes::P003,
                message: "expected `reason` attribute on <unsafe>".to_string(),
                pos: self.peek().pos.clone(),
            });
            String::new()
        };

        self.skip_whitespace();
        if let Err(e) = self.expect(TokenKind::CloseTag) {
            self.errors.emit(e);
        }

        let children = self.parse_nodes(false);

        if let Err(e) = self.expect(TokenKind::CloseOpenTag) {
            self.errors.emit(e);
        }

        UnsafeNode { reason, children, pos }
    }
}

#[cfg(test)]
mod tests;

