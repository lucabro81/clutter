use std::fs;
use std::path::{Path, PathBuf};

use clutter_runtime::DesignTokens;

/// Walks up from `source`'s parent directory looking for a `tokens.json` file.
///
/// Returns the path of the first `tokens.json` found, or an error if the
/// filesystem root is reached without finding one.
pub fn discover_tokens_json(source: &Path) -> Result<PathBuf, String> {
    let start = source
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let mut dir = start;
    loop {
        let candidate = dir.join("tokens.json");
        if candidate.exists() {
            return Ok(candidate);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => {
                return Err(format!(
                    "tokens.json not found in any ancestor of '{}'",
                    source.display()
                ))
            }
        }
    }
}

/// Loads and parses design tokens from either an explicit path or via auto-discovery.
///
/// If `explicit` is `Some`, that path is used directly.
/// Otherwise, [`discover_tokens_json`] is called starting from `source`.
pub fn load_tokens(explicit: Option<&Path>, source: &Path) -> Result<DesignTokens, String> {
    let path = match explicit {
        Some(p) => p.to_path_buf(),
        None => discover_tokens_json(source)?,
    };

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("cannot read '{}': {}", path.display(), e))?;

    DesignTokens::from_str(&content)
        .map_err(|e| format!("invalid tokens.json at '{}': {}", path.display(), e))
}
