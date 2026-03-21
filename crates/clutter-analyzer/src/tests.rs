use super::*;
use clutter_runtime::{
    codes, ComponentNode, EachNode, ExpressionNode, IfNode, Node, Position, ProgramNode,
    PropNode, PropValue, UnsafeNode,
};

fn test_tokens() -> DesignTokens {
    DesignTokens::from_str(r#"{
        "spacing":    ["xs", "sm", "md", "lg", "xl", "xxl"],
        "colors":     ["primary", "secondary", "danger", "surface", "background"],
        "typography": {
            "sizes":   ["xs", "sm", "base", "lg", "xl", "xxl"],
            "weights": ["normal", "medium", "semibold", "bold"]
        },
        "radii":   ["none", "sm", "md", "lg", "full"],
        "shadows": ["sm", "md", "lg"]
    }"#).unwrap()
}

fn pos() -> Position {
    Position { line: 1, col: 1 }
}

fn program(logic_block: &str, template: Vec<Node>) -> ProgramNode {
    ProgramNode { logic_block: logic_block.to_string(), template }
}

fn component(name: &str, props: Vec<PropNode>, children: Vec<Node>) -> Node {
    Node::Component(ComponentNode { name: name.to_string(), props, children, pos: pos() })
}

fn prop_str(name: &str, value: &str) -> PropNode {
    PropNode { name: name.to_string(), value: PropValue::StringValue(value.to_string()), pos: pos() }
}

fn prop_expr(name: &str, expr: &str) -> PropNode {
    PropNode { name: name.to_string(), value: PropValue::ExpressionValue(expr.to_string()), pos: pos() }
}

fn prop_unsafe_val(prop_name: &str, value: &str, reason: &str) -> PropNode {
    PropNode {
        name: prop_name.to_string(),
        value: PropValue::UnsafeValue { value: value.to_string(), reason: reason.to_string() },
        pos: pos(),
    }
}

fn expr_node(value: &str) -> Node {
    Node::Expr(ExpressionNode { value: value.to_string(), pos: pos() })
}

fn if_node(condition: &str, then_children: Vec<Node>) -> Node {
    Node::If(IfNode { condition: condition.to_string(), then_children, else_children: None, pos: pos() })
}

fn each_node(collection: &str, alias: &str, children: Vec<Node>) -> Node {
    Node::Each(EachNode {
        collection: collection.to_string(),
        alias: alias.to_string(),
        children,
        pos: pos(),
    })
}

fn unsafe_node(reason: &str, children: Vec<Node>) -> Node {
    Node::Unsafe(UnsafeNode { reason: reason.to_string(), children, pos: pos() })
}

// --- DesignTokens ---

#[test]
fn design_tokens_parses_valid_json() {
    let t = test_tokens();
    assert!(t.valid_values(TokenCategory::Spacing).contains(&"md".to_string()));
    assert!(t.valid_values(TokenCategory::Color).contains(&"primary".to_string()));
    assert!(t.valid_values(TokenCategory::FontSize).contains(&"lg".to_string()));
    assert!(t.valid_values(TokenCategory::FontWeight).contains(&"bold".to_string()));
    assert!(t.valid_values(TokenCategory::Radius).contains(&"full".to_string()));
    assert!(t.valid_values(TokenCategory::Shadow).contains(&"sm".to_string()));
}

#[test]
fn design_tokens_rejects_invalid_json() {
    assert!(DesignTokens::from_str("not json").is_err());
}

// --- prop_map ---

#[test]
fn prop_map_known_token_prop() {
    assert!(matches!(prop_map("Column", "gap"), Some(PropValidation::Tokens(TokenCategory::Spacing))));
    assert!(matches!(prop_map("Text", "size"), Some(PropValidation::Tokens(TokenCategory::FontSize))));
    assert!(matches!(prop_map("Box", "bg"), Some(PropValidation::Tokens(TokenCategory::Color))));
}

#[test]
fn prop_map_known_enum_prop() {
    assert!(matches!(prop_map("Column", "mainAxis"), Some(PropValidation::Enum(_))));
    assert!(matches!(prop_map("Text", "align"), Some(PropValidation::Enum(_))));
    assert!(matches!(prop_map("Button", "variant"), Some(PropValidation::Enum(_))));
}

#[test]
fn prop_map_any_value_prop() {
    assert!(matches!(prop_map("Text", "value"), Some(PropValidation::AnyValue)));
    assert!(matches!(prop_map("Button", "disabled"), Some(PropValidation::AnyValue)));
    assert!(matches!(prop_map("Input", "placeholder"), Some(PropValidation::AnyValue)));
}

