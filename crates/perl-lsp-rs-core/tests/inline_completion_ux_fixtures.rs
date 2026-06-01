//! Fixture-style UX coverage for deterministic inline completion.
//!
//! These tests keep the visible ghost-text contract readable: the provider
//! should suggest project/style-aware text where it has context and stay silent
//! in zones where automatic ghost text would be noisy or unsafe.

use std::error::Error;

use lsp_types::{Position, Range};
use perl_lsp_rs_core::providers::inline_completion::{
    InlineCompletionItem, InlineCompletionList, InlineCompletionProvider,
};
use perl_parser_core::position::{offset_to_utf16_line_col, utf16_line_col_to_offset};

type TestResult = Result<(), Box<dyn Error>>;

const CURSOR: &str = "<<CURSOR>>";

struct InlineCompletionScenario {
    text: String,
    line: u32,
    character: u32,
}

impl InlineCompletionScenario {
    fn from_fixture(fixture: &str) -> Result<Self, String> {
        let byte = fixture
            .find(CURSOR)
            .ok_or_else(|| "fixture must include <<CURSOR>> marker".to_string())?;
        let text = fixture.replacen(CURSOR, "", 1);
        let (line, character) = offset_to_utf16_line_col(&text, byte);

        Ok(Self { text, line, character })
    }

    fn completions(&self) -> InlineCompletionList {
        InlineCompletionProvider::new().get_inline_completions(
            self.text.as_str(),
            self.line,
            self.character,
        )
    }
}

struct SuggestionFixture {
    name: &'static str,
    source: &'static str,
    first: Option<&'static str>,
    expected: &'static [&'static str],
    not_expected: &'static [&'static str],
}

struct SilentFixture {
    name: &'static str,
    source: &'static str,
}

struct RangeFixture {
    name: &'static str,
    source: &'static str,
    insert_text: &'static str,
    replaces: &'static str,
}

struct AcceptedEditFixture {
    name: &'static str,
    source: &'static str,
    expected_first: &'static str,
    expected_after: &'static str,
}

