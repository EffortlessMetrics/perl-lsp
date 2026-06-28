//! Fixture-style UX coverage for deterministic inline completion.
//!
//! These tests keep the visible ghost-text contract readable: the provider
//! should suggest project/style-aware text where it has context and stay silent
//! in zones where automatic ghost text would be noisy or unsafe.

use std::error::Error;

use lsp_types::{Position, Range};
use perl_lsp_rs_core::providers::inline_completion::{
    InlineCompletionEnvironment, InlineCompletionItem, InlineCompletionList,
    InlineCompletionProvider, InlinePackageMethodFact,
};
use perl_parser_core::position::{offset_to_utf16_line_col, utf16_line_col_to_offset};

type TestResult = Result<(), Box<dyn Error>>;

const CURSOR: &str = "<<CURSOR>>";
const TRY_TINY_BLOCK: &str = "{\n    \n} catch {\n    \n};";
const MOJOLICIOUS_LITE_ROUTE: &str =
    "'/path' => sub {\n    my $c = shift;\n    $c->render(text => 'ok');\n};";
const DANCER_ROUTE: &str = "'/path' => sub {\n    return 'ok';\n};";
const TEST_MORE_IS_ASSERTION: &str = "is($got, $expected, 'test description');";
const TEST2_OK_ASSERTION: &str = "ok($result, 'test description');";
const DBI_PREPARE_METHOD: &str = "prepare()";
const DBI_FETCHROW_HASHREF_METHOD: &str = "fetchrow_hashref()";

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

    fn completions_with_environment(
        &self,
        environment: &InlineCompletionEnvironment,
    ) -> InlineCompletionList {
        InlineCompletionProvider::new().get_inline_completions_with_environment(
            self.text.as_str(),
            self.line,
            self.character,
            environment,
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

struct CompletionPackContract {
    provider_id: &'static str,
    insert_text: &'static str,
    filter_text: &'static str,
    positive: &'static [CompletionPackPositiveCase],
    quiet: &'static [CompletionPackQuietCase],
}

struct CompletionPackPositiveCase {
    name: &'static str,
    source: &'static str,
    expected_replaces: Option<&'static str>,
    expected_after: &'static str,
}

struct CompletionPackQuietCase {
    name: &'static str,
    category: CompletionPackQuietCategory,
    source: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompletionPackQuietCategory {
    ImportAbsent,
    CommentContext,
    StringContext,
    PodContext,
    NearMatchToken,
    VisibleSymbolConflict,
    ParseDamage,
}

const REQUIRED_COMPLETION_PACK_QUIET_CATEGORIES: &[CompletionPackQuietCategory] = &[
    CompletionPackQuietCategory::ImportAbsent,
    CompletionPackQuietCategory::CommentContext,
    CompletionPackQuietCategory::StringContext,
    CompletionPackQuietCategory::PodContext,
    CompletionPackQuietCategory::NearMatchToken,
    CompletionPackQuietCategory::VisibleSymbolConflict,
    CompletionPackQuietCategory::ParseDamage,
];

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
            name: "next_unless_uses_visible_guard_variable",
            source: "sub run {\n    my $should_skip = should_skip();\n    next unless <<CURSOR>>\n}\n",
            first: Some("$should_skip;"),
            expected: &["$should_skip;"],
            not_expected: &["$result;", "$self;"],
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
            name: "moo_self_receiver_prefers_attribute_accessors",
            source: "package Other;\nuse Moo;\nhas 'external' => (is => 'ro');\n\npackage Demo;\nuse Moo;\nhas 'name' => (is => 'ro');\nhas \"email\" => (is => 'rw');\nsub caller {\n    my $self = shift;\n    $self-><<CURSOR>>\n}\n",
            first: Some("name()"),
            expected: &["name()", "email()"],
            not_expected: &["external()", "new()"],
        },
        SuggestionFixture {
            name: "moose_self_receiver_prefers_attribute_accessors",
            source: "package Demo;\nuse Moose;\nhas 'enabled' => (is => 'ro');\nsub caller {\n    my $self = shift;\n    $self-><<CURSOR>>\n}\n",
            first: Some("enabled()"),
            expected: &["enabled()"],
            not_expected: &["new()"],
        },
        SuggestionFixture {
            name: "plain_has_declaration_does_not_become_accessor",
            source: "package Demo;\nhas 'name' => (is => 'ro');\nsub caller {\n    $self-><<CURSOR>>\n}\n",
            first: None,
            expected: &[],
            not_expected: &["name()", "external()", "new()"],
        },
        SuggestionFixture {
            name: "moo_runtime_has_call_does_not_become_accessor",
            source: "package Demo;\nuse Moo;\nsub caller {\n    has 'temporary' => (is => 'ro');\n    $self-><<CURSOR>>\n}\n",
            first: None,
            expected: &[],
            not_expected: &["temporary()", "new()"],
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
        SuggestionFixture {
            name: "lexical_scalar_declaration_suggests_self_shift",
            source: "sub method {\n    my $<<CURSOR>>\n}",
            first: Some("self = shift;"),
            expected: &["self = shift;"],
            not_expected: &["strict;"],
        },
        SuggestionFixture {
            name: "package_declaration_suggests_module_skeleton",
            source: "package <<CURSOR>>",
            first: Some("MyPackage;\n\nuse strict;\nuse warnings;"),
            expected: &["MyPackage;\n\nuse strict;\nuse warnings;"],
            not_expected: &["strict;"],
        },
        SuggestionFixture {
            name: "bless_arguments_suggest_self_and_class",
            source: "sub new {\n    my $class = shift;\n    my $self = {};\n    bless <<CURSOR>>\n}",
            first: Some("$self, $class;"),
            expected: &["$self, $class;"],
            not_expected: &["return $self;"],
        },
        SuggestionFixture {
            name: "constructor_return_prefers_self",
            source: "sub new {\n    my $class = shift;\n    my $self = bless {}, $class;\n    return <<CURSOR>>\n}",
            first: Some("$self;"),
            expected: &["$self;"],
            not_expected: &["return $self;"],
        },
        SuggestionFixture {
            name: "foreach_hash_uses_key_binding",
            source: "my %counts = ();\nforeach <<CURSOR>>",
            first: Some("my $key (keys %counts) {\n    \n}"),
            expected: &["my $key (keys %counts) {\n    \n}"],
            not_expected: &["my $count (@counts) {\n    \n}"],
        },
    ];

    for fixture in fixtures {
        assert_suggestions(fixture)?;
    }

    Ok(())
}

#[test]
fn inline_completion_fixture_corpus_defines_completion_pack_contract() -> TestResult {
    let try_tiny = CompletionPackContract {
        provider_id: "try_tiny_block",
        insert_text: TRY_TINY_BLOCK,
        filter_text: "try",
        positive: &[CompletionPackPositiveCase {
            name: "import_present_valid_try_keyword",
            source: "use Try::Tiny;\ntry <<CURSOR>>",
            expected_replaces: None,
            expected_after: "use Try::Tiny;\ntry {\n    \n} catch {\n    \n};",
        }],
        quiet: &[
            CompletionPackQuietCase {
                name: "import_absent",
                category: CompletionPackQuietCategory::ImportAbsent,
                source: "try <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "comment_context",
                category: CompletionPackQuietCategory::CommentContext,
                source: "use Try::Tiny;\n# try <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "string_context",
                category: CompletionPackQuietCategory::StringContext,
                source: "use Try::Tiny;\nmy $text = \"try <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "pod_context",
                category: CompletionPackQuietCategory::PodContext,
                source: "use Try::Tiny;\n=pod\ntry <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "near_match_token",
                category: CompletionPackQuietCategory::NearMatchToken,
                source: "use Try::Tiny;\ngettry <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "visible_symbol_conflict",
                category: CompletionPackQuietCategory::VisibleSymbolConflict,
                source: "use Try::Tiny;\nmy $try = 1;\n$try <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "parse_damage_extra_closing_paren",
                category: CompletionPackQuietCategory::ParseDamage,
                source: "use Try::Tiny;\ntry <<CURSOR>>)",
            },
        ],
    };

    let mojolicious_lite = CompletionPackContract {
        provider_id: "mojolicious_lite_route",
        insert_text: MOJOLICIOUS_LITE_ROUTE,
        filter_text: "get",
        positive: &[CompletionPackPositiveCase {
            name: "import_present_valid_route_keyword",
            source: "use Mojolicious::Lite;\nget <<CURSOR>>",
            expected_replaces: None,
            expected_after: "use Mojolicious::Lite;\nget '/path' => sub {\n    my $c = shift;\n    $c->render(text => 'ok');\n};",
        }],
        quiet: &[
            CompletionPackQuietCase {
                name: "import_absent",
                category: CompletionPackQuietCategory::ImportAbsent,
                source: "get <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "comment_context",
                category: CompletionPackQuietCategory::CommentContext,
                source: "use Mojolicious::Lite;\n# get <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "string_context",
                category: CompletionPackQuietCategory::StringContext,
                source: "use Mojolicious::Lite;\nmy $text = \"get <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "pod_context",
                category: CompletionPackQuietCategory::PodContext,
                source: "use Mojolicious::Lite;\n=pod\nget <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "near_match_token",
                category: CompletionPackQuietCategory::NearMatchToken,
                source: "use Mojolicious::Lite;\nforget <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "visible_symbol_conflict",
                category: CompletionPackQuietCategory::VisibleSymbolConflict,
                source: "use Mojolicious::Lite;\nmy $get = 1;\n$get <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "parse_damage_extra_closing_paren",
                category: CompletionPackQuietCategory::ParseDamage,
                source: "use Mojolicious::Lite;\nget <<CURSOR>>)",
            },
        ],
    };

    let dancer_route = CompletionPackContract {
        provider_id: "dancer_route",
        insert_text: DANCER_ROUTE,
        filter_text: "get",
        positive: &[
            CompletionPackPositiveCase {
                name: "dancer_import_present_valid_route_keyword",
                source: "use Dancer;\nget <<CURSOR>>",
                expected_replaces: None,
                expected_after: "use Dancer;\nget '/path' => sub {\n    return 'ok';\n};",
            },
            CompletionPackPositiveCase {
                name: "dancer2_import_present_valid_route_keyword",
                source: "use Dancer2;\nget <<CURSOR>>",
                expected_replaces: None,
                expected_after: "use Dancer2;\nget '/path' => sub {\n    return 'ok';\n};",
            },
        ],
        quiet: &[
            CompletionPackQuietCase {
                name: "import_absent",
                category: CompletionPackQuietCategory::ImportAbsent,
                source: "get <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "comment_context",
                category: CompletionPackQuietCategory::CommentContext,
                source: "use Dancer2;\n# get <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "string_context",
                category: CompletionPackQuietCategory::StringContext,
                source: "use Dancer2;\nmy $text = \"get <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "pod_context",
                category: CompletionPackQuietCategory::PodContext,
                source: "use Dancer2;\n=pod\nget <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "near_match_token",
                category: CompletionPackQuietCategory::NearMatchToken,
                source: "use Dancer2;\nforget <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "visible_symbol_conflict",
                category: CompletionPackQuietCategory::VisibleSymbolConflict,
                source: "use Dancer2;\nmy $get = 1;\n$get <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "parse_damage_extra_closing_paren",
                category: CompletionPackQuietCategory::ParseDamage,
                source: "use Dancer2;\nget <<CURSOR>>)",
            },
        ],
    };

    let test_more_assertion = CompletionPackContract {
        provider_id: "test_more_assertion",
        insert_text: TEST_MORE_IS_ASSERTION,
        filter_text: "is",
        positive: &[CompletionPackPositiveCase {
            name: "import_present_visible_actual_expected",
            source: "use Test::More;\n\nmy $got = compute();\nmy $expected = 42;\n\n<<CURSOR>>",
            expected_replaces: None,
            expected_after: "use Test::More;\n\nmy $got = compute();\nmy $expected = 42;\n\nis($got, $expected, 'test description');",
        }],
        quiet: &[
            CompletionPackQuietCase {
                name: "import_absent_visible_actual_expected",
                category: CompletionPackQuietCategory::ImportAbsent,
                source: "my $got = compute();\nmy $expected = 42;\n\n<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "comment_context",
                category: CompletionPackQuietCategory::CommentContext,
                source: "use Test::More;\nmy $got = compute();\nmy $expected = 42;\n# <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "string_context",
                category: CompletionPackQuietCategory::StringContext,
                source: "use Test::More;\nmy $got = compute();\nmy $expected = 42;\nmy $text = \"<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "pod_context",
                category: CompletionPackQuietCategory::PodContext,
                source: "use Test::More;\nmy $got = compute();\nmy $expected = 42;\n=pod\n<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "near_match_token",
                category: CompletionPackQuietCategory::NearMatchToken,
                source: "use Test::More;\nmy $got = compute();\nmy $expected = 42;\nassert <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "visible_symbol_conflict",
                category: CompletionPackQuietCategory::VisibleSymbolConflict,
                source: "use Test::More;\nmy $got = compute();\nmy $expected = 42;\nmy $is = sub {};\n$is <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "parse_damage_incomplete_declaration",
                category: CompletionPackQuietCategory::ParseDamage,
                source: "use Test::More;\nmy $got = compute();\nmy $expected = 42;\nmy <<CURSOR>>",
            },
        ],
    };

    let test2_assertion = CompletionPackContract {
        provider_id: "test2_assertion",
        insert_text: TEST2_OK_ASSERTION,
        filter_text: "ok",
        positive: &[CompletionPackPositiveCase {
            name: "import_present_visible_result",
            source: "use Test2::V0;\n\nmy $result = compute();\n\n<<CURSOR>>",
            expected_replaces: None,
            expected_after: "use Test2::V0;\n\nmy $result = compute();\n\nok($result, 'test description');",
        }],
        quiet: &[
            CompletionPackQuietCase {
                name: "import_absent_visible_result",
                category: CompletionPackQuietCategory::ImportAbsent,
                source: "my $result = compute();\n\n<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "comment_context",
                category: CompletionPackQuietCategory::CommentContext,
                source: "use Test2::V0;\nmy $result = compute();\n# <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "string_context",
                category: CompletionPackQuietCategory::StringContext,
                source: "use Test2::V0;\nmy $result = compute();\nmy $text = \"<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "pod_context",
                category: CompletionPackQuietCategory::PodContext,
                source: "use Test2::V0;\nmy $result = compute();\n=pod\n<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "near_match_token",
                category: CompletionPackQuietCategory::NearMatchToken,
                source: "use Test2::V0;\nmy $result = compute();\nassert <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "visible_symbol_conflict",
                category: CompletionPackQuietCategory::VisibleSymbolConflict,
                source: "use Test2::V0;\nmy $result = compute();\nmy $ok = sub {};\n$ok <<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "parse_damage_incomplete_declaration",
                category: CompletionPackQuietCategory::ParseDamage,
                source: "use Test2::V0;\nmy $result = compute();\nmy <<CURSOR>>",
            },
        ],
    };

    let dbi_database_handle = CompletionPackContract {
        provider_id: "dbi_database_handle_methods",
        insert_text: DBI_PREPARE_METHOD,
        filter_text: "prepare",
        positive: &[CompletionPackPositiveCase {
            name: "import_present_database_handle_partial_method",
            source: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\n$dbh->pr<<CURSOR>>",
            expected_replaces: Some("pr"),
            expected_after: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\n$dbh->prepare()",
        }],
        quiet: &[
            CompletionPackQuietCase {
                name: "import_absent_database_handle_hint",
                category: CompletionPackQuietCategory::ImportAbsent,
                source: "my $dbh = DBI->connect($dsn, $user, $pass);\n$dbh->pr<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "comment_context",
                category: CompletionPackQuietCategory::CommentContext,
                source: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\n# $dbh->pr<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "string_context",
                category: CompletionPackQuietCategory::StringContext,
                source: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\nmy $text = \"$dbh->pr<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "pod_context",
                category: CompletionPackQuietCategory::PodContext,
                source: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\n=pod\n$dbh->pr<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "near_match_receiver_syntax",
                category: CompletionPackQuietCategory::NearMatchToken,
                source: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\n$dbh=>pr<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "non_dbi_receiver_conflict",
                category: CompletionPackQuietCategory::VisibleSymbolConflict,
                source: "use DBI;\nmy $socket = Client->connect($dsn);\n$socket->pr<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "parse_damage_non_scalar_receiver",
                category: CompletionPackQuietCategory::ParseDamage,
                source: "use DBI;\nmy @dbh = DBI->connect($dsn, $user, $pass);\n@dbh->pr<<CURSOR>>",
            },
        ],
    };

    let dbi_statement_handle = CompletionPackContract {
        provider_id: "dbi_statement_handle_methods",
        insert_text: DBI_FETCHROW_HASHREF_METHOD,
        filter_text: "fetchrow_hashref",
        positive: &[CompletionPackPositiveCase {
            name: "import_present_statement_handle_partial_method",
            source: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\nmy $sth = $dbh->prepare($sql);\n$sth->fetch<<CURSOR>>",
            expected_replaces: Some("fetch"),
            expected_after: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\nmy $sth = $dbh->prepare($sql);\n$sth->fetchrow_hashref()",
        }],
        quiet: &[
            CompletionPackQuietCase {
                name: "import_absent_statement_handle_hint",
                category: CompletionPackQuietCategory::ImportAbsent,
                source: "my $dbh = DBI->connect($dsn, $user, $pass);\nmy $sth = $dbh->prepare($sql);\n$sth->fetch<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "comment_context",
                category: CompletionPackQuietCategory::CommentContext,
                source: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\nmy $sth = $dbh->prepare($sql);\n# $sth->fetch<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "string_context",
                category: CompletionPackQuietCategory::StringContext,
                source: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\nmy $sth = $dbh->prepare($sql);\nmy $text = \"$sth->fetch<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "pod_context",
                category: CompletionPackQuietCategory::PodContext,
                source: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\nmy $sth = $dbh->prepare($sql);\n=pod\n$sth->fetch<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "near_match_receiver_syntax",
                category: CompletionPackQuietCategory::NearMatchToken,
                source: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\nmy $sth = $dbh->prepare($sql);\n$sth=>fetch<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "non_dbi_receiver_conflict",
                category: CompletionPackQuietCategory::VisibleSymbolConflict,
                source: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\nmy $query = $builder->prepare($sql);\n$query->fetch<<CURSOR>>",
            },
            CompletionPackQuietCase {
                name: "parse_damage_non_scalar_receiver",
                category: CompletionPackQuietCategory::ParseDamage,
                source: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\nmy @sth = $dbh->prepare($sql);\n@sth->fetch<<CURSOR>>",
            },
        ],
    };

    assert_completion_pack_contract(try_tiny)?;
    assert_completion_pack_contract(mojolicious_lite)?;
    assert_completion_pack_contract(dancer_route)?;
    assert_completion_pack_contract(test_more_assertion)?;
    assert_completion_pack_contract(test2_assertion)?;
    assert_completion_pack_contract(dbi_database_handle)?;
    assert_completion_pack_contract(dbi_statement_handle)
}

#[test]
fn inline_completion_fixture_corpus_uses_request_environment_modules() -> TestResult {
    let scenario = InlineCompletionScenario::from_fixture("use Local::W<<CURSOR>>")?;
    let environment = InlineCompletionEnvironment {
        available_modules: vec![
            "Other::Widget".to_string(),
            "Local::Widget".to_string(),
            "Local::Worker".to_string(),
        ],
        package_methods: Vec::new(),
    };
    let completions = scenario.completions_with_environment(&environment);

    assert_completion_present("workspace_modules", &completions, "Local::Widget;")?;
    assert_completion_present("workspace_modules", &completions, "Local::Worker;")?;
    assert_completion_absent("workspace_modules", &completions, "Other::Widget;")?;

    let first = completions
        .items
        .first()
        .map(|item| item.insert_text.as_str())
        .ok_or("workspace_modules: expected module completion")?;
    if first != "Local::Widget;" {
        return Err(format!("workspace_modules: expected Local::Widget; first, got {first}").into());
    }

    Ok(())
}

#[test]
fn inline_completion_fixture_corpus_covers_dbi_receivers() -> TestResult {
    let fixtures = [
        SuggestionFixture {
            name: "dbi_database_handle_methods",
            source: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\n$dbh->pr<<CURSOR>>",
            first: Some("prepare()"),
            expected: &["prepare()"],
            not_expected: &["fetchrow_hashref()", "new()"],
        },
        SuggestionFixture {
            name: "dbi_statement_handle_methods",
            source: "use DBI;\nmy $dbh = DBI->connect($dsn, $user, $pass);\nmy $sth = $dbh->prepare($sql);\n$sth->fetch<<CURSOR>>",
            first: Some("fetchrow_hashref()"),
            expected: &["fetchrow_hashref()", "fetchrow_array()"],
            not_expected: &["prepare()", "new()"],
        },
    ];

    for fixture in fixtures {
        assert_suggestions(fixture)?;
    }

    Ok(())
}

#[test]
fn inline_completion_fixture_corpus_covers_control_flow_and_shebang_contexts() -> TestResult {
    let fixtures = [
        SuggestionFixture {
            name: "guard_condition_prefers_boolean_lexical",
            source: "my $is_ready = check();\nreturn unless <<CURSOR>>",
            first: Some("$is_ready;"),
            expected: &["$is_ready;"],
            not_expected: &["$self;"],
        },
        SuggestionFixture {
            name: "loop_binding_singularizes_visible_array",
            source: "my @statuses = load_statuses();\nfor <<CURSOR>>",
            first: Some("my $status (@statuses) {\n    \n}"),
            expected: &["my $status (@statuses) {\n    \n}"],
            not_expected: &["my $item (@statuses) {\n    \n}"],
        },
        SuggestionFixture {
            name: "shebang_interpreter",
            source: "#!<<CURSOR>>",
            first: Some("/usr/bin/env perl"),
            expected: &["/usr/bin/env perl"],
            not_expected: &["strict;"],
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
fn inline_completion_fixture_corpus_uses_indexed_package_methods_conservatively() -> TestResult {
    let environment = InlineCompletionEnvironment {
        available_modules: Vec::new(),
        package_methods: vec![
            InlinePackageMethodFact {
                package: "My::Service".to_string(),
                name: "save".to_string(),
            },
            InlinePackageMethodFact {
                package: "My::Service".to_string(),
                name: "search".to_string(),
            },
        ],
    };

    let positive = InlineCompletionScenario::from_fixture("My::Service->sa<<CURSOR>>")?;
    let completions = positive.completions_with_environment(&environment);
    let item =
        completions.items.iter().find(|item| item.insert_text == "save()").ok_or_else(|| {
            format!(
                "indexed_package_methods: expected save(), got {:?}",
                completion_texts(&completions)
            )
        })?;
    if item.filter_text.as_deref() != Some("save") {
        return Err(format!("indexed_package_methods: unexpected filter text {:?}", item).into());
    }
    let range = item.range.as_ref().ok_or("indexed_package_methods: expected replacement range")?;
    let replaced = slice_for_range(positive.text.as_str(), range)?;
    if replaced != "sa" {
        return Err(format!(
            "indexed_package_methods: expected range to replace sa, got {replaced:?}"
        )
        .into());
    }

    let accepted = apply_inline_completion_item(&positive, item)?;
    if accepted != "My::Service->save()" {
        return Err(
            format!("indexed_package_methods: accepted edit mismatch, got {accepted:?}").into()
        );
    }

    let same_package =
        InlineCompletionScenario::from_fixture("package My::Service;\nMy::Service->se<<CURSOR>>")?;
    let completions = same_package.completions_with_environment(&environment);
    let item =
        completions.items.iter().find(|item| item.insert_text == "search()").ok_or_else(|| {
            format!(
                "indexed_package_methods:same_package expected search(), got {:?}",
                completion_texts(&completions)
            )
        })?;
    let range = item
        .range
        .as_ref()
        .ok_or("indexed_package_methods:same_package expected replacement range")?;
    let replaced = slice_for_range(same_package.text.as_str(), range)?;
    if replaced != "se" {
        return Err(format!(
            "indexed_package_methods:same_package expected range to replace se, got {replaced:?}"
        )
        .into());
    }

    let quiet_with_environment = [
        SilentFixture { name: "wrong_package", source: "Other::Service->sa<<CURSOR>>" },
        SilentFixture {
            name: "dynamic_variable_receiver",
            source: "my $service = My::Service->new;\n$service->sa<<CURSOR>>",
        },
        SilentFixture { name: "comment_context", source: "# My::Service->sa<<CURSOR>>" },
        SilentFixture { name: "string_context", source: "my $text = \"My::Service->sa<<CURSOR>>" },
        SilentFixture { name: "pod_context", source: "=pod\nMy::Service->sa<<CURSOR>>" },
        SilentFixture { name: "near_match_receiver_syntax", source: "My::Service=>sa<<CURSOR>>" },
        SilentFixture {
            name: "parse_damage_non_package_receiver",
            source: "my @service;\n@service->sa<<CURSOR>>",
        },
    ];
    for fixture in quiet_with_environment {
        let scenario = InlineCompletionScenario::from_fixture(fixture.source)?;
        let completions = scenario.completions_with_environment(&environment);
        assert_completion_absent(fixture.name, &completions, "save()")?;
        assert_completion_absent(fixture.name, &completions, "search()")?;
    }

    let missing_facts = InlineCompletionScenario::from_fixture("My::Service->sa<<CURSOR>>")?;
    let completions = missing_facts.completions();
    assert_completion_absent("missing_indexed_facts", &completions, "save()")?;
    assert_completion_absent("missing_indexed_facts", &completions, "search()")?;

    Ok(())
}

#[test]
fn inline_completion_fixture_corpus_prefers_workspace_available_modules() -> TestResult {
    let scenario = InlineCompletionScenario::from_fixture("use My::W<<CURSOR>>")?;
    let environment = InlineCompletionEnvironment {
        available_modules: vec![
            "My::Widget".to_string(),
            "My::Worker".to_string(),
            "Other::Widget".to_string(),
        ],
        package_methods: Vec::new(),
    };
    let completions = scenario.completions_with_environment(&environment);
    let inserts = completion_texts(&completions);

    if inserts != vec!["My::Widget;", "My::Worker;"] {
        return Err(format!(
            "workspace module suggestions should match the typed module fragment, got {inserts:?}"
        )
        .into());
    }

    for item in &completions.items {
        let range = item.range.as_ref().ok_or_else(|| {
            format!("{} should replace the typed module fragment", item.insert_text)
        })?;
        let replaced = slice_for_range(scenario.text.as_str(), range)?;
        if replaced != "My::W" {
            return Err(
                format!("{} should replace My::W, got {replaced:?}", item.insert_text).into()
            );
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
            name: "next_unless_guard_condition",
            source: "sub run {\n    my $should_skip = should_skip();\n    next unless <<CURSOR>>\n}\n",
            expected_first: "$should_skip;",
            expected_after: "sub run {\n    my $should_skip = should_skip();\n    next unless $should_skip;\n}\n",
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
        assert_completion_present(fixture.name, &completions, expected)?;
    }

    for unexpected in fixture.not_expected {
        assert_completion_absent(fixture.name, &completions, unexpected)?;
    }

    Ok(())
}

fn assert_completion_present(
    fixture_name: &str,
    completions: &InlineCompletionList,
    expected: &str,
) -> TestResult {
    if !completions.items.iter().any(|item| item.insert_text == expected) {
        return Err(format!(
            "{fixture_name}: expected completion {expected}, got {:?}",
            completion_texts(completions)
        )
        .into());
    }

    Ok(())
}

fn assert_completion_absent(
    fixture_name: &str,
    completions: &InlineCompletionList,
    unexpected: &str,
) -> TestResult {
    if completions.items.iter().any(|item| item.insert_text == unexpected) {
        return Err(format!(
            "{fixture_name}: unexpected completion {unexpected}, got {:?}",
            completion_texts(completions)
        )
        .into());
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

fn assert_completion_pack_contract(contract: CompletionPackContract) -> TestResult {
    assert_completion_pack_receipt_categories(&contract)?;

    for case in contract.positive {
        let scenario = InlineCompletionScenario::from_fixture(case.source)?;
        let completions = scenario.completions();
        let item = completions.items.iter().find(|item| item.insert_text == contract.insert_text);
        let Some(item) = item else {
            return Err(format!(
                "{}:{} expected completion {}, got {:?}",
                contract.provider_id,
                case.name,
                contract.insert_text,
                completion_texts(&completions)
            )
            .into());
        };

        if item.filter_text.as_deref() != Some(contract.filter_text) {
            return Err(format!(
                "{}:{} expected filter_text {:?}, got {:?}",
                contract.provider_id, case.name, contract.filter_text, item.filter_text
            )
            .into());
        }

        match (case.expected_replaces, item.range.as_ref()) {
            (Some(expected), Some(range)) => {
                let replaced = slice_for_range(scenario.text.as_str(), range)?;
                if replaced != expected {
                    return Err(format!(
                        "{}:{} expected replacement range to cover {:?}, got {:?}",
                        contract.provider_id, case.name, expected, replaced
                    )
                    .into());
                }
            }
            (Some(expected), None) => {
                return Err(format!(
                    "{}:{} expected replacement range for {:?}",
                    contract.provider_id, case.name, expected
                )
                .into());
            }
            (None, Some(range)) => {
                return Err(format!(
                    "{}:{} expected insertion-only candidate, got range {:?}",
                    contract.provider_id, case.name, range
                )
                .into());
            }
            (None, None) => {}
        }

        let accepted = apply_inline_completion_item(&scenario, item)?;
        if accepted != case.expected_after {
            return Err(format!(
                "{}:{} accepted edit produced unexpected text\nexpected:\n{}\nactual:\n{}",
                contract.provider_id, case.name, case.expected_after, accepted
            )
            .into());
        }

        let before = parser_diagnostic_count(scenario.text.as_str());
        let after = parser_diagnostic_count(accepted.as_str());
        if after > before {
            return Err(format!(
                "{}:{} accepted edit increased parser diagnostics from {before} to {after}",
                contract.provider_id, case.name
            )
            .into());
        }
    }

    for case in contract.quiet {
        let scenario = InlineCompletionScenario::from_fixture(case.source)?;
        let completions = scenario.completions();
        if completions.items.iter().any(|item| item.insert_text == contract.insert_text) {
            return Err(format!(
                "{}:{} expected pack to stay quiet, got {:?}",
                contract.provider_id,
                case.name,
                completion_texts(&completions)
            )
            .into());
        }
    }

    Ok(())
}

fn assert_completion_pack_receipt_categories(contract: &CompletionPackContract) -> TestResult {
    for required in REQUIRED_COMPLETION_PACK_QUIET_CATEGORIES {
        if !contract.quiet.iter().any(|case| case.category == *required) {
            return Err(format!(
                "{} missing required quiet-path receipt category {required:?}",
                contract.provider_id
            )
            .into());
        }
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
