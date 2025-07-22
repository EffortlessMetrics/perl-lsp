//! Test additional operators and features
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Range operator
        "1..10",
        "@array[1..5]",
        "for (1..10) { }",
        
        // Ternary operator
        "$x ? $y : $z",
        "$age >= 18 ? 'adult' : 'minor'",
        
        // Logical operators (word form)
        "$x and $y",
        "$x or $y",
        "$x xor $y",
        "not $x",
        
        // Binding operators
        "$x = $y",
        "$x += 5",
        "$x -= 3",
        "$x *= 2",
        "$x /= 2",
        "$x .= 'suffix'",
        
        // List operators
        "push @array, $x",
        "pop @array",
        "shift @array",
        "unshift @array, $x",
        
        // Hash operators
        "keys %hash",
        "values %hash",
        "each %hash",
        "delete $hash{key}",
        "exists $hash{key}",
        
        // String operators
        "length $str",
        "substr $str, 0, 5",
        "index $str, 'pattern'",
        "reverse $str",
        
        // Array operators
        "scalar @array",
        "grep { $_ > 5 } @array",
        "map { $_ * 2 } @array",
        "sort @array",
        "reverse @array",
    ];
    
    for test in tests {
        println!("\nTesting: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success!");
                println!("   S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}