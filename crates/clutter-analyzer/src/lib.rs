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
}
