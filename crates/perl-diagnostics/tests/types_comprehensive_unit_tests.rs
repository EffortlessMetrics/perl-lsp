//! Comprehensive unit tests for `perl_diagnostics::types` module.
//!
//! Tests cover the Diagnostic and RelatedInformation structs as defined
//! in the Wave E consolidated crate. DiagnosticSeverity and DiagnosticTag
//! are re-exported from codes:: (type-unified) and tested in type_unification.rs.

use perl_diagnostics::types::{Diagnostic, DiagnosticSeverity, DiagnosticTag, RelatedInformation};

// ---------------------------------------------------------------------------
// DiagnosticSeverity (re-exported from codes::)
// ---------------------------------------------------------------------------

#[test]
fn severity_discriminant_values() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticSeverity::Error as u8, 1);
    assert_eq!(DiagnosticSeverity::Warning as u8, 2);
    assert_eq!(DiagnosticSeverity::Information as u8, 3);
    assert_eq!(DiagnosticSeverity::Hint as u8, 4);
    Ok(())
}

#[test]
fn severity_debug_format() -> Result<(), Box<dyn std::error::Error>> {
    let dbg = format!("{:?}", DiagnosticSeverity::Error);
    assert!(dbg.contains("Error"));
    Ok(())
}

#[test]
fn severity_clone_and_copy() -> Result<(), Box<dyn std::error::Error>> {
    let s = DiagnosticSeverity::Warning;
    let copied = s;
    assert_eq!(s, copied);
    Ok(())
}

#[test]
fn severity_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticSeverity::Error, DiagnosticSeverity::Error);
    assert_ne!(DiagnosticSeverity::Error, DiagnosticSeverity::Warning);
    Ok(())
}

#[test]
fn severity_ordering() -> Result<(), Box<dyn std::error::Error>> {
    // Error(1) < Warning(2) < Information(3) < Hint(4)
    assert!(DiagnosticSeverity::Error < DiagnosticSeverity::Warning);
    assert!(DiagnosticSeverity::Warning < DiagnosticSeverity::Information);
    assert!(DiagnosticSeverity::Information < DiagnosticSeverity::Hint);
    Ok(())
}

#[test]
fn severity_ord_is_total() -> Result<(), Box<dyn std::error::Error>> {
    let mut levels = vec![
        DiagnosticSeverity::Hint,
        DiagnosticSeverity::Error,
        DiagnosticSeverity::Information,
        DiagnosticSeverity::Warning,
    ];
    levels.sort();
    assert_eq!(
        levels,
        vec![
            DiagnosticSeverity::Error,
            DiagnosticSeverity::Warning,
            DiagnosticSeverity::Information,
            DiagnosticSeverity::Hint,
        ]
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// DiagnosticTag (re-exported from codes::)
// ---------------------------------------------------------------------------

#[test]
fn tag_debug_format() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(format!("{:?}", DiagnosticTag::Unnecessary), "Unnecessary");
    assert_eq!(format!("{:?}", DiagnosticTag::Deprecated), "Deprecated");
    Ok(())
}

#[test]
fn tag_clone_and_copy() -> Result<(), Box<dyn std::error::Error>> {
    let tag = DiagnosticTag::Deprecated;
    let copied = tag;
    assert_eq!(tag, copied);
    Ok(())
}

#[test]
fn tag_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DiagnosticTag::Unnecessary, DiagnosticTag::Unnecessary);
    assert_ne!(DiagnosticTag::Unnecessary, DiagnosticTag::Deprecated);
    Ok(())
}

// ---------------------------------------------------------------------------
// RelatedInformation
// ---------------------------------------------------------------------------

#[test]
fn related_info_construction() -> Result<(), Box<dyn std::error::Error>> {
    let info =
        RelatedInformation { location: (10, 20), message: "see declaration here".to_string() };
    assert_eq!(info.location, (10, 20));
    assert_eq!(info.message, "see declaration here");
    Ok(())
}

#[test]
fn related_info_debug_format() -> Result<(), Box<dyn std::error::Error>> {
    let info = RelatedInformation { location: (0, 5), message: "note".to_string() };
    let dbg = format!("{:?}", info);
    assert!(dbg.contains("RelatedInformation"));
    assert!(dbg.contains("note"));
    Ok(())
}

#[test]
fn related_info_clone() -> Result<(), Box<dyn std::error::Error>> {
    let info = RelatedInformation { location: (1, 2), message: "original".to_string() };
    let cloned = info.clone();
    assert_eq!(info, cloned);
    Ok(())
}

#[test]
fn related_info_equality() -> Result<(), Box<dyn std::error::Error>> {
    let a = RelatedInformation { location: (1, 2), message: "msg".to_string() };
    let b = RelatedInformation { location: (1, 2), message: "msg".to_string() };
    let c = RelatedInformation { location: (3, 4), message: "msg".to_string() };
    assert_eq!(a, b);
    assert_ne!(a, c);
    Ok(())
}

