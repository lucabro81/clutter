use clutter_runtime::{LexError, Position, Token, TokenKind};

pub fn tokenize(input: &str) -> (Vec<Token>, Vec<LexError>) {
    let mut tokens: Vec<Token> = Vec::new();
    let mut errors: Vec<LexError> = Vec::new();

    match find_separator(input) {
        None => {
            errors.push(LexError {
                message: "missing --- separator: the logic and template sections must be separated by ---".to_string(),
                pos: Position { line: 1, col: 1 },
            });
            tokens.push(Token {
                kind: TokenKind::Eof,
                value: String::new(),
                pos: Position { line: 1, col: 1 },
            });
        }
        Some((logic_content, sep_line, template_offset)) => {
            tokens.push(Token {
                kind: TokenKind::LogicBlock,
                value: logic_content.to_string(),
                pos: Position { line: 1, col: 1 },
            });
            tokens.push(Token {
                kind: TokenKind::SectionSeparator,
                value: "---".to_string(),
                pos: Position { line: sep_line, col: 1 },
            });

            let template_str = &input[template_offset..];
            let mut lex = TemplateLexer::new(template_str, sep_line + 1);
            lex.scan(&mut tokens, &mut errors);

            let eof_pos = lex.current_pos();
            tokens.push(Token {
                kind: TokenKind::Eof,
                value: String::new(),
                pos: eof_pos,
            });
        }
    }

    (tokens, errors)
}

/// Returns `(logic_content, sep_line_number, byte_offset_of_template_start)`.
fn find_separator(input: &str) -> Option<(&str, usize, usize)> {
    let mut line_start = 0usize;
    let mut current_line = 1usize;

    loop {
        match input[line_start..].find('\n') {
            None => {
                // Last line with no trailing newline.
                let line = &input[line_start..];
                if line == "---" {
                    let logic = input[..line_start].trim_end_matches('\n');
                    return Some((logic, current_line, input.len()));
                }
                return None;
            }
            Some(offset) => {
                let line_end = line_start + offset;
                let line = &input[line_start..line_end];
                if line == "---" {
                    let logic = input[..line_start].trim_end_matches('\n');
                    let template_start = line_end + 1; // skip the '\n' after ---
                    return Some((logic, current_line, template_start));
                }
                current_line += 1;
                line_start = line_end + 1;
                if line_start > input.len() {
                    return None;
                }
            }
        }
    }
}

struct TemplateLexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl TemplateLexer {
    fn new(input: &str, start_line: usize) -> Self {
        TemplateLexer {
            chars: input.chars().collect(),
            pos: 0,
            line: start_line,
            col: 1,
        }
    }

    fn current_pos(&self) -> Position {
        Position { line: self.line, col: self.col }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

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

    fn scan(&mut self, tokens: &mut Vec<Token>, errors: &mut Vec<LexError>) {
        while let Some(ch) = self.peek() {
            match ch {
                '<' => self.scan_tag(tokens, errors),
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
                    errors.push(LexError {
                        message: format!("unexpected character '{}' in template", c),
                        pos,
                    });
                }
            }
        }
    }

    fn scan_tag(&mut self, tokens: &mut Vec<Token>, errors: &mut Vec<LexError>) {
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
            _ => TokenKind::OpenTag,
        };
        tokens.push(Token { kind, value: name, pos: tag_start });

        self.scan_tag_body(tokens, errors);
    }

    fn scan_tag_body(&mut self, tokens: &mut Vec<Token>, errors: &mut Vec<LexError>) {
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
                    errors.push(LexError {
                        message: format!("unexpected character '{}' in tag", c),
                        pos,
                    });
                }
            }
        }
    }

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

