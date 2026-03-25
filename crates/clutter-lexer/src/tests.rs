use super::*;
use clutter_runtime::TokenKind::*;

fn kinds(tokens: &[Token]) -> Vec<TokenKind> {
    tokens.iter().map(|t| t.kind.clone()).collect()
}

/// Returns only the template tokens: those between the last SectionSeparator
/// and the last ComponentClose (exclusive on both ends).
fn template_kinds(tokens: &[Token]) -> Vec<TokenKind> {
    let start = tokens
        .iter()
        .position(|t| t.kind == SectionSeparator)
        .map(|i| i + 1)
        .unwrap_or(0);
    let end = tokens
        .iter()
        .rposition(|t| t.kind == ComponentClose)
        .unwrap_or(tokens.len());
    tokens[start..end].iter().map(|t| t.kind.clone()).collect()
}

/// Wraps a template snippet in a minimal component block for testing.
fn wrap(template: &str) -> String {
    format!("component Main(props: P) {{\n----\n{template}\n}}\n")
}

/// Wraps a template snippet after a logic block.
fn wrap_with_logic(logic: &str, template: &str) -> String {
    format!("component Main(props: P) {{\n{logic}\n----\n{template}\n}}\n")
}

// 1. Minimal component block: empty logic, empty template
#[test]
fn minimal_file() {
    let input = "component Main(props: P) {\n----\n}\n";
    let (tokens, errors) = tokenize(input);
    assert!(errors.is_empty());
    // ComponentOpen, LogicBlock(""), SectionSeparator, ComponentClose, Eof
    assert_eq!(tokens.len(), 5);
    assert_eq!(tokens[1].kind, LogicBlock);
    assert_eq!(tokens[1].value, "");
    assert_eq!(tokens[2].kind, SectionSeparator);
    assert_eq!(tokens[3].kind, ComponentClose);
    assert_eq!(tokens[4].kind, Eof);
}

// 2. Template component without props: <Column>
#[test]
fn component_no_props() {
    let (tokens, errors) = tokenize(&wrap("<Column>"));
    assert!(errors.is_empty());
    assert_eq!(template_kinds(&tokens), vec![OpenTag, CloseTag]);
    let col = tokens.iter().find(|t| t.kind == OpenTag).unwrap();
    assert_eq!(col.value, "Column");
}

// 3. Template component with string prop, position check
#[test]
fn component_string_prop() {
    let (tokens, errors) = tokenize(&wrap("<Column gap=\"md\">"));
    assert!(errors.is_empty());
    assert_eq!(
        template_kinds(&tokens),
        vec![OpenTag, Identifier, Equals, StringLit, CloseTag]
    );
    let open = tokens.iter().find(|t| t.kind == OpenTag).unwrap();
    let id = tokens.iter().find(|t| t.kind == Identifier).unwrap();
    let lit = tokens.iter().find(|t| t.kind == StringLit).unwrap();
    assert_eq!(open.value, "Column");
    assert_eq!(id.value, "gap");
    assert_eq!(lit.value, "md");
    // line 1: component header, line 2: ----, line 3: template
    assert_eq!(open.pos.line, 3);
}

// 4. Template component with expression prop
#[test]
fn component_expression_prop() {
    let (tokens, errors) = tokenize(&wrap("<Column gap={size}>"));
    assert!(errors.is_empty());
    assert_eq!(
        template_kinds(&tokens),
        vec![OpenTag, Identifier, Equals, Expression, CloseTag]
    );
    let expr = tokens.iter().find(|t| t.kind == Expression).unwrap();
    assert_eq!(expr.value, "size");
}

// 5. Self-closing tag
#[test]
fn self_closing_tag() {
    let (tokens, errors) = tokenize(&wrap("<Text />"));
    assert!(errors.is_empty());
    assert_eq!(template_kinds(&tokens), vec![OpenTag, SelfCloseTag]);
    let open = tokens.iter().find(|t| t.kind == OpenTag).unwrap();
    assert_eq!(open.value, "Text");
}

