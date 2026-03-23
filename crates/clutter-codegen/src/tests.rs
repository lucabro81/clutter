use crate::css::generate_css;
use crate::vue::generate_sfc;
use clutter_runtime::{
    ComponentDef, ComponentNode, EachNode, ExpressionNode, FileNode, IfNode, Node, Position,
    PropNode, PropValue, TextNode, UnsafeNode, DesignTokens,
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

// ---------------------------------------------------------------------------
// CSS — base component classes
// ---------------------------------------------------------------------------

#[test]
fn css_column_base_class() {
    let css = generate_css(&test_tokens());
    assert!(css.contains(".clutter-column { display: flex; flex-direction: column; }"), "{css}");
}

#[test]
fn css_row_base_class() {
    let css = generate_css(&test_tokens());
    assert!(css.contains(".clutter-row { display: flex; flex-direction: row; }"), "{css}");
}

#[test]
fn css_box_base_class() {
    let css = generate_css(&test_tokens());
    assert!(css.contains(".clutter-box { box-sizing: border-box; }"), "{css}");
}

#[test]
fn css_text_base_class() {
    let css = generate_css(&test_tokens());
    assert!(css.contains(".clutter-text {"), "{css}");
}

#[test]
fn css_button_base_class() {
    let css = generate_css(&test_tokens());
    assert!(css.contains(".clutter-button { cursor: pointer; }"), "{css}");
}

#[test]
fn css_input_base_class() {
    let css = generate_css(&test_tokens());
    assert!(css.contains(".clutter-input {"), "{css}");
}

// ---------------------------------------------------------------------------
// CSS — spacing token classes (gap, padding, margin)
// ---------------------------------------------------------------------------

#[test]
fn css_gap_classes_per_spacing_token() {
    let css = generate_css(&test_tokens());
    for val in ["xs", "sm", "md", "lg", "xl", "xxl"] {
        let expected = format!(".clutter-gap-{val} {{ gap: var(--spacing-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

#[test]
fn css_padding_classes_per_spacing_token() {
    let css = generate_css(&test_tokens());
    for val in ["xs", "sm", "md", "lg", "xl", "xxl"] {
        let expected = format!(".clutter-padding-{val} {{ padding: var(--spacing-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

#[test]
fn css_margin_classes_per_spacing_token() {
    let css = generate_css(&test_tokens());
    for val in ["xs", "sm", "md", "lg", "xl", "xxl"] {
        let expected = format!(".clutter-margin-{val} {{ margin: var(--spacing-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

// ---------------------------------------------------------------------------
// CSS — color token classes (bg, color)
// ---------------------------------------------------------------------------

#[test]
fn css_bg_classes_per_color_token() {
    let css = generate_css(&test_tokens());
    for val in ["primary", "secondary", "danger", "surface", "background"] {
        let expected = format!(".clutter-bg-{val} {{ background-color: var(--color-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

#[test]
fn css_color_classes_per_color_token() {
    let css = generate_css(&test_tokens());
    for val in ["primary", "secondary", "danger", "surface", "background"] {
        let expected = format!(".clutter-color-{val} {{ color: var(--color-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

// ---------------------------------------------------------------------------
// CSS — typography token classes (size, weight)
// ---------------------------------------------------------------------------

#[test]
fn css_size_classes_per_font_size_token() {
    let css = generate_css(&test_tokens());
    for val in ["xs", "sm", "base", "lg", "xl", "xxl"] {
        let expected = format!(".clutter-size-{val} {{ font-size: var(--size-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

#[test]
fn css_weight_classes_per_font_weight_token() {
    let css = generate_css(&test_tokens());
    for val in ["normal", "medium", "semibold", "bold"] {
        let expected = format!(".clutter-weight-{val} {{ font-weight: var(--weight-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

// ---------------------------------------------------------------------------
// CSS — radius + shadow token classes
// ---------------------------------------------------------------------------

#[test]
fn css_radius_classes_per_radius_token() {
    let css = generate_css(&test_tokens());
    for val in ["none", "sm", "md", "lg", "full"] {
        let expected = format!(".clutter-radius-{val} {{ border-radius: var(--radius-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

#[test]
fn css_shadow_classes_per_shadow_token() {
    let css = generate_css(&test_tokens());
    for val in ["sm", "md", "lg"] {
        let expected = format!(".clutter-shadow-{val} {{ box-shadow: var(--shadow-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

// ---------------------------------------------------------------------------
// AST helpers
// ---------------------------------------------------------------------------

fn pos() -> Position { Position { line: 1, col: 1 } }

fn prop_str(name: &str, val: &str) -> PropNode {
    PropNode { name: name.to_string(), value: PropValue::StringValue(val.to_string()), pos: pos() }
}

fn prop_expr(name: &str, expr: &str) -> PropNode {
    PropNode { name: name.to_string(), value: PropValue::ExpressionValue(expr.to_string()), pos: pos() }
}

fn prop_unsafe_val(name: &str, val: &str) -> PropNode {
    PropNode {
        name: name.to_string(),
        value: PropValue::UnsafeValue { value: val.to_string(), reason: "legacy".to_string() },
        pos: pos(),
    }
}

fn comp_node(name: &str, props: Vec<PropNode>, children: Vec<Node>) -> Node {
    Node::Component(ComponentNode { name: name.to_string(), props, children, pos: pos() })
}

fn text_node(value: &str) -> Node {
    Node::Text(TextNode { value: value.to_string(), pos: pos() })
}

fn expr_node(value: &str) -> Node {
    Node::Expr(ExpressionNode { value: value.to_string(), pos: pos() })
}

fn if_node(cond: &str, then: Vec<Node>, else_: Option<Vec<Node>>) -> Node {
    Node::If(IfNode { condition: cond.to_string(), then_children: then, else_children: else_, pos: pos() })
}

fn each_node(collection: &str, alias: &str, children: Vec<Node>) -> Node {
    Node::Each(EachNode { collection: collection.to_string(), alias: alias.to_string(), children, pos: pos() })
}

fn unsafe_node(children: Vec<Node>) -> Node {
    Node::Unsafe(UnsafeNode { reason: "test".to_string(), children, pos: pos() })
}

fn comp_def(name: &str, logic: &str, template: Vec<Node>) -> ComponentDef {
    ComponentDef {
        name: name.to_string(),
        props_raw: String::new(),
        logic_block: logic.to_string(),
        template,
    }
}

fn file_node(components: Vec<ComponentDef>) -> FileNode {
    FileNode { components }
}

// ---------------------------------------------------------------------------
// Vue — template node generation
// ---------------------------------------------------------------------------

#[test]
fn vue_column_no_props() {
    let sfc = generate_sfc(&comp_def("C", "", vec![comp_node("Column", vec![], vec![])]), &test_tokens());
    assert!(sfc.contains("<div class=\"clutter-column\">"), "{sfc}");
}

#[test]
fn vue_column_string_gap_prop() {
    let sfc = generate_sfc(
        &comp_def("C", "", vec![comp_node("Column", vec![prop_str("gap", "md")], vec![])]),
        &test_tokens(),
    );
    assert!(sfc.contains("class=\"clutter-column clutter-gap-md\""), "{sfc}");
}

#[test]
fn vue_column_expr_gap_prop() {
    let sfc = generate_sfc(
        &comp_def("C", "", vec![comp_node("Column", vec![prop_expr("gap", "size")], vec![])]),
        &test_tokens(),
    );
    assert!(sfc.contains(":gap=\"size\""), "{sfc}");
    assert!(sfc.contains("clutter-column"), "{sfc}");
}

#[test]
fn vue_text_string_value_prop() {
    let sfc = generate_sfc(
        &comp_def("C", "", vec![comp_node("Text", vec![prop_str("value", "Hello")], vec![])]),
        &test_tokens(),
    );
    assert!(sfc.contains("<p class=\"clutter-text\">Hello</p>"), "{sfc}");
}

#[test]
fn vue_text_expr_value_prop() {
    let sfc = generate_sfc(
        &comp_def("C", "", vec![comp_node("Text", vec![prop_expr("value", "title")], vec![])]),
        &test_tokens(),
    );
    assert!(sfc.contains("<p class=\"clutter-text\">{{ title }}</p>"), "{sfc}");
}

#[test]
fn vue_button_variant_and_disabled() {
    let sfc = generate_sfc(
        &comp_def("C", "", vec![comp_node("Button", vec![
            prop_str("variant", "primary"),
            prop_expr("disabled", "loading"),
        ], vec![])]),
        &test_tokens(),
    );
    assert!(sfc.contains("class=\"clutter-button clutter-variant-primary\""), "{sfc}");
    assert!(sfc.contains(":disabled=\"loading\""), "{sfc}");
}

#[test]
fn vue_input_is_self_closing() {
    let sfc = generate_sfc(
        &comp_def("C", "", vec![comp_node("Input", vec![], vec![])]),
        &test_tokens(),
    );
    assert!(sfc.contains("<input class=\"clutter-input\" />"), "{sfc}");
}

#[test]
fn vue_custom_component_passthrough() {
    let sfc = generate_sfc(
        &comp_def("C", "", vec![comp_node("MyCard", vec![prop_str("foo", "bar")], vec![])]),
        &test_tokens(),
    );
    assert!(sfc.contains("<MyCard foo=\"bar\" />"), "{sfc}");
}

#[test]
fn vue_text_node_verbatim() {
    let sfc = generate_sfc(
        &comp_def("C", "", vec![text_node("hello world")]),
        &test_tokens(),
    );
    assert!(sfc.contains("hello world"), "{sfc}");
}

#[test]
fn vue_expression_node() {
    let sfc = generate_sfc(
        &comp_def("C", "", vec![expr_node("count")]),
        &test_tokens(),
    );
    assert!(sfc.contains("{{ count }}"), "{sfc}");
}

#[test]
fn vue_nesting_indentation() {
    let inner = comp_node("Text", vec![], vec![]);
    let outer = comp_node("Column", vec![], vec![inner]);
    let sfc = generate_sfc(&comp_def("C", "", vec![outer]), &test_tokens());
    // Column at depth 0, Text child at depth 1 (2-space indent)
    assert!(sfc.contains("  <p class=\"clutter-text\">"), "{sfc}");
}

#[test]
fn vue_if_single_child_no_else() {
    let child = comp_node("Text", vec![], vec![]);
    let sfc = generate_sfc(
        &comp_def("C", "", vec![if_node("isVisible", vec![child], None)]),
        &test_tokens(),
    );
    assert!(sfc.contains("v-if=\"isVisible\""), "{sfc}");
    assert!(!sfc.contains("<template v-if"), "{sfc}");
}

#[test]
fn vue_if_single_child_with_else() {
    let then_child = comp_node("Text", vec![prop_str("value", "yes")], vec![]);
    let else_child = comp_node("Text", vec![prop_str("value", "no")], vec![]);
    let sfc = generate_sfc(
        &comp_def("C", "", vec![if_node("ok", vec![then_child], Some(vec![else_child]))]),
        &test_tokens(),
    );
    assert!(sfc.contains("v-if=\"ok\""), "{sfc}");
    assert!(sfc.contains("v-else"), "{sfc}");
}

#[test]
fn vue_if_multiple_then_children_uses_template_wrapper() {
    let children = vec![
        comp_node("Text", vec![prop_str("value", "a")], vec![]),
        comp_node("Text", vec![prop_str("value", "b")], vec![]),
    ];
    let sfc = generate_sfc(
        &comp_def("C", "", vec![if_node("cond", children, None)]),
        &test_tokens(),
    );
    assert!(sfc.contains("<template v-if=\"cond\">"), "{sfc}");
}

#[test]
fn vue_each_single_child() {
    let child = comp_node("Text", vec![], vec![]);
    let sfc = generate_sfc(
        &comp_def("C", "", vec![each_node("items", "item", vec![child])]),
        &test_tokens(),
    );
    assert!(sfc.contains("v-for=\"item in items\""), "{sfc}");
    assert!(sfc.contains(":key=\"item\""), "{sfc}");
    assert!(!sfc.contains("<template v-for"), "{sfc}");
}

#[test]
fn vue_each_multiple_children_uses_template_wrapper() {
    let children = vec![
        comp_node("Text", vec![], vec![]),
        comp_node("Text", vec![], vec![]),
    ];
    let sfc = generate_sfc(
        &comp_def("C", "", vec![each_node("items", "item", children)]),
        &test_tokens(),
    );
    assert!(sfc.contains("<template v-for=\"item in items\""), "{sfc}");
}

#[test]
fn vue_unsafe_node_no_wrapper() {
    let child = comp_node("Text", vec![], vec![]);
    let sfc = generate_sfc(
        &comp_def("C", "", vec![unsafe_node(vec![child])]),
        &test_tokens(),
    );
    assert!(!sfc.contains("<unsafe"), "{sfc}");
    assert!(sfc.contains("clutter-text"), "{sfc}");
}

#[test]
fn vue_unsafe_value_prop_raw_no_css_class() {
    let sfc = generate_sfc(
        &comp_def("C", "", vec![comp_node("Column", vec![prop_unsafe_val("gap", "16px")], vec![])]),
        &test_tokens(),
    );
    assert!(sfc.contains("gap=\"16px\""), "{sfc}");
    assert!(!sfc.contains("clutter-gap-16px"), "{sfc}");
}

// ---------------------------------------------------------------------------
// Vue — full SFC generation
// ---------------------------------------------------------------------------

#[test]
fn vue_sfc_empty_template() {
    let sfc = generate_sfc(&comp_def("C", "", vec![]), &test_tokens());
    assert!(sfc.contains("<template>"), "{sfc}");
    assert!(sfc.contains("</template>"), "{sfc}");
}

#[test]
fn vue_sfc_logic_block_in_script_setup() {
    let logic = "const title = \"hello\";";
    let sfc = generate_sfc(&comp_def("C", logic, vec![]), &test_tokens());
    assert!(sfc.contains("<script setup lang=\"ts\">"), "{sfc}");
    assert!(sfc.contains(logic), "{sfc}");
}

#[test]
fn vue_sfc_empty_logic_block_script_present() {
    let sfc = generate_sfc(&comp_def("C", "", vec![]), &test_tokens());
    assert!(sfc.contains("<script setup lang=\"ts\">"), "{sfc}");
    assert!(sfc.contains("</script>"), "{sfc}");
}

#[test]
fn vue_sfc_style_scoped_non_empty() {
    let sfc = generate_sfc(&comp_def("C", "", vec![]), &test_tokens());
    assert!(sfc.contains("<style scoped>"), "{sfc}");
    assert!(sfc.contains("</style>"), "{sfc}");
    // Style block must contain at least the base component classes
    assert!(sfc.contains("clutter-column"), "{sfc}");
}

// generate_node_with_directive injects v-if/v-for by finding the first `<tag`
// boundary.  All existing tests use Text (non-self-closing).  Verify the
// injection also produces valid output for Input, which renders as
// `<input class="clutter-input" />` — a self-closing tag.
#[test]
fn vue_if_directive_on_self_closing_element() {
    let input = comp_node("Input", vec![], vec![]);
    let sfc = generate_sfc(
        &comp_def("C", "", vec![if_node("show", vec![input], None)]),
        &test_tokens(),
    );
    // v-if must appear between the tag name and the class attribute, not after />
    assert!(sfc.contains(r#"<input v-if="show" class="clutter-input" />"#), "{sfc}");
    // Must still be self-closing (no stray open tag left behind)
    assert!(!sfc.contains("</input>"), "{sfc}");
}

// SFC section order: <template> must appear before <script>, which must appear
// before <style scoped>. A viewer or toolchain may depend on this canonical order.
#[test]
fn vue_sfc_sections_in_canonical_order() {
    let sfc = generate_sfc(&comp_def("C", "const x = 1;", vec![]), &test_tokens());
    let template_pos = sfc.find("<template>").expect("<template> not found");
    let script_pos   = sfc.find("<script setup").expect("<script setup> not found");
    let style_pos    = sfc.find("<style scoped>").expect("<style scoped> not found");
    assert!(
        template_pos < script_pos,
        "<template> must precede <script setup>: template={template_pos}, script={script_pos}"
    );
    assert!(
        script_pos < style_pos,
        "<script setup> must precede <style scoped>: script={script_pos}, style={style_pos}"
    );
}

#[test]
fn vue_file_node_one_component() {
    use crate::generate_vue;
    let files = generate_vue(&file_node(vec![comp_def("Main", "", vec![])]), &test_tokens());
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "Main");
}

#[test]
fn vue_file_node_two_components() {
    use crate::generate_vue;
    let files = generate_vue(
        &file_node(vec![comp_def("A", "", vec![]), comp_def("B", "", vec![])]),
        &test_tokens(),
    );
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].name, "A");
    assert_eq!(files[1].name, "B");
}
