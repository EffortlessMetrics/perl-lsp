//! Comprehensive unit tests for `perl-lsp-diagnostic-types`.

use perl_lsp_diagnostic_types::{
    Diagnostic, DiagnosticSeverity, DiagnosticTag, RelatedInformation,
};

// ---------------------------------------------------------------------------
// DiagnosticSeverity
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
// DiagnosticTag
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
fn related_info_different_message_not_equal() -> Result<(), Box<dyn std::error::Error>> {
    let a = RelatedInformation { location: (0, 0), message: "alpha".to_string() };
    let b = RelatedInformation { location: (0, 0), message: "beta".to_string() };
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn related_info_empty_message() -> Result<(), Box<dyn std::error::Error>> {
    let info = RelatedInformation { location: (0, 0), message: String::new() };
    assert!(info.message.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// Diagnostic — construction
// ---------------------------------------------------------------------------

fn make_diagnostic() -> Diagnostic {
    Diagnostic {
        range: (0, 10),
        severity: DiagnosticSeverity::Error,
        code: Some("E001".to_string()),
        message: "syntax error".to_string(),
        related_information: vec![],
        tags: vec![],
        suggestion: None,
    }
}

#[test]
fn diagnostic_basic_construction() -> Result<(), Box<dyn std::error::Error>> {
    let d = make_diagnostic();
    assert_eq!(d.range, (0, 10));
    assert_eq!(d.severity, DiagnosticSeverity::Error);
    assert_eq!(d.code.as_deref(), Some("E001"));
    assert_eq!(d.message, "syntax error");
    assert!(d.related_information.is_empty());
    assert!(d.tags.is_empty());
    Ok(())
}

#[test]
fn diagnostic_no_code() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        range: (5, 15),
        severity: DiagnosticSeverity::Warning,
        code: None,
        message: "unused variable".to_string(),
        related_information: vec![],
        tags: vec![DiagnosticTag::Unnecessary],
        suggestion: None,
    };
    assert!(d.code.is_none());
    assert_eq!(d.tags.len(), 1);
    Ok(())
}

#[test]
fn diagnostic_with_related_information() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        range: (0, 5),
        severity: DiagnosticSeverity::Error,
        code: None,
        message: "undefined subroutine".to_string(),
        related_information: vec![
            RelatedInformation { location: (100, 120), message: "did you mean 'foo'?".to_string() },
            RelatedInformation { location: (200, 220), message: "defined here".to_string() },
        ],
        tags: vec![],
        suggestion: None,
    };
    assert_eq!(d.related_information.len(), 2);
    assert_eq!(d.related_information[0].message, "did you mean 'foo'?");
    assert_eq!(d.related_information[1].location, (200, 220));
    Ok(())
}

#[test]
fn diagnostic_with_multiple_tags() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        range: (0, 1),
        severity: DiagnosticSeverity::Hint,
        code: None,
        message: "deprecated and unused".to_string(),
        related_information: vec![],
        tags: vec![DiagnosticTag::Unnecessary, DiagnosticTag::Deprecated],
        suggestion: None,
    };
    assert_eq!(d.tags.len(), 2);
    assert!(d.tags.contains(&DiagnosticTag::Unnecessary));
    assert!(d.tags.contains(&DiagnosticTag::Deprecated));
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
    let d = Diagnostic {
        range: (10, 20),
        severity: DiagnosticSeverity::Information,
        code: Some("I001".to_string()),
        message: "info".to_string(),
        related_information: vec![RelatedInformation {
            location: (30, 40),
            message: "related".to_string(),
        }],
        tags: vec![DiagnosticTag::Deprecated],
        suggestion: None,
    };
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
fn diagnostic_inequality_different_code() -> Result<(), Box<dyn std::error::Error>> {
    let a = make_diagnostic();
    let mut b = make_diagnostic();
    b.code = Some("E999".to_string());
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

#[test]
fn diagnostic_inequality_different_tags() -> Result<(), Box<dyn std::error::Error>> {
    let a = make_diagnostic();
    let mut b = make_diagnostic();
    b.tags = vec![DiagnosticTag::Deprecated];
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn diagnostic_inequality_different_related_info() -> Result<(), Box<dyn std::error::Error>> {
    let a = make_diagnostic();
    let mut b = make_diagnostic();
    b.related_information =
        vec![RelatedInformation { location: (0, 0), message: "extra".to_string() }];
    assert_ne!(a, b);
    Ok(())
}

// ---------------------------------------------------------------------------
// Diagnostic — edge cases
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_zero_width_range() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        range: (5, 5),
        severity: DiagnosticSeverity::Hint,
        code: None,
        message: "zero-width".to_string(),
        related_information: vec![],
        tags: vec![],
        suggestion: None,
    };
    assert_eq!(d.range.0, d.range.1);
    Ok(())
}

