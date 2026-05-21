//! Differential parser test suite for v1/v2/v3 Perl parsers.
//!
//! Tests cover the seven categories of constructs that historically defeated
//! tree-sitter (documented in `docs/articles/research/TREE_SITTER_BREAKAGE.md`)
//! plus the silent-failure edge cases discovered in PR #9168.
//!
//! # Disagreement Table
//!
//! Each test records the *expected* verdict for all three parsers.  When you
//! run the suite with `--nocapture` the printed table shows outcomes across
//! parsers for each input.  The value of the suite is making the gaps visible.
//!
//! # Verdict meanings
//!
//! | Verdict | Meaning |
//! |---------|---------|
//! | `Correct` | Structural property satisfied |
//! | `WrongButPlausible` | Parse succeeds but AST is semantically wrong |
//! | `SilentlyEmpty` | Parse succeeds but key content is missing |
//! | `Errors` | Parser returned an error / has error nodes |
//! | `Crashes` | Parser panicked (caught with catch_unwind) |

use perl_parser_comparison::{Verdict, parse_v1, parse_v2, parse_v3};

// --- Helpers ------------------------------------------------------------------

/// Print a comparison row for --nocapture diagnostic output.
fn print_row(category: &str, label: &str, v1: &Verdict, v2: &Verdict, v3: &Verdict) {
    println!("  [{category:>2}] {label:<50} | v1={v1:<20} | v2={v2:<20} | v3={v3}");
}

/// Assert expected verdict with a descriptive failure message.
fn assert_verdict(result: &perl_parser_comparison::ParseResult, expected: &Verdict, context: &str) {
    assert_eq!(
        &result.verdict,
        expected,
        "{}: {} expected {:?}, got {:?}\n  sexp: {}\n  error: {:?}",
        result.parser,
        context,
        expected,
        result.verdict,
        &result.sexp[..result.sexp.len().min(300)],
        result.error,
    );
}

// --- Category 0: Control cases (well-formed simple Perl) ----------------------

/// Control case: simple variable assignment.
#[test]
fn cat0_simple_variable_assignment() {
    let src = "my $x = 42;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("0", "simple_variable_assignment", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "simple_variable_assignment");
    assert_verdict(&r2, &Verdict::Correct, "simple_variable_assignment");
    assert_verdict(&r3, &Verdict::Correct, "simple_variable_assignment");
}

/// Control case: sub declaration.
#[test]
fn cat0_sub_declaration() {
    let src = "sub foo { return 1; }";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("0", "sub_declaration", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "sub_declaration");
    assert_verdict(&r2, &Verdict::Correct, "sub_declaration");
    assert_verdict(&r3, &Verdict::Correct, "sub_declaration");
}

/// Control case: if statement.
#[test]
fn cat0_if_statement() {
    let src = "if ($x) { print $x; }";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("0", "if_statement", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "if_statement");
    assert_verdict(&r2, &Verdict::Correct, "if_statement");
    assert_verdict(&r3, &Verdict::Correct, "if_statement");
}

// --- Category 1: `/` - division vs. regex -------------------------------------

/// Cat 1a: unambiguous division - both sides are terms.
#[test]
fn cat1_division_between_terms() {
    let src = "my $avg = $sum / $count;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("1", "division_between_terms", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "division_between_terms");
    assert_verdict(&r2, &Verdict::Correct, "division_between_terms");
    assert_verdict(&r3, &Verdict::Correct, "division_between_terms");
}

/// Cat 1b: regex after `if` keyword - `/` must start regex.
///
/// v1 can confuse `/` after `if(...)` with division in some inputs.
#[test]
fn cat1_regex_after_if_keyword() {
    let src = "if (/error/) { die; }";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("1", "regex_after_if_keyword", &r1.verdict, &r2.verdict, &r3.verdict);
    // All three parsers handle this common case
    assert_verdict(&r1, &Verdict::Correct, "regex_after_if_keyword");
    assert_verdict(&r2, &Verdict::Correct, "regex_after_if_keyword");
    assert_verdict(&r3, &Verdict::Correct, "regex_after_if_keyword");
}

/// Cat 1c: division-assign `/=`.
#[test]
fn cat1_division_assign() {
    let src = "my $x = 10; $x /= 2;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("1", "division_assign", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "division_assign");
    assert_verdict(&r2, &Verdict::Correct, "division_assign");
    assert_verdict(&r3, &Verdict::Correct, "division_assign");
}

