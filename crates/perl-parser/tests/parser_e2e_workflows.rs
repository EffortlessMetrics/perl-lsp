//! End-to-end parser workflows that exercise full-file Perl scenarios.
//!
//! These tests intentionally parse complete, editor-sized snippets rather than
//! isolated grammar productions. They protect the seams between lexing,
//! statement recovery, expression parsing, and AST rendering.

use perl_parser::Parser;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
struct ParserE2eCase<'a> {
    name: &'a str,
    source: &'a str,
    required_fragments: &'a [&'a str],
}

fn parse_clean_sexp(source: &str) -> TestResult<String> {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    assert!(
        output.diagnostics.is_empty(),
        "expected clean parse, got diagnostics: {:?}",
        output.diagnostics
    );
    let sexp = output.ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "expected no recovery ERROR nodes in: {sexp}");
    Ok(sexp)
}

fn assert_case_shape(case: ParserE2eCase<'_>) -> TestResult {
    let sexp = parse_clean_sexp(case.source)?;

    for fragment in case.required_fragments {
        assert!(
            sexp.contains(fragment),
            "case `{}` missing required fragment `{}` in: {}",
            case.name,
            fragment,
            sexp
        );
    }

    Ok(())
}

#[test]
fn e2e_realistic_module_keeps_package_constructor_io_and_method_flow() -> TestResult {
    // Given: a small module that combines package setup, imports, construction,
    // method calls, filehandle IO, regex filtering, and postfix flow control.
    let source = r#"
package My::App::Service;
use strict;
use warnings;
use List::Util qw(first sum);

sub new {
    my ($class, %args) = @_;
    my $self = bless { %args, count => 0 }, $class;
    return $self;
}

sub run {
    my ($self, $path) = @_;
    open my $fh, '<', $path;
    while (my $line = <$fh>) {
        chomp $line;
        next if $line =~ /^#/;
        $self->{count}++;
    }
    return $self->{count};
}
1;
"#;

    // When/Then: the parser preserves each top-level workflow feature without
    // falling back to recovery nodes.
    assert_case_shape(ParserE2eCase {
        name: "module",
        source,
        required_fragments: &[
            "My::App::Service",
            "List::Util",
            "qw(first sum)",
            "bless",
            "open",
            "readline",
            "statement_modifier",
            "unary_++",
            "(return",
        ],
    })
}

#[test]
fn e2e_cli_script_keeps_list_pipeline_heredoc_regex_and_postfix_print() -> TestResult {
    // Given: a script-shaped snippet with common CPAN idioms spanning multiple
    // parser subsystems: qw lists, grep/map/sort blocks, ternary expressions,
    // heredocs, substitution, interpolation, and postfix conditionals.
    let source = r#"
use strict;
use warnings;

my @raw = qw(foo bar baz);
my @items = sort { $a cmp $b } map { uc $_ } grep { /ba/ } @raw;
my $message = @items ? "items" : "empty";
print "$message\n" if @items;

my $template = <<'TEXT';
hello $name
TEXT
$template =~ s/\$name/world/g;
"#;

    // When/Then: the full script remains a clean AST across statement,
    // expression, quote-like, and heredoc boundaries.
    assert_case_shape(ParserE2eCase {
        name: "cli-script",
        source,
        required_fragments: &[
            "foo",
            "bar",
            "baz",
            "sort",
            "map",
            "grep",
            "ternary",
            "statement_modifier",
            "heredoc",
            "substitution",
            "world",
        ],
    })
}

#[test]
fn e2e_recovery_reports_errors_but_keeps_following_declarations_and_assignments() {
    // Given: an editor buffer with valid statements before, between, and after
    // malformed edits.
    let source = r#"
my $ok_before = 1;
my $broken = ;
sub still_parsed { return 42; }
if ($oops > 0 { print $oops; }
my $ok_after = 2;
"#;

    // When: the recovery parser processes the complete buffer.
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let sexp = output.ast.to_sexp();

    // Then: diagnostics are surfaced, recovery markers are present, and later
    // valid declarations are still visible to downstream e2e users such as LSP.
    assert!(
        !output.diagnostics.is_empty(),
        "expected diagnostics for malformed e2e recovery input"
    );
    assert!(sexp.contains("ERROR"), "expected recovery ERROR nodes in: {sexp}");
    assert!(sexp.contains("ok_before"), "expected pre-error declaration in: {sexp}");
    assert!(sexp.contains("still_parsed"), "expected recovered subroutine in: {sexp}");
    assert!(sexp.contains("ok_after"), "expected post-error declaration in: {sexp}");
}
