use super::*;
use clutter_runtime::{codes, TokenKind::*};

fn tok(kind: TokenKind, value: &str) -> Token {
    Token { kind, value: value.to_string(), pos: Position { line: 1, col: 1 } }
}

fn comp_open_tok(name: &str, props_raw: &str) -> Token {
    Token {
        kind: TokenKind::ComponentOpen { name: name.to_string(), props_raw: props_raw.to_string() },
        value: format!("component {}({}) {{", name, props_raw),
        pos: Position { line: 1, col: 1 },
    }
}

/// Wraps template tokens in a single component block for parse_file() tests.
fn file_tokens(name: &str, logic: &str, template: Vec<Token>) -> Vec<Token> {
    let mut tokens = vec![
        comp_open_tok(name, "props: P"),
        tok(LogicBlock, logic),
        tok(SectionSeparator, "----"),
    ];
    tokens.extend(template);
    tokens.push(tok(ComponentClose, "}"));
    tokens.push(tok(Eof, ""));
    tokens
}

// 1. Single component, no props
#[test]
fn single_component_no_props() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Column"),
        tok(CloseTag, ">"),
        tok(CloseOpenTag, "Column"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty());
    assert_eq!(file.components[0].template.len(), 1);
    match &file.components[0].template[0] {
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
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Text"),
        tok(Identifier, "size"),
        tok(Equals, "="),
        tok(StringLit, "md"),
        tok(SelfCloseTag, "/>"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty());
    match &file.components[0].template[0] {
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
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Text"),
        tok(Identifier, "size"),
        tok(Equals, "="),
        tok(Expression, "size"),
        tok(SelfCloseTag, "/>"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty());
    match &file.components[0].template[0] {
        Node::Component(c) => {
            assert_eq!(c.props[0].value, PropValue::ExpressionValue("size".to_string()));
        }
        _ => panic!("expected ComponentNode"),
    }
}

// 4. Two-level nesting: <Column><Text /></Column>
#[test]
fn two_level_nesting() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Column"),
        tok(CloseTag, ">"),
        tok(OpenTag, "Text"),
        tok(SelfCloseTag, "/>"),
        tok(CloseOpenTag, "Column"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty());
    match &file.components[0].template[0] {
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
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "A"),
        tok(CloseTag, ">"),
        tok(OpenTag, "B"),
        tok(CloseTag, ">"),
        tok(OpenTag, "C"),
        tok(SelfCloseTag, "/>"),
        tok(CloseOpenTag, "B"),
        tok(CloseOpenTag, "A"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty());
    match &file.components[0].template[0] {
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
    let tokens = file_tokens("Main", "", vec![tok(OpenTag, "Text"), tok(SelfCloseTag, "/>")]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty());
    match &file.components[0].template[0] {
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
    let tokens = file_tokens("Main", "", vec![
        tok(IfOpen, "if"),
        tok(Identifier, "condition"),
        tok(Equals, "="),
        tok(Expression, "x"),
        tok(CloseTag, ">"),
        tok(OpenTag, "Text"),
        tok(SelfCloseTag, "/>"),
        tok(CloseOpenTag, "if"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty());
    match &file.components[0].template[0] {
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
    let tokens = file_tokens("Main", "", vec![
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
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty());
    match &file.components[0].template[0] {
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
    let tokens = file_tokens("Main", "", vec![
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
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty());
    match &file.components[0].template[0] {
        Node::Each(n) => {
            assert_eq!(n.collection, "items");
            assert_eq!(n.alias, "item");
            assert_eq!(n.children.len(), 1);
        }
        _ => panic!("expected EachNode"),
    }
}

// 10. Non-empty logic block → ComponentDef.logic_block contains the raw TypeScript string
#[test]
fn non_empty_logic_block() {
    let tokens = file_tokens("Main", "const x = 1;", vec![
        tok(OpenTag, "Text"),
        tok(SelfCloseTag, "/>"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty());
    assert_eq!(file.components[0].logic_block, "const x = 1;");
}

// 11. Unclosed tag → ParseError
#[test]
fn unclosed_tag_is_parse_error() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Column"),
        tok(CloseTag, ">"),
        // no CloseOpenTag
    ]);
    let (_file, errors) = Parser::new(tokens).parse_file();
    assert!(!errors.is_empty());
}

// 12. Prop without = or value → ParseError with P001 code
#[test]
fn prop_without_value_is_parse_error() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Text"),
        tok(Identifier, "size"),
        tok(CloseTag, ">"),
        tok(CloseOpenTag, "Text"),
    ]);
    let (_file, errors) = Parser::new(tokens).parse_file();
    assert!(!errors.is_empty());
    assert_eq!(errors[0].code, codes::P001);
}

// 13. <else> outside any <if> → ParseError with P002 code
#[test]
fn else_without_if_is_parse_error() {
    let tokens = file_tokens("Main", "", vec![
        tok(ElseOpen, "else"),
        tok(CloseTag, ">"),
        tok(OpenTag, "Text"),
        tok(SelfCloseTag, "/>"),
        tok(CloseOpenTag, "else"),
    ]);
    let (_file, errors) = Parser::new(tokens).parse_file();
    assert!(!errors.is_empty());
    assert_eq!(errors[0].message, "<else> without matching <if>");
    assert_eq!(errors[0].code, codes::P002);
}

// 14. Well-formed <unsafe reason="test"> → UnsafeNode with reason and one child
#[test]
fn unsafe_block_well_formed() {
    let tokens = file_tokens("Main", "", vec![
        tok(UnsafeOpen, "unsafe"),
        tok(Identifier, "reason"),
        tok(Equals, "="),
        tok(StringLit, "not in the design yet"),
        tok(CloseTag, ">"),
        tok(OpenTag, "Text"),
        tok(SelfCloseTag, "/>"),
        tok(CloseOpenTag, "unsafe"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    assert_eq!(file.components[0].template.len(), 1);
    match &file.components[0].template[0] {
        Node::Unsafe(n) => {
            assert_eq!(n.reason, "not in the design yet");
            assert_eq!(n.children.len(), 1);
        }
        _ => panic!("expected UnsafeNode"),
    }
}

// 15. <unsafe> without reason attr → parse error, node has reason = ""
#[test]
fn unsafe_block_missing_reason() {
    let tokens = file_tokens("Main", "", vec![
        tok(UnsafeOpen, "unsafe"),
        tok(CloseTag, ">"),
        tok(OpenTag, "Text"),
        tok(SelfCloseTag, "/>"),
        tok(CloseOpenTag, "unsafe"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(!errors.is_empty(), "expected a parse error for missing reason");
    assert_eq!(errors[0].code, codes::P003);
    // Node is still constructed despite the error (recovery)
    assert_eq!(file.components[0].template.len(), 1);
    match &file.components[0].template[0] {
        Node::Unsafe(n) => assert_eq!(n.reason, ""),
        _ => panic!("expected UnsafeNode"),
    }
}

// 16. Prop with well-formed unsafe() value → PropValue::UnsafeValue
#[test]
fn prop_unsafe_value_well_formed() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Column"),
        tok(Identifier, "gap"),
        tok(Equals, "="),
        tok(StringLit, "unsafe('16px', 'not in the design yet')"),
        tok(SelfCloseTag, "/>"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    match &file.components[0].template[0] {
        Node::Component(c) => {
            assert_eq!(
                c.props[0].value,
                PropValue::UnsafeValue {
                    value: "16px".to_string(),
                    reason: "not in the design yet".to_string()
                }
            );
        }
        _ => panic!("expected ComponentNode"),
    }
}

// 17. Prop with unsafe() missing reason → PropValue::UnsafeValue with reason = ""
#[test]
fn prop_unsafe_value_missing_reason() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Column"),
        tok(Identifier, "gap"),
        tok(Equals, "="),
        tok(StringLit, "unsafe('16px')"),
        tok(SelfCloseTag, "/>"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    match &file.components[0].template[0] {
        Node::Component(c) => match &c.props[0].value {
            PropValue::UnsafeValue { value, reason } => {
                assert_eq!(value, "16px");
                assert_eq!(reason, "");
            }
            other => panic!("expected UnsafeValue, got {:?}", other),
        },
        _ => panic!("expected ComponentNode"),
    }
}

// 18. Normal string prop still produces StringValue (no regression)
#[test]
fn prop_plain_string_unchanged() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Column"),
        tok(Identifier, "gap"),
        tok(Equals, "="),
        tok(StringLit, "md"),
        tok(SelfCloseTag, "/>"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty());
    match &file.components[0].template[0] {
        Node::Component(c) => {
            assert_eq!(c.props[0].value, PropValue::StringValue("md".to_string()));
        }
        _ => panic!("expected ComponentNode"),
    }
}

// -----------------------------------------------------------------------
// parse_file() structural tests
// -----------------------------------------------------------------------

// 19. Single-component file → FileNode with one ComponentDef, correct template
#[test]
fn parse_file_single_component() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Column"),
        tok(CloseTag, ">"),
        tok(CloseOpenTag, "Column"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    assert_eq!(file.components.len(), 1);
    assert_eq!(file.components[0].name, "Main");
    assert_eq!(file.components[0].template.len(), 1);
}

// 20. Two-component file → FileNode with two ComponentDefs
#[test]
fn parse_file_two_components() {
    let tokens = vec![
        comp_open_tok("A", "props: AP"),
        tok(LogicBlock, ""),
        tok(SectionSeparator, "----"),
        tok(OpenTag, "Column"),
        tok(SelfCloseTag, "/>"),
        tok(ComponentClose, "}"),
        comp_open_tok("B", "props: BP"),
        tok(LogicBlock, ""),
        tok(SectionSeparator, "----"),
        tok(OpenTag, "Row"),
        tok(SelfCloseTag, "/>"),
        tok(ComponentClose, "}"),
        tok(Eof, ""),
    ];
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    assert_eq!(file.components.len(), 2);
    assert_eq!(file.components[0].name, "A");
    assert_eq!(file.components[1].name, "B");
}

// 21. ComponentDef.name and props_raw captured correctly
#[test]
fn parse_file_component_def_metadata() {
    let tokens = file_tokens("Card", "", vec![tok(OpenTag, "Text"), tok(SelfCloseTag, "/>")]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty());
    assert_eq!(file.components[0].name, "Card");
    assert_eq!(file.components[0].props_raw, "props: P");
}

// 22. ComponentDef.logic_block captured correctly
#[test]
fn parse_file_logic_block() {
    let tokens = file_tokens("Main", "const x = 1;", vec![tok(OpenTag, "Text"), tok(SelfCloseTag, "/>")]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty());
    assert_eq!(file.components[0].logic_block, "const x = 1;");
}

// 23. Empty file (only Eof) → FileNode with no components, ParseError
#[test]
fn parse_file_no_component_block() {
    let tokens = vec![tok(Eof, "")];
    let (file, _errors) = Parser::new(tokens).parse_file();
    assert_eq!(file.components.len(), 0);
}

// 25. Raw text node in template body → Node::Text with correct value
#[test]
fn text_node_in_template() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Column"),
        tok(CloseTag, ">"),
        tok(Text, "Hello World"),
        tok(CloseOpenTag, "Column"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    match &file.components[0].template[0] {
        Node::Component(column) => {
            assert_eq!(column.children.len(), 1);
            match &column.children[0] {
                Node::Text(t) => assert_eq!(t.value, "Hello World"),
                other => panic!("expected Node::Text, got {:?}", other),
            }
        }
        other => panic!("expected ComponentNode, got {:?}", other),
    }
}

// 26. Standalone expression token in template body → Node::Expr with correct value
#[test]
fn expr_node_in_template() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Column"),
        tok(CloseTag, ">"),
        tok(Expression, "myVariable"),
        tok(CloseOpenTag, "Column"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    match &file.components[0].template[0] {
        Node::Component(column) => {
            assert_eq!(column.children.len(), 1);
            match &column.children[0] {
                Node::Expr(e) => assert_eq!(e.value, "myVariable"),
                other => panic!("expected Node::Expr, got {:?}", other),
            }
        }
        other => panic!("expected ComponentNode, got {:?}", other),
    }
}

// 28. Prop without `=` in the middle of a valid prop list
// When the first prop is malformed, error recovery skips to the next token
// boundary. This test documents the recovery behaviour: the error is emitted
// and parsing continues to the self-close marker without panicking.
// (The valid prop after the malformed one may be lost depending on whitespace
// token availability — the important invariant is: no panic, at least one error.)
#[test]
fn malformed_prop_followed_by_valid_prop_emits_error_and_recovers() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Text"),
        tok(Identifier, "size"),     // no Equals follows → malformed
        tok(Identifier, "color"),    // "color" is next — acts as recovery boundary
        tok(Equals, "="),
        tok(StringLit, "primary"),
        tok(SelfCloseTag, "/>"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    // Must not panic; must emit at least one error for the missing `=`
    assert!(!errors.is_empty(), "expected parse error for malformed prop");
    // Component node is still produced (no crash)
    assert_eq!(file.components[0].template.len(), 1);
}

// 29. Mismatched closing tag → ParseError (guards against silent corruption)
// Regression: parse_component previously checked only TokenKind, not tag name,
// so <Column>...</Row> would parse without error.
#[test]
fn mismatched_closing_tag_is_parse_error() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Column"),
        tok(CloseTag, ">"),
        tok(OpenTag, "Text"),
        tok(SelfCloseTag, "/>"),
        tok(CloseOpenTag, "Row"), // mismatch: opened Column, closing Row
    ]);
    let (_file, errors) = Parser::new(tokens).parse_file();
    assert!(!errors.is_empty(), "expected a parse error for mismatched closing tag");
    assert!(
        errors.iter().any(|e| e.message.contains("Column") && e.message.contains("Row")),
        "error should name both the expected and actual tags, got: {:?}", errors
    );
}

// 27. Existing template AST structure unchanged inside a component (regression)
#[test]
fn parse_file_template_nodes_unchanged() {
    let tokens = file_tokens("Main", "", vec![
        tok(IfOpen, "if"),
        tok(Identifier, "condition"),
        tok(Equals, "="),
        tok(Expression, "flag"),
        tok(CloseTag, ">"),
        tok(OpenTag, "Text"),
        tok(SelfCloseTag, "/>"),
        tok(CloseOpenTag, "if"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    match &file.components[0].template[0] {
        Node::If(n) => {
            assert_eq!(n.condition, "flag");
            assert_eq!(n.then_children.len(), 1);
        }
        other => panic!("expected IfNode, got {:?}", other),
    }
}

// -----------------------------------------------------------------------
// Event binding tests
// -----------------------------------------------------------------------

// 25. Single @event={handler} produces EventBinding in ComponentNode.events
#[test]
fn event_binding_single() {
    let tokens = file_tokens("Main", "const addRule = () => {}", vec![
        tok(OpenTag, "Button"),
        tok(EventName, "click"),
        tok(Equals, "="),
        tok(Expression, "addRule"),
        tok(SelfCloseTag, "/>"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    match &file.components[0].template[0] {
        Node::Component(c) => {
            assert!(c.props.is_empty(), "expected no props, got {:?}", c.props);
            assert_eq!(c.events.len(), 1);
            assert_eq!(c.events[0].name, "click");
            assert_eq!(c.events[0].handler, "addRule");
        }
        _ => panic!("expected ComponentNode"),
    }
}

// 26. @event mixed with regular props: both are collected separately
#[test]
fn event_binding_mixed_with_props() {
    let tokens = file_tokens("Main", "const fn = () => {}", vec![
        tok(OpenTag, "Button"),
        tok(Identifier, "variant"),
        tok(Equals, "="),
        tok(StringLit, "primary"),
        tok(EventName, "click"),
        tok(Equals, "="),
        tok(Expression, "fn"),
        tok(SelfCloseTag, "/>"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    match &file.components[0].template[0] {
        Node::Component(c) => {
            assert_eq!(c.props.len(), 1);
            assert_eq!(c.props[0].name, "variant");
            assert_eq!(c.events.len(), 1);
            assert_eq!(c.events[0].name, "click");
            assert_eq!(c.events[0].handler, "fn");
        }
        _ => panic!("expected ComponentNode"),
    }
}

// 27. Multiple event bindings on same tag
#[test]
fn event_binding_multiple() {
    let tokens = file_tokens("Main", "const onChange = () => {}\nconst onBlur = () => {}", vec![
        tok(OpenTag, "Input"),
        tok(EventName, "input"),
        tok(Equals, "="),
        tok(Expression, "onChange"),
        tok(EventName, "blur"),
        tok(Equals, "="),
        tok(Expression, "onBlur"),
        tok(SelfCloseTag, "/>"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    match &file.components[0].template[0] {
        Node::Component(c) => {
            assert_eq!(c.events.len(), 2);
            assert_eq!(c.events[0].name, "input");
            assert_eq!(c.events[1].name, "blur");
        }
        _ => panic!("expected ComponentNode"),
    }
}

// -----------------------------------------------------------------------
// indexAs in <each>
// -----------------------------------------------------------------------

// 29. <each ... indexAs="i"> sets index_alias to Some("i")
#[test]
fn each_with_index_alias() {
    let tokens = file_tokens("Main", "const items = []", vec![
        tok(EachOpen, "each"),
        tok(Identifier, "collection"),
        tok(Equals, "="),
        tok(Expression, "items"),
        tok(Identifier, "as"),
        tok(Equals, "="),
        tok(StringLit, "item"),
        tok(Identifier, "indexAs"),
        tok(Equals, "="),
        tok(StringLit, "i"),
        tok(CloseTag, ">"),
        tok(CloseOpenTag, "each"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    match &file.components[0].template[0] {
        Node::Each(n) => {
            assert_eq!(n.alias, "item");
            assert_eq!(n.index_alias, Some("i".to_string()));
        }
        _ => panic!("expected EachNode"),
    }
}

// 31. @click with no `=` following — missing Equals → P001, events vec stays empty
#[test]
fn event_binding_missing_equals_is_parse_error() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Button"),
        tok(EventName, "click"),  // no Equals follows
        tok(SelfCloseTag, "/>"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(!errors.is_empty(), "expected a parse error");
    assert_eq!(errors[0].code, codes::P001, "expected P001 for missing Equals, got: {:?}", errors[0]);
    // Component node is recovered, event is NOT added on error
    assert_eq!(file.components[0].template.len(), 1);
    match &file.components[0].template[0] {
        Node::Component(c) => {
            assert_eq!(c.name, "Button");
            assert!(c.events.is_empty(), "malformed event must not be added to events, got: {:?}", c.events);
        }
        _ => panic!("expected ComponentNode after error recovery"),
    }
}

// 32. @click={} — Equals present but value is not an Expression (self-close follows) → P001
#[test]
fn event_binding_missing_expression_is_parse_error() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Button"),
        tok(EventName, "click"),
        tok(Equals, "="),
        tok(SelfCloseTag, "/>"),   // no Expression token after =
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(!errors.is_empty(), "expected a parse error for missing expression");
    assert_eq!(errors[0].code, codes::P001, "expected P001, got: {:?}", errors[0]);
    match &file.components[0].template[0] {
        Node::Component(c) => {
            assert!(c.events.is_empty(), "malformed event must not be added to events, got: {:?}", c.events);
        }
        _ => panic!("expected ComponentNode after error recovery"),
    }
}

// 30. <each> without indexAs leaves index_alias as None (no regression)
#[test]
fn each_without_index_alias_is_none() {
    let tokens = file_tokens("Main", "const items = []", vec![
        tok(EachOpen, "each"),
        tok(Identifier, "collection"),
        tok(Equals, "="),
        tok(Expression, "items"),
        tok(Identifier, "as"),
        tok(Equals, "="),
        tok(StringLit, "item"),
        tok(CloseTag, ">"),
        tok(CloseOpenTag, "each"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    match &file.components[0].template[0] {
        Node::Each(n) => assert_eq!(n.index_alias, None),
        _ => panic!("expected EachNode"),
    }
}

// -----------------------------------------------------------------------
// @event on control-flow nodes (<if>, <each>)
// -----------------------------------------------------------------------

// Event bindings are only valid on component tags. @event on <if>/<each> is a
// parse error. The parser must emit P001 and recover without hanging, so that
// any well-formed children are still reachable.

// @event after the condition of <if> → P001, IfNode still constructed
#[test]
fn event_binding_on_if_is_parse_error_and_recovers() {
    // Simulates: <if condition={flag} @click={fn}>...</if>
    let tokens = file_tokens("Main", "", vec![
        tok(IfOpen, "if"),
        tok(Identifier, "condition"),
        tok(Equals, "="),
        tok(Expression, "flag"),
        tok(EventName, "click"),   // spurious @click after condition
        tok(Equals, "="),
        tok(Expression, "fn"),
        tok(CloseTag, ">"),
        tok(OpenTag, "Text"),
        tok(SelfCloseTag, "/>"),
        tok(CloseOpenTag, "if"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    // Error must be emitted — and parsing must not hang
    assert!(!errors.is_empty(), "expected parse error for @event on <if>");
    assert!(errors.iter().any(|e| e.code == codes::P001), "expected P001, got: {:?}", errors);
    // The <if> node is still produced
    assert_eq!(file.components[0].template.len(), 1);
    match &file.components[0].template[0] {
        Node::If(n) => {
            assert_eq!(n.condition, "flag");
            assert_eq!(n.then_children.len(), 1, "then-branch should still be parsed");
        }
        _ => panic!("expected IfNode"),
    }
}

// @event after the alias of <each> → P001, EachNode still constructed
#[test]
fn event_binding_on_each_is_parse_error_and_recovers() {
    // Simulates: <each collection={items} as="item" @click={fn}>...</each>
    let tokens = file_tokens("Main", "const items = []", vec![
        tok(EachOpen, "each"),
        tok(Identifier, "collection"),
        tok(Equals, "="),
        tok(Expression, "items"),
        tok(Identifier, "as"),
        tok(Equals, "="),
        tok(StringLit, "item"),
        tok(EventName, "click"),   // spurious @click after alias
        tok(Equals, "="),
        tok(Expression, "fn"),
        tok(CloseTag, ">"),
        tok(OpenTag, "Text"),
        tok(SelfCloseTag, "/>"),
        tok(CloseOpenTag, "each"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(!errors.is_empty(), "expected parse error for @event on <each>");
    assert!(errors.iter().any(|e| e.code == codes::P001), "expected P001, got: {:?}", errors);
    // The <each> node is still produced
    assert_eq!(file.components[0].template.len(), 1);
    match &file.components[0].template[0] {
        Node::Each(n) => {
            assert_eq!(n.collection, "items");
            assert_eq!(n.alias, "item");
            assert_eq!(n.children.len(), 1, "loop body should still be parsed");
        }
        _ => panic!("expected EachNode"),
    }
}

// 28. Component without event bindings has empty events vec (no regression)
#[test]
fn component_without_events_has_empty_events() {
    let tokens = file_tokens("Main", "", vec![
        tok(OpenTag, "Column"),
        tok(SelfCloseTag, "/>"),
    ]);
    let (file, errors) = Parser::new(tokens).parse_file();
    assert!(errors.is_empty());
    match &file.components[0].template[0] {
        Node::Component(c) => assert!(c.events.is_empty()),
        _ => panic!("expected ComponentNode"),
    }
}