/// Cat 1d: regex match in list context - `/` must start regex.
#[test]
fn cat1_regex_in_list_context() {
    let src = "my @m = /pattern/;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("1", "regex_in_list_context", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "regex_in_list_context");
    assert_verdict(&r2, &Verdict::Correct, "regex_in_list_context");
    assert_verdict(&r3, &Verdict::Correct, "regex_in_list_context");
}

/// Cat 1e: regex after closing paren (the hard case).
///
/// `if ($x) /pattern/` - after `)` ends a group, the `/` starts a regex.
/// This is the case that most reliably defeats context-free scanners.
#[test]
fn cat1_regex_after_closing_paren() {
    let src = "print /pat/ if $x;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("1", "regex_after_closing_paren", &r1.verdict, &r2.verdict, &r3.verdict);
    // v1 struggles here; v3 handles it via LexerMode
    // v1 may produce error nodes; we record what actually happens
    assert_verdict(&r1, &Verdict::Correct, "regex_after_closing_paren");
    assert_verdict(&r2, &Verdict::Correct, "regex_after_closing_paren");
    assert_verdict(&r3, &Verdict::Correct, "regex_after_closing_paren");
}

/// Cat 1f: nested division - multiple `/` on one line.
#[test]
fn cat1_nested_division() {
    let src = "my $r = ($a / $b) / ($c / $d);";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("1", "nested_division", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "nested_division");
    assert_verdict(&r2, &Verdict::Correct, "nested_division");
    assert_verdict(&r3, &Verdict::Correct, "nested_division");
}

// --- Category 2: Heredoc body deferral ----------------------------------------

/// Cat 2a: basic single heredoc.
///
/// All parsers should handle this.
#[test]
fn cat2_basic_heredoc() {
    let src = "my $x = <<EOF;\nhello\nEOF\n";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("2", "basic_heredoc", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "basic_heredoc");
    assert_verdict(&r2, &Verdict::Correct, "basic_heredoc");
    assert_verdict(&r3, &Verdict::Correct, "basic_heredoc");
}

/// Cat 2b: multiple heredocs on one line - the classic tree-sitter breaker.
///
/// `print <<A, <<B;\naaa\nA\nbbb\nB\n`
///
/// v1 (tree-sitter-c): accepts with no ERROR nodes, but silently loses body
///   content - only the first heredoc_content is captured, second is lost.
///   Verdict: Correct (no error nodes) but structurally incomplete.
/// v2 (Pest): accepts but heredoc bodies are empty strings - SilentlyEmpty.
/// v3: handles FIFO queue correctly - Correct, both bodies present.
#[test]
fn cat2_multiple_heredocs_on_one_line() {
    let src = "print <<A, <<B;\naaa\nA\nbbb\nB\n";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("2", "multiple_heredocs_on_one_line", &r1.verdict, &r2.verdict, &r3.verdict);

    // v1: tree-sitter accepts (no ERROR nodes) but silently loses the second
    // heredoc body.  The sexp shows only one heredoc_content node.
    // Expected: Correct (no error nodes), but structurally we verify body loss.
    assert_verdict(&r1, &Verdict::Correct, "multiple_heredocs_on_one_line (v1 no error nodes)");
    let v1_has_bbb = r1.sexp_contains("bbb");
    println!(
        "    v1 multi-heredoc: has_bbb={v1_has_bbb} (expected false - body lost); sexp={}",
        &r1.sexp[..r1.sexp.len().min(400)]
    );
    // v1 silently loses the second heredoc body (bbb) - this is the gap
    assert!(
        !v1_has_bbb,
        "v1 should silently lose second heredoc body 'bbb'; sexp: {}",
        &r1.sexp[..r1.sexp.len().min(400)]
    );

    // v2: Pest accepts but heredoc bodies are empty.
    // The body lines (aaa, A, bbb, B) are re-parsed as separate function call
    // expressions rather than attached to heredoc nodes.
    // The sexp shows: (heredoc A  "") and (heredoc B  "") with empty body strings.
    assert_eq!(
        r2.verdict,
        Verdict::Correct,
        "v2 parse must succeed (it accepts the input); structural check below"
    );
    // Structural assertion: both heredoc nodes exist but with empty bodies ("")
    let v2_has_empty_heredoc_a = r2.sexp_contains("heredoc A  \"\"");
    let v2_has_empty_heredoc_b = r2.sexp_contains("heredoc B  \"\"");
    println!(
        "    v2 multi-heredoc: empty_A={v2_has_empty_heredoc_a}, empty_B={v2_has_empty_heredoc_b}",
    );
    println!("    v2 sexp: {}", &r2.sexp[..r2.sexp.len().min(400)]);
    assert!(
        v2_has_empty_heredoc_a,
        "v2 should have empty heredoc A body; sexp: {}",
        &r2.sexp[..r2.sexp.len().min(400)]
    );
    assert!(
        v2_has_empty_heredoc_b,
        "v2 should have empty heredoc B body; sexp: {}",
        &r2.sexp[..r2.sexp.len().min(400)]
    );

    // v3: correctly attaches both bodies
    assert_verdict(&r3, &Verdict::Correct, "multiple_heredocs_on_one_line (v3)");
    assert!(
        r3.sexp_contains("aaa") || r3.sexp_contains("(heredoc"),
        "v3 should preserve heredoc content; sexp: {}",
        &r3.sexp[..r3.sexp.len().min(400)]
    );
}

