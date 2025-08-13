//! Test edge cases in Perl parser
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Increment/decrement operators
        ("++$x", "pre-increment"),
        ("$x++", "post-increment"),
        ("--$x", "pre-decrement"),
        ("$x--", "post-decrement"),
        // Fat arrow in hashes
        ("{ key => 'value' }", "hash with fat arrow"),
        ("{ a => 1, b => 2 }", "hash with multiple pairs"),
        ("my %h = (foo => 'bar')", "hash assignment with fat arrow"),
        // Package names with ::
        ("Foo::Bar", "package name"),
        ("Foo::Bar::Baz", "nested package name"),
        ("My::Class->new()", "method call on package"),
        ("$Foo::Bar::var", "package variable"),
        // Reference operator
        ("\\$scalar", "scalar reference"),
        ("\\@array", "array reference"),
        ("\\%hash", "hash reference"),
        ("\\&sub", "sub reference"),
        // Diamond operator
        ("<>", "diamond operator"),
        ("<STDIN>", "readline from STDIN"),
        ("while (<>) { print }", "diamond in while"),
        // Anonymous subroutines
        ("sub { }", "empty anonymous sub"),
        ("sub { return 42 }", "anonymous sub with return"),
        ("my $f = sub { $_[0] + 1 }", "assign anonymous sub"),
        // Bare builtins
        ("print", "bare print"),
        ("say", "bare say"),
        ("return", "bare return"),
        // Complex expressions
        ("$x->{key}->[0]", "chained dereference"),
        ("$h{$key}", "hash element access"),
        ("@a[0..10]", "array slice"),
        ("%h{'a', 'b'}", "hash slice"),
        // Special variables
        ("$_", "default variable"),
        ("@_", "argument array"),
        ("$!", "error variable"),
        ("$$", "process ID"),
        // Regex operations
        ("/pattern/", "simple regex"),
        ("s/old/new/", "substitution"),
        ("tr/a-z/A-Z/", "transliteration"),
        ("m{pattern}i", "match with braces"),
        // Here docs
        ("<<EOF\nHello\nEOF", "simple heredoc"),
        ("<<'EOF'\nHello\nEOF", "single-quoted heredoc"),
        // Statement forms
        ("do { } while $x", "do-while"),
        ("eval { risky() }", "eval block"),
        ("defined($x)", "defined function"),
        // Modern Perl
        ("use v5.36", "version declaration"),
        ("no warnings 'void'", "pragma with args"),
        ("__PACKAGE__", "package name token"),
        ("__FILE__", "file name token"),
        ("__LINE__", "line number token"),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for (test, desc) in tests {
        print!("Testing {:<25} {:40} ", desc, test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(_ast) => {
                println!("✅");
                // println!("   S-expr: {}", ast.to_sexp());
                passed += 1;
            }
            Err(e) => {
                println!("❌ {}", e);
                failed += 1;
            }
        }
    }

    println!("\nSummary: {} passed, {} failed", passed, failed);
}
