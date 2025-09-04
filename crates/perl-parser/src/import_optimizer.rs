//! Import optimization for Perl modules
//!
//! This module analyzes import statements and usage to optimize imports by:
//! - Detecting unused imports and symbols
//! - Finding duplicate import statements
//! - Consolidating imports to reduce clutter
//! - Generating optimized import statements
//!
//! ## Example
//!
//! ```rust
//! use perl_parser::import_optimizer::ImportOptimizer;
//! use std::path::Path;
//!
//! let optimizer = ImportOptimizer::new();
//! let analysis = optimizer.analyze_file(Path::new("script.pl"))?;
//! let optimized_imports = optimizer.generate_optimized_imports(&analysis);
//! println!("{}", optimized_imports);
//! # Ok::<(), String>(())
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Result of import analysis containing all detected issues and suggestions
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportAnalysis {
    /// Import statements with unused symbols
    pub unused_imports: Vec<UnusedImport>,
    /// Symbols that are used but not imported (currently empty - future enhancement)
    pub missing_imports: Vec<MissingImport>,
    /// Modules that are imported multiple times
    pub duplicate_imports: Vec<DuplicateImport>,
    /// Suggestions for organizing imports (currently empty - future enhancement)
    pub organization_suggestions: Vec<OrganizationSuggestion>,
    /// All imports discovered in the file
    pub imports: Vec<ImportEntry>,
}

/// An import statement containing unused symbols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnusedImport {
    /// Module name
    pub module: String,
    /// List of unused symbols from this import
    pub symbols: Vec<String>,
    /// Line number where this import statement appears (1-indexed)
    pub line: usize,
    /// Reason why symbols are considered unused
    pub reason: String,
}

/// A symbol that is used but not imported (future enhancement)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingImport {
    /// Module name that should be imported
    pub module: String,
    /// List of symbols that need to be imported
    pub symbols: Vec<String>,
    /// Suggested line number to insert the import
    pub suggested_location: usize,
    /// Confidence level of the suggestion (0.0 to 1.0)
    pub confidence: f32,
}

/// A module that is imported multiple times
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateImport {
    /// Module name that is duplicated
    pub module: String,
    /// Line numbers where this module is imported (1-indexed)
    pub lines: Vec<usize>,
    /// Whether these imports can be safely merged
    pub can_merge: bool,
}

/// A suggestion for improving import organization (future enhancement)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSuggestion {
    /// Human-readable description of the suggestion
    pub description: String,
    /// Priority level of this suggestion
    pub priority: SuggestionPriority,
}

/// A single import statement discovered during analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportEntry {
    /// Module name
    pub module: String,
    /// List of imported symbols (empty for bare imports)
    pub symbols: Vec<String>,
    /// Line number where this import appears (1-indexed)
    pub line: usize,
}

/// Priority level for organization suggestions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestionPriority {
    /// High priority - should be addressed immediately
    High,
    /// Medium priority - should be addressed when convenient
    Medium,
    /// Low priority - can be addressed later
    Low,
}

/// Import optimizer for analyzing and optimizing Perl import statements
///
/// The optimizer currently supports:
/// - Parsing basic `use Module qw(symbols)` statements
/// - Detecting unused imported symbols
/// - Finding duplicate imports that can be merged
/// - Generating consolidated import statements
pub struct ImportOptimizer;

