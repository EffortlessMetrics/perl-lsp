//! End-to-end parser conformance tests against a real Perl syntax oracle.
//!
//! These tests exercise whole-file parse flows instead of isolated grammar
//! productions: each fixture is first checked with `perl -c` in the controlled
//! test oracle environment, then parsed through `Parser::parse_with_recovery()`
//! and validated for a clean AST and key structural markers.

use perl_lsp_rs_core::config::PerlOracleEnv;
use perl_parser::Parser;
use std::fs;
use std::process::Output;
use tempfile::TempDir;

#[derive(Debug)]
struct ValidProgramCase {
    name: &'static str,
    source: &'static str,
    expected_fragments: &'static [&'static str],
}

fn perl_syntax_check(source: &str) -> Result<Option<Output>, Box<dyn std::error::Error>> {
    let Some(mut oracle) = PerlOracleEnv::for_dap_test_fixture() else {
        return Ok(None);
    };

    let temp_dir = TempDir::new()?;
    let source_path = temp_dir.path().join("fixture.pl");
    fs::write(&source_path, source)?;

    oracle.cwd = temp_dir.path().to_path_buf();
    oracle.extra_env.insert("LC_ALL".to_string(), "C".to_string());

    let mut command = oracle.into_command();
    let output = command.arg("-c").arg(&source_path).output()?;

    Ok(Some(output))
}

fn assert_real_perl_accepts(source: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let Some(output) = perl_syntax_check(source)? else {
        return Ok(());
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "real Perl rejected valid parser e2e fixture {name}: {stderr}"
    );

    Ok(())
}

fn assert_real_perl_rejects(source: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let Some(output) = perl_syntax_check(source)? else {
        return Ok(());
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "real Perl accepted invalid parser e2e fixture {name}: {stderr}"
    );

    Ok(())
}

fn assert_clean_parser_output(
    source: &str,
    name: &str,
    expected_fragments: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let sexp = output.ast.to_sexp();

    assert!(
        output.diagnostics.is_empty(),
        "parser emitted diagnostics for {name}: {:?}\nAST: {sexp}",
        output.diagnostics
    );
    assert_eq!(
        output.recovered_count, 0,
        "parser recovered from syntax for {name} despite clean Perl oracle acceptance\nAST: {sexp}"
    );
    assert!(!output.terminated_early, "parser terminated early for {name}\nAST: {sexp}");
    assert!(!sexp.contains("ERROR"), "parser produced ERROR nodes for {name}\nAST: {sexp}");

    for fragment in expected_fragments {
        assert!(
            sexp.contains(fragment),
            "parser e2e fixture {name} missing expected AST fragment {fragment:?}\nAST: {sexp}"
        );
    }

    Ok(())
}

#[test]
fn real_perl_valid_programs_parse_cleanly_end_to_end() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        ValidProgramCase {
            name: "package_sub_and_data_flow",
            source: r#"
                package Local::Greeter;
                use strict;
                use warnings;

                sub greet {
                    my ($name, @suffixes) = @_;
                    my %seen = map { $_ => 1 } @suffixes;
                    return join q{ }, 'hello', $name, sort keys %seen;
                }

                my $message = Local::Greeter::greet('Ada', qw(z b a));
                $message =~ s/\s+/ /g;
            "#,
            expected_fragments: &["Local::Greeter", "sub greet", "map", "sort", "substitution"],
        },
        ValidProgramCase {
            name: "heredoc_eval_and_postfix_conditionals",
            source: r#"
                my $banner = <<'END_BANNER';
                line one
                line two
END_BANNER

                my $ok = eval {
                    die 'empty' unless length $banner;
                    1;
                };
                warn $@ if !$ok;
            "#,
            expected_fragments: &["heredoc", "eval", "die", "unless", "warn"],
        },
        ValidProgramCase {
            name: "io_loop_regex_and_continue",
            source: r#"
                while (my $line = <DATA>) {
                    next if $line =~ /^#/;
                    chomp $line;
                    my ($key, $value) = split /=/, $line, 2;
                    print $key, q{:}, $value if defined $value;
                } continue {
                    $. = $.;
                }
                __DATA__
                alpha=1
                #skip
            "#,
            expected_fragments: &["while", "readline", "next", "function", "continue"],
        },
    ];

    for case in cases {
        assert_real_perl_accepts(case.source, case.name)?;
        assert_clean_parser_output(case.source, case.name, case.expected_fragments)?;
    }

    Ok(())
}

#[test]
fn invalid_program_recovers_without_losing_followup_statement()
-> Result<(), Box<dyn std::error::Error>> {
    let name = "missing_assignment_rhs_then_followup";
    let source = r#"
        my $broken = ;
        my $after = 42;
        print $after;
    "#;

    assert_real_perl_rejects(source, name)?;

    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let sexp = output.ast.to_sexp();

    assert!(
        !output.diagnostics.is_empty(),
        "parser should report diagnostics for invalid e2e fixture {name}\nAST: {sexp}"
    );
    assert!(
        sexp.contains("after"),
        "parser recovery lost follow-up declaration for invalid e2e fixture {name}\nAST: {sexp}"
    );
    assert!(
        sexp.contains("print"),
        "parser recovery lost follow-up print statement for invalid e2e fixture {name}\nAST: {sexp}"
    );

    Ok(())
}
