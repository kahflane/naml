//!
//! Diagnostic Module - Rich Error Reporting
//!
//! This module provides error reporting with source context using miette.
//! Errors display line numbers, column positions, and source code snippets.
//!
//! Usage:
//!   let reporter = DiagnosticReporter::new(&source_file);
//!   reporter.report_parse_errors(&errors);
//!   reporter.report_type_errors(&errors);
//!

use miette::{Diagnostic, LabeledSpan, NamedSource, Report, SourceSpan};
use thiserror::Error;

use crate::parser::ParseError;
use crate::source::SourceFile;
use crate::typechecker::TypeError;

#[derive(Debug, Error)]
#[error("{message}")]
pub struct NamlDiagnostic {
    message: String,
    src: NamedSource<String>,
    span: SourceSpan,
    label: String,
    help_text: Option<String>,
}

impl Diagnostic for NamlDiagnostic {
    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.src)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        Some(Box::new(std::iter::once(LabeledSpan::new_primary_with_span(
            Some(self.label.clone()),
            self.span,
        ))))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.help_text
            .as_ref()
            .map(|h| Box::new(h.clone()) as Box<dyn std::fmt::Display>)
    }
}

impl NamlDiagnostic {
    pub fn from_parse_error(err: &ParseError, source: &SourceFile) -> Self {
        let span = err.span;
        let (line, col) = source.line_col(span.start);

        Self {
            message: format!("parse error at {}:{}", line, col),
            src: NamedSource::new(&source.name, source.source.to_string()),
            span: (span.start as usize, (span.end - span.start) as usize).into(),
            label: err.message.clone(),
            help_text: None,
        }
    }

    pub fn from_type_error(err: &TypeError, source: &SourceFile) -> Self {
        let span = err.span();
        let (line, col) = source.line_col(span.start);
        let (message, label, help) = type_error_details(err);

        Self {
            message: format!("{} at {}:{}", message, line, col),
            src: NamedSource::new(&source.name, source.source.to_string()),
            span: (span.start as usize, (span.end - span.start) as usize).into(),
            label,
            help_text: help,
        }
    }
}