#[test]
fn inline_completion_fixture_corpus_returns_expected_ghost_text() -> TestResult {
    let fixtures = [
        SuggestionFixture {
            name: "use_pragmas",
            source: "use <<CURSOR>>",
            first: Some("strict;"),
            expected: &["strict;", "warnings;"],
            not_expected: &["done_testing();"],
        },
        SuggestionFixture {
            name: "use_pragmas_after_format_terminator",
            source: "format STDOUT =\n@<<<<\n$name\n.\nuse <<CURSOR>>",
            first: Some("strict;"),
            expected: &["strict;", "warnings;"],
            not_expected: &["done_testing();"],
        },
        SuggestionFixture {
            name: "test_more_assertion_prefers_visible_actual_expected",
            source: "use Test::More;\n\nmy $got = compute();\nmy $expected = 42;\n\n<<CURSOR>>",
            first: Some("is($got, $expected, 'test description');"),
            expected: &["is($got, $expected, 'test description');"],
            not_expected: &["done_testing();"],
        },
        SuggestionFixture {
            name: "test2_assertion_uses_visible_result",
            source: "use Test2::V0;\n\nmy $result = compute();\n\n<<CURSOR>>",
            first: Some("ok($result, 'test description');"),
            expected: &["ok($result, 'test description');"],
            not_expected: &["done_testing();"],
        },
        SuggestionFixture {
            name: "blank_line_in_sub_uses_visible_lexical",
            source: "sub compute {\n    my $result = build();\n    <<CURSOR>>\n}\n",
            first: Some("return $result;"),
            expected: &["return $result;"],
            not_expected: &["return $ghost;"],
        },
        SuggestionFixture {
            name: "condition_expression_uses_visible_guard_variable",
            source: "sub run {\n    my $ready = check();\n    if (<<CURSOR>>\n}\n",
            first: Some("$ready) {\n    \n}"),
            expected: &["$ready) {\n    \n}"],
            not_expected: &["$self) {\n    \n}"],
        },
        SuggestionFixture {
            name: "lexical_assignment_uses_visible_scalar",
            source: "sub copy {\n    my $result = compute();\n    my $copy = <<CURSOR>>\n}\n",
            first: Some("$result;"),
            expected: &["$result;"],
            not_expected: &["$copy;"],
        },
        SuggestionFixture {
            name: "array_assignment_uses_visible_array",
            source: "sub copy {\n    my @users = fetch_users();\n    my @copy = <<CURSOR>>\n}\n",
            first: Some("@users;"),
            expected: &["@users;"],
            not_expected: &["@copy;", "$users;"],
        },
        SuggestionFixture {
            name: "hash_assignment_uses_visible_hash",
            source: "sub copy {\n    my %users_by_id = load_users();\n    my %copy = <<CURSOR>>\n}\n",
            first: Some("%users_by_id;"),
            expected: &["%users_by_id;"],
            not_expected: &["%copy;", "$users_by_id;"],
        },
        SuggestionFixture {
            name: "self_receiver_prefers_current_package_methods",
            source: "package Other;\nsub external {}\n\npackage Demo;\nsub save {}\nsub display_name {}\nsub caller {\n    my $self = shift;\n    $self-><<CURSOR>>\n}\n",
            first: Some("save()"),
            expected: &["save()", "display_name()"],
            not_expected: &["external()", "new()"],
        },
        SuggestionFixture {
            name: "constructor_completion_keeps_shift_style",
            source: "sub helper {\n    my $self = shift;\n}\n\nsub new<<CURSOR>>",
            first: Some(
                " {\n    my $class = shift;\n    my $self = bless {}, $class;\n    return $self;\n}",
            ),
            expected: &[
                " {\n    my $class = shift;\n    my $self = bless {}, $class;\n    return $self;\n}",
            ],
            not_expected: &[
                " ($class, %args) {\n    my $self = bless {}, $class;\n    return $self;\n}",
            ],
        },
        SuggestionFixture {
            name: "constructor_completion_keeps_at_underscore_style",
            source: "sub helper {\n    my ($self, %args) = @_;\n}\n\nsub new<<CURSOR>>",
            first: Some(
                " {\n    my ($class, %args) = @_;\n    my $self = bless {}, $class;\n    return $self;\n}",
            ),
            expected: &[
                " {\n    my ($class, %args) = @_;\n    my $self = bless {}, $class;\n    return $self;\n}",
            ],
            not_expected: &[
                " {\n    my $class = shift;\n    my $self = bless {}, $class;\n    return $self;\n}",
            ],
        },
        SuggestionFixture {
            name: "constructor_completion_keeps_signature_style",
            source: "sub helper ($self, %args) {\n}\n\nsub new<<CURSOR>>",
            first: Some(
                " ($class, %args) {\n    my $self = bless {}, $class;\n    return $self;\n}",
            ),
            expected: &[
                " ($class, %args) {\n    my $self = bless {}, $class;\n    return $self;\n}",
            ],
            not_expected: &["my $class = shift;"],
        },
    ];

    for fixture in fixtures {
        assert_suggestions(fixture)?;
    }

    Ok(())
}

#[test]
fn inline_completion_fixture_corpus_stays_silent_in_reject_zones() -> TestResult {
    let fixtures = [
        SilentFixture { name: "line_comment", source: "# use <<CURSOR>>" },
        SilentFixture { name: "string_literal", source: "my $text = \"use <<CURSOR>>\";" },
        SilentFixture { name: "unterminated_string", source: "my $text = \"use <<CURSOR>>" },
        SilentFixture { name: "quote_single", source: "my $text = q{use <<CURSOR>>};" },
        SilentFixture { name: "quote_double", source: "my $text = qq{use <<CURSOR>>};" },
        SilentFixture { name: "quote_words", source: "my @words = qw(use <<CURSOR>>);" },
        SilentFixture { name: "quote_command", source: "my $output = qx(use <<CURSOR>>);" },
        SilentFixture { name: "heredoc_body", source: "print <<'EOF';\nuse <<CURSOR>>\nEOF\n" },
        SilentFixture {
            name: "format_body",
            source: "format STDOUT =\nuse <<CURSOR>>\n.\nwrite STDOUT;\n",
        },
        SilentFixture { name: "data_body", source: "__DATA__\nuse <<CURSOR>>\n" },
        SilentFixture { name: "pod_body", source: "=pod\nuse <<CURSOR>>\n=cut\n" },
        SilentFixture { name: "regex_literal", source: "if ($name =~ /use <<CURSOR>>/) {}" },
        SilentFixture { name: "substitution", source: "$name =~ s/use <<CURSOR>>/strict/;" },
        SilentFixture { name: "transliteration", source: "$name =~ tr/use <<CURSOR>>/abc/;" },
    ];

    for fixture in fixtures {
        let scenario = InlineCompletionScenario::from_fixture(fixture.source)?;
        let completions = scenario.completions();
        if !completions.items.is_empty() {
            return Err(format!(
                "{}: expected no inline completions, got {:?}",
                fixture.name,
                completion_texts(&completions)
            )
            .into());
        }
    }

    Ok(())
}

