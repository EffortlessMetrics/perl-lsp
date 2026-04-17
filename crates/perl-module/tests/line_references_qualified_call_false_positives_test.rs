//! Tests for `line_references_qualified_call` false-positive bug fix.
//!
//! Bug: `line_references_qualified_call` incorrectly returns `true` for:
//! 1. String literals containing qualified call patterns (e.g., `"My::Module::func()"`)
//! 2. Package declarations with nested modules (e.g., `package My::Module::Sub;`)
//!
//! The function should only return `true` for actual qualified function calls,
//! not for strings or package declarations.

// Tests for the bug fix - these should FAIL before the fix is implemented

use perl_module::line_references_qualified_call;

// ──────────────────────────────────────────────────────────────
// Bug: False positives on string literals
// ──────────────────────────────────────────────────────────────

/// Bug fix test: string literals should NOT be detected as qualified calls.
/// Even if a string literal contains `My::Module::func()`, it's not a call.
#[test]
fn test_line_references_qualified_call_false_positive_on_double_quoted_string() {
    // Given a string literal containing a qualified call pattern
    let line = r#"my $str = "My::Module::func()";"#;
    let module_name = "My::Module";

    // When we check if the line references a qualified call
    let result = line_references_qualified_call(line, module_name);

    // Then it should NOT match (string literal is not a call)
    assert!(
        !result,
        "line_references_qualified_call should return false for string literals, \
         but returned true for: {}",
        line
    );
}

/// Bug fix test: single-quoted string literals should NOT be detected.
#[test]
fn test_line_references_qualified_call_false_positive_on_single_quoted_string() {
    let line = r#"my $str = 'My::Module::func();';"#;
    let module_name = "My::Module";

    let result = line_references_qualified_call(line, module_name);

    assert!(
        !result,
        "line_references_qualified_call should return false for single-quoted strings, \
         but returned true for: {}",
        line
    );
}

/// Bug fix test: qw() list containing a qualified call pattern should NOT be detected.
#[test]
fn test_line_references_qualified_call_false_positive_on_qw_list() {
    let line = r#"my @mods = qw(My::Module::func My::Module::Sub);"#;
    let module_name = "My::Module";

    // qw() is a word list, not a qualified call, even though it contains :: patterns
    let result = line_references_qualified_call(line, module_name);

    assert!(
        !result,
        "line_references_qualified_call should return false for qw() lists, \
         but returned true for: {}",
        line
    );
}

// ──────────────────────────────────────────────────────────────
// Bug: False positives on package declarations
// ──────────────────────────────────────────────────────────────

/// Bug fix test: package declarations with nested modules should NOT be
/// detected as qualified calls. `package My::Module::Sub;` declares a package
/// named `My::Module::Sub`, it is NOT a call to `My::Module`.
#[test]
fn test_line_references_qualified_call_false_positive_on_package_declaration() {
    let line = "package My::Module::Sub;";
    let module_name = "My::Module";

    let result = line_references_qualified_call(line, module_name);

    assert!(
        !result,
        "line_references_qualified_call should return false for package declarations, \
         but returned true for: {}",
        line
    );
}

/// Bug fix test: package declaration at start of line with whitespace.
#[test]
fn test_line_references_qualified_call_false_positive_on_package_declaration_with_leading_space() {
    let line = "    package My::Module::Sub;";
    let module_name = "My::Module";

    let result = line_references_qualified_call(line, module_name);

    assert!(
        !result,
        "line_references_qualified_call should return false for indented package declarations, \
         but returned true for: {}",
        line
    );
}

/// Bug fix test: package declaration with modern syntax.
#[test]
fn test_line_references_qualified_call_false_positive_on_package_declaration_v5_12() {
    let line = "package My::Module::Sub v1.2.3;";
    let module_name = "My::Module";

    let result = line_references_qualified_call(line, module_name);

    assert!(
        !result,
        "line_references_qualified_call should return false for package declarations with version, \
         but returned true for: {}",
        line
    );
}

// ──────────────────────────────────────────────────────────────
// Correct positives (must continue to work)
// ──────────────────────────────────────────────────────────────

/// Verify actual qualified calls still return true.
#[test]
fn test_line_references_qualified_call_correct_positive_on_qualified_call() {
    let line = "My::Module::func();";
    let module_name = "My::Module";

    let result = line_references_qualified_call(line, module_name);

    assert!(
        result,
        "line_references_qualified_call should return true for qualified calls, \
         but returned false for: {}",
        line
    );
}

/// Verify qualified calls with arguments still return true.
#[test]
fn test_line_references_qualified_call_correct_positive_on_qualified_call_with_args() {
    let line = "My::Module::func($arg1, $arg2);";
    let module_name = "My::Module";

    let result = line_references_qualified_call(line, module_name);

    assert!(
        result,
        "line_references_qualified_call should return true for qualified calls with args, \
         but returned false for: {}",
        line
    );
}

/// Verify qualified calls with method chaining still return true.
#[test]
fn test_line_references_qualified_call_correct_positive_on_chained_call() {
    let line = "My::Module::func()->then();";
    let module_name = "My::Module";

    let result = line_references_qualified_call(line, module_name);

    assert!(
        result,
        "line_references_qualified_call should return true for chained qualified calls, \
         but returned false for: {}",
        line
    );
}

// ──────────────────────────────────────────────────────────────
// Edge cases
// ──────────────────────────────────────────────────────────────

/// Empty line should return false.
#[test]
fn test_line_references_qualified_call_empty_line() {
    assert!(!line_references_qualified_call("", "My::Module"));
}

/// Empty module name should return false.
#[test]
fn test_line_references_qualified_call_empty_module_name() {
    assert!(!line_references_qualified_call("My::Module::func();", ""));
}

/// Module name not present should return false.
#[test]
fn test_line_references_qualified_call_module_not_present() {
    assert!(!line_references_qualified_call("Other::Module::func();", "My::Module"));
}

/// Partial module name match should not trigger (no false positive).
#[test]
fn test_line_references_qualified_call_partial_match_no_false_positive() {
    // "My::ModuleX::func()" contains "My::Module" but is NOT a call to My::Module
    let line = "My::ModuleX::func();";
    let module_name = "My::Module";

    let result = line_references_qualified_call(line, module_name);

    assert!(
        !result,
        "line_references_qualified_call should not match partial module names, \
         but returned true for: {}",
        line
    );
}

/// Multiple qualified calls on same line.
#[test]
fn test_line_references_qualified_call_multiple_calls_same_line() {
    let line = "My::Module::foo(); My::Module::bar();";
    let module_name = "My::Module";

    let result = line_references_qualified_call(line, module_name);

    assert!(
        result,
        "line_references_qualified_call should detect qualified calls even when multiple exist, \
         but returned false for: {}",
        line
    );
}

/// Comment containing qualified call pattern should NOT be detected.
#[test]
fn test_line_references_qualified_call_false_positive_in_comment() {
    let line = "# This calls My::Module::func() for debugging";
    let module_name = "My::Module";

    let result = line_references_qualified_call(line, module_name);

    assert!(
        !result,
        "line_references_qualified_call should return false for comments, \
         but returned true for: {}",
        line
    );
}

/// Nested module name that looks like a call but isn't.
#[test]
fn test_line_references_qualified_call_nested_module_not_a_call() {
    // This is a package declaration for a nested module, not a call
    let line = "package My::Module::Implementation::Detail;";
    let module_name = "My::Module";

    let result = line_references_qualified_call(line, module_name);

    assert!(
        !result,
        "line_references_qualified_call should return false for nested package declarations, \
         but returned true for: {}",
        line
    );
}
