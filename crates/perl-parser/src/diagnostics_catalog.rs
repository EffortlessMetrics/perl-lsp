//! Diagnostic catalog with stable codes for consistent error reporting
//!
//! This module provides a centralized catalog of all diagnostics with
//! stable codes that can be referenced in documentation.

use serde_json::{Value, json};

/// Diagnostic metadata
pub struct DiagnosticMeta {
    /// Stable diagnostic code (e.g., "PL001")
    pub code: Value,
    /// Optional code description for LSP
    pub desc: Option<Value>,
}

impl DiagnosticMeta {
    fn new(code: &str, url: Option<&str>) -> Self {
        Self { code: json!(code), desc: url.map(|u| json!({ "href": u })) }
    }
}

// Perl Parser diagnostics (PL000-PL099)
pub fn parse_error() -> DiagnosticMeta {
    DiagnosticMeta::new("PL001", Some("https://docs.perl-lsp.org/errors/PL001"))
}

pub fn syntax_error() -> DiagnosticMeta {
    DiagnosticMeta::new("PL002", Some("https://docs.perl-lsp.org/errors/PL002"))
}

pub fn unexpected_eof() -> DiagnosticMeta {
    DiagnosticMeta::new("PL003", Some("https://docs.perl-lsp.org/errors/PL003"))
}

// Strict/warnings diagnostics (PL100-PL199)
pub fn missing_strict() -> DiagnosticMeta {
    DiagnosticMeta::new("PL100", Some("https://docs.perl-lsp.org/errors/PL100"))
}

pub fn missing_warnings() -> DiagnosticMeta {
    DiagnosticMeta::new("PL101", Some("https://docs.perl-lsp.org/errors/PL101"))
}

pub fn unused_var() -> DiagnosticMeta {
    DiagnosticMeta::new("PL102", Some("https://docs.perl-lsp.org/errors/PL102"))
}

pub fn undefined_var() -> DiagnosticMeta {
    DiagnosticMeta::new("PL103", Some("https://docs.perl-lsp.org/errors/PL103"))
}

// Package/module diagnostics (PL200-PL299)
pub fn missing_package_declaration() -> DiagnosticMeta {
    DiagnosticMeta::new("PL200", Some("https://docs.perl-lsp.org/errors/PL200"))
}

pub fn duplicate_package() -> DiagnosticMeta {
    DiagnosticMeta::new("PL201", Some("https://docs.perl-lsp.org/errors/PL201"))
}

// Subroutine diagnostics (PL300-PL399)
pub fn duplicate_sub() -> DiagnosticMeta {
    DiagnosticMeta::new("PL300", Some("https://docs.perl-lsp.org/errors/PL300"))
}

pub fn missing_return() -> DiagnosticMeta {
    DiagnosticMeta::new("PL301", Some("https://docs.perl-lsp.org/errors/PL301"))
}

// Best practices (PL400-PL499)
pub fn bareword_filehandle() -> DiagnosticMeta {
    DiagnosticMeta::new("PL400", Some("https://docs.perl-lsp.org/errors/PL400"))
}

pub fn two_arg_open() -> DiagnosticMeta {
    DiagnosticMeta::new("PL401", Some("https://docs.perl-lsp.org/errors/PL401"))
}

pub fn implicit_return() -> DiagnosticMeta {
    DiagnosticMeta::new("PL402", Some("https://docs.perl-lsp.org/errors/PL402"))
}

// Perl::Critic violations (PC000-PC999)
pub fn critic_severity_5() -> DiagnosticMeta {
    DiagnosticMeta::new("PC005", None)
}

pub fn critic_severity_4() -> DiagnosticMeta {
    DiagnosticMeta::new("PC004", None)
}

pub fn critic_severity_3() -> DiagnosticMeta {
    DiagnosticMeta::new("PC003", None)
}

pub fn critic_severity_2() -> DiagnosticMeta {
    DiagnosticMeta::new("PC002", None)
}

pub fn critic_severity_1() -> DiagnosticMeta {
    DiagnosticMeta::new("PC001", None)
}

/// Get diagnostic metadata by message pattern
pub fn from_message(msg: &str) -> Option<DiagnosticMeta> {
    if msg.contains("use strict") {
        Some(missing_strict())
    } else if msg.contains("use warnings") {
        Some(missing_warnings())
    } else if msg.contains("unused variable") || msg.contains("never used") {
        Some(unused_var())
    } else if msg.contains("undefined") || msg.contains("not declared") {
        Some(undefined_var())
    } else if msg.contains("bareword filehandle") {
        Some(bareword_filehandle())
    } else if msg.contains("two-argument") || msg.contains("2-arg") {
        Some(two_arg_open())
    } else if msg.contains("parse error") || msg.contains("syntax error") {
        Some(parse_error())
    } else {
        None
    }
}
