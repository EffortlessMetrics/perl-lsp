//! File-local symbol table for bareword/regex disambiguation.
//!
//! Enables the lexer to correctly identify known subroutine names so that
//! `identifier /regex/` is lexed as a function call with a regex argument
//! rather than `identifier / expr` (division).
//!
//! # How it works
//!
//! Before lexing begins, the parser performs a lightweight pre-pass over the
//! source text to collect all `sub NAME` declarations into a `LocalSymbolTable`.
//! That table is then attached to the [`LexerConfig`][crate::LexerConfig] so
//! that the lexer can consult it when resolving bareword/slash ambiguity.
//!
//! # Limitations (v1)
//!
//! - ASCII identifiers only; Unicode sub names are not recognized.
//! - No imported symbols (workspace symbol table is a follow-up).
//! - Dynamic subs (`eval "sub foo { }"`, `AUTOLOAD`) are not tracked.

use std::collections::HashSet;

/// File-local subroutine symbol table built from a pre-pass scan.
///
/// # Forward references
///
/// Because this is a whole-file pre-pass, subroutines declared *after* their
/// first call site are recognized correctly.  A forward reference like:
///
/// ```text
/// builder /pattern/;
/// sub builder { ... }
/// ```
///
/// will be parsed as a function call with a regex argument.
#[derive(Debug, Clone, Default)]
pub struct LocalSymbolTable {
    known_subs: HashSet<Box<str>>,
}

impl LocalSymbolTable {
    /// Scan Perl source for `sub NAME` declarations and build a symbol table.
    ///
    /// This is a best-effort O(n) pre-pass. It skips line comments (`#…`) and
    /// simple single- and double-quoted string literals so that `# sub foo` or
    /// `"sub foo"` patterns do not produce false positives.  POD sections and
    /// heredocs are *not* specially handled; declarations inside them are a
    /// known (harmless) false-positive class.
    pub fn scan_subs(input: &str) -> Self {
        let mut known_subs = HashSet::new();
        let bytes = input.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            match bytes[i] {
                // Line comments: skip to end of line.
                b'#' => {
                    while i < len && bytes[i] != b'\n' {
                        i += 1;
                    }
                }

                // Single-quoted strings: skip contents verbatim (backslash only
                // escapes `\'` and `\\` inside `'...'`).
                b'\'' => {
                    i += 1;
                    while i < len {
                        if bytes[i] == b'\\' && i + 1 < len {
                            i += 2; // skip backslash + next char
                        } else if bytes[i] == b'\'' {
                            i += 1;
                            break;
                        } else {
                            i += 1;
                        }
                    }
                }

                // Double-quoted strings: skip contents (backslash escapes).
                b'"' => {
                    i += 1;
                    while i < len {
                        if bytes[i] == b'\\' && i + 1 < len {
                            i += 2;
                        } else if bytes[i] == b'"' {
                            i += 1;
                            break;
                        } else {
                            i += 1;
                        }
                    }
                }

                // Potential `sub` keyword.
                b's' if i + 3 <= len && bytes[i..i + 3] == *b"sub" => {
                    // Word-boundary guard: the char before must not be an ident char.
                    let prev_ident = i > 0 && is_ident_byte(bytes[i - 1]);
                    // The char immediately after "sub" must not be an ident char.
                    let next_ident = i + 3 < len && is_ident_byte(bytes[i + 3]);

                    if !prev_ident && !next_ident {
                        let mut j = i + 3;
                        // Skip horizontal whitespace and newlines.
                        while j < len && matches!(bytes[j], b' ' | b'\t' | b'\r' | b'\n') {
                            j += 1;
                        }
                        // Collect ASCII identifier (must start with letter or `_`).
                        if j < len && (bytes[j].is_ascii_alphabetic() || bytes[j] == b'_') {
                            let start = j;
                            while j < len && is_ident_byte(bytes[j]) {
                                j += 1;
                            }
                            // All bytes in start..j are ASCII, so this is valid UTF-8.
                            known_subs.insert(input[start..j].into());
                        }
                        i = j;
                    } else {
                        i += 1;
                    }
                }

                _ => {
                    i += 1;
                }
            }
        }

        Self { known_subs }
    }

    /// Return `true` if `name` was declared as a subroutine in this file.
    pub fn is_known_sub(&self, name: &str) -> bool {
        self.known_subs.contains(name)
    }

    /// Return the number of subroutine names recorded.
    pub fn len(&self) -> usize {
        self.known_subs.len()
    }

    /// Return `true` if no subroutines have been recorded.
    pub fn is_empty(&self) -> bool {
        self.known_subs.is_empty()
    }
}

