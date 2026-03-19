//! Parser for the Clutter compiler: from the token stream to the AST.
//!
//! This crate receives the [`Token`] stream produced by `clutter-lexer` and
//! constructs a [`ProgramNode`] — the root of the AST — ready for semantic analysis.
//!
//! # Structure of a `.clutter` file
//!
//! ```text
//! [TypeScript logic block — opaque]    ← TokenKind::LogicBlock
//! ---                                   ← TokenKind::SectionSeparator
//! [template — JSX-like nodes]           ← sequence of tags / text / expressions
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
//! partially constructed `ProgramNode`.
//!
//! # Usage
//!
//! ```rust,ignore
//! use clutter_lexer::tokenize;
//! use clutter_parser::Parser;
//!
//! let src = "const x = 1;\n---\n<Text value={x} />";
//! let (tokens, _lex_errors) = tokenize(src);
//! let (program, parse_errors) = Parser::new(tokens).parse_program();
//! ```

use clutter_runtime::{
    ComponentNode, EachNode, ExpressionNode, IfNode, Node, ParseError, Position, ProgramNode,
    PropNode, PropValue, TextNode, Token, TokenKind,
};

/// Clutter template parser.
///
/// Consumes a [`Token`] stream (produced by `clutter-lexer`) and constructs the
/// corresponding [`ProgramNode`]. The internal state is a cursor over the token
/// vector (`pos`) and an error accumulator (`errors`).
///
/// Create a `Parser` with [`Parser::new`] and start parsing with
/// [`Parser::parse_program`].
pub struct Parser {
    /// The complete token stream produced by the lexer.
    tokens: Vec<Token>,
    /// Index of the current token (cursor).
    pos: usize,
    /// Errors accumulated during parsing (error recovery).
    errors: Vec<ParseError>,
}

