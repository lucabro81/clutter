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

fn parse(name: &str) -> clutter_runtime::FileNode {
    let src = fixture(name);
    let (tokens, lex_errors) = tokenize(&src);
    assert!(lex_errors.is_empty(), "unexpected lex errors: {:?}", lex_errors);
    let (file, parse_errors) = Parser::new(tokens).parse_file();
    assert!(parse_errors.is_empty(), "unexpected parse errors: {:?}", parse_errors);
    file
}

// 1. simple_component.clutter → one ComponentDef, template has one Column
#[test]
fn simple_component() {
    let file = parse("simple_component");
    assert_eq!(file.components.len(), 1);
    match &file.components[0].template[0] {
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
    let file = parse("props");
    match &file.components[0].template[0] {
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
    let file = parse("nesting");
    match &file.components[0].template[0] {
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
    let file = parse("if_else");
    match &file.components[0].template[0] {
        Node::If(n) => {
            assert_eq!(n.condition, "isVisible");
            assert_eq!(n.then_children.len(), 1);
            let else_kids = n.else_children.as_ref().expect("expected else branch");
            assert_eq!(else_kids.len(), 1);
        }
        _ => panic!("expected IfNode"),
    }
}

// 5. logic_block.clutter → ComponentDef.logic_block is non-empty and contains "label"
#[test]
fn logic_block() {
    let file = parse("logic_block");
    assert!(!file.components[0].logic_block.is_empty());
    assert!(file.components[0].logic_block.contains("label"));
}

// 6. orphan_else.clutter → parse error with descriptive message, no panic
#[test]
fn orphan_else_produces_error() {
    let src = fixture("orphan_else");
    let (tokens, lex_errors) = tokenize(&src);
    assert!(lex_errors.is_empty());
    let (_file, parse_errors) = Parser::new(tokens).parse_file();
    assert!(!parse_errors.is_empty());
    assert_eq!(parse_errors[0].message, "<else> without matching <if>");
}

// 7. complex.clutter → Column > Text + if > Row > each > Text; logic_block non-empty
#[test]
fn complex() {
    let file = parse("complex");
    assert!(!file.components[0].logic_block.is_empty());
    assert_eq!(file.components[0].template.len(), 1);
    match &file.components[0].template[0] {
        Node::Component(column) => {
            assert_eq!(column.name, "Column");
            assert_eq!(column.children.len(), 2);
            match &column.children[1] {
                Node::If(n) => {
                    assert_eq!(n.condition, "isVisible");
                    assert!(n.else_children.is_none());
                    match &n.then_children[0] {
                        Node::Component(row) => {
                            assert_eq!(row.name, "Row");
                            match &row.children[0] {
                                Node::Each(e) => {
                                    assert_eq!(e.collection, "items");
                                    assert_eq!(e.alias, "item");
                                    assert_eq!(e.children.len(), 1);
                                }
                                _ => panic!("expected EachNode"),
                            }
                        }
                        _ => panic!("expected Row"),
                    }
                }
                _ => panic!("expected IfNode"),
            }
        }
        _ => panic!("expected Column"),
    }
}
