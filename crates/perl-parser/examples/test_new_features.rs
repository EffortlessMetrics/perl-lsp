//! Test all newly implemented features
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Substitution operators
        "$str =~ s/old/new/g",
        "$str =~ s{pattern}{replacement}gims",
        "$str =~ tr/a-z/A-Z/",
        "$str =~ y/0-9/a-j/",
        // Heredocs
        "my $text = <<EOF;
Some text
EOF
",
        "print <<'LITERAL';
No interpolation $here
LITERAL
",
        // Eval blocks
        "eval { risky_operation() }",
        "my $result = eval { 1 / $x } || 'error'",
        "eval \"print 'dynamic'\"",
        // Do blocks
        "do { my $x = compute(); process($x) }",
        "do 'config.pl'",
        // Given/when
        "given ($value) {
            when (1) { say 'one' }
            when ([2,3,4]) { say 'small' }
            when (/^foo/) { say 'foo' }
            default { say 'other' }
        }",
        // Smart match
        "$x ~~ @array",
        "$str ~~ /pattern/",
        "$value ~~ [1, 2, 3]",
        "$hash ~~ %other",
        // Combined features
        "eval {
            given ($input) {
                when (/^\\d+$/) { return int($input) }
                when (/^\\w+$/) { return lc($input) }
                default { die 'Invalid input' }
            }
        }",
        // Complex substitution with eval
        "$text =~ s/(\\w+)/eval { uc($1) }/ge",
    ];

    println!("Testing all newly implemented Perl parser features:\n");

    let mut passed = 0;
    let mut failed = 0;

    for test in tests {
        print!("Testing: {} ... ", test.lines().next().unwrap_or(test));
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(_) => {
                println!("✅ PASS");
                passed += 1;
            }
            Err(e) => {
                println!("❌ FAIL: {}", e);
                failed += 1;
            }
        }
    }

    println!("\n=== Summary ===");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);
    println!("Total:  {}", passed + failed);
    println!("Success rate: {:.1}%", (passed as f64 / (passed + failed) as f64) * 100.0);
}