impl Parser {
    /// Creates a new `Parser` from a token stream.
    ///
    /// The vector must end with a [`TokenKind::Eof`] token; the lexer always
    /// guarantees this. Without a trailing `Eof`, `peek`/`advance` could go
    /// out of bounds.
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0, errors: Vec::new() }
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

    /// Appends an error to the internal accumulator (`self.errors`).
    ///
    /// Centralises [`ParseError`] creation so that all error sites use the same
    /// pattern — analogous to the same-named method in `clutter-lexer`.
    fn emit(&mut self, message: impl Into<String>, pos: Position) {
        self.errors.push(ParseError { message: message.into(), pos });
    }

    /// Public entry point: parses an entire `.clutter` file.
    ///
    /// Consumes the [`TokenKind::LogicBlock`] (if present) and the
    /// [`TokenKind::SectionSeparator`] (`---`), then delegates template parsing
    /// to [`Self::parse_nodes`].
    ///
    /// # Returns
    ///
    /// A pair `(ProgramNode, Vec<ParseError>)`:
    /// - `ProgramNode` contains the raw logic block and the top-level template nodes.
    /// - `Vec<ParseError>` contains all errors accumulated during parsing (may be
    ///   non-empty even if `ProgramNode` is partially constructed).
    pub fn parse_program(&mut self) -> (ProgramNode, Vec<ParseError>) {
        // Lexer always emits LogicBlock + SectionSeparator first
        let logic_block = if self.peek().kind == TokenKind::LogicBlock {
            self.advance().value
        } else {
            String::new()
        };

        if self.peek().kind == TokenKind::SectionSeparator {
            self.advance();
        }

        self.skip_whitespace();
        let template = self.parse_nodes(false);

        let errors = std::mem::take(&mut self.errors);
        (ProgramNode { logic_block, template }, errors)
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
                TokenKind::CloseOpenTag | TokenKind::Eof => true,
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
                self.emit("<else> without matching <if>", tok.pos);
                while !matches!(self.peek().kind, TokenKind::CloseOpenTag | TokenKind::Eof) {
                    self.advance();
                }
                if self.peek().kind == TokenKind::CloseOpenTag {
                    self.advance();
                }
                None
            }
            _ => {
                self.emit(format!("unexpected token in template: {:?}", tok.kind), tok.pos.clone());
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
        let props = self.parse_props();
        self.skip_whitespace();

        if self.peek().kind == TokenKind::SelfCloseTag {
            self.advance();
            return ComponentNode { name, props, children: vec![], pos };
        }

        if let Err(e) = self.expect(TokenKind::CloseTag) {
            self.emit(e.message, e.pos);
        }

        let children = self.parse_nodes(false);

        if let Err(e) = self.expect(TokenKind::CloseOpenTag) {
            self.emit(e.message, e.pos);
        }

        ComponentNode { name, props, children, pos }
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
    fn parse_props(&mut self) -> Vec<PropNode> {
        let mut props = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek().kind {
                TokenKind::CloseTag | TokenKind::SelfCloseTag | TokenKind::Eof => break,
                _ => match self.parse_prop() {
                    Ok(prop) => props.push(prop),
                    Err(e) => {
                        self.emit(e.message, e.pos);
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
        props
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
                PropValue::StringValue(val_tok.value)
            }
            TokenKind::Expression => {
                self.advance();
                PropValue::ExpressionValue(val_tok.value)
            }
            _ => {
                return Err(ParseError {
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
            },
            Err(e) => {
                self.emit(e.message, e.pos);
                String::new()
            }
        };

        self.skip_whitespace();
        if let Err(e) = self.expect(TokenKind::CloseTag) {
            self.emit(e.message, e.pos);
        }

        // then-branch: stop on ElseOpen
        let then_children = self.parse_nodes(true);

        // optional else-branch
        let else_children = if self.peek().kind == TokenKind::ElseOpen {
            self.advance(); // consume <else
            self.skip_whitespace();
            if let Err(e) = self.expect(TokenKind::CloseTag) {
                self.emit(e.message, e.pos);
            }
            let nodes = self.parse_nodes(false);
            if let Err(e) = self.expect(TokenKind::CloseOpenTag) { // </else>
                self.emit(e.message, e.pos);
            }
            Some(nodes)
        } else {
            None
        };

        self.skip_whitespace();
        if let Err(e) = self.expect(TokenKind::CloseOpenTag) { // </if>
            self.emit(e.message, e.pos);
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
            },
            Err(e) => {
                self.emit(e.message, e.pos);
                String::new()
            }
        };

        self.skip_whitespace();
        let alias = match self.parse_prop() {
            Ok(prop) => match prop.value {
                PropValue::StringValue(v) => v,
                PropValue::ExpressionValue(v) => v,
            },
            Err(e) => {
                self.emit(e.message, e.pos);
                String::new()
            }
        };

        self.skip_whitespace();
        if let Err(e) = self.expect(TokenKind::CloseTag) {
            self.emit(e.message, e.pos);
        }

        let children = self.parse_nodes(false);

        if let Err(e) = self.expect(TokenKind::CloseOpenTag) {
            self.emit(e.message, e.pos);
        }

        EachNode { collection, alias, children, pos }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clutter_runtime::TokenKind::*;

    fn tok(kind: TokenKind, value: &str) -> Token {
        Token { kind, value: value.to_string(), pos: Position { line: 1, col: 1 } }
    }

    fn program_tokens(template: Vec<Token>) -> Vec<Token> {
        let mut tokens = vec![tok(LogicBlock, ""), tok(SectionSeparator, "---")];
        tokens.extend(template);
        tokens.push(tok(Eof, ""));
        tokens
    }

    // 1. Single component, no props
    #[test]
    fn single_component_no_props() {
        let tokens = program_tokens(vec![
            tok(OpenTag, "Column"),
            tok(CloseTag, ">"),
            tok(CloseOpenTag, "Column"),
        ]);
        let (program, errors) = Parser::new(tokens).parse_program();
        assert!(errors.is_empty());
        assert_eq!(program.template.len(), 1);
        match &program.template[0] {
            Node::Component(c) => {
                assert_eq!(c.name, "Column");
                assert!(c.props.is_empty());
                assert!(c.children.is_empty());
            }
            _ => panic!("expected ComponentNode"),
        }
    }

    // 2. Component with string prop
    #[test]
    fn component_string_prop() {
        let tokens = program_tokens(vec![
            tok(OpenTag, "Text"),
            tok(Identifier, "size"),
            tok(Equals, "="),
            tok(StringLit, "md"),
            tok(SelfCloseTag, "/>"),
        ]);
        let (program, errors) = Parser::new(tokens).parse_program();
        assert!(errors.is_empty());
        match &program.template[0] {
            Node::Component(c) => {
                assert_eq!(c.props.len(), 1);
                assert_eq!(c.props[0].name, "size");
                assert_eq!(c.props[0].value, PropValue::StringValue("md".to_string()));
            }
            _ => panic!("expected ComponentNode"),
        }
    }

    // 3. Component with expression prop
    #[test]
    fn component_expression_prop() {
        let tokens = program_tokens(vec![
            tok(OpenTag, "Text"),
            tok(Identifier, "size"),
            tok(Equals, "="),
            tok(Expression, "size"),
            tok(SelfCloseTag, "/>"),
        ]);
        let (program, errors) = Parser::new(tokens).parse_program();
        assert!(errors.is_empty());
        match &program.template[0] {
            Node::Component(c) => {
                assert_eq!(c.props[0].value, PropValue::ExpressionValue("size".to_string()));
            }
            _ => panic!("expected ComponentNode"),
        }
    }

    // 4. Two-level nesting: <Column><Text /></Column>
    #[test]
    fn two_level_nesting() {
        let tokens = program_tokens(vec![
            tok(OpenTag, "Column"),
            tok(CloseTag, ">"),
            tok(OpenTag, "Text"),
            tok(SelfCloseTag, "/>"),
            tok(CloseOpenTag, "Column"),
        ]);
        let (program, errors) = Parser::new(tokens).parse_program();
        assert!(errors.is_empty());
        match &program.template[0] {
            Node::Component(column) => {
                assert_eq!(column.children.len(), 1);
                match &column.children[0] {
                    Node::Component(text) => assert_eq!(text.name, "Text"),
                    _ => panic!("expected ComponentNode child"),
                }
            }
            _ => panic!("expected ComponentNode"),
        }
    }

    // 5. Deep nesting (3 levels): <A><B><C /></B></A>
    #[test]
    fn deep_nesting() {
        let tokens = program_tokens(vec![
            tok(OpenTag, "A"),
            tok(CloseTag, ">"),
            tok(OpenTag, "B"),
            tok(CloseTag, ">"),
            tok(OpenTag, "C"),
            tok(SelfCloseTag, "/>"),
            tok(CloseOpenTag, "B"),
            tok(CloseOpenTag, "A"),
        ]);
        let (program, errors) = Parser::new(tokens).parse_program();
        assert!(errors.is_empty());
        match &program.template[0] {
            Node::Component(a) => match &a.children[0] {
                Node::Component(b) => match &b.children[0] {
                    Node::Component(c) => assert_eq!(c.name, "C"),
                    _ => panic!("expected C"),
                },
                _ => panic!("expected B"),
            },
            _ => panic!("expected A"),
        }
    }

    // 6. Self-closing component: <Text />
    #[test]
    fn self_closing_component() {
        let tokens = program_tokens(vec![tok(OpenTag, "Text"), tok(SelfCloseTag, "/>")]);
        let (program, errors) = Parser::new(tokens).parse_program();
        assert!(errors.is_empty());
        match &program.template[0] {
            Node::Component(c) => {
                assert_eq!(c.name, "Text");
                assert!(c.children.is_empty());
            }
            _ => panic!("expected ComponentNode"),
        }
    }

    // 7. <if condition={x}> without <else> → IfNode { else_children: None }
    #[test]
    fn if_without_else() {
        let tokens = program_tokens(vec![
            tok(IfOpen, "if"),
            tok(Identifier, "condition"),
            tok(Equals, "="),
            tok(Expression, "x"),
            tok(CloseTag, ">"),
            tok(OpenTag, "Text"),
            tok(SelfCloseTag, "/>"),
            tok(CloseOpenTag, "if"),
        ]);
        let (program, errors) = Parser::new(tokens).parse_program();
        assert!(errors.is_empty());
        match &program.template[0] {
            Node::If(n) => {
                assert_eq!(n.condition, "x");
                assert_eq!(n.then_children.len(), 1);
                assert!(n.else_children.is_none());
            }
            _ => panic!("expected IfNode"),
        }
    }

    // 8. <if> with <else> → IfNode { else_children: Some([...]) }
    #[test]
    fn if_with_else() {
        let tokens = program_tokens(vec![
            tok(IfOpen, "if"),
            tok(Identifier, "condition"),
            tok(Equals, "="),
            tok(Expression, "x"),
            tok(CloseTag, ">"),
            tok(OpenTag, "A"),
            tok(SelfCloseTag, "/>"),
            tok(ElseOpen, "else"),
            tok(CloseTag, ">"),
            tok(OpenTag, "B"),
            tok(SelfCloseTag, "/>"),
            tok(CloseOpenTag, "else"),
            tok(CloseOpenTag, "if"),
        ]);
        let (program, errors) = Parser::new(tokens).parse_program();
        assert!(errors.is_empty());
        match &program.template[0] {
            Node::If(n) => {
                assert_eq!(n.then_children.len(), 1);
                let else_kids = n.else_children.as_ref().expect("expected else branch");
                assert_eq!(else_kids.len(), 1);
            }
            _ => panic!("expected IfNode"),
        }
    }

    // 9. <each collection={items} as="item">
    #[test]
    fn each_node() {
        let tokens = program_tokens(vec![
            tok(EachOpen, "each"),
            tok(Identifier, "collection"),
            tok(Equals, "="),
            tok(Expression, "items"),
            tok(Identifier, "as"),
            tok(Equals, "="),
            tok(StringLit, "item"),
            tok(CloseTag, ">"),
            tok(OpenTag, "Text"),
            tok(SelfCloseTag, "/>"),
            tok(CloseOpenTag, "each"),
        ]);
        let (program, errors) = Parser::new(tokens).parse_program();
        assert!(errors.is_empty());
        match &program.template[0] {
            Node::Each(n) => {
                assert_eq!(n.collection, "items");
                assert_eq!(n.alias, "item");
                assert_eq!(n.children.len(), 1);
            }
            _ => panic!("expected EachNode"),
        }
    }

    // 10. Non-empty logic block → ProgramNode.logic_block contains the raw TypeScript string
    #[test]
    fn non_empty_logic_block() {
        let tokens = vec![
            tok(LogicBlock, "const x = 1;"),
            tok(SectionSeparator, "---"),
            tok(OpenTag, "Text"),
            tok(SelfCloseTag, "/>"),
            tok(Eof, ""),
        ];
        let (program, errors) = Parser::new(tokens).parse_program();
        assert!(errors.is_empty());
        assert_eq!(program.logic_block, "const x = 1;");
    }

    // 11. Unclosed tag → ParseError
    #[test]
    fn unclosed_tag_is_parse_error() {
        let tokens = program_tokens(vec![
            tok(OpenTag, "Column"),
            tok(CloseTag, ">"),
            // no CloseOpenTag
        ]);
        let (_program, errors) = Parser::new(tokens).parse_program();
        assert!(!errors.is_empty());
    }

    // 12. Prop without = or value → ParseError
    #[test]
    fn prop_without_value_is_parse_error() {
        let tokens = program_tokens(vec![
            tok(OpenTag, "Text"),
            tok(Identifier, "size"),
            tok(CloseTag, ">"),
            tok(CloseOpenTag, "Text"),
        ]);
        let (_program, errors) = Parser::new(tokens).parse_program();
        assert!(!errors.is_empty());
    }

    // 13. <else> outside any <if> → ParseError with descriptive message
    #[test]
    fn else_without_if_is_parse_error() {
        let tokens = program_tokens(vec![
            tok(ElseOpen, "else"),
            tok(CloseTag, ">"),
            tok(OpenTag, "Text"),
            tok(SelfCloseTag, "/>"),
            tok(CloseOpenTag, "else"),
        ]);
        let (_program, errors) = Parser::new(tokens).parse_program();
        assert!(!errors.is_empty());
        assert_eq!(errors[0].message, "<else> without matching <if>");
    }
}
