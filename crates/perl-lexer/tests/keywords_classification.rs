//! Tests for Perl keyword classification: recognition, categorization,
//! case sensitivity across all lookup helpers, modern keyword membership,
//! and declaration keyword cross-list coverage.

use perl_lexer::{
    DAP_COMPLETION_KEYWORDS, KEYWORDS, LEXER_KEYWORDS, LSP_COMPLETION_KEYWORDS,
    LSP_RUNTIME_COMPLETION_KEYWORDS, PARSER_LSP_KEYWORDS, RENAME_KEYWORDS,
    is_dap_completion_keyword, is_keyword, is_lexer_keyword, is_lsp_completion_keyword,
    is_lsp_runtime_completion_keyword, is_parser_lsp_keyword, is_rename_keyword,
};

// ---------------------------------------------------------------------------
// Case sensitivity for helpers not covered by existing tests
// ---------------------------------------------------------------------------

#[test]
fn rename_lookup_is_case_sensitive() {
    assert!(is_rename_keyword("my"));
    assert!(!is_rename_keyword("My"));
    assert!(!is_rename_keyword("MY"));

    assert!(is_rename_keyword("sub"));
    assert!(!is_rename_keyword("Sub"));
    assert!(!is_rename_keyword("SUB"));

    assert!(is_rename_keyword("package"));
    assert!(!is_rename_keyword("Package"));
    assert!(!is_rename_keyword("PACKAGE"));
}

#[test]
fn parser_lsp_lookup_is_case_sensitive() {
    assert!(is_parser_lsp_keyword("if"));
    assert!(!is_parser_lsp_keyword("If"));
    assert!(!is_parser_lsp_keyword("IF"));

    assert!(is_parser_lsp_keyword("while"));
    assert!(!is_parser_lsp_keyword("While"));
    assert!(!is_parser_lsp_keyword("WHILE"));

    assert!(is_parser_lsp_keyword("eval"));
    assert!(!is_parser_lsp_keyword("Eval"));
    assert!(!is_parser_lsp_keyword("EVAL"));
}

#[test]
fn runtime_completion_lookup_is_case_sensitive() {
    assert!(is_lsp_runtime_completion_keyword("my"));
    assert!(!is_lsp_runtime_completion_keyword("My"));
    assert!(!is_lsp_runtime_completion_keyword("MY"));

    assert!(is_lsp_runtime_completion_keyword("for"));
    assert!(!is_lsp_runtime_completion_keyword("For"));
    assert!(!is_lsp_runtime_completion_keyword("FOR"));

    assert!(is_lsp_runtime_completion_keyword("print"));
    assert!(!is_lsp_runtime_completion_keyword("Print"));
    assert!(!is_lsp_runtime_completion_keyword("PRINT"));
}

// ---------------------------------------------------------------------------
// Modern Perl keywords: field, class, method — cross-list membership
// ---------------------------------------------------------------------------

#[test]
fn field_keyword_cross_list_membership() {
    assert!(is_keyword("field"));
    assert!(is_lexer_keyword("field"));
    // field is in LSP_COMPLETION_KEYWORDS (Perl 5.38+ class syntax)
    assert!(is_lsp_completion_keyword("field"));
    // field is not in other lists
    assert!(!is_dap_completion_keyword("field"));
    assert!(!is_lsp_runtime_completion_keyword("field"));
    assert!(!is_rename_keyword("field"));
    assert!(!is_parser_lsp_keyword("field"));
}

#[test]
fn class_keyword_cross_list_membership() {
    assert!(is_keyword("class"));
    assert!(is_lexer_keyword("class"));
    // class is in LSP_COMPLETION_KEYWORDS (Perl 5.38+ class syntax)
    assert!(is_lsp_completion_keyword("class"));
    // class is not in other lists
    assert!(!is_dap_completion_keyword("class"));
    assert!(!is_lsp_runtime_completion_keyword("class"));
    assert!(!is_rename_keyword("class"));
    assert!(!is_parser_lsp_keyword("class"));
}