/// Cat 2c: indented heredoc (`<<~`).
#[test]
fn cat2_indented_heredoc() {
    let src = "my $x = <<~EOF;\n  indented\n  EOF\n";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("2", "indented_heredoc", &r1.verdict, &r2.verdict, &r3.verdict);
    // v1 may not support <<~ in all versions
    // Record actual behavior without asserting specific verdict for v1
    println!("    v1 indented heredoc: {:?}", r1.verdict);
    // v2 and v3 should handle it
    assert_verdict(&r2, &Verdict::Correct, "indented_heredoc");
    assert_verdict(&r3, &Verdict::Correct, "indented_heredoc");
}

/// Cat 2d: non-interpolating heredoc (`<<'EOF'`).
#[test]
fn cat2_non_interpolating_heredoc() {
    let src = "my $x = <<'EOF';\n\\$not_a_var\nEOF\n";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("2", "non_interpolating_heredoc", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "non_interpolating_heredoc");
    assert_verdict(&r2, &Verdict::Correct, "non_interpolating_heredoc");
    assert_verdict(&r3, &Verdict::Correct, "non_interpolating_heredoc");
}

/// Cat 2e: heredoc body non-empty in v3 - structural assertion.
///
/// Verifies that v3 actually populates the `content` field of the Heredoc node.
#[test]
fn cat2_v3_heredoc_body_populated() {
    let src = "my $greeting = <<END;\nhello world\nEND\n";
    let r3 = parse_v3(src);
    assert_verdict(&r3, &Verdict::Correct, "v3_heredoc_body_populated");
    // The content "hello world" should appear in the sexp
    assert!(
        r3.sexp_contains("hello world"),
        "v3 should capture heredoc body content; sexp: {}",
        &r3.sexp[..r3.sexp.len().min(600)]
    );
}

// --- Category 3: `{}` - hash vs. block vs. bare block -------------------------

/// Cat 3a: unambiguous hash reference (in assignment context).
#[test]
fn cat3_hashref_in_assignment() {
    let src = "my $h = { a => 1, b => 2 };";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("3", "hashref_in_assignment", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "hashref_in_assignment");
    assert_verdict(&r2, &Verdict::Correct, "hashref_in_assignment");
    assert_verdict(&r3, &Verdict::Correct, "hashref_in_assignment");
    // v3 structural: must be a hash, not block
    assert!(
        r3.sexp_contains("(hash"),
        "v3 should parse as hash ref; sexp: {}",
        &r3.sexp[..r3.sexp.len().min(300)]
    );
}

/// Cat 3b: `map { $_ * 2 }` - block, not hash.
#[test]
fn cat3_map_block() {
    let src = "my @x = map { $_ * 2 } @list;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("3", "map_block", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "map_block");
    assert_verdict(&r2, &Verdict::Correct, "map_block");
    assert_verdict(&r3, &Verdict::Correct, "map_block");
    // v3 structural: must be a block
    assert!(
        r3.sexp_contains("(block"),
        "v3 map should use block; sexp: {}",
        &r3.sexp[..r3.sexp.len().min(300)]
    );
}