/// Check if a module is a pragma (affects compilation, no exports)
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    /// Create a new import optimizer instance
    pub fn new() -> Self {
        Self
    }

    /// Analyze imports in a Perl file and detect issues
    ///
    /// This method:
    /// - Parses basic `use Module qw(symbols)` statements using regex
    /// - Detects unused symbols by checking if they appear in the code
    /// - Identifies duplicate imports of the same module
    /// - Returns a comprehensive analysis with all findings
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the Perl file to analyze
    ///
    /// # Returns
    ///
    /// Returns `ImportAnalysis` with detected issues or an error string if the file cannot be read.
    ///
    /// # Limitations
    ///
    /// - Only supports simple qw() syntax
    /// - Does not handle complex import patterns
    /// - Symbol usage detection is basic regex matching
    pub fn analyze_file(&self, file_path: &Path) -> Result<ImportAnalysis, String> {
        let content = std::fs::read_to_string(file_path).map_err(|e| e.to_string())?;

        // Regex for basic `use` statement parsing
        let re_use = Regex::new(r"^\s*use\s+([A-Za-z0-9_:]+)(?:\s+qw\(([^)]*)\))?\s*;")
            .map_err(|e| e.to_string())?;

        let mut imports = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            if let Some(caps) = re_use.captures(line) {
                let module = caps[1].to_string();
                let symbols_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let symbols = if symbols_str.is_empty() {
                    Vec::new()
                } else {
                    symbols_str
                        .split_whitespace()
                        .filter(|s| !s.is_empty())
                        .map(|s| s.trim_matches(|c| c == ',' || c == ';' || c == '"'))
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                };
                imports.push(ImportEntry { module, symbols, line: idx + 1 });
            }
        }

        // Build map for duplicate detection
        let mut module_to_lines: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for imp in &imports {
            module_to_lines.entry(imp.module.clone()).or_default().push(imp.line);
        }
        let duplicate_imports = module_to_lines
            .iter()
            .filter(|(_, lines)| lines.len() > 1)
            .map(|(module, lines)| DuplicateImport {
                module: module.clone(),
                lines: lines.clone(),
                can_merge: true,
            })
            .collect::<Vec<_>>();

        // Build content without `use` lines for symbol usage detection
        let non_use_content = content
            .lines()
            .filter(
                |line| {
                    !line.trim_start().starts_with("use ") && !line.trim_start().starts_with("#")
                }, // Exclude comment lines
            )
            .collect::<Vec<_>>()
            .join(
                "
",
            );

        // Determine unused symbols for each import entry
        let mut unused_imports = Vec::new();
        let dumper_re = Regex::new(r"\bDumper\b").map_err(|e| e.to_string())?;
        for imp in &imports {
            let mut unused_symbols = Vec::new();

            // If there are explicit symbols (like qw()), check each one
            if !imp.symbols.is_empty() {
                for sym in &imp.symbols {
                    let re = Regex::new(&format!(r"\b{}\b", regex::escape(sym)))
                        .map_err(|e| e.to_string())?;

                    // Check if symbol is used in non-use content
                    if !re.is_match(&non_use_content) {
                        unused_symbols.push(sym.clone());
                    }
                }
            } else {
                // Skip pragma modules like strict, warnings, etc.
                let is_pragma = matches!(
                    imp.module.as_str(),
                    "strict"
                        | "warnings"
                        | "utf8"
                        | "bytes"
                        | "integer"
                        | "locale"
                        | "overload"
                        | "sigtrap"
                        | "subs"
                        | "vars"
                );

                if !is_pragma {
                    // For bare imports of non-pragma modules, check if module is used
                    let module_pattern = format!(r"\b{}\b", regex::escape(&imp.module));
                    let re = Regex::new(&module_pattern).map_err(|e| e.to_string())?;

                    // Also check for qualified function calls like Module::function
                    let qualified_pattern = format!(r"{}::", regex::escape(&imp.module));
                    let qualified_re = Regex::new(&qualified_pattern).map_err(|e| e.to_string())?;

                    // Special handling for Data::Dumper - check for Dumper function usage
                    let is_used = if imp.module == "Data::Dumper" {
                        dumper_re.is_match(&non_use_content)
                    } else {
                        re.is_match(&non_use_content) || qualified_re.is_match(&non_use_content)
                    };

                    if !is_used {
                        // Mark the entire module as unused
                        unused_imports.push(UnusedImport {
                            module: imp.module.clone(),
                            symbols: vec![], // Empty symbols for bare import
                            line: imp.line,
                            reason: "Module not used in code".to_string(),
                        });
                    }
                }
            }

            // Create unused import entry if there are unused symbols
            if !unused_symbols.is_empty() {
                unused_imports.push(UnusedImport {
                    module: imp.module.clone(),
                    symbols: unused_symbols,
                    line: imp.line,
                    reason: "Symbols not used in code".to_string(),
                });
            }
        }

        // TODO: Implement missing import detection
        let missing_imports = Vec::new();

        // TODO: Implement organization suggestions
        let organization_suggestions = Vec::new();

        Ok(ImportAnalysis {
            imports,
            unused_imports,
            missing_imports,
            duplicate_imports,
            organization_suggestions,
        })
    }

    /// Generate optimized import statements from analysis results
    ///
    /// This method takes the results of import analysis and generates
    /// a cleaned up version of the imports with:
    /// - Unused symbols removed
    /// - Missing imports added
    /// - Duplicates consolidated
    /// - Alphabetical ordering
    ///
    /// # Arguments
    ///
    /// * `analysis` - The import analysis results
    ///
    /// # Returns
    ///
    /// A string containing the optimized import statements, one per line
    pub fn generate_optimized_imports(&self, analysis: &ImportAnalysis) -> String {
        let mut optimized_imports = Vec::new();

        // Create a map to track which modules we want to keep and their symbols
        let mut module_symbols: BTreeMap<String, Vec<String>> = BTreeMap::new();

        // Get a list of all unused symbols per module
        let mut unused_by_module: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for unused in &analysis.unused_imports {
            unused_by_module
                .entry(unused.module.clone())
                .or_default()
                .extend(unused.symbols.clone());
        }

        // Process existing imports, consolidating duplicates and removing unused symbols
        for import in &analysis.imports {
            // Keep only symbols that are not unused
            let kept_symbols: Vec<String> = import
                .symbols
                .iter()
                .filter(|sym| {
                    if let Some(unused_symbols) = unused_by_module.get(&import.module) {
                        !unused_symbols.contains(sym)
                    } else {
                        true // Keep all symbols if no unused symbols found for this module
                    }
                })
                .cloned()
                .collect();

            // Add to module_symbols map (this automatically consolidates duplicates)
            let entry = module_symbols.entry(import.module.clone()).or_default();
            entry.extend(kept_symbols);

            // Remove duplicates and sort for consistency
            entry.sort();
            entry.dedup();
        }

        // Add missing imports
        for missing in &analysis.missing_imports {
            let entry = module_symbols.entry(missing.module.clone()).or_default();
            entry.extend(missing.symbols.clone());
            entry.sort();
            entry.dedup();
        }

        // Generate import statements - only include modules that have symbols to import
        // or are bare imports (originally had empty symbols)
        for (module, symbols) in &module_symbols {
            // Check if this was originally a bare import by seeing if any original import had empty symbols
            let was_bare_import =
                analysis.imports.iter().any(|imp| imp.module == *module && imp.symbols.is_empty());

            if symbols.is_empty() && was_bare_import {
                // Bare import (like 'use strict;')
                optimized_imports.push(format!("use {};", module));
            } else if !symbols.is_empty() {
                // Import with symbols
                let symbol_list = symbols.join(" ");
                optimized_imports.push(format!("use {} qw({});", module, symbol_list));
            }
            // Skip modules with no symbols that weren't originally bare imports (all symbols were unused)
        }

        // Sort alphabetically for consistency
        optimized_imports.sort();
        optimized_imports.join("\n")
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
    #[ignore = "Missing import detection not yet implemented"]
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
                ImportEntry { module: "strict".to_string(), symbols: vec![], line: 1 },
                ImportEntry { module: "warnings".to_string(), symbols: vec![], line: 2 },
                ImportEntry {
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
    #[ignore = "Missing import detection not yet implemented"]
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
    #[ignore = "Missing import detection not yet implemented"]
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
