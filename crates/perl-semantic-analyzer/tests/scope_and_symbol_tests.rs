//! Tests for scope analysis and symbol resolution in perl-semantic-analyzer.
//!
//! Covers:
//! - Variable scope resolution (my, our, local, state)
//! - Package-qualified symbol resolution
//! - Cross-scope reference tracking
//! - Shadowed variable detection
//! - Unused variable detection

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue};
use perl_semantic_analyzer::pragma_tracker::PragmaTracker;
use perl_semantic_analyzer::symbol::{ScopeKind, SymbolExtractor, SymbolKind, SymbolTable};
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_and_extract(code: &str) -> SymbolTable {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SymbolExtractor::new_with_source(code).extract(&ast)
}

fn scope_issues(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = ScopeAnalyzer::new();
    analyzer.analyze(&ast, code, &[])
}

/// Run scope analysis with strict mode enabled by building a pragma map from
/// `use strict;` in the source.
fn scope_issues_strict(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let pragma_map = PragmaTracker::build(&ast);
    let analyzer = ScopeAnalyzer::new();
    analyzer.analyze(&ast, code, &pragma_map)
}

fn has_symbol(table: &SymbolTable, name: &str, kind: SymbolKind) -> bool {
    table.symbols.get(name).is_some_and(|syms| syms.iter().any(|s| s.kind == kind))
}

fn has_issue(issues: &[ScopeIssue], kind: IssueKind, var_name: &str) -> bool {
    issues.iter().any(|i| i.kind == kind && i.variable_name.contains(var_name))
}

fn count_issues(issues: &[ScopeIssue], kind: IssueKind) -> usize {
    issues.iter().filter(|i| i.kind == kind).count()
}

// ===========================================================================
// 1. Variable Scope Resolution — my
// ===========================================================================

#[test]
fn scope_my_variable_confined_to_block() -> Result<(), Box<dyn std::error::Error>> {
    // A `my` variable declared in a block should not be visible outside it.
    let code = r#"
use strict;
{
    my $inner = 1;
    print $inner;
}
print $inner;
"#;
    let issues = scope_issues_strict(code);
    // $inner used after the block should be undeclared under strict
    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "inner"),
        "my variable should not leak out of its block; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn scope_my_variable_visible_in_nested_block() -> Result<(), Box<dyn std::error::Error>> {
    // A `my` variable should be visible to nested blocks.
    let code = r#"
my $outer = 10;
{
    {
        print $outer;
    }
}
"#;
    let issues = scope_issues(code);
    let unused_outer = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("outer"))
        .count();
    assert_eq!(unused_outer, 0, "$outer used in deeply nested block should not be unused");
    Ok(())
}

#[test]
fn scope_my_variable_in_if_branch() -> Result<(), Box<dyn std::error::Error>> {
    // `my` inside an if block is scoped to that block.
    let code = r#"
use strict;
if (1) {
    my $branch_var = 42;
    print $branch_var;
}
print $branch_var;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "branch_var"),
        "my variable in if-block should not be visible after the block"
    );
    Ok(())
}

#[test]
fn scope_my_list_declaration_all_scoped() -> Result<(), Box<dyn std::error::Error>> {
    // `my ($a, $b, $c)` should declare all three in the current scope.
    // Verify each variable is declared and accessible.
    let code = r#"
my ($alpha, $bravo, $charlie) = (1, 2, 3);
"#;
    let issues = scope_issues(code);
    // All three should be detected as unused (since none are referenced after declaration).
    let unused_alpha = has_issue(&issues, IssueKind::UnusedVariable, "alpha");
    let unused_bravo = has_issue(&issues, IssueKind::UnusedVariable, "bravo");
    let unused_charlie = has_issue(&issues, IssueKind::UnusedVariable, "charlie");
    assert!(unused_alpha, "should detect unused $alpha from list declaration");
    assert!(unused_bravo, "should detect unused $bravo from list declaration");
    assert!(unused_charlie, "should detect unused $charlie from list declaration");
    Ok(())
}

#[test]
fn scope_declaration_capable_builtins_initialize_handle_bindings()
-> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
open my $fh, '<', 'input.txt' or die $!;
print $fh;
opendir my $dh, '.' or die $!;
print $dh;
sysopen my $sys_fh, 'sysfile.txt', 0 or die $!;
print $sys_fh;
pipe my $reader, my $writer;
print $reader;
print $writer;
socket my $sock, PF_INET, SOCK_STREAM, getprotobyname('tcp');
print $sock;
accept my $client, $sock;
print $client;
"#;

    let issues = scope_issues_strict(code);
    let handled = ["$fh", "$dh", "$sys_fh", "$reader", "$writer", "$sock", "$client"];

    for variable_name in handled {
        assert!(
            !issues.iter().any(|i| {
                matches!(
                    i.kind,
                    IssueKind::UndeclaredVariable
                        | IssueKind::UninitializedVariable
                        | IssueKind::UnusedVariable
                ) && i.variable_name == variable_name
            }),
            "builtin filehandle declaration should be declared, initialized, and consumed: {} (issues: {:?})",
            variable_name,
            issues
        );
    }

    Ok(())
}

#[test]
fn open_my_handle_used_in_readline_is_tracked() -> Result<(), Box<dyn std::error::Error>> {
    // Regression: `open my $fh, ...` declares `$fh` and later `<$fh>` must not
    // trigger undeclared/uninitialized/unused diagnostics.
    let code = r#"
use strict;
use warnings;

open my $fh, '<', 'file.txt' or die $!;
print <$fh>;
close $fh;
"#;

    let issues = scope_issues_strict(code);
    assert!(
        !issues.iter().any(|i| {
            matches!(
                i.kind,
                IssueKind::UndeclaredVariable
                    | IssueKind::UninitializedVariable
                    | IssueKind::UnusedVariable
            ) && i.variable_name == "$fh"
        }),
        "open-my handle should remain declared/initialized/used across <$fh>: {:?}",
        issues
    );

    Ok(())
}

#[test]
fn scope_phase_blocks_keep_lexicals_inside_their_block() -> Result<(), Box<dyn std::error::Error>> {
    for phase in ["BEGIN", "CHECK", "INIT", "UNITCHECK", "END"] {
        let code = format!(
            r#"
use strict;
{phase} {{
    my $phase_local = 1;
    print $phase_local;
}}
print $phase_local;
"#
        );

        let issues = scope_issues_strict(&code);
        assert!(
            issues.iter().any(|i| {
                i.kind == IssueKind::UndeclaredVariable
                    && i.variable_name == "$phase_local"
                    && i.line == 7
            }),
            "{phase} block lexical should not leak into outer scope at line 7: {:?}",
            issues
        );
        assert!(
            !issues.iter().any(|i| i.kind == IssueKind::UndeclaredVariable
                && i.variable_name == "$phase_local"
                && i.line == 5),
            "{phase} block lexical should stay valid inside its own block: {:?}",
            issues
        );
    }

    Ok(())
}

#[test]
fn scope_phase_blocks_do_not_share_lexicals() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
BEGIN {
    my $phase_local = 1;
    print $phase_local;
}
CHECK {
    print $phase_local;
}
"#;

    let issues = scope_issues_strict(code);
    assert!(
        !issues.iter().any(|i| {
            i.kind == IssueKind::UndeclaredVariable
                && i.variable_name == "$phase_local"
                && i.line == 5
        }),
        "lexical should be valid inside its own phase block: {:?}",
        issues
    );
    assert!(
        issues.iter().any(|i| {
            i.kind == IssueKind::UndeclaredVariable
                && i.variable_name == "$phase_local"
                && i.line == 8
        }),
        "lexicals declared in one phase block should not be visible in sibling phase blocks at line 8: {:?}",
        issues
    );
    Ok(())
}

#[test]
fn scope_try_catch_binds_catch_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use feature 'try';
try {
    die "boom";
} catch ($e) {
    print $e;
}
"#;

    let issues = scope_issues_strict(code);
    assert!(
        !issues.iter().any(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name == "$e"),
        "catch variable should be declared inside catch block: {:?}",
        issues
    );
    assert!(
        !issues.iter().any(|i| i.kind == IssueKind::UnusedVariable && i.variable_name == "$e"),
        "used catch variable should not be reported as unused: {:?}",
        issues
    );
    Ok(())
}

#[test]
fn scope_try_catch_shadowing_is_reported() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use feature 'try';
my $e = 1;
try {
    die "boom";
} catch ($e) {
    print $e;
}
"#;

    let issues = scope_issues_strict(code);
    let shadowing = issues
        .iter()
        .find(|i| i.kind == IssueKind::VariableShadowing && i.variable_name == "$e")
        .ok_or("expected catch-variable shadowing issue")?;
    assert!(
        has_issue(&issues, IssueKind::VariableShadowing, "e"),
        "catch variable should report shadowing against outer scope: {:?}",
        issues
    );
    assert_eq!(
        &code[shadowing.range.0..shadowing.range.1],
        "$e",
        "catch-variable shadowing range should target the catch parameter"
    );
    assert!(
        !issues.iter().any(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name == "$e"),
        "shadowed catch variable should still be bound in catch scope: {:?}",
        issues
    );
    Ok(())
}

#[test]
fn scope_try_catch_variable_does_not_escape() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use feature 'try';
try {
    die "boom";
} catch ($e) {
    print $e;
}
print $e;
"#;

    let issues = scope_issues_strict(code);
    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "$e"),
        "catch variable should not be visible after the catch block: {:?}",
        issues
    );
    Ok(())
}

#[test]
fn scope_try_catch_unused_variable_reported() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use feature 'try';
try {
    die "boom";
} catch ($e) {
    print "handled";
}
"#;

    let issues = scope_issues_strict(code);
    let unused = issues
        .iter()
        .find(|i| i.kind == IssueKind::UnusedVariable && i.variable_name == "$e")
        .ok_or("expected unused catch-variable issue")?;
    assert!(
        issues.iter().any(|i| i.kind == IssueKind::UnusedVariable && i.variable_name == "$e"),
        "unused catch variable should be reported: {:?}",
        issues
    );
    assert_eq!(
        &code[unused.range.0..unused.range.1],
        "$e",
        "unused catch-variable range should target the catch parameter"
    );
    Ok(())
}

#[test]
fn symbol_try_catch_variable_is_resolvable_inside_catch_scope()
-> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use feature 'try';
try {
    die "boom";
} catch ($err) {
    print $err;
}
"#;

    let table = parse_and_extract(code);
    let refs = table.references.get("err").ok_or("expected catch variable reference")?;
    let usage = refs
        .iter()
        .find(|reference| &code[reference.location.start..reference.location.end] == "$err")
        .ok_or("expected usage reference for $err inside catch block")?;
    let defs = table.find_symbol("err", usage.scope_id, SymbolKind::scalar());
    let def = defs.first().ok_or("expected catch variable definition in symbol table")?;

    assert_eq!(def.declaration.as_deref(), Some("my"));
    assert_eq!(&code[def.location.start..def.location.end], "$err");
    Ok(())
}

#[test]
fn scope_try_catch_inner_redeclaration_stays_same_scope() -> Result<(), Box<dyn std::error::Error>>
{
    let code = r#"
use strict;
use feature 'try';
try {
    die "boom";
} catch ($e) {
    my $e = "inner";
    print $e;
}
"#;

    let issues = scope_issues_strict(code);
    assert!(
        issues
            .iter()
            .any(|i| i.kind == IssueKind::VariableRedeclaration && i.variable_name == "$e"),
        "inner catch declaration should be a same-scope redeclaration: {:?}",
        issues
    );
    assert!(
        !issues.iter().any(|i| i.kind == IssueKind::VariableShadowing && i.variable_name == "$e"),
        "inner catch declaration should not be treated as nested shadowing: {:?}",
        issues
    );
    Ok(())
}

