mod cpan_test_helpers;
use cpan_test_helpers::*;

// =============================================================================
// Basic if/elsif/else chains (should already work, regression coverage)
// =============================================================================

#[test]
fn test_if_else_basic() {
    assert_clean_parse("if ($x) { foo(); } else { bar(); }");
}

#[test]
fn test_if_elsif_else_basic() {
    assert_clean_parse("if ($x) { 1; } elsif ($y) { 2; } else { 3; }");
}

#[test]
fn test_if_multiple_elsif() {
    assert_clean_parse(
        "if ($a) { 1; } elsif ($b) { 2; } elsif ($c) { 3; } elsif ($d) { 4; } else { 5; }",
    );
}

#[test]
fn test_if_elsif_no_else() {
    assert_clean_parse("if ($x) { foo(); } elsif ($y) { bar(); }");
}

// =============================================================================
// unless with else/elsif (the main fix: unless now supports else/elsif branches)
// =============================================================================

#[test]
fn test_unless_else() {
    assert_clean_parse("unless ($done) { work(); } else { cleanup(); }");
}

#[test]
fn test_unless_elsif_else() {
    assert_clean_parse(
        "unless ($done) { work(); } elsif ($paused) { wait_more(); } else { cleanup(); }",
    );
}

#[test]
fn test_unless_multiple_elsif_else() {
    assert_clean_parse(
        r#"
unless ($error) {
    proceed();
} elsif ($warning) {
    log_warning();
} elsif ($info) {
    log_info();
} else {
    log_debug();
}
"#,
    );
}

#[test]
fn test_unless_elsif_no_else() {
    assert_clean_parse("unless ($a) { foo(); } elsif ($b) { bar(); }");
}

// =============================================================================
// Complex if/elsif/else with various condition types
// =============================================================================

#[test]
fn test_if_elsif_with_complex_conditions() {
    assert_clean_parse(
        r#"
if ($x > 0 && $y < 10) {
    positive();
} elsif ($x == 0 || $y == 0) {
    zero();
} elsif (defined $z) {
    has_z();
} else {
    fallback();
}
"#,
    );
}

#[test]
fn test_if_elsif_with_regex_condition() {
    assert_clean_parse(
        r#"
if ($line =~ /^#/) {
    comment();
} elsif ($line =~ /^\s*$/) {
    blank();
} else {
    code();
}
"#,
    );
}

#[test]
fn test_nested_if_elsif_else() {
    assert_clean_parse(
        r#"
if ($a) {
    if ($b) {
        inner1();
    } elsif ($c) {
        inner2();
    } else {
        inner3();
    }
} elsif ($d) {
    if ($e) {
        inner4();
    } else {
        inner5();
    }
} else {
    outer();
}
"#,
    );
}

#[test]
fn test_if_elsif_with_my_in_condition() {
    assert_clean_parse(
        r#"
if (my $x = get_value()) {
    use_x($x);
} elsif (my $y = get_fallback()) {
    use_y($y);
} else {
    default_action();
}
"#,
    );
}

#[test]
fn test_if_elsif_with_function_calls() {
    assert_clean_parse(
        r#"
if (is_admin($user)) {
    admin_panel();
} elsif (is_moderator($user)) {
    mod_panel();
} elsif (is_member($user)) {
    member_panel();
} else {
    guest_panel();
}
"#,
    );
}

// =============================================================================
// Orphaned else/elsif recovery (error recovery, should not crash)
// =============================================================================

#[test]
fn test_orphaned_else_does_not_crash() {
    // An orphaned else should produce an AST (with errors recorded), not crash
    let ast = parse("else { fallback(); }");
    let sexp = ast.to_sexp();
    // Should produce an If node wrapping the else block
    assert!(sexp.contains("if"), "orphaned else should produce an if node, got: {}", sexp);
}

#[test]
fn test_orphaned_elsif_does_not_crash() {
    // An orphaned elsif should produce an AST (with errors recorded), not crash
    let ast = parse("elsif ($x) { foo(); }");
    let sexp = ast.to_sexp();
    // Should produce an If node wrapping the elsif
    assert!(sexp.contains("if"), "orphaned elsif should produce an if node, got: {}", sexp);
}

#[test]
fn test_orphaned_elsif_with_else_chain() {
    // An orphaned elsif followed by else should still parse the chain
    let ast = parse("elsif ($x) { foo(); } else { bar(); }");
    let sexp = ast.to_sexp();
    assert!(sexp.contains("if"), "orphaned elsif chain should produce an if node, got: {}", sexp);
}

#[test]
fn test_orphaned_else_followed_by_valid_code() {
    // After recovering from orphaned else, following valid code should parse
    let ast = parse("else { bad(); }\nmy $x = 1;");
    let sexp = ast.to_sexp();
    // Should have both the recovered If and the variable declaration
    assert!(sexp.contains("if"), "should contain recovered if node, got: {}", sexp);
}

// =============================================================================
// else/elsif as hash keys (fat arrow autoquoting)
// =============================================================================

#[test]
fn test_else_as_hash_key() {
    assert_clean_parse("my %dispatch = (if => \\&handle_if, else => \\&handle_else);");
}

#[test]
fn test_elsif_as_hash_key() {
    assert_clean_parse("my %h = (elsif => 1, else => 2);");
}

// =============================================================================
// CPAN-style patterns with if/elsif/else
// =============================================================================

#[test]
fn test_cpan_dispatcher_pattern() {
    assert_clean_parse(
        r#"
sub dispatch {
    my ($type) = @_;
    if ($type eq 'create') {
        create_handler();
    } elsif ($type eq 'read') {
        read_handler();
    } elsif ($type eq 'update') {
        update_handler();
    } elsif ($type eq 'delete') {
        delete_handler();
    } else {
        die "Unknown type: $type";
    }
}
"#,
    );
}

#[test]
fn test_cpan_error_handling_pattern() {
    assert_clean_parse(
        r#"
eval {
    risky_operation();
};
if ($@) {
    if (ref $@ eq 'My::Error') {
        handle_custom($@);
    } elsif ($@ =~ /timeout/) {
        handle_timeout();
    } else {
        die $@;
    }
}
"#,
    );
}

#[test]
fn test_if_elsif_in_loop() {
    assert_clean_parse(
        r#"
for my $item (@list) {
    if ($item->{type} eq 'A') {
        process_a($item);
    } elsif ($item->{type} eq 'B') {
        process_b($item);
    } elsif ($item->{type} eq 'C') {
        process_c($item);
    } else {
        warn "Unknown type: $item->{type}";
    }
}
"#,
    );
}

#[test]
fn test_unless_else_in_sub() {
    assert_clean_parse(
        r#"
sub validate {
    my ($input) = @_;
    unless (defined $input) {
        return 0;
    } else {
        return length($input) > 0;
    }
}
"#,
    );
}
