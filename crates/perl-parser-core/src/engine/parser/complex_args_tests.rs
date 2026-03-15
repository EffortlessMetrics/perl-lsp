//! Tests for complex list and hash construction expressions used as function
//! arguments.  Covers anonymous hash refs `{}`, array refs `[]`, and nested
//! constructors inside argument positions.

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use crate::parser::Parser;
    use perl_tdd_support::must;

    /// Helper: parse code, assert no ERROR nodes.
    fn parse_ok(code: &str) -> String {
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "parse should succeed without errors for: {}\ngot: {}",
            code,
            sexp
        );
        sexp
    }

    /// Debug dump: parse code and print sexp (for manual analysis)
    #[test]
    fn debug_dump_all_cases() {
        let cases = [
            "foo({key => $val}, @rest);",
            "foo([$a, $b], {c => $d});",
            "my %h = (key => [1, 2, 3], other => {a => 1});",
            "push @a, map { $_ * 2 } @b;",
            r"return (key => $val, other => \@arr);",
            "foo({}, []);",
            "foo({a => 1, b => 2});",
            "my @x = ({a => 1}, {b => 2});",
            "push @arr, {key => $val};",
            "die {message => 'oops'};",
            "return {key => $val};",
            "foo({a => {b => [1, 2, {c => 3}]}});",
            "foo(bar({a => 1}), baz([$x]));",
            // Builtin functions without parens that take hash/array ref args
            "push @arr, {key => $val}, $extra;",
            "unshift @arr, [$a, $b];",
            // Hash ref after comma in builtin
            "warn {err => 1}, 'extra';",
            // Complex: block-form builtins inside argument lists
            "push @a, map { $_ * 2 } @b;",
            "push @a, sort { $b <=> $a } @b;",
            "push @a, grep { $_ > 0 } @b;",
            // Hash ref as value in larger expression
            "my $x = foo({a => 1}) || bar({b => 2});",
            "my $x = $cond ? {a => 1} : {b => 2};",
            // Array slice / hash slice in args
            "foo(@arr[0..2], {key => $val});",
            // Anonymous sub in args (should work)
            "foo(sub { 1 }, {key => $val});",
            // Hash ref in list assignment
            "my ($a, $b) = ({x => 1}, [2, 3]);",
            // Chained method with hash ref
            "$obj->method({a => 1})->other({b => 2});",
        ];
        for code in &cases {
            let mut parser = Parser::new(code);
            let result = parser.parse();
            match result {
                Ok(ast) => {
                    let sexp = ast.to_sexp();
                    let status = if sexp.contains("ERROR") { "ERRORS" } else { "OK" };
                    eprintln!("[{}] {} => {}", status, code, sexp);
                }
                Err(e) => {
                    eprintln!("[FATAL] {} => {:?}", code, e);
                }
            }
        }
    }

    // ----- hash ref as function argument -----

    #[test]
    fn hash_ref_as_first_arg() {
        // foo({key => $val}, @rest)
        let sexp = parse_ok("foo({key => $val}, @rest);");
        assert!(
            sexp.contains("function_call_expression") || sexp.contains("call"),
            "should be a function call: {}",
            sexp
        );
        assert!(sexp.contains("hash"), "should contain a hash ref: {}", sexp);
        assert!(sexp.contains("(variable @ rest)"), "should contain @rest arg: {}", sexp);
    }

    #[test]
    fn mixed_ref_types_as_args() {
        // foo([$a, $b], {c => $d})
        let sexp = parse_ok("foo([$a, $b], {c => $d});");
        assert!(
            sexp.contains("function_call_expression") || sexp.contains("call"),
            "should be a function call: {}",
            sexp
        );
        assert!(sexp.contains("(array"), "should contain array ref: {}", sexp);
        assert!(sexp.contains("hash"), "should contain hash ref: {}", sexp);
    }

    #[test]
    fn nested_constructors_in_hash_init() {
        // my %h = (key => [1, 2, 3], other => {a => 1})
        let sexp = parse_ok("my %h = (key => [1, 2, 3], other => {a => 1});");
        assert!(sexp.contains("(array"), "should contain array literal: {}", sexp);
        assert!(sexp.contains("hash"), "should contain hash ref: {}", sexp);
    }

    #[test]
    fn block_builtin_as_argument() {
        // push @a, map { $_ * 2 } @b
        let sexp = parse_ok("push @a, map { $_ * 2 } @b;");
        assert!(sexp.contains("(call push"), "should be a push call: {}", sexp);
        assert!(sexp.contains("(call map"), "should contain map call: {}", sexp);
        assert!(sexp.contains("(variable @ b)"), "should contain @b inside map: {}", sexp);
    }

    #[test]
    fn return_with_hash_like_list() {
        // return (key => $val, other => \@arr)
        let sexp = parse_ok(r"return (key => $val, other => \@arr);");
        assert!(sexp.contains("(return"), "should be a return: {}", sexp);
    }

    // ----- additional edge cases -----

    #[test]
    fn empty_hash_ref_and_array_ref_as_args() {
        // foo({}, [])
        let sexp = parse_ok("foo({}, []);");
        assert!(
            sexp.contains("function_call_expression") || sexp.contains("call"),
            "should be a function call: {}",
            sexp
        );
    }

    #[test]
    fn hash_ref_with_multiple_pairs_as_arg() {
        // foo({a => 1, b => 2})
        let sexp = parse_ok("foo({a => 1, b => 2});");
        assert!(
            sexp.contains("function_call_expression") || sexp.contains("call"),
            "should be a function call: {}",
            sexp
        );
        assert!(sexp.contains("hash"), "should contain hash: {}", sexp);
    }

    #[test]
    fn array_of_hash_refs() {
        // my @x = ({a => 1}, {b => 2})
        let sexp = parse_ok("my @x = ({a => 1}, {b => 2});");
        assert!(sexp.contains("hash"), "should contain hash refs: {}", sexp);
    }

    #[test]
    fn hash_ref_arg_to_method_call() {
        // $obj->method({key => $val})
        let sexp = parse_ok(r#"$obj->method({key => $val});"#);
        assert!(sexp.contains("(method_call"), "should be a method call: {}", sexp);
        assert!(sexp.contains("hash"), "should contain hash ref arg: {}", sexp);
    }

    #[test]
    fn nested_array_ref_in_hash_ref_arg() {
        // foo({items => [$a, $b, $c]})
        let sexp = parse_ok("foo({items => [$a, $b, $c]});");
        assert!(
            sexp.contains("function_call_expression") || sexp.contains("call"),
            "should be a function call: {}",
            sexp
        );
        assert!(sexp.contains("(array"), "should contain array ref: {}", sexp);
    }

    #[test]
    fn hash_ref_with_nested_hash_ref() {
        // foo({outer => {inner => 1}})
        let sexp = parse_ok("foo({outer => {inner => 1}});");
        assert!(
            sexp.contains("function_call_expression") || sexp.contains("call"),
            "should be a function call: {}",
            sexp
        );
    }

    #[test]
    fn multiple_hash_refs_as_args() {
        // foo({a => 1}, {b => 2}, {c => 3})
        let sexp = parse_ok("foo({a => 1}, {b => 2}, {c => 3});");
        assert!(
            sexp.contains("function_call_expression") || sexp.contains("call"),
            "should be a function call: {}",
            sexp
        );
    }

    #[test]
    fn hash_ref_after_scalar_arg() {
        // foo($x, {key => $val})
        let sexp = parse_ok("foo($x, {key => $val});");
        assert!(
            sexp.contains("function_call_expression") || sexp.contains("call"),
            "should be a function call: {}",
            sexp
        );
        assert!(sexp.contains("hash"), "should contain hash ref: {}", sexp);
    }

    #[test]
    fn array_ref_then_hash_ref_then_scalar() {
        // foo([$a], {b => $c}, $d)
        let sexp = parse_ok("foo([$a], {b => $c}, $d);");
        assert!(
            sexp.contains("function_call_expression") || sexp.contains("call"),
            "should be a function call: {}",
            sexp
        );
    }

    // ----- builtin function arguments with complex types -----

    #[test]
    fn push_with_hash_ref() {
        // push @arr, {key => $val}
        let sexp = parse_ok("push @arr, {key => $val};");
        assert!(sexp.contains("(call push"), "should be a push call: {}", sexp);
        assert!(sexp.contains("hash"), "should contain hash ref: {}", sexp);
    }

    #[test]
    fn push_with_array_ref() {
        // push @arr, [$a, $b]
        let sexp = parse_ok("push @arr, [$a, $b];");
        assert!(sexp.contains("(call push"), "should be a push call: {}", sexp);
        assert!(sexp.contains("(array"), "should contain array ref: {}", sexp);
    }

    #[test]
    fn die_with_hash_ref() {
        // die {message => "error", code => 42}
        let sexp = parse_ok(r#"die {message => "error", code => 42};"#);
        assert!(sexp.contains("(call die"), "should be a die call: {}", sexp);
    }

    #[test]
    fn return_hash_ref() {
        // return {key => $val}
        let sexp = parse_ok("return {key => $val};");
        assert!(sexp.contains("(return"), "should be a return: {}", sexp);
    }

    // ----- builtins without parens taking hash/array ref followed by more args -----

    #[test]
    fn push_hash_ref_then_more_args() {
        // push @arr, {key => $val}, $extra
        let sexp = parse_ok("push @arr, {key => $val}, $extra;");
        assert!(sexp.contains("(call push"), "should be a push call: {}", sexp);
        assert!(sexp.contains("hash"), "should contain hash ref: {}", sexp);
        assert!(sexp.contains("(variable $ extra)"), "should contain $extra: {}", sexp);
    }

    #[test]
    fn unshift_with_array_ref() {
        // unshift @arr, [$a, $b]
        let sexp = parse_ok("unshift @arr, [$a, $b];");
        assert!(sexp.contains("(call unshift"), "should be an unshift call: {}", sexp);
        assert!(sexp.contains("(array"), "should contain array ref: {}", sexp);
    }

    // ----- deeply nested constructors -----

    #[test]
    fn deeply_nested_mixed_constructors() {
        // foo({a => {b => [1, 2, {c => 3}]}})
        let sexp = parse_ok("foo({a => {b => [1, 2, {c => 3}]}});");
        assert!(
            sexp.contains("function_call_expression") || sexp.contains("call"),
            "should be a function call: {}",
            sexp
        );
    }

    #[test]
    fn nested_function_calls_with_refs() {
        // foo(bar({a => 1}), baz([$x]))
        let sexp = parse_ok("foo(bar({a => 1}), baz([$x]));");
        assert!(
            sexp.contains("function_call_expression") || sexp.contains("call"),
            "should parse nested calls: {}",
            sexp
        );
    }

    // ----- declaration keyword autoquoting in list/hash contexts -----

    #[test]
    fn my_keyword_autoquoted_as_hash_key_in_parens() {
        // my %h = (my => "value", use => "something")
        // `my` before `=>` should be autoquoted as a string, not parsed as a declaration
        let sexp = parse_ok(r#"my %h = (my => "value", use => "something");"#);
        assert!(sexp.contains("\"my\""), "my should be autoquoted: {}", sexp);
        assert!(sexp.contains("\"use\""), "use should be autoquoted: {}", sexp);
    }

    #[test]
    fn our_keyword_autoquoted_as_hash_key_in_parens() {
        // (our => 1, local => 2, state => 3)
        let sexp = parse_ok("my %h = (our => 1, local => 2, state => 3);");
        assert!(sexp.contains("\"our\""), "our should be autoquoted: {}", sexp);
        assert!(sexp.contains("\"local\""), "local should be autoquoted: {}", sexp);
        assert!(sexp.contains("\"state\""), "state should be autoquoted: {}", sexp);
    }

    #[test]
    fn my_keyword_autoquoted_in_function_call_args() {
        // foo(my => 1, our => 2)
        let sexp = parse_ok("foo(my => 1, our => 2);");
        assert!(!sexp.contains("ERROR"), "should parse without errors: {}", sexp);
        assert!(sexp.contains("\"my\""), "my should be autoquoted in function args: {}", sexp);
    }

    #[test]
    fn my_keyword_autoquoted_in_brace_hash() {
        // { my => 1, our => 2 }
        let sexp = parse_ok("my $h = {my => 1, our => 2};");
        assert!(!sexp.contains("ERROR"), "should parse without errors: {}", sexp);
    }
}
