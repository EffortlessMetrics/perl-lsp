use perl_parser::Parser;

fn main() {
    println!("Testing remaining edge cases from failure analysis...\n");

    let test_cases = vec![
        // Quote operators with non-standard delimiters
        ("qq#hello $world#", "qq with # delimiter"),
        ("m<pattern>", "match with angle brackets"),
        ("s{foo}[bar]", "substitution with mixed delimiters"),
        ("q!literal!", "q with ! delimiter"),
        ("qw/word1 word2/", "qw with / delimiter"),
        // Tie operations
        ("tie my @array, 'Class'", "tie with array declaration"),
        ("tie my %hash, 'Class'", "tie with hash declaration"),
        // Array/hash slicing
        ("@array[0..$#array]", "array slice with $# operator"),
        ("@hash{@keys}", "hash slice"),
        ("$ref->@[0..5]", "postfix array slice"),
        // Assignment in conditionals
        ("while (my $line = <>) { }", "declaration in while condition"),
        ("if (my $result = compute()) { }", "declaration in if condition"),
        // Named regex captures
        ("m{(?<name>\\w+)}g", "named capture group"),
        ("$+{name}", "access named capture"),
        // Special blocks without sub
        ("AUTOLOAD { }", "bare AUTOLOAD block"),
        ("DESTROY { }", "bare DESTROY block"),
        // Incomplete expressions
        ("!!", "double negation prefix"),
        ("~~", "standalone smartmatch"),
        // Prototypes with signatures
        ("sub qux :prototype($) ($x) { }", "prototype with signature"),
    ];

    let mut passed = 0;
    let mut failed = 0;
    let mut failures = Vec::new();

    for (code, desc) in test_cases {
        print!("Testing {}: ", desc);
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(_) => {
                println!("✅ PASS");
                passed += 1;
            }
            Err(e) => {
                println!("❌ FAIL - {}", e);
                failed += 1;
                failures.push((desc, format!("{}", e)));
            }
        }
    }

    println!("\n=== Summary ===");
    println!("✅ Passed: {}", passed);
    println!("❌ Failed: {}", failed);

    if !failures.is_empty() {
        println!("\n=== Failures ===");
        for (desc, error) in failures {
            println!("- {}: {}", desc, error);
        }
    }
}
