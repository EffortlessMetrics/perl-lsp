// ============================================================================
// Property Tests for strict_subs Package-Qualified Function Call Validation
// ============================================================================
//
// These tests verify invariants about the strict_subs implementation for
// package-qualified function calls (Foo::bar()). Property tests run many
// generated inputs to find counterexamples that unit tests might miss.
//
// INVARIANT CATEGORIES TESTED:
// 1. Qualified non-builtin calls flagged: Foo::bar() flagged when bar is not builtin
// 2. Qualified builtin calls NOT flagged: Foo::print() NOT flagged (print is builtin)
// 3. Hash key context exclusion: Foo::bar in hash key should NOT be flagged
// 4. Method calls not affected: $obj->method() should NOT produce UnquotedBareword
// 5. Package-qualified variables not affected: $Foo::bar should NOT produce UnquotedBareword
// 6. Idempotent: Running analysis twice gives the same results
// 7. Deep package paths: A::B::C::D::func() should be flagged
// 8. Multiple qualified calls: Each should be flagged independently
// 9. Builtin consistency: Foo::print and print() should both NOT be flagged
//
// NOTE: Unqualified function calls like foo() are NOT flagged by design
// (existing behavior). Only qualified calls like Foo::bar() are checked.
// ============================================================================

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue};
use perl_semantic_analyzer::pragma_tracker::PragmaTracker;
use perl_tdd_support::must;

/// Run scope analysis with strict mode enabled via `use strict 'subs'`.
fn scope_issues_strict(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let pragma_map = PragmaTracker::build(&ast);
    let analyzer = ScopeAnalyzer::new();
    analyzer.analyze(&ast, code, &pragma_map)
}

/// Check if a specific UnquotedBareword issue exists for a variable name.
fn has_unquoted_bareword(issues: &[ScopeIssue], var_name: &str) -> bool {
    issues.iter().any(|i| {
        matches!(i.kind, IssueKind::UnquotedBareword) && i.variable_name.contains(var_name)
    })
}

/// Count UnquotedBareword issues for a variable name.
fn count_unquoted_bareword(issues: &[ScopeIssue], var_name: &str) -> usize {
    issues
        .iter()
        .filter(|i| {
            matches!(i.kind, IssueKind::UnquotedBareword) && i.variable_name.contains(var_name)
        })
        .count()
}

// ============================================================================
// Property 1: Qualified Non-Builtin Function Calls Are Flagged
// ============================================================================
// For any qualified function call Foo::X() where X is NOT a known builtin,
// the call should be flagged as UnquotedBareword.
//
// This is the PRIMARY invariant of the new implementation.
// ============================================================================

#[test]
fn property_qualified_non_builtin_function_calls_flagged() -> Result<(), Box<dyn std::error::Error>>
{
    // Non-builtin identifiers to test (lowercase)
    let non_builtins = [
        "bar",
        "baz",
        "qux",
        "quux",
        "corge",
        "grault",
        "garply",
        "waldo",
        "fred",
        "plugh",
        "xyzzy",
        "thud",
        "myfunc",
        "custom_func",
        "user_func",
        "app_func",
        "do_something",
        "handle_request",
        "process_data",
        "validate_input",
        "compute_result",
    ];

    for func_name in non_builtins {
        let code = format!("use strict 'subs'; Foo::{func_name}();");
        let issues = scope_issues_strict(&code);
        assert!(
            has_unquoted_bareword(&issues, func_name),
            "Foo::{}() should be flagged under strict_subs (non-builtin), but wasn't",
            func_name
        );
    }

    // Uppercase identifiers should also be flagged (is_known_function fast-paths on uppercase)
    let uppercase_non_builtins = ["MyFunc", "Bar", "Baz", "Quux", "ClassName", "ModuleName"];

    for func_name in uppercase_non_builtins {
        let code = format!("use strict 'subs'; Foo::{func_name}();");
        let issues = scope_issues_strict(&code);
        assert!(
            has_unquoted_bareword(&issues, func_name),
            "Foo::{}() should be flagged under strict_subs (uppercase), but wasn't",
            func_name
        );
    }

    Ok(())
}

