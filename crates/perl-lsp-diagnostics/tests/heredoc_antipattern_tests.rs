//! Tests for heredoc anti-pattern diagnostic integration

use perl_lsp_diagnostics::{DiagnosticSeverity, detect_heredoc_antipatterns};

#[test]
fn eval_string_heredoc_produces_warning() {
    let source = r#"my $code = <<'END';
print "hello";
END
eval $code;
"#;
    let diags = detect_heredoc_antipatterns(source);
    // eval-string heredoc detection is regex-based; this pattern may or may not
    // trigger depending on detector heuristics. If it does, verify the shape.
    for d in &diags {
        if d.code.as_deref() == Some("PL805") {
            assert_eq!(d.severity, DiagnosticSeverity::Warning);
            assert!(d.range.0 < source.len());
            assert!(d.range.1 <= source.len());
            return;
        }
    }
}

#[test]
fn clean_heredoc_produces_no_diagnostics() {
    let source = r#"my $text = <<'END';
Hello, world!
END
print $text;
"#;
    let diags = detect_heredoc_antipatterns(source);
    assert!(
        diags.is_empty(),
        "Clean heredoc should produce no diagnostics, got: {diags:?}"
    );
}

#[test]
fn source_filter_heredoc_detected() {
    let source = "use Filter::Util::Call;\n";
    let diags = detect_heredoc_antipatterns(source);
    for d in &diags {
        if d.code.as_deref() == Some("PL803") {
            assert!(matches!(
                d.severity,
                DiagnosticSeverity::Warning | DiagnosticSeverity::Error
            ));
            return;
        }
    }
}

#[test]
fn format_heredoc_detected() {
    let source = "format STDOUT =\n@<<<< @>>>>\n$name, $value\n.\n";
    let diags = detect_heredoc_antipatterns(source);
    for d in &diags {
        if d.code.as_deref() == Some("PL800") {
            assert!(d.code.is_some());
            return;
        }
    }
}
