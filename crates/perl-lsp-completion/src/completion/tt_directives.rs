//! Template Toolkit directive and filter completions.
//!
//! Provides snippet completions for TT directives (FOREACH, IF, INCLUDE, …)
//! and filter names (html, uri, upper, …) when the cursor is inside a
//! `[% … %]` block in a `.tt` or `.tt2` file.

use super::{context::CompletionContext, items::CompletionItem, items::CompletionItemKind};

// ─── Directives ──────────────────────────────────────────────────────────────

/// Block directives: require a matching `END`.
/// Each entry is `(label, snippet, documentation)`.
pub const TT_BLOCK_DIRECTIVES: &[(&str, &str, &str)] = &[
    ("FOREACH", "FOREACH ${1:item} IN ${2:list}\n    ${0}\nEND", "Iterate over a list"),
    ("FOR", "FOR ${1:item} IN ${2:list}\n    ${0}\nEND", "Iterate over a list (alias for FOREACH)"),
    ("IF", "IF ${1:condition}\n    ${0}\nEND", "Conditional block"),
    ("UNLESS", "UNLESS ${1:condition}\n    ${0}\nEND", "Negated conditional block"),
    ("WHILE", "WHILE ${1:condition}\n    ${0}\nEND", "While loop"),
    ("SWITCH", "SWITCH ${1:var}\n    CASE ${2:val}\n    ${0}\nEND", "Switch/case block"),
    ("BLOCK", "BLOCK ${1:name}\n    ${0}\nEND", "Named block definition"),
    ("FILTER", "FILTER ${1:name}\n    ${0}\nEND", "Apply filter to block content"),
    ("TRY", "TRY\n    ${0}\nCATCH\nEND", "Exception-handling block"),
    ("RAWPERL", "RAWPERL\n    ${0}\nEND", "Embedded raw Perl (requires EVAL_PERL)"),
    ("PERL", "PERL\n    ${0}\nEND", "Embedded Perl code (requires EVAL_PERL)"),
    ("WRAPPER", "WRAPPER '${1:template.tt}'\n    ${0}\nEND", "Wrap content in a template"),
    ("MACRO", "MACRO ${1:name}(${2:args}) BLOCK\n    ${0}\nEND", "Define a reusable macro"),
];

/// Inline directives: no `END` required.
pub const TT_INLINE_DIRECTIVES: &[(&str, &str, &str)] = &[
    ("INCLUDE", "INCLUDE '${1:template.tt}'", "Include a template file"),
    ("PROCESS", "PROCESS '${1:template.tt}'", "Process a template file"),
    ("INSERT", "INSERT '${1:file.txt}'", "Insert raw file content"),
    ("SET", "SET ${1:var} = ${2:value}", "Assign a variable"),
    ("DEFAULT", "DEFAULT ${1:var} = ${2:value}", "Assign if not already set"),
    ("GET", "GET ${1:expr}", "Evaluate and output an expression"),
    ("CALL", "CALL ${1:expr}", "Evaluate without producing output"),
    ("THROW", "THROW ${1:type} '${2:message}'", "Throw an exception"),
    ("USE", "USE ${1:Plugin}", "Load a TT plugin"),
    ("STOP", "STOP", "Stop processing the current template"),
    ("CLEAR", "CLEAR", "Clear the output buffer"),
    ("RETURN", "RETURN", "Return from the current block"),
    ("NEXT", "NEXT", "Skip to next iteration (in FOREACH)"),
    ("LAST", "LAST", "End loop immediately (in FOREACH)"),
    ("BREAK", "BREAK", "Alias for LAST"),
    ("END", "END", "End a block directive"),
    ("ELSIF", "ELSIF ${1:condition}", "Else-if branch"),
    ("ELSE", "ELSE", "Else branch"),
    ("CASE", "CASE ${1:value}", "Case branch in SWITCH"),
    ("CATCH", "CATCH ${1:type}", "Catch block in TRY"),
    ("FINAL", "FINAL", "Final block in TRY (always executed)"),
    ("META", "META ${1:key} = '${2:value}'", "Template metadata"),
    (
        "TAGS",
        "TAGS ${1:html}",
        "Change tag delimiters (e.g. html, metatext, template, asp, php, mason)",
    ),
    ("DEBUG", "DEBUG ${1:on}", "Enable/disable debug output"),
];

// ─── Loop variables ───────────────────────────────────────────────────────────