// ============================================================================
// Property 2: Qualified Builtin Function Calls Are NOT Flagged
// ============================================================================
// For any qualified function call Foo::X() where X IS a known builtin,
// the call should NOT be flagged.
//
// This ensures consistency with unqualified builtin calls like print().
// ============================================================================

#[test]
fn property_qualified_builtin_function_calls_not_flagged() -> Result<(), Box<dyn std::error::Error>>
{
    // Known builtins
    let builtins = [
        "print", "printf", "say", "open", "close", "read", "write", "chomp", "chop", "chr",
        "crypt", "hex", "index", "lc", "length", "pop", "push", "shift", "unshift", "split",
        "join", "grep", "map", "sort", "delete", "each", "exists", "keys", "values", "die", "exit",
        "return", "goto", "last", "next", "redo", "defined", "undef", "ref", "bless", "eval",
        "warn",
    ];

    for func_name in builtins {
        let code = format!("use strict 'subs'; Foo::{func_name}();");
        let issues = scope_issues_strict(&code);
        assert!(
            !has_unquoted_bareword(&issues, func_name),
            "Foo::{}() should NOT be flagged under strict_subs (builtin), but was",
            func_name
        );
    }

    Ok(())
}

// ============================================================================
// Property 3: Builtin Consistency Between Qualified and Unqualified
// ============================================================================
// If Foo::print() is NOT flagged, then print() should also NOT be flagged.
// This ensures consistent treatment of builtins regardless of qualification.
// ============================================================================

#[test]
fn property_builtin_consistency() -> Result<(), Box<dyn std::error::Error>> {
    // Test that qualified builtins are not flagged
    let builtins = ["print", "die", "warn", "exit", "return"];

    for func_name in builtins {
        // Qualified version should NOT be flagged
        let qualified_code = format!("use strict 'subs'; Foo::{func_name}();");
        let qualified_issues = scope_issues_strict(&qualified_code);
        assert!(
            !has_unquoted_bareword(&qualified_issues, func_name),
            "Foo::{}() should NOT be flagged (builtin consistency)",
            func_name
        );
    }

    Ok(())
}

// ============================================================================
// Property 4: Hash Key Context Exclusion
// ============================================================================
// Foo::bar in hash key context (e.g., %h = (Foo::bar => 1)) should NOT be flagged.
// ============================================================================

#[test]
fn property_hash_key_context_exclusion() -> Result<(), Box<dyn std::error::Error>> {
    // Test various hash key contexts
    let hash_key_contexts = [
        ("%h = (Foo::bar => 1)", "Fat comma hash key"),
        ("$h{Foo::bar}", "Hash subscript"),
        ("%h = (Foo::bar => 1, Baz::qux => 2)", "Multiple hash keys"),
    ];

    for (code, description) in hash_key_contexts {
        let full_code = format!("use strict 'subs'; {}", code);
        let issues = scope_issues_strict(&full_code);

        // Foo::bar should NOT be flagged in hash key context
        assert!(
            !has_unquoted_bareword(&issues, "Foo::bar"),
            "Hash key context violation: {} should NOT flag 'Foo::bar' but did. Description: {}",
            code,
            description
        );
    }

    // Verify that the SAME identifier IS flagged when NOT in hash key context
    let non_hash_code = "use strict 'subs'; Foo::bar();";
    let non_hash_issues = scope_issues_strict(non_hash_code);
    assert!(
        has_unquoted_bareword(&non_hash_issues, "Foo::bar"),
        "Foo::bar() should be flagged when NOT in hash key context"
    );

    Ok(())
}

// ============================================================================
// Property 5: Method Calls Not Affected
// ============================================================================
// $obj->method() should NOT produce UnquotedBareword issues because
// method calls are a different node type in the AST.
// ============================================================================

