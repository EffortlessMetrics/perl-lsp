use perl_parser::Parser;

fn main() {
    let test_cases = vec![
        "format STDOUT =\n@<<<<<<< @|||||||| @>>>>>>>>\n$name,   $ssn,     $salary\n.",
        "format =\n@<<<<<<<<<\n$value\n.",
        "format Something =\nTest @###.##\n$price\n.",
        "format REPORT_TOP =\n                        Passwd File\nName                Login    UID   GID Home\n------------------------------------------------------------------\n.",
    ];
    
    for code in test_cases {
        println!("Testing:\n{}", code);
        println!("---");
        
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("✓ Success! AST:\n{}", ast.to_sexp());
            }
            Err(e) => {
                println!("✗ Error: {:?}", e);
            }
        }
        println!();
    }
}