fn is_text_char(c: char) -> bool {
    c.is_alphanumeric()
        || matches!(c, '.' | ',' | '!' | '?' | ':' | ';' | '\'' | '(' | ')' | '[' | ']' | '-' | '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use clutter_runtime::TokenKind::*;

    fn kinds(tokens: &[Token]) -> Vec<TokenKind> {
        tokens.iter().map(|t| t.kind.clone()).collect()
    }

    // 1. Minimal file: just "---" and empty template
    #[test]
    fn minimal_file() {
        let (tokens, errors) = tokenize("---\n");
        assert!(errors.is_empty());
        assert_eq!(kinds(&tokens), vec![LogicBlock, SectionSeparator, Eof]);
        assert_eq!(tokens[0].value, "");
    }

    // 2. Component without props
    #[test]
    fn component_no_props() {
        let (tokens, errors) = tokenize("---\n<Column>");
        assert!(errors.is_empty());
        assert_eq!(
            kinds(&tokens),
            vec![LogicBlock, SectionSeparator, OpenTag, CloseTag, Eof]
        );
        assert_eq!(tokens[2].value, "Column");
    }

    // 3. Component with string prop, position check
    #[test]
    fn component_string_prop() {
        let (tokens, errors) = tokenize("---\n<Column gap=\"md\">");
        assert!(errors.is_empty());
        assert_eq!(
            kinds(&tokens),
            vec![
                LogicBlock,
                SectionSeparator,
                OpenTag,
                Identifier,
                Equals,
                StringLit,
                CloseTag,
                Eof
            ]
        );
        assert_eq!(tokens[2].value, "Column");
        assert_eq!(tokens[3].value, "gap");
        assert_eq!(tokens[5].value, "md");
        // OpenTag is on line 2
        assert_eq!(tokens[2].pos.line, 2);
    }

    // 4. Component with expression prop
    #[test]
    fn component_expression_prop() {
        let (tokens, errors) = tokenize("---\n<Column gap={size}>");
        assert!(errors.is_empty());
        assert_eq!(
            kinds(&tokens),
            vec![
                LogicBlock,
                SectionSeparator,
                OpenTag,
                Identifier,
                Equals,
                Expression,
                CloseTag,
                Eof
            ]
        );
        assert_eq!(tokens[5].value, "size");
    }

    // 5. Self-closing tag
    #[test]
    fn self_closing_tag() {
        let (tokens, errors) = tokenize("---\n<Text />");
        assert!(errors.is_empty());
        assert_eq!(
            kinds(&tokens),
            vec![LogicBlock, SectionSeparator, OpenTag, SelfCloseTag, Eof]
        );
        assert_eq!(tokens[2].value, "Text");
    }

    // 6. Closing tag
    #[test]
    fn closing_tag() {
        let (tokens, errors) = tokenize("---\n</Column>");
        assert!(errors.is_empty());
        assert_eq!(
            kinds(&tokens),
            vec![LogicBlock, SectionSeparator, CloseOpenTag, Eof]
        );
        assert_eq!(tokens[2].value, "Column");
    }

    // 7. Nesting
    #[test]
    fn nesting() {
        let (tokens, errors) = tokenize("---\n<Column><Text /></Column>");
        assert!(errors.is_empty());
        assert_eq!(
            kinds(&tokens),
            vec![
                LogicBlock,
                SectionSeparator,
                OpenTag,
                CloseTag,
                OpenTag,
                SelfCloseTag,
                CloseOpenTag,
                Eof
            ]
        );
        assert_eq!(tokens[2].value, "Column");
        assert_eq!(tokens[4].value, "Text");
        assert_eq!(tokens[6].value, "Column");
    }

    // 8. Logic section with real TypeScript
    #[test]
    fn logic_section() {
        let input = "const x = 1\nconst y = 2\n---\n<Text />";
        let (tokens, errors) = tokenize(input);
        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, LogicBlock);
        assert_eq!(tokens[0].value, "const x = 1\nconst y = 2");
        assert_eq!(tokens[1].kind, SectionSeparator);
    }

    // 9. Control flow: <if condition={x}>
    #[test]
    fn control_flow_if() {
        let (tokens, errors) = tokenize("---\n<if condition={x}>");
        assert!(errors.is_empty());
        assert_eq!(
            kinds(&tokens),
            vec![
                LogicBlock,
                SectionSeparator,
                IfOpen,
                Identifier,
                Equals,
                Expression,
                CloseTag,
                Eof
            ]
        );
        assert_eq!(tokens[3].value, "condition");
        assert_eq!(tokens[5].value, "x");
    }

    // 10. Control flow: <else>
    #[test]
    fn control_flow_else() {
        let (tokens, errors) = tokenize("---\n<else>");
        assert!(errors.is_empty());
        assert_eq!(
            kinds(&tokens),
            vec![LogicBlock, SectionSeparator, ElseOpen, CloseTag, Eof]
        );
    }

    // 11. Control flow: <each item={items} as="item">
    #[test]
    fn control_flow_each() {
        let (tokens, errors) = tokenize("---\n<each item={items} as=\"item\">");
        assert!(errors.is_empty());
        assert_eq!(
            kinds(&tokens),
            vec![
                LogicBlock,
                SectionSeparator,
                EachOpen,
                Identifier,
                Equals,
                Expression,
                Identifier,
                Equals,
                StringLit,
                CloseTag,
                Eof
            ]
        );
        assert_eq!(tokens[3].value, "item");
        assert_eq!(tokens[5].value, "items");
        assert_eq!(tokens[6].value, "as");
        assert_eq!(tokens[8].value, "item");
    }

    // 12. Unrecognized character → Unknown, no panic, lexing continues
    #[test]
    fn unknown_char() {
        let (tokens, errors) = tokenize("---\n@");
        assert!(!errors.is_empty());
        assert!(kinds(&tokens).contains(&Unknown));
        // Eof must be present even when there are errors
        assert_eq!(tokens.last().unwrap().kind, Eof);
    }

    // 13. File without --- separator → explicit LexError
    #[test]
    fn missing_separator() {
        let (_tokens, errors) = tokenize("<Column>");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("---"));
    }

    // 14. Correct positions across multiple lines
    #[test]
    fn position_tracking() {
        let input = "---\n<Column>\n<Text />";
        let (tokens, _) = tokenize(input);
        // SectionSeparator on line 1
        let sep = tokens.iter().find(|t| t.kind == SectionSeparator).unwrap();
        assert_eq!(sep.pos.line, 1);
        // <Column> on line 2
        let col = tokens.iter().find(|t| t.kind == OpenTag && t.value == "Column").unwrap();
        assert_eq!(col.pos.line, 2);
        // <Text /> on line 3
        let txt = tokens.iter().find(|t| t.kind == OpenTag && t.value == "Text").unwrap();
        assert_eq!(txt.pos.line, 3);
    }

    // 15. Eof is always the last token
    #[test]
    fn eof_is_last() {
        let inputs = ["---\n", "---\n<Column>", "---\n<Text />"];
        for input in &inputs {
            let (tokens, _) = tokenize(input);
            assert_eq!(tokens.last().unwrap().kind, Eof, "Eof missing for: {input}");
        }
    }
}
