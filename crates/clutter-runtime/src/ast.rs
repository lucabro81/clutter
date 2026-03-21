use crate::position::Position;

/// Value of a component prop.
///
/// A prop can have a string literal value — to be validated against the design
/// system — or a TypeScript expression — evaluated at runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    /// String literal: `gap="md"`. Must be present in the design system.
    StringValue(String),
    /// TypeScript expression: `gap={myVar}`. The identifier name is checked
    /// by the analyzer against bindings declared in the logic block.
    ExpressionValue(String),
    /// Explicit unsafe bypass: `gap="unsafe('16px', 'not in the design yet')"`.
    /// The `value` is passed through without token validation; `reason` must be
    /// non-empty or the analyzer emits CLT106.
    UnsafeValue { value: String, reason: String },
}

/// A single `name=value` prop on a component.
#[derive(Debug, Clone, PartialEq)]
pub struct PropNode {
    /// Prop name (e.g. `"gap"`, `"size"`).
    pub name: String,
    /// Prop value (string or expression).
    pub value: PropValue,
    /// Position in the source (first character of the name).
    pub pos: Position,
}

/// A component from the closed vocabulary (e.g. `<Column>`, `<Text />`).
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentNode {
    /// Component name (e.g. `"Column"`, `"Text"`).
    pub name: String,
    /// Props declared on the opening tag.
    pub props: Vec<PropNode>,
    /// Children: present only if the tag is not self-closing.
    pub children: Vec<Node>,
    /// Position of the opening tag in the source.
    pub pos: Position,
}

/// Static text between tags (not an interpolation, not structural whitespace).
#[derive(Debug, Clone, PartialEq)]
pub struct TextNode {
    /// The raw text.
    pub value: String,
    /// Position in the source.
    pub pos: Position,
}

/// Interpolation of a TypeScript expression in the template: `{expr}`.
///
/// The expression name is checked by the analyzer (CLT104) against bindings
/// declared in the logic block.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionNode {
    /// Name of the interpolated identifier (e.g. `"title"`, `"count"`).
    pub value: String,
    /// Position in the source.
    pub pos: Position,
}

/// Conditional node `<if condition={expr}>…</if>` with an optional else branch.
#[derive(Debug, Clone, PartialEq)]
pub struct IfNode {
    /// The condition expression (identifier name).
    pub condition: String,
    /// Children of the `then` branch (between `<if>` and `<else>` or `</if>`).
    pub then_children: Vec<Node>,
    /// Children of the `else` branch, present only if the `<else>` tag is declared.
    pub else_children: Option<Vec<Node>>,
    /// Position of the `<if>` tag in the source.
    pub pos: Position,
}

/// Unsafe escape-hatch block `<unsafe reason="...">…</unsafe>`.
///
/// Permits complex `{}` expressions inside the template (CLT107 is suppressed
/// within this block). Requires a non-empty `reason`; an empty reason causes
/// the analyzer to emit CLT105.
#[derive(Debug, Clone, PartialEq)]
pub struct UnsafeNode {
    /// The mandatory justification for bypassing design-system rules.
    pub reason: String,
    /// Children of the unsafe block (may include complex expressions).
    pub children: Vec<Node>,
    /// Position of the `<unsafe>` tag in the source.
    pub pos: Position,
}

/// Iteration node `<each collection={expr} as="alias">…</each>`.
#[derive(Debug, Clone, PartialEq)]
pub struct EachNode {
    /// The collection expression (identifier name).
    pub collection: String,
    /// The alias assigned to the current element (local binding for children).
    pub alias: String,
    /// Children of the loop body.
    pub children: Vec<Node>,
    /// Position of the `<each>` tag in the source.
    pub pos: Position,
}

/// A template node: the union of all possible node types.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Closed-vocabulary component (e.g. `<Column>`, `<Text />`).
    Component(ComponentNode),
    /// Static text.
    Text(TextNode),
    /// Expression interpolation `{expr}`.
    Expr(ExpressionNode),
    /// Conditional `<if>`.
    If(IfNode),
    /// Iteration `<each>`.
    Each(EachNode),
    /// Unsafe escape-hatch block `<unsafe>`.
    Unsafe(UnsafeNode),
}

/// The root of the AST produced by the parser.
///
/// Corresponds to an entire `.clutter` file. The file structure is:
///
/// ```text
/// [TypeScript logic block — opaque to the compiler]
/// ---
/// [template — AST nodes]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramNode {
    /// Raw content of the TypeScript logic block (before `---`).
    /// May be empty if the file starts directly with `---`.
    pub logic_block: String,
    /// Top-level nodes of the template (after `---`).
    pub template: Vec<Node>,
}
