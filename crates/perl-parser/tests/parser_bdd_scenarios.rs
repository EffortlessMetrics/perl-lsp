//! Behavior-driven parser scenarios for core Perl workflows.
//!
//! The goal of this suite is to keep high-value parser behavior readable as
//! executable user stories.

use perl_parser::Parser;
use perl_tdd_support::must;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn parse_sexp(code: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    Ok(ast.to_sexp())
}

#[test]
fn bdd_given_named_sub_with_assignment_when_parsed_then_ast_contains_subroutine_and_assignment()
-> TestResult {
    // Given: a developer writes a basic subroutine that mutates a lexical scalar.
    let code = r#"
        sub greet {
            my $name = "world";
            $name = "perl";
            return $name;
        }
    "#;

    // When: the parser processes the source.
    let sexp = parse_sexp(code)?;

    // Then: core semantic structure appears in the AST.
    assert!(sexp.contains("sub "), "Expected Subroutine node in: {sexp}");
    assert!(sexp.contains("assignment_"), "Expected Assignment node in: {sexp}");
    assert!(sexp.contains("(return"), "Expected Return node in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_regex_substitution_when_parsed_then_pattern_replacement_and_flags_are_retained()
-> TestResult {
    // Given: a developer normalizes identifiers with substitution flags.
    let code = r#"$value =~ s/(\w+)/prefix_$1/gi;"#;

    // When: the parser processes the statement.
    let sexp = parse_sexp(code)?;

    // Then: regex substitution semantics are preserved in AST text form.
    assert!(sexp.contains("substitution"), "Expected Substitution node in: {sexp}");
    assert!(sexp.contains("prefix_$1"), "Expected replacement text in: {sexp}");
    assert!(sexp.contains("gi"), "Expected modifier flags in: {sexp}");
    assert!(
        !sexp.contains("ERROR"),
        "Did not expect recovery ERROR nodes for valid substitution: {sexp}"
    );

    Ok(())
}

#[test]
fn bdd_given_match_switch_when_parsed_then_given_and_when_constructs_are_present() -> TestResult {
    // Given: a developer uses Perl given/when smart-match control flow.
    let code = r#"
        given ($topic) {
            when (/^foo/) { print "foo"; }
            when (/^bar/) { print "bar"; }
            default { print "other"; }
        }
    "#;

    // When: the parser builds syntax trees.
    let sexp = parse_sexp(code)?;

    // Then: control-flow specific nodes are present and parse is clean.
    assert!(sexp.contains("(given "), "Expected Given node in: {sexp}");
    assert!(sexp.contains("(when "), "Expected When node in: {sexp}");
    assert!(sexp.contains("(default "), "Expected Default node in: {sexp}");
    assert!(
        !sexp.contains("ERROR"),
        "Did not expect recovery ERROR nodes for valid given/when: {sexp}"
    );

    Ok(())
}

#[test]
fn bdd_given_incomplete_if_when_parsed_then_parser_recovers_with_error_nodes_instead_of_crashing() {
    // Given: a developer is in the middle of editing incomplete syntax.
    let code = "if ($x > 10 { print $x;";

    // When: the parser processes malformed incremental text.
    let mut parser = Parser::new(code);
    let result = parser.parse();

    // Then: parser should recover, producing either ParseError or ERROR AST node.
    match result {
        Ok(ast) => {
            let sexp = ast.to_sexp();
            assert!(
                sexp.contains("ERROR"),
                "Expected ERROR recovery node for malformed input: {sexp}"
            );
        }
        Err(err) => {
            let message = err.to_string();
            assert!(!message.is_empty(), "Expected diagnostic message when parse returns Err");
        }
    }
}

#[test]
fn bdd_given_multiple_realistic_statements_when_parsed_then_program_shape_is_stable() {
    // Given: a small realistic script using strict/warnings, loops, and conditionals.
    let code = r#"
        use strict;
        use warnings;

        my @values = (1, 2, 3);
        for my $v (@values) {
            if ($v % 2 == 0) {
                print "even";
            } else {
                print "odd";
            }
        }
    "#;

    // When: the parser builds the full AST.
    let sexp = must(parse_sexp(code));

    // Then: top-level shape includes declarations and structured control flow.
    assert!(sexp.contains("(use "), "Expected Use declarations in: {sexp}");
    assert!(sexp.contains("(for") || sexp.contains("(foreach"), "Expected loop node in: {sexp}");
    assert!(sexp.contains("(if "), "Expected If node in: {sexp}");
    assert!(
        !sexp.contains("ERROR"),
        "Did not expect recovery ERROR nodes for valid script: {sexp}"
    );
}

#[test]
fn bdd_given_postfix_flow_and_ternary_when_parsed_then_control_flow_nodes_are_retained()
-> TestResult {
    // Given: a developer writes concise Perl with postfix conditionals and ternary expressions.
    let code = r#"
        my $count = 2;
        print "nonzero" if $count;
        my $label = $count > 1 ? "many" : "one";
    "#;

    // When: the parser processes the snippet.
    let sexp = parse_sexp(code)?;

    // Then: compact control-flow structure remains visible in AST output.
    assert!(sexp.contains("statement_modifier"), "Expected statement modifier node in: {sexp}");
    assert!(sexp.contains("ternary"), "Expected ternary node in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_unclosed_quote_when_parsed_then_recovery_is_reported_without_panicking() {
    // Given: a developer is typing and leaves a quoted string unfinished.
    let code = r#"my $name = "perl; print $name;"#;

    // When: the parser attempts to build an AST.
    let mut parser = Parser::new(code);
    let result = parser.parse();

    // Then: parser should return an error or emit recovery nodes, but never panic.
    match result {
        Ok(ast) => {
            let sexp = ast.to_sexp();
            assert!(
                sexp.contains("ERROR") || sexp.contains("unknown"),
                "Expected recovery marker for malformed quoted string: {sexp}"
            );
        }
        Err(err) => {
            assert!(!err.to_string().is_empty(), "Expected non-empty parse failure message");
        }
    }
}

#[test]
fn bdd_given_package_and_constructor_pattern_when_parsed_then_namespace_and_bless_flow_are_preserved()
-> TestResult {
    // Given: a developer writes a package with a constructor that blesses a hashref.
    let code = r#"
        package My::Service;
        use strict;
        use warnings;

        sub new {
            my ($class, %args) = @_;
            my $self = bless { %args }, $class;
            return $self;
        }
    "#;

    // When: the parser processes this object-construction pattern.
    let sexp = parse_sexp(code)?;

    // Then: namespace + constructor flow is represented without recovery artifacts.
    assert!(sexp.contains("My::Service"), "Expected package name in AST output: {sexp}");
    assert!(sexp.contains("bless"), "Expected bless call in AST output: {sexp}");
    assert!(sexp.contains("(return"), "Expected Return node in: {sexp}");
    assert!(!sexp.contains("ERROR"), "Did not expect recovery ERROR nodes for valid code: {sexp}");

    Ok(())
}

#[test]
fn bdd_given_partial_hashref_literal_when_parsed_then_parser_recovers_without_panicking() {
    // Given: a developer is typing a hashref literal and stops mid-expression.
    let code = r#"
        my $cfg = {
            host => "localhost",
            port =>
    "#;

    // When: the parser attempts to process incomplete input.
    let mut parser = Parser::new(code);
    let result = parser.parse();

    // Then: parser should recover (ERROR node) or return a descriptive parse failure.
    match result {
        Ok(ast) => {
            let sexp = ast.to_sexp();
            assert!(
                sexp.contains("ERROR") || sexp.contains("unknown"),
                "Expected recovery marker for incomplete hashref literal: {sexp}"
            );
        }
        Err(err) => {
            assert!(!err.to_string().is_empty(), "Expected non-empty parse failure message");
        }
    }
}