#[test]
fn prop_map_unknown_component_returns_none() {
    assert!(prop_map("Grid", "gap").is_none());
}

#[test]
fn prop_map_unknown_prop_on_known_component_returns_none() {
    assert!(prop_map("Column", "color").is_none());
    assert!(prop_map("Text", "border").is_none());
}

// --- analyze() ---

// 1. Valid prop value → no errors
#[test]
fn analyze_valid_prop_no_errors() {
    let t = test_tokens();
    let p = program("", vec![component("Column", vec![prop_str("gap", "md")], vec![])]);
    let (errors, _) = analyze(&p, &t);
    assert!(errors.is_empty());
}

// 2. Invalid prop value → CLT102 with message listing valid values
#[test]
fn analyze_invalid_token_value_error() {
    let t = test_tokens();
    let p = program("", vec![component("Column", vec![prop_str("gap", "xl2")], vec![])]);
    let (errors, _) = analyze(&p, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("xl2"), "message should mention the bad value");
    assert!(errors[0].message.contains("gap"), "message should mention the prop name");
}

// 3. ExpressionValue prop with known identifier → no errors
#[test]
fn analyze_expression_prop_known_ident_no_errors() {
    let t = test_tokens();
    let p = program("const myVar = 4;", vec![
        component("Column", vec![prop_expr("gap", "myVar")], vec![]),
    ]);
    let (errors, _) = analyze(&p, &t);
    assert!(errors.is_empty());
}

// 4. ExpressionValue prop with unknown identifier → CLT104
#[test]
fn analyze_expression_prop_unknown_ident_error() {
    let t = test_tokens();
    let p = program("", vec![component("Column", vec![prop_expr("gap", "unknown")], vec![])]);
    let (errors, _) = analyze(&p, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("unknown"), "message should mention the identifier");
}

// 5. Unknown component → CLT103
#[test]
fn analyze_unknown_component_error() {
    let t = test_tokens();
    let p = program("", vec![component("Grid", vec![], vec![])]);
    let (errors, _) = analyze(&p, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Grid"));
}

// 6. Unknown prop on known component → CLT101
#[test]
fn analyze_unknown_prop_on_known_component_error() {
    let t = test_tokens();
    let p = program("", vec![component("Column", vec![prop_str("color", "primary")], vec![])]);
    let (errors, _) = analyze(&p, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("color"));
    assert!(errors[0].message.contains("Column"));
}

// 7. Multiple errors collected
#[test]
fn analyze_multiple_errors_collected() {
    let t = test_tokens();
    let p = program("", vec![
        component("Column", vec![prop_str("gap", "bad1")], vec![]),
        component("Column", vec![prop_str("gap", "bad2")], vec![]),
    ]);
    let (errors, _) = analyze(&p, &t);
    assert_eq!(errors.len(), 2);
}

// 8. Nested component — props validated the same way
#[test]
fn analyze_nested_component_props_validated() {
    let t = test_tokens();
    let inner = component("Text", vec![prop_str("size", "huge")], vec![]);
    let p = program("", vec![component("Column", vec![], vec![inner])]);
    let (errors, _) = analyze(&p, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("huge"));
}

// 9. Children of <if>/<each> validated recursively
#[test]
fn analyze_if_each_children_validated() {
    let t = test_tokens();
    let bad_child = component("Text", vec![prop_str("size", "nope")], vec![]);
    let p = program("const flag = true;", vec![if_node("flag", vec![bad_child])]);
    let (errors, _) = analyze(&p, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("nope"));
}

// 10. Empty template → no errors
#[test]
fn analyze_empty_template_no_errors() {
    let t = test_tokens();
    let p = program("", vec![]);
    let (errors, _) = analyze(&p, &t);
    assert!(errors.is_empty());
}

// 11. ExpressionNode with known identifier → no errors
#[test]
fn analyze_expression_node_known_ident_no_errors() {
    let t = test_tokens();
    let p = program("const title = \"Hello\";", vec![expr_node("title")]);
    let (errors, _) = analyze(&p, &t);
    assert!(errors.is_empty());
}

// 12. ExpressionNode with unknown identifier → CLT104
#[test]
fn analyze_expression_node_unknown_ident_error() {
    let t = test_tokens();
    let p = program("", vec![expr_node("foo")]);
    let (errors, _) = analyze(&p, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("foo"));
}

// 13. <each> alias in scope for children → no CLT104
#[test]
fn analyze_each_alias_in_scope_for_children() {
    let t = test_tokens();
    let child = component("Text", vec![prop_expr("value", "item")], vec![]);
    let p = program("const items = [];", vec![
        each_node("items", "item", vec![child]),
    ]);
    let (errors, _) = analyze(&p, &t);
    assert!(errors.is_empty());
}

// --- unsafe block (CLT105) ---

// 14. Well-formed <unsafe reason="…"> → no errors, one warning mentioning the reason
#[test]
fn analyze_unsafe_block_well_formed_emits_warning() {
    let t = test_tokens();
    let p = program("", vec![unsafe_node("not in the design yet", vec![
        component("Column", vec![prop_str("gap", "md")], vec![]),
    ])]);
    let (errors, warnings) = analyze(&p, &t);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("not in the design yet"));
}

