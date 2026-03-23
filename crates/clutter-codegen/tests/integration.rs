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

// 1. valid.clutter → generates a valid SFC with all three blocks
#[test]
fn valid_clutter_generates_valid_sfc() {
    let (file, tokens) = pipeline("valid");
    let files = generate_vue(&file, &tokens);
    assert_eq!(files.len(), 1);
    let sfc = &files[0].content;
    assert!(sfc.contains("<template>"), "{sfc}");
    assert!(sfc.contains("</template>"), "{sfc}");
    assert!(sfc.contains("<script setup"), "{sfc}");
    assert!(sfc.contains("</script>"), "{sfc}");
    assert!(sfc.contains("<style scoped>"), "{sfc}");
    assert!(sfc.contains("</style>"), "{sfc}");
}

// 2. logic_block.clutter → logic block appears verbatim in <script setup>
#[test]
fn logic_block_appears_in_script_setup() {
    let (file, tokens) = pipeline("logic_block");
    let files = generate_vue(&file, &tokens);
    let sfc = &files[0].content;
    assert!(sfc.contains("const label = \"hello\";"), "{sfc}");
    assert!(sfc.contains("const isVisible = true;"), "{sfc}");
}

// 3. if_else.clutter → output contains v-if and v-else
#[test]
fn if_else_generates_v_if_and_v_else() {
    let (file, tokens) = pipeline("if_else");
    let files = generate_vue(&file, &tokens);
    let sfc = &files[0].content;
    assert!(sfc.contains("v-if="), "{sfc}");
    assert!(sfc.contains("v-else"), "{sfc}");
}

// 4. nesting.clutter → Column child (Text) is indented by 2 spaces
#[test]
fn nesting_is_correctly_indented() {
    let (file, tokens) = pipeline("nesting");
    let files = generate_vue(&file, &tokens);
    let sfc = &files[0].content;
    // Text is the child of Column → depth 1 → 2-space indent
    assert!(sfc.contains("  <p ") || sfc.contains("  <p>"), "{sfc}");
}
