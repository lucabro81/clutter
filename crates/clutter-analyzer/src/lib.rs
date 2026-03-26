//! Semantic analyzer for the Clutter compiler.
//!
//! Third stage of the compilation pipeline:
//!
//! ```text
//! .clutter  →  Lexer  →  Parser  →  **Analyzer**  →  Codegen
//! ```
//!
//! Receives a [`FileNode`] (output of the parser) and a [`DesignTokens`]
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
//! The prop → category mapping is defined in [`vocabulary::VocabularyMap`].
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
//! ## Unsafe validation (CLT105–107)
//!
//! Well-formed unsafe constructs emit an [`AnalyzerWarning`] but do not block
//! compilation. Malformed ones (missing/empty reason) are hard errors.
//!
//! | Code   | Kind  | Trigger |
//! |--------|-------|---------|
//! | CLT105 | error | `<unsafe>` block with missing or empty `reason` |
//! | CLT106 | error | `unsafe('val', 'reason')` with empty reason |
//! | CLT107 | error | Complex `{}` expression outside an `<unsafe>` block |
//!
//! # Usage
//!
//! ```ignore
//! let json = std::fs::read_to_string("tokens.json")?;
//! let tokens = DesignTokens::from_str(&json)?;
//! let (errors, warnings) = analyze_file(&file, &tokens);
//! if errors.is_empty() {
//!     // proceed to codegen
//! }
//! ```

use std::collections::HashSet;

use clutter_runtime::{
    codes, AnalyzerError, AnalyzerWarning, ComponentDef, ComponentNode, EachNode,
    FileNode, IfNode, Node, Position, PropNode, PropValue, UnsafeNode,
};

mod vocabulary;

pub use clutter_runtime::DesignTokens;
use vocabulary::{PropValidation, VocabularyMap};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Semantically analyses a `.clutter` file and returns all errors and warnings.
///
/// Iterates over all [`ComponentDef`]s in the [`FileNode`]:
///
/// 1. Collects the set of component names defined in the file (for CLT103 suppression
///    on custom components).
/// 2. For each component: extracts identifiers from the logic block, then walks the
///    template with [`analyze_nodes`].
///
/// # Returns
///
/// `(errors, warnings)`. An empty `errors` vec means the file is valid and can
/// proceed to codegen.
///
/// # Examples
///
/// ```
/// use clutter_analyzer::{analyze_file, DesignTokens};
///
/// let json = r#"{"spacing":["sm","md"],"colors":["primary"],"typography":{"sizes":[],"weights":[]},"radii":[],"shadows":[]}"#;
/// let tokens = DesignTokens::from_str(json).unwrap();
///
/// let src = "component Foo(props: FooProps) {\n----\n<Column gap=\"sm\" />\n}";
/// let (tok, _) = clutter_lexer::tokenize(src);
/// let (file, _) = clutter_parser::Parser::new(tok).parse_file();
/// let (errors, _warnings) = analyze_file(&file, &tokens);
/// assert!(errors.is_empty());
/// ```
pub fn analyze_file(
    file: &FileNode,
    design_tokens: &DesignTokens,
) -> (Vec<AnalyzerError>, Vec<AnalyzerWarning>) {
    let vocab = VocabularyMap::new();
    let custom_components: HashSet<String> =
        file.components.iter().map(|c| c.name.clone()).collect();

    let mut all_errors = Vec::new();
    let mut all_warnings = Vec::new();

    for comp_def in &file.components {
        let identifiers = extract_identifiers(&comp_def.logic_block);
        analyze_component_def(
            comp_def,
            design_tokens,
            &vocab,
            &custom_components,
            &identifiers,
            &mut all_errors,
            &mut all_warnings,
        );
    }

    (all_errors, all_warnings)
}

// ---------------------------------------------------------------------------
// Recursive walker
// ---------------------------------------------------------------------------

/// Walks all template nodes of a single [`ComponentDef`].
fn analyze_component_def(
    comp_def: &ComponentDef,
    tokens: &DesignTokens,
    vocab: &VocabularyMap,
    custom_components: &HashSet<String>,
    identifiers: &HashSet<String>,
    errors: &mut Vec<AnalyzerError>,
    warnings: &mut Vec<AnalyzerWarning>,
) {
    analyze_nodes(
        &comp_def.template,
        tokens,
        vocab,
        custom_components,
        identifiers,
        errors,
        warnings,
        false,
    );
}

fn analyze_nodes(
    nodes: &[Node],
    tokens: &DesignTokens,
    vocab: &VocabularyMap,
    custom_components: &HashSet<String>,
    identifiers: &HashSet<String>,
    errors: &mut Vec<AnalyzerError>,
    warnings: &mut Vec<AnalyzerWarning>,
    in_unsafe: bool,
) {
    for node in nodes {
        match node {
            Node::Component(c) => analyze_component(c, tokens, vocab, custom_components, identifiers, errors, warnings, in_unsafe),
            Node::Expr(e) => check_expr_value(&e.value, &e.pos, identifiers, in_unsafe, errors),
            Node::If(i) => analyze_if(i, tokens, vocab, custom_components, identifiers, errors, warnings, in_unsafe),
            Node::Each(e) => analyze_each(e, tokens, vocab, custom_components, identifiers, errors, warnings, in_unsafe),
            Node::Unsafe(u) => analyze_unsafe(u, tokens, vocab, custom_components, identifiers, errors, warnings),
            Node::Text(_) => {}
        }
    }
}

