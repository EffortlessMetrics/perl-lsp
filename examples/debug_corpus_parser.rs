use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string("/home/steven/code/tree-sitter-perl/test/corpus/pod")?;
    
    println!("=== Full file content ===");
    println!("{}", content);
    
    println!("\n=== Line by line analysis ===");
    for (i, line) in content.lines().enumerate() {
        println!("{:3}: '{}'", i+1, line);
        if line.starts_with("================================================================================") {
            println!("     ^^ SEPARATOR");
        } else if line.starts_with("----") {
            println!("     ^^ DASH SEPARATOR");
        }
    }
    
    // Now run the parsing logic
    println!("\n=== Parsing logic simulation (NEW) ===");
    let mut current_name = String::new();
    let mut current_source = String::new();
    let mut current_expected = String::new();
    let mut in_source = false;
    let mut in_expected = false;
    
    for (i, line) in content.lines().enumerate() {
        println!("Line {}: '{}' | name='{}' in_source={}, in_expected={}", i+1, line, current_name, in_source, in_expected);
        
        if line.starts_with("================================================================================") {
            if !current_name.is_empty() && !current_source.is_empty() && !current_expected.is_empty() {
                println!("  >>> SAVING TEST CASE: '{}'", current_name);
                println!("      SOURCE: '{}'", current_source.trim());
                println!("      EXPECTED: '{}'", current_expected.trim());
            }
            
            // Start new test case or continue looking for test name
            if !current_name.is_empty() {
                // We already have a name, so this is the closing separator, start source mode
                in_source = true;
                in_expected = false;
                println!("  >>> CLOSING SEPARATOR - START SOURCE MODE");
            } else {
                // This is the opening separator, start fresh
                current_name.clear();
                current_source.clear();
                current_expected.clear();
                in_source = false;
                in_expected = false;
                println!("  >>> OPENING SEPARATOR - RESET STATE");
            }
        } else if line.starts_with("----") {
            in_source = false;
            in_expected = true;
            println!("  >>> SWITCH TO EXPECTED MODE");
        } else if !current_name.is_empty() && in_source {
            // We're collecting source code
            if !current_source.is_empty() {
                current_source.push('\n');
            }
            current_source.push_str(line);
            println!("  >>> ADD TO SOURCE: '{}'", current_source);
        } else if in_expected {
            // We're collecting expected output
            if !current_expected.is_empty() {
                current_expected.push('\n');
            }
            current_expected.push_str(line);
            println!("  >>> ADD TO EXPECTED");
        } else if current_name.is_empty() && !line.trim().is_empty() && !line.starts_with("=") {
            // This is the test case name
            current_name = line.trim().to_string();
            println!("  >>> SET NAME: '{}'", current_name);
        } else {
            println!("  >>> IGNORE");
        }
    }
    
    // Final test case
    if !current_name.is_empty() && !current_source.is_empty() && !current_expected.is_empty() {
        println!("\n>>> FINAL TEST CASE: '{}'", current_name);
        println!("    SOURCE: '{}'", current_source.trim());
    }
    
    Ok(())
}