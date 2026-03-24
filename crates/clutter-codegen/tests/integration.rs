use clutter_analyzer::{analyze_file, DesignTokens};
use clutter_codegen::generate_vue;
use clutter_lexer::tokenize;
use clutter_parser::Parser;

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/../../fixtures/{}.clutter",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("fixture not found: {}", path))
}

fn tokens_json() -> DesignTokens {
    let path = format!("{}/../../tokens.json", env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("tokens.json not found: {}", path));
    DesignTokens::from_str(&src).expect("tokens.json should parse")
}

fn pipeline(fixture_name: &str) -> (clutter_runtime::FileNode, DesignTokens) {
    let src = fixture(fixture_name);
    let (tokens, lex_errors) = tokenize(&src);
    assert!(lex_errors.is_empty(), "unexpected lex errors: {lex_errors:?}");
    let (file, parse_errors) = Parser::new(tokens).parse_file();
    assert!(parse_errors.is_empty(), "unexpected parse errors: {parse_errors:?}");
    let design_tokens = tokens_json();
    let (errors, _) = analyze_file(&file, &design_tokens);
    assert!(errors.is_empty(), "unexpected analyzer errors: {errors:?}");
    (file, design_tokens)
}

// 1. valid.clutter → SFC structure + CSS classes from string props + Vue interpolation
//    from expression prop
#[test]
fn valid_clutter_generates_valid_sfc() {
    let (file, tokens) = pipeline("valid");
    let files = generate_vue(&file);
    assert_eq!(files.len(), 1);
    let sfc = &files[0].content;
    // Structure: only <template> and <script setup>; no <style> (CSS lives in clutter.css)
    assert!(sfc.contains("<template>"), "{sfc}");
    assert!(sfc.contains("</template>"), "{sfc}");
    assert!(sfc.contains("<script setup lang=\"ts\">"), "{sfc}");
    assert!(sfc.contains("</script>"), "{sfc}");
    assert!(!sfc.contains("<style"), "SFC should not contain a <style> section: {sfc}");
    // Column element: gap and padding props become CSS utility classes on the element
    assert!(sfc.contains(r#"class="clutter-column clutter-gap-md clutter-padding-lg""#), "{sfc}");
    // Text element: size and color props become CSS utility classes; expression value → interpolation
    assert!(sfc.contains(r#"class="clutter-text clutter-size-base clutter-color-primary""#), "{sfc}");
    // Expression prop on Text value → Vue interpolation
    assert!(sfc.contains("{{ title }}"), "{sfc}");
}

// 2. logic_block.clutter → logic block appears verbatim in <script setup>
#[test]
fn logic_block_appears_in_script_setup() {
    let (file, tokens) = pipeline("logic_block");
    let files = generate_vue(&file);
    let sfc = &files[0].content;
    assert!(sfc.contains("const label = \"hello\";"), "{sfc}");
    assert!(sfc.contains("const isVisible = true;"), "{sfc}");
}

// 3. if_else.clutter → v-if with exact condition value, v-else on sibling element
#[test]
fn if_else_generates_v_if_and_v_else() {
    let (file, tokens) = pipeline("if_else");
    let files = generate_vue(&file);
    let sfc = &files[0].content;
    assert!(sfc.contains("v-if=\"isVisible\""), "{sfc}");
    assert!(sfc.contains("v-else"), "{sfc}");
}

// 4. nesting.clutter → Column at depth 0, Text child at depth 1 (2-space indent)
#[test]
fn nesting_is_correctly_indented() {
    let (file, tokens) = pipeline("nesting");
    let files = generate_vue(&file);
    let sfc = &files[0].content;
    assert!(sfc.contains("<div class=\"clutter-column\">"), "{sfc}");
    assert!(sfc.contains("  <p class=\"clutter-text clutter-size-sm\">"), "{sfc}");
}

// 5. complex.clutter → if + each + deep nesting all in one SFC
#[test]
fn complex_generates_v_for_and_v_if() {
    let (file, tokens) = pipeline("complex");
    let files = generate_vue(&file);
    assert_eq!(files.len(), 1);
    let sfc = &files[0].content;
    // v-if on the Row (single then-child)
    assert!(sfc.contains("v-if=\"isVisible\""), "{sfc}");
    // v-for on the Text (single each-child)
    assert!(sfc.contains("v-for=\"item in items\""), "{sfc}");
    assert!(sfc.contains(":key=\"item\""), "{sfc}");
    // Expression values rendered as Vue interpolation
    assert!(sfc.contains("{{ title }}"), "{sfc}");
    assert!(sfc.contains("{{ item }}"), "{sfc}");
}

// 6. props.clutter → string prop → CSS class, expression prop on Text → interpolation
#[test]
fn props_generate_css_classes_and_bindings() {
    let (file, tokens) = pipeline("props");
    let files = generate_vue(&file);
    let sfc = &files[0].content;
    // size="base" → CSS utility class
    assert!(sfc.contains("clutter-size-base"), "{sfc}");
    // value={label} on Text → Vue interpolation
    assert!(sfc.contains("{{ label }}"), "{sfc}");
}

// 7. unsafe_block.clutter → <unsafe> wrapper absent, children rendered normally
#[test]
fn unsafe_block_transparent_in_output() {
    let (file, tokens) = pipeline("unsafe_block");
    let files = generate_vue(&file);
    let sfc = &files[0].content;
    assert!(!sfc.contains("<unsafe"), "{sfc}");
    assert!(sfc.contains("clutter-text"), "{sfc}");
    assert!(sfc.contains("{{ count }}"), "{sfc}");
}

// 8. multi_component.clutter → two GeneratedFiles, correct names and content
#[test]
fn multi_component_generates_two_files() {
    let (file, tokens) = pipeline("multi_component");
    let files = generate_vue(&file);
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].name, "Card");
    assert_eq!(files[1].name, "MainComponent");
    // Card: Box with padding — check element-level class attribute (template, not style block)
    assert!(files[0].content.contains(r#"class="clutter-box clutter-padding-md""#), "Card SFC: {}", files[0].content);
    // MainComponent: Column with gap — check element-level class attribute
    assert!(files[1].content.contains(r#"class="clutter-column clutter-gap-lg""#), "Main SFC: {}", files[1].content);
    // Custom component passthrough — <Card /> only appears in template, never in style
    assert!(files[1].content.contains("<Card />"), "Main SFC: {}", files[1].content);
}
