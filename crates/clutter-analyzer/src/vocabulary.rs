//! Built-in component vocabulary: schema definitions and prop validation rules.

use std::collections::HashMap;

use clutter_runtime::TokenCategory;

// ---------------------------------------------------------------------------
// PropValidation
// ---------------------------------------------------------------------------

/// Validation rule applicable to a prop in the closed vocabulary.
///
/// Returned by [`VocabularyMap::prop`]: `None` means the prop is not recognised
/// on the given component (→ CLT101).
pub(super) enum PropValidation {
    /// The value must be present in a design system token category.
    Tokens(TokenCategory),
    /// The value must be one of the elements in the fixed set provided.
    Enum(&'static [&'static str]),
    /// The prop is valid with any string value; if it is an expression, the
    /// identifier name is still subject to the CLT104 check.
    AnyValue,
}

// ---------------------------------------------------------------------------
// ComponentSchema + VocabularyMap
// ---------------------------------------------------------------------------

/// Schema for one built-in component: its set of recognised props.
pub(super) struct ComponentSchema {
    pub(super) props: HashMap<&'static str, PropValidation>,
}

/// Single source of truth for the built-in component vocabulary.
///
/// Constructed once at the start of [`super::analyze_file`] via
/// [`VocabularyMap::new`].
///
/// # Extension point
///
/// When custom component schemas or file-based vocabulary are needed, the
/// extension point is `VocabularyMap::new()`. The rest of the analyzer is
/// unchanged.
pub(super) struct VocabularyMap {
    components: HashMap<&'static str, ComponentSchema>,
}

impl VocabularyMap {
    /// Constructs the built-in vocabulary.
    pub(super) fn new() -> Self {
        use PropValidation::*;
        use TokenCategory::*;

        const LAYOUT_AXES: &[&str] =
            &["start", "end", "center", "spaceBetween", "spaceAround", "spaceEvenly"];
        const CROSS_AXES: &[&str] = &["start", "end", "center", "stretch"];
        const ALIGNS: &[&str] = &["left", "center", "right"];
        const BTN_VARIANTS: &[&str] = &["primary", "secondary", "outline", "ghost", "danger"];
        const BTN_SIZES: &[&str] = &["sm", "md", "lg"];
        const INPUT_TYPES: &[&str] = &["text", "email", "password", "number"];

        macro_rules! schema {
            ($($prop:expr => $rule:expr),* $(,)?) => {{
                let mut props = HashMap::new();
                $(props.insert($prop, $rule);)*
                ComponentSchema { props }
            }};
        }

        let mut components: HashMap<&'static str, ComponentSchema> = HashMap::new();

        components.insert("Column", schema! {
            "gap"       => Tokens(Spacing),
            "padding"   => Tokens(Spacing),
            "mainAxis"  => Enum(LAYOUT_AXES),
            "crossAxis" => Enum(CROSS_AXES),
        });
        components.insert("Row", schema! {
            "gap"       => Tokens(Spacing),
            "padding"   => Tokens(Spacing),
            "mainAxis"  => Enum(LAYOUT_AXES),
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
    pub(super) fn contains(&self, name: &str) -> bool {
        self.components.contains_key(name)
    }

    /// Returns the validation rule for the `(component, prop)` pair.
    ///
    /// - `Some(rule)` if the prop is recognised on the component.
    /// - `None` if the prop is not in the schema (→ CLT101 for the caller).
    pub(super) fn prop(&self, component: &str, prop: &str) -> Option<&PropValidation> {
        self.components.get(component)?.props.get(prop)
    }
}
