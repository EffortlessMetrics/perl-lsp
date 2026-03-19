use perl_parser_core::DiagnosticId;

#[test]
fn diagnostic_id_is_u32() -> Result<(), Box<dyn std::error::Error>> {
    let id: DiagnosticId = 42;
    assert_eq!(id, 42u32);
    let id2: DiagnosticId = 0;
    assert_eq!(id2, 0u32);
    let id3: DiagnosticId = u32::MAX;
    assert_eq!(id3, u32::MAX);
    Ok(())
}