/// Return `true` if byte `b` can appear inside a Perl identifier (`[A-Za-z0-9_]`).
fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::LocalSymbolTable;

    // --- scan_subs: basic recognition ---

    #[test]
    fn scan_subs_empty_source_gives_empty_table() {
        assert!(LocalSymbolTable::scan_subs("").is_empty());
    }

    #[test]
    fn scan_subs_single_declaration_is_recognized() {
        let st = LocalSymbolTable::scan_subs("sub foo { }");
        assert!(st.is_known_sub("foo"));
        assert_eq!(st.len(), 1);
    }

    #[test]
    fn scan_subs_multiple_declarations_are_all_recognized() {
        let src = "sub alpha { }\nsub beta { }\nsub gamma { }";
        let st = LocalSymbolTable::scan_subs(src);
        assert!(st.is_known_sub("alpha"));
        assert!(st.is_known_sub("beta"));
        assert!(st.is_known_sub("gamma"));
        assert_eq!(st.len(), 3);
    }

    #[test]
    fn scan_subs_forward_declaration_with_semicolon() {
        let st = LocalSymbolTable::scan_subs("sub builder;");
        assert!(st.is_known_sub("builder"));
    }

    #[test]
    fn scan_subs_prototype_declaration() {
        let st = LocalSymbolTable::scan_subs("sub transform ($$) { }");
        assert!(st.is_known_sub("transform"));
    }

    #[test]
    fn scan_subs_underscore_prefix() {
        let st = LocalSymbolTable::scan_subs("sub _private { }");
        assert!(st.is_known_sub("_private"));
    }

    #[test]
    fn scan_subs_alphanumeric_name_with_digits() {
        let st = LocalSymbolTable::scan_subs("sub process2 { }");
        assert!(st.is_known_sub("process2"));
    }

    // --- scan_subs: comment filtering ---

    #[test]
    fn scan_subs_skips_line_comment() {
        let src = "# sub commented_out { }\nsub real_sub { }";
        let st = LocalSymbolTable::scan_subs(src);
        assert!(!st.is_known_sub("commented_out"), "should not register commented-out subs");
        assert!(st.is_known_sub("real_sub"));
    }

    #[test]
    fn scan_subs_inline_comment_after_declaration() {
        let src = "sub real { } # sub fake { }";
        let st = LocalSymbolTable::scan_subs(src);
        assert!(st.is_known_sub("real"));
        assert!(!st.is_known_sub("fake"));
    }

    // --- scan_subs: string filtering ---

    #[test]
    fn scan_subs_skips_single_quoted_string() {
        let src = r#"my $x = 'sub in_string { }'; sub real { }"#;
        let st = LocalSymbolTable::scan_subs(src);
        assert!(!st.is_known_sub("in_string"), "should not register subs in string literals");
        assert!(st.is_known_sub("real"));
    }

    #[test]
    fn scan_subs_skips_double_quoted_string() {
        let src = r#"my $x = "sub in_string { }"; sub real { }"#;
        let st = LocalSymbolTable::scan_subs(src);
        assert!(!st.is_known_sub("in_string"));
        assert!(st.is_known_sub("real"));
    }

    // --- scan_subs: word-boundary checks ---

    #[test]
    fn scan_subs_does_not_match_substring() {
        // "substring" contains "sub" but is NOT a sub declaration
        let src = "my $x = substring(1, 2);";
        let st = LocalSymbolTable::scan_subs(src);
        assert!(!st.is_known_sub("string"), "should not match 'sub' inside 'substring'");
        assert!(st.is_empty());
    }

    #[test]
    fn scan_subs_does_not_match_sub_suffix() {
        let src = "my $x = foosubfoo;";
        let st = LocalSymbolTable::scan_subs(src);
        assert!(st.is_empty());
    }

    // --- is_known_sub: negative cases ---

    #[test]
    fn is_known_sub_unknown_name_returns_false() {
        let st = LocalSymbolTable::scan_subs("sub foo { }");
        assert!(!st.is_known_sub("bar"));
        assert!(!st.is_known_sub(""));
    }

    // --- default ---

    #[test]
    fn default_produces_empty_table() {
        let st = LocalSymbolTable::default();
        assert!(st.is_empty());
        assert!(!st.is_known_sub("anything"));
    }
}
