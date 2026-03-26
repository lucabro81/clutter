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

use clutter_runtime::FileNode;

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
///
/// The returned SFCs contain no `<style>` section. Design-system CSS should
/// be generated separately with [`generate_css`] and written as `clutter.css`.
///
/// # Examples
///
/// ```
/// let src = "component Foo(props: FooProps) {\n----\n<Column />\n}";
/// let (tok, _) = clutter_lexer::tokenize(src);
/// let (file, _) = clutter_parser::Parser::new(tok).parse_file();
/// let sfcs = clutter_codegen::generate_vue(&file);
/// assert_eq!(sfcs.len(), 1);
/// assert_eq!(sfcs[0].name, "Foo");
/// ```
pub fn generate_vue(file: &FileNode) -> Vec<GeneratedFile> {
    file.components
        .iter()
        .map(|comp| GeneratedFile {
            name: comp.name.clone(),
            content: vue::generate_sfc(comp),
        })
        .collect()
}

/// Re-exports the global CSS generator so callers do not need to reach into the
/// internal `css` submodule.
pub use css::generate_css;