// 6. Closing tag
#[test]
fn closing_tag() {
    let (tokens, errors) = tokenize(&wrap("</Column>"));
    assert!(errors.is_empty());
    assert_eq!(template_kinds(&tokens), vec![CloseOpenTag]);
    let close = tokens.iter().find(|t| t.kind == CloseOpenTag).unwrap();
    assert_eq!(close.value, "Column");
}

// 7. Nesting
#[test]
fn nesting() {
    let (tokens, errors) = tokenize(&wrap("<Column><Text /></Column>"));
    assert!(errors.is_empty());
    assert_eq!(
        template_kinds(&tokens),
        vec![OpenTag, CloseTag, OpenTag, SelfCloseTag, CloseOpenTag]
    );
    let opens: Vec<_> = tokens.iter().filter(|t| t.kind == OpenTag).collect();
    let close = tokens.iter().find(|t| t.kind == CloseOpenTag).unwrap();
    assert_eq!(opens[0].value, "Column");
    assert_eq!(opens[1].value, "Text");
    assert_eq!(close.value, "Column");
}

// 8. Logic section with real TypeScript
#[test]
fn logic_section() {
    let input = wrap_with_logic("const x = 1\nconst y = 2", "<Text />");
    let (tokens, errors) = tokenize(&input);
    assert!(errors.is_empty());
    let logic = tokens.iter().find(|t| t.kind == LogicBlock).unwrap();
    assert_eq!(logic.value, "const x = 1\nconst y = 2");
    let sep = tokens.iter().find(|t| t.kind == SectionSeparator).unwrap();
    assert_eq!(sep.value, "----");
}

// 9. Control flow: <if condition={x}>
#[test]
fn control_flow_if() {
    let (tokens, errors) = tokenize(&wrap("<if condition={x}>"));
    assert!(errors.is_empty());
    assert_eq!(
        template_kinds(&tokens),
        vec![IfOpen, Identifier, Equals, Expression, CloseTag]
    );
    let id = tokens.iter().find(|t| t.kind == Identifier).unwrap();
    let expr = tokens.iter().find(|t| t.kind == Expression).unwrap();
    assert_eq!(id.value, "condition");
    assert_eq!(expr.value, "x");
}

// 10. Control flow: <else>
#[test]
fn control_flow_else() {
    let (tokens, errors) = tokenize(&wrap("<else>"));
    assert!(errors.is_empty());
    assert_eq!(template_kinds(&tokens), vec![ElseOpen, CloseTag]);
}

// 11. Control flow: <each item={items} as="item">
#[test]
fn control_flow_each() {
    let (tokens, errors) = tokenize(&wrap("<each item={items} as=\"item\">"));
    assert!(errors.is_empty());
    assert_eq!(
        template_kinds(&tokens),
        vec![EachOpen, Identifier, Equals, Expression, Identifier, Equals, StringLit, CloseTag]
    );
    let ids: Vec<_> = tokens.iter().filter(|t| t.kind == Identifier).collect();
    let exprs: Vec<_> = tokens.iter().filter(|t| t.kind == Expression).collect();
    let lits: Vec<_> = tokens.iter().filter(|t| t.kind == StringLit).collect();
    assert_eq!(ids[0].value, "item");
    assert_eq!(exprs[0].value, "items");
    assert_eq!(ids[1].value, "as");
    assert_eq!(lits[0].value, "item");
}

// 12. Unrecognised character → Unknown, no panic, lexing continues
#[test]
fn unknown_char() {
    let (tokens, errors) = tokenize(&wrap("@"));
    assert!(!errors.is_empty());
    assert!(kinds(&tokens).contains(&Unknown));
    assert_eq!(tokens.last().unwrap().kind, Eof);
    assert_eq!(errors[0].code, codes::L002);
    assert_eq!(errors[0].message, "unexpected character '@' in template");
}

// 13. File without component block → L001 LexError, Eof always present
#[test]
fn missing_separator() {
    let (tokens, errors) = tokenize("<Column>");
    assert!(!errors.is_empty());
    assert_eq!(errors[0].code, codes::L001);
    assert_eq!(tokens.last().unwrap().kind, Eof);
}

