//! Expanded edge case tests for Perl parser
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Statement modifiers
        ("print $x if $y", "if statement modifier"),
        ("die unless $valid", "unless statement modifier"),
        ("next while $continue", "while statement modifier"),
        ("last until $done", "until statement modifier"),
        ("redo for @list", "for statement modifier"),
        
        // Complex operator precedence
        ("$a && $b || $c", "logical operators"),
        ("$x = $y == $z", "assignment vs comparison"),
        ("$a + $b * $c", "arithmetic precedence"),
        ("!$x && $y", "not vs and"),
        ("$a ? $b : $c ? $d : $e", "nested ternary"),
        
        // String operators
        ("'hello' . 'world'", "string concatenation"),
        ("'abc' x 3", "string repetition"),
        ("$str =~ s/foo/bar/g", "substitution with flags"),
        ("$str !~ /pattern/", "negative match"),
        
        // List operations
        ("(1, 2, 3)", "list literal"),
        ("(1..10)", "range in list"),
        ("@array[0, 2, 4]", "array slice with list"),
        ("@hash{qw(a b c)}", "hash slice"),
        ("push @arr, 1, 2, 3", "push multiple values"),
        
        // File operations
        ("open my $fh, '<', 'file.txt'", "three-arg open"),
        ("print $fh 'text'", "print to filehandle"),
        ("<$fh>", "readline from filehandle"),
        ("-e 'file.txt'", "file test operator"),
        ("-f $file && -r $file", "multiple file tests"),
        
        // Anonymous references
        ("[1, 2, 3]", "anonymous array ref"),
        ("{a => 1, b => 2}", "anonymous hash ref"),
        ("sub { print }", "anonymous sub ref"),
        ("\\do { my $x = 1 }", "reference to do block"),
        
        // Special literals
        ("__PACKAGE__", "package name"),
        ("__FILE__", "file name"),
        ("__LINE__", "line number"),
        ("__END__", "end marker"),
        ("__DATA__", "data section marker"),
        
        // Quoted constructs
        ("q{hello}", "single quoted string"),
        ("qq{hello $world}", "double quoted string"),
        ("qw(foo bar baz)", "quoted words"),
        ("qr{pattern}ims", "quoted regex"),
        ("qx{ls -la}", "quoted execution"),
        
        // Complex dereferences
        ("$hash->{key}->[0]->{sub}", "deep dereference"),
        ("@{$array_ref}", "array dereference"),
        ("%{$hash_ref}", "hash dereference"),
        ("&{$code_ref}", "code dereference"),
        ("${$scalar_ref}", "scalar dereference"),
        
        // Method calls
        ("$obj->method", "simple method call"),
        ("$obj->method()", "method call with parens"),
        ("$obj->method(1, 2)", "method call with args"),
        ("Class->new", "class method call"),
        ("$obj->$method", "dynamic method call"),
        
        // Special cases
        ("goto LABEL", "goto statement"),
        ("last LABEL", "labeled last"),
        ("next LABEL", "labeled next"),
        ("LABEL: for (@list) { }", "labeled loop"),
        
        // Prototypes and attributes
        ("sub foo ($) { }", "sub with prototype"),
        ("sub bar : lvalue { }", "sub with attribute"),
        ("my $x :shared", "variable attribute"),
        
        // Formats
        ("format STDOUT = ", "format declaration"),
        ("write", "write statement"),
        
        // Edge cases with barewords
        ("foo => 'bar'", "bareword before fat arrow"),
        ("foo()", "bareword function call"),
        ("Foo::Bar::", "trailing colons"),
        ("v1.2.3", "version string"),
        ("v49", "simple version string"),
        
        // Unicode and special chars
        ("my $café = 1", "unicode identifier"),
        ("sub π { 3.14159 }", "unicode sub name"),
        ("my $Σ = 0", "greek letter variable"),
        
        // Complex expressions at EOF
        ("1 + 2", "expression at EOF"),
        ("foo()", "call at EOF"),
        ("$x->{y}", "dereference at EOF"),
        
        // Empty constructs
        ("sub {}", "empty anonymous sub"),
        ("{}", "empty hash ref"),
        ("[]", "empty array ref"),
        ("()", "empty list"),
        
        // Octal and hex literals
        ("0755", "octal literal"),
        ("0o755", "modern octal"),
        ("0xFF", "hex literal"),
        ("0b1010", "binary literal"),
        
        // Special regex cases
        ("//", "empty regex"),
        ("s///", "empty substitution"),
        ("tr///", "empty transliteration"),
        ("m##", "regex with # delimiter"),
        
        // Continue blocks
        ("while (1) { } continue { }", "while with continue"),
        ("for (;;) { } continue { }", "for with continue"),
        
        // Ellipsis operator
        ("...", "ellipsis operator"),
        ("if ($x) { ... }", "ellipsis in block"),
        
        // Smart match operator
        ("$x ~~ $y", "smart match"),
        ("$x ~~ @array", "smart match with array"),
        ("$x ~~ /pattern/", "smart match with regex"),
        
        // State variables
        ("state $x = 0", "state variable"),
        ("state $x", "state without init"),
        
        // Given/when
        ("given ($x) { when (1) { } }", "given/when"),
        ("default { }", "default in given"),
        
        // Try/catch
        ("try { } catch { }", "try/catch"),
        ("try { } catch ($e) { }", "try/catch with var"),
        ("try { } finally { }", "try/finally"),
        
        // Defer blocks
        ("defer { cleanup() }", "defer block"),
        
        // Class/method syntax (newer Perl)
        ("class Foo { }", "class declaration"),
        ("method bar { }", "method declaration"),
        ("field $x", "field declaration"),
        
        // Complex number formats
        ("1_234_567", "number with underscores"),
        ("1.23e-10", "scientific notation"),
        (".5", "decimal without leading zero"),
        ("5.", "decimal without trailing digits"),
        
        // Special assignment operators
        ("$x //= 0", "defined-or assign"),
        ("$x &&= 1", "and assign"),
        ("$x ||= 1", "or assign"),
        ("$x .= 'foo'", "concat assign"),
        ("$x x= 3", "repeat assign"),
        
        // Postfix dereference (newer syntax)
        ("$ref->@*", "postfix array deref"),
        ("$ref->%*", "postfix hash deref"),
        ("$ref->$*", "postfix scalar deref"),
        ("$ref->&*", "postfix code deref"),
        
        // Loop control
        ("for (1..10) { }", "range in for"),
        ("for my $i (0..9) { }", "for with my"),
        ("foreach my $x (@list) { }", "foreach with my"),
        ("for (@{$ref}) { }", "for with deref"),
        
        // Chained comparisons
        ("$a < $b < $c", "chained less than"),
        ("$x == $y == $z", "chained equality"),
        
        // Special subroutine calls
        ("&foo", "sub call with &"),
        ("&foo()", "sub call with & and parens"),
        ("goto &sub", "goto sub"),
        
        // ISA operator
        ("$obj ISA 'Class'", "ISA operator"),
        ("$x isa $y", "isa operator (newer)"),
        
        // Bit operators
        ("$x & $y", "bitwise and"),
        ("$x | $y", "bitwise or"),
        ("$x ^ $y", "bitwise xor"),
        ("~$x", "bitwise not"),
        ("$x << 2", "left shift"),
        ("$x >> 2", "right shift"),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for (test, desc) in tests {
        print!("Testing {:<35} {:45} ", desc, test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(_ast) => {
                println!("✅");
                passed += 1;
            }
            Err(e) => {
                println!("❌ {}", e);
                failed += 1;
            }
        }
    }
    
    println!("\nSummary: {} passed, {} failed", passed, failed);
    
    if failed > 0 {
        println!("\nNote: Some failures are expected for unimplemented features");
    }
}