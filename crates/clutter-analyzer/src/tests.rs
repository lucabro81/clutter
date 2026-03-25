use super::*;
use clutter_runtime::TokenCategory;
use clutter_runtime::{
    codes, ComponentDef, ComponentNode, EachNode, EventBinding, ExpressionNode, FileNode,
    IfNode, Node, Position, PropNode, PropValue, UnsafeNode,
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

fn component(name: &str, props: Vec<PropNode>, children: Vec<Node>) -> Node {
    Node::Component(ComponentNode { name: name.to_string(), props, events: vec![], children, pos: pos() })
}

fn component_with_events(
    name: &str,
    props: Vec<PropNode>,
    events: Vec<EventBinding>,
    children: Vec<Node>,
) -> Node {
    Node::Component(ComponentNode { name: name.to_string(), props, events, children, pos: pos() })
}

fn event(name: &str, handler: &str) -> EventBinding {
    EventBinding { name: name.to_string(), handler: handler.to_string(), pos: pos() }
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
        index_alias: None,
        children,
        pos: pos(),
    })
}

fn unsafe_node(reason: &str, children: Vec<Node>) -> Node {
    Node::Unsafe(UnsafeNode { reason: reason.to_string(), children, pos: pos() })
}

fn comp_def(name: &str, logic: &str, template: Vec<Node>) -> ComponentDef {
    ComponentDef {
        name: name.to_string(),
        props_raw: "props: P".to_string(),
        logic_block: logic.to_string(),
        template,
    }
}

fn single_file(logic: &str, template: Vec<Node>) -> FileNode {
    FileNode { components: vec![comp_def("Main", logic, template)] }
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

// --- analyze_file() ---

// 1. Valid prop value → no errors
#[test]
fn analyze_valid_prop_no_errors() {
    let t = test_tokens();
    let f = single_file("", vec![component("Column", vec![prop_str("gap", "md")], vec![])]);
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty());
}

// 2. Invalid prop value → CLT102 with message listing valid values
#[test]
fn analyze_invalid_token_value_error() {
    let t = test_tokens();
    let f = single_file("", vec![component("Column", vec![prop_str("gap", "xl2")], vec![])]);
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("xl2"), "message should mention the bad value");
    assert!(errors[0].message.contains("gap"), "message should mention the prop name");
}

// 3. ExpressionValue prop with known identifier → no errors
#[test]
fn analyze_expression_prop_known_ident_no_errors() {
    let t = test_tokens();
    let f = single_file("const myVar = 4;", vec![
        component("Column", vec![prop_expr("gap", "myVar")], vec![]),
    ]);
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty());
}

// 4. ExpressionValue prop with unknown identifier → CLT104
#[test]
fn analyze_expression_prop_unknown_ident_error() {
    let t = test_tokens();
    let f = single_file("", vec![component("Column", vec![prop_expr("gap", "unknown")], vec![])]);
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("unknown"), "message should mention the identifier");
}

// 5. Unknown component → CLT103
#[test]
fn analyze_unknown_component_error() {
    let t = test_tokens();
    let f = single_file("", vec![component("Grid", vec![], vec![])]);
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Grid"));
}

// 6. Unknown prop on known component → CLT101
#[test]
fn analyze_unknown_prop_on_known_component_error() {
    let t = test_tokens();
    let f = single_file("", vec![component("Column", vec![prop_str("color", "primary")], vec![])]);
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("color"));
    assert!(errors[0].message.contains("Column"));
}

// 7. Multiple errors collected
#[test]
fn analyze_multiple_errors_collected() {
    let t = test_tokens();
    let f = single_file("", vec![
        component("Column", vec![prop_str("gap", "bad1")], vec![]),
        component("Column", vec![prop_str("gap", "bad2")], vec![]),
    ]);
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 2);
}

// 8. Nested component — props validated the same way
#[test]
fn analyze_nested_component_props_validated() {
    let t = test_tokens();
    let inner = component("Text", vec![prop_str("size", "huge")], vec![]);
    let f = single_file("", vec![component("Column", vec![], vec![inner])]);
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("huge"));
}

// 9. Children of <if>/<each> validated recursively
#[test]
fn analyze_if_each_children_validated() {
    let t = test_tokens();
    let bad_child = component("Text", vec![prop_str("size", "nope")], vec![]);
    let f = single_file("const flag = true;", vec![if_node("flag", vec![bad_child])]);
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("nope"));
}