#[test]
fn method_keyword_cross_list_membership() {
    assert!(is_keyword("method"));
    assert!(is_lexer_keyword("method"));
    // method is in LSP_COMPLETION_KEYWORDS (Perl 5.38+ class syntax)
    assert!(is_lsp_completion_keyword("method"));
    // method is not in other lists
    assert!(!is_dap_completion_keyword("method"));
    assert!(!is_lsp_runtime_completion_keyword("method"));
    assert!(!is_rename_keyword("method"));
    assert!(!is_parser_lsp_keyword("method"));
}

#[test]
fn modern_keywords_case_sensitivity() {
    for kw in ["field", "class", "method"] {
        assert!(is_keyword(kw), "{kw} should be recognized");
        // Capitalized and uppercase variants must not match
        let capitalized = {
            let mut chars = kw.chars();
            match chars.next() {
                Some(c) => {
                    let mut s = c.to_uppercase().to_string();
                    s.extend(chars);
                    s
                }
                None => String::new(),
            }
        };
        let uppercased = kw.to_uppercase();
        assert!(!is_keyword(&capitalized), "{capitalized} should not be a keyword");
        assert!(!is_keyword(&uppercased), "{uppercased} should not be a keyword");
    }
}

// ---------------------------------------------------------------------------
// Declaration keywords: my, our, local, state — cross-list coverage
// ---------------------------------------------------------------------------

#[test]
fn our_cross_list_membership() {
    assert!(is_keyword("our"));
    assert!(is_lsp_completion_keyword("our"));
    assert!(is_dap_completion_keyword("our"));
    assert!(is_lsp_runtime_completion_keyword("our"));
    assert!(is_rename_keyword("our"));
    assert!(is_parser_lsp_keyword("our"));
    assert!(is_lexer_keyword("our"));
}

#[test]
fn state_cross_list_membership() {
    assert!(is_keyword("state"));
    assert!(is_lsp_completion_keyword("state"));
    // state is not in every list
    assert!(is_lsp_runtime_completion_keyword("state"));
    assert!(is_rename_keyword("state"));
    assert!(is_lexer_keyword("state"));
}

#[test]
fn local_cross_list_membership() {
    assert!(is_keyword("local"));
    assert!(is_lsp_completion_keyword("local"));
    assert!(is_dap_completion_keyword("local"));
    assert!(is_lsp_runtime_completion_keyword("local"));
    assert!(is_rename_keyword("local"));
    assert!(is_parser_lsp_keyword("local"));
    assert!(is_lexer_keyword("local"));
}

// ---------------------------------------------------------------------------
// Try/catch/finally — cross-list membership
// ---------------------------------------------------------------------------

#[test]
fn try_catch_finally_cross_list_membership() {
    // try, catch, finally are in KEYWORDS and LEXER_KEYWORDS
    for kw in ["try", "catch", "finally"] {
        assert!(is_keyword(kw), "{kw} should be in KEYWORDS");
        assert!(is_lexer_keyword(kw), "{kw} should be in LEXER_KEYWORDS");
    }
    // catch and finally are in LSP_COMPLETION_KEYWORDS (Perl 5.34+ try/catch)
    assert!(is_lsp_completion_keyword("catch"));
    assert!(is_lsp_completion_keyword("finally"));
    // try is not in LSP_COMPLETION_KEYWORDS (already handled by 'try' snippet)
    assert!(!is_lsp_completion_keyword("try"));
    // catch/finally not in other lists
    for kw in ["catch", "finally"] {
        assert!(!is_dap_completion_keyword(kw), "{kw} should not be in DAP_COMPLETION_KEYWORDS");
        assert!(
            !is_lsp_runtime_completion_keyword(kw),
            "{kw} should not be in LSP_RUNTIME_COMPLETION_KEYWORDS"
        );
        assert!(!is_rename_keyword(kw), "{kw} should not be in RENAME_KEYWORDS");
    }
}

// ---------------------------------------------------------------------------
// Control flow category completeness
// ---------------------------------------------------------------------------

#[test]
fn all_control_flow_keywords_in_parser_lsp() {
    // Control flow keywords should all appear in PARSER_LSP_KEYWORDS
    for kw in [
        "if", "elsif", "else", "unless", "while", "until", "for", "foreach", "last", "next",
        "redo", "return", "goto",
    ] {
        assert!(is_parser_lsp_keyword(kw), "control-flow {kw} should be in PARSER_LSP_KEYWORDS");
    }
}

