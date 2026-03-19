//! Parser del compilatore Clutter: dal flusso di token all'AST.
//!
//! Questo crate riceve il flusso di [`Token`] prodotto da `clutter-lexer` e costruisce
//! un [`ProgramNode`] — la radice dell'AST — pronto per l'analisi semantica.
//!
//! # Struttura del file `.clutter`
//!
//! ```text
//! [logic block TypeScript — opaco]    ← TokenKind::LogicBlock
//! ---                                  ← TokenKind::SectionSeparator
//! [template — nodi JSX-like]           ← sequenza di tag / testo / espressioni
//! ```
//!
//! Il parser processa i token in ordine e costruisce l'albero ricorsivamente:
//! ogni tag aperto (`<Name`) avvia il parsing del nodo corrispondente, che
//! raccoglie le prop e poi ricorre sui figli fino al tag di chiusura (`</Name>`).
//!
//! # Strategia di error recovery
//!
//! Il parser non si ferma al primo errore. Quando incontra un token inatteso:
//!
//! - **a livello di prop** (`parse_prop` ritorna `Err`): salta token fino alla
//!   prossima *prop boundary* (whitespace, `>`, `/>`, EOF) e continua le prop
//!   successive.
//! - **a livello di nodo** (token inatteso nella sequenza di template): salta token
//!   fino alla prossima *tag boundary* (`>`, `</…>`, EOF) e continua.
//! - **`<else>` orfano** (fuori da `<if>`): emette un errore specifico e consuma
//!   l'intero blocco `<else>…</else>` prima di riprendere.
//!
//! Tutti gli errori vengono raccolti in un `Vec<ParseError>` restituito insieme
//! al `ProgramNode` parzialmente costruito.
//!
//! # Utilizzo
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

/// Parser del template Clutter.
///
/// Consuma un flusso di [`Token`] (prodotto da `clutter-lexer`) e costruisce il
/// [`ProgramNode`] corrispondente. La struttura interna è un cursore sul vettore
/// di token (`pos`) e un accumulatore di errori (`errors`).
///
/// Crea un `Parser` con [`Parser::new`] e avvia il parsing con
/// [`Parser::parse_program`].
pub struct Parser {
    /// Il flusso completo di token prodotto dal lexer.
    tokens: Vec<Token>,
    /// Indice del token corrente (cursore).
    pos: usize,
    /// Errori accumulati durante il parsing (error recovery).
    errors: Vec<ParseError>,
}

