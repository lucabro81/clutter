//! Semantic analyzer for the Clutter compiler.
//!
//! Third stage of the compilation pipeline:
//!
//! ```text
//! .clutter  →  Lexer  →  Parser  →  **Analyzer**  →  Codegen
//! ```
//!
//! Receives a [`ProgramNode`] (output of the parser) and a [`DesignTokens`]
//! (loaded from `tokens.json`) and produces a list of [`AnalyzerError`]. An empty
//! list means the source file is semantically valid.
//!
//! # Errors produced
//!
//! | Code    | Cause                                                                  |
//! |---------|------------------------------------------------------------------------|
//! | CLT101  | Unknown prop on a known component (e.g. `color` on `Column`)          |
//! | CLT102  | String value not present in the design system or the fixed enum        |
//! | CLT103  | Component not belonging to the closed vocabulary                       |
//! | CLT104  | Identifier used in an expression not declared in the logic block       |
//!
//! # Validation rules
//!
//! ## Prop type checking (CLT101–103)
//!
//! Every prop with a string literal value is checked against the design system.
//! The prop → category mapping is hardcoded in [`prop_map`] for the POC; all
//! valid values for a category are read from [`DesignTokens`].
//!
//! ## Reference checking (CLT104)
//!
//! Every expression `{name}` in the template — both as a [`Node::Expr`] and as a
//! [`PropValue::ExpressionValue`] — is checked against the set of identifiers
//! declared in the TypeScript logic block. Identifiers are extracted via a shallow
//! scan in [`extract_identifiers`].
//!
//! The alias introduced by `<each collection={…} as="alias">` is added to the valid
//! identifier set for the children of that node only.
//!
//! ## Unsafe validation (CLT105–106)
//!
//! Not yet implemented: requires `<unsafe>` support in the lexer and parser.
//! See the backlog for details.
//!
//! # Usage
//!
//! ```ignore
//! let json = std::fs::read_to_string("tokens.json")?;
//! let tokens = DesignTokens::from_str(&json)?;
//! let errors = analyze(&program, &tokens);
//! if errors.is_empty() {
//!     // proceed to codegen
//! }
//! ```

use std::collections::HashSet;

use clutter_runtime::{
    AnalyzerError, ComponentNode, EachNode, IfNode, Node, Position, PropNode, PropValue,
    ProgramNode,
};
use serde::Deserialize;

/// Closed vocabulary of components recognised by the analyzer.
///
/// A component not present in this list produces a CLT103 error. Its children
/// are still analysed recursively to collect all errors present.
const KNOWN_COMPONENTS: &[&str] = &["Column", "Row", "Box", "Text", "Button", "Input"];

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Semantically analyses a Clutter program and returns all errors found.
///
/// This is the crate's public function and represents the entire analysis phase.
/// It is called after the lexer and parser have produced a [`ProgramNode`] with
/// no errors.
///
/// # Algorithm
///
/// 1. Extracts identifiers declared in the TypeScript logic block.
/// 2. Recursively visits all template nodes via [`analyze_nodes`].
/// 3. Returns the complete error list (does not stop at the first error).
///
/// # Returns
///
/// An empty [`Vec<AnalyzerError>`] means the file is valid. Each element
/// describes a single semantic problem with a message and source position.
pub fn analyze(program: &ProgramNode, tokens: &DesignTokens) -> Vec<AnalyzerError> {
    let identifiers = extract_identifiers(&program.logic_block);
    let mut errors = Vec::new();
    analyze_nodes(&program.template, tokens, &identifiers, &mut errors);
    errors
}

// ---------------------------------------------------------------------------
// Recursive walker
// ---------------------------------------------------------------------------

/// Visits a slice of nodes and accumulates errors.
///
/// Dispatches each [`Node`] to its specific validator:
///
/// - [`Node::Component`] → [`analyze_component`]
/// - [`Node::Expr`] → CLT104 check on the identifier
/// - [`Node::If`] → [`analyze_if`]
/// - [`Node::Each`] → [`analyze_each`]
/// - [`Node::Text`] → no action (static text, nothing to validate)
fn analyze_nodes(
    nodes: &[Node],
    tokens: &DesignTokens,
    identifiers: &HashSet<String>,
    errors: &mut Vec<AnalyzerError>,
) {
    for node in nodes {
        match node {
            Node::Component(c) => analyze_component(c, tokens, identifiers, errors),
            Node::Expr(e) => {
                if let Some(err) = check_reference(&e.value, &e.pos, identifiers) {
                    errors.push(err);
                }
            }
            Node::If(i) => analyze_if(i, tokens, identifiers, errors),
            Node::Each(e) => analyze_each(e, tokens, identifiers, errors),
            Node::Text(_) => {}
        }
    }
}

