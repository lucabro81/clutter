//! Shared types for the entire Clutter compiler pipeline.
//!
//! This crate is the common dependency for all others (`clutter-lexer`,
//! `clutter-parser`, `clutter-analyzer`, `clutter-codegen`). It defines the
//! data structures exchanged between pipeline stages and the [`Diagnostic`]
//! trait shared by all error and warning types.
//!
//! # Modules
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`codes`] | Machine-readable error/warning code constants |
//! | [`position`] | [`Position`] — source location |
//! | [`tokens`] | [`Token`], [`TokenKind`] — lexer output |
//! | [`ast`] | AST node types — parser output |
//! | [`diagnostics`] | [`Diagnostic`] trait + [`LexError`], [`ParseError`], [`AnalyzerError`], [`AnalyzerWarning`] |

pub mod codes;
pub mod design_tokens;
pub mod position;
pub mod tokens;
pub mod ast;
pub mod diagnostics;

pub use design_tokens::{DesignTokens, TokenCategory};
pub use position::Position;
pub use tokens::{Token, TokenKind};
pub use ast::{
    ComponentDef, ComponentNode, EachNode, EventBinding, ExpressionNode, FileNode, IfNode,
    Node, PropNode, PropValue, TextNode, UnsafeNode,
};
pub use diagnostics::{
    AnalyzerError, AnalyzerWarning, Diagnostic, DiagnosticCollector, LexError, ParseError,
};

#[cfg(test)]
mod tests;
