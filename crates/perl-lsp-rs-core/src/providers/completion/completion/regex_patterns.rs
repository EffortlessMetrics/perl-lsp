//! Regex pattern completion for Perl
//!
//! Provides completion suggestions for common regex constructs when the cursor
//! is inside a regex literal (`/…/`, `m/…/`, `qr/…/`, `s/…/…/`).

use super::items::CompletionItemKind;
use super::{context::CompletionContext, items::CompletionItem};

/// A single regex completion suggestion.
struct RegexSuggestion {
    label: &'static str,
    insert: &'static str,
    detail: &'static str,
    doc: &'static str,
    sort_key: &'static str,
}

/// All regex construct suggestions grouped by category.
fn regex_suggestions() -> &'static [RegexSuggestion] {
    &[
        // ── Character classes ──────────────────────────────────────────
        RegexSuggestion {
            label: "\\d",
            insert: "\\d",
            detail: "character class",
            doc: "Match a digit character [0-9]",
            sort_key: "0_charclass_d",
        },
        RegexSuggestion {
            label: "\\w",
            insert: "\\w",
            detail: "character class",
            doc: "Match a word character [a-zA-Z0-9_]",
            sort_key: "0_charclass_w",
        },
        RegexSuggestion {
            label: "\\s",
            insert: "\\s",
            detail: "character class",
            doc: "Match a whitespace character",
            sort_key: "0_charclass_s",
        },
        RegexSuggestion {
            label: "\\D",
            insert: "\\D",
            detail: "character class",
            doc: "Match a non-digit character",
            sort_key: "0_charclass_D",
        },
        RegexSuggestion {
            label: "\\W",
            insert: "\\W",
            detail: "character class",
            doc: "Match a non-word character",
            sort_key: "0_charclass_W",
        },
        RegexSuggestion {
            label: "\\S",
            insert: "\\S",
            detail: "character class",
            doc: "Match a non-whitespace character",
            sort_key: "0_charclass_S",
        },
        RegexSuggestion {
            label: "\\h",
            insert: "\\h",
            detail: "character class",
            doc: "Match a horizontal whitespace character",
            sort_key: "0_charclass_h",
        },
        RegexSuggestion {
            label: "\\H",
            insert: "\\H",
            detail: "character class",
            doc: "Match a non-horizontal whitespace character",
            sort_key: "0_charclass_H",
        },
        RegexSuggestion {
            label: "\\v",
            insert: "\\v",
            detail: "character class",
            doc: "Match a vertical whitespace character",
            sort_key: "0_charclass_v",
        },
        RegexSuggestion {
            label: "\\V",
            insert: "\\V",
            detail: "character class",
            doc: "Match a non-vertical whitespace character",
            sort_key: "0_charclass_V",
        },
        RegexSuggestion {
            label: "\\R",
            insert: "\\R",
            detail: "character class",
            doc: "Match a Unicode linebreak sequence",
            sort_key: "0_charclass_R",
        },
        RegexSuggestion {
            label: "[...]",
            insert: "[${1}]",
            detail: "character class",
            doc: "Custom character class",
            sort_key: "0_charclass_custom",
        },
        RegexSuggestion {
            label: "[^...]",
            insert: "[^${1}]",
            detail: "character class",
            doc: "Negated character class",
            sort_key: "0_charclass_negated",
        },
        // ── Anchors ───────────────────────────────────────────────────
        RegexSuggestion {
            label: "^",
            insert: "^",
            detail: "anchor",
            doc: "Match start of string (or line in /m mode)",
            sort_key: "1_anchor_caret",
        },
        RegexSuggestion {
            label: "$",
            insert: "$",
            detail: "anchor",
            doc: "Match end of string (or line in /m mode)",
            sort_key: "1_anchor_dollar",
        },
        RegexSuggestion {
            label: "\\b",
            insert: "\\b",
            detail: "anchor",
            doc: "Match word boundary",
            sort_key: "1_anchor_b",
        },
        RegexSuggestion {
            label: "\\B",
            insert: "\\B",
            detail: "anchor",
            doc: "Match non-word boundary",
            sort_key: "1_anchor_B",
        },
        RegexSuggestion {
            label: "\\A",
            insert: "\\A",
            detail: "anchor",
            doc: "Match absolute start of string",
            sort_key: "1_anchor_A",
        },
        RegexSuggestion {
            label: "\\z",
            insert: "\\z",
            detail: "anchor",
            doc: "Match absolute end of string",
            sort_key: "1_anchor_z",
        },
        RegexSuggestion {
            label: "\\Z",
            insert: "\\Z",
            detail: "anchor",
            doc: "Match end of string (before optional final newline)",
            sort_key: "1_anchor_Z",
        },
        // ── Quantifiers ───────────────────────────────────────────────
        RegexSuggestion {
            label: "*",
            insert: "*",
            detail: "quantifier",
            doc: "Match zero or more times (greedy)",
            sort_key: "2_quant_star",
        },
        RegexSuggestion {
            label: "+",
            insert: "+",
            detail: "quantifier",
            doc: "Match one or more times (greedy)",
            sort_key: "2_quant_plus",
        },
        RegexSuggestion {
            label: "?",
            insert: "?",
            detail: "quantifier",
            doc: "Match zero or one time",
            sort_key: "2_quant_question",
        },
        RegexSuggestion {
            label: "{n}",
            insert: "{${1:n}}",
            detail: "quantifier",
            doc: "Match exactly n times",
            sort_key: "2_quant_exact",
        },
        RegexSuggestion {
            label: "{n,}",
            insert: "{${1:n},}",
            detail: "quantifier",
            doc: "Match n or more times",
            sort_key: "2_quant_min",
        },
        RegexSuggestion {
            label: "{n,m}",
            insert: "{${1:n},${2:m}}",
            detail: "quantifier",
            doc: "Match between n and m times",
            sort_key: "2_quant_range",
        },
        // ── Groups ────────────────────────────────────────────────────
        RegexSuggestion {
            label: "(...)",
            insert: "(${1})",
            detail: "group",
            doc: "Capturing group",
            sort_key: "3_group_capture",
        },
        RegexSuggestion {
            label: "(?:...)",
            insert: "(?:${1})",
            detail: "group",
            doc: "Non-capturing group",
            sort_key: "3_group_noncapture",
        },
        RegexSuggestion {
            label: "(?=...)",
            insert: "(?=${1})",
            detail: "group",
            doc: "Positive lookahead",
            sort_key: "3_group_lookahead",
        },
        RegexSuggestion {
            label: "(?!...)",
            insert: "(?!${1})",
            detail: "group",
            doc: "Negative lookahead",
            sort_key: "3_group_neg_lookahead",
        },
        RegexSuggestion {
            label: "(?<=...)",
            insert: "(?<=${1})",
            detail: "group",
            doc: "Positive lookbehind",
            sort_key: "3_group_lookbehind",
        },
        RegexSuggestion {
            label: "(?<!...)",
            insert: "(?<!${1})",
            detail: "group",
            doc: "Negative lookbehind",
            sort_key: "3_group_neg_lookbehind",
        },
        RegexSuggestion {
            label: "(?<name>...)",
            insert: "(?<${1:name}>${2})",
            detail: "group",
            doc: "Named capture group (Perl 5.10+). Capture is available as $+{name}.",
            sort_key: "3_group_named_capture",
        },
        // ── Common patterns ───────────────────────────────────────────
        RegexSuggestion {
            label: "\\d+",
            insert: "\\d+",
            detail: "common pattern",
            doc: "One or more digits",
            sort_key: "4_pattern_digits",
        },
        RegexSuggestion {
            label: "\\w+",
            insert: "\\w+",
            detail: "common pattern",
            doc: "One or more word characters",
            sort_key: "4_pattern_word",
        },
        RegexSuggestion {
            label: "\\s+",
            insert: "\\s+",
            detail: "common pattern",
            doc: "One or more whitespace characters",
            sort_key: "4_pattern_space",
        },
        RegexSuggestion {
            label: ".*?",
            insert: ".*?",
            detail: "common pattern",
            doc: "Non-greedy match of any characters",
            sort_key: "4_pattern_nongreedy_any",
        },
        RegexSuggestion {
            label: ".+?",
            insert: ".+?",
            detail: "common pattern",
            doc: "Non-greedy match of one or more characters",
            sort_key: "4_pattern_nongreedy_plus",
        },
    ]
}

