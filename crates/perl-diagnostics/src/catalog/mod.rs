//! Diagnostic metadata catalog.
//!
//! Functions to build LSP-facing metadata payloads from stable diagnostic codes.
//! This module provides a focused mapping from [`crate::codes::DiagnosticCode`]
//! to LSP-facing metadata.

use crate::codes::DiagnosticCode;
use serde_json::{Value, json};

/// Diagnostic metadata payload used by LSP diagnostics.
pub struct DiagnosticMeta {
    /// Stable diagnostic code (for example, `"PL001"`).
    pub code: Value,
    /// Optional code description object containing a docs URL.
    pub desc: Option<Value>,
    /// Optional human-readable context hint explaining what the diagnostic
    /// means and how to resolve it.  `None` for codes (e.g. Perl::Critic)
    /// whose per-policy descriptions already serve this purpose.
    pub hint: Option<&'static str>,
}

impl Default for DiagnosticMeta {
    fn default() -> Self {
        Self { code: json!("PL001"), desc: None, hint: None }
    }
}

impl DiagnosticMeta {
    fn from_code(code: DiagnosticCode) -> Self {
        Self {
            code: json!(code.as_str()),
            desc: code.documentation_url().map(|url| json!({ "href": url })),
            hint: code.context_hint(),
        }
    }
}

/// Build LSP diagnostic metadata from a stable diagnostic code.
#[must_use]
pub fn diagnostic_meta(code: DiagnosticCode) -> DiagnosticMeta {
    DiagnosticMeta::from_code(code)
}

/// General parse error diagnostic (PL001).
#[must_use]
pub fn parse_error() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::ParseError)
}

/// Syntax error diagnostic (PL002).
#[must_use]
pub fn syntax_error() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::SyntaxError)
}

/// Unexpected end-of-file diagnostic (PL003).
#[must_use]
pub fn unexpected_eof() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::UnexpectedEof)
}

/// Missing `use strict` pragma diagnostic (PL100).
#[must_use]
pub fn missing_strict() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::MissingStrict)
}

/// Missing `use warnings` pragma diagnostic (PL101).
#[must_use]
pub fn missing_warnings() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::MissingWarnings)
}

/// Unused variable diagnostic (PL102).
#[must_use]
pub fn unused_var() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::UnusedVariable)
}

/// Undefined variable diagnostic (PL103).
#[must_use]
pub fn undefined_var() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::UndefinedVariable)
}

/// Missing package declaration diagnostic (PL200).
#[must_use]
pub fn missing_package_declaration() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::MissingPackageDeclaration)
}

/// Duplicate package declaration diagnostic (PL201).
#[must_use]
pub fn duplicate_package() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::DuplicatePackage)
}

/// Duplicate subroutine definition diagnostic (PL300).
#[must_use]
pub fn duplicate_sub() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::DuplicateSubroutine)
}

/// Missing explicit return statement diagnostic (PL301).
#[must_use]
pub fn missing_return() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::MissingReturn)
}

/// Bareword filehandle usage diagnostic (PL400).
#[must_use]
pub fn bareword_filehandle() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::BarewordFilehandle)
}

/// Two-argument `open()` usage diagnostic (PL401).
#[must_use]
pub fn two_arg_open() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::TwoArgOpen)
}

/// Implicit return value diagnostic (PL402).
#[must_use]
pub fn implicit_return() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::ImplicitReturn)
}

/// Eval / try error-flow diagnostic (PL407).
#[must_use]
pub fn eval_error_flow() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::EvalErrorFlow)
}

/// Perl::Critic severity-5 violation diagnostic (PC005).
#[must_use]
pub fn critic_severity_5() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::CriticSeverity5)
}

/// Perl::Critic severity-4 violation diagnostic (PC004).
#[must_use]
pub fn critic_severity_4() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::CriticSeverity4)
}

/// Perl::Critic severity-3 violation diagnostic (PC003).
#[must_use]
pub fn critic_severity_3() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::CriticSeverity3)
}

/// Perl::Critic severity-2 violation diagnostic (PC002).
#[must_use]
pub fn critic_severity_2() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::CriticSeverity2)
}

/// Perl::Critic severity-1 violation diagnostic (PC001).
#[must_use]
pub fn critic_severity_1() -> DiagnosticMeta {
    diagnostic_meta(DiagnosticCode::CriticSeverity1)
}

/// Guess diagnostic metadata from a free-form message.
#[must_use]
pub fn from_message(msg: &str) -> Option<DiagnosticMeta> {
    DiagnosticCode::from_message(msg).map(diagnostic_meta)
}

#[cfg(test)]
mod tests {
    use super::{eval_error_flow, from_message, parse_error};

    #[test]
    fn parse_error_includes_stable_code_and_docs_url() {
        let meta = parse_error();
        assert_eq!(meta.code, "PL001");
        assert_eq!(
            meta.desc,
            Some(serde_json::json!({ "href": "https://docs.perl-lsp.org/errors/PL001" }))
        );
    }

    #[test]
    fn critic_codes_have_no_docs_url() {
        let meta = super::critic_severity_1();
        assert_eq!(meta.code, "PC001");
        assert!(meta.desc.is_none());
    }

    #[test]
    fn eval_error_flow_has_stable_code_and_docs_url() {
        let meta = eval_error_flow();
        assert_eq!(meta.code, "PL407");
        assert_eq!(
            meta.desc,
            Some(serde_json::json!({ "href": "https://docs.perl-lsp.org/errors/PL407" }))
        );
    }

    #[test]
    fn message_inference_is_case_insensitive() {
        let meta = from_message("Missing USE STRICT pragma");
        assert!(meta.is_some());
        assert_eq!(meta.as_ref().map(|m| &m.code), Some(&serde_json::json!("PL100")));
    }
}