/// Validates a component node: checks the name, props, and recurses into children.
///
/// # Logic
///
/// 1. If the name is not in [`KNOWN_COMPONENTS`] → CLT103 error; props are skipped
///    (no point validating them for an unknown component), but children are still
///    analysed to collect all possible errors.
/// 2. If the component is known → each prop is validated with [`validate_prop`].
/// 3. In both cases, recurse into children with the same identifier set.
fn analyze_component(
    node: &ComponentNode,
    tokens: &DesignTokens,
    identifiers: &HashSet<String>,
    errors: &mut Vec<AnalyzerError>,
) {
    if !KNOWN_COMPONENTS.contains(&node.name.as_str()) {
        errors.push(AnalyzerError {
            message: format!("CLT103: unknown component '{}'", node.name),
            pos: node.pos.clone(),
        });
        // Still recurse into children even for unknown components
    } else {
        for prop in &node.props {
            errors.extend(validate_prop(&node.name, prop, tokens, identifiers));
        }
    }
    analyze_nodes(&node.children, tokens, identifiers, errors);
}

/// Validates a single prop and returns zero or more errors.
///
/// The logic depends on what [`prop_map`] returns for the `(component, prop.name)` pair:
///
/// | `prop_map` result      | Action                                                                        |
/// |------------------------|-------------------------------------------------------------------------------|
/// | `None`                 | CLT101: unknown prop on the component                                         |
/// | `Some(AnyValue)`       | No value check; if the value is an expression → CLT104                        |
/// | `Some(Tokens(cat))`    | If string: checks against `tokens.valid_values(cat)` → CLT102 if absent; if expression → CLT104 |
/// | `Some(Enum(vals))`     | If string: checks against the fixed list `vals` → CLT102 if absent; if expression → CLT104 |
///
/// [`PropValue::ExpressionValue`] values are never checked against design tokens
/// because their value is determined at runtime: they are instead checked as
/// identifier references (CLT104).
fn validate_prop(
    component: &str,
    prop: &PropNode,
    tokens: &DesignTokens,
    identifiers: &HashSet<String>,
) -> Vec<AnalyzerError> {
    let mut errors = Vec::new();
    match prop_map(component, &prop.name) {
        None => {
            errors.push(AnalyzerError {
                message: format!(
                    "CLT101: unknown prop '{}' on '{}'",
                    prop.name, component
                ),
                pos: prop.pos.clone(),
            });
        }
        Some(PropValidation::AnyValue) => {
            if let PropValue::ExpressionValue(ref name) = prop.value {
                if let Some(err) = check_reference(name, &prop.pos, identifiers) {
                    errors.push(err);
                }
            }
        }
        Some(PropValidation::Tokens(cat)) => match &prop.value {
            PropValue::StringValue(val) => {
                let valid = tokens.valid_values(cat);
                if !valid.contains(val) {
                    errors.push(AnalyzerError {
                        message: format!(
                            "CLT102: invalid value '{}' for prop '{}' on '{}'. Valid values: {}",
                            val,
                            prop.name,
                            component,
                            valid.join(", ")
                        ),
                        pos: prop.pos.clone(),
                    });
                }
            }
            PropValue::ExpressionValue(name) => {
                if let Some(err) = check_reference(name, &prop.pos, identifiers) {
                    errors.push(err);
                }
            }
        },
        Some(PropValidation::Enum(vals)) => match &prop.value {
            PropValue::StringValue(val) => {
                if !vals.contains(&val.as_str()) {
                    errors.push(AnalyzerError {
                        message: format!(
                            "CLT102: invalid value '{}' for prop '{}' on '{}'. Valid values: {}",
                            val,
                            prop.name,
                            component,
                            vals.join(", ")
                        ),
                        pos: prop.pos.clone(),
                    });
                }
            }
            PropValue::ExpressionValue(name) => {
                if let Some(err) = check_reference(name, &prop.pos, identifiers) {
                    errors.push(err);
                }
            }
        },
    }
    errors
}

