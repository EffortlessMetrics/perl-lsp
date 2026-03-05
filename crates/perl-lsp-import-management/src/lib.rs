//! Perl import-management helpers for LSP code actions.
//!
//! This crate intentionally focuses on a single responsibility:
//! collecting, classifying, and rewriting `use`/`require` statements.

/// Guess a module name for a common function.
#[must_use]
pub fn guess_module_for_function(func: &str) -> Option<String> {
    match func {
        "dumper" => Some("Data::Dumper"),
        "encode" | "decode" => Some("Encode"),
        "basename" | "dirname" => Some("File::Basename"),
        "mkpath" | "rmtree" => Some("File::Path"),
        "slurp" => Some("File::Slurp"),
        "decode_json" | "encode_json" => Some("JSON"),
        _ => None,
    }
    .map(str::to_string)
}

/// Collect import statements (`use` and `require`) from source lines.
#[must_use]
pub fn collect_imports(lines: &[String]) -> Vec<String> {
    let mut imports = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("use ") || trimmed.starts_with("require ") {
            imports.push(line.clone());
        }
    }

    imports
}

/// Sort imports by category: pragmas, core, CPAN-style, then local.
#[must_use]
pub fn sort_imports(imports: Vec<String>) -> Vec<String> {
    let mut pragmas = Vec::new();
    let mut core = Vec::new();
    let mut cpan = Vec::new();
    let mut local = Vec::new();

    for import in imports {
        if import.contains("strict")
            || import.contains("warnings")
            || import.contains("utf8")
            || import.contains("feature")
        {
            pragmas.push(import);
        } else if import.contains("::") {
            cpan.push(import);
        } else if import.starts_with("use lib") || import.contains("./") {
            local.push(import);
        } else {
            core.push(import);
        }
    }

    pragmas.sort();
    core.sort();
    cpan.sort();
    local.sort();

    let mut result = Vec::new();
    result.extend(pragmas);
    result.extend(core);
    result.extend(cpan);
    result.extend(local);

    result
}

/// Find the byte range containing the contiguous import block boundaries.
#[must_use]
pub fn find_imports_range(source: &str, lines: &[String]) -> Option<(usize, usize)> {
    let imports = collect_imports(lines);
    if imports.is_empty() {
        return None;
    }

    let first = source.find(imports.first()?)?;
    let last_line = imports.last()?;
    let last = source.find(last_line)?;
    let last_end = last + last_line.len();

    Some((first, last_end))
}
