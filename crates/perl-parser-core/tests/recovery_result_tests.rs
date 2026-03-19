use perl_parser_core::error_recovery::RecoveryResult;

#[test]
fn recovery_result_variants_distinct() -> Result<(), Box<dyn std::error::Error>> {
    let recovered = RecoveryResult::Recovered(3);
    let at_sync = RecoveryResult::AtSyncPoint;
    let exhausted = RecoveryResult::BudgetExhausted;
    let eof = RecoveryResult::ReachedEof;

    assert_ne!(recovered, at_sync);
    assert_ne!(at_sync, exhausted);
    assert_ne!(exhausted, eof);
    assert_ne!(eof, recovered);
    Ok(())
}

#[test]
fn recovery_result_clone_eq() -> Result<(), Box<dyn std::error::Error>> {
    let original = RecoveryResult::Recovered(5);
    let cloned = original;
    assert_eq!(original, cloned);
    Ok(())
}
