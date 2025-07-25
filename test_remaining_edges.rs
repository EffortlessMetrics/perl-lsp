use perl_parser::Parser;

fn main() {
    let test_cases = vec![
        // Indirect object syntax variations
        ("print STDOUT 'hello';", "indirect object with bareword filehandle"),
        ("method $object @args;", "indirect method call"),
        ("new Class $arg1, $arg2;", "indirect constructor"),
        
        // Complex dereferencing
        ("$foo->@*", "postfix array dereference"),
        ("$foo->%*", "postfix hash dereference"),
        ("$foo->**", "postfix glob dereference"),
        ("$foo->$*", "postfix scalar dereference"),
        
        // Typeglobs and symbol table
        ("*foo = \\&bar;", "typeglob assignment"),
        ("*{$package . '::foo'} = \\&bar;", "dynamic typeglob"),
        
        // Special literals
        ("__PACKAGE__", "__PACKAGE__ literal"),
        ("__FILE__", "__FILE__ literal"),
        ("__LINE__", "__LINE__ literal"),
        ("__SUB__", "__SUB__ literal"),
        
        // Attributes on lexicals
        ("my $x :shared = 1;", "lexical with attribute"),
        ("my ($x :shared, $y :locked);", "multiple lexicals with attributes"),
        
        // Tied variables
        ("tie $scalar, 'Tie::Scalar';", "tie scalar"),
        ("tie @array, 'Tie::Array';", "tie array"),
        ("tie %hash, 'Tie::Hash';", "tie hash"),
        
        // Special blocks
        ("AUTOLOAD { print 'autoloading'; }", "AUTOLOAD block"),
        ("DESTROY { print 'destroying'; }", "DESTROY block"),
        
        // v-strings
        ("v1.2.3", "v-string literal"),
        ("use v5.36;", "use with v-string"),
        
        // Octal and binary literals
        ("0b1010", "binary literal"),
        ("0755", "octal literal"),
        ("0o755", "explicit octal literal"),
        
        // Special regex modifiers
        ("m/pattern/aa", "regex with aa modifier"),
        ("s/old/new/r", "substitution with r modifier"),
        
        // Yada-yada operator
        ("sub todo { ... }", "yada-yada operator"),
        
        // State variables in different contexts
        ("state $x = state $y = 1;", "nested state declarations"),
        
        // Special quote-like operators
        ("qx{ls -la}", "qx with braces"),
        ("qr{pattern}msi", "qr with modifiers"),
    ];
    
    let mut failed = Vec::new();
    
    for (code, description) in test_cases {
        print!("Testing {}: {} ... ", description, code);
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(_) => println!("✓ PASS"),
            Err(e) => {
                println!("✗ FAIL: {:?}", e);
                failed.push((code, description));
            }
        }
    }
    
    if !failed.is_empty() {
        println!("\n{} tests failed:", failed.len());
        for (code, desc) in failed {
            println!("  - {}: {}", desc, code);
        }
    } else {
        println!("\nAll tests passed!");
    }
}