#[test]
fn all_declaration_keywords_in_rename() {
    for kw in ["my", "our", "local", "state"] {
        assert!(is_rename_keyword(kw), "declaration {kw} should be in RENAME_KEYWORDS");
    }
}

// ---------------------------------------------------------------------------
// Non-keywords that resemble real Perl identifiers
// ---------------------------------------------------------------------------

#[test]
fn common_perl_module_names_are_not_keywords() {
    for name in [
        "strict", "warnings", "Carp", "Data", "Dumper", "File", "Path", "Moose", "Moo", "DBI",
        "CGI", "LWP", "JSON", "YAML", "Test", "Exporter",
    ] {
        assert!(!is_keyword(name), "module name {name} should not be a keyword");
    }
}

#[test]
fn common_perl_builtins_not_in_keywords_are_rejected() {
    // These are real Perl builtins that are NOT in the keyword list
    for name in [
        "chdir",
        "chmod",
        "chown",
        "chroot",
        "closedir",
        "crypt",
        "dbmclose",
        "dbmopen",
        "dump",
        "endgrent",
        "endhostent",
        "endnetent",
        "endprotoent",
        "endpwent",
        "endservent",
        "eof",
        "exec",
        "fcntl",
        "fileno",
        "flock",
        "fork",
    ] {
        assert!(!is_keyword(name), "builtin {name} is not in KEYWORDS (by design)");
    }
}

// ---------------------------------------------------------------------------
// Keyword entries all start with a letter or underscore
// ---------------------------------------------------------------------------

#[test]
fn all_keywords_start_with_letter_or_underscore() {
    for &kw in KEYWORDS {
        let first = kw.chars().next();
        assert!(
            first.is_some_and(|c| c.is_ascii_alphabetic() || c == '_'),
            "keyword {kw:?} should start with a letter or underscore"
        );
    }
}

// ---------------------------------------------------------------------------
// Exhaustive: every KEYWORDS entry is found by is_keyword
// ---------------------------------------------------------------------------

#[test]
fn every_keyword_entry_roundtrips_through_is_keyword() {
    let mut count = 0;
    for &kw in KEYWORDS {
        assert!(is_keyword(kw), "KEYWORDS entry {kw:?} not found by is_keyword");
        count += 1;
    }
    // Sanity: we actually iterated over a non-trivial number of entries
    assert!(count >= 120, "expected at least 120 keywords, got {count}");
}

// ---------------------------------------------------------------------------
// Exhaustive: non-members across all specialized lookup functions
// ---------------------------------------------------------------------------

#[test]
fn not_every_keyword_is_in_every_specialized_list() {
    // Some keywords appear in all specialized lists, but no specialized list
    // contains ALL keywords (they are proper subsets).
    // Verify that each specialized list is missing at least one KEYWORDS entry.
    let specialized: &[(&str, fn(&str) -> bool)] = &[
        ("LSP_COMPLETION", is_lsp_completion_keyword),
        ("DAP_COMPLETION", is_dap_completion_keyword),
        ("LSP_RUNTIME", is_lsp_runtime_completion_keyword),
        ("RENAME", is_rename_keyword),
        ("PARSER_LSP", is_parser_lsp_keyword),
        ("LEXER", is_lexer_keyword),
    ];
    for &(name, lookup) in specialized {
        let missing_count = KEYWORDS.iter().filter(|&&kw| !lookup(kw)).count();
        assert!(missing_count > 0, "{name} should not contain every KEYWORDS entry");
    }
}

// ---------------------------------------------------------------------------
// Specialized list sizes relative to each other
// ---------------------------------------------------------------------------

#[test]
fn lexer_keywords_larger_than_rename_keywords() {
    assert!(
        LEXER_KEYWORDS.len() > RENAME_KEYWORDS.len(),
        "LEXER_KEYWORDS ({}) should be larger than RENAME_KEYWORDS ({})",
        LEXER_KEYWORDS.len(),
        RENAME_KEYWORDS.len()
    );
}