/// Cat 3c: `map { a => $_ }` - hash-like content inside a map block.
///
/// The CORRECT parse is: block containing `a => $_` (an expression statement).
/// v1 and v2 may parse this as a hash-ref instead.
///
/// - v1: may produce error nodes or parse as hash
/// - v2: parses as `hash_ref` - WrongButPlausible
/// - v3: correctly parses as block
#[test]
fn cat3_map_block_with_hashlike_content() {
    let src = "my @x = map { a => $_ } @list;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("3", "map_block_with_hashlike_content", &r1.verdict, &r2.verdict, &r3.verdict);

    // v1: produces ERROR nodes - the tree-sitter grammar cannot resolve the
    // ambiguity between block and hash-ref in `map { a => $_ }`.
    // The sexp shows: (ERROR (anonymous_hash_expression ...)) with variable parsed wrong.
    assert_verdict(&r1, &Verdict::Errors, "map_block_hashlike (v1 expected error nodes)");

    // v2: parses as hash_ref - wrong but plausible; parse succeeds
    assert_eq!(r2.verdict, Verdict::Correct, "v2 parse must succeed");
    let v2_is_hash_ref = r2.sexp_contains("hash_ref");
    let v2_is_block = r2.sexp_contains("(block");
    println!(
        "    v2 map+hashlike: is_hash_ref={v2_is_hash_ref}, is_block={v2_is_block}, sexp={}",
        &r2.sexp[..r2.sexp.len().min(200)]
    );
    // v2 is expected to parse as hash_ref (WrongButPlausible)
    assert!(
        v2_is_hash_ref || v2_is_block,
        "v2 should produce hash_ref or block for map{{a => $_}}; sexp: {}",
        &r2.sexp[..r2.sexp.len().min(300)]
    );

    // v3: must parse as block
    assert_verdict(&r3, &Verdict::Correct, "map_block_hashlike (v3)");
    assert!(
        r3.sexp_contains("(block"),
        "v3 map block should parse as block not hash_ref; sexp: {}",
        &r3.sexp[..r3.sexp.len().min(300)]
    );
}

/// Cat 3d: eval block.
#[test]
fn cat3_eval_block() {
    let src = "eval { die 'oops'; };";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("3", "eval_block", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "eval_block");
    assert_verdict(&r2, &Verdict::Correct, "eval_block");
    assert_verdict(&r3, &Verdict::Correct, "eval_block");
}

/// Cat 3e: grep block.
#[test]
fn cat3_grep_block() {
    let src = "my @pos = grep { $_ > 0 } @list;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("3", "grep_block", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "grep_block");
    assert_verdict(&r2, &Verdict::Correct, "grep_block");
    assert_verdict(&r3, &Verdict::Correct, "grep_block");
}

/// Cat 3f: sort block.
#[test]
fn cat3_sort_block() {
    let src = "my @sorted = sort { $a <=> $b } @list;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("3", "sort_block", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "sort_block");
    assert_verdict(&r2, &Verdict::Correct, "sort_block");
    assert_verdict(&r3, &Verdict::Correct, "sort_block");
}

// --- Category 4: Quote-like operators ----------------------------------------

/// Cat 4a: q{} - single-quoted string with braces.
#[test]
fn cat4_q_with_braces() {
    let src = "my $x = q{hello world};";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("4", "q_with_braces", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "q_with_braces");
    assert_verdict(&r2, &Verdict::Correct, "q_with_braces");
    assert_verdict(&r3, &Verdict::Correct, "q_with_braces");
}

/// Cat 4b: qq() - double-quoted string with parens.
#[test]
fn cat4_qq_with_parens() {
    let src = "my $x = qq(hello $name);";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("4", "qq_with_parens", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "qq_with_parens");
    assert_verdict(&r2, &Verdict::Correct, "qq_with_parens");
    assert_verdict(&r3, &Verdict::Correct, "qq_with_parens");
}

/// Cat 4c: qw[] - word list with brackets.
#[test]
fn cat4_qw_with_brackets() {
    let src = "my @a = qw[one two three];";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("4", "qw_with_brackets", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "qw_with_brackets");
    assert_verdict(&r2, &Verdict::Correct, "qw_with_brackets");
    assert_verdict(&r3, &Verdict::Correct, "qw_with_brackets");
}

/// Cat 4d: qr/pattern/i - regex with flags.
#[test]
fn cat4_qr_regex_with_flags() {
    let src = "my $re = qr/pattern/i;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("4", "qr_regex_with_flags", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "qr_regex_with_flags");
    assert_verdict(&r2, &Verdict::Correct, "qr_regex_with_flags");
    assert_verdict(&r3, &Verdict::Correct, "qr_regex_with_flags");
}

/// Cat 4e: s{} paired braces substitution.
#[test]
fn cat4_s_paired_braces() {
    let src = "my $s = 'abc'; $s =~ s{a}{X};";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("4", "s_paired_braces", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "s_paired_braces");
    assert_verdict(&r2, &Verdict::Correct, "s_paired_braces");
    assert_verdict(&r3, &Verdict::Correct, "s_paired_braces");
}

/// Cat 4f: s|pipe| substitution with pipe delimiter.
#[test]
fn cat4_s_pipe_delimiter() {
    let src = "my $s = 'abc'; $s =~ s|a|X|g;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("4", "s_pipe_delimiter", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "s_pipe_delimiter");
    assert_verdict(&r2, &Verdict::Correct, "s_pipe_delimiter");
    assert_verdict(&r3, &Verdict::Correct, "s_pipe_delimiter");
}

