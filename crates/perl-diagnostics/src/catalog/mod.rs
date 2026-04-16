//! Diagnostic metadata catalog.
//!
//! Functions to build and work with diagnostic metadata.

use std::fmt;

/// Diagnostic metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticMeta {
    /// The diagnostic code as a string.
    pub code: String,
    /// The severity level.
    pub severity: String,
    /// The message.
    pub message: String,
}

impl fmt::Display for DiagnosticMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

/// Build diagnostic metadata from a code.
pub fn diagnostic_meta(_code: crate::codes::DiagnosticCode) -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Parse error diagnostic.
pub fn parse_error(_msg: &str) -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Syntax error diagnostic.
pub fn syntax_error(_msg: &str) -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Unexpected EOF diagnostic.
pub fn unexpected_eof() -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Missing strict diagnostic.
pub fn missing_strict() -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Missing warnings diagnostic.
pub fn missing_warnings() -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Unused variable diagnostic.
pub fn unused_var(_name: &str) -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Undefined variable diagnostic.
pub fn undefined_var(_name: &str) -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Missing package declaration diagnostic.
pub fn missing_package_declaration() -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Duplicate package diagnostic.
pub fn duplicate_package(_name: &str) -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Duplicate subroutine diagnostic.
pub fn duplicate_sub(_name: &str) -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Missing return diagnostic.
pub fn missing_return() -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Bareword filehandle diagnostic.
pub fn bareword_filehandle(_name: &str) -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Two-argument open diagnostic.
pub fn two_arg_open() -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Implicit return diagnostic.
pub fn implicit_return() -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Eval error flow diagnostic.
pub fn eval_error_flow() -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Perl::Critic severity 5 diagnostic.
pub fn critic_severity_5(_rule: &str) -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Perl::Critic severity 4 diagnostic.
pub fn critic_severity_4(_rule: &str) -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Perl::Critic severity 3 diagnostic.
pub fn critic_severity_3(_rule: &str) -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Perl::Critic severity 2 diagnostic.
pub fn critic_severity_2(_rule: &str) -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Perl::Critic severity 1 diagnostic.
pub fn critic_severity_1(_rule: &str) -> DiagnosticMeta {
    DiagnosticMeta::default()
}

/// Infer a diagnostic code from a message.
pub fn from_message(_msg: &str) -> Option<crate::codes::DiagnosticCode> {
    None
}