#[test]
fn dap_keywords_larger_than_rename_keywords() {
    assert!(
        DAP_COMPLETION_KEYWORDS.len() > RENAME_KEYWORDS.len(),
        "DAP_COMPLETION_KEYWORDS ({}) should be larger than RENAME_KEYWORDS ({})",
        DAP_COMPLETION_KEYWORDS.len(),
        RENAME_KEYWORDS.len()
    );
}

// ---------------------------------------------------------------------------
// Keyword categorization: groups are disjoint where expected
// ---------------------------------------------------------------------------

#[test]
fn phase_blocks_are_disjoint_from_declaration_keywords() {
    let phase_blocks = ["BEGIN", "CHECK", "END", "INIT", "UNITCHECK"];
    let declarations = ["my", "our", "local", "state"];
    for pb in phase_blocks {
        for decl in &declarations {
            assert_ne!(pb, *decl, "phase blocks and declarations should be disjoint");
        }
    }
}

#[test]
fn comparison_operators_are_disjoint_from_control_flow() {
    let comparisons = ["cmp", "eq", "ge", "gt", "le", "lt", "ne"];
    let control_flow = [
        "if", "elsif", "else", "unless", "while", "until", "for", "foreach", "last", "next",
        "redo", "return", "goto",
    ];
    for comp in comparisons {
        assert!(!control_flow.contains(&comp), "{comp} should not be in control flow");
    }
}

// ---------------------------------------------------------------------------
// All KEYWORDS entries that start with uppercase are phase blocks or dunders
// ---------------------------------------------------------------------------

#[test]
fn uppercase_keywords_are_phase_blocks_or_dunders() {
    let phase_blocks = ["AUTOLOAD", "BEGIN", "CHECK", "DESTROY", "END", "INIT", "UNITCHECK"];
    for &kw in KEYWORDS {
        if kw.starts_with("__") {
            continue; // dunder tokens handled separately
        }
        let first_char = kw.chars().next();
        if first_char.is_some_and(|c| c.is_ascii_uppercase()) {
            assert!(
                phase_blocks.contains(&kw),
                "uppercase keyword {kw:?} should be a known phase block"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Substring and prefix/suffix non-matches
// ---------------------------------------------------------------------------

#[test]
fn keyword_substrings_are_not_keywords() {
    // "sub" is a keyword but its substrings/superstrings should not be
    assert!(is_keyword("sub"));
    assert!(!is_keyword("su"));
    assert!(!is_keyword("ub"));
    assert!(!is_keyword("subroutine"));
    assert!(!is_keyword("subprocess"));
}

#[test]
fn keyword_with_leading_trailing_underscores_rejected() {
    // Single underscores around keywords should not match
    assert!(!is_keyword("_my"));
    assert!(!is_keyword("my_"));
    assert!(!is_keyword("_if_"));
    assert!(!is_keyword("_sub"));
}

// ---------------------------------------------------------------------------
// Each specialized list has unique entries not in all other specialized lists
// ---------------------------------------------------------------------------

#[test]
fn each_specialized_list_has_unique_content() -> Result<(), String> {
    // Verify each specialized list is non-empty and its first entry is in KEYWORDS
    let lists: &[(&str, &[&str])] = &[
        ("LSP_COMPLETION", LSP_COMPLETION_KEYWORDS),
        ("DAP_COMPLETION", DAP_COMPLETION_KEYWORDS),
        ("LSP_RUNTIME", LSP_RUNTIME_COMPLETION_KEYWORDS),
        ("RENAME", RENAME_KEYWORDS),
        ("PARSER_LSP", PARSER_LSP_KEYWORDS),
        ("LEXER", LEXER_KEYWORDS),
    ];

    for &(name, list) in lists {
        let first = list.first().ok_or(format!("{name} is empty"))?;
        assert!(is_keyword(first), "{name} first entry should be in KEYWORDS");
    }

    // The lists differ in content — verify pairwise differences
    assert_ne!(LSP_COMPLETION_KEYWORDS, DAP_COMPLETION_KEYWORDS);
    assert_ne!(LSP_COMPLETION_KEYWORDS, RENAME_KEYWORDS);
    assert_ne!(DAP_COMPLETION_KEYWORDS, LEXER_KEYWORDS);
    assert_ne!(RENAME_KEYWORDS, PARSER_LSP_KEYWORDS);
    Ok(())
}