// 10. Empty template → no errors
#[test]
fn analyze_empty_template_no_errors() {
    let t = test_tokens();
    let f = single_file("", vec![]);
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty());
}

// 11. ExpressionNode with known identifier → no errors
#[test]
fn analyze_expression_node_known_ident_no_errors() {
    let t = test_tokens();
    let f = single_file("const title = \"Hello\";", vec![expr_node("title")]);
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty());
}

// 12. ExpressionNode with unknown identifier → CLT104
#[test]
fn analyze_expression_node_unknown_ident_error() {
    let t = test_tokens();
    let f = single_file("", vec![expr_node("foo")]);
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("foo"));
}

// 13. <each> alias in scope for children → no CLT104
#[test]
fn analyze_each_alias_in_scope_for_children() {
    let t = test_tokens();
    let child = component("Text", vec![prop_expr("value", "item")], vec![]);
    let f = single_file("const items = [];", vec![
        each_node("items", "item", vec![child]),
    ]);
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty());
}

// --- unsafe block (CLT105) ---

// 14. Well-formed <unsafe reason="…"> → no errors, one warning mentioning the reason
#[test]
fn analyze_unsafe_block_well_formed_emits_warning() {
    let t = test_tokens();
    let f = single_file("", vec![unsafe_node("not in the design yet", vec![
        component("Column", vec![prop_str("gap", "md")], vec![]),
    ])]);
    let (errors, warnings) = analyze_file(&f, &t);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("not in the design yet"));
}

// 15. <unsafe reason=""> → CLT105 error, no warning
#[test]
fn analyze_unsafe_block_empty_reason_clt105() {
    let t = test_tokens();
    let f = single_file("", vec![unsafe_node("", vec![])]);
    let (errors, warnings) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("CLT105"));
    assert_eq!(errors[0].code, codes::CLT105);
    assert!(warnings.is_empty());
}

// 16. Children inside well-formed unsafe still validate CLT104
#[test]
fn analyze_unsafe_block_children_still_validate_clt104() {
    let t = test_tokens();
    let f = single_file("", vec![unsafe_node("valid reason", vec![expr_node("undeclared")])]);
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("CLT104"));
    assert_eq!(errors[0].code, codes::CLT104);
}

// --- unsafe prop value (CLT106) ---

// 17. Well-formed unsafe() value → no error, one warning mentioning the reason
#[test]
fn analyze_unsafe_value_well_formed_emits_warning() {
    let t = test_tokens();
    let f = single_file("", vec![component("Column", vec![
        prop_unsafe_val("gap", "16px", "not in the design yet"),
    ], vec![])]);
    let (errors, warnings) = analyze_file(&f, &t);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("not in the design yet"));
}

// 18. unsafe() value with empty reason → CLT106 error, no warning
#[test]
fn analyze_unsafe_value_empty_reason_clt106() {
    let t = test_tokens();
    let f = single_file("", vec![component("Column", vec![
        prop_unsafe_val("gap", "16px", ""),
    ], vec![])]);
    let (errors, warnings) = analyze_file(&f, &t);
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
    let f = single_file("const count = 0;", vec![expr_node("count")]);
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty());
}

// 20. Complex expression outside unsafe → CLT107
#[test]
fn analyze_complex_expr_outside_unsafe_clt107() {
    let t = test_tokens();
    let f = single_file("", vec![expr_node("count + 1")]);
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.iter().any(|e| e.message.contains("CLT107")),
        "complex expression should trigger CLT107, got: {:?}", errors);
    assert!(errors.iter().any(|e| e.code == codes::CLT107));
}

// 21. Complex expression inside well-formed unsafe → CLT107 suppressed
#[test]
fn analyze_complex_expr_inside_unsafe_no_clt107() {
    let t = test_tokens();
    let f = single_file("const count = 0;", vec![
        unsafe_node("I know what I'm doing", vec![expr_node("count + 1")]),
    ]);
    let (errors, _) = analyze_file(&f, &t);
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

// -----------------------------------------------------------------------
// analyze_file() — multi-component API tests
// -----------------------------------------------------------------------

// 22. analyze_file: valid prop → no errors
#[test]
fn analyze_file_valid_prop_no_errors() {
    let t = test_tokens();
    let f = single_file("", vec![component("Column", vec![prop_str("gap", "md")], vec![])]);
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty());
}

// 23. analyze_file: invalid prop value → CLT102
#[test]
fn analyze_file_invalid_token_value_clt102() {
    let t = test_tokens();
    let f = single_file("", vec![component("Column", vec![prop_str("gap", "xl2")], vec![])]);
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, codes::CLT102);
}