#[test]
fn inline_completion_fixture_corpus_uses_editor_safe_replacement_ranges() -> TestResult {
    let fixtures = [
        RangeFixture {
            name: "use_partial_token",
            source: "use str<<CURSOR>>",
            insert_text: "strict;",
            replaces: "str",
        },
        RangeFixture {
            name: "method_arrow_partial_token",
            source: "$obj->n<<CURSOR>>",
            insert_text: "new()",
            replaces: "n",
        },
    ];

    for fixture in fixtures {
        assert_replacement_range(fixture)?;
    }

    Ok(())
}

#[test]
fn inline_completion_fixture_corpus_applies_accepted_edits_without_parse_regressions() -> TestResult
{
    let fixtures = [
        AcceptedEditFixture {
            name: "partial_use_replacement",
            source: "use str<<CURSOR>>\n",
            expected_first: "strict;",
            expected_after: "use strict;\n",
        },
        AcceptedEditFixture {
            name: "visible_lexical_return",
            source: "sub compute {\n    my $result = build();\n    <<CURSOR>>\n}\n",
            expected_first: "return $result;",
            expected_after: "sub compute {\n    my $result = build();\n    return $result;\n}\n",
        },
        AcceptedEditFixture {
            name: "lexical_assignment_rhs",
            source: "sub copy {\n    my $result = compute();\n    my $copy = <<CURSOR>>\n}\n",
            expected_first: "$result;",
            expected_after: "sub copy {\n    my $result = compute();\n    my $copy = $result;\n}\n",
        },
        AcceptedEditFixture {
            name: "array_assignment_rhs",
            source: "sub copy {\n    my @users = fetch_users();\n    my @copy = <<CURSOR>>\n}\n",
            expected_first: "@users;",
            expected_after: "sub copy {\n    my @users = fetch_users();\n    my @copy = @users;\n}\n",
        },
        AcceptedEditFixture {
            name: "hash_assignment_rhs",
            source: "sub copy {\n    my %users_by_id = load_users();\n    my %copy = <<CURSOR>>\n}\n",
            expected_first: "%users_by_id;",
            expected_after: "sub copy {\n    my %users_by_id = load_users();\n    my %copy = %users_by_id;\n}\n",
        },
        AcceptedEditFixture {
            name: "self_receiver_method",
            source: "package Demo;\nsub save {}\nsub caller {\n    my $self = shift;\n    $self-><<CURSOR>>\n}\n",
            expected_first: "save()",
            expected_after: "package Demo;\nsub save {}\nsub caller {\n    my $self = shift;\n    $self->save()\n}\n",
        },
        AcceptedEditFixture {
            name: "constructor_shift_style",
            source: "sub helper {\n    my $self = shift;\n}\n\nsub new<<CURSOR>>\n",
            expected_first: " {\n    my $class = shift;\n    my $self = bless {}, $class;\n    return $self;\n}",
            expected_after: "sub helper {\n    my $self = shift;\n}\n\nsub new {\n    my $class = shift;\n    my $self = bless {}, $class;\n    return $self;\n}\n",
        },
    ];

    for fixture in fixtures {
        assert_accepted_edit(fixture)?;
    }

    Ok(())
}

#[test]
fn inline_completion_fixture_corpus_rejects_invalid_accepted_edit_ranges() -> TestResult {
    let scenario = InlineCompletionScenario::from_fixture("use str<<CURSOR>>\n")?;
    let item = InlineCompletionItem {
        insert_text: "strict;".to_string(),
        filter_text: Some("strict".to_string()),
        range: Some(Range { start: Position::new(0, 4), end: Position::new(0, 1) }),
        command: None,
    };

    let Err(error) = apply_inline_completion_item(&scenario, &item) else {
        return Err("invalid replacement range should be rejected".into());
    };
    if !error.contains("invalid replacement range 4..1") {
        return Err(format!("unexpected invalid-range error: {error}").into());
    }

    Ok(())
}

fn assert_suggestions(fixture: SuggestionFixture) -> TestResult {
    let scenario = InlineCompletionScenario::from_fixture(fixture.source)?;
    let completions = scenario.completions();

    if let Some(first) = fixture.first {
        let actual =
            completions.items.first().map(|item| item.insert_text.as_str()).ok_or_else(|| {
                format!("{}: expected first completion {first}, got none", fixture.name)
            })?;
        if actual != first {
            return Err(format!(
                "{}: expected first completion {first}, got {actual}",
                fixture.name
            )
            .into());
        }
    }

    for expected in fixture.expected {
        if !completions.items.iter().any(|item| item.insert_text == *expected) {
            return Err(format!(
                "{}: expected completion {expected}, got {:?}",
                fixture.name,
                completion_texts(&completions)
            )
            .into());
        }
    }

    for unexpected in fixture.not_expected {
        if completions.items.iter().any(|item| item.insert_text == *unexpected) {
            return Err(format!(
                "{}: unexpected completion {unexpected}, got {:?}",
                fixture.name,
                completion_texts(&completions)
            )
            .into());
        }
    }

    Ok(())
}