/// Special loop variables available inside `FOREACH` blocks.
pub const TT_LOOP_VARS: &[(&str, &str)] = &[
    ("loop.count", "Current iteration count (1-based)"),
    ("loop.index", "Current iteration index (0-based)"),
    ("loop.first", "True on the first iteration"),
    ("loop.last", "True on the last iteration"),
    ("loop.size", "Total number of items"),
    ("loop.max", "Index of the last item (size - 1)"),
    ("loop.prev", "Previous item"),
    ("loop.next", "Next item"),
    ("loop.parity", "'odd' or 'even' string"),
    ("loop.odd", "True on odd iterations"),
    ("loop.even", "True on even iterations"),
];

// ─── Filters ─────────────────────────────────────────────────────────────────

/// Scalar filters used after the `|` pipe operator.
pub const TT_FILTERS: &[(&str, &str)] = &[
    ("html", "Escape HTML special characters"),
    ("html_entity", "Encode as HTML entities"),
    ("html_para", "Wrap paragraphs in <p> tags"),
    ("html_break", "Replace newlines with <br>"),
    ("html_line_break", "Replace newlines with <br /> (strict XHTML)"),
    ("xml", "Escape XML/HTML plus apostrophes"),
    ("uri", "URI-encode the value"),
    ("url", "URL-encode the value (less aggressive than uri)"),
    ("upper", "Convert to uppercase"),
    ("lower", "Convert to lowercase"),
    ("ucfirst", "Uppercase the first character"),
    ("lcfirst", "Lowercase the first character"),
    ("trim", "Trim leading and trailing whitespace"),
    ("collapse", "Collapse whitespace runs to a single space"),
    ("truncate", "Truncate to N characters"),
    ("repeat", "Repeat the value N times"),
    ("replace", "Replace a pattern with a string"),
    ("remove", "Remove a pattern"),
    ("null", "Discard output"),
    ("indent", "Indent lines by N spaces"),
    ("format", "Apply sprintf-style format"),
    ("redirect", "Redirect output to a file"),
    ("stderr", "Output to STDERR"),
    ("stdout", "Output to STDOUT"),
    ("eval", "Evaluate block as template text"),
    ("evaltt", "Evaluate block as template text (alias for eval)"),
    ("perl", "Evaluate block as Perl code (requires EVAL_PERL)"),
    ("evalperl", "Evaluate block as Perl code (alias for perl)"),
];

// ─── Public completion functions ─────────────────────────────────────────────

/// Add TT directive completions when the cursor is inside a `[% … %]` block.
pub fn add_tt_directive_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
) {
    let prefix_upper = context.prefix.to_ascii_uppercase();

    for (name, snippet, doc) in TT_BLOCK_DIRECTIVES.iter().chain(TT_INLINE_DIRECTIVES.iter()) {
        if context.prefix.is_empty() || name.starts_with(prefix_upper.as_str()) {
            completions.push(CompletionItem {
                label: name.to_string(),
                kind: CompletionItemKind::Keyword,
                detail: Some("TT directive".to_string()),
                documentation: Some(doc.to_string()),
                insert_text: Some(snippet.to_string()),
                sort_text: Some(format!("0_{name}")),
                filter_text: Some(name.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
                commit_characters: None,
            });
        }
    }

    // Loop variable completions (e.g. `loop.count`, `loop.first`)
    for (var, doc) in TT_LOOP_VARS {
        if context.prefix.is_empty() || var.starts_with(context.prefix.as_str()) {
            completions.push(CompletionItem {
                label: var.to_string(),
                kind: CompletionItemKind::Variable,
                detail: Some("TT loop variable".to_string()),
                documentation: Some(doc.to_string()),
                insert_text: Some(var.to_string()),
                sort_text: Some(format!("1_{var}")),
                filter_text: Some(var.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
                commit_characters: None,
            });
        }
    }
}

/// Add TT filter completions when the cursor follows a `|` pipe inside a directive.
pub fn add_tt_filter_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
) {
    for (name, doc) in TT_FILTERS {
        if context.prefix.is_empty() || name.starts_with(context.prefix.as_str()) {
            completions.push(CompletionItem {
                label: name.to_string(),
                kind: CompletionItemKind::Function,
                detail: Some("TT filter".to_string()),
                documentation: Some(doc.to_string()),
                insert_text: Some(name.to_string()),
                sort_text: Some(format!("0_{name}")),
                filter_text: Some(name.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
                commit_characters: None,
            });
        }
    }
}