fn analyze_component(
    node: &ComponentNode,
    tokens: &DesignTokens,
    vocab: &VocabularyMap,
    custom_components: &HashSet<String>,
    identifiers: &HashSet<String>,
    errors: &mut Vec<AnalyzerError>,
    warnings: &mut Vec<AnalyzerWarning>,
    in_unsafe: bool,
) {
    if vocab.contains(&node.name) {
        // Built-in component: validate props using VocabularyMap
        for prop in &node.props {
            let (prop_errors, prop_warnings) = validate_prop(&node.name, prop, tokens, vocab, identifiers, in_unsafe);
            errors.extend(prop_errors);
            warnings.extend(prop_warnings);
        }
    } else if custom_components.contains(&node.name) {
        // Custom component: recognised, props treated as AnyValue (CLT101/102 suppressed)
        for prop in &node.props {
            if let PropValue::ExpressionValue(ref expr) = prop.value {
                check_expr_value(expr, &prop.pos, identifiers, in_unsafe, errors);
            }
        }
    } else {
        // Unknown component: CLT103
        errors.push(AnalyzerError {
            code: codes::CLT103,
            message: format!("CLT103: unknown component '{}'", node.name),
            pos: node.pos,
        });
    }
    // Event handlers are validated as declared identifiers regardless of component type.
    for ev in &node.events {
        if let Some(err) = check_reference(&ev.handler, &ev.pos, identifiers) {
            errors.push(err);
        }
    }
    analyze_nodes(&node.children, tokens, vocab, custom_components, identifiers, errors, warnings, in_unsafe);
}

fn validate_prop(
    component: &str,
    prop: &PropNode,
    tokens: &DesignTokens,
    vocab: &VocabularyMap,
    identifiers: &HashSet<String>,
    in_unsafe: bool,
) -> (Vec<AnalyzerError>, Vec<AnalyzerWarning>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if let PropValue::UnsafeValue { value, reason } = &prop.value {
        if reason.is_empty() {
            errors.push(AnalyzerError {
                code: codes::CLT106,
                message: format!(
                    "CLT106: unsafe value '{}' for prop '{}' on '{}' is missing a reason. \
                     Use unsafe('{}', 'your reason here')",
                    value, prop.name, component, value
                ),
                pos: prop.pos,
            });
        } else {
            warnings.push(AnalyzerWarning {
                code: codes::W002,
                message: format!(
                    "WARN: unsafe value '{}' used for prop '{}' on '{}' — reason: {}",
                    value, prop.name, component, reason
                ),
                pos: prop.pos,
            });
        }
        return (errors, warnings);
    }

    match vocab.prop(component, &prop.name) {
        None => {
            errors.push(AnalyzerError {
                code: codes::CLT101,
                message: format!("CLT101: unknown prop '{}' on '{}'", prop.name, component),
                pos: prop.pos,
            });
        }
        Some(PropValidation::AnyValue) => {
            if let PropValue::ExpressionValue(ref expr) = prop.value {
                check_expr_value(expr, &prop.pos, identifiers, in_unsafe, &mut errors);
            }
        }
        Some(PropValidation::Tokens(cat)) => match &prop.value {
            PropValue::StringValue(val) => {
                let valid = tokens.valid_values(*cat);
                if !valid.contains(val) {
                    errors.push(AnalyzerError {
                        code: codes::CLT102,
                        message: format!(
                            "CLT102: invalid value '{}' for prop '{}' on '{}'. Valid values: {}",
                            val, prop.name, component, valid.join(", ")
                        ),
                        pos: prop.pos,
                    });
                }
            }
            PropValue::ExpressionValue(expr) => {
                check_expr_value(expr, &prop.pos, identifiers, in_unsafe, &mut errors);
            }
            PropValue::UnsafeValue { .. } => unreachable!("handled above"),
        },
        Some(PropValidation::Enum(vals)) => match &prop.value {
            PropValue::StringValue(val) => {
                if !vals.contains(&val.as_str()) {
                    errors.push(AnalyzerError {
                        code: codes::CLT102,
                        message: format!(
                            "CLT102: invalid value '{}' for prop '{}' on '{}'. Valid values: {}",
                            val, prop.name, component, vals.join(", ")
                        ),
                        pos: prop.pos,
                    });
                }
            }
            PropValue::ExpressionValue(expr) => {
                check_expr_value(expr, &prop.pos, identifiers, in_unsafe, &mut errors);
            }
            PropValue::UnsafeValue { .. } => unreachable!("handled above"),
        },
    }
    (errors, warnings)
}