fn assert_replacement_range(fixture: RangeFixture) -> TestResult {
    let scenario = InlineCompletionScenario::from_fixture(fixture.source)?;
    let completions = scenario.completions();
    let item =
        completions.items.iter().find(|item| item.insert_text == fixture.insert_text).ok_or_else(
            || {
                format!(
                    "{}: expected completion {}, got {:?}",
                    fixture.name,
                    fixture.insert_text,
                    completion_texts(&completions)
                )
            },
        )?;
    let range = item
        .range
        .as_ref()
        .ok_or_else(|| format!("{}: expected replacement range", fixture.name))?;
    let replaced = slice_for_range(scenario.text.as_str(), range)?;

    if replaced != fixture.replaces {
        return Err(format!(
            "{}: expected replacement range to cover {:?}, got {:?}",
            fixture.name, fixture.replaces, replaced
        )
        .into());
    }

    Ok(())
}

fn assert_accepted_edit(fixture: AcceptedEditFixture) -> TestResult {
    let scenario = InlineCompletionScenario::from_fixture(fixture.source)?;
    let completions = scenario.completions();
    let item = completions.items.first().ok_or_else(|| {
        format!("{}: expected first completion {}, got none", fixture.name, fixture.expected_first)
    })?;

    if item.insert_text != fixture.expected_first {
        return Err(format!(
            "{}: expected first completion {}, got {}",
            fixture.name, fixture.expected_first, item.insert_text
        )
        .into());
    }

    let accepted = apply_inline_completion_item(&scenario, item)?;
    if accepted != fixture.expected_after {
        return Err(format!(
            "{}: accepted edit produced unexpected text\nexpected:\n{}\nactual:\n{}",
            fixture.name, fixture.expected_after, accepted
        )
        .into());
    }

    let before = parser_diagnostic_count(scenario.text.as_str());
    let after = parser_diagnostic_count(accepted.as_str());
    if after > before {
        return Err(format!(
            "{}: accepted edit increased parser diagnostics from {before} to {after}",
            fixture.name
        )
        .into());
    }

    Ok(())
}

fn completion_texts(completions: &InlineCompletionList) -> Vec<&str> {
    completions.items.iter().map(|item| item.insert_text.as_str()).collect()
}

fn apply_inline_completion_item(
    scenario: &InlineCompletionScenario,
    item: &InlineCompletionItem,
) -> Result<String, String> {
    let cursor =
        utf16_line_col_to_offset(scenario.text.as_str(), scenario.line, scenario.character);
    let (start, end) = item
        .range
        .as_ref()
        .map(|range| {
            let start = utf16_line_col_to_offset(
                scenario.text.as_str(),
                range.start.line,
                range.start.character,
            );
            let end = utf16_line_col_to_offset(
                scenario.text.as_str(),
                range.end.line,
                range.end.character,
            );
            Ok::<_, String>((start, end))
        })
        .transpose()?
        .unwrap_or((cursor, cursor));
    if end < start {
        return Err(format!("invalid replacement range {start}..{end}"));
    }

    let mut accepted =
        String::with_capacity(scenario.text.len() - (end - start) + item.insert_text.len());
    accepted.push_str(
        scenario
            .text
            .get(..start)
            .ok_or_else(|| format!("invalid replacement start offset {start}"))?,
    );
    accepted.push_str(item.insert_text.as_str());
    accepted.push_str(
        scenario.text.get(end..).ok_or_else(|| format!("invalid replacement end offset {end}"))?,
    );
    Ok(accepted)
}

fn parser_diagnostic_count(source: &str) -> usize {
    perl_parser::Parser::new(source).parse_with_recovery().diagnostics.len()
}

fn slice_for_range<'a>(text: &'a str, range: &Range) -> Result<&'a str, String> {
    let start = utf16_line_col_to_offset(text, range.start.line, range.start.character);
    let end = utf16_line_col_to_offset(text, range.end.line, range.end.character);
    text.get(start..end).ok_or_else(|| format!("invalid UTF-8 boundaries for range {start}..{end}"))
}
