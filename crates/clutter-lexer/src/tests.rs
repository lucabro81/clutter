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

// 12. Unrecognised character → Unknown, no panic, lexing continues
#[test]
fn unknown_char() {
    let (tokens, errors) = tokenize("---\n@");
    assert!(!errors.is_empty());
    assert!(kinds(&tokens).contains(&Unknown));
    // Eof must be present even when there are errors
    assert_eq!(tokens.last().unwrap().kind, Eof);
    // Error must carry the L002 code and a precise message
    assert_eq!(errors[0].code, codes::L002);
    assert_eq!(errors[0].message, "unexpected character '@' in template");
}

// 13. File without --- separator → explicit LexError
#[test]
fn missing_separator() {
    let (_tokens, errors) = tokenize("<Column>");
    assert!(!errors.is_empty());
    assert!(errors[0].message.contains("---"));
    // Error must carry the L001 code
    assert_eq!(errors[0].code, codes::L001);
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

// 16. <unsafe reason="x"> emits UnsafeOpen
#[test]
fn unsafe_open_tag() {
    let (tokens, errors) = tokenize("---\n<unsafe reason=\"x\">");
    assert!(errors.is_empty());
    assert_eq!(
        kinds(&tokens),
        vec![
            LogicBlock,
            SectionSeparator,
            UnsafeOpen,
            Identifier,
            Equals,
            StringLit,
            CloseTag,
            Eof
        ]
    );
    assert_eq!(tokens[2].value, "unsafe");
    assert_eq!(tokens[3].value, "reason");
    assert_eq!(tokens[5].value, "x");
}

// 17. </unsafe> emits CloseOpenTag with value "unsafe"
#[test]
fn unsafe_close_tag() {
    let (tokens, errors) = tokenize("---\n</unsafe>");
    assert!(errors.is_empty());
    assert_eq!(
        kinds(&tokens),
        vec![LogicBlock, SectionSeparator, CloseOpenTag, Eof]
    );
    assert_eq!(tokens[2].value, "unsafe");
}
