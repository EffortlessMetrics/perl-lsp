//! Tests for grep/map/sort with block form followed by a trailing list.
//!
//! In Perl, `grep { BLOCK } LIST`, `map { BLOCK } LIST`, and
//! `sort { BLOCK } LIST` do not require a comma between the block
//! and the list.  These tests verify the parser handles this correctly
//! in both statement and expression (assignment RHS) contexts.

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use crate::parser::Parser;
    use perl_tdd_support::must;

    // ---- grep ----

    #[test]
    fn grep_block_simple_comparison() {
        let code = "grep { $_ > 5 } @array;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("(call grep"),
            "should be a grep call: {}",
            sexp
        );
        assert!(sexp.contains("(block"), "should contain a block: {}", sexp);
        assert!(
            sexp.contains("(variable @ array)"),
            "trailing list should be inside call: {}",
            sexp
        );
    }

    #[test]
    fn grep_block_method_call_in_block() {
        let code = "grep { $_->is_valid } @items;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("(variable @ items)"),
            "trailing list should be inside call: {}",
            sexp
        );
    }

    #[test]
    fn grep_block_regex_in_block() {
        let code = r#"grep { /pattern/ } @strings;"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("(variable @ strings)"),
            "trailing list should be inside call: {}",
            sexp
        );
    }

    #[test]
    fn grep_block_assigned_to_variable() {
        let code = "my @result = grep { $_->is_valid } @items;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "should not contain ERROR: {}",
            sexp
        );
        // The trailing list must be inside the grep call, not a separate statement
        assert!(
            sexp.contains("(call grep") && sexp.contains("(variable @ items)"),
            "trailing list should be inside the grep call: {}",
            sexp
        );
    }

    // ---- map ----

    #[test]
    fn map_block_with_range() {
        let code = "map { $_ * 2 } 1..10;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("(call map"), "should be a map call: {}", sexp);
        assert!(
            sexp.contains(".."),
            "trailing range should be inside call: {}",
            sexp
        );
    }

    #[test]
    fn map_block_array_subscript_in_block() {
        let code = r#"map { $_->[0] + $_->[1] } @pairs;"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("(variable @ pairs)"),
            "trailing list should be inside call: {}",
            sexp
        );
    }

    #[test]
    fn map_block_assigned_to_variable() {
        let code = r#"my @mapped = map { $_->[0] + $_->[1] } @pairs;"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "should not contain ERROR: {}",
            sexp
        );
        assert!(
            sexp.contains("(call map") && sexp.contains("(variable @ pairs)"),
            "trailing list should be inside the map call: {}",
            sexp
        );
    }

    // ---- sort ----

    #[test]
    fn sort_block_numeric_comparison() {
        let code = "sort { $a <=> $b } @numbers;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("(call sort"),
            "should be a sort call: {}",
            sexp
        );
        assert!(
            sexp.contains("(variable @ numbers)"),
            "trailing list should be inside call: {}",
            sexp
        );
    }

    #[test]
    fn sort_block_hash_access_in_block() {
        let code = r#"sort { $a->{name} cmp $b->{name} } @records;"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("(variable @ records)"),
            "trailing list should be inside call: {}",
            sexp
        );
    }

    #[test]
    fn sort_block_function_call_in_block() {
        let code = r#"sort { length($a) <=> length($b) } keys %hash;"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("(call sort"),
            "should be a sort call: {}",
            sexp
        );
        // keys %hash should be inside the sort call
        assert!(
            sexp.contains("(call keys"),
            "keys should be inside sort call: {}",
            sexp
        );
    }

    #[test]
    fn sort_block_assigned_to_variable() {
        let code = r#"my @sorted = sort { $a->{name} cmp $b->{name} } @records;"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "should not contain ERROR: {}",
            sexp
        );
        assert!(
            sexp.contains("(call sort") && sexp.contains("(variable @ records)"),
            "trailing list should be inside the sort call: {}",
            sexp
        );
    }

    #[test]
    fn sort_block_with_keys_hash_assigned() {
        let code = r#"my @x = sort { length($a) <=> length($b) } keys %hash;"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "should not contain ERROR: {}",
            sexp
        );
        assert!(
            sexp.contains("(call sort") && sexp.contains("(call keys"),
            "keys call should be inside the sort call: {}",
            sexp
        );
    }

    // ---- chained builtins ----

    #[test]
    fn chained_grep_map() {
        let code = r#"grep { /pattern/ } map { lc $_ } @strings;"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "should not contain ERROR: {}",
            sexp
        );
        assert!(
            sexp.contains("(call grep"),
            "outer should be grep: {}",
            sexp
        );
        assert!(sexp.contains("(call map"), "inner should be map: {}", sexp);
    }

    #[test]
    fn chained_sort_map() {
        let code = r#"my @result = sort { $a cmp $b } map { lc } @strings;"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "should not contain ERROR: {}",
            sexp
        );
        assert!(
            sexp.contains("(call sort"),
            "outer should be sort: {}",
            sexp
        );
        assert!(sexp.contains("(call map"), "inner should be map: {}", sexp);
    }

    // ---- edge cases ----

    #[test]
    fn grep_block_with_comma_before_list() {
        // Perl also allows a comma: grep { ... }, @array
        let code = "grep { $_ > 5 }, @array;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            sexp.contains("(call grep"),
            "should be a grep call: {}",
            sexp
        );
        assert!(
            sexp.contains("(variable @ array)"),
            "trailing list with comma should work: {}",
            sexp
        );
    }

    #[test]
    fn sort_block_empty_block() {
        // sort {} @array -- empty block comparison
        let code = "sort {} @array;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "should not contain ERROR: {}",
            sexp
        );
    }
}
