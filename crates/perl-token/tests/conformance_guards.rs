use std::error::Error;
use std::fs;

use perl_token::{Token, TokenCategory, TokenKind};

const EXPECTED_TOKEN_KIND_COUNT: usize = 132;

#[test]
fn runtime_dependencies_remain_empty() -> Result<(), Box<dyn Error>> {
    let manifest = fs::read_to_string("Cargo.toml")?;
    let mut in_dependencies = false;

    for raw_line in manifest.lines() {
        let line = raw_line.trim();

        if line.starts_with('[') {
            in_dependencies = line == "[dependencies]";
            continue;
        }

        if !in_dependencies || line.is_empty() || line.starts_with('#') {
            continue;
        }

        return Err(format!("runtime dependency entry is not allowed in perl-token: {line}").into());
    }

    Ok(())
}

#[test]
fn token_and_token_kind_api_snapshot_is_stable() {
    let token = Token { kind: TokenKind::Identifier, text: "foo".into(), start: 10, end: 13 };
    assert_eq!(token.kind, TokenKind::Identifier);
    assert_eq!(token.start, 10);
    assert_eq!(token.end, 13);

    let names: Vec<String> = TokenKind::all().iter().map(|kind| format!("{kind:?}")).collect();
    assert_eq!(names.len(), EXPECTED_TOKEN_KIND_COUNT);
    assert_eq!(
        names,
        vec![
            "My",
            "Our",
            "Local",
            "State",
            "Sub",
            "If",
            "Elsif",
            "Else",
            "Unless",
            "While",
            "Until",
            "For",
            "Foreach",
            "Return",
            "Package",
            "Use",
            "No",
            "Begin",
            "End",
            "Check",
            "Init",
            "Unitcheck",
            "Eval",
            "Do",
            "Given",
            "When",
            "Default",
            "Try",
            "Catch",
            "Finally",
            "Continue",
            "Next",
            "Last",
            "Redo",
            "Goto",
            "Class",
            "Method",
            "Field",
            "Format",
            "Undef",
            "Defer",
            "Assign",
            "Plus",
            "Minus",
            "Star",
            "Slash",
            "Percent",
            "Power",
            "LeftShift",
            "RightShift",
            "BitwiseAnd",
            "BitwiseOr",
            "BitwiseXor",
            "BitwiseNot",
            "PlusAssign",
            "MinusAssign",
            "StarAssign",
            "SlashAssign",
            "PercentAssign",
            "DotAssign",
            "AndAssign",
            "OrAssign",
            "XorAssign",
            "PowerAssign",
            "LeftShiftAssign",
            "RightShiftAssign",
            "LogicalAndAssign",
            "LogicalOrAssign",
            "DefinedOrAssign",
            "Equal",
            "NotEqual",
            "Match",
            "NotMatch",
            "SmartMatch",
            "Less",
            "Greater",
            "LessEqual",
            "GreaterEqual",
            "Spaceship",
            "StringCompare",
            "And",
            "Or",
            "Not",
            "DefinedOr",
            "WordAnd",
            "WordOr",
            "WordNot",
            "WordXor",
            "Arrow",
            "FatArrow",
            "Dot",
            "Range",
            "Ellipsis",
            "Increment",
            "Decrement",
            "DoubleColon",
            "Question",
            "Colon",
            "Backslash",
            "LeftParen",
            "RightParen",
            "LeftBrace",
            "RightBrace",
            "LeftBracket",
            "RightBracket",
            "Semicolon",
            "Comma",
            "Number",
            "String",
            "Regex",
            "Substitution",
            "Transliteration",
            "QuoteSingle",
            "QuoteDouble",
            "QuoteWords",
            "QuoteCommand",
            "HeredocStart",
            "HeredocBody",
            "FormatBody",
            "DataMarker",
            "DataBody",
            "VString",
            "UnknownRest",
            "HeredocDepthLimit",
            "Identifier",
            "ScalarSigil",
            "ArraySigil",
            "HashSigil",
            "SubSigil",
            "GlobSigil",
            "Eof",
            "Unknown"
        ]
    );
}

#[test]
fn tokenkind_metadata_is_complete_and_in_sync() {
    assert_eq!(TokenKind::all().len(), TokenKind::metadata_count());

    for kind in TokenKind::all() {
        let metadata = kind.metadata();
        assert!(!metadata.display_name.is_empty(), "missing display_name for {kind:?}");
        match metadata.category {
            TokenCategory::Keyword
            | TokenCategory::Operator
            | TokenCategory::Delimiter
            | TokenCategory::Literal
            | TokenCategory::Identifier
            | TokenCategory::Special => {}
        }
    }
}

#[test]
fn docs_track_leaf_contract_and_variant_count() -> Result<(), Box<dyn Error>> {
    let readme = fs::read_to_string("README.md")?;
    let roadmap = fs::read_to_string("ROADMAP.md")?;
    let expected_count_line = format!("TokenKind variants: {}", TokenKind::all().len());

    for doc in [&readme, &roadmap] {
        if !doc.contains("tiny stable leaf crate") {
            return Err("missing leaf-crate stability contract note".into());
        }
        if !doc.contains(&expected_count_line) {
            return Err(format!("missing TokenKind count note: {expected_count_line}").into());
        }
        if !doc.contains("Conformance update rule") {
            return Err("missing conformance update rule note".into());
        }
    }

    Ok(())
}
