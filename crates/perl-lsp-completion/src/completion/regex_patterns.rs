//! Regex pattern completion for Perl
//!
//! Provides completion suggestions for common regex constructs when the cursor
//! is inside a regex literal (`/…/`, `m/…/`, `qr/…/`, `s/…/…/`).

use super::{context::CompletionContext, items::CompletionItem};
use crate::completion::items::CompletionItemKind;

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
pub fn add_regex_completions(completions: &mut Vec<CompletionItem>, context: &CompletionContext) {
    let prefix = &context.prefix;

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
                text_edit_range: Some((context.prefix_start, context.position)),
            });
        }
    }
}
