use perl_parser::Parser;

fn main() {
    println!("Testing potential edge case failures...\n");
    
    let test_cases = vec![
        // Complex format with special picture lines
        (r#"format COMPLEX =
@<<<<<<<<<<<<<<<<<
$name
~~^<<<<<<<<<<<<<<<
$text
.
"#, "format with ~~ continuation"),
        
        // Regex with code assertion
        ("/(?{ $foo = 1 })/", "regex with code assertion"),
        ("/(??{ $pattern })/", "regex with deferred assertion"),
        
        // Complex prototypes
        ("sub foo ($$$) { }", "sub with $$$ prototype"),
        ("sub bar (\\@) { }", "sub with \\@ prototype"),
        ("sub baz (*) { }", "sub with * prototype"),
        ("sub qux (;$) { }", "sub with optional scalar prototype"),
        
        // Variable attributes with multiple attributes
        ("my $x :shared :unique;", "variable with multiple attributes"),
        ("our @arr :shared :unique = ();", "array with attributes and init"),
        
        // Complex tie with list
        ("tie my ($a, $b), 'Class'", "tie with list of variables"),
        
        // Complex given/when
        ("given ($x) { when ([1,2,3]) { } when (/foo/) { } default { } }", "complex given/when"),
        
        // Indirect object syntax
        ("new Class $arg1, $arg2", "indirect object syntax"),
        ("print STDERR $message", "indirect filehandle"),
        
        // Complex dereferencing
        ("$ref->$*", "scalar dereference with postfix"),
        ("$ref->**", "glob dereference with postfix"),
        
        // Special operators in void context
        ("<>", "diamond operator alone"),
        ("<<>>", "double diamond operator"),
        
        // State variables with attributes
        ("state $x :shared = 42;", "state variable with attribute"),
        
        // Complex package declarations
        ("package Foo::Bar 1.23 { }", "package with version and block"),
        ("package Foo::Bar v1.2.3;", "package with v-string version"),
        
        // Typeglobs and symbol table manipulation
        ("*foo = \\&bar", "typeglob assignment"),
        ("*{$name} = \\&code", "dynamic typeglob"),
        
        // Experimental features
        ("use feature 'switch'; given ($x) { when (1) { } }", "feature pragma with given/when"),
        
        // Complex heredoc edge cases
        (r#"print <<"EOF", <<"EOF2";
First heredoc
EOF
Second heredoc
EOF2
"#, "multiple heredocs on same line"),
        
        // Nested quotes
        ("qq{foo {bar} baz}", "nested braces in qq"),
        ("qr{(?:{foo})}", "nested braces in qr"),
        
        // Signature with slurpy and optional
        ("sub foo ($x, $y = 42, @rest) { }", "signature with default and slurpy"),
        
        // Complex file tests
        ("-f -r -w -x $file", "chained file tests"),
        
        // Smartmatch variations
        ("$x ~~ \\&code", "smartmatch with coderef"),
        ("$x ~~ qr/pattern/", "smartmatch with qr//"),
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
    let total = passed + failed;
    let percentage = if total > 0 {
        (passed as f64 / total as f64) * 100.0
    } else {
        100.0
    };
    println!("Success rate: {:.1}%", percentage);
    
    if !failures.is_empty() {
        println!("\n=== Failures ===");
        for (desc, error) in failures {
            println!("- {}: {}", desc, error);
        }
    }
}