/// Add regex construct completions when the cursor is inside a regex literal.
///
/// Completions are filtered by the prefix text already typed inside the regex.
/// For example, if the user typed `\\` inside a regex, only escape sequences
/// (like `\\d`, `\\w`, `\\s`) are suggested.
fn find_regex_prefix_start(line_prefix: &str) -> Option<usize> {
    let mut starts: Vec<usize> = line_prefix.char_indices().map(|(idx, _)| idx).collect();
    starts.push(line_prefix.len());

    for start in starts {
        let candidate = &line_prefix[start..];
        if candidate.is_empty() {
            continue;
        }
        if regex_suggestions().iter().any(|suggestion| suggestion.label.starts_with(candidate)) {
            return Some(start);
        }
    }

    None
}

/// Add regex flag completions when the cursor is positioned after the closing
/// delimiter of a regex literal (e.g., `$x =~ /foo/|` or `m/foo/i|`).
///
/// Flags are operator-aware:
/// - `tr`/`y` operators accept only `c`, `d`, `s`.
/// - All other operators (`m`, `s`, `qr`, bare `/`) accept the standard
///   Perl regex flag set.
///
/// Already-typed flag characters are excluded from suggestions.
///
/// Non-`/` delimiters (`m{...}`, `m!...!`, etc.) are not detected by
/// `is_in_regex_flags` and are out of scope for this implementation.
pub fn add_regex_flag_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
) {
    let flag_chars: &[char] = &['g', 'i', 'm', 's', 'x', 'e', 'r', 'a', 'd', 'u', 'p', 'l', 'c'];
    let before = &source[..context.position];
    let without_flags = before.trim_end_matches(|c: char| flag_chars.contains(&c));
    let already_typed: &str = &before[without_flags.len()..];

    // Detect whether the operator is tr/y. Walk the whitespace-separated tokens
    // looking for one that is exactly `tr` or `y` followed by `/...`, or a
    // combined form like `tr/a-z/A-Z` (the whole first slash-delimited segment).
    // This handles bare `tr/...`, `y/...`, and binding forms `$x =~ tr/...`.
    let before_close = without_flags.trim_end_matches('/');
    let is_tr = before_close.split_whitespace().any(|token| {
        token == "tr" || token == "y" || token.starts_with("tr/") || token.starts_with("y/")
    });

    let flags: &[(&str, &str)] = if is_tr {
        &[
            ("c", "complement the search list"),
            ("d", "delete characters not in replacement list"),
            ("s", "squash duplicate replaced characters"),
        ]
    } else {
        &[
            ("g", "global — match/substitute all occurrences"),
            ("i", "case-insensitive match"),
            ("m", "multi-line — ^ and $ match line boundaries"),
            ("s", "single-line — . matches newline"),
            ("x", "extended — allow whitespace and comments in pattern"),
            ("e", "evaluate replacement as Perl expression (s/// only)"),
            ("r", "return modified copy, don't modify original (5.14+)"),
            ("a", "restrict \\d, \\s, \\w to ASCII"),
            ("p", "preserve pre/post match strings in $`, $&, $'"),
        ]
    };

    for (flag, doc) in flags {
        if already_typed.contains(flag) {
            continue; // skip already-used flags
        }
        completions.push(CompletionItem {
            label: flag.to_string(),
            kind: CompletionItemKind::Keyword,
            detail: Some("regex flag".to_string()),
            documentation: Some(doc.to_string()),
            insert_text: Some(flag.to_string()),
            sort_text: Some(format!("5_flag_{flag}")),
            filter_text: Some(flag.to_string()),
            additional_edits: vec![],
            text_edit_range: Some((context.position, context.position)),
            commit_characters: None,
        });
    }
}

pub fn add_regex_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
) {
    let line_start = source[..context.position].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let line_prefix = &source[line_start..context.position];
    let regex_prefix_start = find_regex_prefix_start(line_prefix);
    let (prefix, replace_start) = match regex_prefix_start {
        Some(rel_start) => (&line_prefix[rel_start..], line_start + rel_start),
        None if context.prefix.is_empty() => ("", context.position),
        None => return,
    };

    for suggestion in regex_suggestions() {
        if prefix.is_empty() || suggestion.label.starts_with(prefix) {
            completions.push(CompletionItem {
                label: suggestion.label.to_string(),
                kind: CompletionItemKind::Snippet,
                detail: Some(format!("regex {}", suggestion.detail)),
                documentation: Some(suggestion.doc.to_string()),
                insert_text: Some(suggestion.insert.to_string()),
                sort_text: Some(suggestion.sort_key.to_string()),
                filter_text: Some(suggestion.label.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((replace_start, context.position)),
                commit_characters: None,
            });
        }
    }
}
