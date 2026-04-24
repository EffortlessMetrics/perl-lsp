use perl_token::TokenKind;

fn build_table() -> String {
    let mut table = String::from("| TokenKind | display_name | category | canonical lexeme |\n");
    table.push_str("| --- | --- | --- | --- |\n");

    for kind in TokenKind::ALL {
        let canonical = kind.canonical_lexeme().unwrap_or("-");
        table.push_str(&format!(
            "| {:?} | {} | {} | {} |\n",
            kind,
            kind.display_name(),
            kind.category().as_str(),
            canonical
        ));
    }

    table
}

fn fnv1a_64(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in input.as_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[test]
fn display_name_table_snapshot_is_stable() {
    let table = build_table();
    let fingerprint = fnv1a_64(&table);

    assert_eq!(
        fingerprint, "18c10ba4e8b96fdb",
        "TokenKind display table changed. If intentional, update the fingerprint and review table:\n{}",
        table
    );
}

#[test]
fn display_name_is_non_empty_for_every_token_kind() {
    for kind in TokenKind::ALL {
        assert!(!kind.display_name().is_empty(), "display_name() returned empty for {kind:?}");
    }
}
