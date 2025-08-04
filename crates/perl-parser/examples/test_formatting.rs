//! Example of using the code formatter

use perl_parser::{CodeFormatter, FormattingOptions};

fn main() {
    let formatter = CodeFormatter::new();
    
    // Test unformatted code
    let code = r#"sub test{my$x=1;my$y=2;print"$x + $y = ",($x+$y),"\n";}"#;
    
    println!("Original code:");
    println!("{}", code);
    println!();
    
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: Some(true),
        insert_final_newline: Some(true),
        trim_final_newlines: Some(true),
    };
    
    match formatter.format_document(code, &options) {
        Ok(edits) => {
            println!("Formatting succeeded!");
            for edit in &edits {
                println!("Edit range: {:?}", edit.range);
                println!("Formatted code:");
                println!("{}", edit.new_text);
            }
        }
        Err(e) => {
            eprintln!("Formatting failed: {}", e);
        }
    }
}