//! Tests for PL_sv_* internal special variable recognition — issue #3542
//!
//! $PL_sv_yes, $PL_sv_no, and $PL_sv_undef are Perl internal special values
//! used in XS/C extension code and introspection. The scope analyzer must not
//! report them as undeclared variables under `use strict`.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue};
use perl_semantic_analyzer::pragma_tracker::PragmaTracker;
use perl_tdd_support::must;

fn scope_issues(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let pragma_map = PragmaTracker::build(&ast);
    let analyzer = ScopeAnalyzer::new();
    analyzer.analyze(&ast, code, &pragma_map)
}

#[test]
fn pl_sv_yes_not_undeclared_under_strict() -> Result<(), Box<dyn std::error::Error>> {
    // $PL_sv_yes represents internal true value in Perl internals (perlguts).
    // Must not trigger UndeclaredVariable under strict.
    let code = r#"
use strict;
use warnings;

if ($PL_sv_yes) {
    print "true\n";
}
"#;
    let issues = scope_issues(code);
    let fp: Vec<_> = issues
        .iter()
        .filter(|i| {
            i.kind == IssueKind::UndeclaredVariable && i.variable_name.contains("PL_sv_yes")
        })
        .collect();
    assert!(
        fp.is_empty(),
        "$PL_sv_yes must not be reported as undeclared under strict; got: {:?}",
        fp
    );
    Ok(())
}

#[test]
fn pl_sv_no_not_undeclared_under_strict() -> Result<(), Box<dyn std::error::Error>> {
    // $PL_sv_no represents internal false value in Perl internals (perlguts).
    // Must not trigger UndeclaredVariable under strict.
    let code = r#"
use strict;
use warnings;

if (!$PL_sv_no) {
    print "false\n";
}
"#;
    let issues = scope_issues(code);
    let fp: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name.contains("PL_sv_no"))
        .collect();
    assert!(
        fp.is_empty(),
        "$PL_sv_no must not be reported as undeclared under strict; got: {:?}",
        fp
    );
    Ok(())
}

#[test]
fn pl_sv_undef_not_undeclared_under_strict() -> Result<(), Box<dyn std::error::Error>> {
    // $PL_sv_undef is the internal undef value in Perl internals (perlguts).
    // Must not trigger UndeclaredVariable under strict.
    let code = r#"
use strict;
use warnings;

my $val = $PL_sv_undef;
print defined($val) ? "defined" : "undef";
"#;
    let issues = scope_issues(code);
    let fp: Vec<_> = issues
        .iter()
        .filter(|i| {
            i.kind == IssueKind::UndeclaredVariable && i.variable_name.contains("PL_sv_undef")
        })
        .collect();
    assert!(
        fp.is_empty(),
        "$PL_sv_undef must not be reported as undeclared under strict; got: {:?}",
        fp
    );
    Ok(())
}

#[test]
fn pl_sv_vars_all_together_no_undeclared() -> Result<(), Box<dyn std::error::Error>> {
    // All three PL_sv_* variables used together must produce zero UndeclaredVariable issues.
    let code = r#"
use strict;
use warnings;

sub check_sv {
    my $x = $PL_sv_yes;
    my $y = $PL_sv_no;
    my $z = $PL_sv_undef;
    return ($x, $y, $z);
}
"#;
    let issues = scope_issues(code);
    let fp: Vec<_> = issues
        .iter()
        .filter(|i| {
            i.kind == IssueKind::UndeclaredVariable
                && (i.variable_name.contains("PL_sv_yes")
                    || i.variable_name.contains("PL_sv_no")
                    || i.variable_name.contains("PL_sv_undef"))
        })
        .collect();
    assert!(
        fp.is_empty(),
        "PL_sv_* variables together must not be reported as undeclared; got: {:?}",
        fp
    );
    Ok(())
}
