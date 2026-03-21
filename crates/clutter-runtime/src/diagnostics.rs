use crate::position::Position;

/// Common interface for all compiler diagnostics (errors and warnings).
///
/// Implemented by [`LexError`], [`ParseError`], [`AnalyzerError`], and
/// [`AnalyzerWarning`]. Allows diagnostic-agnostic code (e.g. the CLI renderer,
/// error catalogue) to handle any diagnostic through a single trait object
/// `&dyn Diagnostic`.
pub trait Diagnostic {
    /// Machine-readable code (e.g. `"CLT102"`). Stable across releases.
    fn code(&self) -> &'static str;
    /// Human-readable description of the problem.
    fn message(&self) -> &str;
    /// Position in the source where the diagnostic was detected.
    fn pos(&self) -> &Position;
    /// Returns `true` for errors, `false` for warnings.
    fn is_error(&self) -> bool;
}

// ---------------------------------------------------------------------------
// Lexer diagnostics
// ---------------------------------------------------------------------------

/// Error produced by the lexer during tokenisation.
///
/// The lexer does not stop at the first error: it continues scanning and
/// accumulates all errors in a `Vec<LexError>` returned alongside the token stream.
#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    /// Machine-readable error code (e.g. `codes::L001`). Stable across messages.
    pub code: &'static str,
    /// Human-readable description of the problem.
    pub message: String,
    /// Position in the source where the error was detected.
    pub pos: Position,
}

impl Diagnostic for LexError {
    fn code(&self) -> &'static str { self.code }
    fn message(&self) -> &str { &self.message }
    fn pos(&self) -> &Position { &self.pos }
    fn is_error(&self) -> bool { true }
}

// ---------------------------------------------------------------------------
// Parser diagnostics
// ---------------------------------------------------------------------------

/// Error produced by the parser during AST construction.
///
/// The parser does not stop at the first error: it applies a recovery strategy
/// (advances to the next prop boundary or tag boundary) and accumulates all
/// errors in a `Vec<ParseError>`.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// Machine-readable error code (e.g. `codes::P001`). Stable across messages.
    pub code: &'static str,
    /// Human-readable description of the problem.
    pub message: String,
    /// Position in the source where the error was detected.
    pub pos: Position,
}

impl Diagnostic for ParseError {
    fn code(&self) -> &'static str { self.code }
    fn message(&self) -> &str { &self.message }
    fn pos(&self) -> &Position { &self.pos }
    fn is_error(&self) -> bool { true }
}

// ---------------------------------------------------------------------------
// Analyzer diagnostics
// ---------------------------------------------------------------------------

/// Semantic error produced by the analyzer.
///
/// The analyzer collects all semantic errors (CLT101–107) into a
/// `Vec<AnalyzerError>` without stopping at the first. An empty list means
/// the file is valid and can proceed to codegen.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzerError {
    /// Machine-readable error code (e.g. `codes::CLT102`). Stable across messages.
    pub code: &'static str,
    /// Human-readable description of the problem, prefixed with the error code.
    pub message: String,
    /// Position in the source where the error was detected.
    pub pos: Position,
}

impl Diagnostic for AnalyzerError {
    fn code(&self) -> &'static str { self.code }
    fn message(&self) -> &str { &self.message }
    fn pos(&self) -> &Position { &self.pos }
    fn is_error(&self) -> bool { true }
}

/// Warning produced by the analyzer for intentional but non-standard usage.
///
/// A warning does not block compilation — the file proceeds to codegen.
/// Warnings are emitted for well-formed unsafe constructs (`<unsafe reason="...">`,
/// `unsafe('val', 'reason')`), which are valid but bypass design-system rules.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzerWarning {
    /// Machine-readable warning code (e.g. `codes::W001`). Stable across messages.
    pub code: &'static str,
    /// Human-readable description.
    pub message: String,
    /// Position in the source where the warning was detected.
    pub pos: Position,
}

impl Diagnostic for AnalyzerWarning {
    fn code(&self) -> &'static str { self.code }
    fn message(&self) -> &str { &self.message }
    fn pos(&self) -> &Position { &self.pos }
    fn is_error(&self) -> bool { false }
}