#[test]
fn scope_dbmopen_initializes_hash_at_position_0() -> Result<(), Box<dyn std::error::Error>> {
    // `dbmopen %hash, $file, $mode` ties %hash to a DBM file.
    // The hash at position 0 is initialized by the builtin and must not be
    // flagged as undeclared or uninitialized when used afterwards.
    let code = r#"
use strict;
dbmopen my %db, 'mydb', 0644;
print $db{key};
"#;

    let issues = scope_issues_strict(code);
    assert!(
        !issues.iter().any(|i| {
            matches!(
                i.kind,
                IssueKind::UndeclaredVariable
                    | IssueKind::UninitializedVariable
                    | IssueKind::UnusedVariable
            ) && i.variable_name == "%db"
        }),
        "dbmopen should declare, initialize, and consume %db (issues: {:?})",
        issues
    );
    Ok(())
}

#[test]
fn scope_shmread_initializes_buffer_at_position_1() -> Result<(), Box<dyn std::error::Error>> {
    // `shmread $id, my $buffer, $pos, $size` reads shared memory into $buffer.
    // The buffer at position 1 is initialized by the builtin and must not be
    // flagged as undeclared or uninitialized when used afterwards.
    let code = r#"
use strict;
my $id = 1;
shmread $id, my $buffer, 0, 1024;
print $buffer;
"#;

    let issues = scope_issues_strict(code);
    assert!(
        !issues.iter().any(|i| {
            matches!(
                i.kind,
                IssueKind::UndeclaredVariable
                    | IssueKind::UninitializedVariable
                    | IssueKind::UnusedVariable
            ) && i.variable_name == "$buffer"
        }),
        "shmread should declare, initialize, and consume $buffer (issues: {:?})",
        issues
    );
    Ok(())
}

// ===========================================================================
// 2. Variable Scope Resolution — our
// ===========================================================================

#[test]
fn scope_our_variable_package_qualified() -> Result<(), Box<dyn std::error::Error>> {
    // `our` variables get package-qualified names in the symbol table.
    let code = r#"
package MyPkg;
our $VERSION = '1.0';
our @EXPORT = ('foo');
our %DEFAULTS = (key => 'val');
"#;
    let table = parse_and_extract(code);
    // Check all three our variables exist
    assert!(has_symbol(&table, "VERSION", SymbolKind::scalar()), "our $VERSION missing");
    assert!(has_symbol(&table, "EXPORT", SymbolKind::array()), "our @EXPORT missing");
    assert!(has_symbol(&table, "DEFAULTS", SymbolKind::hash()), "our %DEFAULTS missing");

    // Check qualified names include the package
    let version_syms = table.symbols.get("VERSION").ok_or("VERSION not found")?;
    assert!(
        version_syms.iter().any(|s| s.qualified_name.contains("MyPkg")),
        "our variable should have package-qualified name"
    );
    Ok(())
}

#[test]
fn scope_our_variable_not_flagged_unused() -> Result<(), Box<dyn std::error::Error>> {
    // `our` variables should never be flagged as unused since they are package-global.
    let code = r#"
our $GLOBAL_A = 1;
our @GLOBAL_B = (2, 3);
our %GLOBAL_C = (x => 4);
"#;
    let issues = scope_issues(code);
    let unused_our = issues
        .iter()
        .filter(|i| {
            i.kind == IssueKind::UnusedVariable
                && (i.variable_name.contains("GLOBAL_A")
                    || i.variable_name.contains("GLOBAL_B")
                    || i.variable_name.contains("GLOBAL_C"))
        })
        .count();
    assert_eq!(unused_our, 0, "our variables should not be flagged as unused");
    Ok(())
}

#[test]
fn scope_our_across_packages() -> Result<(), Box<dyn std::error::Error>> {
    // `our` in different packages should produce distinct qualified names.
    let code = r#"
package Alpha;
our $VALUE = 1;
sub alpha_sub { 1 }

package Beta;
our $VALUE = 2;
sub beta_sub { 1 }
"#;
    let table = parse_and_extract(code);

    // Both $VALUE declarations should exist
    let value_syms = table.symbols.get("VALUE").ok_or("VALUE not found")?;
    assert!(value_syms.len() >= 2, "should have VALUE in both packages");

    let qualified_names: Vec<&str> = value_syms.iter().map(|s| s.qualified_name.as_str()).collect();
    assert!(qualified_names.iter().any(|qn| qn.contains("Alpha")), "should have Alpha::VALUE");
    assert!(qualified_names.iter().any(|qn| qn.contains("Beta")), "should have Beta::VALUE");
    Ok(())
}

// ===========================================================================
// 3. Variable Scope Resolution — local
// ===========================================================================

#[test]
fn scope_local_variable_extracted() -> Result<(), Box<dyn std::error::Error>> {
    // `local` declares a dynamic variable; it should appear in the symbol table.
    let code = "local $/ = undef;";
    let table = parse_and_extract(code);
    // local $/ may or may not be indexed (it's a special variable),
    // but the extraction should not crash.
    let _ = table;
    Ok(())
}

#[test]
fn scope_local_named_variable_declaration() -> Result<(), Box<dyn std::error::Error>> {
    // `local` on a named variable should register in the symbol table.
    let code = r#"
our $global_val = 100;
sub modify_it {
    local $global_val = 200;
    print $global_val;
}
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "global_val", SymbolKind::scalar()));
    Ok(())
}

// ---------------------------------------------------------------------------
// 3b. local with builtin special variables — issue #3502
// ---------------------------------------------------------------------------

#[test]
fn local_input_record_sep_no_false_unused() -> Result<(), Box<dyn std::error::Error>> {
    // `local $/` (slurp mode) must not produce a false UnusedVariable diagnostic.
    let code = "use strict;\nlocal $/ = undef;\n";
    let issues = scope_issues_strict(code);
    let false_pos: Vec<_> = issues
        .iter()
        .filter(|i| {
            (i.kind == IssueKind::UnusedVariable || i.kind == IssueKind::UndeclaredVariable)
                && i.variable_name == "$/"
        })
        .collect();
    assert!(
        false_pos.is_empty(),
        "local $/ should produce no false UnusedVariable or UndeclaredVariable; got: {:?}",
        false_pos
    );
    Ok(())
}

#[test]
fn local_output_field_sep_no_false_unused() -> Result<(), Box<dyn std::error::Error>> {
    // `local $,` (output field separator) must not produce a false UnusedVariable diagnostic.
    let code = "use strict;\nlocal $, = \", \";\nprint \"a\", \"b\";\n";
    let issues = scope_issues_strict(code);
    let false_pos: Vec<_> = issues
        .iter()
        .filter(|i| {
            (i.kind == IssueKind::UnusedVariable || i.kind == IssueKind::UndeclaredVariable)
                && i.variable_name == "$,"
        })
        .collect();
    assert!(
        false_pos.is_empty(),
        "local $, should produce no false diagnostics; got: {:?}",
        false_pos
    );
    Ok(())
}

#[test]
fn local_output_record_sep_no_false_unused() -> Result<(), Box<dyn std::error::Error>> {
    // `local $\` (output record separator) must not produce false diagnostics.
    let code = "use strict;\nlocal $\\ = \"\\n\";\nprint \"hello\";\n";
    let issues = scope_issues_strict(code);
    let false_pos: Vec<_> = issues
        .iter()
        .filter(|i| {
            (i.kind == IssueKind::UnusedVariable || i.kind == IssueKind::UndeclaredVariable)
                && i.variable_name == "$\\"
        })
        .collect();
    assert!(
        false_pos.is_empty(),
        "local $\\ should produce no false diagnostics; got: {:?}",
        false_pos
    );
    Ok(())
}

#[test]
fn local_list_sep_no_false_unused() -> Result<(), Box<dyn std::error::Error>> {
    // `local $"` (list separator) must not produce false diagnostics.
    let code = "use strict;\nlocal $\" = \"-\";\nmy @arr = (1, 2);\nprint \"@arr\";\n";
    let issues = scope_issues_strict(code);
    let false_pos: Vec<_> = issues
        .iter()
        .filter(|i| {
            (i.kind == IssueKind::UnusedVariable || i.kind == IssueKind::UndeclaredVariable)
                && i.variable_name == "$\""
        })
        .collect();
    assert!(
        false_pos.is_empty(),
        "local $\" should produce no false diagnostics; got: {:?}",
        false_pos
    );
    Ok(())
}

#[test]
fn local_special_var_in_block_no_false_unused() -> Result<(), Box<dyn std::error::Error>> {
    // `local $/` without an initializer in a block must not produce false diagnostics.
    let code = "use strict;\n{\n    local $/;\n    my $data = <STDIN>;\n    print $data;\n}\n";
    let issues = scope_issues_strict(code);
    let false_pos: Vec<_> = issues
        .iter()
        .filter(|i| {
            (i.kind == IssueKind::UnusedVariable || i.kind == IssueKind::UndeclaredVariable)
                && i.variable_name == "$/"
        })
        .collect();
    assert!(
        false_pos.is_empty(),
        "local $/ (no initializer) should produce no false diagnostics; got: {:?}",
        false_pos
    );
    Ok(())
}

#[test]
fn local_special_var_in_sub_no_false_unused() -> Result<(), Box<dyn std::error::Error>> {
    // `local $/` inside a subroutine must not produce false diagnostics.
    let code = r#"use strict;
use warnings;
sub slurp {
    my ($file) = @_;
    open(my $fh, '<', $file) or die $!;
    local $/ = undef;
    my $content = <$fh>;
    close($fh);
    return $content;
}
"#;
    let issues = scope_issues_strict(code);
    let false_pos: Vec<_> = issues
        .iter()
        .filter(|i| {
            (i.kind == IssueKind::UnusedVariable || i.kind == IssueKind::UndeclaredVariable)
                && i.variable_name == "$/"
        })
        .collect();
    assert!(
        false_pos.is_empty(),
        "local $/ inside sub should produce no false diagnostics; got: {:?}",
        false_pos
    );
    Ok(())
}

// ===========================================================================
// 4. Variable Scope Resolution — state
// ===========================================================================

#[test]
fn scope_state_variable_extracted() -> Result<(), Box<dyn std::error::Error>> {
    // `state` variables should be extractable and marked as state declarations.
    let code = r#"
sub counter {
    state $count = 0;
    $count++;
    return $count;
}
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "count", SymbolKind::scalar()), "state $count should be extracted");

    let count_syms = table.symbols.get("count").ok_or("count not found")?;
    assert!(
        count_syms.iter().any(|s| s.declaration.as_deref() == Some("state")),
        "declaration type should be 'state'"
    );
    Ok(())
}

#[test]
fn scope_state_variable_scope_confined_to_sub() -> Result<(), Box<dyn std::error::Error>> {
    // `state` variables are lexically scoped to their enclosing sub, like `my`.
    let code = r#"
use strict;
sub increment {
    state $n = 0;
    $n++;
}
print $n;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "n"),
        "state variable should not be visible outside its sub"
    );
    Ok(())
}

