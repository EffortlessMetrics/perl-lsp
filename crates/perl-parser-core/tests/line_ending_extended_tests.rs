use perl_parser_core::{
    LineEnding,
    // Position mapping
    PositionMapper,
};

#[test]
fn cr_only_detection() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\rline2\rline3";
    let mapper = PositionMapper::new(source);
    let ending = mapper.line_ending();
    // CR-only should be detected
    assert!(
        ending == LineEnding::Cr || ending == LineEnding::Mixed,
        "expected Cr or Mixed, got {:?}",
        ending
    );
    Ok(())
}

#[test]
fn lf_detection() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2\nline3";
    let mapper = PositionMapper::new(source);
    assert_eq!(mapper.line_ending(), LineEnding::Lf);
    Ok(())
}

#[test]
fn crlf_detection() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\r\nline2\r\nline3";
    let mapper = PositionMapper::new(source);
    assert_eq!(mapper.line_ending(), LineEnding::CrLf);
    Ok(())
}

#[test]
fn mixed_line_ending_detection() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2\r\nline3\rline4";
    let mapper = PositionMapper::new(source);
    assert_eq!(mapper.line_ending(), LineEnding::Mixed);
    Ok(())
}

#[test]
fn no_newline_defaults_to_lf() -> Result<(), Box<dyn std::error::Error>> {
    let source = "single line no newline";
    let mapper = PositionMapper::new(source);
    // When there are no newlines, default should be Lf
    assert_eq!(mapper.line_ending(), LineEnding::Lf);
    Ok(())
}

#[test]
fn line_ending_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(LineEnding::Lf, LineEnding::Lf);
    assert_eq!(LineEnding::CrLf, LineEnding::CrLf);
    assert_eq!(LineEnding::Cr, LineEnding::Cr);
    assert_eq!(LineEnding::Mixed, LineEnding::Mixed);
    assert_ne!(LineEnding::Lf, LineEnding::CrLf);
    assert_ne!(LineEnding::Cr, LineEnding::Mixed);
    Ok(())
}
