//! Design token types: categories, raw JSON deserialization, and value lookup.

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Token category
// ---------------------------------------------------------------------------

/// Design token category that a prop value may belong to.
///
/// Used by [`super::vocabulary::PropValidation::Tokens`] to direct the lookup
/// of valid values in [`DesignTokens::valid_values`].
#[derive(Debug, Clone, Copy)]
pub(crate) enum TokenCategory {
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

// ---------------------------------------------------------------------------
// DesignTokens
// ---------------------------------------------------------------------------

/// Internal JSON structure of `tokens.json` for the typography section.
#[derive(Debug, Deserialize)]
struct Typography {
    sizes: Vec<String>,
    weights: Vec<String>,
}

/// Design system deserialised from `tokens.json`.
///
/// Holds the valid values for every token category. Built once at the start of
/// [`super::analyze_file`] and passed read-only throughout the entire tree walk.
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
    /// Used by [`super::validate_prop`] to check the prop value and to build the
    /// CLT102 error message listing accepted values.
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
