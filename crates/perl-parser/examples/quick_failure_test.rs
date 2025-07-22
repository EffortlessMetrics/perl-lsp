//! Quick test of specific failure patterns
use perl_parser::Parser;

fn test_pattern(code: &str, desc: &str) {
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(_) => println!("✅ {}", desc),
        Err(e) => {
            println!("❌ {}", desc);
            println!("   Code: {}", code);
            println!("   Error: {:?}", e);
        }
    }
}

fn main() {
    println!("Testing specific failure patterns:\n");
    
    // From additional tests
    println!("=== Multiple Heredocs ===");
    test_pattern("func(<<EOF, <<'END');", "multiple heredocs in call");
    
    println!("\n=== Named Capture Groups ===");
    test_pattern(r"m{(?<name>\w+)}g", "named capture group");
    
    println!("\n=== Prototypes with Signatures ===");
    test_pattern("sub qux :prototype($) ($x) { }", "prototype with signature");
    
    println!("\n=== Assignment in While ===");
    test_pattern("while (my $line = <>) { }", "assignment in while");
    
    println!("\n=== Tie Operations ===");
    test_pattern("tie my @array, 'Class'", "tie array");
    
    println!("\n=== Array Slicing ===");
    test_pattern("@list[0..$#list]", "full array slice");
    
    println!("\n=== Quote-like Operators ===");
    test_pattern("qq#hello $world#", "qq with hash delimiter");
    test_pattern("m<pattern>", "match with angle brackets");
    
    println!("\n=== Attributes ===");
    test_pattern("sub foo : method { }", "method attribute");
    test_pattern("sub bar : lvalue method { }", "multiple attributes");
    
    println!("\n=== Special Blocks ===");
    test_pattern("AUTOLOAD { }", "autoload block");
    test_pattern("DESTROY { }", "destructor block");
    
    println!("\n=== Format Declarations ===");
    test_pattern(r#"format STDOUT =
@<<<<< @||||| @>>>>>
$name, $age, $score
.
"#, "basic format declaration");
    
    println!("\n=== Double Operators ===");
    test_pattern("!!", "double negation");
    test_pattern("~~", "standalone smartmatch");
}