// 14. Correct positions across multiple lines
#[test]
fn position_tracking() {
    // line 1: component header
    // line 2: ----
    // line 3: <Column>
    // line 4: <Text />
    // line 5: }
    let input = "component Main(props: P) {\n----\n<Column>\n<Text />\n}\n";
    let (tokens, _) = tokenize(input);
    let sep = tokens.iter().find(|t| t.kind == SectionSeparator).unwrap();
    assert_eq!(sep.pos.line, 2);
    let col = tokens.iter().find(|t| t.kind == OpenTag && t.value == "Column").unwrap();
    assert_eq!(col.pos.line, 3);
    let txt = tokens.iter().find(|t| t.kind == OpenTag && t.value == "Text").unwrap();
    assert_eq!(txt.pos.line, 4);
}

// 15. Eof is always the last token
#[test]
fn eof_is_last() {
    let inputs = [
        "component Main(props: P) {\n----\n}\n",
        "component Main(props: P) {\n----\n<Column>\n}\n",
        "component Main(props: P) {\n----\n<Text />\n}\n",
    ];
    for input in &inputs {
        let (tokens, _) = tokenize(input);
        assert_eq!(tokens.last().unwrap().kind, Eof, "Eof missing for: {input}");
    }
}

// 16. <unsafe reason="x"> emits UnsafeOpen
#[test]
fn unsafe_open_tag() {
    let (tokens, errors) = tokenize(&wrap("<unsafe reason=\"x\">"));
    assert!(errors.is_empty());
    assert_eq!(
        template_kinds(&tokens),
        vec![UnsafeOpen, Identifier, Equals, StringLit, CloseTag]
    );
    let open = tokens.iter().find(|t| t.kind == UnsafeOpen).unwrap();
    let id = tokens.iter().find(|t| t.kind == Identifier).unwrap();
    let lit = tokens.iter().find(|t| t.kind == StringLit).unwrap();
    assert_eq!(open.value, "unsafe");
    assert_eq!(id.value, "reason");
    assert_eq!(lit.value, "x");
}

// 17. </unsafe> emits CloseOpenTag with value "unsafe"
#[test]
fn unsafe_close_tag() {
    let (tokens, errors) = tokenize(&wrap("</unsafe>"));
    assert!(errors.is_empty());
    assert_eq!(template_kinds(&tokens), vec![CloseOpenTag]);
    let close = tokens.iter().find(|t| t.kind == CloseOpenTag).unwrap();
    assert_eq!(close.value, "unsafe");
}

// -----------------------------------------------------------------------
// New-format specific tests
// -----------------------------------------------------------------------

// 18. Minimal component block — empty logic and empty template
#[test]
fn component_block_minimal() {
    let input = "component Main(props: P) {\n----\n}\n";
    let (tokens, errors) = tokenize(input);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    assert_eq!(tokens.len(), 5, "got: {tokens:?}");
    assert!(
        matches!(&tokens[0].kind, TokenKind::ComponentOpen { name, .. } if name == "Main"),
        "expected ComponentOpen(Main), got {:?}", tokens[0].kind
    );
    assert_eq!(tokens[1].kind, LogicBlock);
    assert_eq!(tokens[1].value, "");
    assert_eq!(tokens[2].kind, SectionSeparator);
    assert_eq!(tokens[2].value, "----");
    assert_eq!(tokens[3].kind, ComponentClose);
    assert_eq!(tokens[4].kind, Eof);
}

// 19. props_raw is captured verbatim between ( and )
#[test]
fn component_open_props_raw() {
    let input = "component Card(title: string, size: SpacingToken) {\n----\n}\n";
    let (tokens, errors) = tokenize(input);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    assert!(
        matches!(&tokens[0].kind,
            TokenKind::ComponentOpen { name, props_raw }
            if name == "Card" && props_raw == "title: string, size: SpacingToken"
        ),
        "got {:?}", tokens[0].kind
    );
}

// 20. Logic block captured between ComponentOpen and SectionSeparator
#[test]
fn component_with_logic() {
    let input = "component Main(props: P) {\nconst x = 1\nconst y = 2\n----\n}\n";
    let (tokens, errors) = tokenize(input);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    assert_eq!(tokens[1].kind, LogicBlock);
    assert_eq!(tokens[1].value, "const x = 1\nconst y = 2");
}