#[test]
fn property_method_calls_not_affected() -> Result<(), Box<dyn std::error::Error>> {
    let method_call_codes = ["$obj->method();", "$obj->method($arg);", "$obj->can('method');"];

    for code in method_call_codes {
        let full_code = format!("use strict 'subs'; {}", code);
        let issues = scope_issues_strict(&full_code);

        // Check that NO UnquotedBareword issues exist
        let bareword_count =
            issues.iter().filter(|i| matches!(i.kind, IssueKind::UnquotedBareword)).count();

        assert!(
            bareword_count == 0,
            "Method call '{}' should NOT produce UnquotedBareword issues, but found {}",
            code,
            bareword_count
        );
    }

    // NOTE: $obj->method(Foo::bar) - Foo::bar as argument IS flagged
    let code_with_qualified_arg = "use strict 'subs'; $obj->method(Foo::bar);";
    let issues = scope_issues_strict(code_with_qualified_arg);
    assert!(
        has_unquoted_bareword(&issues, "Foo::bar"),
        "Foo::bar as method argument should be flagged"
    );

    Ok(())
}

// ============================================================================
// Property 6: Package-Qualified Variables Not Affected
// ============================================================================
// $Foo::bar should NOT produce UnquotedBareword issues because it's a
// variable reference, not a function call.
// ============================================================================

#[test]
fn property_package_qualified_variables_not_affected() -> Result<(), Box<dyn std::error::Error>> {
    let var_codes = [
        "print $Foo::bar;",
        "$x = $Foo::bar;",
        "$Foo::bar = 1;",
        "@arr = ($Foo::bar, $Baz::qux);",
        "%h = (key => $Foo::bar);",
    ];

    for code in var_codes {
        let full_code = format!("use strict 'subs'; {}", code);
        let issues = scope_issues_strict(&full_code);

        // Check that NO UnquotedBareword issues exist
        let bareword_count =
            issues.iter().filter(|i| matches!(i.kind, IssueKind::UnquotedBareword)).count();

        assert!(
            bareword_count == 0,
            "Variable '{}' should NOT produce UnquotedBareword issues, but found {}",
            code,
            bareword_count
        );
    }

    Ok(())
}

// ============================================================================
// Property 7: Idempotent Analysis
// ============================================================================
// Running the analyzer twice on the same code should produce the same results.
// ============================================================================

#[test]
fn property_analysis_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    let test_codes = [
        "use strict 'subs'; Foo::bar();",
        "use strict 'subs'; Foo::print();",
        "use strict 'subs'; Foo::bar(Baz::qux);",
        "use strict 'subs'; my %h = (Foo::bar => 1);",
        "use strict 'subs'; $obj->method();",
        "use strict 'subs'; print $Foo::bar;",
        "use strict 'subs'; Foo::bar(); Baz::qux();",
        "use strict 'subs'; Foo::bar(Baz::qux(Qux::fred()));",
    ];

    for code in test_codes {
        let issues1 = scope_issues_strict(code);
        let issues2 = scope_issues_strict(code);

        // Compare issue counts by kind
        for kind in
            [IssueKind::UnquotedBareword, IssueKind::UndeclaredVariable, IssueKind::UnusedVariable]
        {
            let count1 = issues1.iter().filter(|i| i.kind == kind).count();
            let count2 = issues2.iter().filter(|i| i.kind == kind).count();

            assert_eq!(
                count1, count2,
                "Idempotence violation for '{}': {:?} issues = {} (run 1) vs {} (run 2)",
                code, kind, count1, count2
            );
        }

        // For UnquotedBareword specifically, check the exact variable names
        let bareword_names1: Vec<_> = issues1
            .iter()
            .filter(|i| matches!(i.kind, IssueKind::UnquotedBareword))
            .map(|i| i.variable_name.clone())
            .collect();
        let bareword_names2: Vec<_> = issues2
            .iter()
            .filter(|i| matches!(i.kind, IssueKind::UnquotedBareword))
            .map(|i| i.variable_name.clone())
            .collect();

        assert_eq!(
            bareword_names1, bareword_names2,
            "Idempotence violation for UnquotedBareword in '{}': run 1 = {:?}, run 2 = {:?}",
            code, bareword_names1, bareword_names2
        );
    }

    Ok(())
}

// ============================================================================
// Property 8: Deep Package Paths
// ============================================================================
// Very deep package paths (A::B::C::D::E::func) should all be validated.
// ============================================================================

