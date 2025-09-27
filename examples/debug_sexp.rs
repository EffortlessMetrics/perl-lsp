use perl_parser::Parser;

fn main() {
    let statements = vec![
        "'\"\"0a\u{1680}\"\t\u{1680}]".to_string(),
        "\t\t\t\u{a0}A\u{205f}A]\u{3000}'".to_string(),
    ];
    let code = statements.join("\n");
    println!("Code: {:?}", code);
    
    if let Ok(ast) = Parser::new(&code).parse() {
        let sexp = ast.to_sexp();
        println!("S-expression: {}", sexp);
        println!("S-expression bytes: {:?}", sexp.bytes().collect::<Vec<_>>());
        
        // Count manually
        let mut balance = 0;
        for ch in sexp.chars() {
            if ch == '(' { balance += 1; }
            if ch == ')' { balance -= 1; }
        }
        println!("Simple balance: {}", balance);
    }
}
