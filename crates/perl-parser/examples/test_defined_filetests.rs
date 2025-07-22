//! Test defined operator and file test operators
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // defined operator
        "defined $x",
        "defined($x)",
        "if (defined $hash{key}) { }",
        "unless (defined $var) { }",
        
        // File test operators
        "-e $file",
        "-f $file",
        "-d $dir",
        "-r $file",
        "-w $file",
        "-x $file",
        "-s $file",
        
        // In conditionals
        "if (-e $file) { }",
        "die unless -f $file",
        "if (-d $dir && -w $dir) { }",
        
        // Chained file tests
        "-f -r $file",
        
        // With $_
        "-e",
        "if (-f) { }",
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