use perl_parser::Parser;

fn main() {
    println!("🔧 Verifying S-expression generation fixes for Issue #72\n");

    let test_cases = vec![
        // Originally failing cases
        ("if ($x > 10) { print $x; }", vec!["binary_>", "if"]),
        ("while ($i < 10) { $i++; }", vec!["binary_<", "while", "unary_++"]),
        ("$result = ($a + $b) * $c;", vec!["binary_*", "binary_+", "assignment"]),
        // Additional verification cases
        ("$x and $y or $z", vec!["binary_and", "binary_or"]),
        ("!$flag", vec!["unary_not"]),
        ("-$number", vec!["unary_-"]),
        ("$ref->@*", vec!["unary_->@*"]),
        ("open FILE, \"test\" or die", vec!["call open", "call die", "binary_or"]),
        ("print \"Hello $name\"", vec!["call print", "string_interpolated"]),
        ("print 'Hello world'", vec!["call print", "string"]),
    ];

    let mut passed = 0;
    let mut total = 0;

    for (code, expected_patterns) in test_cases {
        total += 1;
        println!("Testing: {}", code);

        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                let sexp = ast.to_sexp();
                println!("  S-expression: {}", sexp);

                let mut all_found = true;
                for pattern in &expected_patterns {
                    if sexp.contains(pattern) {
                        println!("  ✅ Found: {}", pattern);
                    } else {
                        println!("  ❌ Missing: {}", pattern);
                        all_found = false;
                    }
                }

                if all_found {
                    println!("  ✅ PASS\n");
                    passed += 1;
                } else {
                    println!("  ❌ FAIL\n");
                }
            }
            Err(e) => {
                println!("  ❌ Parse error: {}\n", e);
            }
        }
    }

    println!("📊 Summary: {}/{} tests passed", passed, total);
    if passed == total {
        println!("🎉 All S-expression generation issues have been resolved!");
    } else {
        println!("⚠️ Some issues remain to be fixed");
    }
}