/// Validates an `<if>` node.
///
/// Checks that the `condition` expression is a declared identifier (CLT104), then
/// recurses into both the `then` branch and the optional `else` branch.
/// The identifier set is not extended: `<if>` introduces no new bindings.
fn analyze_if(
    node: &IfNode,
    tokens: &DesignTokens,
    identifiers: &HashSet<String>,
    errors: &mut Vec<AnalyzerError>,
) {
    if let Some(err) = check_reference(&node.condition, &node.pos, identifiers) {
        errors.push(err);
    }
    analyze_nodes(&node.then_children, tokens, identifiers, errors);
    if let Some(else_children) = &node.else_children {
        analyze_nodes(else_children, tokens, identifiers, errors);
    }
}

/// Validates an `<each>` node.
///
/// Checks that `collection` is a declared identifier (CLT104), then recurses into
/// children with an identifier set **extended** with the loop alias.
///
/// The alias (`node.alias`) is a binding introduced by `<each>` itself — for example,
/// `<each collection={items} as="item">` brings `"item"` into scope for all children.
/// It would be incorrect to report CLT104 for `{item}` used inside the loop.
fn analyze_each(
    node: &EachNode,
    tokens: &DesignTokens,
    identifiers: &HashSet<String>,
    errors: &mut Vec<AnalyzerError>,
) {
    if let Some(err) = check_reference(&node.collection, &node.pos, identifiers) {
        errors.push(err);
    }
    // The alias is in scope for children only — clone to avoid polluting the outer scope.
    let mut child_ids = identifiers.clone();
    child_ids.insert(node.alias.clone());
    analyze_nodes(&node.children, tokens, &child_ids, errors);
}

