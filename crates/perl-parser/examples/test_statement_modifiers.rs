//! Test statement modifiers
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // If modifier
        "print 'hello' if $x",
        "die 'error' if $x > 10",
        "return 42 if defined $value",
        
        // Unless modifier
        "print 'hello' unless $quiet",
        "next unless $item",
        "die unless $required",
        
        // While modifier
        "print while <STDIN>",
        "$x++ while $x < 10",
        "shift @array while @array",
        
        // Until modifier
        "sleep 1 until $ready",
        "$x-- until $x == 0",
        "wait until $done",
        
        // For/foreach modifier
        "print for @items",
        "say $_ for 1..10",
        "$sum += $_ for @numbers",
        
        // Complex examples
        "print \"$_\\n\" for grep { $_ > 5 } @numbers",
        "warn 'Empty!' unless @array",
        "$hash{$_}++ for split /\\s+/, $text",
        
        // Multiple modifiers (not allowed, should fail)
        "print if $x if $y",
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