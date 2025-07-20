use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar_inline = r#"
WHITESPACE = _{ " " | "\t" | "\r" | "\n" }

program = { SOI ~ statement* ~ EOI }

statement = {
    format_declaration
    | expression_statement
}

format_declaration = {
    "format" ~ identifier? ~ "=" ~ NEWLINE ~
    format_lines ~
    format_end
}

format_lines = { format_line* }
format_line = { 
    !format_end ~ (!NEWLINE ~ ANY)* ~ NEWLINE
}
format_end = { "." ~ NEWLINE? }

expression_statement = { expression ~ ";"? }
expression = { identifier }
identifier = @{ ASCII_ALPHA ~ (ASCII_ALPHANUMERIC | "_")* }

NEWLINE = { "\n" | "\r\n" }
"#]
struct TestParser;

fn main() {
    let input = "format STDOUT =\ntest line\n.\n";
    
    match TestParser::parse(Rule::program, input) {
        Ok(pairs) => {
            println!("Success!");
            for pair in pairs {
                println!("{:#?}", pair);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}