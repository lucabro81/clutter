use clutter_runtime::AnalyzerError;
use serde::Deserialize;

#[derive(Debug, Clone, Copy)]
enum TokenCategory {
    Spacing,
    Color,
    FontSize,
    FontWeight,
    Radius,
    Shadow,
}

#[derive(Debug, Deserialize)]
struct Typography {
    sizes: Vec<String>,
    weights: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DesignTokens {
    spacing: Vec<String>,
    colors: Vec<String>,
    typography: Typography,
    radii: Vec<String>,
    shadows: Vec<String>,
}

enum PropValidation {
    Tokens(TokenCategory),
    Enum(&'static [&'static str]),
    AnyValue,
}

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

impl DesignTokens {
    pub fn from_str(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub(crate) fn valid_values(&self, category: TokenCategory) -> &[String] {
        match category {
            TokenCategory::Spacing => &self.spacing,
            TokenCategory::Color => &self.colors,
            TokenCategory::FontSize => &self.typography.sizes,
            TokenCategory::FontWeight => &self.typography.weights,
            TokenCategory::Radius => &self.radii,
            TokenCategory::Shadow => &self.shadows,
        }
    }
}

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
}