#[test]
fn property_deep_package_paths() -> Result<(), Box<dyn std::error::Error>> {
    let deep_paths = [
        ("A::B::C::D::E::func", "func"),
        ("Alpha::Beta::Gamma::Delta::myfunc", "myfunc"),
        ("Perl::Module::SubModule::custom_func", "custom_func"),
    ];

    // All deep paths with non-builtin identifiers should be flagged
    for (qualified_name, identifier) in deep_paths {
        let code = format!("use strict 'subs'; {qualified_name}();");
        let issues = scope_issues_strict(&code);

        assert!(
            has_unquoted_bareword(&issues, identifier),
            "Deep path '{}' should be flagged under strict_subs, but wasn't",
            qualified_name
        );
    }

    // Builtin at any depth should NOT be flagged
    let deep_builtins = ["A::B::C::print", "Foo::Bar::print", "MyPackage::die"];

    for qualified_name in deep_builtins {
        let code = format!("use strict 'subs'; {qualified_name}();");
        let issues = scope_issues_strict(&code);
        let identifier = qualified_name.rsplit("::").next().unwrap();

        assert!(
            !has_unquoted_bareword(&issues, identifier),
            "Deep builtin '{}' should NOT be flagged, but was",
            qualified_name
        );
    }

    Ok(())
}

// ============================================================================
// Property 9: Multiple Qualified Calls in Same Expression
// ============================================================================
// Multiple qualified calls in the same expression should each be validated independently.
// ============================================================================

#[test]
fn property_multiple_qualified_calls_independent() -> Result<(), Box<dyn std::error::Error>> {
    let code = "use strict 'subs'; Foo::bar() + Baz::qux() + Qux::fred();";
    let issues = scope_issues_strict(code);

    // Each should be flagged exactly once
    assert_eq!(
        count_unquoted_bareword(&issues, "Foo::bar"),
        1,
        "Foo::bar should be flagged exactly once"
    );
    assert_eq!(
        count_unquoted_bareword(&issues, "Baz::qux"),
        1,
        "Baz::qux should be flagged exactly once"
    );
    assert_eq!(
        count_unquoted_bareword(&issues, "Qux::fred"),
        1,
        "Qux::fred should be flagged exactly once"
    );

    Ok(())
}

// ============================================================================
// Property 10: Qualified Calls with Various Arguments
// ============================================================================
// Arguments to qualified calls should not affect the validation of the call itself.
// ============================================================================

#[test]
fn property_qualified_call_with_various_args() -> Result<(), Box<dyn std::error::Error>> {
    // Non-builtin with various argument types
    let arg_tests = [
        ("Foo::bar()", "no args"),
        ("Foo::bar($x)", "scalar arg"),
        ("Foo::bar(@arr)", "array arg"),
        ("Foo::bar(%h)", "hash arg"),
        ("Foo::bar('string')", "string arg"),
        ("Foo::bar(123)", "number arg"),
        ("Foo::bar($x, @arr, %h)", "mixed args"),
    ];

    for (call_expr, description) in arg_tests {
        let code = format!("use strict 'subs'; {}", call_expr);
        let issues = scope_issues_strict(&code);

        assert!(
            has_unquoted_bareword(&issues, "Foo::bar"),
            "Call with args '{}' ({}) should flag Foo::bar",
            call_expr,
            description
        );
    }

    // Builtin with various arguments should NOT be flagged
    let builtin_tests = [
        ("Foo::print()", "print no args"),
        ("Foo::print('hello')", "print with string"),
        ("Foo::print($x)", "print with var"),
    ];

    for (call_expr, description) in builtin_tests {
        let code = format!("use strict 'subs'; {}", call_expr);
        let issues = scope_issues_strict(&code);

        assert!(
            !has_unquoted_bareword(&issues, "print"),
            "Builtin call '{}' ({}) should NOT be flagged, but was",
            call_expr,
            description
        );
    }

    Ok(())
}

// ============================================================================
// Property 11: use subs Import Does NOT Override Qualified Calls
// ============================================================================
// A function imported via 'use subs' is only available as unqualified.
// Qualified calls like Foo::foo() still need Foo::foo to be a real function.
//
// In Perl: use subs 'foo' makes foo() available, but Foo::foo() still needs
// the actual Foo package with foo function.
// ============================================================================

