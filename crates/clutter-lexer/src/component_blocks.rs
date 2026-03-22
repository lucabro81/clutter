//! Component block discovery: scanning a `.clutter` source for
//! `component Name(…) { … }` blocks.

use clutter_runtime::Position;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single `component Name(props) { … }` block extracted from the source.
pub(super) struct ComponentBlock {
    /// Component name (e.g. `"MainComponent"`).
    pub(super) name: String,
    /// Raw props signature between `(` and `)` (e.g. `"props: MainProps"`).
    pub(super) props_raw: String,
    /// The raw `component Name(…) {` line, stored as the token value.
    pub(super) header_raw: String,
    /// Source position of the `component` keyword (1-based line, col 1).
    pub(super) open_pos: Position,
    /// Everything between the opening `{` line and the closing `}` line,
    /// joined with newlines. Includes the `----` separator line.
    pub(super) body: String,
    /// Absolute 1-based line number of the first line of the body.
    pub(super) body_start_line: usize,
    /// Source position of the closing `}` (1-based line, col 1).
    pub(super) close_pos: Position,
}

/// State machine for accumulating an in-progress component block.
struct ActiveComponent {
    name: String,
    props_raw: String,
    header_raw: String,
    open_pos: Position,
    body_lines: Vec<String>,
    body_start_line: usize,
    /// True once `----` has been seen; after this, `}` terminates the block.
    seen_separator: bool,
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Scans `input` line by line and returns all complete `component` blocks found.
///
/// A block starts with a line matching `component Name(…) {` and ends with a
/// line whose trimmed content is `}` — but only after the `----` separator has
/// been seen (so `}` in TypeScript logic does not close the block prematurely).
pub(super) fn find_components(input: &str) -> Vec<ComponentBlock> {
    let mut result = Vec::new();
    let mut active: Option<ActiveComponent> = None;
    let mut current_line = 1usize;

    for line in input.lines() {
        if let Some(ref mut ac) = active {
            if !ac.seen_separator && line.trim() == "----" {
                ac.seen_separator = true;
                ac.body_lines.push(line.to_string());
            } else if ac.seen_separator && line.trim() == "}" {
                let body = ac.body_lines.join("\n");
                result.push(ComponentBlock {
                    name: ac.name.clone(),
                    props_raw: ac.props_raw.clone(),
                    header_raw: ac.header_raw.clone(),
                    open_pos: ac.open_pos,
                    body,
                    body_start_line: ac.body_start_line,
                    close_pos: Position { line: current_line, col: 1 },
                });
                active = None;
            } else {
                ac.body_lines.push(line.to_string());
            }
        } else if let Some((name, props_raw)) = parse_component_header(line) {
            active = Some(ActiveComponent {
                name,
                props_raw,
                header_raw: line.to_string(),
                open_pos: Position { line: current_line, col: 1 },
                body_lines: Vec::new(),
                body_start_line: current_line + 1,
                seen_separator: false,
            });
        }
        current_line += 1;
    }

    result
}

/// Finds the `----` separator inside a component body string.
///
/// `start_line` is the absolute 1-based line number of the first line of `body`,
/// used to compute the absolute line number of the separator.
///
/// Returns `Some((logic_content, sep_line, template_str))` where:
/// - `logic_content`: raw text before `----`, trailing newlines stripped.
/// - `sep_line`: absolute 1-based line number of the `----` line.
/// - `template_str`: the string slice starting immediately after the `\n`
///   that follows `----` (the template content).
///
/// Returns `None` if `----` does not appear as a standalone line in `body`.
pub(super) fn find_section_separator<'a>(
    body: &'a str,
    start_line: usize,
) -> Option<(&'a str, usize, &'a str)> {
    let mut line_start = 0usize;
    let mut current_line = start_line;

    loop {
        match body[line_start..].find('\n') {
            None => {
                let line = &body[line_start..];
                if line == "----" {
                    let logic = body[..line_start].trim_end_matches('\n');
                    return Some((logic, current_line, ""));
                }
                return None;
            }
            Some(offset) => {
                let line_end = line_start + offset;
                let line = &body[line_start..line_end];
                if line == "----" {
                    let logic = body[..line_start].trim_end_matches('\n');
                    let template_start = line_end + 1;
                    return Some((logic, current_line, &body[template_start..]));
                }
                current_line += 1;
                line_start = line_end + 1;
                if line_start > body.len() {
                    return None;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Attempts to parse a `component Name(props) {` header line.
///
/// Returns `Some((name, props_raw))` on success, `None` otherwise.
/// The `props_raw` is the verbatim content between `(` and the last `)`.
fn parse_component_header(line: &str) -> Option<(String, String)> {
    let rest = line.trim().strip_prefix("component ")?;
    let paren_open = rest.find('(')?;
    let name = rest[..paren_open].trim().to_string();
    if name.is_empty() {
        return None;
    }
    let after_name = &rest[paren_open + 1..];
    let paren_close = after_name.rfind(')')?;
    let props_raw = after_name[..paren_close].to_string();
    let after_close = after_name[paren_close + 1..].trim();
    if after_close != "{" {
        return None;
    }
    Some((name, props_raw))
}
