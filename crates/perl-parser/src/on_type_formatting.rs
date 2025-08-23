// crates/perl-parser/src/on_type_formatting.rs
use serde_json::json;
use serde_json::Value;

pub fn compute_on_type_edit(text: &str, line: u32, col: u32, ch: char) -> Option<Vec<Value>> {
    let lines: Vec<&str> = text.lines().collect();
    
    if line as usize >= lines.len() {
        return None;
    }
    
    match ch {
        '{' => {
            // After typing '{', add proper indentation for next line
            // Determine current line indentation
            let current_indent = get_indentation(&lines[line as usize]);
            let new_indent = current_indent + 2; // Add 2 spaces
            
            // Return newline with increased indent
            Some(vec![json!({
                "range": { 
                    "start": {"line": line, "character": col}, 
                    "end": {"line": line, "character": col} 
                },
                "newText": format!("\n{}", " ".repeat(new_indent))
            })])
        }
        '}' => {
            // After typing '}', adjust indentation of current line
            if line > 0 {
                let current_line = lines[line as usize];
                let current_indent = get_indentation(current_line);
                
                // Find matching opening brace to determine proper indent
                let target_indent = find_matching_brace_indent(&lines, line as usize)
                    .unwrap_or_else(|| current_indent.saturating_sub(2));
                
                if current_indent != target_indent {
                    // Replace leading whitespace
                    let _trimmed = current_line.trim_start();
                    Some(vec![json!({
                        "range": { 
                            "start": {"line": line, "character": 0}, 
                            "end": {"line": line, "character": current_indent as u32} 
                        },
                        "newText": " ".repeat(target_indent)
                    })])
                } else {
                    None
                }
            } else {
                None
            }
        }
        ';' => {
            // After semicolon at end of line, maintain indent on next line
            let current_indent = get_indentation(&lines[line as usize]);
            Some(vec![json!({
                "range": { 
                    "start": {"line": line, "character": col}, 
                    "end": {"line": line, "character": col} 
                },
                "newText": format!("\n{}", " ".repeat(current_indent))
            })])
        }
        '\n' | '\r' => {
            // After newline, maintain current indentation
            if line > 0 {
                let prev_line = lines[(line - 1) as usize];
                let prev_indent = get_indentation(prev_line);
                
                // If previous line ends with '{', increase indent
                let trimmed = prev_line.trim_end();
                let indent = if trimmed.ends_with('{') {
                    prev_indent + 2
                } else {
                    prev_indent
                };
                
                Some(vec![json!({
                    "range": { 
                        "start": {"line": line, "character": 0}, 
                        "end": {"line": line, "character": 0} 
                    },
                    "newText": " ".repeat(indent)
                })])
            } else {
                None
            }
        }
        _ => None
    }
}

fn get_indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn find_matching_brace_indent(lines: &[&str], closing_line: usize) -> Option<usize> {
    let mut brace_count = 1;
    
    // Search backwards for matching opening brace
    for i in (0..closing_line).rev() {
        let line = lines[i];
        for ch in line.chars().rev() {
            match ch {
                '}' => brace_count += 1,
                '{' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        return Some(get_indentation(line));
                    }
                }
                _ => {}
            }
        }
    }
    
    None
}