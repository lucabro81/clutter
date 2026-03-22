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
//! let (errors, warnings) = analyze_file(&file, &tokens);
//! if errors.is_empty() {
//!     // proceed to codegen
//! }
//! ```

use std::collections::{HashMap, HashSet};

use clutter_runtime::{
    codes, AnalyzerError, AnalyzerWarning, ComponentDef, ComponentNode, EachNode, FileNode,
    IfNode, Node, Position, PropNode, PropValue, UnsafeNode,
};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// VocabularyMap — single source of truth for the built-in vocabulary
// ---------------------------------------------------------------------------

/// Schema for one built-in component: its set of recognised props.
struct ComponentSchema {
    props: HashMap<&'static str, PropValidation>,
}

/// Single source of truth for the built-in component vocabulary.
///
/// Constructed once at the start of [`analyze_file`] via [`VocabularyMap::new`].
/// Replaces the separate [`KNOWN_COMPONENTS`] slice and [`prop_map`] function.
///
/// # Extension point
///
/// When custom component schemas or file-based vocabulary are needed,
/// the extension point is `VocabularyMap::new()`. The rest of the analyzer
/// is unchanged.
struct VocabularyMap {
    components: HashMap<&'static str, ComponentSchema>,
}

impl VocabularyMap {
    /// Constructs the built-in vocabulary.
    fn new() -> Self {
        use PropValidation::*;
        use TokenCategory::*;

        const LAYOUT_AXES: &[&str] = &["start", "end", "center", "spaceBetween", "spaceAround", "spaceEvenly"];
        const CROSS_AXES:  &[&str] = &["start", "end", "center", "stretch"];
        const ALIGNS:      &[&str] = &["left", "center", "right"];
        const BTN_VARIANTS: &[&str] = &["primary", "secondary", "outline", "ghost", "danger"];
        const BTN_SIZES:    &[&str] = &["sm", "md", "lg"];
        const INPUT_TYPES:  &[&str] = &["text", "email", "password", "number"];

        macro_rules! schema {
            ($($prop:expr => $rule:expr),* $(,)?) => {{
                let mut props = HashMap::new();
                $(props.insert($prop, $rule);)*
                ComponentSchema { props }
            }};
        }

        let mut components: HashMap<&'static str, ComponentSchema> = HashMap::new();

        components.insert("Column", schema! {
            "gap"      => Tokens(Spacing),
            "padding"  => Tokens(Spacing),
            "mainAxis" => Enum(LAYOUT_AXES),
            "crossAxis" => Enum(CROSS_AXES),
        });
        components.insert("Row", schema! {
            "gap"      => Tokens(Spacing),
            "padding"  => Tokens(Spacing),
            "mainAxis" => Enum(LAYOUT_AXES),
            "crossAxis" => Enum(CROSS_AXES),
        });
        components.insert("Text", schema! {
            "value"  => AnyValue,
            "size"   => Tokens(FontSize),
            "weight" => Tokens(FontWeight),
            "color"  => Tokens(Color),
            "align"  => Enum(ALIGNS),
        });
        components.insert("Button", schema! {
            "variant"  => Enum(BTN_VARIANTS),
            "size"     => Enum(BTN_SIZES),
            "disabled" => AnyValue,
        });
        components.insert("Box", schema! {
            "bg"      => Tokens(Color),
            "padding" => Tokens(Spacing),
            "margin"  => Tokens(Spacing),
            "radius"  => Tokens(Radius),
            "shadow"  => Tokens(Shadow),
        });
        components.insert("Input", schema! {
            "placeholder" => AnyValue,
            "value"       => AnyValue,
            "type"        => Enum(INPUT_TYPES),
        });

        VocabularyMap { components }
    }

    /// Returns `true` if `name` is a built-in component in the vocabulary.
    fn contains(&self, name: &str) -> bool {
        self.components.contains_key(name)
    }

    /// Returns the validation rule for the `(component, prop)` pair.
    ///
    /// - `Some(rule)` if the prop is recognised on the component.
    /// - `None` if the prop is not in the schema (→ CLT101 for the caller).
    fn prop(&self, component: &str, prop: &str) -> Option<&PropValidation> {
        self.components.get(component)?.props.get(prop)
    }
}

// ---------------------------------------------------------------------------
// Public entry point — new multi-component API
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
// Public entry point — deprecated single-component API
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Recursive walker
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

/// Checks that `name` is present in the set of declared identifiers.
///
/// Returns `None` if the reference is valid, or `Some(AnalyzerError)` with error
/// code CLT104 otherwise.
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