// 24. analyze_file: unknown prop on known component → CLT101
#[test]
fn analyze_file_unknown_prop_clt101() {
    let t = test_tokens();
    let f = single_file("", vec![component("Column", vec![prop_str("color", "primary")], vec![])]);
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, codes::CLT101);
}

// 25. analyze_file: unknown component → CLT103
#[test]
fn analyze_file_unknown_component_clt103() {
    let t = test_tokens();
    let f = single_file("", vec![component("Grid", vec![], vec![])]);
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, codes::CLT103);
}

// 26. analyze_file: CLT104 still fires for undeclared identifier
#[test]
fn analyze_file_undeclared_identifier_clt104() {
    let t = test_tokens();
    let f = single_file("", vec![component("Column", vec![prop_expr("gap", "unknown")], vec![])]);
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, codes::CLT104);
}

// 27. analyze_file: two components — errors collected independently
#[test]
fn analyze_file_two_components_errors_independent() {
    let t = test_tokens();
    let f = FileNode {
        components: vec![
            comp_def("A", "", vec![component("Column", vec![prop_str("gap", "bad1")], vec![])]),
            comp_def("B", "", vec![component("Text", vec![prop_str("size", "bad2")], vec![])]),
        ],
    };
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 2);
}

// 28. analyze_file: component defined in the same file → no CLT103
#[test]
fn analyze_file_custom_component_no_clt103() {
    let t = test_tokens();
    let f = FileNode {
        components: vec![
            comp_def("Card", "", vec![]),
            comp_def("Main", "", vec![component("Card", vec![], vec![])]),
        ],
    };
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.iter().all(|e| e.code != codes::CLT103),
        "CLT103 should not fire for a component defined in the same file");
}

// 29. analyze_file: custom component props are not validated (AnyValue treatment)
#[test]
fn analyze_file_custom_component_props_not_validated() {
    let t = test_tokens();
    let f = FileNode {
        components: vec![
            comp_def("Card", "", vec![]),
            comp_def("Main", "", vec![
                component("Card", vec![prop_str("gap", "nonsense_value")], vec![]),
            ]),
        ],
    };
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty(), "custom component props should not be validated, got: {:?}", errors);
}

// 30. analyze_file: CLT105 still fires inside a component
#[test]
fn analyze_file_clt105_unsafe_empty_reason() {
    let t = test_tokens();
    let f = single_file("", vec![unsafe_node("", vec![])]);
    let (errors, _) = analyze_file(&f, &t);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, codes::CLT105);
}

// 31. analyze_file: CLT107 still fires for complex expressions outside unsafe
#[test]
fn analyze_file_clt107_complex_expression() {
    let t = test_tokens();
    let f = single_file("", vec![expr_node("count + 1")]);
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.iter().any(|e| e.code == codes::CLT107));
}

// -----------------------------------------------------------------------
// is_simple_identifier contract tests
// Documents CLT107's definition: only bare alphanumeric/underscore identifiers
// are allowed outside <unsafe>; anything else (property access, indexing,
// function calls) must be wrapped.
// -----------------------------------------------------------------------

// 32. Underscore-prefixed identifiers are simple (valid JS convention)
#[test]
fn simple_identifier_underscore_prefix_no_clt107() {
    let t = test_tokens();
    // "_private" starts with _ and has only alphanumeric chars after → simple
    let f = single_file("const _private = 1;", vec![expr_node("_private")]);
    let (errors, _) = analyze_file(&f, &t);
    assert!(
        !errors.iter().any(|e| e.code == codes::CLT107),
        "underscore-prefix identifier should not trigger CLT107, got: {:?}", errors
    );
}

