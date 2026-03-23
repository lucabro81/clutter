use crate::css::generate_css;
use clutter_runtime::DesignTokens;

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

// ---------------------------------------------------------------------------
// CSS — base component classes
// ---------------------------------------------------------------------------

#[test]
fn css_column_base_class() {
    let css = generate_css(&test_tokens());
    assert!(css.contains(".clutter-column { display: flex; flex-direction: column; }"), "{css}");
}

#[test]
fn css_row_base_class() {
    let css = generate_css(&test_tokens());
    assert!(css.contains(".clutter-row { display: flex; flex-direction: row; }"), "{css}");
}

#[test]
fn css_box_base_class() {
    let css = generate_css(&test_tokens());
    assert!(css.contains(".clutter-box { box-sizing: border-box; }"), "{css}");
}

#[test]
fn css_text_base_class() {
    let css = generate_css(&test_tokens());
    assert!(css.contains(".clutter-text {"), "{css}");
}

#[test]
fn css_button_base_class() {
    let css = generate_css(&test_tokens());
    assert!(css.contains(".clutter-button { cursor: pointer; }"), "{css}");
}

#[test]
fn css_input_base_class() {
    let css = generate_css(&test_tokens());
    assert!(css.contains(".clutter-input {"), "{css}");
}

// ---------------------------------------------------------------------------
// CSS — spacing token classes (gap, padding, margin)
// ---------------------------------------------------------------------------

#[test]
fn css_gap_classes_per_spacing_token() {
    let css = generate_css(&test_tokens());
    for val in ["xs", "sm", "md", "lg", "xl", "xxl"] {
        let expected = format!(".clutter-gap-{val} {{ gap: var(--spacing-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

#[test]
fn css_padding_classes_per_spacing_token() {
    let css = generate_css(&test_tokens());
    for val in ["xs", "sm", "md", "lg", "xl", "xxl"] {
        let expected = format!(".clutter-padding-{val} {{ padding: var(--spacing-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

#[test]
fn css_margin_classes_per_spacing_token() {
    let css = generate_css(&test_tokens());
    for val in ["xs", "sm", "md", "lg", "xl", "xxl"] {
        let expected = format!(".clutter-margin-{val} {{ margin: var(--spacing-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

// ---------------------------------------------------------------------------
// CSS — color token classes (bg, color)
// ---------------------------------------------------------------------------

#[test]
fn css_bg_classes_per_color_token() {
    let css = generate_css(&test_tokens());
    for val in ["primary", "secondary", "danger", "surface", "background"] {
        let expected = format!(".clutter-bg-{val} {{ background-color: var(--color-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

#[test]
fn css_color_classes_per_color_token() {
    let css = generate_css(&test_tokens());
    for val in ["primary", "secondary", "danger", "surface", "background"] {
        let expected = format!(".clutter-color-{val} {{ color: var(--color-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

// ---------------------------------------------------------------------------
// CSS — typography token classes (size, weight)
// ---------------------------------------------------------------------------

#[test]
fn css_size_classes_per_font_size_token() {
    let css = generate_css(&test_tokens());
    for val in ["xs", "sm", "base", "lg", "xl", "xxl"] {
        let expected = format!(".clutter-size-{val} {{ font-size: var(--size-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

#[test]
fn css_weight_classes_per_font_weight_token() {
    let css = generate_css(&test_tokens());
    for val in ["normal", "medium", "semibold", "bold"] {
        let expected = format!(".clutter-weight-{val} {{ font-weight: var(--weight-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

// ---------------------------------------------------------------------------
// CSS — radius + shadow token classes
// ---------------------------------------------------------------------------

#[test]
fn css_radius_classes_per_radius_token() {
    let css = generate_css(&test_tokens());
    for val in ["none", "sm", "md", "lg", "full"] {
        let expected = format!(".clutter-radius-{val} {{ border-radius: var(--radius-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}

#[test]
fn css_shadow_classes_per_shadow_token() {
    let css = generate_css(&test_tokens());
    for val in ["sm", "md", "lg"] {
        let expected = format!(".clutter-shadow-{val} {{ box-shadow: var(--shadow-{val}); }}");
        assert!(css.contains(&expected), "missing {expected}\n{css}");
    }
}
