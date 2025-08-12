use perl_parser::{
    Parser,
    pragma_tracker::PragmaTracker,
    scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue},
};

fn analyze_code(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Failed to parse");
    let analyzer = ScopeAnalyzer::new();
    let pragma_map = PragmaTracker::build(&ast);
    analyzer.analyze(&ast, code, &pragma_map)
}

#[test]
fn test_undefined_variable_detection() {
    let code = r#"
use strict;
print $undefined_var;
"#;

    let issues = analyze_code(code);
    assert!(
        issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
    );
}

#[test]
fn test_unused_variable_detection() {
    let code = r#"
use warnings;
my $unused = 42;
print "Hello";
"#;

    let issues = analyze_code(code);
    assert!(
        issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UnusedVariable))
    );
}

#[test]
fn test_variable_shadowing() {
    let code = r#"
my $x = 1;
{
    my $x = 2;  # shadows outer $x
    print $x;
}
"#;

    let issues = analyze_code(code);
    assert!(
        issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::VariableShadowing))
    );
}

#[test]
fn test_our_variable_not_undefined() {
    let code = r#"
use strict;
our $global_var = 42;
print $global_var;
"#;

    let issues = analyze_code(code);
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
    );
}

#[test]
fn test_local_variable_not_undefined() {
    let code = r#"
use strict;
local $ENV{PATH} = '/usr/bin';
print $ENV{PATH};
"#;

    let issues = analyze_code(code);
    // $ENV is a built-in global
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable) && (i.variable_name == "$ENV")));
}

#[test]
fn test_package_variable_tracking() {
    let code = r#"
package Foo;
use strict;
our $package_var = 42;

package Bar;
use strict;
print $Foo::package_var;  # Should be ok
"#;

    let issues = analyze_code(code);
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
                && (i.variable_name == "$Foo::package_var"))
    );
}

#[test]
fn test_subroutine_parameters() {
    let code = r#"
use strict;
sub foo {
    my ($x, $y) = @_;
    return $x + $y;
}
"#;

    let issues = analyze_code(code);
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
    );
}

#[test]
fn test_foreach_loop_variable() {
    let code = r#"
use strict;
my @arr = (1, 2, 3);
foreach my $item (@arr) {
    print $item;
}
"#;

    let issues = analyze_code(code);
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
    );
}

#[test]
fn test_while_loop_variable() {
    let code = r#"
use strict;
while (my $line = <STDIN>) {
    print $line;
}
"#;

    let issues = analyze_code(code);
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
    );
}

#[test]
fn test_if_condition_variable() {
    let code = r#"
use strict;
if (my $result = some_func()) {
    print $result;
}
"#;

    let issues = analyze_code(code);
    // some_func is undefined but $result should be ok
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
                && (i.variable_name == "$result"))
    );
}

#[test]
fn test_nested_scopes() {
    let code = r#"
use strict;
my $outer = 1;
{
    my $inner = 2;
    print $outer;  # ok
    print $inner;  # ok
}
print $outer;  # ok
print $inner;  # undefined
"#;

    let issues = analyze_code(code);
    assert!(
        issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
                && (i.variable_name == "$inner"))
    );
}

#[test]
fn test_builtin_variables() {
    let code = r#"
use strict;
print $_;
print @ARGV;
print %ENV;
print $0;
print $!;
print $@;
print $$;
"#;

    let issues = analyze_code(code);
    // None of these should be undefined
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
    );
}

#[test]
fn test_unused_variable_basic() {
    let code = r#"
my $unused = 42;
"#;

    let issues = analyze_code(code);

    // Check for unused variables
    let unused_issues: Vec<_> = issues
        .iter()
        .filter(|i| matches!(i.kind, IssueKind::UnusedVariable))
        .collect();
    assert!(!unused_issues.is_empty());
}

#[test]
fn test_my_declaration_in_list() {
    let code = r#"
use strict;
my ($x, $y, $z) = (1, 2, 3);
print "$x $y $z";
"#;

    let issues = analyze_code(code);
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
    );
}

#[test]
fn test_hash_slice_not_undefined() {
    let code = r#"
use strict;
my %hash = (a => 1, b => 2);
my @values = @hash{'a', 'b'};
print @values;
"#;

    let issues = analyze_code(code);
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
    );
}

#[test]
fn test_array_slice_not_undefined() {
    let code = r#"
use strict;
my @array = (1, 2, 3, 4);
my @slice = @array[0..2];
print @slice;
"#;

    let issues = analyze_code(code);
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
    );
}

#[test]
fn test_special_blocks() {
    let code = r#"
use strict;
BEGIN {
    my $begin_var = 1;
    print $begin_var;  # ok
}
END {
    my $end_var = 2;
    print $end_var;  # ok
}
"#;

    let issues = analyze_code(code);
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
    );
}

#[test]
fn test_use_vars_pragma() {
    let code = r#"
use strict;
use vars qw($global_var @global_array);
$global_var = 42;
@global_array = (1, 2, 3);
"#;

    let issues = analyze_code(code);
    // Variables declared with 'use vars' should not be undefined
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
    );
}

#[test]
fn test_glob_assignment() {
    let code = r#"
use strict;
*alias = *main::original;
"#;

    let issues = analyze_code(code);
    // Glob assignments are special and shouldn't trigger undefined warnings
    // Note: This might need special handling in the analyzer
    assert!(issues.len() <= 2); // Only pragma warnings expected
}

#[test]
fn test_state_variable() {
    let code = r#"
use strict;
use feature 'state';
sub counter {
    state $count = 0;
    return ++$count;
}
"#;

    let issues = analyze_code(code);
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
    );
}

#[test]
fn test_regex_capture_variables() {
    let code = r#"
use strict;
my $text = "hello world";
if ($text =~ /(\w+)\s+(\w+)/) {
    print "$1 $2";  # Capture variables should be recognized
}
"#;

    let issues = analyze_code(code);
    // $1, $2 etc. are special regex capture variables
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
                && (i.variable_name == "$1" || i.variable_name == "$2"))
    );
}

#[test]
fn test_eval_block() {
    let code = r#"
use strict;
eval {
    my $eval_var = 42;
    print $eval_var;  # ok
};
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(
        |i| matches!(i.kind, IssueKind::UndeclaredVariable) && (i.variable_name == "$eval_var")
    ));
}

#[test]
fn test_do_block() {
    let code = r#"
use strict;
my $result = do {
    my $temp = 42;
    $temp * 2
};
print $result;
"#;

    let issues = analyze_code(code);
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
    );
}

#[test]
fn test_given_when() {
    let code = r#"
use strict;
use feature 'switch';
my $value = 5;
given ($value) {
    when (1) { print "one" }
    when (2) { print "two" }
    default { print "other" }
}
"#;

    let issues = analyze_code(code);
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
    );
}