// 15. <unsafe reason=""> → CLT105 error, no warning
#[test]
fn analyze_unsafe_block_empty_reason_clt105() {
    let t = test_tokens();
    let p = program("", vec![unsafe_node("", vec![])]);
    let (errors, warnings) = analyze(&p, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("CLT105"));
    assert_eq!(errors[0].code, codes::CLT105);
    assert!(warnings.is_empty());
}

// 16. Children inside well-formed unsafe still validate CLT104
#[test]
fn analyze_unsafe_block_children_still_validate_clt104() {
    let t = test_tokens();
    let p = program("", vec![unsafe_node("valid reason", vec![expr_node("undeclared")])]);
    let (errors, _) = analyze(&p, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("CLT104"));
    assert_eq!(errors[0].code, codes::CLT104);
}

// --- unsafe prop value (CLT106) ---

// 17. Well-formed unsafe() value → no error, one warning mentioning the reason
#[test]
fn analyze_unsafe_value_well_formed_emits_warning() {
    let t = test_tokens();
    let p = program("", vec![component("Column", vec![
        prop_unsafe_val("gap", "16px", "not in the design yet"),
    ], vec![])]);
    let (errors, warnings) = analyze(&p, &t);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("not in the design yet"));
}

// 18. unsafe() value with empty reason → CLT106 error, no warning
#[test]
fn analyze_unsafe_value_empty_reason_clt106() {
    let t = test_tokens();
    let p = program("", vec![component("Column", vec![
        prop_unsafe_val("gap", "16px", ""),
    ], vec![])]);
    let (errors, warnings) = analyze(&p, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("CLT106"));
    assert_eq!(errors[0].code, codes::CLT106);
    assert!(warnings.is_empty());
}

// --- CLT107: complex expression outside unsafe block ---

// 19. Simple identifier outside unsafe → no CLT107
#[test]
fn analyze_simple_expr_no_clt107() {
    let t = test_tokens();
    let p = program("const count = 0;", vec![expr_node("count")]);
    let (errors, _) = analyze(&p, &t);
    assert!(errors.is_empty());
}

// 20. Complex expression outside unsafe → CLT107
#[test]
fn analyze_complex_expr_outside_unsafe_clt107() {
    let t = test_tokens();
    let p = program("", vec![expr_node("count + 1")]);
    let (errors, _) = analyze(&p, &t);
    assert!(errors.iter().any(|e| e.message.contains("CLT107")),
        "complex expression should trigger CLT107, got: {:?}", errors);
    assert!(errors.iter().any(|e| e.code == codes::CLT107));
}

// 21. Complex expression inside well-formed unsafe → CLT107 suppressed
#[test]
fn analyze_complex_expr_inside_unsafe_no_clt107() {
    let t = test_tokens();
    let p = program("const count = 0;", vec![
        unsafe_node("I know what I'm doing", vec![expr_node("count + 1")]),
    ]);
    let (errors, _) = analyze(&p, &t);
    assert!(!errors.iter().any(|e| e.message.contains("CLT107")));
}

// --- extract_identifiers ---

#[test]
fn extract_identifiers_const_let_var() {
    let ids = extract_identifiers("const title = \"Hello\";\nlet count = 0;\nvar flag = true;");
    assert!(ids.contains("title"));
    assert!(ids.contains("count"));
    assert!(ids.contains("flag"));
}

#[test]
fn extract_identifiers_function_and_component() {
    let ids = extract_identifiers("function handleClick() {}\ncomponent Card(props) {}");
    assert!(ids.contains("handleClick"));
    assert!(ids.contains("Card"));
}

#[test]
fn extract_identifiers_empty_logic_block() {
    assert!(extract_identifiers("").is_empty());
}

#[test]
fn extract_identifiers_does_not_include_values() {
    let ids = extract_identifiers("const title = \"Hello\";");
    assert!(!ids.contains("Hello"));
}
