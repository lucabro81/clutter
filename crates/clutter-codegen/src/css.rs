//! CSS generation for the Vue SFC target.
//!
//! Produces a single `<style scoped>` block containing:
//! - One base class per built-in component (structural/behavioural rules).
//! - One utility class per (prop-name × token-value) pair, using CSS custom
//!   properties for design-system values (`gap: var(--spacing-md)`).

use clutter_runtime::DesignTokens;

// ---------------------------------------------------------------------------
// Prop → CSS mapping
// ---------------------------------------------------------------------------

/// Describes how a single prop name maps to a CSS property and token category.
struct PropMapping {
    /// CSS class prefix, e.g. `"gap"` → `.clutter-gap-{val}`.
    prop: &'static str,
    /// CSS property written in the rule body, e.g. `"gap"`.
    css_property: &'static str,
    /// CSS custom-property prefix, e.g. `"--spacing"` → `var(--spacing-{val})`.
    var_prefix: &'static str,
}

const PROP_MAPPINGS: &[PropMapping] = &[
    PropMapping { prop: "gap",     css_property: "gap",              var_prefix: "--spacing" },
    PropMapping { prop: "padding", css_property: "padding",          var_prefix: "--spacing" },
    PropMapping { prop: "margin",  css_property: "margin",           var_prefix: "--spacing" },
    PropMapping { prop: "bg",      css_property: "background-color", var_prefix: "--color"   },
    PropMapping { prop: "color",   css_property: "color",            var_prefix: "--color"   },
    PropMapping { prop: "size",    css_property: "font-size",        var_prefix: "--size"    },
    PropMapping { prop: "weight",  css_property: "font-weight",      var_prefix: "--weight"  },
    PropMapping { prop: "radius",  css_property: "border-radius",    var_prefix: "--radius"  },
    PropMapping { prop: "shadow",  css_property: "box-shadow",       var_prefix: "--shadow"  },
];

fn token_values<'a>(mapping: &PropMapping, tokens: &'a DesignTokens) -> &'a [String] {
    match mapping.prop {
        "gap" | "padding" | "margin" => tokens.spacing(),
        "bg"  | "color"              => tokens.colors(),
        "size"                       => tokens.font_sizes(),
        "weight"                     => tokens.font_weights(),
        "radius"                     => tokens.radii(),
        "shadow"                     => tokens.shadows(),
        _                            => &[],
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Generates the full CSS for inclusion in a Vue SFC `<style scoped>` block.
pub fn generate_css(tokens: &DesignTokens) -> String {
    let mut out = String::new();

    // CSS custom property definitions
    if let Some(vars) = tokens.variables() {
        out.push_str(":root {\n");
        for (name, value) in vars {
            out.push_str(&format!("  {name}: {value};\n"));
        }
        out.push_str("}\n\n");
    }

    // Base component classes
    out.push_str(".clutter-column { display: flex; flex-direction: column; }\n");
    out.push_str(".clutter-row { display: flex; flex-direction: row; }\n");
    out.push_str(".clutter-box { box-sizing: border-box; }\n");
    out.push_str(".clutter-text { }\n");
    out.push_str(".clutter-button { cursor: pointer; }\n");
    out.push_str(".clutter-input { }\n");

    // Token-value utility classes
    for mapping in PROP_MAPPINGS {
        for val in token_values(mapping, tokens) {
            out.push_str(&format!(
                ".clutter-{}-{} {{ {}: var({}-{}); }}\n",
                mapping.prop, val, mapping.css_property, mapping.var_prefix, val
            ));
        }
    }

    out
}