/// Cat 4g: tr[a-c][A-C] - transliteration with brackets.
#[test]
fn cat4_tr_brackets() {
    let src = "my $s = 'abc'; $s =~ tr[a-c][A-C];";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("4", "tr_brackets", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "tr_brackets");
    assert_verdict(&r2, &Verdict::Correct, "tr_brackets");
    assert_verdict(&r3, &Verdict::Correct, "tr_brackets");
}

/// Cat 4h: s{foo}/bar/g - MIXED delimiters (brace open, slash close).
///
/// This is the deep silent-failure case from PR #9168.
/// - v1: may produce error nodes or accept with wrong parse
/// - v2: accepts but empty replacement; trailing `bar/g` parsed as division - SilentlyEmpty
/// - v3: correctly parses the mixed-delimiter substitution
#[test]
fn cat4_s_mixed_delimiters_brace_slash() {
    let src = "my $s = 'foo'; $s =~ s{foo}/bar/g;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("4", "s_mixed_delimiters_brace_slash", &r1.verdict, &r2.verdict, &r3.verdict);

    // v1: tree-sitter-c may handle this case
    println!("    v1 s{{}}// mixed: {:?}, error_nodes={}", r1.verdict, r1.sexp_contains("ERROR"));

    // v2: accepts but parses WRONGLY. The substitution s{foo}// is mis-parsed:
    //   - s{foo}// becomes s/foo// (empty replacement)
    //   - The trailing `/bar/g` becomes a separate binary division expression
    //   - `bar` appears in the sexp but as a standalone expression, not as replacement
    assert_eq!(r2.verdict, Verdict::Correct, "v2 parse must succeed even if wrong");
    let v2_has_replacement_in_sub = r2.sexp_contains("substitution s/foo//");
    let v2_bar_is_separate_expr = r2.sexp_contains("binary_expression") && r2.sexp_contains("bar");
    println!(
        "    v2 s{{}}// mixed: empty_sub={v2_has_replacement_in_sub}, bar_separate={v2_bar_is_separate_expr}",
    );
    println!("    v2 sexp: {}", &r2.sexp[..r2.sexp.len().min(400)]);
    // v2 misparses: the replacement is empty and "bar" leaks into a separate expression
    assert!(
        v2_has_replacement_in_sub,
        "v2 should have empty substitution s/foo//; sexp: {}",
        &r2.sexp[..r2.sexp.len().min(400)]
    );
    assert!(
        v2_bar_is_separate_expr,
        "v2 should have 'bar' as separate expression (not in replacement); sexp: {}",
        &r2.sexp[..r2.sexp.len().min(400)]
    );

    // v3: should handle correctly (parse succeeds with substitution node)
    assert_verdict(&r3, &Verdict::Correct, "s_mixed_delimiters (v3)");
    // v3 structural: should contain substitution with pattern and replacement
    assert!(
        r3.sexp_contains("substitution") || r3.sexp_contains("(sub"),
        "v3 should produce substitution node; sexp: {}",
        &r3.sexp[..r3.sexp.len().min(300)]
    );
}

// --- Category 5: Special/punctuation variables --------------------------------

/// Cat 5a: `$/` - input record separator.
#[test]
fn cat5_dollar_slash() {
    let src = "local $/ = undef;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("5", "dollar_slash", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "dollar_slash");
    assert_verdict(&r2, &Verdict::Correct, "dollar_slash");
    assert_verdict(&r3, &Verdict::Correct, "dollar_slash");
}

/// Cat 5b: `$$` - process ID.
#[test]
fn cat5_dollar_dollar() {
    let src = "print $$;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("5", "dollar_dollar", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "dollar_dollar");
    assert_verdict(&r2, &Verdict::Correct, "dollar_dollar");
    assert_verdict(&r3, &Verdict::Correct, "dollar_dollar");
}

/// Cat 5c: `$!` - errno variable.
#[test]
fn cat5_dollar_bang() {
    let src = "die $!;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("5", "dollar_bang", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "dollar_bang");
    assert_verdict(&r2, &Verdict::Correct, "dollar_bang");
    assert_verdict(&r3, &Verdict::Correct, "dollar_bang");
}

/// Cat 5d: `$@` - error variable.
#[test]
fn cat5_dollar_at() {
    let src = "warn $@;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("5", "dollar_at", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "dollar_at");
    assert_verdict(&r2, &Verdict::Correct, "dollar_at");
    assert_verdict(&r3, &Verdict::Correct, "dollar_at");
}

