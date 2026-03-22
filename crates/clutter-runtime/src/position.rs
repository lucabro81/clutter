/// Position of a token or AST node in the `.clutter` source file.
///
/// Points to the start of the token (first character). Lines and columns are
/// 1-indexed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    /// Line number (1-based).
    pub line: usize,
    /// Column number (1-based).
    pub col: usize,
}
