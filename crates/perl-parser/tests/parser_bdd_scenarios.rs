//! Behavior-driven parser scenarios for high-value Perl syntax paths.
//!
//! These tests use a lightweight Given/When/Then harness so parser behavior can
//! be read as executable specifications.

mod support;

use perl_parser::Parser;
use support::parser_error_helpers::{assert_parse_error, assert_parse_success};

type TestResult = Result<(), String>;

struct BddScenario<'a> {
    name: &'a str,
    given: &'a str,
    expect_error_signal: bool,
    then_fragments: &'a [&'a str],
}

impl<'a> BddScenario<'a> {
    fn run(&self) -> TestResult {
        // Given
        let input = self.given;

        // When
        let mut parser = Parser::new(input);
        let ast = parser.parse().map_err(|e| format!("{}: parse failed: {}", self.name, e))?;
        let sexp = ast.to_sexp();

        // Then
        if self.expect_error_signal {
            assert_parse_error(input);
        } else {
            assert_parse_success(input);
        }

        for fragment in self.then_fragments {
            if !sexp.contains(fragment) {
                return Err(format!(
                    "{}: AST did not contain expected fragment: {}",
                    self.name, fragment
                ));
            }
        }

        Ok(())
    }
}

#[test]
fn bdd_core_parsing_behaviors() -> TestResult {
    let scenarios = [
        BddScenario {
            name: "Given a subroutine declaration, when parsing, then a subroutine node is present",
            given: "sub add { my ($a, $b) = @_; return $a + $b; }",
            expect_error_signal: false,
            then_fragments: &["sub"],
        },
        BddScenario {
            name: "Given heredoc input, when parsing, then heredoc content is captured",
            given: "my $text = <<'EOF';\nhello from bdd\nEOF\n",
            expect_error_signal: false,
            then_fragments: &["heredoc"],
        },
        BddScenario {
            name: "Given match and substitution operators, when parsing, then regex forms are represented",
            given: "$x =~ /foo/i; $x =~ s/foo/bar/g;",
            expect_error_signal: false,
            then_fragments: &["match", "substitution"],
        },
        BddScenario {
            name: "Given malformed control flow, when parsing, then recovery emits error signal",
            given: "if ($x { print $x;",
            expect_error_signal: true,
            then_fragments: &["ERROR"],
        },
    ];

    for scenario in scenarios {
        scenario.run()?;
    }

    Ok(())
}