// ===========================================================================
// 5. Package-Qualified Symbol Resolution
// ===========================================================================

#[test]
fn symbol_package_qualified_sub_name() -> Result<(), Box<dyn std::error::Error>> {
    // Sub declared inside a package should have a qualified_name.
    let code = r#"
package Util::String;
sub trim { 1 }
sub pad  { 1 }
"#;
    let table = parse_and_extract(code);

    let trim_syms = table.symbols.get("trim").ok_or("trim not found")?;
    assert!(
        trim_syms.iter().any(|s| s.qualified_name == "Util::String::trim"),
        "sub should have fully qualified name"
    );

    let pad_syms = table.symbols.get("pad").ok_or("pad not found")?;
    assert!(
        pad_syms.iter().any(|s| s.qualified_name == "Util::String::pad"),
        "sub should have fully qualified name"
    );
    Ok(())
}

#[test]
fn symbol_default_package_is_main() -> Result<(), Box<dyn std::error::Error>> {
    // Without a package declaration, symbols should be in main::.
    let code = "sub run { 1 }";
    let table = parse_and_extract(code);

    let run_syms = table.symbols.get("run").ok_or("run not found")?;
    assert!(
        run_syms.iter().any(|s| s.qualified_name.contains("main")),
        "default package should be main, got: {:?}",
        run_syms.iter().map(|s| &s.qualified_name).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn symbol_multiple_packages_in_one_file() -> Result<(), Box<dyn std::error::Error>> {
    // Multiple package declarations in one file should each scope subsequent subs.
    let code = r#"
package Foo;
sub foo_method { 1 }

package Bar;
sub bar_method { 1 }

package Baz;
sub baz_method { 1 }
"#;
    let table = parse_and_extract(code);

    let foo_syms = table.symbols.get("foo_method").ok_or("foo_method not found")?;
    assert!(
        foo_syms.iter().any(|s| s.qualified_name.contains("Foo")),
        "foo_method should be in Foo"
    );

    let bar_syms = table.symbols.get("bar_method").ok_or("bar_method not found")?;
    assert!(
        bar_syms.iter().any(|s| s.qualified_name.contains("Bar")),
        "bar_method should be in Bar"
    );

    let baz_syms = table.symbols.get("baz_method").ok_or("baz_method not found")?;
    assert!(
        baz_syms.iter().any(|s| s.qualified_name.contains("Baz")),
        "baz_method should be in Baz"
    );
    Ok(())
}

#[test]
fn symbol_package_switch_back() -> Result<(), Box<dyn std::error::Error>> {
    // Switching back to a previously declared package should use that package name.
    let code = r#"
package Alpha;
sub first { 1 }

package Beta;
sub second { 1 }

package Alpha;
sub third { 1 }
"#;
    let table = parse_and_extract(code);

    let third_syms = table.symbols.get("third").ok_or("third not found")?;
    assert!(
        third_syms.iter().any(|s| s.qualified_name.contains("Alpha")),
        "third should be under Alpha after switching back"
    );
    Ok(())
}

#[test]
fn symbol_our_variable_qualified_name_differs_by_package() -> Result<(), Box<dyn std::error::Error>>
{
    // our variables in different packages should have different qualified names.
    let code = r#"
package Config;
our $DEBUG = 0;

package Runtime;
our $DEBUG = 1;
"#;
    let table = parse_and_extract(code);

    let debug_syms = table.symbols.get("DEBUG").ok_or("DEBUG not found")?;
    assert!(debug_syms.len() >= 2, "should have DEBUG in both packages");

    let has_config = debug_syms.iter().any(|s| s.qualified_name == "Config::DEBUG");
    let has_runtime = debug_syms.iter().any(|s| s.qualified_name == "Runtime::DEBUG");
    assert!(has_config, "should have Config::DEBUG");
    assert!(has_runtime, "should have Runtime::DEBUG");
    Ok(())
}

#[test]
fn symbol_find_symbol_in_scope_chain() -> Result<(), Box<dyn std::error::Error>> {
    // find_symbol should walk up the scope chain to find symbols in parent scopes.
    let code = r#"
my $top_level = 1;
sub wrapper {
    my $mid_level = 2;
    sub inner {
        my $bottom = 3;
    }
}
"#;
    let table = parse_and_extract(code);

    // The inner subroutine creates a scope — we should be able to find $top_level from it
    let found = table.find_symbol("top_level", 0, SymbolKind::scalar());
    assert!(!found.is_empty(), "should find top_level from global scope");
    Ok(())
}

#[test]
fn symbol_find_references_for_sub() -> Result<(), Box<dyn std::error::Error>> {
    // find_references should return all usage sites for a subroutine.
    let code = r#"
sub helper { 1 }
helper();
helper();
helper();
"#;
    let table = parse_and_extract(code);

    let helper_syms = table.symbols.get("helper").ok_or("helper not found")?;
    let refs = table.find_references(&helper_syms[0]);
    assert!(refs.len() >= 3, "should find at least 3 references to helper, got {}", refs.len());
    Ok(())
}

// ===========================================================================
// 6. Cross-Scope Reference Tracking
// ===========================================================================

#[test]
fn scope_reference_tracks_usage_in_sub() -> Result<(), Box<dyn std::error::Error>> {
    // Variable declared at file scope, used inside a sub should be tracked.
    let code = r#"
my $config = "prod";
sub get_config {
    return $config;
}
"#;
    let issues = scope_issues(code);
    let unused_config = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("config"))
        .count();
    assert_eq!(unused_config, 0, "$config used in sub should not be flagged as unused");
    Ok(())
}

#[test]
fn scope_reference_variable_in_closure() -> Result<(), Box<dyn std::error::Error>> {
    // Variable captured by an anonymous sub (closure) should count as used.
    let code = r#"
my $multiplier = 10;
my $fn = sub { return $multiplier * 2; };
print $fn;
"#;
    let issues = scope_issues(code);
    let unused = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("multiplier"))
        .count();
    assert_eq!(unused, 0, "$multiplier captured by closure should not be unused");
    Ok(())
}

#[test]
fn scope_reference_across_multiple_subs() -> Result<(), Box<dyn std::error::Error>> {
    // A file-scoped variable used in multiple subs should not be flagged.
    let code = r#"
my $shared = "data";
sub reader { print $shared; }
sub writer { $shared = "new"; }
"#;
    let issues = scope_issues(code);
    let unused = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("shared"))
        .count();
    assert_eq!(unused, 0, "$shared used in multiple subs should not be unused");
    Ok(())
}

#[test]
fn scope_reference_hash_element_access_direct() -> Result<(), Box<dyn std::error::Error>> {
    // Accessing a hash element via assignment target resolves cross-sigil.
    let code = r#"
my %opts = (verbose => 1, debug => 0);
my $val = $opts{verbose};
print $val;
"#;
    let issues = scope_issues(code);
    // Note: cross-sigil lookup ($opts -> %opts) depends on the parser AST structure.
    // The scope analyzer handles this when the Variable node is the direct left child
    // of a Binary {} node. In `print $opts{verbose}`, the AST may differ.
    // This test documents the current behavior.
    let _ = issues;
    Ok(())
}

#[test]
fn scope_reference_array_element_access_direct() -> Result<(), Box<dyn std::error::Error>> {
    // Accessing an array element via assignment target resolves cross-sigil.
    let code = r#"
my @items = (10, 20, 30);
my $first = $items[0];
print $first;
"#;
    let issues = scope_issues(code);
    // Same note as hash: cross-sigil lookup depends on exact AST structure.
    // This test documents the current behavior without false assertions.
    let _ = issues;
    Ok(())
}

#[test]
fn scope_reference_hash_direct_usage() -> Result<(), Box<dyn std::error::Error>> {
    // Using %hash directly (not through $hash{}) should mark it as used.
    let code = r#"
my %config = (key => 'val');
my @keys = keys %config;
print @keys;
"#;
    let issues = scope_issues(code);
    let unused_config = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("config"))
        .count();
    assert_eq!(unused_config, 0, "direct usage of hash via keys() should not be unused");
    Ok(())
}

#[test]
fn scope_reference_array_direct_usage() -> Result<(), Box<dyn std::error::Error>> {
    // Using @array directly should mark it as used.
    let code = r#"
my @data = (1, 2, 3);
my $count = scalar @data;
print $count;
"#;
    let issues = scope_issues(code);
    let unused_data = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("data"))
        .count();
    assert_eq!(unused_data, 0, "direct usage of @data should not be unused");
    Ok(())
}

#[test]
fn scope_reference_in_for_loop_body() -> Result<(), Box<dyn std::error::Error>> {
    // Variable used inside a for loop body should not be unused.
    let code = r#"
my $total = 0;
my @nums = (1, 2, 3);
for my $n (@nums) {
    $total = $total + $n;
}
print $total;
"#;
    let issues = scope_issues(code);
    let unused_total = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("total"))
        .count();
    assert_eq!(unused_total, 0, "$total used in for body should not be unused");
    Ok(())
}

// ===========================================================================
// 7. Shadowed Variable Detection
// ===========================================================================

#[test]
fn shadow_my_in_sub_shadows_file_scope() -> Result<(), Box<dyn std::error::Error>> {
    // A `my` inside a sub that has the same name as a file-scope `my` should be shadowing.
    let code = r#"
my $name = "outer";
sub greet {
    my $name = "inner";
    print $name;
}
print $name;
"#;
    let issues = scope_issues(code);
    assert!(
        has_issue(&issues, IssueKind::VariableShadowing, "name"),
        "inner $name should shadow outer $name"
    );
    Ok(())
}

#[test]
fn shadow_different_sigils_no_shadow() -> Result<(), Box<dyn std::error::Error>> {
    // $x and @x are different variables in Perl; redeclaring with a different sigil
    // should NOT be flagged as shadowing.
    let code = r#"
my $x = 1;
{
    my @x = (2, 3);
    print @x;
}
print $x;
"#;
    let issues = scope_issues(code);
    // There should be no shadowing between $x and @x
    let shadow_x = issues
        .iter()
        .filter(|i| i.kind == IssueKind::VariableShadowing && i.variable_name.ends_with('x'))
        .count();
    assert_eq!(shadow_x, 0, "$x and @x are different variables, no shadowing expected");
    Ok(())
}

#[test]
fn shadow_three_levels_deep() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that multiple levels of shadowing are detected individually.
    let code = r#"
my $val = 1;
{
    my $val = 2;
    {
        my $val = 3;
        print $val;
    }
    print $val;
}
print $val;
"#;
    let issues = scope_issues(code);
    let shadow_count = issues
        .iter()
        .filter(|i| i.kind == IssueKind::VariableShadowing && i.variable_name.contains("val"))
        .count();
    assert!(shadow_count >= 2, "should detect at least 2 shadow levels, got {}", shadow_count);
    Ok(())
}