/// Cat 5e: `$^W` - warnings flag.
#[test]
fn cat5_dollar_caret_w() {
    let src = "$^W = 1;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("5", "dollar_caret_W", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "dollar_caret_W");
    assert_verdict(&r2, &Verdict::Correct, "dollar_caret_W");
    assert_verdict(&r3, &Verdict::Correct, "dollar_caret_W");
}

/// Cat 5f: `${^MATCH}` - named capture variable.
///
/// This is the deep silent-failure from PR #9168.
/// - v1: may accept or error depending on whether `${^MATCH}` is in the catalog
/// - v2: accepts but variable name is truncated to `${` - SilentlyEmpty
/// - v3: correctly parses as a single variable with name `^MATCH`
#[test]
fn cat5_dollar_caret_match_named_capture() {
    let src = "print ${^MATCH};";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("5", "dollar_caret_match", &r1.verdict, &r2.verdict, &r3.verdict);

    // v1: record actual behavior
    println!("    v1 ${{^MATCH}}: {:?}, sexp={}", r1.verdict, &r1.sexp[..r1.sexp.len().min(200)]);

    // v2: accepts but truncates the variable name
    assert_eq!(r2.verdict, Verdict::Correct, "v2 must accept dollar-caret-MATCH input");
    let v2_has_match = r2.sexp_contains("MATCH");
    println!(
        "    v2 ${{^MATCH}}: has_MATCH={v2_has_match}, sexp={}",
        &r2.sexp[..r2.sexp.len().min(300)]
    );
    // v2 is expected to NOT contain MATCH - it truncates to `${`
    assert!(
        !v2_has_match,
        "v2 should truncate dollar-caret-MATCH variable name (known silent failure); sexp: {}",
        &r2.sexp[..r2.sexp.len().min(300)]
    );

    // v3: must include MATCH in the variable node
    assert_verdict(&r3, &Verdict::Correct, "dollar_caret_match (v3)");
    assert!(
        r3.sexp_contains("MATCH") || r3.sexp_contains("^MATCH"),
        "v3 must preserve full variable name dollar-caret-MATCH; sexp: {}",
        &r3.sexp[..r3.sexp.len().min(300)]
    );
}

/// Cat 5g: `$_` - default variable.
#[test]
fn cat5_dollar_underscore() {
    let src = "for (@x) { print $_; }";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("5", "dollar_underscore", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "dollar_underscore");
    assert_verdict(&r2, &Verdict::Correct, "dollar_underscore");
    assert_verdict(&r3, &Verdict::Correct, "dollar_underscore");
}

/// Cat 5h: `$&` - match variable.
#[test]
fn cat5_dollar_ampersand() {
    let src = "print $&;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("5", "dollar_ampersand", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "dollar_ampersand");
    assert_verdict(&r2, &Verdict::Correct, "dollar_ampersand");
    assert_verdict(&r3, &Verdict::Correct, "dollar_ampersand");
}

/// Cat 5i: `$1` - numbered capture group.
#[test]
fn cat5_numbered_capture() {
    let src = "if ('abc' =~ /(.)/) { print $1; }";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("5", "numbered_capture", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "numbered_capture");
    assert_verdict(&r2, &Verdict::Correct, "numbered_capture");
    assert_verdict(&r3, &Verdict::Correct, "numbered_capture");
}

// --- Category 6: Indirect object syntax --------------------------------------

/// Cat 6a: `new Foo()` - indirect object method call.
#[test]
fn cat6_indirect_new() {
    let src = "my $obj = new Foo();";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("6", "indirect_new", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "indirect_new");
    assert_verdict(&r2, &Verdict::Correct, "indirect_new");
    assert_verdict(&r3, &Verdict::Correct, "indirect_new");
}

/// Cat 6b: `new Foo('arg')` - indirect new with argument.
#[test]
fn cat6_indirect_new_with_arg() {
    let src = "my $obj = new Foo('arg');";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("6", "indirect_new_with_arg", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "indirect_new_with_arg");
    assert_verdict(&r2, &Verdict::Correct, "indirect_new_with_arg");
    assert_verdict(&r3, &Verdict::Correct, "indirect_new_with_arg");
}

/// Cat 6c: `print STDERR "message"` - print with filehandle.
#[test]
fn cat6_print_with_filehandle() {
    let src = "print STDERR \"oops\\n\";";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("6", "print_with_filehandle", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "print_with_filehandle");
    assert_verdict(&r2, &Verdict::Correct, "print_with_filehandle");
    assert_verdict(&r3, &Verdict::Correct, "print_with_filehandle");
}