fn type_error_details(err: &TypeError) -> (String, String, Option<String>) {
    match err {
        TypeError::TypeMismatch { expected, found, .. } => (
            format!("type mismatch: expected {}, found {}", expected, found),
            format!("expected {}", expected),
            Some(format!("change this to type {}", expected)),
        ),
        TypeError::UndefinedVariable { name, .. } => (
            format!("undefined variable '{}'", name),
            "not found in this scope".to_string(),
            Some("check spelling or declare the variable".to_string()),
        ),
        TypeError::UndefinedType { name, .. } => (
            format!("undefined type '{}'", name),
            "unknown type".to_string(),
            Some("check spelling or import the type".to_string()),
        ),
        TypeError::UndefinedFunction { name, .. } => (
            format!("undefined function '{}'", name),
            "function not found".to_string(),
            Some("check spelling or define the function".to_string()),
        ),
        TypeError::UndefinedField { ty, field, .. } => (
            format!("type '{}' has no field '{}'", ty, field),
            format!("no field '{}'", field),
            None,
        ),
        TypeError::UndefinedMethod { ty, method, .. } => (
            format!("type '{}' has no method '{}'", ty, method),
            format!("no method '{}'", method),
            None,
        ),
        TypeError::DuplicateDefinition { name, .. } => (
            format!("duplicate definition of '{}'", name),
            "already defined".to_string(),
            Some("rename or remove one of the definitions".to_string()),
        ),
        TypeError::InvalidOperation { op, ty, .. } => (
            format!("invalid operation '{}' on type '{}'", op, ty),
            format!("cannot use '{}' here", op),
            None,
        ),
        TypeError::InvalidBinaryOp { op, left, right, .. } => (
            format!("cannot apply '{}' to {} and {}", op, left, right),
            format!("invalid operands for '{}'", op),
            None,
        ),
        TypeError::InferenceFailed { .. } => (
            "could not infer type".to_string(),
            "type unknown".to_string(),
            Some("add a type annotation".to_string()),
        ),
        TypeError::WrongArgCount { expected, found, .. } => (
            format!("expected {} arguments, found {}", expected, found),
            format!("expected {} args", expected),
            None,
        ),
        TypeError::WrongTypeArgCount { expected, found, .. } => (
            format!("expected {} type arguments, found {}", expected, found),
            format!("expected {} type args", expected),
            None,
        ),
        TypeError::NotCallable { ty, .. } => (
            format!("type '{}' is not callable", ty),
            "not a function".to_string(),
            None,
        ),
        TypeError::NotIndexable { ty, .. } => (
            format!("type '{}' cannot be indexed", ty),
            "not indexable".to_string(),
            None,
        ),
        TypeError::NotIterable { ty, .. } => (
            format!("type '{}' is not iterable", ty),
            "not iterable".to_string(),
            None,
        ),
        TypeError::ImmutableAssignment { name, .. } => (
            format!("cannot assign to immutable variable '{}'", name),
            "immutable".to_string(),
            Some("use 'var' instead of 'const' to make mutable".to_string()),
        ),
        TypeError::MissingReturn { .. } => (
            "function must return a value".to_string(),
            "missing return".to_string(),
            Some("add a return statement".to_string()),
        ),
        TypeError::UnreachableCode { .. } => (
            "unreachable code".to_string(),
            "never executed".to_string(),
            Some("remove unreachable code".to_string()),
        ),
        TypeError::BreakOutsideLoop { .. } => (
            "break outside of loop".to_string(),
            "not inside a loop".to_string(),
            Some("break can only be used inside loops".to_string()),
        ),
        TypeError::ContinueOutsideLoop { .. } => (
            "continue outside of loop".to_string(),
            "not inside a loop".to_string(),
            Some("continue can only be used inside loops".to_string()),
        ),
        TypeError::PlatformMismatch { feature, platform, .. } => (
            format!("feature '{}' not available on '{}'", feature, platform),
            format!("not on {}", platform),
            None,
        ),
        TypeError::BoundNotSatisfied { ty, bound, .. } => (
            format!("type '{}' does not satisfy bound '{}'", ty, bound),
            format!("does not implement {}", bound),
            Some(format!("implement {} for {}", bound, ty)),
        ),
        TypeError::NoBoundForMethod { param, method, .. } => (
            format!(
                "no bound provides method '{}' for type parameter '{}'",
                method, param
            ),
            format!("method '{}' not found", method),
            Some(format!(
                "add a bound like '{}: SomeTrait' that provides '{}'",
                param, method
            )),
        ),
        TypeError::Custom { message, .. } => (
            message.clone(),
            "error".to_string(),
            None,
        ),
        TypeError::MissingInterfaceMethod { struct_name, interface_name, method_name, .. } => (
            format!(
                "struct '{}' is missing method '{}' required by interface '{}'",
                struct_name, method_name, interface_name
            ),
            format!("missing method '{}'", method_name),
            Some(format!("implement '{}' for struct '{}'", method_name, struct_name)),
        ),
        TypeError::UnknownModule { path, .. } => (
            format!("unknown module '{}'", path),
            "module not found".to_string(),
            Some("check the module path".to_string()),
        ),
        TypeError::UnknownModuleSymbol { module, symbol, .. } => (
            format!("unknown symbol '{}' in module '{}'", symbol, module),
            format!("'{}' not found", symbol),
            Some(format!("check available exports in '{}'", module)),
        ),
        TypeError::PrivateSymbol { module, symbol, .. } => (
            format!("symbol '{}' in module '{}' is not public", symbol, module),
            "not public".to_string(),
            Some("mark with 'pub' to export".to_string()),
        ),
        TypeError::ModuleFileError { path, reason, .. } => (
            format!("cannot read module '{}': {}", path, reason),
            "module file error".to_string(),
            Some("check that the file exists and is valid".to_string()),
        ),
    }
}

pub struct DiagnosticReporter<'a> {
    source: &'a SourceFile,
}

impl<'a> DiagnosticReporter<'a> {
    pub fn new(source: &'a SourceFile) -> Self {
        Self { source }
    }

    pub fn report_parse_error(&self, err: &ParseError) {
        let diag = NamlDiagnostic::from_parse_error(err, self.source);
        let report = Report::new(diag);
        eprintln!("{:?}", report);
    }

    pub fn report_type_error(&self, err: &TypeError) {
        let diag = NamlDiagnostic::from_type_error(err, self.source);
        let report = Report::new(diag);
        eprintln!("{:?}", report);
    }

    pub fn report_parse_errors(&self, errors: &[ParseError]) {
        for err in errors {
            self.report_parse_error(err);
        }
    }

    pub fn report_type_errors(&self, errors: &[TypeError]) {
        for err in errors {
            self.report_type_error(err);
        }
    }

    pub fn has_errors(parse_errors: &[ParseError], type_errors: &[TypeError]) -> bool {
        !parse_errors.is_empty() || !type_errors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Span;

    #[test]
    fn test_diagnostic_from_parse_error() {
        let source = SourceFile::new("test.naml", "fn main() { }");
        let err = ParseError {
            message: "unexpected token".to_string(),
            span: Span::new(3, 7, 0),
        };

        let diag = NamlDiagnostic::from_parse_error(&err, &source);
        assert!(diag.message.contains("1:4"));
    }

    #[test]
    fn test_diagnostic_from_type_error() {
        let source = SourceFile::new("test.naml", "var x: int = true;");
        let err = TypeError::TypeMismatch {
            expected: "int".to_string(),
            found: "bool".to_string(),
            span: Span::new(13, 17, 0),
        };

        let diag = NamlDiagnostic::from_type_error(&err, &source);
        assert!(diag.message.contains("type mismatch"));
        assert!(diag.help_text.is_some());
    }
}
