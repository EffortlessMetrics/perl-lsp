//! Import optimization for Perl modules (stub implementation)
//!
//! This module analyzes import statements and usage to optimize imports.
//! Currently a stub implementation to demonstrate the architecture.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A single import statement found in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStatement {
    pub module: String,
    pub symbols: Vec<String>,
    pub line: usize,
}

/// Result of import analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportAnalysis {
    /// All import statements that were parsed from the file
    pub imports: Vec<ImportStatement>,
    /// Imports that appear in the file but are never referenced
    pub unused_imports: Vec<UnusedImport>,
    /// References that appear to require imports that are missing
    pub missing_imports: Vec<MissingImport>,
    /// Multiple import statements for the same module
    pub duplicate_imports: Vec<DuplicateImport>,
    /// Style suggestions for organizing import blocks
    pub organization_suggestions: Vec<OrganizationSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnusedImport {
    pub module: String,
    pub symbols: Vec<String>,
    pub line: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingImport {
    pub module: String,
    pub symbols: Vec<String>,
    pub suggested_location: usize,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateImport {
    pub module: String,
    pub lines: Vec<usize>,
    pub can_merge: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSuggestion {
    pub description: String,
    pub priority: SuggestionPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestionPriority {
    High,
    Medium,
    Low,
}

/// Import optimizer
pub struct ImportOptimizer;

/// Check if a module is a pragma (affects compilation, no exports)
fn is_pragma_module(module: &str) -> bool {
    matches!(
        module,
        "strict"
            | "warnings"
            | "utf8"
            | "bytes"
            | "locale"
            | "integer"
            | "less"
            | "sigtrap"
            | "subs"
            | "vars"
            | "feature"
            | "autodie"
            | "autouse"
            | "base"
            | "parent"
            | "lib"
            | "bigint"
            | "bignum"
            | "bigrat"
    )
}

/// Get known exports for popular Perl modules
fn get_known_module_exports(module: &str) -> Vec<&'static str> {
    match module {
        "Data::Dumper" => vec!["Dumper"],
        "JSON" => vec!["encode_json", "decode_json", "to_json", "from_json"],
        "YAML" => vec!["Load", "Dump", "LoadFile", "DumpFile"],
        "Storable" => vec!["store", "retrieve", "freeze", "thaw"],
        "List::Util" => vec!["first", "max", "min", "sum", "reduce", "shuffle", "uniq"],
        "Scalar::Util" => vec!["blessed", "reftype", "looks_like_number", "weaken"],
        "File::Spec" => vec!["catfile", "catdir", "splitpath", "splitdir"],
        "File::Basename" => vec!["basename", "dirname", "fileparse"],
        "Cwd" => vec!["getcwd", "abs_path", "realpath"],
        "Time::HiRes" => vec!["time", "sleep", "usleep", "gettimeofday"],
        "Digest::MD5" => vec!["md5", "md5_hex", "md5_base64"],
        "MIME::Base64" => vec!["encode_base64", "decode_base64"],
        "URI::Escape" => vec!["uri_escape", "uri_unescape"],
        "LWP::Simple" => vec!["get", "head", "getprint", "getstore", "mirror"],
        "CGI" => vec!["param", "header", "start_html", "end_html"],
        "DBI" => vec![],      // DBI is object-oriented, no default exports
        "strict" => vec![],   // Pragma, no exports
        "warnings" => vec![], // Pragma, no exports
        "utf8" => vec![],     // Pragma, no exports
        _ => vec![],
    }
}

impl ImportOptimizer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze imports in a file
    ///
    /// This performs a lightweight scan of the file to find `use` statements and
    /// simple `Module::symbol` references. The analysis identifies unused,
    /// missing and duplicate imports.
    pub fn analyze_file(&self, file_path: &Path) -> Result<ImportAnalysis, String> {
        let text = fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?;

        // Regex to capture `use Module;` and `use Module qw(...);` statements
        let import_re = Regex::new(
            r"(?m)^[ \t]*use\s+([A-Za-z_][\w:]*)\s*(?:qw\s*[({\[]([^)}\]]*)[)}\]])?\s*;\s*(?:#.*)?$"
        ).unwrap();
        // Regex to capture `Module::symbol` references (more precise)
        let module_ref_re = Regex::new(
            r"\b([A-Z][A-Za-z0-9_]*(?:::[A-Z][A-Za-z0-9_]*)*)::([A-Za-z_][A-Za-z0-9_]*)\b",
        )
        .unwrap();

        // Regex patterns to exclude strings, comments, and regex literals (non-greedy)
        let string_re = Regex::new(r#""(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'"|qq?\([^)]*\)|qr\([^)]*\)|qq?\{[^}]*\}|qr\{[^}]*\}|qq?\[[^\]]*\]|qr\[[^\]]*\]|qq?/[^/]*/[gimosxp]*|qr/[^/]*/[gimosxp]*"#).unwrap();
        let comment_re = Regex::new(r"(?m)#.*$").unwrap();
        let pod_re = Regex::new(r"(?s)^=\w+.*?^=cut").unwrap();

        // Parse import statements along with their byte ranges to facilitate
        // subsequent searches for usage.
        #[derive(Clone)]
        struct ImportLine {
            stmt: ImportStatement,
            start: usize,
            end: usize,
        }

        let mut imports: Vec<ImportLine> = Vec::new();
        for caps in import_re.captures_iter(&text) {
            let m = caps.get(0).unwrap();
            let module = caps[1].to_string();

            // Extract symbols from qw() or qw{} or qw[] constructs
            let symbols = caps
                .get(2)
                .map(|m| {
                    m.as_str()
                        .split_whitespace()
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();

            let line = text[..m.start()].lines().count() + 1;
            imports.push(ImportLine {
                stmt: ImportStatement { module, symbols, line },
                start: m.start(),
                end: m.end(),
            });
        }

        // Build a version of the file without import lines for usage analysis
        let mut text_without_imports = String::new();
        let mut last = 0;
        for imp in &imports {
            text_without_imports.push_str(&text[last..imp.start]);
            last = imp.end;
        }
        text_without_imports.push_str(&text[last..]);

        // Remove strings, comments, and POD sections for more accurate analysis
        let mut clean_text = text_without_imports.clone();

        // Replace strings and regex literals with spaces to preserve line numbers
        clean_text = string_re
            .replace_all(&clean_text, |m: &regex::Captures<'_>| {
                " ".repeat(m.get(0).unwrap().as_str().len())
            })
            .to_string();

        // Replace comments with spaces to preserve line numbers
        clean_text = comment_re
            .replace_all(&clean_text, |m: &regex::Captures<'_>| {
                " ".repeat(m.get(0).unwrap().as_str().len())
            })
            .to_string();

        // Remove POD sections
        clean_text = pod_re.replace_all(&clean_text, "").to_string();

        // Detect duplicates - group by module name and check for actual duplicates
        let mut module_lines: HashMap<String, Vec<usize>> = HashMap::new();
        for imp in &imports {
            module_lines.entry(imp.stmt.module.clone()).or_default().push(imp.stmt.line);
        }
        let mut duplicate_imports = Vec::new();
        for (module, mut lines) in module_lines {
            if lines.len() > 1 {
                lines.sort();
                duplicate_imports.push(DuplicateImport { module, lines, can_merge: true });
            }
        }

        // Detect unused imports using clean text (without strings/comments)
        let mut unused_imports = Vec::new();
        for imp in &imports {
            let mut unused_symbols = Vec::new();
            if imp.stmt.symbols.is_empty() {
                // For modules without explicit imports, check for common usage patterns
                let mut is_used = false;

                // Check 1: Pragma modules (strict, warnings, etc.) are always considered used
                if is_pragma_module(&imp.stmt.module) {
                    is_used = true;
                }

                if !is_used {
                    // Check 2: Full module name usage (Module::function)
                    let module_pattern = format!(r"\b{}::", regex::escape(&imp.stmt.module));
                    let module_re = Regex::new(&module_pattern).unwrap();
                    if module_re.is_match(&clean_text) {
                        is_used = true;
                    }
                }

                if !is_used {
                    // Check 3: Common known exports for popular modules
                    let known_exports = get_known_module_exports(&imp.stmt.module);
                    for export in known_exports {
                        let export_re =
                            Regex::new(&format!(r"\b{}\b", regex::escape(export))).unwrap();
                        if export_re.is_match(&clean_text) {
                            is_used = true;
                            break;
                        }
                    }
                }

                if !is_used {
                    unused_imports.push(UnusedImport {
                        module: imp.stmt.module.clone(),
                        symbols: vec![],
                        line: imp.stmt.line,
                        reason: "Module not used".to_string(),
                    });
                }
            } else {
                // For explicit symbol imports, check each symbol usage
                for sym in &imp.stmt.symbols {
                    let re = Regex::new(&format!(r"\b{}\b", regex::escape(sym))).unwrap();
                    if !re.is_match(&clean_text) {
                        unused_symbols.push(sym.clone());
                    }
                }
                if !unused_symbols.is_empty() {
                    unused_imports.push(UnusedImport {
                        module: imp.stmt.module.clone(),
                        symbols: unused_symbols,
                        line: imp.stmt.line,
                        reason: "Imported symbols not used".to_string(),
                    });
                }
            }
        }

        // Detect missing imports based on Module::symbol references (using clean text)
        let mut missing_map: HashMap<(String, String), usize> = HashMap::new();
        for caps in module_ref_re.captures_iter(&clean_text) {
            let module = caps[1].to_string();
            let symbol = caps[2].to_string();
            let pos = caps.get(0).unwrap().start();
            let line = clean_text[..pos].lines().count() + 1;

            // Skip built-in Perl classes and common false positives
            if matches!(
                module.as_str(),
                "CORE" | "SUPER" | "UNIVERSAL" | "IO" | "File" | "Cwd" | "HTTP" | "LWP"
            ) {
                continue;
            }

            let import = imports.iter().find(|i| i.stmt.module == module);
            if let Some(imp) = import {
                // If module is imported but specific symbol is not in qw() list
                if !imp.stmt.symbols.is_empty() && !imp.stmt.symbols.contains(&symbol) {
                    missing_map.entry((module.clone(), symbol.clone())).or_insert(line);
                }
            } else {
                // Module not imported at all
                missing_map.entry((module.clone(), symbol.clone())).or_insert(line);
            }
        }

        let mut missing_imports = Vec::new();
        for ((module, symbol), line) in missing_map {
            missing_imports.push(MissingImport {
                module,
                symbols: vec![symbol],
                suggested_location: line,
                confidence: 0.7,
            });
        }

        Ok(ImportAnalysis {
            imports: imports.into_iter().map(|i| i.stmt).collect(),
            unused_imports,
            missing_imports,
            duplicate_imports,
            organization_suggestions: vec![],
        })
    }

    /// Generate optimized import statements based on analysis
    ///
    /// The optimized imports remove unused items, merge duplicates and add
    /// any missing imports detected during analysis.
    pub fn generate_optimized_imports(&self, analysis: &ImportAnalysis) -> String {
        let mut module_map: HashMap<String, Vec<String>> = HashMap::new();

        // Start with current imports
        for imp in &analysis.imports {
            module_map.insert(imp.module.clone(), imp.symbols.clone());
        }

        // Remove unused imports/symbols
        for unused in &analysis.unused_imports {
            if let Some(symbols) = module_map.get_mut(&unused.module) {
                if unused.symbols.is_empty() {
                    module_map.remove(&unused.module);
                } else {
                    symbols.retain(|s| !unused.symbols.contains(s));
                    if symbols.is_empty() {
                        module_map.remove(&unused.module);
                    }
                }
            }
        }

        // Add missing imports
        for missing in &analysis.missing_imports {
            let entry = module_map.entry(missing.module.clone()).or_default();
            for sym in &missing.symbols {
                if !entry.contains(sym) {
                    entry.push(sym.clone());
                }
            }
        }

        // Create sorted import block
        let mut modules: Vec<_> = module_map.into_iter().collect();
        modules.sort_by(|a, b| a.0.cmp(&b.0));

        let mut lines = Vec::new();
        for (module, mut symbols) in modules {
            symbols.sort();
            if symbols.is_empty() {
                lines.push(format!("use {};", module));
            } else {
                lines.push(format!("use {} qw({});", module, symbols.join(" ")));
            }
        }

        lines.join("\n")
    }
}

impl Default for ImportOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_file(content: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let file_path = temp_dir.path().join("test.pl");
        fs::write(&file_path, content).expect("Failed to write test file");
        (temp_dir, file_path)
    }

    #[test]
    fn test_basic_import_analysis() {
        let optimizer = ImportOptimizer::new();
        let content = r#"#!/usr/bin/perl
use strict;
use warnings;
use Data::Dumper;

print Dumper(\@ARGV);
"#;

        let (_temp_dir, file_path) = create_test_file(content);
        let analysis = optimizer.analyze_file(&file_path).expect("Analysis should succeed");

        assert_eq!(analysis.imports.len(), 3);
        assert_eq!(analysis.imports[0].module, "strict");
        assert_eq!(analysis.imports[1].module, "warnings");
        assert_eq!(analysis.imports[2].module, "Data::Dumper");

        // Data::Dumper should not be marked as unused since Dumper is used
        assert!(analysis.unused_imports.is_empty());
    }

    #[test]
    fn test_unused_import_detection() {
        let optimizer = ImportOptimizer::new();
        let content = r#"use strict;
use warnings;
use Data::Dumper;  # This is not used
use JSON;          # This is not used

print "Hello World\n";
"#;

        let (_temp_dir, file_path) = create_test_file(content);
        let analysis = optimizer.analyze_file(&file_path).expect("Analysis should succeed");

        assert_eq!(analysis.unused_imports.len(), 2);
        assert!(analysis.unused_imports.iter().any(|u| u.module == "Data::Dumper"));
        assert!(analysis.unused_imports.iter().any(|u| u.module == "JSON"));
    }

    #[test]
    fn test_missing_import_detection() {
        let optimizer = ImportOptimizer::new();
        let content = r#"use strict;
use warnings;

# Using JSON::encode_json without importing JSON
my $json = JSON::encode_json({key => 'value'});

# Using Data::Dumper::Dumper without importing Data::Dumper  
print Data::Dumper::Dumper(\@ARGV);
"#;

        let (_temp_dir, file_path) = create_test_file(content);
        let analysis = optimizer.analyze_file(&file_path).expect("Analysis should succeed");

        assert_eq!(analysis.missing_imports.len(), 2);
        assert!(analysis.missing_imports.iter().any(|m| m.module == "JSON"));
        assert!(analysis.missing_imports.iter().any(|m| m.module == "Data::Dumper"));
    }

    #[test]
    fn test_duplicate_import_detection() {
        let optimizer = ImportOptimizer::new();
        let content = r#"use strict;
use warnings;
use Data::Dumper;
use JSON;
use Data::Dumper;  # Duplicate

print Dumper(\@ARGV);
"#;

        let (_temp_dir, file_path) = create_test_file(content);
        let analysis = optimizer.analyze_file(&file_path).expect("Analysis should succeed");

        assert_eq!(analysis.duplicate_imports.len(), 1);
        assert_eq!(analysis.duplicate_imports[0].module, "Data::Dumper");
        assert_eq!(analysis.duplicate_imports[0].lines.len(), 2);
        assert!(analysis.duplicate_imports[0].can_merge);
    }

    #[test]
    fn test_qw_import_parsing() {
        let optimizer = ImportOptimizer::new();
        let content = r#"use List::Util qw(first max min sum);
use Scalar::Util qw(blessed reftype);

my @nums = (1, 2, 3, 4, 5);
print "Max: " . max(@nums) . "\n";
print "Sum: " . sum(@nums) . "\n";
print "First: " . first { $_ > 3 } @nums;
"#;

        let (_temp_dir, file_path) = create_test_file(content);
        let analysis = optimizer.analyze_file(&file_path).expect("Analysis should succeed");

        assert_eq!(analysis.imports.len(), 2);

        let list_util = analysis.imports.iter().find(|i| i.module == "List::Util").unwrap();
        assert_eq!(list_util.symbols, vec!["first", "max", "min", "sum"]);

        let scalar_util = analysis.imports.iter().find(|i| i.module == "Scalar::Util").unwrap();
        assert_eq!(scalar_util.symbols, vec!["blessed", "reftype"]);

        // Should detect unused symbols in both modules
        assert_eq!(analysis.unused_imports.len(), 2);

        let list_util_unused =
            analysis.unused_imports.iter().find(|u| u.module == "List::Util").unwrap();
        assert_eq!(list_util_unused.symbols, vec!["min"]);

        let scalar_util_unused =
            analysis.unused_imports.iter().find(|u| u.module == "Scalar::Util").unwrap();
        assert_eq!(scalar_util_unused.symbols, vec!["blessed", "reftype"]);
    }

    #[test]
    fn test_generate_optimized_imports() {
        let optimizer = ImportOptimizer::new();

        let analysis = ImportAnalysis {
            imports: vec![
                ImportStatement { module: "strict".to_string(), symbols: vec![], line: 1 },
                ImportStatement { module: "warnings".to_string(), symbols: vec![], line: 2 },
                ImportStatement {
                    module: "List::Util".to_string(),
                    symbols: vec!["first".to_string(), "max".to_string(), "unused".to_string()],
                    line: 3,
                },
            ],
            unused_imports: vec![UnusedImport {
                module: "List::Util".to_string(),
                symbols: vec!["unused".to_string()],
                line: 3,
                reason: "Symbol not used".to_string(),
            }],
            missing_imports: vec![MissingImport {
                module: "Data::Dumper".to_string(),
                symbols: vec!["Dumper".to_string()],
                suggested_location: 10,
                confidence: 0.8,
            }],
            duplicate_imports: vec![],
            organization_suggestions: vec![],
        };

        let optimized = optimizer.generate_optimized_imports(&analysis);

        // Should be sorted alphabetically
        let expected_lines = [
            "use Data::Dumper qw(Dumper);",
            "use List::Util qw(first max);",
            "use strict;",
            "use warnings;",
        ];

        assert_eq!(optimized, expected_lines.join("\n"));
    }

    #[test]
    fn test_empty_file_analysis() {
        let optimizer = ImportOptimizer::new();
        let content = "";

        let (_temp_dir, file_path) = create_test_file(content);
        let analysis = optimizer.analyze_file(&file_path).expect("Analysis should succeed");

        assert!(analysis.imports.is_empty());
        assert!(analysis.unused_imports.is_empty());
        assert!(analysis.missing_imports.is_empty());
        assert!(analysis.duplicate_imports.is_empty());
    }

    #[test]
    fn test_complex_perl_code_analysis() {
        let optimizer = ImportOptimizer::new();
        let content = r#"#!/usr/bin/perl
use strict;
use warnings;
use Data::Dumper;
use JSON qw(encode_json decode_json);
use LWP::UserAgent;  # Unused
use File::Spec::Functions qw(catfile catdir);

# Complex code with various patterns
my $data = { key => 'value', numbers => [1, 2, 3] };
my $json_string = encode_json($data);
print "JSON: $json_string\n";

# Using File::Spec but not all imported functions
my $path = catfile('/tmp', 'test.json');
print "Path: $path\n";

# Using modules without explicit imports
my $response = HTTP::Tiny::new()->get('http://example.com');
print Dumper($response);
"#;

        let (_temp_dir, file_path) = create_test_file(content);
        let analysis = optimizer.analyze_file(&file_path).expect("Analysis should succeed");

        // Should detect unused imports
        assert!(analysis.unused_imports.iter().any(|u| u.module == "LWP::UserAgent"));

        // Should detect unused symbols from File::Spec::Functions
        let file_spec_unused =
            analysis.unused_imports.iter().find(|u| u.module == "File::Spec::Functions");
        if let Some(unused) = file_spec_unused {
            assert!(unused.symbols.contains(&"catdir".to_string()));
        }

        // Should detect missing import for HTTP::Tiny
        assert!(analysis.missing_imports.iter().any(|m| m.module == "HTTP::Tiny"));
    }

    #[test]
    fn test_regex_edge_cases() {
        let optimizer = ImportOptimizer::new();
        let content = r#"use strict;
use warnings;

# These should not be detected as module references
my $string = "This is not JSON::encode_json in a string";
my $regex = qr/Data::Dumper/;
print "Module::Name is just text";

# This should be detected
my $result = JSON::encode_json({test => 1});
"#;

        let (_temp_dir, file_path) = create_test_file(content);
        let analysis = optimizer.analyze_file(&file_path).expect("Analysis should succeed");

        // Should only detect the actual module usage, not the ones in strings/regex
        assert_eq!(analysis.missing_imports.len(), 1);
        assert_eq!(analysis.missing_imports[0].module, "JSON");
    }
}
