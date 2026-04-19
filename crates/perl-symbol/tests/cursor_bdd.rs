use perl_symbol::cursor::{
    CursorSymbolKind, extract_symbol_from_source, get_symbol_range_at_position,
};

#[test]
fn given_scalar_when_cursor_on_name_then_extracts_symbol() -> Result<(), String> {
    let source = "my $count = 1;";
    let cursor = source
        .find("count")
        .ok_or_else(|| "expected symbol in fixture".to_string())?;

    let result = extract_symbol_from_source(cursor, source);
    assert_eq!(
        result,
        Some(("count".to_string(), CursorSymbolKind::Scalar))
    );
    Ok(())
}

#[test]
fn given_subroutine_without_sigil_when_cursor_on_name_then_defaults_to_subroutine() {
    let source = "calculate();";
    let cursor = 0;

    let result = extract_symbol_from_source(cursor, source);
    assert_eq!(
        result,
        Some(("calculate".to_string(), CursorSymbolKind::Subroutine))
    );
}

#[test]
fn given_symbol_with_sigil_when_getting_range_then_returns_sigil_and_name() -> Result<(), String> {
    let source = "print $total;";
    let cursor = source
        .find("total")
        .ok_or_else(|| "expected symbol in fixture".to_string())?;

    let range = get_symbol_range_at_position(cursor, source);
    assert_eq!(range, Some((6, 12)));
    Ok(())
}
