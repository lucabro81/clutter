//! Machine-readable error and warning codes for the Clutter compiler.
//!
//! Every diagnostic type carries a `code: &'static str` field pointing to one
//! of the constants defined here. Using the constant (rather than a string
//! literal) lets tests assert `error.code == codes::CLT102` instead of
//! checking a fragile substring of the human-readable message.
//!
//! # Code ranges
//!
//! | Range        | Stage    | Description                       |
//! |--------------|----------|-----------------------------------|
//! | `L001–L002`  | Lexer    | Tokenisation errors               |
//! | `P001–P003`  | Parser   | Structural / grammar errors       |
//! | `CLT101–107` | Analyzer | Semantic / design-system errors   |
//! | `W001–W002`  | Analyzer | Non-blocking unsafe-usage warnings|

// ---------------------------------------------------------------------------
// Lexer codes
// ---------------------------------------------------------------------------

/// Missing `---` separator between the logic block and the template.
pub const L001: &str = "L001";

/// Unexpected / unrecognised character in the template or inside a tag.
pub const L002: &str = "L002";

// ---------------------------------------------------------------------------
// Parser codes
// ---------------------------------------------------------------------------

/// Structural mismatch: expected token X, found Y (e.g. missing prop value,
/// unexpected tag nesting).
pub const P001: &str = "P001";

/// `<else>` without a matching `<if>`.
pub const P002: &str = "P002";

/// `<unsafe>` tag is missing the `reason` attribute or its value is empty.
pub const P003: &str = "P003";

// ---------------------------------------------------------------------------
// Analyzer codes
// ---------------------------------------------------------------------------

/// Unknown component name — not in the closed vocabulary.
pub const CLT101: &str = "CLT101";

/// Invalid prop value for a token-checked prop — value not in `tokens.json`.
pub const CLT102: &str = "CLT102";

/// Unknown prop name for a known component.
pub const CLT103: &str = "CLT103";

/// Expression references an identifier not declared in the logic block.
pub const CLT104: &str = "CLT104";

/// `<unsafe>` block with a missing or empty `reason` attribute.
pub const CLT105: &str = "CLT105";

/// `unsafe('value', 'reason')` prop value with an empty reason string.
pub const CLT106: &str = "CLT106";

/// Complex `{}` expression used in the template outside an `<unsafe>` block.
pub const CLT107: &str = "CLT107";

// ---------------------------------------------------------------------------
// Analyzer warning codes
// ---------------------------------------------------------------------------

/// Well-formed `<unsafe reason="...">` block — compilation proceeds, but the
/// escape hatch is flagged for visibility.
pub const W001: &str = "W001";

/// Well-formed `unsafe('value', 'reason')` prop value — compilation proceeds,
/// but the bypass is flagged for visibility.
pub const W002: &str = "W002";
