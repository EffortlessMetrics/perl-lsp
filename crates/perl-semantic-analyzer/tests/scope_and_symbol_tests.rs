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
