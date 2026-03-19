use perl_parser_core::error_recovery::SyncPoint;

#[test]
fn all_variants_exist() -> Result<(), Box<dyn std::error::Error>> {
    let variants =
        [SyncPoint::Semicolon, SyncPoint::CloseBrace, SyncPoint::Keyword, SyncPoint::Eof];
    // All are distinct
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            assert_ne!(variants[i], variants[j]);
        }
    }
    Ok(())
}

#[test]
fn debug_format() -> Result<(), Box<dyn std::error::Error>> {
    assert!(format!("{:?}", SyncPoint::Semicolon).contains("Semicolon"));
    assert!(format!("{:?}", SyncPoint::CloseBrace).contains("CloseBrace"));
    assert!(format!("{:?}", SyncPoint::Keyword).contains("Keyword"));
    assert!(format!("{:?}", SyncPoint::Eof).contains("Eof"));
    Ok(())
}