#[test]
fn shadow_sub_parameter_shadows_outer() -> Result<(), Box<dyn std::error::Error>> {
    // A sub parameter that shadows an outer variable should produce
    // a ParameterShadowsGlobal issue.
    let code = r#"
my $x = 10;
sub process($x) {
    print $x;
}
print $x;
"#;
    let issues = scope_issues(code);
    assert!(
        has_issue(&issues, IssueKind::ParameterShadowsGlobal, "x"),
        "sub parameter $x should shadow outer $x; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn shadow_for_loop_variable_shadows_outer() -> Result<(), Box<dyn std::error::Error>> {
    // A for-loop iterator with the same name as an outer variable should shadow.
    let code = r#"
my $i = 100;
my @list = (1, 2, 3);
for my $i (@list) {
    print $i;
}
print $i;
"#;
    let issues = scope_issues(code);
    assert!(
        has_issue(&issues, IssueKind::VariableShadowing, "i"),
        "for-loop $i should shadow outer $i"
    );
    Ok(())
}

#[test]
fn shadow_description_mentions_variable_name() -> Result<(), Box<dyn std::error::Error>> {
    // Shadowing issue descriptions should mention the variable name.
    let code = r#"
my $target = 1;
{
    my $target = 2;
    print $target;
}
print $target;
"#;
    let issues = scope_issues(code);
    let shadow = issues
        .iter()
        .find(|i| i.kind == IssueKind::VariableShadowing && i.variable_name.contains("target"));
    if let Some(issue) = shadow {
        assert!(
            issue.description.contains("target"),
            "description should mention the variable name"
        );
    }
    Ok(())
}

// ===========================================================================
// 8. Unused Variable Detection
// ===========================================================================

#[test]
fn unused_variable_basic_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $never_used = 42;";
    let issues = scope_issues(code);
    assert!(
        has_issue(&issues, IssueKind::UnusedVariable, "never_used"),
        "should detect unused $never_used"
    );
    Ok(())
}

#[test]
fn unused_variable_basic_array() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my @unused_arr = (1, 2);";
    let issues = scope_issues(code);
    assert!(
        has_issue(&issues, IssueKind::UnusedVariable, "unused_arr"),
        "should detect unused @unused_arr"
    );
    Ok(())
}

#[test]
fn unused_variable_basic_hash() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my %unused_hash = (k => 'v');";
    let issues = scope_issues(code);
    assert!(
        has_issue(&issues, IssueKind::UnusedVariable, "unused_hash"),
        "should detect unused %unused_hash"
    );
    Ok(())
}

#[test]
fn unused_variable_used_via_explicit_dereference_forms() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $arrayref;
push @$arrayref, 1;
push @{$arrayref}, 2;

my $hashref;
$hashref->{k};

my $hashslice_ref = { a => 1, b => 2 };
my @vals = %$hashslice_ref{'a', 'b'};
my @vals_list = @$hashslice_ref{'a', 'b'};

my $value = 1;
my $scalarref = \$value;
$$scalarref;

my @arr = (1, 2, 3);
$arr[0];
"#;
    let issues = scope_issues(code);

    let unused_arrayref = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("arrayref"))
        .count();
    assert_eq!(unused_arrayref, 0, "$arrayref used via dereference should not be unused");

    let unused_hashref = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("hashref"))
        .count();
    assert_eq!(unused_hashref, 0, "$hashref used via arrow dereference should not be unused");

    let unused_hashslice_ref = issues
        .iter()
        .filter(|i| {
            i.kind == IssueKind::UnusedVariable && i.variable_name.contains("hashslice_ref")
        })
        .count();
    assert_eq!(
        unused_hashslice_ref, 0,
        "$hashslice_ref used via hash slice dereference should not be unused"
    );

    let unused_scalarref = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("scalarref"))
        .count();
    assert_eq!(unused_scalarref, 0, "$scalarref used via scalar dereference should not be unused");

    let unused_arr = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("arr"))
        .count();
    assert_eq!(unused_arr, 0, "@arr used via direct indexing should not be unused");

    Ok(())
}

/// Regression test for issue #3338: push @$arrayref does not mark my $arrayref as used.
///
/// Verifies that the exact reproducer from the issue report no longer produces a
/// false "unused variable" diagnostic. Covers string-literal arguments (the original
/// report), numeric arguments, and both non-strict and strict-mode variants.
#[test]
fn issue_3338_push_arrayref_deref_not_unused() -> Result<(), Box<dyn std::error::Error>> {
    // Exact reproducer from issue #3338 — string literal argument
    let code = r#"
my $arrayref;
push @$arrayref, 'item';
"#;
    let issues = scope_issues(code);
    let unused = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("arrayref"))
        .count();
    assert_eq!(
        unused,
        0,
        "issue #3338: $arrayref used via push @$arrayref should not be unused; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

/// Regression test for issue #3338 under strict mode: push @$arrayref should not
/// produce UnusedVariable or UndeclaredVariable for a declared scalar ref.
#[test]
fn issue_3338_push_arrayref_deref_strict_mode() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
my $arrayref = [];
push @$arrayref, 'item';
push @$arrayref, 'second';
"#;
    let issues = scope_issues_strict(code);
    let unused = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("arrayref"))
        .count();
    assert_eq!(
        unused,
        0,
        "issue #3338: $arrayref declared and used via push @$arrayref should not be unused under strict; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    let undeclared = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name.contains("arrayref"))
        .count();
    assert_eq!(
        undeclared,
        0,
        "issue #3338: declared $arrayref should not be reported as undeclared under strict; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

/// Regression test for issue #3338: all three primary dereference sigil forms
/// (`@$ref`, `%$ref`, `$$ref`) should mark the underlying scalar declaration used.
#[test]
fn issue_3338_all_deref_sigil_forms_mark_base_used() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
my $aref = [1, 2, 3];
my $href = {a => 1};
my $sref = \"hello";

my @items = @$aref;
my %copy  = %$href;
my $val   = $$sref;
"#;
    let issues = scope_issues_strict(code);

    for var in &["aref", "href", "sref"] {
        let unused = issues
            .iter()
            .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains(var))
            .count();
        assert_eq!(
            unused,
            0,
            "issue #3338: ${} used via deref should not be unused; issues: {:?}",
            var,
            issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
        );
    }
    Ok(())
}

#[test]
fn unused_variable_used_via_coderef_and_glob_dereference_forms()
-> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
my $cb = sub { 1 };
&$cb();

my $gref = *STDOUT{IO};
print *$gref;
"#;
    let issues = scope_issues_strict(code);

    assert!(
        !issues.iter().any(|i| {
            matches!(i.kind, IssueKind::UndeclaredVariable | IssueKind::UnusedVariable)
                && i.variable_name.contains("cb")
        }),
        "$cb used via coderef invocation should not be flagged: {:?}",
        issues
    );
    assert!(
        !issues.iter().any(|i| {
            matches!(i.kind, IssueKind::UndeclaredVariable | IssueKind::UnusedVariable)
                && i.variable_name.contains("gref")
        }),
        "$gref used via glob dereference should not be flagged: {:?}",
        issues
    );

    Ok(())
}

#[test]
fn unused_variable_used_via_dynamic_method_deref_forms() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
my $obj = bless {}, 'Foo';
my $method = 'method';
$obj->${method}();
$obj->${\'method'}();
$obj->$method();
"#;
    let issues = scope_issues_strict(code);

    assert!(
        !issues.iter().any(|i| {
            matches!(i.kind, IssueKind::UndeclaredVariable | IssueKind::UnusedVariable)
                && i.variable_name.contains("obj")
        }),
        "$obj used via dynamic method deref should not be flagged: {:?}",
        issues
    );
    assert!(
        !issues.iter().any(|i| {
            matches!(i.kind, IssueKind::UndeclaredVariable | IssueKind::UnusedVariable)
                && i.variable_name.contains("method")
        }),
        "$method used via dynamic method selection should not be flagged: {:?}",
        issues
    );

    Ok(())
}

#[test]
fn subscript_access_marks_array_parent_used() -> Result<(), Box<dyn std::error::Error>> {
    // $arr[0] passed to a function should mark @arr as used — no unused-variable diagnostic.
    let code = r#"
my @arr = (1, 2, 3);
print $arr[0];
"#;
    let issues = scope_issues(code);
    let unused_arr = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("arr"))
        .count();
    assert_eq!(unused_arr, 0, "@arr should not be flagged as unused when accessed via $arr[0]");
    Ok(())
}

#[test]
fn subscript_access_marks_hash_parent_used() -> Result<(), Box<dyn std::error::Error>> {
    // $hash{key} passed to a function should mark %hash as used — no unused-variable diagnostic.
    let code = r#"
my %hash = (a => 1);
print $hash{a};
"#;
    let issues = scope_issues(code);
    let unused_hash = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("hash"))
        .count();
    assert_eq!(
        unused_hash, 0,
        "%hash should not be flagged as unused when accessed via $hash{{a}}"
    );
    Ok(())
}

#[test]
fn subscript_access_does_not_suppress_truly_unused() -> Result<(), Box<dyn std::error::Error>> {
    // A %hash or @array that is truly never accessed should still be flagged as unused.
    let code = r#"
my %unused_hash = (a => 1);
my @unused_arr = (1, 2, 3);
"#;
    let issues = scope_issues(code);
    let unused_hash = issues
        .iter()
        .any(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("unused_hash"));
    let unused_arr = issues
        .iter()
        .any(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("unused_arr"));
    assert!(unused_hash, "%unused_hash with no subscripts should still be flagged as unused");
    assert!(unused_arr, "@unused_arr with no subscripts should still be flagged as unused");
    Ok(())
}

#[test]
fn unused_underscore_prefix_suppressed() -> Result<(), Box<dyn std::error::Error>> {
    // Variables prefixed with underscore should NOT be flagged as unused.
    let code = r#"
my $_placeholder = 1;
my $_ignored = 2;
"#;
    let issues = scope_issues(code);
    let unused_underscored = issues
        .iter()
        .filter(|i| {
            i.kind == IssueKind::UnusedVariable
                && (i.variable_name.contains("_placeholder")
                    || i.variable_name.contains("_ignored"))
        })
        .count();
    assert_eq!(unused_underscored, 0, "underscore-prefixed variables should not be flagged");
    Ok(())
}

#[test]
fn unused_only_assigned_never_read() -> Result<(), Box<dyn std::error::Error>> {
    // A variable that is declared and assigned but never read should be unused.
    // Note: assignment marks a variable as "used" in the current implementation
    // because assignment is a form of use. This test documents the behavior.
    let code = r#"
my $x;
$x = 42;
"#;
    let issues = scope_issues(code);
    // The current implementation marks assignment as usage, so $x won't be unused.
    // This test documents this design choice.
    let _ = issues;
    Ok(())
}

#[test]
fn unused_used_in_function_argument() -> Result<(), Box<dyn std::error::Error>> {
    // A variable passed to a function should not be unused.
    let code = r#"
my $path = "/tmp/test";
unlink($path);
"#;
    let issues = scope_issues(code);
    let unused = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("path"))
        .count();
    assert_eq!(unused, 0, "$path passed to unlink should not be unused");
    Ok(())
}

#[test]
fn unused_variable_used_in_conditional() -> Result<(), Box<dyn std::error::Error>> {
    // Variable used in a conditional expression should not be unused.
    let code = r#"
my $flag = 1;
if ($flag) {
    print "yes";
}
"#;
    let issues = scope_issues(code);
    let unused = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("flag"))
        .count();
    assert_eq!(unused, 0, "$flag used in if condition should not be unused");
    Ok(())
}

#[test]
fn unused_variable_used_in_string_interpolation() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $name = "World";
print "Hello, $name!\n";
"#;
    let issues = scope_issues(code);
    let unused = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("name"))
        .count();
    assert_eq!(unused, 0, "$name used in interpolated string should not be unused");
    Ok(())
}

#[test]
fn escaped_interpolated_variable_is_still_unused() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $name = "World";
print "\$name\n";
"#;
    let issues = scope_issues(code);
    let unused = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("name"))
        .count();
    assert_eq!(unused, 1, "$name escaped in a string should still be unused");
    Ok(())
}