fn analyze_if(
    node: &IfNode,
    tokens: &DesignTokens,
    vocab: &VocabularyMap,
    custom_components: &HashSet<String>,
    identifiers: &HashSet<String>,
    errors: &mut Vec<AnalyzerError>,
    warnings: &mut Vec<AnalyzerWarning>,
    in_unsafe: bool,
) {
    if let Some(err) = check_reference(&node.condition, &node.pos, identifiers) {
        errors.push(err);
    }
    analyze_nodes(&node.then_children, tokens, vocab, custom_components, identifiers, errors, warnings, in_unsafe);
    if let Some(else_children) = &node.else_children {
        analyze_nodes(else_children, tokens, vocab, custom_components, identifiers, errors, warnings, in_unsafe);
    }
}

fn analyze_each(
    node: &EachNode,
    tokens: &DesignTokens,
    vocab: &VocabularyMap,
    custom_components: &HashSet<String>,
    identifiers: &HashSet<String>,
    errors: &mut Vec<AnalyzerError>,
    warnings: &mut Vec<AnalyzerWarning>,
    in_unsafe: bool,
) {
    if let Some(err) = check_reference(&node.collection, &node.pos, identifiers) {
        errors.push(err);
    }
    let mut child_ids = identifiers.clone();
    child_ids.insert(node.alias.clone());
    if let Some(ref idx) = node.index_alias {
        child_ids.insert(idx.clone());
    }
    analyze_nodes(&node.children, tokens, vocab, custom_components, &child_ids, errors, warnings, in_unsafe);
}

fn analyze_unsafe(
    node: &UnsafeNode,
    tokens: &DesignTokens,
    vocab: &VocabularyMap,
    custom_components: &HashSet<String>,
    identifiers: &HashSet<String>,
    errors: &mut Vec<AnalyzerError>,
    warnings: &mut Vec<AnalyzerWarning>,
) {
    if node.reason.is_empty() {
        errors.push(AnalyzerError {
            code: codes::CLT105,
            message: "CLT105: <unsafe> block is missing a non-empty `reason` attribute. \
                      Use <unsafe reason=\"your reason here\">"
                .to_string(),
            pos: node.pos,
        });
    } else {
        warnings.push(AnalyzerWarning {
            code: codes::W001,
            message: format!("WARN: <unsafe> block used — reason: {}", node.reason),
            pos: node.pos,
        });
        analyze_nodes(&node.children, tokens, vocab, custom_components, identifiers, errors, warnings, true);
    }
}

// ---------------------------------------------------------------------------
// Expression helpers
// ---------------------------------------------------------------------------

/// Validates an `ExpressionValue` in a prop (or `Node::Expr` in the template).
///
/// - Complex expression outside `<unsafe>` → CLT107 error.
/// - Simple identifier outside `<unsafe>` → CLT104 check (must be declared).
/// - Anything inside `<unsafe>` → only CLT104 check for simple identifiers;
///   complex expressions are silently allowed (opaque to the analyzer).
fn check_expr_value(
    expr: &str,
    pos: &Position,
    identifiers: &HashSet<String>,
    in_unsafe: bool,
    errors: &mut Vec<AnalyzerError>,
) {
    if is_simple_identifier(expr) {
        if let Some(err) = check_reference(expr, pos, identifiers) {
            errors.push(err);
        }
    } else if is_member_access(expr) {
        // Member access: validate only the base identifier (e.g. `rule` in `rule.field`).
        let base = expr.split('.').next().unwrap_or(expr);
        if let Some(err) = check_reference(base, pos, identifiers) {
            errors.push(err);
        }
    } else if !in_unsafe {
        errors.push(AnalyzerError {
            code: codes::CLT107,
            message: format!(
                "CLT107: complex expression '{}' is not allowed in the template. \
                 Move the logic to the logic block or wrap in <unsafe reason=\"...\">",
                expr
            ),
            pos: pos.clone(),
        });
    }
    // Complex expression inside <unsafe>: allowed without any check.
}

/// Returns `true` if `s` is a bare identifier: only ASCII letters, digits, and `_`,
/// starting with a letter or `_`. This is the only expression form allowed in the
/// template outside an `<unsafe>` block (CLT107).
fn is_simple_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

/// Returns `true` if `s` is a dotted member access expression of the form
/// `identifier.identifier[.identifier]*` — no function calls, operators, or brackets.
///
/// This is the only complex expression form allowed in the template outside `<unsafe>`.
/// The base identifier (before the first `.`) is validated against declared identifiers;
/// the suffix is passed through opaquely.
fn is_member_access(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    parts.iter().all(|part| is_simple_identifier(part))
}

/// Checks that `name` is present in the set of declared identifiers.
///
/// Returns `None` if the reference is valid, or `Some(AnalyzerError)` with error
/// code CLT104 otherwise.
fn check_reference(
    name: &str,
    pos: &Position,
    identifiers: &HashSet<String>,
) -> Option<AnalyzerError> {
    if identifiers.contains(name) {
        None
    } else {
        Some(AnalyzerError {
            code: codes::CLT104,
            message: format!("CLT104: undeclared identifier '{}'", name),
            pos: pos.clone(),
        })
    }
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
fn extract_identifiers(logic_block: &str) -> HashSet<String> {
    let mut ids = HashSet::new();
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
