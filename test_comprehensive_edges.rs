use perl_parser::Parser;

fn main() {
    let test_cases = vec![
        // Perl 5.36+ features
        ("use builtin 'trim'; trim($x)", "builtin functions"),
        ("class Point { field $x; field $y; }", "class with fields"),
        ("method add($other) { ... }", "method signatures"),
        
        // Special regex cases
        ("m<pattern>", "angle bracket delimiters"),
        ("s(old)(new)g", "parenthesis delimiters"),
        ("tr[a-z][A-Z]", "bracket delimiters"),
        ("y|a|b|", "y/// transliteration"),
        
        // Special quoting
        ("q<text>", "q with angle brackets"),
        ("qq|text $var|", "qq with pipes"),
        ("qw< one two three >", "qw with angles"),
        ("qx'command'", "qx with single quotes"),
        
        // Special variables
        ("$#{$arrayref}", "last index of arrayref"),
        ("@{$arrayref}[0..5]", "array slice of ref"),
        ("@hash{@keys}", "hash slice"),
        ("%hash{@keys}", "key-value slice"),
        
        // Prototypes edge cases  
        ("sub foo($$) { }", "scalar prototype"),
        ("sub bar(\\@) { }", "ref prototype"),
        ("sub baz(*) { }", "glob prototype"),
        ("sub quux(;$) { }", "optional prototype"),
        ("sub test(_) { }", "underscore prototype"),
        
        // Attributes edge cases
        ("sub foo :lvalue { }", "lvalue attribute"),
        ("sub bar :method :locked { }", "multiple attributes"),
        ("our $var :unique = 1;", "our with attribute"),
        
        // Special blocks
        ("INIT { }", "INIT block"),
        ("CHECK { }", "CHECK block"),
        ("UNITCHECK { }", "UNITCHECK block"),
        
        // Special operators
        ("$x // $y", "defined-or operator"),
        ("$x //= $y", "defined-or assignment"),
        ("$x ~~ $y", "smart match"),
        ("$x ... $y", "yada yada yada"),
        ("$x .. $y", "range operator"),
        ("$x x 3", "string repetition"),
        
        // Special syntax
        ("goto &sub", "goto subroutine"),
        ("goto LABEL", "goto label"),
        ("LABEL: for (...) { }", "labeled loop"),
        ("next LABEL", "next with label"),
        ("last LABEL", "last with label"),
        ("redo LABEL", "redo with label"),
        
        // File test operators
        ("-e $file", "file exists"),
        ("-r -w -x $file", "chained file tests"),
        ("-M $file > 7", "file age test"),
        
        // Special function calls
        ("do { ... }", "do block"),
        ("do 'file.pl'", "do file"),
        ("eval { ... }", "eval block"),
        ("eval 'code'", "eval string"),
        
        // Package and version
        ("package Foo 1.23;", "package with version"),
        ("package Foo::Bar { }", "package block"),
        ("require 5.36.0;", "require version"),
        ("use 5.36.0;", "use version"),
        
        // Special heredoc cases
        ("<<~'EOF'", "indented heredoc"),
        ("<<~\\EOF", "indented escaped heredoc"),
        ("print <<EOF, <<'EOF2';", "multiple heredocs"),
        
        // Context forcing
        ("scalar @array", "scalar context"),
        ("@{[ $x + $y ]}", "list interpolation"),
        ("~~@array", "boolean context"),
        
        // Special assignments
        ("local $/ = undef;", "local special var"),
        ("local *STDOUT = *STDERR;", "local typeglob"),
        ("our ($x, $y, $z);", "our list declaration"),
        
        // Loop control
        ("for my $i (1..10) { }", "for with my"),
        ("foreach my $x (@list) { }", "foreach with my"),
        ("while (my $line = <>) { }", "while with my"),
        
        // Special derefs
        ("$$ref", "scalar deref"),
        ("@$ref", "array deref"),
        ("%$ref", "hash deref"),
        ("&$ref", "code deref"),
        ("*$ref", "glob deref"),
        
        // Subroutine refs
        ("\\&foo", "sub reference"),
        ("&{$subref}(@args)", "sub deref call"),
        ("&$subref(@args)", "simple sub deref"),
        
        // Special literals  
        ("0x1234", "hex literal"),
        ("0o1234", "octal literal with o"),
        ("1_234_567", "underscore in number"),
        ("1.23e-10", "scientific notation"),
        
        // Unicode in different contexts
        ("package Café;", "unicode package name"),
        ("sub café { }", "unicode sub name"),
        ("café();", "unicode function call"),
        
        // Nested structures
        ("$hash{key1}{key2}[0]", "nested access"),
        ("$ref->{key}->[$i]->{$k}", "chained arrows"),
        
        // Special use cases
        ("use constant FOO => 42;", "constant pragma"),
        ("use vars qw($x @y %z);", "vars pragma"),
        ("no warnings 'void';", "no pragma"),
    ];
    
    let mut failed = Vec::new();
    let mut passed = 0;
    
    for (code, description) in &test_cases {
        print!("{:.<50} ", description);
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(_) => {
                println!("✓ PASS");
                passed += 1;
            }
            Err(e) => {
                println!("✗ FAIL: {:?}", e);
                failed.push((code.to_string(), description.to_string(), format!("{:?}", e)));
            }
        }
    }
    
    println!("\n========== SUMMARY ==========");
    println!("Total tests: {}", test_cases.len());
    println!("Passed: {} ({:.1}%)", passed, (passed as f64 / test_cases.len() as f64) * 100.0);
    println!("Failed: {} ({:.1}%)", failed.len(), (failed.len() as f64 / test_cases.len() as f64) * 100.0);
    
    if !failed.is_empty() {
        println!("\n========== FAILURES ==========");
        for (code, desc, error) in &failed {
            println!("\n{}: {}", desc, code);
            println!("  Error: {}", error);
        }
    }
}