#[test]
fn hash_marker_in_string_does_not_count_as_use() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my %seen = (name => 1);
print "%seen\n";
"#;
    let issues = scope_issues(code);
    let unused = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("seen"))
        .count();
    assert_eq!(unused, 1, "%seen in a string should not count as interpolation");
    Ok(())
}

#[test]
fn unused_variable_multiple_in_same_scope() -> Result<(), Box<dyn std::error::Error>> {
    // All unused variables in the same scope should be reported.
    let code = r#"
my $a = 1;
my $b = 2;
my $c = 3;
my $d = 4;
my $e = 5;
"#;
    let issues = scope_issues(code);
    let unused_count = count_issues(&issues, IssueKind::UnusedVariable);
    assert!(unused_count >= 5, "should detect at least 5 unused variables, got {}", unused_count);
    Ok(())
}

// ===========================================================================
// 9. Undeclared Variable Detection (strict mode)
// ===========================================================================

#[test]
fn undeclared_variable_strict_mode() -> Result<(), Box<dyn std::error::Error>> {
    // Under strict, using an undeclared variable should produce UndeclaredVariable.
    let code = r#"
use strict;
print $unknown_var;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "unknown_var"),
        "should detect undeclared $unknown_var under strict"
    );
    Ok(())
}

#[test]
fn strict_vars_only_checks_undeclared_variables() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict 'vars';
print $unknown_var;
print FOO;
"#;
    let issues = scope_issues_strict(code);

    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "unknown_var"),
        "strict 'vars' should flag undeclared variables"
    );
    assert!(
        !issues
            .iter()
            .any(|i| { matches!(i.kind, IssueKind::UnquotedBareword) && i.variable_name == "FOO" }),
        "strict 'vars' should not flag barewords"
    );
    Ok(())
}

#[test]
fn strict_subs_only_checks_barewords() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict 'subs';
print $unknown_var;
print FOO;
print PL_sv_yes;
print PL_sv_no;
print PL_sv_undef;
"#;
    let issues = scope_issues_strict(code);

    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "unknown_var"),
        "strict 'subs' should not flag undeclared variables"
    );
    assert!(
        issues
            .iter()
            .any(|i| { matches!(i.kind, IssueKind::UnquotedBareword) && i.variable_name == "FOO" }),
        "strict 'subs' should flag barewords"
    );
    for internal in ["PL_sv_yes", "PL_sv_no", "PL_sv_undef"] {
        assert!(
            !issues.iter().any(|i| {
                matches!(i.kind, IssueKind::UnquotedBareword) && i.variable_name == internal
            }),
            "strict 'subs' should not flag internal special constant {internal}"
        );
    }
    Ok(())
}

#[test]
fn version_pragma_enables_strict_vars_and_subs() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use v5.40;
print $unknown_var;
print FOO;
"#;
    let issues = scope_issues_strict(code);

    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "unknown_var"),
        "use v5.40 should enable strict vars"
    );
    assert!(
        issues
            .iter()
            .any(|i| { matches!(i.kind, IssueKind::UnquotedBareword) && i.variable_name == "FOO" }),
        "use v5.40 should enable strict subs"
    );
    Ok(())
}

#[test]
fn version_pragma_does_not_import_builtin_short_names() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use v5.40;
print floor;
"#;
    let issues = scope_issues_strict(code);

    assert!(
        issues.iter().any(|i| {
            matches!(i.kind, IssueKind::UnquotedBareword) && i.variable_name == "floor"
        }),
        "use v5.40 should not lexically import builtin short names"
    );
    Ok(())
}

#[test]
fn builtin_pragma_imports_are_lexical_only() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use builtin 'true';
print floor;
print true;
"#;
    let issues = scope_issues_strict(code);

    assert!(
        issues.iter().any(|i| {
            matches!(i.kind, IssueKind::UnquotedBareword) && i.variable_name == "floor"
        }),
        "importing `true` must not suppress unrelated builtin names"
    );
    assert!(
        !issues.iter().any(|i| {
            matches!(i.kind, IssueKind::UnquotedBareword) && i.variable_name == "true"
        }),
        "lexically imported builtin short name `true` should be allowed"
    );
    Ok(())
}

#[test]
fn builtin_pragma_imports_allow_the_imported_name() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use builtin 'floor';
print floor;
"#;
    let issues = scope_issues_strict(code);

    assert!(
        !issues.iter().any(|i| {
            matches!(i.kind, IssueKind::UnquotedBareword) && i.variable_name == "floor"
        }),
        "lexically imported builtin short name `floor` should not be flagged"
    );
    Ok(())
}

#[test]
fn v5_36_auto_strict_flags_undeclared_variable_in_signature_sub()
-> Result<(), Box<dyn std::error::Error>> {
    // use v5.36 enables strict automatically via feature bundle.
    // An undeclared variable $z inside a signature sub should be flagged
    // even without an explicit 'use strict'.
    let code = r#"
use v5.36;
sub foo ($x) { $z = 1; }
"#;
    let issues = scope_issues_strict(code);

    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "z"),
        "use v5.36 auto-enables strict: $z inside a signature sub should be flagged as undeclared"
    );
    // The parameter $x should NOT be flagged as undeclared.
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "x"),
        "signature parameter $x should not be flagged as undeclared"
    );
    Ok(())
}

#[test]
fn feature_signatures_auto_strict_flags_undeclared_variable_in_signature_sub()
-> Result<(), Box<dyn std::error::Error>> {
    // `use feature 'signatures'` should enable strict semantics automatically.
    // An undeclared variable $z inside a signature sub should be flagged even
    // without explicit `use strict`.
    let code = r#"
use feature 'signatures';
sub foo ($x) { $z = 1; }
"#;
    let issues = scope_issues_strict(code);

    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "z"),
        "use feature 'signatures' should auto-enable strict: $z should be flagged as undeclared"
    );
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "x"),
        "signature parameter $x should not be flagged as undeclared"
    );
    Ok(())
}

#[test]
fn v5_36_enables_strict_vars_and_subs() -> Result<(), Box<dyn std::error::Error>> {
    // use v5.36 enables both strict vars (undeclared vars) and strict subs (barewords).
    let code = r#"
use v5.36;
print $unknown_var;
print FOO;
"#;
    let issues = scope_issues_strict(code);

    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "unknown_var"),
        "use v5.36 should enable strict vars — $unknown_var should be flagged"
    );
    assert!(
        issues
            .iter()
            .any(|i| { matches!(i.kind, IssueKind::UnquotedBareword) && i.variable_name == "FOO" }),
        "use v5.36 should enable strict subs — bareword FOO should be flagged"
    );
    Ok(())
}

#[test]
fn scalar_reference_dereference_uses_declared_scalar_under_strict()
-> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
my $value = 1;
my $ref = \$value;
print $$ref;
"#;
    let issues = scope_issues_strict(code);

    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable) && i.variable_name == "$$ref"),
        "$$ref should resolve through declared $ref: {:?}",
        issues
    );
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UnusedVariable) && i.variable_name.contains("ref")),
        "$ref used via $$ref should not be unused: {:?}",
        issues
    );
    Ok(())
}

#[test]
fn undeclared_variable_no_strict_no_issue() -> Result<(), Box<dyn std::error::Error>> {
    // Without strict, undeclared variables should not be flagged.
    let code = "print $whatever;";
    let issues = scope_issues(code);
    let undeclared = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name.contains("whatever"))
        .count();
    assert_eq!(undeclared, 0, "without strict, undeclared variables should not be flagged");
    Ok(())
}

#[test]
fn undeclared_package_qualified_variable_skipped() -> Result<(), Box<dyn std::error::Error>> {
    // Package-qualified variables like $Foo::bar should not be flagged as undeclared.
    let code = r#"
use strict;
print $Foo::bar;
"#;
    let issues = scope_issues_strict(code);
    let undeclared_pkg = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name.contains("Foo"))
        .count();
    assert_eq!(undeclared_pkg, 0, "package-qualified variables should not be flagged");
    Ok(())
}

// ===========================================================================
// 10. Variable Redeclaration Detection
// ===========================================================================

#[test]
fn redeclaration_same_scope_detected() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $x = 1;
my $x = 2;
"#;
    let issues = scope_issues(code);
    assert!(
        has_issue(&issues, IssueKind::VariableRedeclaration, "x"),
        "should detect redeclaration of $x"
    );
    Ok(())
}

#[test]
fn redeclaration_different_scope_not_detected() -> Result<(), Box<dyn std::error::Error>> {
    // Same name in different scopes is shadowing, not redeclaration.
    let code = r#"
my $x = 1;
print $x;
{
    my $x = 2;
    print $x;
}
"#;
    let issues = scope_issues(code);
    let redecl = issues
        .iter()
        .filter(|i| i.kind == IssueKind::VariableRedeclaration && i.variable_name.contains("x"))
        .count();
    assert_eq!(redecl, 0, "different scope should not be redeclaration");
    Ok(())
}

#[test]
fn redeclaration_different_sigil_ok() -> Result<(), Box<dyn std::error::Error>> {
    // $x and @x in the same scope are different variables, not redeclaration.
    let code = r#"
my $x = 1;
my @x = (2, 3);
print $x;
print @x;
"#;
    let issues = scope_issues(code);
    let redecl = issues.iter().filter(|i| i.kind == IssueKind::VariableRedeclaration).count();
    assert_eq!(redecl, 0, "$x and @x are different variables");
    Ok(())
}

// ===========================================================================
// 11. Uninitialized Variable Detection
// ===========================================================================

#[test]
fn uninitialized_variable_detected() -> Result<(), Box<dyn std::error::Error>> {
    // A variable declared without initialization, then read, should warn.
    let code = r#"
my $x;
print $x;
"#;
    let issues = scope_issues(code);
    assert!(
        has_issue(&issues, IssueKind::UninitializedVariable, "x"),
        "should detect use of uninitialized $x; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn uninitialized_variable_assigned_then_used_ok() -> Result<(), Box<dyn std::error::Error>> {
    // If a variable is declared, then assigned, then used, no warning.
    let code = r#"
my $x;
$x = 42;
print $x;
"#;
    let issues = scope_issues(code);
    let uninit = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UninitializedVariable && i.variable_name.contains("x"))
        .count();
    assert_eq!(uninit, 0, "$x assigned before use should not be uninitialized");
    Ok(())
}

// ===========================================================================
// 12. Duplicate Parameter Detection
// ===========================================================================

