use super::*;
use diagnostics::{Diagnostic, DiagnosticCollector};

fn pos() -> Position {
    Position { line: 1, col: 1 }
}

// --- Diagnostic trait: LexError ---

#[test]
fn lex_error_diagnostic_code() {
    let e = LexError { code: "L001", message: "missing separator".into(), pos: pos() };
    assert_eq!(e.code(), "L001");
}

#[test]
fn lex_error_diagnostic_message() {
    let e = LexError { code: "L001", message: "missing separator".into(), pos: pos() };
    assert_eq!(e.message(), "missing separator");
}

#[test]
fn lex_error_diagnostic_pos() {
    let e = LexError { code: "L001", message: "missing separator".into(), pos: pos() };
    assert_eq!(e.pos(), &pos());
}

#[test]
fn lex_error_is_error() {
    let e = LexError { code: "L001", message: "msg".into(), pos: pos() };
    assert!(e.is_error());
}

// --- Diagnostic trait: ParseError ---

#[test]
fn parse_error_diagnostic_code() {
    let e = ParseError { code: "P001", message: "unexpected token".into(), pos: pos() };
    assert_eq!(e.code(), "P001");
}

#[test]
fn parse_error_is_error() {
    let e = ParseError { code: "P001", message: "msg".into(), pos: pos() };
    assert!(e.is_error());
}

// --- Diagnostic trait: AnalyzerError ---

#[test]
fn analyzer_error_diagnostic_code() {
    let e = AnalyzerError { code: "CLT102", message: "invalid value".into(), pos: pos() };
    assert_eq!(e.code(), "CLT102");
}

#[test]
fn analyzer_error_is_error() {
    let e = AnalyzerError { code: "CLT102", message: "msg".into(), pos: pos() };
    assert!(e.is_error());
}

// --- Diagnostic trait: AnalyzerWarning ---

#[test]
fn analyzer_warning_diagnostic_code() {
    let w = AnalyzerWarning { code: "W001", message: "unsafe block used".into(), pos: pos() };
    assert_eq!(w.code(), "W001");
}

#[test]
fn analyzer_warning_is_not_error() {
    let w = AnalyzerWarning { code: "W001", message: "msg".into(), pos: pos() };
    assert!(!w.is_error());
}

// --- Diagnostic as dyn trait ---

#[test]
fn diagnostic_trait_object_works() {
    let diagnostics: Vec<Box<dyn Diagnostic>> = vec![
        Box::new(LexError { code: "L001", message: "a".into(), pos: pos() }),
        Box::new(ParseError { code: "P001", message: "b".into(), pos: pos() }),
        Box::new(AnalyzerError { code: "CLT101", message: "c".into(), pos: pos() }),
        Box::new(AnalyzerWarning { code: "W001", message: "d".into(), pos: pos() }),
    ];
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    assert_eq!(errors.len(), 3);
}

// --- DiagnosticCollector ---

#[test]
fn collector_starts_empty() {
    let c: DiagnosticCollector<LexError> = DiagnosticCollector::new();
    assert!(c.into_vec().is_empty());
}

#[test]
fn collector_emit_accumulates() {
    let mut c = DiagnosticCollector::new();
    c.emit(LexError { code: "L001", message: "a".into(), pos: pos() });
    c.emit(LexError { code: "L002", message: "b".into(), pos: pos() });
    assert_eq!(c.into_vec().len(), 2);
}

#[test]
fn collector_into_vec_preserves_order() {
    let mut c = DiagnosticCollector::new();
    c.emit(LexError { code: "L001", message: "first".into(), pos: pos() });
    c.emit(LexError { code: "L002", message: "second".into(), pos: pos() });
    let v = c.into_vec();
    assert_eq!(v[0].message, "first");
    assert_eq!(v[1].message, "second");
}
