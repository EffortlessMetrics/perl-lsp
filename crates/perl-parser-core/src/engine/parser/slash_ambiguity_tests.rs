#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use crate::parser::Parser;
    use perl_tdd_support::must;

    // ───────────────────────────────────────────────────────────────────
    // Division cases: `/` should be treated as division operator
    // ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_variable_division() {
        // $x / $y  ->  division
        let code = "my $res = $x / $y;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("binary_/"), "Should be division: {sexp}");
        assert!(!sexp.contains("regex"), "Should not be regex: {sexp}");
    }

    #[test]
    fn test_division_after_paren() {
        // ($sum) / $count  ->  division after closing paren
        let code = "my $avg = ($sum) / $count;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("binary_/"), "Should be division after paren: {sexp}");
        assert!(!sexp.contains("regex"), "Should not be regex: {sexp}");
    }

    #[test]
    fn test_division_in_condition() {
        // if ($x / 2 > 0) { ... }
        let code = "if ($x / 2 > 0) { 1; }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("binary_/"), "Should be division in condition: {sexp}");
    }

    #[test]
    fn test_division_after_hash_deref() {
        // $hash{key} / 10  ->  division after hash access
        let code = "$hash{key} / 10;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("binary_/"), "Should be division after hash deref: {sexp}");
    }

    #[test]
    fn test_division_assign() {
        // $x /= 2  ->  division-assign compound operator
        let code = "$x /= 2;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("assignment_/assign"), "Should be division-assign: {sexp}");
    }

    #[test]
    fn test_chained_division() {
        // $x / $y / $z  ->  two divisions
        let code = "$x / $y / $z;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.matches("binary_/").count() >= 2, "Should be two divisions: {sexp}");
        assert!(!sexp.contains("regex"), "Should not be regex: {sexp}");
    }

    #[test]
    fn test_division_after_number() {
        // 10 / 3  ->  division
        let code = "my $x = 10 / 3;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("binary_/"), "Should be division after number: {sexp}");
    }

    #[test]
    fn test_division_after_closing_bracket() {
        // $arr[0] / 2  ->  division after array access
        let code = "$arr[0] / 2;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("binary_/"), "Should be division after closing bracket: {sexp}");
    }

    // ───────────────────────────────────────────────────────────────────
    // Regex cases: `/` should be treated as regex delimiter
    // ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_regex_after_if() {
        // if (/pattern/) { 1 }  ->  regex
        let code = "if (/pattern/) { 1; }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("regex"), "Should be regex after if: {sexp}");
    }

    #[test]
    fn test_regex_in_grep() {
        // grep /regex/, @list  ->  regex
        let code = "grep /abc/, @list;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("regex"), "Should be regex in grep: {sexp}");
    }

    #[test]
    fn test_regex_in_binding() {
        // $str =~ /foo/  ->  regex in binding
        let code = "$str =~ /foo/;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("regex") || sexp.contains("match"), "Should be regex match: {sexp}");
    }

    #[test]
    fn test_regex_in_void_context() {
        // /pattern/  at statement start  ->  regex
        let code = "/pattern/;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("regex"), "Should be regex in void context: {sexp}");
    }

    #[test]
    fn test_regex_with_interpolation() {
        // /$x/  ->  regex with interpolated variable
        let code = "/$x/;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("regex"), "Should be regex with interpolation: {sexp}");
    }

    #[test]
    fn test_regex_in_split() {
        // split /,/, $str  ->  regex in split
        let code = "split /,/, $str;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("regex"), "Should be regex in split: {sexp}");
    }

    #[test]
    fn test_regex_in_map() {
        // map /pattern/, @list  ->  regex in map (less common but valid Perl)
        let code = "map /abc/, @list;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("regex"), "Should be regex in map: {sexp}");
    }

    #[test]
    fn test_regex_after_while() {
        // while (/pattern/) { 1 }  ->  regex
        let code = "while (/abc/) { 1; }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("regex"), "Should be regex after while: {sexp}");
    }

    #[test]
    fn test_regex_after_unless() {
        // unless (/pattern/) { 1 }  ->  regex
        let code = "unless (/abc/) { 1; }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("regex"), "Should be regex after unless: {sexp}");
    }

    // ───────────────────────────────────────────────────────────────────
    // Mixed / edge cases
    // ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_division_after_function_call() {
        // time / 60  ->  division (time is a nullary builtin)
        let code = "time / 60;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("binary_/"), "Should be division after nullary function: {sexp}");
    }

    #[test]
    fn test_defined_or() {
        // $x // $y  ->  defined-or, not regex
        let code = "my $val = $x // $y;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("binary_//"), "Should be defined-or: {sexp}");
    }

    #[test]
    fn test_defined_or_assign() {
        // $x //= $y  ->  defined-or-assign
        let code = "$x //= $y;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("assignment_//assign"), "Should be defined-or-assign: {sexp}");
    }

    #[test]
    fn test_complex_expression_with_division_and_regex() {
        // Mix of division and regex in the same program
        let code = "my $avg = $sum / $count; if (/done/) { 1; }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("binary_/"), "Should contain division: {sexp}");
        assert!(sexp.contains("regex"), "Should also contain regex: {sexp}");
    }

    #[test]
    fn test_regex_after_comma() {
        // Regex after comma (in list context)
        let code = "my @r = (1, /abc/);";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("regex"), "Should be regex after comma: {sexp}");
    }

    #[test]
    fn test_regex_after_open_paren() {
        // (/pattern/)  ->  regex inside parens
        let code = "(/pattern/);";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("regex"), "Should be regex after open paren: {sexp}");
    }

    #[test]
    fn test_addition_then_division() {
        // ($x + $y) / 2  ->  division
        let code = "my $avg = ($x + $y) / 2;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(sexp.contains("binary_/"), "Should be division: {sexp}");
        assert!(sexp.contains("binary_+"), "Should have addition: {sexp}");
    }
}
