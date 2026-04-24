use perl_parser_core::Parser;

type R = Result<(), Box<dyn std::error::Error>>;

fn assert_clean_sexp_without_error_nodes(source: &str) -> R {
    let mut parser = Parser::new(source);
    let parsed = parser.parse_with_recovery();
    let sexp = parsed.ast.to_sexp();

    let error_markers = ["(error ", "(Error ", "(missing_expression", " ERROR "];
    for marker in error_markers {
        if sexp.contains(marker) {
            return Err(format!(
                "Expected parse without ERROR-like nodes for source:\n{source}\n\nS-expression:\n{sexp}"
            )
            .into());
        }
    }

    Ok(())
}

fn assert_has_unclosed_interpolation_diagnostic(source: &str) -> R {
    let mut parser = Parser::new(source);
    let parsed = parser.parse_with_recovery();

    let has_expected = parsed
        .diagnostics
        .iter()
        .map(ToString::to_string)
        .any(|diag| diag.contains("Unclosed") && diag.contains("interpolated"));
    if !has_expected {
        return Err(format!(
            "Expected unclosed interpolation diagnostic for source:\n{source}\n\nDiagnostics:\n{:?}",
            parsed.diagnostics
        )
        .into());
    }

    Ok(())
}

#[test]
fn double_quote_incomplete_hash_key() -> R {
    let source = r#"my $msg = "Key: $hash{incomplete";"#;
    assert_clean_sexp_without_error_nodes(source)?;
    assert_has_unclosed_interpolation_diagnostic(source)?;
    Ok(())
}

#[test]
fn double_quote_incomplete_array_index() -> R {
    let source = r#"my $item = "Element: $array[0";"#;
    assert_clean_sexp_without_error_nodes(source)?;
    assert_has_unclosed_interpolation_diagnostic(source)?;
    Ok(())
}

#[test]
fn double_quote_incomplete_arrow_hash_field() -> R {
    let source = r#"my $msg = "Nested: $obj->{field";"#;
    assert_clean_sexp_without_error_nodes(source)?;
    assert_has_unclosed_interpolation_diagnostic(source)?;
    Ok(())
}

#[test]
fn double_quote_incomplete_mixed_array_index() -> R {
    let source = r#"my $msg = "Mixed: $array[$i";"#;
    assert_clean_sexp_without_error_nodes(source)?;
    assert_has_unclosed_interpolation_diagnostic(source)?;
    Ok(())
}

#[test]
fn double_quote_complete_interpolation_cases() -> R {
    let source = r#"my $msg = "Complete: $hash{key} $array[0] $obj->{field}";"#;
    assert_clean_sexp_without_error_nodes(source)?;

    let mut parser = Parser::new(source);
    let parsed = parser.parse_with_recovery();
    let has_unclosed = parsed
        .diagnostics
        .iter()
        .map(ToString::to_string)
        .any(|diag| diag.contains("Unclosed") && diag.contains("interpolated"));
    if has_unclosed {
        return Err(format!(
            "Did not expect unclosed interpolation diagnostics for complete interpolation. Diagnostics: {:?}",
            parsed.diagnostics
        )
        .into());
    }

    Ok(())
}
