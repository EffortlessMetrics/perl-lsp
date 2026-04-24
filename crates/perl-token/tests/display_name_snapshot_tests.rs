use perl_token::TokenKind;

fn render_snapshot_table() -> String {
    let mut out = String::from("| TokenKind | display_name | category | canonical_lexeme |\n");
    out.push_str("|---|---|---|---|\n");

    for kind in TokenKind::all() {
        let display_name = kind.display_name();
        let canonical_lexeme = kind.canonical_lexeme().unwrap_or("-");
        out.push_str(&format!(
            "| {:?} | {} | {} | {} |\n",
            kind,
            display_name,
            kind.category().as_str(),
            canonical_lexeme
        ));
    }

    out
}

#[test]
fn every_token_kind_has_non_empty_display_name() {
    for kind in TokenKind::all() {
        assert!(
            !kind.display_name().trim().is_empty(),
            "TokenKind::{kind:?} has an empty display_name"
        );
    }
}

#[test]
fn token_kind_display_table_snapshot() {
    if std::env::var("UPDATE_SNAPSHOT").as_deref() == Ok("1") {
        if let Err(error) =
            std::fs::write("tests/snapshots/token_kind_display_table.md", render_snapshot_table())
        {
            panic!("failed to update token_kind_display_table snapshot: {error}");
        }
    }

    let expected = include_str!("snapshots/token_kind_display_table.md");
    assert_eq!(render_snapshot_table().trim(), expected.trim());
}