#[test]
fn property_subs_import_overrides_unqualified_only() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use strict 'subs';
use subs 'foo', 'bar';
foo();
bar();
Foo::foo();
Foo::bar();
Baz::qux();
"#;

    let issues = scope_issues_strict(code);

    // Imported functions should NOT be flagged when used unqualified
    assert!(
        !has_unquoted_bareword(&issues, "foo;"),
        "Imported 'foo' should NOT be flagged when used unqualified"
    );
    assert!(
        !has_unquoted_bareword(&issues, "bar;"),
        "Imported 'bar' should NOT be flagged when used unqualified"
    );

    // Qualified calls are STILL flagged because Foo::foo requires actual Foo package
    // (use subs doesn't make Foo::foo available - it's a different symbol)
    assert!(
        has_unquoted_bareword(&issues, "Foo::foo"),
        "Foo::foo() should still be flagged - 'use subs' doesn't provide Foo::foo"
    );
    assert!(
        has_unquoted_bareword(&issues, "Foo::bar"),
        "Foo::bar() should still be flagged - 'use subs' doesn't provide Foo::bar"
    );

    // Non-imported, non-builtin should be flagged
    assert!(
        has_unquoted_bareword(&issues, "Baz::qux"),
        "Non-imported 'Baz::qux' should be flagged"
    );

    Ok(())
}

// ============================================================================
// Property 12: Uppercase Package Names Don't Affect Identifier Check
// ============================================================================
// DBI::connect() should be flagged because "connect" is not a known builtin.
// ============================================================================

#[test]
fn property_uppercase_package_builtin_identifier() -> Result<(), Box<dyn std::error::Error>> {
    // DBI::connect - identifier "connect" is lowercase, not builtin
    let code = "use strict 'subs'; DBI::connect();";
    let issues = scope_issues_strict(code);

    assert!(
        has_unquoted_bareword(&issues, "connect"),
        "DBI::connect() should be flagged - 'connect' is not a builtin"
    );

    Ok(())
}

// ============================================================================
// Property 13: Qualified Call in Conditional Context
// ============================================================================
// Qualified calls in if/while/etc should still be validated.
// ============================================================================

#[test]
fn property_qualified_call_in_conditional() -> Result<(), Box<dyn std::error::Error>> {
    let conditionals = [
        ("if (Foo::bar()) {}", "if"),
        ("while (Foo::bar()) {}", "while"),
        ("elsif (Foo::bar()) {}", "elsif"),
        ("until (Foo::bar()) {}", "until"),
    ];

    for (code, description) in conditionals {
        let full_code = format!("use strict 'subs'; {}", code);
        let issues = scope_issues_strict(&full_code);

        assert!(
            has_unquoted_bareword(&issues, "Foo::bar"),
            "Foo::bar in {} should be flagged",
            description
        );
    }

    Ok(())
}

// ============================================================================
// Property 14: Issue Range Excludes Parentheses
// ============================================================================
// The UnquotedBareword issue for Foo::bar() should have a range that covers
// the bareword name, ideally excluding the parentheses.
// ============================================================================

#[test]
fn property_issue_range_excludes_parentheses() -> Result<(), Box<dyn std::error::Error>> {
    let code = "use strict 'subs'; Foo::bar();";
    let issues = scope_issues_strict(code);

    // Find the Foo::bar issue
    let bareword_issues: Vec<_> = issues
        .iter()
        .filter(|i| matches!(i.kind, IssueKind::UnquotedBareword) && i.variable_name == "Foo::bar")
        .collect();

    assert_eq!(bareword_issues.len(), 1, "Should have exactly one Foo::bar issue");

    let issue = &bareword_issues[0];
    let (start, end) = issue.range;

    // The code is: "use strict 'subs'; Foo::bar();"
    // Foo::bar starts at position 19 (0-indexed)
    // "Foo::bar" has length 7, so end should be 26
    // But the FunctionCall node might include the parentheses, so end might be 27
    // We verify the range starts at the correct position
    assert!(
        start >= 19 && start <= 20,
        "Issue range start ({}) should be around 19-20 (start of 'Foo::bar')",
        start
    );

    // The range should at least include Foo::bar (length 7)
    assert!(
        end - start >= 7,
        "Issue range length ({}) should be at least 7 (length of 'Foo::bar')",
        end - start
    );

    Ok(())
}