/// Checks that `name` is present in the set of declared identifiers.
///
/// Returns `None` if the reference is valid, or `Some(AnalyzerError)` with error
/// code CLT104 otherwise. Used by [`validate_prop`], [`analyze_if`], [`analyze_each`],
/// and directly by [`analyze_nodes`] for [`Node::Expr`] nodes.
fn check_reference(name: &str, pos: &Position, identifiers: &HashSet<String>) -> Option<AnalyzerError> {
    if identifiers.contains(name) {
        None
    } else {
        Some(AnalyzerError {
            message: format!("CLT104: undeclared identifier '{}'", name),
            pos: pos.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// Design token category that a prop value may belong to.
///
/// Used by [`PropValidation::Tokens`] to direct the lookup of valid values
/// in [`DesignTokens::valid_values`].
#[derive(Debug, Clone, Copy)]
enum TokenCategory {
    /// Spacing: gap, padding, margin. E.g. `xs | sm | md | lg | xl | xxl`.
    Spacing,
    /// Semantic colours. E.g. `primary | secondary | danger | surface | background`.
    Color,
    /// Typography sizes. E.g. `xs | sm | base | lg | xl | xxl`.
    FontSize,
    /// Typography weights. E.g. `normal | medium | semibold | bold`.
    FontWeight,
    /// Border radii. E.g. `none | sm | md | lg | full`.
    Radius,
    /// Shadows. E.g. `sm | md | lg`.
    Shadow,
}

/// Internal JSON structure of `tokens.json` for the typography section.
#[derive(Debug, Deserialize)]
struct Typography {
    sizes: Vec<String>,
    weights: Vec<String>,
}

/// Design system deserialised from `tokens.json`.
///
/// Holds the valid values for every token category. Built once at the start of
/// [`analyze`] and passed read-only throughout the entire tree walk.
///
/// # Expected JSON format
///
/// ```json
/// {
///   "spacing":    ["xs", "sm", "md", "lg", "xl", "xxl"],
///   "colors":     ["primary", "secondary", "danger", "surface", "background"],
///   "typography": { "sizes": [...], "weights": [...] },
///   "radii":      ["none", "sm", "md", "lg", "full"],
///   "shadows":    ["sm", "md", "lg"]
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct DesignTokens {
    spacing: Vec<String>,
    colors: Vec<String>,
    typography: Typography,
    radii: Vec<String>,
    shadows: Vec<String>,
}

/// Validation rule applicable to a prop in the closed vocabulary.
///
/// [`prop_map`] returns an `Option<PropValidation>`: `None` means the prop is not
/// recognised on the given component (→ CLT101).
enum PropValidation {
    /// The value must be present in a design system token category.
    Tokens(TokenCategory),
    /// The value must be one of the elements in the fixed set provided.
    Enum(&'static [&'static str]),
    /// The prop is valid with any string value; if it is an expression, the
    /// identifier name is still subject to the CLT104 check.
    AnyValue,
}

// ---------------------------------------------------------------------------
// Identifier extraction
// ---------------------------------------------------------------------------

/// Extracts identifiers declared in the TypeScript logic block.
///
/// Performs a shallow keyword-based scan: captures the name that immediately
/// follows `const`, `let`, `var`, `function`, or `component`.
///
/// # Known limitations
///
/// This implementation is intentionally approximate and suitable for the POC:
///
/// - **Destructuring**: `const { a, b } = obj` → neither `a` nor `b` are extracted.
/// - **Imports**: `import foo from "bar"` → `foo` is not extracted.
/// - **Type aliases** and closure variables are not recognised.
///
/// These cases are documented in the backlog as a *known limitation*.
fn extract_identifiers(logic_block: &str) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    let mut prev = "";
    for token in logic_block.split_whitespace() {
        // Take only the leading identifier portion: "handleClick(" → "handleClick"
        let name = token.split(|c: char| !c.is_alphanumeric() && c != '_').next().unwrap_or("");
        if matches!(prev, "const" | "let" | "var" | "function" | "component") && !name.is_empty() {
            ids.insert(name.to_string());
        }
        prev = token;
    }
    ids
}

// ---------------------------------------------------------------------------
// Prop map — closed vocabulary
// ---------------------------------------------------------------------------

/// Returns the validation rule for the `(component, prop)` pair.
///
/// # Returns
///
/// - `Some(PropValidation)` if the prop is recognised on the given component.
/// - `None` in two distinct cases, indistinguishable by signature but handled
///   differently by the caller [`validate_prop`]:
///   - The prop does not exist on the component (e.g. `color` on `Column`) → CLT101.
///   - The component itself is not in the vocabulary (e.g. `Grid`) → CLT103 is emitted
///     before this function is called, so `None` here is never reached for unknown
///     components.
///
/// # Extensibility
///
/// The map is hardcoded for the POC. Introducing new built-in components or
/// dynamic props is discussed in the backlog ("Dynamic prop map / custom components").
fn prop_map(component: &str, prop: &str) -> Option<PropValidation> {
    use PropValidation::*;
    use TokenCategory::*;

    const LAYOUT_AXES: &[&str] = &["start", "end", "center", "spaceBetween", "spaceAround", "spaceEvenly"];
    const CROSS_AXES:  &[&str] = &["start", "end", "center", "stretch"];
    const ALIGNS:      &[&str] = &["left", "center", "right"];
    const BTN_VARIANTS: &[&str] = &["primary", "secondary", "outline", "ghost", "danger"];
    const BTN_SIZES:    &[&str] = &["sm", "md", "lg"];
    const INPUT_TYPES:  &[&str] = &["text", "email", "password", "number"];

    match (component, prop) {
        ("Column" | "Row", "gap" | "padding") => Some(Tokens(Spacing)),
        ("Column" | "Row", "mainAxis")        => Some(Enum(LAYOUT_AXES)),
        ("Column" | "Row", "crossAxis")       => Some(Enum(CROSS_AXES)),
        ("Text", "value")                     => Some(AnyValue),
        ("Text", "size")                      => Some(Tokens(FontSize)),
        ("Text", "weight")                    => Some(Tokens(FontWeight)),
        ("Text", "color")                     => Some(Tokens(Color)),
        ("Text", "align")                     => Some(Enum(ALIGNS)),
        ("Button", "variant")                 => Some(Enum(BTN_VARIANTS)),
        ("Button", "size")                    => Some(Enum(BTN_SIZES)),
        ("Button", "disabled")                => Some(AnyValue),
        ("Box", "bg")                         => Some(Tokens(Color)),
        ("Box", "padding" | "margin")         => Some(Tokens(Spacing)),
        ("Box", "radius")                     => Some(Tokens(Radius)),
        ("Box", "shadow")                     => Some(Tokens(Shadow)),
        ("Input", "placeholder" | "value")    => Some(AnyValue),
        ("Input", "type")                     => Some(Enum(INPUT_TYPES)),
        ("Column" | "Row" | "Text" | "Button" | "Box" | "Input", _) => None, // known component, unknown prop
        _ => None, // unknown component
    }
}

// ---------------------------------------------------------------------------
// DesignTokens — impl
// ---------------------------------------------------------------------------

impl DesignTokens {
    /// Deserialises a [`DesignTokens`] from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns a [`serde_json::Error`] if the JSON is malformed or any required
    /// field is missing (`spacing`, `colors`, `typography`, `radii`, `shadows`).
    pub fn from_str(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Returns the slice of valid values for the requested category.
    ///
    /// Used by [`validate_prop`] to check the prop value and to build the CLT102
    /// error message listing accepted values.
    pub(crate) fn valid_values(&self, category: TokenCategory) -> &[String] {
        match category {
            TokenCategory::Spacing    => &self.spacing,
            TokenCategory::Color      => &self.colors,
            TokenCategory::FontSize   => &self.typography.sizes,
            TokenCategory::FontWeight => &self.typography.weights,
            TokenCategory::Radius     => &self.radii,
            TokenCategory::Shadow     => &self.shadows,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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

    // --- analyze() helpers ---

    use clutter_runtime::{
        ComponentNode, EachNode, ExpressionNode, IfNode, Node, Position, ProgramNode, PropNode,
        PropValue,
    };

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

    // 1. Valid prop value → no errors
    #[test]
    fn analyze_valid_prop_no_errors() {
        let t = test_tokens();
        let p = program("", vec![component("Column", vec![prop_str("gap", "md")], vec![])]);
        assert!(analyze(&p, &t).is_empty());
    }

    // 2. Invalid prop value → CLT102 with message listing valid values
    #[test]
    fn analyze_invalid_token_value_error() {
        let t = test_tokens();
        let p = program("", vec![component("Column", vec![prop_str("gap", "xl2")], vec![])]);
        let errors = analyze(&p, &t);
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
        assert!(analyze(&p, &t).is_empty());
    }

    // 4. ExpressionValue prop with unknown identifier → CLT104
    #[test]
    fn analyze_expression_prop_unknown_ident_error() {
        let t = test_tokens();
        let p = program("", vec![component("Column", vec![prop_expr("gap", "unknown")], vec![])]);
        let errors = analyze(&p, &t);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("unknown"), "message should mention the identifier");
    }

    // 5. Unknown component → CLT103
    #[test]
    fn analyze_unknown_component_error() {
        let t = test_tokens();
        let p = program("", vec![component("Grid", vec![], vec![])]);
        let errors = analyze(&p, &t);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Grid"));
    }

    // 6. Unknown prop on known component → CLT101
    #[test]
    fn analyze_unknown_prop_on_known_component_error() {
        let t = test_tokens();
        let p = program("", vec![component("Column", vec![prop_str("color", "primary")], vec![])]);
        let errors = analyze(&p, &t);
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
        assert_eq!(analyze(&p, &t).len(), 2);
    }

    // 8. Nested component — props validated the same way
    #[test]
    fn analyze_nested_component_props_validated() {
        let t = test_tokens();
        let inner = component("Text", vec![prop_str("size", "huge")], vec![]);
        let p = program("", vec![component("Column", vec![], vec![inner])]);
        let errors = analyze(&p, &t);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("huge"));
    }

    // 9. Children of <if>/<each> validated recursively
    #[test]
    fn analyze_if_each_children_validated() {
        let t = test_tokens();
        let bad_child = component("Text", vec![prop_str("size", "nope")], vec![]);
        let p = program("const flag = true;", vec![if_node("flag", vec![bad_child])]);
        let errors = analyze(&p, &t);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("nope"));
    }

    // 10. Empty template → no errors
    #[test]
    fn analyze_empty_template_no_errors() {
        let t = test_tokens();
        let p = program("", vec![]);
        assert!(analyze(&p, &t).is_empty());
    }

    // 11. ExpressionNode with known identifier → no errors
    #[test]
    fn analyze_expression_node_known_ident_no_errors() {
        let t = test_tokens();
        let p = program("const title = \"Hello\";", vec![expr_node("title")]);
        assert!(analyze(&p, &t).is_empty());
    }

    // 12. ExpressionNode with unknown identifier → CLT104
    #[test]
    fn analyze_expression_node_unknown_ident_error() {
        let t = test_tokens();
        let p = program("", vec![expr_node("foo")]);
        let errors = analyze(&p, &t);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("foo"));
    }

    // 13. <each> alias in scope for children → no CLT104
    #[test]
    fn analyze_each_alias_in_scope_for_children() {
        let t = test_tokens();
        // each collection={items} as="item" → "item" must be in scope for children
        let child = component("Text", vec![prop_expr("value", "item")], vec![]);
        let p = program("const items = [];", vec![
            each_node("items", "item", vec![child]),
        ]);
        assert!(analyze(&p, &t).is_empty());
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
}
