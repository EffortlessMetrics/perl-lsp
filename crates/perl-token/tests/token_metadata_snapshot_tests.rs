use std::error::Error;
use std::fs;

use perl_token::TokenKind;

const SNAPSHOT_PATH: &str = "tests/snapshots/token_kind_metadata_table.md";

fn render_metadata_table() -> String {
    let mut output = String::from("# TokenKind metadata snapshot\n\n");
    output.push_str("| TokenKind | display_name | category |\n");
    output.push_str("|---|---|---|\n");

    for kind in TokenKind::all() {
        let metadata = kind.metadata();
        output.push_str(&format!(
            "| {:?} | {} | {:?} |\n",
            kind, metadata.display_name, metadata.category
        ));
    }

    output
}

#[test]
fn token_kind_metadata_snapshot() -> Result<(), Box<dyn Error>> {
    let expected = fs::read_to_string(SNAPSHOT_PATH)?;
    let actual = render_metadata_table();
    assert_eq!(actual, expected, "TokenKind metadata snapshot changed");
    Ok(())
}
