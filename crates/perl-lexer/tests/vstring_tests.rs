//! Tests for v-string (version string) tokenization.
//!
//! V-strings are a Perl literal form like `v5.26.0` or `v5.10` commonly used
//! in `use`, `require`, and version declarations.

use perl_lexer::{PerlLexer, TokenType};

type R = Result<(), Box<dyn std::error::Error>>;

/// Collect only the significant (non-whitespace, non-newline, non-EOF) tokens.
fn significant(input: &str) -> Vec<perl_lexer::Token> {
    PerlLexer::new(input)
        .collect_tokens()
        .into_iter()
        .filter(|t| {
            !matches!(t.token_type, TokenType::Whitespace | TokenType::Newline | TokenType::EOF)
        })
        .collect()
}

// ===========================================================================
// Basic v-string recognition
// ===========================================================================

#[test]
fn test_vstring_three_part_version() -> R {
    let toks = significant("v5.26.0");
    assert_eq!(toks.len(), 1, "expected single token, got: {:?}", toks);
    assert!(
        matches!(&toks[0].token_type, TokenType::Version(v) if v.as_ref() == "v5.26.0"),
        "expected Version(v5.26.0), got: {:?}",
        toks[0].token_type
    );
    assert_eq!(toks[0].text.as_ref(), "v5.26.0");
    assert_eq!(toks[0].start, 0);
    assert_eq!(toks[0].end, 7);
    Ok(())
}

#[test]
fn test_vstring_two_part_version() -> R {
    let toks = significant("v5.10");
    assert_eq!(toks.len(), 1, "expected single token, got: {:?}", toks);
    assert!(
        matches!(&toks[0].token_type, TokenType::Version(v) if v.as_ref() == "v5.10"),
        "expected Version(v5.10), got: {:?}",
        toks[0].token_type
    );
    Ok(())
}

#[test]
fn test_vstring_four_part_version() -> R {
    let toks = significant("v1.2.3.4");
    assert_eq!(toks.len(), 1, "expected single token, got: {:?}", toks);
    assert!(
        matches!(&toks[0].token_type, TokenType::Version(v) if v.as_ref() == "v1.2.3.4"),
        "expected Version(v1.2.3.4), got: {:?}",
        toks[0].token_type
    );
    Ok(())
}

// ===========================================================================
// V-strings in context
// ===========================================================================

#[test]
fn test_vstring_in_use_statement() -> R {
    let toks = significant("use v5.26;");
    // Should have: use, v5.26, ;  -- but v5.26 needs a dot, so it qualifies
    // Actually "use" is a keyword, "v5.26" is a v-string, ";" is semicolon
    let version_tok = toks.iter().find(|t| matches!(&t.token_type, TokenType::Version(_)));
    assert!(
        version_tok.is_some(),
        "expected a Version token in 'use v5.26;', got: {:?}",
        toks.iter().map(|t| format!("{:?}", t.token_type)).collect::<Vec<_>>()
    );
    let vt = version_tok.ok_or("no version token")?;
    assert!(
        matches!(&vt.token_type, TokenType::Version(v) if v.as_ref() == "v5.26"),
        "expected Version(v5.26), got: {:?}",
        vt.token_type
    );
    Ok(())
}

