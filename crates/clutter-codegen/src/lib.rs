//! Code generator for the Clutter compiler — Vue SFC target.
//!
//! Receives a validated [`FileNode`] and a [`DesignTokens`] instance and
//! produces one Vue SFC (`.vue` file) per [`ComponentDef`].
//!
//! # Entry point
//!
//! ```ignore
//! let files = generate_vue(&file_node, &design_tokens);
//! // files: Vec<GeneratedFile>  — one entry per component
//! ```

use clutter_runtime::{DesignTokens, FileNode};

pub mod css;
pub mod vue;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// A single generated output file produced by the code generator.
pub struct GeneratedFile {
    /// File name without extension (e.g. `"MainComponent"` → `MainComponent.vue`).
    pub name: String,
    /// Full file content ready to be written to disk.
    pub content: String,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Generates one Vue SFC per [`ComponentDef`] in the given [`FileNode`].
pub fn generate_vue(file: &FileNode, tokens: &DesignTokens) -> Vec<GeneratedFile> {
    file.components
        .iter()
        .map(|comp| GeneratedFile {
            name: comp.name.clone(),
            content: vue::generate_sfc(comp, tokens),
        })
        .collect()
}
