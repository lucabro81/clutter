use clutter_lexer::tokenize;
use clutter_parser::Parser;
use clutter_runtime::Node;

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/../../fixtures/{}.clutter",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("fixture not found: {}", path))
}

fn parse(name: &str) -> clutter_runtime::ProgramNode {
    let src = fixture(name);
    let (tokens, lex_errors) = tokenize(&src);
    assert!(lex_errors.is_empty(), "unexpected lex errors: {:?}", lex_errors);
    let (program, parse_errors) = Parser::new(tokens).parse_program();
    assert!(parse_errors.is_empty(), "unexpected parse errors: {:?}", parse_errors);
    program
}

// 1. simple_component.clutter → one ComponentNode named Column, no children
#[test]
fn simple_component() {
    let program = parse("simple_component");
    assert_eq!(program.template.len(), 1);
    match &program.template[0] {
        Node::Component(c) => {
            assert_eq!(c.name, "Column");
            assert!(c.props.is_empty());
            assert!(c.children.is_empty());
        }
        _ => panic!("expected ComponentNode"),
    }
}

// 2. props.clutter → Text with one string prop and one expression prop
#[test]
fn props() {
    let program = parse("props");
    match &program.template[0] {
        Node::Component(c) => {
            assert_eq!(c.name, "Text");
            assert_eq!(c.props.len(), 2);
            assert_eq!(c.props[0].name, "size");
            assert_eq!(c.props[1].name, "value");
        }
        _ => panic!("expected ComponentNode"),
    }
}

// 3. nesting.clutter → Column with one Text child
#[test]
fn nesting() {
    let program = parse("nesting");
    match &program.template[0] {
        Node::Component(column) => {
            assert_eq!(column.name, "Column");
            assert_eq!(column.children.len(), 1);
            match &column.children[0] {
                Node::Component(text) => assert_eq!(text.name, "Text"),
                _ => panic!("expected Text child"),
            }
        }
        _ => panic!("expected Column"),
    }
}

// 4. if_else.clutter → IfNode with both branches populated
#[test]
fn if_else() {
    let program = parse("if_else");
    match &program.template[0] {
        Node::If(n) => {
            assert_eq!(n.condition, "isVisible");
            assert_eq!(n.then_children.len(), 1);
            let else_kids = n.else_children.as_ref().expect("expected else branch");
            assert_eq!(else_kids.len(), 1);
        }
        _ => panic!("expected IfNode"),
    }
}

// 5. logic_block.clutter → ProgramNode.logic_block is non-empty
#[test]
fn logic_block() {
    let program = parse("logic_block");
    assert!(!program.logic_block.is_empty());
    assert!(program.logic_block.contains("label"));
}