impl Parser {
    /// Crea un nuovo `Parser` dal flusso di token.
    ///
    /// Il vettore deve terminare con un token [`TokenKind::Eof`]; il lexer lo
    /// garantisce sempre. Senza `Eof` finale i metodi `peek`/`advance`
    /// potrebbero andare fuori dai limiti.
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0, errors: Vec::new() }
    }

    /// Ritorna un riferimento al token corrente senza consumarlo.
    ///
    /// Se il cursore è già sull'ultimo token (`Eof`), restituisce sempre quel
    /// token — mai un accesso fuori dai limiti.
    fn peek(&self) -> &Token {
        // Always safe: tokenize always ends with Eof
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    /// Consuma il token corrente e avanza il cursore.
    ///
    /// Ritorna il token consumato. Se il cursore è già sull'`Eof`, lo restituisce
    /// senza avanzare ulteriormente (il cursore rimane fermo sull'ultimo token).
    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    /// Consuma il token corrente solo se ha il `kind` atteso.
    ///
    /// # Restituisce
    ///
    /// - `Ok(token)` se il token corrente corrisponde a `kind`.
    /// - `Err(ParseError)` se il token corrente è diverso da `kind`; il cursore
    ///   **non** avanza e l'errore descrive il mismatch.
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

    /// Salta tutti i token [`TokenKind::Whitespace`] consecutivi.
    ///
    /// Usato sistematicamente prima di ogni `peek` significativo per ignorare
    /// la spaziatura strutturale tra tag e prop.
    fn skip_whitespace(&mut self) {
        while self.peek().kind == TokenKind::Whitespace {
            self.advance();
        }
    }

    /// Aggiunge un errore all'accumulatore interno (`self.errors`).
    ///
    /// Centralizza la creazione di [`ParseError`] in modo che tutti i siti di
    /// errore usino lo stesso pattern — analogo al metodo omonimo in `clutter-lexer`.
    fn emit(&mut self, message: impl Into<String>, pos: Position) {
        self.errors.push(ParseError { message: message.into(), pos });
    }

    /// Punto di ingresso pubblico: esegue il parsing dell'intero file `.clutter`.
    ///
    /// Consuma il [`TokenKind::LogicBlock`] (se presente) e il
    /// [`TokenKind::SectionSeparator`] (`---`), poi delega il parsing del
    /// template a [`Self::parse_nodes`].
    ///
    /// # Restituisce
    ///
    /// Una coppia `(ProgramNode, Vec<ParseError>)`:
    /// - `ProgramNode` contiene il logic block grezzo e i nodi di primo livello
    ///   del template.
    /// - `Vec<ParseError>` contiene tutti gli errori accumulati durante il parsing
    ///   (può essere non vuoto anche se `ProgramNode` è parzialmente costruito).
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

    /// Raccoglie una sequenza di nodi del template fino a una condizione di stop.
    ///
    /// Chiama [`Self::parse_node`] in loop fino a incontrare uno dei token di stop:
    /// - [`TokenKind::CloseOpenTag`] (`</…>`) — fine del blocco figlio corrente.
    /// - [`TokenKind::Eof`] — fine del file.
    /// - [`TokenKind::ElseOpen`] — solo se `allow_else = true` (ramo then di `<if>`).
    ///
    /// # Parametro `allow_else`
    ///
    /// - `true`: il token `ElseOpen` fa terminare il loop *senza consumarlo*. È
    ///   usato da [`Self::parse_if`] per delimitare il ramo `then`.
    /// - `false`: `ElseOpen` non è un token di stop valido; se incontrato viene
    ///   passato a [`Self::parse_node`] che lo tratta come `<else>` orfano e
    ///   emette un errore.
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

    /// Riconosce e delega il parsing del singolo nodo corrente nel template.
    ///
    /// Ispeziona il token corrente con `peek` e smista:
    ///
    /// | Token             | Azione                                               |
    /// |-------------------|------------------------------------------------------|
    /// | `OpenTag`         | Avanza, chiama [`Self::parse_component`]             |
    /// | `IfOpen`          | Avanza, chiama [`Self::parse_if`]                    |
    /// | `EachOpen`        | Avanza, chiama [`Self::parse_each`]                  |
    /// | `Text`            | Costruisce un [`TextNode`]                           |
    /// | `Expression`      | Costruisce un [`ExpressionNode`]                     |
    /// | `Whitespace`      | Consuma e ritorna `None` (ignorato)                  |
    /// | `ElseOpen`        | `<else>` orfano — errore, consuma fino a `</else>`   |
    /// | altri             | Token inatteso — errore, avanza fino a tag boundary  |
    ///
    /// # Restituisce
    ///
    /// `Some(Node)` se il token genera un nodo, `None` se è ignorato (whitespace)
    /// o se l'error recovery non produce un nodo valido.
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

    /// Esegue il parsing di un componente già identificato dal suo `OpenTag`.
    ///
    /// Viene chiamato da [`Self::parse_node`] dopo che l'`OpenTag` è stato
    /// consumato e il nome estratto. Parsing:
    ///
    /// 1. Raccoglie le prop con [`Self::parse_props`].
    /// 2. Se il token successivo è `SelfCloseTag` (`/>`): ritorna subito un
    ///    `ComponentNode` senza figli.
    /// 3. Altrimenti si aspetta `CloseTag` (`>`), poi raccoglie i figli con
    ///    [`Self::parse_nodes`], poi si aspetta `CloseOpenTag` (`</Name>`).
    ///
    /// Gli errori su `CloseTag` e `CloseOpenTag` mancanti vengono emessi e il
    /// parsing continua (best-effort).
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

    /// Raccoglie tutte le prop di un tag fino al marcatore di fine-props.
    ///
    /// Chiama [`Self::parse_prop`] in loop; si ferma quando incontra
    /// `CloseTag`, `SelfCloseTag` o `Eof`.
    ///
    /// # Error recovery
    ///
    /// Se `parse_prop` ritorna un errore, l'errore viene emesso e il cursore
    /// avanza fino alla prossima *prop boundary* (whitespace, `>`, `/>`, EOF),
    /// dopodiché il loop riprende con la prop successiva.
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

    /// Esegue il parsing di una singola prop `name=value`.
    ///
    /// Sequenza attesa di token:
    /// ```text
    /// Identifier  Equals  ( StringLit | Expression )
    /// ```
    ///
    /// # Restituisce
    ///
    /// - `Ok(PropNode)` con `name`, `value` (`StringValue` o `ExpressionValue`),
    ///   e la posizione del token `Identifier`.
    /// - `Err(ParseError)` se uno qualsiasi dei token attesi manca o ha kind
    ///   diverso. In questo caso il cursore si ferma al token inatteso; è compito
    ///   del chiamante fare error recovery.
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

    /// Esegue il parsing di un nodo condizionale `<if condition={expr}>`.
    ///
    /// Viene chiamato da [`Self::parse_node`] dopo che l'`IfOpen` è stato consumato.
    /// Parsing:
    ///
    /// 1. Legge la prop `condition={expr}` tramite [`Self::parse_prop`].
    /// 2. Consuma `CloseTag` (`>`).
    /// 3. Raccoglie i figli del ramo `then` con `parse_nodes(allow_else=true)`:
    ///    il loop si ferma senza consumare `ElseOpen`.
    /// 4. Se il token successivo è `ElseOpen`: consuma `<else>`, raccoglie i
    ///    figli del ramo `else`, consuma `</else>`. Imposta `else_children`.
    /// 5. Consuma `</if>`.
    ///
    /// # Parametro `pos`
    ///
    /// La posizione del token `<if` originale, passata dal chiamante prima che
    /// il token venisse consumato.
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

    /// Esegue il parsing di un nodo iterativo `<each collection={expr} as="alias">`.
    ///
    /// Viene chiamato da [`Self::parse_node`] dopo che l'`EachOpen` è stato consumato.
    /// Parsing:
    ///
    /// 1. Legge la prima prop (`collection={expr}`) tramite [`Self::parse_prop`].
    /// 2. Legge la seconda prop (`as="alias"`) tramite [`Self::parse_prop`].
    /// 3. Consuma `CloseTag` (`>`).
    /// 4. Raccoglie i figli del corpo con [`Self::parse_nodes`].
    /// 5. Consuma `</each>`.
    ///
    /// L'alias letto qui è un identificatore locale: l'analyzer aggiungerà
    /// `alias` all'insieme degli identificatori in scope prima di validare i
    /// figli (regola CLT104).
    ///
    /// # Parametro `pos`
    ///
    /// La posizione del token `<each` originale, passata dal chiamante prima che
    /// il token venisse consumato.
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
