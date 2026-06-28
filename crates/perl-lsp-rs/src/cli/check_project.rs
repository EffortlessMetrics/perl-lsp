use std::collections::HashMap;
use std::path::Path;

struct FileError {
    path: String,
    errors: Vec<String>,
}

pub(super) fn run_check_project(dir: &str) -> i32 {
    let root = Path::new(dir);
    let metadata = match root.metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("{dir}: directory not found");
            return 1;
        }
        Err(error) => {
            eprintln!("{dir}: cannot access directory: {error}");
            return 1;
        }
    };
    if !metadata.is_dir() {
        eprintln!("{dir}: not a directory");
        return 1;
    }

    let mut results = ProjectCheckResults::default();
    let walker = walkdir::WalkDir::new(root).follow_links(true).into_iter();

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !is_supported_perl_file(path) {
            continue;
        }

        results.total += 1;
        let path_str = path.display().to_string();
        process_file(path, path_str, &mut results);
    }

    emit_report(dir, &results)
}

#[derive(Default)]
struct ProjectCheckResults {
    total: usize,
    clean: usize,
    file_errors: Vec<FileError>,
    category_counts: HashMap<String, usize>,
}

fn is_supported_perl_file(path: &Path) -> bool {
    const EXTENSIONS: &[&str] = &["pm", "pl", "t"];
    path.is_file()
        && path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| EXTENSIONS.contains(&e))
            .unwrap_or(false)
}

fn process_file(path: &Path, path_str: String, results: &mut ProjectCheckResults) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            record_file_error(path_str, format!("read error: {e}"), results);
            return;
        }
    };

    let mut parser = perl_parser::Parser::new(&source);
    let parse_result = parser.parse();
    let recovered_errors = parser.errors();

    let mut errors_for_file: Vec<String> = Vec::new();

    for err in recovered_errors {
        record_category(&format!("{err}"), results);
        errors_for_file.push(format!("{err}"));
    }

    if let Err(ref e) = parse_result {
        record_category(&format!("{e}"), results);
        errors_for_file.push(format!("{e}"));
    }

    if errors_for_file.is_empty() {
        results.clean += 1;
    } else {
        results.file_errors.push(FileError { path: path_str, errors: errors_for_file });
    }
}

fn record_file_error(path: String, message: String, results: &mut ProjectCheckResults) {
    results.file_errors.push(FileError { path, errors: vec![message.clone()] });
    record_category(&message, results);
}

fn record_category(message: &str, results: &mut ProjectCheckResults) {
    let category = categorize_error(message);
    results.category_counts.entry(category).and_modify(|c| *c += 1).or_insert(1);
}

fn emit_report(dir: &str, results: &ProjectCheckResults) -> i32 {
    println!("Perl Project Parsability Report");
    println!("===============================");
    println!();
    println!("Directory: {dir}");
    println!("Files scanned: {}", results.total);

    if results.total == 0 {
        println!();
        println!("No Perl files (.pm, .pl, .t) found.");
        return 0;
    }

    let pct = (results.clean as f64 / results.total as f64) * 100.0;
    println!("Clean parses: {}/{} ({pct:.1}%)", results.clean, results.total);
    println!();

    if !results.file_errors.is_empty() {
        println!("Parse errors:");
        for fe in &results.file_errors {
            for err in &fe.errors {
                println!("  {}: {err}", fe.path);
            }
        }
        println!();
    }

    emit_category_section(&results.category_counts);

    if pct >= 80.0 {
        println!("Assessment: PASS ({pct:.1}% parsable)");
        0
    } else {
        println!("Assessment: FAIL ({pct:.1}% parsable, threshold 80%)");
        1
    }
}

fn emit_category_section(category_counts: &HashMap<String, usize>) {
    if category_counts.is_empty() {
        return;
    }

    let mut cats: Vec<_> = category_counts.iter().collect();
    cats.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    println!("Top issue categories:");
    for (cat, count) in &cats {
        println!("  {cat}: {count}");
    }

    let suggested_fixes: Vec<_> = cats
        .iter()
        .filter_map(|(cat, _)| remediation_hint_for_category(cat).map(|hint| (cat.as_str(), hint)))
        .take(3)
        .collect();

    if !suggested_fixes.is_empty() {
        println!();
        println!("Suggested next steps:");
        for (category, suggestion) in suggested_fixes {
            println!("  {category}: {suggestion}");
        }
    }
    println!();
}

fn categorize_error(msg: &str) -> String {
    if msg.contains("Unexpected end of input") {
        "Unexpected EOF".to_string()
    } else if msg.contains("expected") && msg.contains("found") {
        "Unexpected token".to_string()
    } else if msg.contains("Invalid syntax") {
        "Syntax error".to_string()
    } else if msg.contains("Lexer error") {
        "Lexer error".to_string()
    } else if msg.contains("recursion") || msg.contains("Recursion") {
        "Recursion limit".to_string()
    } else if msg.contains("read error") {
        "IO error".to_string()
    } else {
        "Other".to_string()
    }
}

fn remediation_hint_for_category(category: &str) -> Option<&'static str> {
    match category {
        "Unexpected EOF" => Some(
            "Check for unclosed blocks, quotes, or heredocs near the end of each failing file.",
        ),
        "Unexpected token" => Some(
            "Run `perl -c <file>` to compare parser output and inspect the token shown in the error.",
        ),
        "Syntax error" => {
            Some("Review recently edited lines for malformed declarations or expressions.")
        }
        "Lexer error" => {
            Some("Look for invalid bytes, malformed UTF-8, or unterminated strings/regex literals.")
        }
        "Recursion limit" => Some(
            "Minimize deeply nested constructs and isolate the smallest snippet that reproduces the issue.",
        ),
        "IO error" => {
            Some("Check file permissions and symbolic links, then rerun with readable paths.")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{categorize_error, remediation_hint_for_category};

    #[test]
    fn categorize_error_maps_known_cases() {
        assert_eq!(categorize_error("Unexpected end of input while parsing"), "Unexpected EOF");
        assert_eq!(categorize_error("expected ; but found }"), "Unexpected token");
        assert_eq!(categorize_error("Invalid syntax near token"), "Syntax error");
        assert_eq!(categorize_error("Lexer error: invalid byte"), "Lexer error");
        assert_eq!(categorize_error("Recursion depth exceeded"), "Recursion limit");
        assert_eq!(categorize_error("read error: permission denied"), "IO error");
        assert_eq!(categorize_error("something new"), "Other");
    }

    #[test]
    fn remediation_hints_cover_major_error_categories() {
        assert!(remediation_hint_for_category("Unexpected EOF").is_some());
        assert!(remediation_hint_for_category("Unexpected token").is_some());
        assert!(remediation_hint_for_category("Syntax error").is_some());
        assert!(remediation_hint_for_category("Lexer error").is_some());
        assert!(remediation_hint_for_category("Recursion limit").is_some());
        assert!(remediation_hint_for_category("IO error").is_some());
    }

    #[test]
    fn remediation_hints_skip_unknown_categories() {
        assert!(remediation_hint_for_category("Other").is_none());
    }
}
