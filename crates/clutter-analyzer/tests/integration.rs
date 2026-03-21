use clutter_analyzer::{analyze, DesignTokens};
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

fn pipeline(fixture_name: &str) -> (clutter_runtime::ProgramNode, DesignTokens) {
    let src = fixture(fixture_name);
    let (tokens, lex_errors) = tokenize(&src);
    assert!(lex_errors.is_empty(), "unexpected lex errors: {:?}", lex_errors);
    let (program, parse_errors) = Parser::new(tokens).parse_program();
    assert!(parse_errors.is_empty(), "unexpected parse errors: {:?}", parse_errors);
    (program, tokens_json())
}

#[test]
fn valid_file_no_errors() {
    let (program, tokens) = pipeline("valid");
    let (errors, _) = analyze(&program, &tokens);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn invalid_token_file_has_errors() {
    let (program, tokens) = pipeline("invalid_token");
    let (errors, _) = analyze(&program, &tokens);
    assert!(!errors.is_empty(), "expected at least one error");
    // gap="xl2" → CLT102
    assert!(errors.iter().any(|e| e.message.contains("xl2")), "expected error for 'xl2'");
    // size="huge" → CLT102
    assert!(errors.iter().any(|e| e.message.contains("huge")), "expected error for 'huge'");
}

#[test]
fn complex_file_no_errors() {
    let (program, tokens) = pipeline("complex");
    let (errors, _) = analyze(&program, &tokens);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn unsafe_block_file_emits_warning_no_errors() {
    let (program, tokens) = pipeline("unsafe_block");
    let (errors, warnings) = analyze(&program, &tokens);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    assert!(!warnings.is_empty(), "expected at least one warning for <unsafe> block");
    assert!(warnings.iter().any(|w| w.message.contains("WARN")));
}

#[test]
fn unsafe_value_file_emits_warning_no_errors() {
    let (program, tokens) = pipeline("unsafe_value");
    let (errors, warnings) = analyze(&program, &tokens);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    assert!(!warnings.is_empty(), "expected at least one warning for unsafe() value");
    assert!(warnings.iter().any(|w| w.message.contains("WARN")));
}

#[test]
fn clt107_complex_expr_file_has_error() {
    let (program, tokens) = pipeline("clt107_complex_expr");
    let (errors, _) = analyze(&program, &tokens);
    assert!(
        errors.iter().any(|e| e.message.contains("CLT107")),
        "expected CLT107 error for complex expression, got: {:?}", errors
    );
}