// 21. Template tokens present inside component block
#[test]
fn component_with_template() {
    let input = "component Main(props: P) {\n----\n<Column></Column>\n}\n";
    let (tokens, errors) = tokenize(input);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    assert!(matches!(&tokens[0].kind, TokenKind::ComponentOpen { .. }));
    assert_eq!(tokens[2].kind, SectionSeparator);
    let close_idx = tokens.iter().rposition(|t| t.kind == ComponentClose).unwrap();
    assert_eq!(tokens.last().unwrap().kind, Eof);
    let col_idx = tokens.iter().position(|t| t.kind == OpenTag && t.value == "Column").unwrap();
    assert!(col_idx > 2 && col_idx < close_idx);
}

// 22. Two component blocks produce two ComponentOpen/ComponentClose pairs
#[test]
fn two_component_blocks() {
    let input = concat!(
        "component A(props: AP) {\n----\n}\n",
        "component B(props: BP) {\n----\n}\n"
    );
    let (tokens, errors) = tokenize(input);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    let opens: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(&t.kind, TokenKind::ComponentOpen { .. }))
        .collect();
    let closes: Vec<_> = tokens.iter().filter(|t| t.kind == ComponentClose).collect();
    assert_eq!(opens.len(), 2, "expected 2 ComponentOpen tokens");
    assert_eq!(closes.len(), 2, "expected 2 ComponentClose tokens");
    assert!(matches!(&opens[0].kind, TokenKind::ComponentOpen { name, .. } if name == "A"));
    assert!(matches!(&opens[1].kind, TokenKind::ComponentOpen { name, .. } if name == "B"));
}

// 23. ComponentOpen position is the line of the `component` keyword
#[test]
fn component_open_position() {
    let input = "\ncomponent Main(props: P) {\n----\n}\n";
    let (tokens, errors) = tokenize(input);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    assert_eq!(tokens[0].pos.line, 2, "ComponentOpen should be on line 2");
}

// 24. File without any component block → LexError, Eof always present
#[test]
fn missing_component_block() {
    let (tokens, errors) = tokenize("<Column>");
    assert!(!errors.is_empty(), "expected a lex error");
    assert_eq!(tokens.last().unwrap().kind, Eof, "Eof must be present even on error");
}

// 25. @event={handler} inside a tag emits EventName, Equals, Expression
#[test]
fn event_binding_single() {
    let (tokens, errors) = tokenize(&wrap("<Button @click={addRule} />"));
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    assert_eq!(
        template_kinds(&tokens),
        vec![OpenTag, EventName, Equals, Expression, SelfCloseTag]
    );
    let ev = tokens.iter().find(|t| t.kind == EventName).unwrap();
    let expr = tokens.iter().find(|t| t.kind == Expression).unwrap();
    assert_eq!(ev.value, "click");
    assert_eq!(expr.value, "addRule");
}

// 26. @event mixed with regular props: props before and after the event binding
#[test]
fn event_binding_mixed_with_props() {
    let (tokens, errors) =
        tokenize(&wrap("<Button variant=\"primary\" @click={fn} size=\"md\" />"));
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    assert_eq!(
        template_kinds(&tokens),
        vec![
            OpenTag,
            Identifier, Equals, StringLit, // variant="primary"
            EventName, Equals, Expression,  // @click={fn}
            Identifier, Equals, StringLit,  // size="md"
            SelfCloseTag,
        ]
    );
    let ev = tokens.iter().find(|t| t.kind == EventName).unwrap();
    assert_eq!(ev.value, "click");
}

// 27. Multiple event bindings on the same tag
#[test]
fn event_binding_multiple() {
    let (tokens, errors) = tokenize(&wrap("<Input @input={onChange} @blur={onBlur} />"));
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    let event_names: Vec<_> = tokens
        .iter()
        .filter(|t| t.kind == EventName)
        .map(|t| t.value.as_str())
        .collect();
    assert_eq!(event_names, vec!["input", "blur"]);
}