#[test]
fn related_info_default() -> Result<(), Box<dyn std::error::Error>> {
    let info = RelatedInformation::default();
    assert!(info.message.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// Diagnostic — construction with new struct shape
// ---------------------------------------------------------------------------

fn make_diagnostic() -> Diagnostic {
    Diagnostic {
        code: perl_diagnostics::codes::DiagnosticCode::ParseError,
        severity: DiagnosticSeverity::Error,
        range: (0, 10),
        message: "syntax error".to_string(),
        related_information: None,
        tags: None,
    }
}

#[test]
fn diagnostic_basic_construction() -> Result<(), Box<dyn std::error::Error>> {
    let d = make_diagnostic();
    assert_eq!(d.range, (0, 10));
    assert_eq!(d.severity, DiagnosticSeverity::Error);
    assert_eq!(d.code, perl_diagnostics::codes::DiagnosticCode::ParseError);
    assert_eq!(d.message, "syntax error");
    assert!(d.related_information.is_none());
    assert!(d.tags.is_none());
    Ok(())
}

#[test]
fn diagnostic_default() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic::default();
    // Default has a valid code (default DiagnosticCode)
    assert_eq!(d.severity, DiagnosticSeverity::Error);
    Ok(())
}

#[test]
fn diagnostic_with_severity_field() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic { severity: DiagnosticSeverity::Warning, ..Default::default() };
    assert_eq!(d.severity, DiagnosticSeverity::Warning);
    Ok(())
}

#[test]
fn diagnostic_with_tags() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic { tags: Some(vec![DiagnosticTag::Unnecessary]), ..Default::default() };
    assert!(d.tags.is_some());
    assert_eq!(d.tags.as_ref().map(|t| t.len()), Some(1));
    Ok(())
}

#[test]
fn diagnostic_with_related_information() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        related_information: Some(vec![RelatedInformation {
            location: (100, 120),
            message: "did you mean 'foo'?".to_string(),
        }]),
        ..Default::default()
    };
    assert!(d.related_information.is_some());
    assert_eq!(d.related_information.as_ref().map(|r| r.len()), Some(1));
    Ok(())
}

// ---------------------------------------------------------------------------
// Diagnostic — traits
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_debug_format() -> Result<(), Box<dyn std::error::Error>> {
    let d = make_diagnostic();
    let dbg = format!("{:?}", d);
    assert!(dbg.contains("Diagnostic"));
    assert!(dbg.contains("syntax error"));
    Ok(())
}

#[test]
fn diagnostic_clone() -> Result<(), Box<dyn std::error::Error>> {
    let d = make_diagnostic();
    let cloned = d.clone();
    assert_eq!(d, cloned);
    Ok(())
}

#[test]
fn diagnostic_equality_same() -> Result<(), Box<dyn std::error::Error>> {
    let a = make_diagnostic();
    let b = make_diagnostic();
    assert_eq!(a, b);
    Ok(())
}

#[test]
fn diagnostic_inequality_different_range() -> Result<(), Box<dyn std::error::Error>> {
    let a = make_diagnostic();
    let mut b = make_diagnostic();
    b.range = (1, 11);
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn diagnostic_inequality_different_severity() -> Result<(), Box<dyn std::error::Error>> {
    let a = make_diagnostic();
    let mut b = make_diagnostic();
    b.severity = DiagnosticSeverity::Hint;
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn diagnostic_inequality_different_message() -> Result<(), Box<dyn std::error::Error>> {
    let a = make_diagnostic();
    let mut b = make_diagnostic();
    b.message = "different".to_string();
    assert_ne!(a, b);
    Ok(())
}

// ---------------------------------------------------------------------------
// Collection behaviour
// ---------------------------------------------------------------------------

#[test]
fn diagnostics_can_be_collected_in_vec() -> Result<(), Box<dyn std::error::Error>> {
    let diagnostics: Vec<Diagnostic> = (0..3_usize)
        .map(|i| Diagnostic {
            range: (i, i + 10),
            severity: DiagnosticSeverity::Warning,
            message: format!("warning {i}"),
            ..Default::default()
        })
        .collect();
    assert_eq!(diagnostics.len(), 3);
    assert_eq!(diagnostics[2].message, "warning 2");
    Ok(())
}

#[test]
fn severity_can_be_used_as_sort_key() -> Result<(), Box<dyn std::error::Error>> {
    let mut diagnostics = [
        Diagnostic { severity: DiagnosticSeverity::Hint, ..Default::default() },
        Diagnostic { severity: DiagnosticSeverity::Error, ..Default::default() },
    ];
    diagnostics.sort_by_key(|d| d.severity);
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Error);
    assert_eq!(diagnostics[1].severity, DiagnosticSeverity::Hint);
    Ok(())
}

// ---------------------------------------------------------------------------
// Structural mutation (fields are public)
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_fields_are_mutable() -> Result<(), Box<dyn std::error::Error>> {
    let mut d = make_diagnostic();
    d.range = (100, 200);
    d.severity = DiagnosticSeverity::Hint;
    d.message = "updated".to_string();
    d.related_information =
        Some(vec![RelatedInformation { location: (0, 0), message: "added".to_string() }]);
    d.tags = Some(vec![DiagnosticTag::Deprecated]);

    assert_eq!(d.range, (100, 200));
    assert_eq!(d.severity, DiagnosticSeverity::Hint);
    assert_eq!(d.message, "updated");
    assert_eq!(d.related_information.as_ref().map(|r| r.len()), Some(1));
    assert_eq!(d.tags.as_ref().map(|t| t.len()), Some(1));
    Ok(())
}