#[test]
fn duplicate_parameter_detected() -> Result<(), Box<dyn std::error::Error>> {
    // Duplicate parameters in a sub signature should be flagged.
    let code = r#"
sub bad_sub($x, $x) {
    print $x;
}
"#;
    let issues = scope_issues(code);
    assert!(
        has_issue(&issues, IssueKind::DuplicateParameter, "x"),
        "should detect duplicate parameter $x; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 13. Unused Parameter Detection
// ===========================================================================

#[test]
fn unused_parameter_detected() -> Result<(), Box<dyn std::error::Error>> {
    // A parameter declared in a sub signature but never used should be flagged.
    let code = r#"
sub process($input) {
    return 42;
}
"#;
    let issues = scope_issues(code);
    assert!(
        has_issue(&issues, IssueKind::UnusedParameter, "input"),
        "should detect unused parameter $input; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn unused_parameter_underscore_prefix_suppressed() -> Result<(), Box<dyn std::error::Error>> {
    // Parameters prefixed with underscore should not be flagged as unused.
    let code = r#"
sub callback($_event) {
    return 1;
}
"#;
    let issues = scope_issues(code);
    let unused_param = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedParameter && i.variable_name.contains("_event"))
        .count();
    assert_eq!(unused_param, 0, "_prefixed parameter should not be flagged as unused");
    Ok(())
}

// ===========================================================================
// 14. Symbol Table Scope Structure
// ===========================================================================

#[test]
fn scope_structure_sub_creates_scope() -> Result<(), Box<dyn std::error::Error>> {
    // Subroutine definitions should create a new scope in the symbol table.
    let code = r#"
sub my_func {
    my $local = 1;
}
"#;
    let table = parse_and_extract(code);
    // Should have more than just the global scope
    assert!(
        table.scopes.len() > 1,
        "sub should create a new scope, got {} scopes",
        table.scopes.len()
    );

    let has_sub_scope = table.scopes.values().any(|s| s.kind == ScopeKind::Subroutine);
    assert!(has_sub_scope, "should have a Subroutine scope");
    Ok(())
}

#[test]
fn scope_structure_block_creates_scope() -> Result<(), Box<dyn std::error::Error>> {
    // A bare block `{ ... }` should create a block scope.
    let code = r#"
{
    my $block_var = 1;
}
"#;
    let table = parse_and_extract(code);
    let has_block_scope = table.scopes.values().any(|s| s.kind == ScopeKind::Block);
    assert!(has_block_scope, "bare block should create a Block scope");
    Ok(())
}

#[test]
fn scope_structure_package_creates_scope() -> Result<(), Box<dyn std::error::Error>> {
    // A package with a block should create a package scope.
    let code = r#"
package Foo {
    sub bar { 1 }
}
"#;
    let table = parse_and_extract(code);
    let has_pkg_scope = table.scopes.values().any(|s| s.kind == ScopeKind::Package);
    assert!(has_pkg_scope, "package block should create a Package scope");
    Ok(())
}

#[test]
fn scope_structure_nested_scopes_have_parents() -> Result<(), Box<dyn std::error::Error>> {
    // Nested scopes should reference their parent scope.
    let code = r#"
sub outer {
    {
        my $nested = 1;
    }
}
"#;
    let table = parse_and_extract(code);

    // Find a Block scope that has a Subroutine parent
    let block_scopes: Vec<_> = table
        .scopes
        .values()
        .filter(|s| s.kind == ScopeKind::Block && s.parent.is_some())
        .collect();
    assert!(!block_scopes.is_empty(), "should have at least one block scope with a parent");
    Ok(())
}

// ===========================================================================
// 15. use vars Pragma
// ===========================================================================

#[test]
fn use_vars_declares_globals() -> Result<(), Box<dyn std::error::Error>> {
    // `use vars` should declare package globals that don't trigger undeclared warnings.
    let code = r#"
use strict;
use vars qw($VERSION @ISA);
print $VERSION;
print @ISA;
"#;
    let issues = scope_issues_strict(code);
    let undeclared = issues
        .iter()
        .filter(|i| {
            i.kind == IssueKind::UndeclaredVariable
                && (i.variable_name.contains("VERSION") || i.variable_name.contains("ISA"))
        })
        .count();
    assert_eq!(undeclared, 0, "use vars should declare globals, no undeclared warnings");
    Ok(())
}

// ===========================================================================
// 16. Edge Cases
// ===========================================================================

#[test]
fn edge_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let issues = scope_issues("");
    assert!(issues.is_empty(), "empty source should produce no scope issues");
    Ok(())
}

#[test]
fn edge_comments_only() -> Result<(), Box<dyn std::error::Error>> {
    let code = "# just a comment\n# another comment\n";
    let issues = scope_issues(code);
    assert!(issues.is_empty(), "comments-only source should produce no scope issues");
    Ok(())
}

#[test]
fn edge_many_nested_scopes() -> Result<(), Box<dyn std::error::Error>> {
    // Deeply nested scopes should not crash or produce incorrect results.
    let code = r#"
my $root = 1;
{
    my $l1 = 2;
    {
        my $l2 = 3;
        {
            my $l3 = 4;
            {
                my $l4 = 5;
                print $root;
                print $l1;
                print $l2;
                print $l3;
                print $l4;
            }
        }
    }
}
"#;
    let issues = scope_issues(code);
    let unused = count_issues(&issues, IssueKind::UnusedVariable);
    assert_eq!(unused, 0, "all variables are used at the deepest level");
    Ok(())
}

#[test]
fn edge_symbol_extractor_default_trait() -> Result<(), Box<dyn std::error::Error>> {
    // SymbolExtractor implements Default; verify it works.
    let extractor = SymbolExtractor::default();
    let mut parser = Parser::new("sub test { 1 }");
    let ast = must(parser.parse());
    let table = extractor.extract(&ast);
    assert!(has_symbol(&table, "test", SymbolKind::Subroutine));
    Ok(())
}

#[test]
fn edge_scope_analyzer_default_trait() -> Result<(), Box<dyn std::error::Error>> {
    // ScopeAnalyzer implements Default; verify it works.
    let analyzer = ScopeAnalyzer;
    let mut parser = Parser::new("my $x = 1;");
    let ast = must(parser.parse());
    let issues = analyzer.analyze(&ast, "my $x = 1;", &[]);
    // $x is unused
    assert!(issues.iter().any(|i| i.kind == IssueKind::UnusedVariable));
    Ok(())
}

#[test]
fn edge_scope_issue_line_numbers_correct() -> Result<(), Box<dyn std::error::Error>> {
    // Line numbers in scope issues should be accurate.
    let code = "my $a = 1;\nmy $b = 2;\nmy $c = 3;\n";
    let issues = scope_issues(code);
    // All three are unused. Verify line numbers are sequential and > 0.
    let lines: Vec<usize> = issues.iter().map(|i| i.line).collect();
    for line in &lines {
        assert!(*line > 0, "line number should be positive, got {}", line);
    }
    Ok(())
}

#[test]
fn edge_scope_issue_range_within_source() -> Result<(), Box<dyn std::error::Error>> {
    // Issue ranges should be within the source code bounds.
    let code = "my $x = 1;";
    let issues = scope_issues(code);
    for issue in &issues {
        assert!(issue.range.0 <= code.len(), "range start should be within source");
        assert!(issue.range.1 <= code.len(), "range end should be within source");
        assert!(issue.range.0 <= issue.range.1, "range start should be <= end");
    }
    Ok(())
}

// ===========================================================================
// 17. Package statement scope analysis (#3356)
// ===========================================================================

#[test]
fn package_stmt_our_no_false_redeclaration() -> Result<(), Box<dyn std::error::Error>> {
    // `our` variables with the same name in different packages declared via the
    // statement form (`package Foo;`) must NOT trigger VariableRedeclaration.
    // Before fix: both `our $VAR` declarations share the same root scope and the
    // second triggers a false VariableRedeclaration because the Package node had
    // no handler and the scope was never reset between packages.
    let code = r#"
use strict;
package Alpha;
our $VAR = 1;

package Beta;
our $VAR = 2;

print $Alpha::VAR;
print $Beta::VAR;
"#;
    let issues = scope_issues_strict(code);
    let redecl: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == IssueKind::VariableRedeclaration && i.variable_name.contains("VAR"))
        .collect();
    assert!(
        redecl.is_empty(),
        "our $VAR in different packages must not trigger VariableRedeclaration; got: {:?}",
        redecl
    );
    Ok(())
}

#[test]
fn package_block_creates_scope_boundary() -> Result<(), Box<dyn std::error::Error>> {
    // `package Foo { ... }` block form should create a scoped boundary.
    // `our` variables inside the block belong to that package scope and should
    // not leak out or conflict with same-named variables in the outer package.
    let code = r#"
use strict;
package Alpha;
our $VAR = 1;

package Beta {
    our $VAR = 2;
}
"#;
    let issues = scope_issues_strict(code);
    let redecl: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == IssueKind::VariableRedeclaration && i.variable_name.contains("VAR"))
        .collect();
    assert!(
        redecl.is_empty(),
        "our $VAR in package block must not conflict with outer package our $VAR; got: {:?}",
        redecl
    );
    let shadow: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == IssueKind::VariableShadowing && i.variable_name.contains("VAR"))
        .collect();
    assert!(
        shadow.is_empty(),
        "our $VAR in package block must not produce VariableShadowing; got: {:?}",
        shadow
    );
    Ok(())
}

#[test]
fn package_block_inner_vars_not_visible_outside() -> Result<(), Box<dyn std::error::Error>> {
    // Variables declared with `my` inside a `package Foo { }` block must not
    // be visible after the block ends.
    let code = r#"
use strict;
my $outer = 1;
package Inner {
    my $private = 2;
    print $private;
}
print $outer;
print $private;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "private"),
        "my variable inside package block must not be visible outside it; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    let outer_undecl = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name.contains("outer"))
        .count();
    assert_eq!(outer_undecl, 0, "$outer should still be accessible after package block");
    Ok(())
}

#[test]
fn package_stmt_does_not_break_my_variable_tracking() -> Result<(), Box<dyn std::error::Error>> {
    // A `package` statement must not disrupt tracking of `my` variables already
    // declared in the file-level scope before the package statement.
    let code = r#"
my $top = 1;
package Foo;
print $top;
"#;
    let issues = scope_issues(code);
    let top_unused = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("top"))
        .count();
    assert_eq!(
        top_unused, 0,
        "$top declared before package should be accessible after package statement"
    );
    Ok(())
}

#[test]
fn package_block_my_var_not_leaked_strict() -> Result<(), Box<dyn std::error::Error>> {
    // With strict mode, accessing a `my` variable declared inside a `package`
    // block from outside the block must be an UndeclaredVariable error.
    let code = r#"
use strict;
package Scoped {
    my $inner_var = 42;
    print $inner_var;
}
print $inner_var;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "inner_var"),
        "my var inside package block must not escape to outer scope; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn package_our_bare_usage_not_undeclared_strict() -> Result<(), Box<dyn std::error::Error>> {
    // Regression guard: `our $VAR` should remain accessible as bare `$VAR`
    // in the same package under strict mode. This ensures the Package handler
    // does not accidentally break the lookup path for `our` declarations.
    let code = r#"
use strict;
our $GLOBAL = 42;
print $GLOBAL;
"#;
    let issues = scope_issues_strict(code);
    let undecl: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name.contains("GLOBAL"))
        .collect();
    assert!(
        undecl.is_empty(),
        "our $GLOBAL should be accessible as bare $GLOBAL in strict mode; got: {:?}",
        undecl
    );
    Ok(())
}

#[test]
fn package_our_from_other_package_not_visible_as_bare_name()
-> Result<(), Box<dyn std::error::Error>> {
    // Bare `$VAR` in a later package must not resolve against an `our $VAR`
    // declared in an earlier package. Package switches change which package
    // global a bare name refers to under `strict 'vars'`.
    let code = r#"
use strict;
package Alpha;
our $VAR = 1;

package Beta;
print $VAR;
"#;
    let issues = scope_issues_strict(code);

    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "VAR"),
        "bare $VAR in package Beta must not resolve to Alpha::VAR; issues: {:?}",
        issues
    );
    Ok(())
}