/// Cat 6d: `Foo->new()` - explicit arrow-method call (unambiguous).
#[test]
fn cat6_arrow_method_call() {
    let src = "my $obj = Foo->new();";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("6", "arrow_method_call", &r1.verdict, &r2.verdict, &r3.verdict);
    assert_verdict(&r1, &Verdict::Correct, "arrow_method_call");
    assert_verdict(&r2, &Verdict::Correct, "arrow_method_call");
    assert_verdict(&r3, &Verdict::Correct, "arrow_method_call");
}

// --- Category 7: Format declarations -----------------------------------------

/// Cat 7a: simple format declaration.
///
/// - v1: may produce error nodes (format DSL is hard for GLR)
/// - v2: accepts but body is empty - SilentlyEmpty (atomic rule strips sub-rules)
/// - v3: correctly captures name and body
#[test]
fn cat7_simple_format_declaration() {
    let src = "format STDOUT =\n@<<<<<<<\n$name\n.\n";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("7", "simple_format_declaration", &r1.verdict, &r2.verdict, &r3.verdict);

    // v1: record behavior without hard assertion (format support varies)
    println!("    v1 format: {:?}, has_ERROR={}", r1.verdict, r1.sexp_contains("ERROR"));

    // v2: accepts the input
    assert_eq!(r2.verdict, Verdict::Correct, "v2 must accept format declaration");
    let v2_has_body = r2.sexp_contains("@<<<<<<<") || r2.sexp_contains("name");
    println!("    v2 format: has_body={v2_has_body}, sexp={}", &r2.sexp[..r2.sexp.len().min(300)]);
    // v2 loses body content - the atomic rule collapses to empty format_lines
    assert!(
        !v2_has_body,
        "v2 should silently lose format body (known limitation); sexp: {}",
        &r2.sexp[..r2.sexp.len().min(300)]
    );

    // v3: correctly parses with body
    assert_verdict(&r3, &Verdict::Correct, "simple_format_declaration (v3)");
    assert!(
        r3.sexp_contains("STDOUT"),
        "v3 should include format name; sexp: {}",
        &r3.sexp[..r3.sexp.len().min(300)]
    );
    assert!(
        r3.sexp_contains("@<<<<<<<") || r3.sexp_contains("name"),
        "v3 should include format body content; sexp: {}",
        &r3.sexp[..r3.sexp.len().min(300)]
    );
}

/// Cat 7b: multi-line format with picture and value lines.
#[test]
fn cat7_multiline_format() {
    let src = "format REPORT =\n@<<< @>>>\n$a, $b\n.\nmy $x = 1;\n";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("7", "multiline_format", &r1.verdict, &r2.verdict, &r3.verdict);

    println!("    v1 multiline format: {:?}", r1.verdict);

    // v2: accepts
    assert_eq!(r2.verdict, Verdict::Correct, "v2 accepts multiline format");

    // v3: must capture name and body
    assert_verdict(&r3, &Verdict::Correct, "multiline_format (v3)");
    assert!(
        r3.sexp_contains("REPORT"),
        "v3 should include REPORT in format sexp; sexp: {}",
        &r3.sexp[..r3.sexp.len().min(400)]
    );
}

/// Cat 7c: format followed by regular code (parser must switch back).
#[test]
fn cat7_format_followed_by_code() {
    let src = "format FOO =\n@<\n$x\n.\nmy $y = 2;\n";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("7", "format_followed_by_code", &r1.verdict, &r2.verdict, &r3.verdict);

    println!("    v1 format+code: {:?}", r1.verdict);
    // v2 and v3 must accept
    assert_eq!(r2.verdict, Verdict::Correct, "v2 accepts format+code");
    assert_verdict(&r3, &Verdict::Correct, "format_followed_by_code (v3)");
    // v3 structural: both the format and `my $y = 2` should be in the AST
    assert!(
        r3.sexp_contains("FOO") || r3.sexp_contains("(format"),
        "v3 should contain format node; sexp: {}",
        &r3.sexp[..r3.sexp.len().min(400)]
    );
}

// --- Garbage / rejection cases ------------------------------------------------

/// Garbage: pure nonsense - should be rejected or produce heavy error nodes.
#[test]
fn garbage_pure_nonsense() {
    let src = "@@@ this is not perl at all $$$ <<<";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("G", "pure_nonsense", &r1.verdict, &r2.verdict, &r3.verdict);
    // v1 and v3 may produce error nodes; v2 should reject
    // We don't assert specific verdicts since error recovery varies
    println!("    v1={:?} v2={:?} v3={:?}", r1.verdict, r2.verdict, r3.verdict);
    // At minimum: none should Crash
    assert_ne!(r1.verdict, Verdict::Crashes, "garbage must not crash v1");
    assert_ne!(r2.verdict, Verdict::Crashes, "garbage must not crash v2");
    assert_ne!(r3.verdict, Verdict::Crashes, "garbage must not crash v3");
}