// 33. Member access (dot notation) on a declared base is now allowed — no CLT107.
// Previously this required <unsafe>; the new semantics validate only the base identifier.
#[test]
fn property_access_allowed_with_declared_base() {
    let t = test_tokens();
    let f = single_file("const foo = {};", vec![expr_node("foo.bar")]);
    let (errors, _) = analyze_file(&f, &t);
    assert!(
        !errors.iter().any(|e| e.code == codes::CLT107),
        "member access 'foo.bar' with declared base should not trigger CLT107, got: {:?}", errors
    );
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

// 34. Array index access is complex → CLT107
#[test]
fn array_index_access_triggers_clt107() {
    let t = test_tokens();
    let f = single_file("const items = [];", vec![expr_node("items[0]")]);
    let (errors, _) = analyze_file(&f, &t);
    assert!(
        errors.iter().any(|e| e.code == codes::CLT107),
        "index access 'items[0]' should trigger CLT107, got: {:?}", errors
    );
}

// -----------------------------------------------------------------------
// <each> alias scoping
// -----------------------------------------------------------------------

// 32. <each> alias is valid inside its own body
#[test]
fn each_alias_valid_inside_body() {
    let t = test_tokens();
    // const items declared; alias "item" only in scope inside <each>
    let f = single_file(
        "const items = [];",
        vec![each_node("items", "item", vec![
            component("Text", vec![prop_expr("value", "item")], vec![]),
        ])],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty(), "alias should be valid inside <each>, got: {:?}", errors);
}

// 33. <each> alias does NOT leak outside its block → CLT104 outside
// Guards against alias scope leakage: the identifier added for the alias
// must not remain visible to siblings that come after the <each>.
#[test]
fn each_alias_does_not_leak_outside_block() {
    let t = test_tokens();
    // "item" is only declared as an alias inside the each; using it outside → CLT104
    let f = single_file(
        "const items = [];",
        vec![
            each_node("items", "item", vec![
                component("Text", vec![prop_expr("value", "item")], vec![]),
            ]),
            // sibling node after the each — alias must be out of scope here
            component("Text", vec![prop_expr("value", "item")], vec![]),
        ],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(
        errors.iter().any(|e| e.code == codes::CLT104 && e.message.contains("item")),
        "expected CLT104 for 'item' used outside <each>, got: {:?}", errors
    );
}

// -----------------------------------------------------------------------
// Event binding validation
// -----------------------------------------------------------------------

// -----------------------------------------------------------------------
// indexAs scope in <each>
// -----------------------------------------------------------------------

fn each_node_with_index(
    collection: &str,
    alias: &str,
    index_alias: &str,
    children: Vec<Node>,
) -> Node {
    Node::Each(EachNode {
        collection: collection.to_string(),
        alias: alias.to_string(),
        index_alias: Some(index_alias.to_string()),
        children,
        pos: pos(),
    })
}

// indexAs variable is in scope inside the each body → no error
#[test]
fn index_alias_in_scope_inside_each() {
    let t = test_tokens();
    let f = single_file(
        "const items = []",
        vec![each_node_with_index(
            "items",
            "item",
            "i",
            vec![component("Text", vec![prop_expr("value", "i")], vec![])],
        )],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

// indexAs variable used outside the each → CLT104
#[test]
fn index_alias_out_of_scope_outside_each() {
    let t = test_tokens();
    let f = single_file(
        "const items = []",
        vec![
            each_node_with_index("items", "item", "i", vec![]),
            // sibling after the each — i must be out of scope
            component("Text", vec![prop_expr("value", "i")], vec![]),
        ],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(
        errors.iter().any(|e| e.code == codes::CLT104 && e.message.contains("'i'")),
        "expected CLT104 for 'i' outside each scope, got: {:?}", errors
    );
}

// indexAs and alias both in scope simultaneously
#[test]
fn index_alias_and_alias_both_in_scope() {
    let t = test_tokens();
    let f = single_file(
        "const items = []",
        vec![each_node_with_index(
            "items",
            "item",
            "i",
            vec![
                component("Text", vec![prop_expr("value", "item")], vec![]),
                component("Text", vec![prop_expr("value", "i")], vec![]),
            ],
        )],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

// -----------------------------------------------------------------------
// Member access in expression props
// -----------------------------------------------------------------------

// {rule.field} with base identifier declared → no error
#[test]
fn member_access_declared_base_ok() {
    let t = test_tokens();
    let f = single_file(
        "const rule = {}",
        vec![component("Text", vec![prop_expr("value", "rule.field")], vec![])],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

// {rule.field} with base identifier NOT declared → CLT104 on base
#[test]
fn member_access_undeclared_base_clt104() {
    let t = test_tokens();
    let f = single_file(
        "",
        vec![component("Text", vec![prop_expr("value", "rule.field")], vec![])],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(
        errors.iter().any(|e| e.code == codes::CLT104 && e.message.contains("rule")),
        "expected CLT104 for undeclared base 'rule', got: {:?}", errors
    );
}

// {rule.field.nested} multi-level access → ok when base declared
#[test]
fn member_access_multi_level_ok() {
    let t = test_tokens();
    let f = single_file(
        "const rule = {}",
        vec![component("Text", vec![prop_expr("value", "rule.field.nested")], vec![])],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

// {a + b} is still CLT107 (no regression)
#[test]
fn member_access_operator_still_clt107() {
    let t = test_tokens();
    let f = single_file(
        "const a = 1\nconst b = 2",
        vec![component("Text", vec![prop_expr("value", "a + b")], vec![])],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(
        errors.iter().any(|e| e.code == codes::CLT107),
        "expected CLT107 for 'a + b', got: {:?}", errors
    );
}

// {fn()} is still CLT107 (no regression)
#[test]
fn member_access_function_call_still_clt107() {
    let t = test_tokens();
    let f = single_file(
        "const fn = () => {}",
        vec![component("Text", vec![prop_expr("value", "fn()")], vec![])],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(
        errors.iter().any(|e| e.code == codes::CLT107),
        "expected CLT107 for 'fn()', got: {:?}", errors
    );
}

// Member access works inside <each> for the loop alias
#[test]
fn member_access_on_each_alias_ok() {
    let t = test_tokens();
    let f = single_file(
        "const rules = []",
        vec![each_node(
            "rules",
            "rule",
            vec![component("Text", vec![prop_expr("value", "rule.field")], vec![])],
        )],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

// {foreign.field} inside <each> where `foreign` is neither the loop alias nor declared
// in the logic block → CLT104 on base `foreign`.
// Regression guard: member access validation must check the actual scope at the point of use,
// not just whether any identifier is in scope.
#[test]
fn member_access_undeclared_base_in_each_clt104() {
    let t = test_tokens();
    let f = single_file(
        "const rules = []",
        vec![each_node(
            "rules",
            "rule",
            vec![component("Text", vec![prop_expr("value", "foreign.field")], vec![])],
        )],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(
        errors.iter().any(|e| e.code == codes::CLT104 && e.message.contains("foreign")),
        "expected CLT104 for undeclared base 'foreign' inside <each>, got: {:?}", errors
    );
}

// -----------------------------------------------------------------------
// Select built-in component
// -----------------------------------------------------------------------

// <Select options={opts} value={v} size="base" /> → no errors
#[test]
fn select_valid_props_ok() {
    let t = test_tokens();
    let f = single_file(
        "const opts = []\nconst v = ''",
        vec![component("Select", vec![
            prop_expr("options", "opts"),
            prop_expr("value", "v"),
            prop_str("size", "base"),
        ], vec![])],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

// <Select size="invalid" /> → CLT102
#[test]
fn select_invalid_size_clt102() {
    let t = test_tokens();
    let f = single_file(
        "",
        vec![component("Select", vec![prop_str("size", "invalid")], vec![])],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(
        errors.iter().any(|e| e.code == codes::CLT102),
        "expected CLT102 for invalid size, got: {:?}", errors
    );
}

// <Select unknown="x" /> → CLT101
#[test]
fn select_unknown_prop_clt101() {
    let t = test_tokens();
    let f = single_file(
        "",
        vec![component("Select", vec![prop_str("unknown", "x")], vec![])],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(
        errors.iter().any(|e| e.code == codes::CLT101),
        "expected CLT101 for unknown prop, got: {:?}", errors
    );
}

// -----------------------------------------------------------------------
// Event binding validation
// -----------------------------------------------------------------------

// Event handler declared in logic block → no error
#[test]
fn event_handler_declared_ok() {
    let t = test_tokens();
    let f = single_file(
        "const addRule = () => {}",
        vec![component_with_events(
            "Button",
            vec![prop_str("variant", "primary")],
            vec![event("click", "addRule")],
            vec![],
        )],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

// Event handler not declared → CLT104
#[test]
fn event_handler_undeclared_clt104() {
    let t = test_tokens();
    let f = single_file(
        "",
        vec![component_with_events(
            "Button",
            vec![],
            vec![event("click", "addRule")],
            vec![],
        )],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(
        errors.iter().any(|e| e.code == codes::CLT104 && e.message.contains("addRule")),
        "expected CLT104 for undeclared handler 'addRule', got: {:?}", errors
    );
}

// Events do not interfere with prop validation (no regression)
#[test]
fn event_binding_does_not_affect_prop_validation() {
    let t = test_tokens();
    let f = single_file(
        "const handleClick = () => {}",
        vec![component_with_events(
            "Button",
            vec![prop_str("variant", "primary"), prop_str("size", "md")],
            vec![event("click", "handleClick")],
            vec![],
        )],
    );
    let (errors, _) = analyze_file(&f, &t);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}