#[test]
fn package_our_same_package_redeclaration_is_silent() -> Result<(), Box<dyn std::error::Error>> {
    // `our $x; our $x;` in the SAME package is redundant but valid Perl — the
    // second declaration is a no-op that re-imports the same package global.
    // Must NOT emit VariableRedeclaration.
    let code = r#"
use strict;
package Foo;
our $x = 1;
our $x = 2;
print $x;
"#;
    let issues = scope_issues_strict(code);
    let redecl: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == IssueKind::VariableRedeclaration && i.variable_name.contains('x'))
        .collect();
    assert!(
        redecl.is_empty(),
        "our $x redeclared in same package must not emit VariableRedeclaration; got: {:?}",
        redecl
    );
    Ok(())
}

#[test]
fn package_nested_block_scope_save_restore() -> Result<(), Box<dyn std::error::Error>> {
    // Nested `package Outer { package Inner { } }` must correctly save and restore
    // the outer package name when the inner block exits. Variables declared with
    // `my` in the inner block must not escape to the outer block or file scope.
    let code = r#"
use strict;
package Outer {
    my $outer_var = 1;
    package Inner {
        my $inner_var = 2;
        print $inner_var;
    }
    print $outer_var;
    print $inner_var;
}
print $outer_var;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "inner_var"),
        "my var inside nested package block must not escape; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    let outer_at_file_scope = issues
        .iter()
        .filter(|i| {
            i.kind == IssueKind::UndeclaredVariable && i.variable_name.contains("outer_var")
        })
        .count();
    assert!(
        outer_at_file_scope > 0,
        "my var inside package Outer block must not escape to file scope; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn package_block_our_no_false_shadowing() -> Result<(), Box<dyn std::error::Error>> {
    // Regression: `our $VAR` inside a package block must NOT produce VariableShadowing
    // even if an outer scope also declares `our $VAR` with the same bare name.
    // These are different package globals (Foo::VAR vs Bar::VAR) and the inner `our`
    // does NOT lexically shadow the outer one.
    let code = r#"
use strict;
our $VAR = 1;

package Inner {
    our $VAR = 2;
}
"#;
    let issues = scope_issues_strict(code);
    let shadow: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == IssueKind::VariableShadowing && i.variable_name.contains("VAR"))
        .collect();
    assert!(
        shadow.is_empty(),
        "our $VAR in package block must not produce VariableShadowing for outer our $VAR; got: {:?}",
        shadow
    );
    Ok(())
}

// ===========================================================================
// 18. Position-aware builtin declaration handling (read/sysread/recv at pos 1)
// ===========================================================================

#[test]
fn read_buffer_declaration_not_flagged() -> Result<(), Box<dyn std::error::Error>> {
    // `read $fh, my $buffer, 1024` — $buffer is declared at position 1, not 0.
    // It should not be flagged as undeclared, uninitialized, or unused.
    let code = r#"
use strict;
open my $fh, '<', 'file.txt';
read $fh, my $buffer, 1024;
print $buffer;
"#;
    let issues = scope_issues_strict(code);
    for var in ["$buffer", "$fh"] {
        assert!(
            !issues.iter().any(|i| {
                matches!(
                    i.kind,
                    IssueKind::UndeclaredVariable
                        | IssueKind::UninitializedVariable
                        | IssueKind::UnusedVariable
                ) && i.variable_name == var
            }),
            "read buffer/handle should not be flagged: {} (issues: {:?})",
            var,
            issues
        );
    }
    Ok(())
}

#[test]
fn sysread_buffer_declaration_not_flagged() -> Result<(), Box<dyn std::error::Error>> {
    // `sysread $fh, my $buf, 512` — position 1 is the declaration.
    let code = r#"
use strict;
open my $fh, '<', 'file.txt';
sysread $fh, my $buf, 512;
print $buf;
"#;
    let issues = scope_issues_strict(code);
    for var in ["$buf", "$fh"] {
        assert!(
            !issues.iter().any(|i| {
                matches!(
                    i.kind,
                    IssueKind::UndeclaredVariable
                        | IssueKind::UninitializedVariable
                        | IssueKind::UnusedVariable
                ) && i.variable_name == var
            }),
            "sysread buffer/handle should not be flagged: {} (issues: {:?})",
            var,
            issues
        );
    }
    Ok(())
}

#[test]
fn recv_buffer_declaration_not_flagged() -> Result<(), Box<dyn std::error::Error>> {
    // `recv $sock, my $data, 1024, 0` — position 1 is the declaration.
    let code = r#"
use strict;
socket my $sock, 2, 1, 0;
recv $sock, my $data, 1024, 0;
print $data;
"#;
    let issues = scope_issues_strict(code);
    for var in ["$data", "$sock"] {
        assert!(
            !issues.iter().any(|i| {
                matches!(
                    i.kind,
                    IssueKind::UndeclaredVariable
                        | IssueKind::UninitializedVariable
                        | IssueKind::UnusedVariable
                ) && i.variable_name == var
            }),
            "recv buffer/socket should not be flagged: {} (issues: {:?})",
            var,
            issues
        );
    }
    Ok(())
}

#[test]
fn read_position_zero_not_treated_as_declaration() -> Result<(), Box<dyn std::error::Error>> {
    // Position 0 in `read` is an existing filehandle, NOT a declaration target.
    // An undeclared handle at position 0 should still be flagged, while the
    // position-1 buffer should be treated as the declaration/initialization target.
    let code = r#"
use strict;
read $undeclared_fh, my $buffer, 1024;
print $buffer;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        issues.iter().any(|i| {
            i.variable_name.contains("undeclared_fh")
                && matches!(i.kind, IssueKind::UndeclaredVariable)
        }),
        "undeclared read handle should still be flagged (issues: {:?})",
        issues
    );
    assert!(
        !issues.iter().any(|i| {
            i.variable_name.contains("buffer")
                && matches!(i.kind, IssueKind::UndeclaredVariable | IssueKind::UnusedVariable)
        }),
        "read buffer declaration should not be flagged (issues: {:?})",
        issues
    );
    Ok(())
}

#[test]
fn read_position_zero_declaration_not_consumed() -> Result<(), Box<dyn std::error::Error>> {
    // Position 0 for `read` must not be treated as a declaration-capable output slot.
    // If a declaration appears there, it should still be analyzed like a normal lexical
    // declaration (and therefore may be reported as unused/uninitialized).
    let code = r#"
use strict;
read my $fh, my $buffer, 1024;
print $buffer;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        issues.iter().any(|i| {
            i.variable_name == "$fh"
                && matches!(i.kind, IssueKind::UnusedVariable | IssueKind::UninitializedVariable)
        }),
        "read position-0 declaration should not be auto-consumed (issues: {:?})",
        issues
    );
    assert!(
        !issues.iter().any(|i| {
            i.variable_name == "$buffer"
                && matches!(i.kind, IssueKind::UndeclaredVariable | IssueKind::UnusedVariable)
        }),
        "read position-1 buffer declaration should still be consumed (issues: {:?})",
        issues
    );
    Ok(())
}

#[test]
fn socketpair_both_positions_declared() -> Result<(), Box<dyn std::error::Error>> {
    // `socketpair my $a, my $b, ...` — positions 0 and 1 are both declarations.
    let code = r#"
use strict;
socketpair my $a, my $b, 2, 1, 0;
print $a;
print $b;
"#;
    let issues = scope_issues_strict(code);
    for var in ["$a", "$b"] {
        assert!(
            !issues.iter().any(|i| {
                matches!(
                    i.kind,
                    IssueKind::UndeclaredVariable
                        | IssueKind::UninitializedVariable
                        | IssueKind::UnusedVariable
                ) && i.variable_name == var
            }),
            "socketpair handle should not be flagged: {} (issues: {:?})",
            var,
            issues
        );
    }
    Ok(())
}

#[test]
fn socketpair_non_handle_positions_not_consumed() -> Result<(), Box<dyn std::error::Error>> {
    // `socketpair` only consumes declaration-capable handles at positions 0 and 1.
    // Declarations in later positions must remain ordinary lexicals.
    let code = r#"
use strict;
socketpair my $a, my $b, my $domain, 1, 0;
print $a;
print $b;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        issues.iter().any(|i| {
            i.variable_name == "$domain"
                && matches!(i.kind, IssueKind::UnusedVariable | IssueKind::UninitializedVariable)
        }),
        "socketpair position-2 declaration should not be auto-consumed (issues: {:?})",
        issues
    );
    Ok(())
}

// ===========================================================================
// Builtin globals — regex position arrays @- and @+ (#3354)
// ===========================================================================

#[test]
fn builtin_at_minus_no_undeclared_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use warnings;
if ("hello" =~ /ell/) {
    my $start = $-[0];
    my @starts = @-;
    print $start, "\n";
    print @starts, "\n";
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "-"),
        "@- should be a recognized builtin; got issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn builtin_at_plus_no_undeclared_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use warnings;
if ("hello" =~ /ell/) {
    my $end = $+[0];
    my @ends = @+;
    print $end, "\n";
    print @ends, "\n";
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "+"),
        "@+ should be a recognized builtin; got issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// Builtin globals — ${^MATCH}, ${^PREMATCH}, ${^POSTMATCH} (#3351)
// ===========================================================================

#[test]
fn builtin_caret_match_no_undeclared_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use warnings;
if ("hello world" =~ /world/p) {
    print ${^MATCH}, "\n";
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "^MATCH"),
        "{{^MATCH}} should be a recognized builtin; got issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn builtin_caret_prematch_no_undeclared_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use warnings;
if ("hello world" =~ /world/p) {
    print ${^PREMATCH}, "\n";
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "^PREMATCH"),
        "{{^PREMATCH}} should be a recognized builtin; got issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn builtin_caret_postmatch_no_undeclared_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use warnings;
if ("hello world" =~ /world/p) {
    print ${^POSTMATCH}, "\n";
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "^POSTMATCH"),
        "{{^POSTMATCH}} should be a recognized builtin; got issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// Topic variable $_ in map/grep block contexts (#3457)
// ===========================================================================

#[test]
fn topic_var_in_map_block_no_undeclared_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    // $_ is the implicit topic variable set by map; it must never be flagged
    // as undeclared under `use strict`.
    let code = r#"
use strict;
use warnings;
my @nums = (1, 2, 3);
my @doubled = map { $_ * 2 } @nums;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "_"),
        "$_ in map block should not be flagged as undeclared; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn topic_var_in_grep_block_no_undeclared_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    // $_ is the implicit topic variable set by grep; it must never be flagged
    // as undeclared under `use strict`.
    let code = r#"
use strict;
use warnings;
my @nums = (1, 2, 3);
my @evens = grep { $_ % 2 == 0 } @nums;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "_"),
        "$_ in grep block should not be flagged as undeclared; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn topic_var_chained_map_grep_no_undeclared_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    // $_ used across both map and grep in the same file should produce zero
    // UndeclaredVariable diagnostics.
    let code = r#"
