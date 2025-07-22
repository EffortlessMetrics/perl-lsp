//! Even more edge case tests for Perl parser - beyond the 200 we already have
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Filehandle edge cases
        ("print STDERR 'error'", "print to named filehandle"),
        ("print {$fh} 'text'", "print to filehandle ref"),
        ("<>", "null filehandle (all files)"),
        
        // Special literals
        ("__PACKAGE__", "package literal"),
        ("__FILE__", "file literal"),
        ("__LINE__", "line literal"),
        ("__SUB__", "current sub reference"),
        ("__END__", "end of code marker"),
        ("__DATA__", "data section marker"),
        
        // Numeric edge cases
        ("0b1010_1010", "binary with underscores"),
        ("0x1234_5678", "hex with underscores"),
        ("1_234.567_890", "decimal with underscores"),
        ("1e10", "scientific notation"),
        ("1E-10", "scientific with negative exponent"),
        
        // String edge cases
        ("\"\\x{263A}\"", "hex escape in string"),
        ("\"\\N{SNOWMAN}\"", "named unicode in string"),
        ("\"\\c[\"", "control character"),
        ("q!hello!", "q with exclamation delimiter"),
        ("qq#hello $world#", "qq with hash delimiter"),
        
        // Here-doc edge cases
        ("print <<~EOF;\n    indented\n    EOF", "indented heredoc"),
        ("print <<\\EOF;\nliteral\nEOF", "non-interpolating heredoc"),
        
        // Regex modifiers
        ("/pattern/msixpodualngc", "all regex modifiers"),
        ("m<pattern>", "match with angle brackets"),
        ("s{old}{new}g", "substitution with braces"),
        ("tr/a-z/A-Z/", "transliteration"),
        ("y/a-z/A-Z/", "y/// transliteration"),
        
        // Special arrays/hashes
        ("@_", "argument array"),
        ("@+", "regex capture end positions"),
        ("@-", "regex capture start positions"),
        ("%+", "named captures hash"),
        ("%-", "named captures array hash"),
        ("%ENV", "environment hash"),
        ("%SIG", "signal hash"),
        
        // Special scalars
        ("$$", "process ID"),
        ("$?", "child error"),
        ("$!", "system error"),
        ("$@", "eval error"),
        ("$&", "match string"),
        ("$`", "prematch string"),
        ("$'", "postmatch string"),
        ("$.", "line number"),
        ("$/", "input record separator"),
        ("$\\", "output record separator"),
        ("$|", "autoflush"),
        ("$^O", "OS name"),
        
        // Subroutine attributes
        ("sub foo : method { }", "method attribute"),
        ("sub bar : lvalue method { }", "multiple attributes"),
        
        // Prototypes
        ("sub mysub($$) { }", "two scalar prototype"),
        ("sub mygrep(&@) { }", "block and list prototype"),
        ("sub mymap(\\@) { }", "reference prototype"),
        
        // Special operators
        ("$x .= $y", "concatenation assignment"),
        ("$x x= 3", "repetition assignment"),
        ("$x >>= 2", "right shift assignment"),
        ("$x <<= 2", "left shift assignment"),
        ("-X _", "file test on last stat"),
        ("-f -r $file", "chained file tests"),
        
        // Context operators
        ("scalar @array", "scalar context"),
        ("wantarray", "context check"),
        
        // Loop control
        ("LABEL: while (1) { last LABEL }", "labeled last"),
        ("OUTER: for (@a) { INNER: for (@b) { next OUTER } }", "nested labels"),
        
        // Special use cases
        ("use 5.010;", "version requirement"),
        ("use feature 'say';", "feature pragma"),
        ("no warnings 'void';", "disable warnings"),
        
        // Operator edge cases
        ("!!", "double negation"),
        ("~~", "standalone smartmatch"),
        ("\\\\$x", "double reference"),
        
        // List/array operations
        ("@array[0..$#array]", "full array slice"),
        ("@hash{@keys}", "hash slice"),
        ("delete @hash{@keys}", "delete slice"),
        ("exists $hash{$key}", "exists operator"),
        
        // Anonymous constructs
        ("sub { $_[0] + $_[1] }", "anonymous sub with @_"),
        ("[\\$x, \\$y]", "arrayref of refs"),
        
        // Tied variables
        ("tied %hash", "get tied object"),
        
        // Special blocks
        ("AUTOLOAD { }", "autoload block"),
        ("DESTROY { }", "destructor block"),
        
        // v-strings
        ("v1.2.3", "v-string"),
        ("v65.66.67", "v-string as characters"),
        
        // Glob operations
        ("<*.pl>", "glob pattern"),
        ("glob('*.txt')", "glob function"),
        
        // Special assignment
        ("local $/ = undef", "localize and undefine"),
        ("local $| = 1", "localize and set"),
        
        // Octal edge cases
        ("0377", "old-style octal"),
        ("0o377", "new-style octal"),
        
        // Unicode in identifiers
        ("my $café = 1", "accented identifier"),
        ("sub π { 3.14159 }", "greek identifier"),
        ("my $♥ = 'love'", "emoji identifier"),
        
        // Compound statements
        ("do { $x } while $y", "do-while"),
        ("do { $x } until $y", "do-until"),
    ];
    
    let mut parser = Parser::new("");
    let mut passed = 0;
    let mut failed = 0;
    
    println!("Running {} more edge case tests...\n", tests.len());
    
    for (code, description) in tests {
        parser = Parser::new(code);
        match parser.parse() {
            Ok(_) => {
                println!("✅ {:<50} {}", description, code);
                passed += 1;
            }
            Err(e) => {
                println!("❌ {:<50} {}", description, code);
                println!("   Error: {:?}", e);
                failed += 1;
            }
        }
    }
    
    println!("\nMore Edge Cases Summary: {} passed, {} failed", passed, failed);
    
    if failed > 0 {
        std::process::exit(1);
    }
}