#[test]
fn diagnostic_large_range() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        range: (0, usize::MAX),
        severity: DiagnosticSeverity::Error,
        code: None,
        message: "whole file".to_string(),
        related_information: vec![],
        tags: vec![],
        suggestion: None,
    };
    assert_eq!(d.range.1, usize::MAX);
    Ok(())
}

#[test]
fn diagnostic_unicode_message() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        range: (0, 1),
        severity: DiagnosticSeverity::Warning,
        code: None,
        message: "未定義の変数 — café ñ 日本語".to_string(),
        related_information: vec![],
        tags: vec![],
        suggestion: None,
    };
    assert!(d.message.contains("未定義"));
    assert!(d.message.contains("café"));
    Ok(())
}

#[test]
fn diagnostic_empty_message() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        range: (0, 0),
        severity: DiagnosticSeverity::Information,
        code: None,
        message: String::new(),
        related_information: vec![],
        tags: vec![],
        suggestion: None,
    };
    assert!(d.message.is_empty());
    Ok(())
}

#[test]
fn diagnostic_empty_code_string() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        range: (0, 0),
        severity: DiagnosticSeverity::Information,
        code: Some(String::new()),
        message: "has empty code".to_string(),
        related_information: vec![],
        tags: vec![],
        suggestion: None,
    };
    assert_eq!(d.code.as_deref(), Some(""));
    Ok(())
}

// ---------------------------------------------------------------------------
// Collection behaviour
// ---------------------------------------------------------------------------

#[test]
fn diagnostics_can_be_collected_in_vec() -> Result<(), Box<dyn std::error::Error>> {
    let diagnostics: Vec<Diagnostic> = (0..3)
        .map(|i| Diagnostic {
            range: (i, i + 10),
            severity: DiagnosticSeverity::Warning,
            code: None,
            message: format!("warning {i}"),
            related_information: vec![],
            tags: vec![],
            suggestion: None,
        })
        .collect();
    assert_eq!(diagnostics.len(), 3);
    assert_eq!(diagnostics[2].message, "warning 2");
    Ok(())
}

#[test]
fn severity_can_be_used_as_sort_key() -> Result<(), Box<dyn std::error::Error>> {
    let mut diagnostics = [
        Diagnostic {
            range: (0, 1),
            severity: DiagnosticSeverity::Hint,
            code: None,
            message: "hint".to_string(),
            related_information: vec![],
            tags: vec![],
            suggestion: None,
        },
        Diagnostic {
            range: (0, 1),
            severity: DiagnosticSeverity::Error,
            code: None,
            message: "error".to_string(),
            related_information: vec![],
            tags: vec![],
            suggestion: None,
        },
    ];
    diagnostics.sort_by_key(|d| d.severity);
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Error);
    assert_eq!(diagnostics[1].severity, DiagnosticSeverity::Hint);
    Ok(())
}

#[test]
fn tags_can_be_filtered() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        range: (0, 1),
        severity: DiagnosticSeverity::Warning,
        code: None,
        message: "tagged".to_string(),
        related_information: vec![],
        tags: vec![DiagnosticTag::Unnecessary, DiagnosticTag::Deprecated],
        suggestion: None,
    };
    let deprecated: Vec<_> = d.tags.iter().filter(|t| **t == DiagnosticTag::Deprecated).collect();
    assert_eq!(deprecated.len(), 1);
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
    d.code = None;
    d.message = "updated".to_string();
    d.related_information
        .push(RelatedInformation { location: (0, 0), message: "added".to_string() });
    d.tags.push(DiagnosticTag::Deprecated);

    assert_eq!(d.range, (100, 200));
    assert_eq!(d.severity, DiagnosticSeverity::Hint);
    assert!(d.code.is_none());
    assert_eq!(d.message, "updated");
    assert_eq!(d.related_information.len(), 1);
    assert_eq!(d.tags.len(), 1);
    Ok(())
}

#[test]
fn related_info_fields_are_mutable() -> Result<(), Box<dyn std::error::Error>> {
    let mut info = RelatedInformation { location: (0, 0), message: "old".to_string() };
    info.location = (10, 20);
    info.message = "new".to_string();
    assert_eq!(info.location, (10, 20));
    assert_eq!(info.message, "new");
    Ok(())
}