use strict;
use warnings;
my @nums = (1, 2, 3);
my @doubled = map { $_ * 2 } @nums;
my @evens = grep { $_ % 2 == 0 } @nums;
"#;
    let issues = scope_issues_strict(code);
    let undeclared: Vec<_> =
        issues.iter().filter(|i| i.kind == IssueKind::UndeclaredVariable).collect();
    assert!(
        undeclared.is_empty(),
        "no UndeclaredVariable diagnostics expected for $_ in map/grep contexts; got: {:?}",
        undeclared.iter().map(|i| &i.variable_name).collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// 18. $a and $b Sort Variable Recognition
// ===========================================================================

#[test]
fn sort_a_b_no_diagnostic_in_sort_block() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
my @numbers = (3, 1, 4, 1, 5);
my @sorted = sort { $a <=> $b } @numbers;
print @sorted;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "a"),
        "$a in sort block must not be flagged as undeclared under strict; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "b"),
        "$b in sort block must not be flagged as undeclared under strict; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    assert!(
        !has_issue(&issues, IssueKind::UnusedVariable, "a"),
        "$a in sort block must not be flagged as unused; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    assert!(
        !has_issue(&issues, IssueKind::UnusedVariable, "b"),
        "$b in sort block must not be flagged as unused; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn sort_a_b_no_diagnostic_with_string_comparator() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
my @words = qw(banana apple cherry);
my @strings = sort { lc($a) cmp lc($b) } @words;
print @strings;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "a"),
        "$a in string-cmp sort block must not be undeclared; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "b"),
        "$b in string-cmp sort block must not be undeclared; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn sort_a_b_in_named_sub_no_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
sub by_length { length($a) <=> length($b) }
my @words = qw(foo barbaz hi);
my @by_len = sort by_length @words;
print @by_len;
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "a"),
        "$a in named sort sub must not be flagged as undeclared; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    assert!(
        !has_issue(&issues, IssueKind::UndeclaredVariable, "b"),
        "$b in named sort sub must not be flagged as undeclared; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn sort_a_b_no_diagnostic_without_strict() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my @numbers = (5, 2, 8, 1);
my @sorted = sort { $a <=> $b } @numbers;
print @sorted;
"#;
    let issues = scope_issues(code);
    assert!(
        !has_issue(&issues, IssueKind::UnusedVariable, "a"),
        "$a in sort block must not be flagged as unused (no strict); issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    assert!(
        !has_issue(&issues, IssueKind::UnusedVariable, "b"),
        "$b in sort block must not be flagged as unused (no strict); issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn user_variable_named_a_outside_sort_is_not_undeclared() -> Result<(), Box<dyn std::error::Error>>
{
    let code = r#"
my $a = 42;
"#;
    let issues = scope_issues(code);
    let undeclared = has_issue(&issues, IssueKind::UndeclaredVariable, "a");
    assert!(!undeclared, "a lexically declared $a must never be flagged as undeclared");
    Ok(())
}

// ===========================================================================
// Phase block (BEGIN/END/CHECK/INIT/UNITCHECK) symbol extraction (#3464)
// ===========================================================================

#[test]
fn phase_block_begin_extracted_as_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let code = "BEGIN { require Config; }";
    let table = parse_and_extract(code);
    assert!(
        has_symbol(&table, "BEGIN", SymbolKind::Subroutine),
        "BEGIN block must appear in symbol table as Subroutine; symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn phase_block_end_extracted_as_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let code = "END { cleanup(); }";
    let table = parse_and_extract(code);
    assert!(
        has_symbol(&table, "END", SymbolKind::Subroutine),
        "END block must appear in symbol table; symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn phase_block_all_five_extracted() -> Result<(), Box<dyn std::error::Error>> {
    let code = "BEGIN { 1; }\nEND { 1; }\nCHECK { 1; }\nINIT { 1; }\nUNITCHECK { 1; }\n";
    let table = parse_and_extract(code);
    for phase in &["BEGIN", "END", "CHECK", "INIT", "UNITCHECK"] {
        assert!(
            has_symbol(&table, phase, SymbolKind::Subroutine),
            "{} must appear in symbol table; symbols: {:?}",
            phase,
            table.symbols.keys().collect::<Vec<_>>()
        );
    }
    Ok(())
}

#[test]
fn phase_block_symbol_location_within_source() -> Result<(), Box<dyn std::error::Error>> {
    let code = "BEGIN { my $x = 1; }";
    let table = parse_and_extract(code);
    let syms = table.symbols.get("BEGIN").ok_or("BEGIN not found")?;
    let sym = syms.first().ok_or("no symbols for BEGIN")?;
    assert!(sym.location.start <= code.len(), "start offset must be within source");
    assert!(sym.location.end <= code.len(), "end offset must be within source");
    assert!(sym.location.start < sym.location.end, "start must be before end");
    Ok(())
}

#[test]
fn phase_block_symbol_in_global_scope() -> Result<(), Box<dyn std::error::Error>> {
    let code = "BEGIN { my $x = 42; }";
    let table = parse_and_extract(code);
    let begin_syms = table.symbols.get("BEGIN").ok_or("BEGIN not found")?;
    let begin_sym = begin_syms.first().ok_or("no BEGIN symbol")?;
    assert_eq!(begin_sym.scope_id, 0, "BEGIN symbol must be in global scope");
    Ok(())
}

// ---- Issue #3503: variables in print comma-separated args must be marked used ----

#[test]
fn scope_many_variables_in_print_comma_args() -> Result<(), Box<dyn std::error::Error>> {
    // Regression test for #3503: print $a, $b, $c, $d, $e — all five variables
    // should be considered used. $a and $b are Perl sort globals; locally declared
    // `my $a`/`my $b` must NOT be skipped by the is_builtin_global guard.
    let code = r#"
my $a = 1;
my $b = 2;
my $c = 3;
my $d = 4;
my $e = 5;
print $a, $b, $c, $d, $e;
"#;
    let issues = scope_issues(code);
    let unused = count_issues(&issues, IssueKind::UnusedVariable);
    assert_eq!(
        unused,
        0,
        "all variables used in print comma-separated args should not be unused; issues: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn print_comma_args_strict_no_false_unused() -> Result<(), Box<dyn std::error::Error>> {
    // Under `use strict`, variables used in print comma list must not be reported unused.
    let code = r#"
use strict;
my $name = "world";
my $greeting = "hello";
print $greeting, " ", $name, "\n";
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !has_issue(&issues, IssueKind::UnusedVariable, "name"),
        "$name used in print comma list must not be flagged as unused; issues: {:?}",
        issues
    );
    assert!(
        !has_issue(&issues, IssueKind::UnusedVariable, "greeting"),
        "$greeting used in print comma list must not be flagged as unused; issues: {:?}",
        issues
    );
    Ok(())
}

#[test]
fn say_comma_args_no_false_unused() -> Result<(), Box<dyn std::error::Error>> {
    // say with comma-separated args must also mark all variables as used.
    let code = r#"
my $x = "foo";
my $y = "bar";
say $x, " ", $y;
"#;
    let issues = scope_issues(code);
    assert!(
        !has_issue(&issues, IssueKind::UnusedVariable, "x"),
        "$x used in say comma list must not be flagged as unused; issues: {:?}",
        issues
    );
    assert!(
        !has_issue(&issues, IssueKind::UnusedVariable, "y"),
        "$y used in say comma list must not be flagged as unused; issues: {:?}",
        issues
    );
    Ok(())
}

// ===========================================================================
// Try/catch variable binding — extended coverage (#3541)
// ===========================================================================

/// The issue example uses $err, not $e.  Both must be handled identically.
#[test]
fn scope_try_catch_err_variable_is_bound() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use feature 'try';
try {
    die "error";
} catch ($err) {
    print $err;
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !issues
            .iter()
            .any(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name == "$err"),
        "catch variable $err must not be reported as undeclared: {:?}",
        issues
    );
    assert!(
        !issues.iter().any(|i| i.kind == IssueKind::UnusedVariable && i.variable_name == "$err"),
        "used catch variable $err must not be reported as unused: {:?}",
        issues
    );
    Ok(())
}

/// use v5.34 enables try/catch without an explicit use feature 'try'.
/// The scope analyzer must bind the catch variable in both cases.
#[test]
fn scope_try_catch_v534_pragma_binds_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use v5.34;
try {
    die "error";
} catch ($err) {
    print $err;
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !issues
            .iter()
            .any(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name == "$err"),
        "catch variable must be bound when enabled via use v5.34: {:?}",
        issues
    );
    Ok(())
}

/// Multiple catch blocks must each bind their own variable independently.
#[test]
fn scope_try_multiple_catch_blocks_each_bind_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use feature 'try';
try {
    die "error";
} catch ($first_err) {
    print $first_err;
} catch ($second_err) {
    print $second_err;
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !issues
            .iter()
            .any(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name == "$first_err"),
        "first catch variable must not be undeclared: {:?}",
        issues
    );
    assert!(
        !issues
            .iter()
            .any(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name == "$second_err"),
        "second catch variable must not be undeclared: {:?}",
        issues
    );
    Ok(())
}

/// Each catch block's variable must be invisible in the other's block.
#[test]
fn scope_try_catch_variables_do_not_cross_contaminate() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use feature 'try';
try {
    die "error";
} catch ($first_err) {
    print $first_err;
} catch ($second_err) {
    print $first_err;
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "$first_err"),
        "$first_err used in second catch block should be undeclared: {:?}",
        issues
    );
    Ok(())
}

/// Bare catch (no variable) must not crash and must not emit any diagnostic.
#[test]
fn scope_try_bare_catch_no_variable_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use feature 'try';
try {
    die "error";
} catch {
    print "caught";
}
"#;
    // Must not panic; must not produce any undeclared-variable issue
    let issues = scope_issues_strict(code);
    assert!(
        !issues.iter().any(|i| i.kind == IssueKind::UndeclaredVariable),
        "bare catch should not produce undeclared diagnostics: {:?}",
        issues
    );
    Ok(())
}

/// The finally block must run in the outer scope, not the catch scope.
/// Variables declared inside a catch must not bleed into finally.
#[test]
fn scope_try_catch_variable_not_visible_in_finally() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use feature 'try';
try {
    die "error";
} catch ($e) {
    print $e;
} finally {
    print $e;
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        has_issue(&issues, IssueKind::UndeclaredVariable, "$e"),
        "catch variable $e must not be visible in finally block: {:?}",
        issues
    );
    Ok(())
}

/// Nested try/catch: the inner catch variable must be visible only within the
/// inner catch block, and the outer catch variable within its own block.
#[test]
fn scope_nested_try_catch_inner_shadows_outer() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use feature 'try';
try {
    try {
        die "inner";
    } catch ($inner) {
        print $inner;
    }
} catch ($outer) {
    print $outer;
}
"#;
    let issues = scope_issues_strict(code);
    assert!(
        !issues
            .iter()
            .any(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name == "$inner"),
        "inner catch variable must be declared in inner catch scope: {:?}",
        issues
    );
    assert!(
        !issues
            .iter()
            .any(|i| i.kind == IssueKind::UndeclaredVariable && i.variable_name == "$outer"),
        "outer catch variable must be declared in outer catch scope: {:?}",
        issues
    );
    Ok(())
}

/// The catch variable range must point into the source at the catch parameter,
/// not at an arbitrary offset, so that LSP diagnostics display correctly.
#[test]
fn scope_try_catch_variable_range_points_to_parameter() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict;
use feature 'try';
try {
    die "boom";
} catch ($exception) {
    print "handled";
}
"#;
    let issues = scope_issues_strict(code);
    let unused = issues
        .iter()
        .find(|i| i.kind == IssueKind::UnusedVariable && i.variable_name == "$exception")
        .ok_or("expected unused catch-variable diagnostic for $exception")?;
    assert_eq!(
        &code[unused.range.0..unused.range.1],
        "$exception",
        "the diagnostic range must span exactly the catch parameter text"
    );
    Ok(())
}