#[test]
fn test_vstring_in_require_statement() -> R {
    let toks = significant("require v5.10.0;");
    let version_tok = toks.iter().find(|t| matches!(&t.token_type, TokenType::Version(_)));
    assert!(
        version_tok.is_some(),
        "expected a Version token in 'require v5.10.0;', got: {:?}",
        toks.iter().map(|t| format!("{:?}", t.token_type)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_vstring_in_assignment() -> R {
    // $VERSION = v1.2.3;
    let toks = significant("$VERSION = v1.2.3;");
    let version_tok = toks.iter().find(|t| matches!(&t.token_type, TokenType::Version(_)));
    assert!(
        version_tok.is_some(),
        "expected a Version token in '$VERSION = v1.2.3;', got: {:?}",
        toks.iter().map(|t| format!("{:?}", t.token_type)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// Edge cases: things that should NOT be v-strings
// ===========================================================================

#[test]
fn test_bare_v_is_identifier() -> R {
    // Just "v" by itself is an identifier
    let toks = significant("v");
    assert_eq!(toks.len(), 1);
    assert!(
        matches!(&toks[0].token_type, TokenType::Identifier(_) | TokenType::Keyword(_)),
        "expected identifier for bare 'v', got: {:?}",
        toks[0].token_type
    );
    Ok(())
}

#[test]
fn test_v_followed_by_alpha_is_identifier() -> R {
    // "var" should be a normal identifier, not a v-string
    let toks = significant("var");
    assert_eq!(toks.len(), 1);
    assert!(
        matches!(&toks[0].token_type, TokenType::Identifier(_) | TokenType::Keyword(_)),
        "expected identifier for 'var', got: {:?}",
        toks[0].token_type
    );
    Ok(())
}

#[test]
fn test_v_digits_no_dot_is_identifier() -> R {
    // "v5" without a dot should remain an identifier (conservative approach)
    let toks = significant("v5");
    assert_eq!(toks.len(), 1);
    assert!(
        matches!(&toks[0].token_type, TokenType::Identifier(_) | TokenType::Keyword(_)),
        "expected identifier for 'v5', got: {:?}",
        toks[0].token_type
    );
    Ok(())
}

#[test]
fn test_v_digits_underscore_is_identifier() -> R {
    // "v5_test" should be a normal identifier
    let toks = significant("v5_test");
    assert_eq!(toks.len(), 1);
    assert!(
        matches!(&toks[0].token_type, TokenType::Identifier(_) | TokenType::Keyword(_)),
        "expected identifier for 'v5_test', got: {:?}",
        toks[0].token_type
    );
    Ok(())
}

#[test]
fn test_v_digits_alpha_is_identifier() -> R {
    // "v5x" should be a normal identifier
    let toks = significant("v5x");
    assert_eq!(toks.len(), 1);
    assert!(
        matches!(&toks[0].token_type, TokenType::Identifier(_) | TokenType::Keyword(_)),
        "expected identifier for 'v5x', got: {:?}",
        toks[0].token_type
    );
    Ok(())
}

// ===========================================================================
// Span correctness
// ===========================================================================

#[test]
fn test_vstring_span_in_statement() -> R {
    // "use v5.26.0;"
    //  0123456789...
    let toks = significant("use v5.26.0;");
    let version_tok = toks
        .iter()
        .find(|t| matches!(&t.token_type, TokenType::Version(_)))
        .ok_or("no version token found")?;
    assert_eq!(version_tok.start, 4, "v-string should start at byte 4");
    assert_eq!(version_tok.end, 11, "v-string should end at byte 11");
    Ok(())
}

// ===========================================================================
// Dot-terminated v-string: trailing dot not consumed
// ===========================================================================

#[test]
fn test_vstring_trailing_dot_not_consumed() -> R {
    // "v5.26." -- the trailing dot (not followed by digit) should NOT be part of v-string
    let toks = significant("v5.26.");
    // Should get: Version("v5.26"), then some token for "."
    let version_tok = toks.iter().find(|t| matches!(&t.token_type, TokenType::Version(_)));
    assert!(
        version_tok.is_some(),
        "expected Version token for 'v5.26.', got: {:?}",
        toks.iter().map(|t| format!("{:?}", t.token_type)).collect::<Vec<_>>()
    );
    let vt = version_tok.ok_or("no version token")?;
    assert!(
        matches!(&vt.token_type, TokenType::Version(v) if v.as_ref() == "v5.26"),
        "expected Version(v5.26), got: {:?}",
        vt.token_type
    );
    Ok(())
}

#[test]
fn test_vstring_large_version_numbers() -> R {
    let toks = significant("v536.100.200");
    assert_eq!(toks.len(), 1);
    assert!(
        matches!(&toks[0].token_type, TokenType::Version(v) if v.as_ref() == "v536.100.200"),
        "expected Version(v536.100.200), got: {:?}",
        toks[0].token_type
    );
    Ok(())
}
