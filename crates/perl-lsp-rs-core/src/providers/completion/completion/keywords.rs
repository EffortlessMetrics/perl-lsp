//! Keyword completion for Perl
//!
//! Provides completion for Perl keywords with snippet expansion.
//!
//! Sort tier: 5_ — keywords come after core builtins (3_) and workspace
//! symbols (4_) because they are always available and match via snippet
//! expansion. Users typing a partial identifier are usually looking for their
//! own symbols or a builtin before a keyword.

use super::{context::CompletionContext, items::CompletionItem};
use perl_lexer::LSP_COMPLETION_KEYWORDS;

/// Canonical Perl keywords for completion.
#[must_use]
pub fn keywords() -> &'static [&'static str] {
    LSP_COMPLETION_KEYWORDS
}

/// Return a brief documentation string for a Perl keyword.
fn keyword_doc(keyword: &str) -> Option<&'static str> {
    match keyword {
        "sub" => Some("Declare a named or anonymous subroutine. Usage: sub name { BLOCK }"),
        "if" => Some("Conditional execution. Usage: if (CONDITION) { BLOCK } elsif { } else { }"),
        "elsif" => Some("Additional condition branch in an if/elsif chain."),
        "else" => Some("Default branch executed when no if/elsif condition is true."),
        "unless" => Some("Execute BLOCK when CONDITION is false. Opposite of 'if'."),
        "while" => Some("Loop while CONDITION is true. Usage: while (CONDITION) { BLOCK }"),
        "until" => Some("Loop until CONDITION becomes true. Opposite of 'while'."),
        "for" => Some("C-style for loop. Usage: for (INIT; CONDITION; STEP) { BLOCK }"),
        "foreach" => Some("Iterate over a list. Usage: foreach my $item (@list) { BLOCK }"),
        "do" => Some("Execute a BLOCK or file. Usage: do { BLOCK } or do FILE"),
        "package" => {
            Some("Declare a package (namespace). Usage: package Name; or package Name { BLOCK }")
        }
        "use" => {
            Some("Load a module at compile time and import symbols. Usage: use Module qw(sym)")
        }
        "no" => Some("Unimport a module's symbols. Usage: no warnings 'experimental'"),
        "require" => Some("Load a module at runtime. Usage: require Module or require 'file.pl'"),
        "return" => Some("Return from a subroutine with an optional value."),
        "my" => Some("Declare a lexically-scoped variable. Usage: my $var = VALUE"),
        "our" => Some("Declare a package-scoped variable. Usage: our $VAR"),
        "local" => Some("Temporarily change a global variable's value. Usage: local $var = VALUE"),
        "next" => Some("Skip to the next iteration of the enclosing loop."),
        "last" => Some("Exit the innermost enclosing loop immediately."),
        "redo" => Some("Restart the current loop iteration without re-evaluating the condition."),
        "given" => Some("Experimental switch statement. Requires 'use feature :5.10'."),
        "when" => Some("Experimental case arm inside 'given'. Requires 'use feature :5.10'."),
        "default" => {
            Some("Experimental default case inside 'given'. Requires 'use feature :5.10'.")
        }
        "and" => Some("Low-precedence logical AND. Same as '&&' but lower precedence."),
        "or" => Some("Low-precedence logical OR. Same as '||' but lower precedence."),
        "not" => Some("Low-precedence logical NOT. Same as '!' but lower precedence."),
        "xor" => Some("Low-precedence logical exclusive OR."),
        "eq" => Some("String equality comparison. Returns true if strings are equal."),
        "ne" => Some("String inequality comparison. Returns true if strings differ."),
        "lt" => Some("String less-than comparison."),
        "gt" => Some("String greater-than comparison."),
        "le" => Some("String less-than-or-equal comparison."),
        "ge" => Some("String greater-than-or-equal comparison."),
        "cmp" => Some("String comparison returning -1, 0, or 1."),
        "x" => Some("Repetition operator. List: (LIST) x N. String: EXPR x N"),
        "print" => Some("Print a list to a filehandle. Usage: print FILEHANDLE LIST"),
        "say" => Some("Like print but appends a newline. Requires 'use feature :5.10'."),
        "chomp" => Some("Remove trailing newline from a string or list."),
        _ => None,
    }
}

/// Add keyword completions
pub fn add_keyword_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    keywords: &[&'static str],
) {
    for &keyword in keywords {
        if keyword.starts_with(&context.prefix) {
            let (insert_text, snippet) = match keyword {
                "sub" => ("sub ${1:name} {\n    $0\n}", true),
                "if" => ("if ($1) {\n    $0\n}", true),
                "elsif" => ("elsif ($1) {\n    $0\n}", true),
                "else" => ("else {\n    $0\n}", true),
                "unless" => ("unless ($1) {\n    $0\n}", true),
                "while" => ("while ($1) {\n    $0\n}", true),
                "for" => ("for (my $i = 0; $i < $1; $i++) {\n    $0\n}", true),
                "foreach" => ("foreach my $${1:item} (@${2:array}) {\n    $0\n}", true),
                "package" => ("package ${1:Name};\n\n$0", true),
                "use" => ("use ${1:Module};\n$0", true),
                _ => (keyword, false),
            };

            completions.push(CompletionItem {
                label: keyword.to_string(),
                kind: if snippet {
                    super::items::CompletionItemKind::Snippet
                } else {
                    super::items::CompletionItemKind::Keyword
                },
                detail: Some("keyword".to_string()),
                documentation: keyword_doc(keyword).map(str::to_string),
                insert_text: Some(insert_text.to_string()),
                // Tier 5: keywords sort after special vars (0_), user vars (1_),
                // user funcs (2_), core builtins (3_), and workspace symbols (4_).
                sort_text: Some(format!("5_{}", keyword)),
                filter_text: Some(keyword.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
                commit_characters: None,
            });
        }
    }
}
