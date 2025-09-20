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
/// General parse error diagnostic
///
/// # Returns
///
/// Diagnostic metadata for PL001 parse errors
pub fn parse_error() -> DiagnosticMeta {
    DiagnosticMeta::new("PL001", Some("https://docs.perl-lsp.org/errors/PL001"))
}

/// Syntax error diagnostic
///
/// # Returns
///
/// Diagnostic metadata for PL002 syntax errors
pub fn syntax_error() -> DiagnosticMeta {
    DiagnosticMeta::new("PL002", Some("https://docs.perl-lsp.org/errors/PL002"))
}

/// Unexpected end-of-file diagnostic
///
/// # Returns
///
/// Diagnostic metadata for PL003 unexpected EOF errors
pub fn unexpected_eof() -> DiagnosticMeta {
    DiagnosticMeta::new("PL003", Some("https://docs.perl-lsp.org/errors/PL003"))
}

// Strict/warnings diagnostics (PL100-PL199)
/// Missing 'use strict' pragma diagnostic
///
/// # Returns
///
/// Diagnostic metadata for PL100 missing strict warnings
pub fn missing_strict() -> DiagnosticMeta {
    DiagnosticMeta::new("PL100", Some("https://docs.perl-lsp.org/errors/PL100"))
}

/// Missing 'use warnings' pragma diagnostic
///
/// # Returns
///
/// Diagnostic metadata for PL101 missing warnings
pub fn missing_warnings() -> DiagnosticMeta {
    DiagnosticMeta::new("PL101", Some("https://docs.perl-lsp.org/errors/PL101"))
}

/// Unused variable diagnostic
///
/// # Returns
///
/// Diagnostic metadata for PL102 unused variable warnings
pub fn unused_var() -> DiagnosticMeta {
    DiagnosticMeta::new("PL102", Some("https://docs.perl-lsp.org/errors/PL102"))
}

/// Undefined variable diagnostic
///
/// # Returns
///
/// Diagnostic metadata for PL103 undefined variable errors
pub fn undefined_var() -> DiagnosticMeta {
    DiagnosticMeta::new("PL103", Some("https://docs.perl-lsp.org/errors/PL103"))
}

// Package/module diagnostics (PL200-PL299)

/// Generate diagnostic for missing package declaration in email script modules
///
/// Used within PSTX pipeline when email scripts lack proper package declarations,
/// which can lead to symbol collision issues during large-scale PST processing.
///
/// # Returns
///
/// Diagnostic metadata with error code PL200 for package declaration issues
///
/// # PSTX Integration
///
/// Essential for Extract stage validation of email script module structure
pub fn missing_package_declaration() -> DiagnosticMeta {
    DiagnosticMeta::new("PL200", Some("https://docs.perl-lsp.org/errors/PL200"))
}

/// Generate diagnostic for duplicate package declarations in email scripts
///
/// Detects multiple package declarations within email script content that could
/// cause namespace conflicts during PSTX pipeline processing of complex PST files.
///
/// # Returns
///
/// Diagnostic metadata with error code PL201 for duplicate package issues
pub fn duplicate_package() -> DiagnosticMeta {
    DiagnosticMeta::new("PL201", Some("https://docs.perl-lsp.org/errors/PL201"))
}

// Subroutine diagnostics (PL300-PL399)

/// Generate diagnostic for duplicate subroutine definitions in email scripts
///
/// Identifies redefined subroutines that could cause runtime errors during
/// email processing workflows within the PSTX pipeline Normalize stage.
///
/// # Returns
///
/// Diagnostic metadata with error code PL300 for subroutine redefinition
pub fn duplicate_sub() -> DiagnosticMeta {
    DiagnosticMeta::new("PL300", Some("https://docs.perl-lsp.org/errors/PL300"))
}

/// Generate diagnostic for missing explicit return statements
///
/// Flags subroutines lacking explicit return statements, which can lead to
/// unexpected behavior in email filtering scripts during PST processing.
///
/// # Returns
///
/// Diagnostic metadata with error code PL301 for missing return statements
pub fn missing_return() -> DiagnosticMeta {
    DiagnosticMeta::new("PL301", Some("https://docs.perl-lsp.org/errors/PL301"))
}

// Best practices (PL400-PL499)

/// Generate diagnostic for bareword filehandle usage in email scripts
///
/// Identifies bareword filehandle usage that can cause security vulnerabilities
/// during email processing, particularly important for enterprise PST workflows.
///
/// # Returns
///
/// Diagnostic metadata with error code PL400 for bareword filehandle issues
pub fn bareword_filehandle() -> DiagnosticMeta {
    DiagnosticMeta::new("PL400", Some("https://docs.perl-lsp.org/errors/PL400"))
}

/// Generate diagnostic for two-argument open() calls in email scripts
///
/// Flags potentially unsafe two-argument open() calls that could introduce
/// security risks when processing email attachments within PSTX pipelines.
///
/// # Returns
///
/// Diagnostic metadata with error code PL401 for unsafe open() usage
pub fn two_arg_open() -> DiagnosticMeta {
    DiagnosticMeta::new("PL401", Some("https://docs.perl-lsp.org/errors/PL401"))
}

/// Generate diagnostic for implicit return values in email script functions
///
/// Detects implicit return behavior that could lead to unexpected results
/// during email filtering and processing operations within PSTX workflows.
///
/// # Returns
///
/// Diagnostic metadata with error code PL402 for implicit return issues
pub fn implicit_return() -> DiagnosticMeta {
    DiagnosticMeta::new("PL402", Some("https://docs.perl-lsp.org/errors/PL402"))
}

// Perl::Critic violations (PC000-PC999)

/// Generate diagnostic for Perl::Critic severity level 5 violations
///
/// Maps gentle (severity 5) Perl::Critic policy violations found in email
/// scripts during PSTX pipeline code quality analysis.
///
/// # Returns
///
/// Diagnostic metadata with error code PC005 for gentle policy violations
pub fn critic_severity_5() -> DiagnosticMeta {
    DiagnosticMeta::new("PC005", None)
}

/// Generate diagnostic for Perl::Critic severity level 4 violations
///
/// Maps stern (severity 4) Perl::Critic policy violations in email scripts
/// that indicate code quality issues requiring attention.
///
/// # Returns
///
/// Diagnostic metadata with error code PC004 for stern policy violations
pub fn critic_severity_4() -> DiagnosticMeta {
    DiagnosticMeta::new("PC004", None)
}

/// Generate diagnostic for Perl::Critic severity level 3 violations
///
/// Maps harsh (severity 3) Perl::Critic policy violations that represent
/// significant code quality issues in email processing scripts.
///
/// # Returns
///
/// Diagnostic metadata with error code PC003 for harsh policy violations
pub fn critic_severity_3() -> DiagnosticMeta {
    DiagnosticMeta::new("PC003", None)
}

/// Generate diagnostic for Perl::Critic severity level 2 violations
///
/// Maps stern (severity 2) Perl::Critic policy violations indicating serious
/// code quality problems that could affect email processing reliability.
///
/// # Returns
///
/// Diagnostic metadata with error code PC002 for stern policy violations
pub fn critic_severity_2() -> DiagnosticMeta {
    DiagnosticMeta::new("PC002", None)
}

/// Generate diagnostic for Perl::Critic severity level 1 violations
///
/// Maps brutal (severity 1) Perl::Critic policy violations representing
/// critical code quality issues that must be addressed in email scripts.
///
/// # Returns
///
/// Diagnostic metadata with error code PC001 for brutal policy violations
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
