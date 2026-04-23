//! Tests for block-taking builtins (map, grep, sort) inside for-loop iterator lists
//! and with multi-statement blocks.
//!
//! In Perl, `for my $x (map { BLOCK } LIST)` is valid -- the builtins map, grep,
//! and sort can appear inside the parenthesized iterator list of a for/foreach loop.
//! The parser must recognize `{ ... }` as a builtin block (not a hash reference)
//! in this context.
//!
//! Additionally, builtin blocks may contain multiple statements separated by
//! semicolons, e.g. `map { my $y = uc $_; $y } @list`.

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use crate::parser::Parser;
    use perl_tdd_support::must;

    // ---- map block in for iterator ----

    #[test]
    fn map_block_in_for_iterator_with_variable() {
        let code = "for my $x (map { uc } @list) { }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call map"), "should contain a map call: {}", sexp);
        assert!(sexp.contains("foreach"), "should be a foreach loop: {}", sexp);
        assert!(sexp.contains("(variable @ list)"), "map call should include @list: {}", sexp);
    }

    // ---- grep block in for iterator ----

    #[test]
    fn grep_block_in_for_iterator_with_variable() {
        let code = "for my $x (grep { defined } @values) { }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call grep"), "should contain a grep call: {}", sexp);
        assert!(sexp.contains("foreach"), "should be a foreach loop: {}", sexp);
    }

    // ---- sort block in for iterator ----

    #[test]
    fn sort_block_in_for_iterator_with_variable() {
        let code = "for my $x (sort { $a cmp $b } @list) { }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call sort"), "should contain a sort call: {}", sexp);
        assert!(sexp.contains("foreach"), "should be a foreach loop: {}", sexp);
    }

    // ---- for without variable (implicit $_) ----

    #[test]
    fn map_block_in_for_iterator_without_variable() {
        let code = "for (map { $_ * 2 } 1..10) { }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call map"), "should contain a map call: {}", sexp);
        assert!(sexp.contains(".."), "map call should include range: {}", sexp);
    }

    // ---- map block with split inside (bonus) ----

    #[test]
    fn map_block_with_split_inside() {
        let code = r#"my @r = map { split /,/ } @lines;"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call map"), "should contain a map call: {}", sexp);
        assert!(
            sexp.contains("(variable @ lines)"),
            "trailing list should be inside map call: {}",
            sexp
        );
    }

    // ---- foreach keyword variant ----

    #[test]
    fn map_block_in_foreach_iterator() {
        let code = "foreach my $x (map { uc } @list) { }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call map"), "should contain a map call: {}", sexp);
    }

    // ---- nested builtins in for ----

    #[test]
    fn nested_map_grep_in_for_iterator() {
        let code = "for my $x (map { uc } grep { defined } @list) { }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call map"), "should contain outer map call: {}", sexp);
        assert!(sexp.contains("(call grep"), "should contain inner grep call: {}", sexp);
    }

    #[test]
    fn sort_with_complex_block_in_for_iterator() {
        let code = "for my $x (sort { lc($a) cmp lc($b) } @words) { }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call sort"), "should contain sort call: {}", sexp);
        assert!(sexp.contains("(variable @ words)"), "should include @words in sort: {}", sexp);
    }

    #[test]
    fn map_with_multiple_list_args_in_for() {
        let code = "for my $x (map { $_ + 1 } @a, @b) { }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call map"), "should contain map call: {}", sexp);
    }

    #[test]
    fn grep_block_with_body_in_for() {
        let code = "for my $x (grep { $_ > 0 } @nums) { print $x; }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
    }

    // ---- multi-statement blocks ----

    #[test]
    fn map_multi_statement_block_in_for_iterator() {
        let code = "for my $x (map { my $y = uc $_; $y } @list) { }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call map"), "should contain map call: {}", sexp);
    }

    #[test]
    fn map_multi_statement_block_standalone() {
        let code = "my @r = map { my $y = uc $_; $y } @list;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call map"), "should contain map call: {}", sexp);
    }

    #[test]
    fn grep_multi_statement_block_standalone() {
        let code = "my @r = grep { my $v = $_; defined $v } @list;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call grep"), "should contain grep call: {}", sexp);
    }

    #[test]
    fn sort_multi_statement_block_standalone() {
        let code = "my @r = sort { my $x = lc $a; my $y = lc $b; $x cmp $y } @list;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call sort"), "should contain sort call: {}", sexp);
    }

    #[test]
    fn grep_with_negation_in_for_iterator() {
        let code = "for my $x (grep { !ref($_) } @mixed) { }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call grep"), "should contain grep call: {}", sexp);
    }

    #[test]
    fn grep_defined_with_arg_in_block() {
        let code = "grep { defined $_ } @list;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
        assert!(sexp.contains("(call defined"), "block should contain defined call: {}", sexp);
    }

    #[test]
    fn for_with_only_map_on_empty_list() {
        let code = "for my $x (map { uc } ()) { }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "should not contain ERROR: {}", sexp);
    }
}
