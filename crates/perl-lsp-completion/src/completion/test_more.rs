//! Test::More function completions for Perl
//!
//! Provides completion for Test::More functions in test contexts.

use super::{context::CompletionContext, items::CompletionItem};

/// Test::More function completions
pub const TEST_MORE_EXPORTS: &[(&str, &str, &str)] = &[
    ("ok", "ok(${1:condition}, ${2:name});", "Test condition is true"),
    ("is", "is(${1:got}, ${2:expected}, ${3:name});", "Test values are equal"),
    ("isnt", "isnt(${1:got}, ${2:not_expected}, ${3:name});", "Test values are not equal"),
    ("like", "like(${1:got}, ${2:qr/.../}, ${3:name});", "Test string matches regex"),
    ("unlike", "unlike(${1:got}, ${2:qr/.../}, ${3:name});", "Test string doesn't match regex"),
    ("cmp_ok", "cmp_ok(${1:got}, '${2:op}', ${3:expected}, ${4:name});", "Compare using operator"),
    ("isa_ok", "isa_ok(${1:ref}, '${2:class}', ${3:name});", "Test object is of class"),
    ("can_ok", "can_ok(${1:class_or_obj}, ${2:@methods});", "Test object/class can do methods"),
    ("pass", "pass(${1:name});", "Unconditionally pass test"),
    ("fail", "fail(${1:name});", "Unconditionally fail test"),
    ("diag", "diag(${1:message});", "Print diagnostic message"),
    ("note", "note(${1:message});", "Print note message"),
    ("explain", "explain(${1:\\$ref});", "Dump data structure"),
    ("skip", "skip(${1:why}, ${2:how_many});", "Skip tests"),
    ("todo_skip", "todo_skip(${1:why}, ${2:how_many});", "Mark tests as TODO"),
    ("BAIL_OUT", "BAIL_OUT(${1:reason});", "Stop all testing"),
    ("subtest", "subtest '${1:name}' => sub {\n    ${0}\n};", "Run a subtest"),
    ("done_testing", "done_testing(${1:tests});", "Finish testing"),
    ("plan", "plan tests => ${1:num};", "Declare test plan"),
    ("use_ok", "use_ok('${1:Module}');", "Test module loads"),
    ("require_ok", "require_ok('${1:Module}');", "Test module requires"),
    (
        "is_deeply",
        "is_deeply(${1:\\$got}, ${2:\\$expected}, ${3:name});",
        "Deep structure comparison",
    ),
    ("new_ok", "new_ok('${1:Class}', [${2:args}], ${3:name});", "Test object creation"),
];

/// Add Test::More completions
pub fn add_test_more_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
) {
    for (name, snippet, doc) in TEST_MORE_EXPORTS {
        if context.prefix.is_empty() || name.starts_with(&context.prefix) {
            completions.push(CompletionItem {
                label: name.to_string(),
                kind: crate::completion::items::CompletionItemKind::Function,
                detail: Some("Test::More".to_string()),
                documentation: Some(doc.to_string()),
                insert_text: Some(snippet.to_string()),
                sort_text: Some(format!("2_{}", name)),
                filter_text: Some(name.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
            });
        }
    }
}
