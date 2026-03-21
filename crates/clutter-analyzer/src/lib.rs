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
//! let errors = analyze(&program, &tokens);
//! if errors.is_empty() {
//!     // proceed to codegen
//! }
//! ```

use std::collections::HashSet;

use clutter_runtime::{
    codes, AnalyzerError, AnalyzerWarning, ComponentNode, EachNode, IfNode, Node, Position,
    PropNode, PropValue, ProgramNode, UnsafeNode,
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
/// Returns `(errors, warnings)`. An empty `errors` vec means the file is valid
/// and can proceed to codegen. `warnings` lists well-formed unsafe constructs
/// that were deliberately used to bypass design-system rules.
pub fn analyze(
    program: &ProgramNode,
    tokens: &DesignTokens,
) -> (Vec<AnalyzerError>, Vec<AnalyzerWarning>) {
    let identifiers = extract_identifiers(&program.logic_block);
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    analyze_nodes(&program.template, tokens, &identifiers, &mut errors, &mut warnings, false);
    (errors, warnings)
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
    warnings: &mut Vec<AnalyzerWarning>,
    in_unsafe: bool,
) {
    for node in nodes {
        match node {
            Node::Component(c) => analyze_component(c, tokens, identifiers, errors, warnings, in_unsafe),
            Node::Expr(e) => {
                check_expr_value(&e.value, &e.pos, identifiers, in_unsafe, errors);
            }
            Node::If(i) => analyze_if(i, tokens, identifiers, errors, warnings, in_unsafe),
            Node::Each(e) => analyze_each(e, tokens, identifiers, errors, warnings, in_unsafe),
            Node::Unsafe(u) => analyze_unsafe(u, tokens, identifiers, errors, warnings),
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
    warnings: &mut Vec<AnalyzerWarning>,
    in_unsafe: bool,
) {
    if !KNOWN_COMPONENTS.contains(&node.name.as_str()) {
        errors.push(AnalyzerError {
            code: codes::CLT103,
            message: format!("CLT103: unknown component '{}'", node.name),
            pos: node.pos.clone(),
        });
        // Still recurse into children even for unknown components
    } else {
        for prop in &node.props {
            let (prop_errors, prop_warnings) = validate_prop(&node.name, prop, tokens, identifiers, in_unsafe);
            errors.extend(prop_errors);
            warnings.extend(prop_warnings);
        }
    }
    analyze_nodes(&node.children, tokens, identifiers, errors, warnings, in_unsafe);
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
    in_unsafe: bool,
) -> (Vec<AnalyzerError>, Vec<AnalyzerWarning>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // UnsafeValue bypasses all token/enum validation regardless of the prop_map result.
    if let PropValue::UnsafeValue { value, reason } = &prop.value {
        if reason.is_empty() {
            errors.push(AnalyzerError {
                code: codes::CLT106,
                message: format!(
                    "CLT106: unsafe value '{}' for prop '{}' on '{}' is missing a reason. \
                     Use unsafe('{}', 'your reason here')",
                    value, prop.name, component, value
                ),
                pos: prop.pos.clone(),
            });
        } else {
            warnings.push(AnalyzerWarning {
                code: codes::W002,
                message: format!(
                    "WARN: unsafe value '{}' used for prop '{}' on '{}' — reason: {}",
                    value, prop.name, component, reason
                ),
                pos: prop.pos.clone(),
            });
        }
        return (errors, warnings);
    }

    match prop_map(component, &prop.name) {
        None => {
            errors.push(AnalyzerError {
                code: codes::CLT101,
                message: format!(
                    "CLT101: unknown prop '{}' on '{}'",
                    prop.name, component
                ),
                pos: prop.pos.clone(),
            });
        }
        Some(PropValidation::AnyValue) => {
            if let PropValue::ExpressionValue(ref expr) = prop.value {
                check_expr_value(expr, &prop.pos, identifiers, in_unsafe, &mut errors);
            }
        }
        Some(PropValidation::Tokens(cat)) => match &prop.value {
            PropValue::StringValue(val) => {
                let valid = tokens.valid_values(cat);
                if !valid.contains(val) {
                    errors.push(AnalyzerError {
                        code: codes::CLT102,
                        message: format!(
                            "CLT102: invalid value '{}' for prop '{}' on '{}'. Valid values: {}",
                            val, prop.name, component, valid.join(", ")
                        ),
                        pos: prop.pos.clone(),
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
                        pos: prop.pos.clone(),
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
    warnings: &mut Vec<AnalyzerWarning>,
    in_unsafe: bool,
) {
    if let Some(err) = check_reference(&node.condition, &node.pos, identifiers) {
        errors.push(err);
    }
    analyze_nodes(&node.then_children, tokens, identifiers, errors, warnings, in_unsafe);
    if let Some(else_children) = &node.else_children {
        analyze_nodes(else_children, tokens, identifiers, errors, warnings, in_unsafe);
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
    warnings: &mut Vec<AnalyzerWarning>,
    in_unsafe: bool,
) {
    if let Some(err) = check_reference(&node.collection, &node.pos, identifiers) {
        errors.push(err);
    }
    // The alias is in scope for children only — clone to avoid polluting the outer scope.
    let mut child_ids = identifiers.clone();
    child_ids.insert(node.alias.clone());
    analyze_nodes(&node.children, tokens, &child_ids, errors, warnings, in_unsafe);
}

/// Validates an `<unsafe>` escape-hatch block.
///
/// - Empty `reason` → CLT105 error; children are **not** recursed (the block is malformed).
/// - Non-empty `reason` → [`AnalyzerWarning`]; children are analysed with `in_unsafe = true`
///   (CLT107 is suppressed, but CLT104 still fires for simple undeclared identifiers).
fn analyze_unsafe(
    node: &UnsafeNode,
    tokens: &DesignTokens,
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
            pos: node.pos.clone(),
        });
    } else {
        warnings.push(AnalyzerWarning {
            code: codes::W001,
            message: format!("WARN: <unsafe> block used — reason: {}", node.reason),
            pos: node.pos.clone(),
        });
        analyze_nodes(&node.children, tokens, identifiers, errors, warnings, true);
    }
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
            code: codes::CLT104,
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
mod tests;

