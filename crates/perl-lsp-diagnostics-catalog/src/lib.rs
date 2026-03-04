//! LSP-facing diagnostic metadata catalog.
//!
//! This crate converts canonical diagnostic codes into JSON payloads used by
//! LSP `Diagnostic` structures (`code` and `codeDescription`).

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_diagnostics_codes::DiagnosticCode;
use serde_json::{Value, json};

/// Diagnostic metadata compatible with LSP payload fields.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticMeta {
    /// Stable diagnostic code (e.g. `"PL001"`).
    pub code: Value,
    /// Optional code description payload for LSP.
    pub desc: Option<Value>,
}

impl DiagnosticMeta {
    fn from_code(code: DiagnosticCode) -> Self {
        Self {
            code: json!(code.as_str()),
            desc: code.documentation_url().map(|url| json!({ "href": url })),
        }
    }
}

/// General parse error diagnostic metadata (PL001).
pub fn parse_error() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::ParseError)
}

/// Syntax error diagnostic metadata (PL002).
pub fn syntax_error() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::SyntaxError)
}

/// Unexpected end-of-file diagnostic metadata (PL003).
pub fn unexpected_eof() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::UnexpectedEof)
}

/// Missing `use strict` diagnostic metadata (PL100).
pub fn missing_strict() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::MissingStrict)
}

/// Missing `use warnings` diagnostic metadata (PL101).
pub fn missing_warnings() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::MissingWarnings)
}

/// Unused variable diagnostic metadata (PL102).
pub fn unused_var() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::UnusedVariable)
}

/// Undefined variable diagnostic metadata (PL103).
pub fn undefined_var() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::UndefinedVariable)
}

/// Missing package declaration diagnostic metadata (PL200).
pub fn missing_package_declaration() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::MissingPackageDeclaration)
}

/// Duplicate package declaration diagnostic metadata (PL201).
pub fn duplicate_package() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::DuplicatePackage)
}

/// Duplicate subroutine definition diagnostic metadata (PL300).
pub fn duplicate_sub() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::DuplicateSubroutine)
}

/// Missing explicit return diagnostic metadata (PL301).
pub fn missing_return() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::MissingReturn)
}

/// Bareword filehandle diagnostic metadata (PL400).
pub fn bareword_filehandle() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::BarewordFilehandle)
}

/// Two-argument `open` diagnostic metadata (PL401).
pub fn two_arg_open() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::TwoArgOpen)
}

/// Implicit return diagnostic metadata (PL402).
pub fn implicit_return() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::ImplicitReturn)
}

/// Perl::Critic severity 5 diagnostic metadata (PC005).
pub fn critic_severity_5() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::CriticSeverity5)
}

/// Perl::Critic severity 4 diagnostic metadata (PC004).
pub fn critic_severity_4() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::CriticSeverity4)
}

/// Perl::Critic severity 3 diagnostic metadata (PC003).
pub fn critic_severity_3() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::CriticSeverity3)
}

/// Perl::Critic severity 2 diagnostic metadata (PC002).
pub fn critic_severity_2() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::CriticSeverity2)
}

/// Perl::Critic severity 1 diagnostic metadata (PC001).
pub fn critic_severity_1() -> DiagnosticMeta {
    DiagnosticMeta::from_code(DiagnosticCode::CriticSeverity1)
}

/// Infer diagnostic metadata from a message payload.
pub fn from_message(msg: &str) -> Option<DiagnosticMeta> {
    DiagnosticCode::from_message(msg).map(DiagnosticMeta::from_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_docs_url_for_perl_lsp_codes() {
        let meta = parse_error();
        assert_eq!(meta.code, json!("PL001"));
        assert_eq!(meta.desc, Some(json!({ "href": "https://docs.perl-lsp.org/errors/PL001" })));
    }

    #[test]
    fn critic_codes_do_not_include_doc_url() {
        let meta = critic_severity_3();
        assert_eq!(meta.code, json!("PC003"));
        assert_eq!(meta.desc, None);
    }

    #[test]
    fn maps_message_to_metadata_case_insensitive() {
        let maybe_meta = from_message("Undefined symbol in package");
        assert_eq!(maybe_meta.map(|meta| meta.code), Some(json!("PL103")));
    }
}