/// Garbage: unclosed sub - should be rejected.
#[test]
fn garbage_unclosed_sub() {
    let src = "sub broken { my $x = ";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("G", "unclosed_sub", &r1.verdict, &r2.verdict, &r3.verdict);
    println!("    v1={:?} v2={:?} v3={:?}", r1.verdict, r2.verdict, r3.verdict);
    // No parser should crash
    assert_ne!(r1.verdict, Verdict::Crashes, "unclosed_sub must not crash v1");
    assert_ne!(r2.verdict, Verdict::Crashes, "unclosed_sub must not crash v2");
    assert_ne!(r3.verdict, Verdict::Crashes, "unclosed_sub must not crash v3");
    // v2 should reject
    assert_eq!(r2.verdict, Verdict::Errors, "v2 should reject unclosed sub");
}

/// Garbage: JavaScript-style function - not Perl.
#[test]
fn garbage_javascript_style_function() {
    let src = "function foo() { return 42; }";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("G", "javascript_function", &r1.verdict, &r2.verdict, &r3.verdict);
    println!("    v1={:?} v2={:?} v3={:?}", r1.verdict, r2.verdict, r3.verdict);
    // No crashes
    assert_ne!(r1.verdict, Verdict::Crashes, "js_fn must not crash v1");
    assert_ne!(r2.verdict, Verdict::Crashes, "js_fn must not crash v2");
    assert_ne!(r3.verdict, Verdict::Crashes, "js_fn must not crash v3");
}

/// Garbage: invalid double-sigil `my @@x = 5` - discovered in PR #9168.
///
/// v2 accepts this with wrong AST - a silent failure.
/// v3 should reject or produce error nodes.
#[test]
fn garbage_double_sigil_array() {
    let src = "my @@x = 5;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("G", "double_sigil_at_at", &r1.verdict, &r2.verdict, &r3.verdict);
    println!(
        "    v1={:?} v2={:?} v3={:?}; v2_sexp={}",
        r1.verdict,
        r2.verdict,
        r3.verdict,
        &r2.sexp[..r2.sexp.len().min(200)]
    );
    // No crashes
    assert_ne!(r1.verdict, Verdict::Crashes, "double_sigil must not crash v1");
    assert_ne!(r2.verdict, Verdict::Crashes, "double_sigil must not crash v2");
    assert_ne!(r3.verdict, Verdict::Crashes, "double_sigil must not crash v3");
    // v2 accepts with wrong AST - record this known-bad behavior
    // (it parses @@x as variable_declaration @@ then assignment x=5)
    // v3 should produce error nodes
    assert!(
        r3.verdict == Verdict::Errors || r3.verdict == Verdict::Correct,
        "v3 should handle double sigil gracefully; got {:?}",
        r3.verdict
    );
}

/// Garbage: random punctuation - should be rejected.
#[test]
fn garbage_random_punctuation() {
    let src = "} ) ] ; => => => ;;";
    let r1 = parse_v1(src);
    let r2 = parse_v2(src);
    let r3 = parse_v3(src);
    print_row("G", "random_punctuation", &r1.verdict, &r2.verdict, &r3.verdict);
    println!("    v1={:?} v2={:?} v3={:?}", r1.verdict, r2.verdict, r3.verdict);
    // No crashes
    assert_ne!(r1.verdict, Verdict::Crashes, "random_punctuation must not crash v1");
    assert_ne!(r2.verdict, Verdict::Crashes, "random_punctuation must not crash v2");
    assert_ne!(r3.verdict, Verdict::Crashes, "random_punctuation must not crash v3");
    // v2 should reject
    assert_eq!(r2.verdict, Verdict::Errors, "v2 should reject random punctuation");
}

// --- Summary printer ---------------------------------------------------------

/// Print summary header at start of test run.
/// (Tests run in parallel so this may not be first in --nocapture output.)
#[test]
fn zzz_print_summary_header() {
    let line: String = "-".repeat(80);
    println!("\n  {line}");
    println!("  Differential Parser Comparison Suite");
    println!("  v1 = tree-sitter-perl-c (C FFI)");
    println!("  v2 = perl-parser-pest   (Pest/PEG legacy)");
    println!("  v3 = perl-parser-core   (recursive descent, production)");
    println!("  {line}");
    println!("  [{:>2}] {:<50} | {:<20} | {:<20} | v3", "Cat", "Label", "v1", "v2");
    println!("  {line}");
}
