//! Design token types shared across the compiler pipeline.
//!
//! [`DesignTokens`] is deserialised from `tokens.json` once at the start of
//! compilation and passed read-only to the analyzer (for prop validation) and
//! to the codegen (for CSS class generation).

use serde::Deserialize;

// ---------------------------------------------------------------------------
// TokenCategory
// ---------------------------------------------------------------------------

/// Design token category that a prop value may belong to.
///
/// Used by the analyzer's prop validation and by the codegen's CSS generator
/// to look up valid values in [`DesignTokens`].
#[derive(Debug, Clone, Copy)]
pub enum TokenCategory {
    /// Spacing: gap, padding, margin.
    Spacing,
    /// Semantic colours.
    Color,
    /// Typography sizes.
    FontSize,
    /// Typography weights.
    FontWeight,
    /// Border radii.
    Radius,
    /// Shadows.
    Shadow,
}

// ---------------------------------------------------------------------------
// DesignTokens
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct Typography {
    sizes: Vec<String>,
    weights: Vec<String>,
}

/// Design system deserialised from `tokens.json`.
///
/// Holds the valid values for every token category. Built once at the start of
/// compilation and passed read-only to the analyzer and the codegen.
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
    pub fn from_str(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Returns the valid values for the given token category.
    ///
    /// Used by the analyzer for prop validation.
    pub fn valid_values(&self, category: TokenCategory) -> &[String] {
        match category {
            TokenCategory::Spacing    => &self.spacing,
            TokenCategory::Color      => &self.colors,
            TokenCategory::FontSize   => &self.typography.sizes,
            TokenCategory::FontWeight => &self.typography.weights,
            TokenCategory::Radius     => &self.radii,
            TokenCategory::Shadow     => &self.shadows,
        }
    }

    /// Returns the spacing token values.
    pub fn spacing(&self) -> &[String] { &self.spacing }

    /// Returns the color token values.
    pub fn colors(&self) -> &[String] { &self.colors }

    /// Returns the font-size token values.
    pub fn font_sizes(&self) -> &[String] { &self.typography.sizes }

    /// Returns the font-weight token values.
    pub fn font_weights(&self) -> &[String] { &self.typography.weights }

    /// Returns the border-radius token values.
    pub fn radii(&self) -> &[String] { &self.radii }

    /// Returns the shadow token values.
    pub fn shadows(&self) -> &[String] { &self.shadows }
}
