use perl_parser_core::error_recovery::RecoveryResult;

#[test]
fn recovered_with_count() -> Result<(), Box<dyn std::error::Error>> {
    let r = RecoveryResult::Recovered(5);
    if let RecoveryResult::Recovered(n) = r {
        assert_eq!(n, 5);
    } else {
        return Err("expected Recovered".into());
    }
    Ok(())
}

#[test]
fn all_variants_debug() -> Result<(), Box<dyn std::error::Error>> {
    let variants = vec![
        RecoveryResult::Recovered(0),
        RecoveryResult::AtSyncPoint,
        RecoveryResult::BudgetExhausted,
        RecoveryResult::ReachedEof,
    ];
    for v in &variants {
        let dbg = format!("{:?}", v);
        assert!(!dbg.is_empty());
    }
    Ok(())
}
