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
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Result of import analysis containing all detected issues and suggestions
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportAnalysis {
    /// Import statements with unused symbols
    pub unused_imports: Vec<UnusedImport>,
    /// Symbols that are used but not imported
    pub missing_imports: Vec<MissingImport>,
    /// Modules that are imported multiple times
    pub duplicate_imports: Vec<DuplicateImport>,
    /// Suggestions for organizing imports
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

/// A symbol that is used but not imported
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

/// A suggestion for improving import organization
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
fn get_known_module_exports(module: &str) -> Option<Vec<&'static str>> {
    match module {
        "Data::Dumper" => Some(vec!["Dumper"]),
        "JSON" => Some(vec!["encode_json", "decode_json", "to_json", "from_json"]),
        "YAML" => Some(vec!["Load", "Dump", "LoadFile", "DumpFile"]),
        "Storable" => Some(vec!["store", "retrieve", "freeze", "thaw"]),
        "List::Util" => Some(vec!["first", "max", "min", "sum", "reduce", "shuffle", "uniq"]),
        "Scalar::Util" => Some(vec!["blessed", "reftype", "looks_like_number", "weaken"]),
        "File::Spec" => Some(vec!["catfile", "catdir", "splitpath", "splitdir"]),
        "File::Basename" => Some(vec!["basename", "dirname", "fileparse"]),
        "Cwd" => Some(vec!["getcwd", "abs_path", "realpath"]),
        "Time::HiRes" => Some(vec!["time", "sleep", "usleep", "gettimeofday"]),
        "Digest::MD5" => Some(vec!["md5", "md5_hex", "md5_base64"]),
        "MIME::Base64" => Some(vec!["encode_base64", "decode_base64"]),
        "URI::Escape" => Some(vec!["uri_escape", "uri_unescape"]),
        "LWP::Simple" => Some(vec!["get", "head", "getprint", "getstore", "mirror"]),
        "LWP::UserAgent" => Some(vec![]),
        "CGI" => Some(vec!["param", "header", "start_html", "end_html"]),
        "DBI" => Some(vec![]),    // DBI is object-oriented, no default exports
        "strict" => Some(vec![]), // Pragma, no exports
        "warnings" => Some(vec![]), // Pragma, no exports
        "utf8" => Some(vec![]),   // Pragma, no exports
        _ => None,
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
        self.analyze_content(&content)
    }

    /// Analyze imports in Perl content and detect issues
    ///
    /// This method:
    /// - Parses basic `use Module qw(symbols)` statements using regex
    /// - Detects unused symbols by checking if they appear in the code
    /// - Identifies duplicate imports of the same module
    /// - Returns a comprehensive analysis with all findings
    ///
    /// # Arguments
    ///
    /// * `content` - The Perl source code content to analyze
    ///
    /// # Returns
    ///
    /// Returns `ImportAnalysis` with detected issues or an error string if analysis fails.
    ///
    /// # Limitations
    ///
    /// - Only supports simple qw() syntax
    /// - Does not handle complex import patterns
    /// - Symbol usage detection is basic regex matching
    pub fn analyze_content(&self, content: &str) -> Result<ImportAnalysis, String> {
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

        // Pre-compile regex for special Data::Dumper case
        let dumper_re = Regex::new(r"\bDumper\b").map_err(|e| e.to_string())?;

        // Determine unused symbols for each import entry
        let mut unused_imports = Vec::new();
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
                    // For bare imports (without qw()), check if the module or any of its known exports are used
                    let (_is_known_module, known_exports) =
                        match get_known_module_exports(&imp.module) {
                            Some(exports) => (true, exports),
                            None => (false, Vec::new()),
                        };
                    let mut is_used = false;

                    // First check if the module is directly referenced (e.g., Module::function)
                    let module_pattern = format!(r"\b{}\b", regex::escape(&imp.module));
                    let module_re = Regex::new(&module_pattern).map_err(|e| e.to_string())?;
                    if module_re.is_match(&non_use_content) {
                        is_used = true;
                    }

                    // Also check for qualified function calls like Module::function
                    if !is_used {
                        let qualified_pattern = format!(r"{}::", regex::escape(&imp.module));
                        let qualified_re =
                            Regex::new(&qualified_pattern).map_err(|e| e.to_string())?;
                        if qualified_re.is_match(&non_use_content) {
                            is_used = true;
                        }
                    }

                    // Special handling for Data::Dumper - check for Dumper function usage
                    if !is_used && imp.module == "Data::Dumper" {
                        if dumper_re.is_match(&non_use_content) {
                            is_used = true;
                        }
                    }

                    // Then check if any known exports are used
                    if !is_used && !known_exports.is_empty() {
                        for export in &known_exports {
                            let export_pattern = format!(r"\b{}\b", regex::escape(export));
                            let export_re =
                                Regex::new(&export_pattern).map_err(|e| e.to_string())?;
                            if export_re.is_match(&non_use_content) {
                                is_used = true;
                                break;
                            }
                        }
                    }

                    // For bare imports, be conservative - many modules have side effects
                    // Only mark as unused if we're confident it's safe to remove
                    if !is_used && _is_known_module {
                        // If the module has no known exports (empty vec), it's likely object-oriented
                        // and safe to mark as unused if not used
                        if known_exports.is_empty() {
                            unused_symbols.push("(bare import)".to_string());
                        }
                        // For modules with exports, be conservative and don't mark as unused
                        // since they might have side effects
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

        // Missing import detection
        let imported_modules: BTreeSet<String> =
            imports.iter().map(|imp| imp.module.clone()).collect();

        // Strip strings and comments before scanning for Module::symbol patterns
        let string_re = Regex::new("'[^']*'|\"[^\"]*\"").map_err(|e| e.to_string())?;
        let stripped = string_re.replace_all(content, " ").to_string();
        let regex_literal_re = Regex::new(r"qr/[^/]*/").map_err(|e| e.to_string())?;
        let stripped = regex_literal_re.replace_all(&stripped, " ").to_string();
        let comment_re = Regex::new(r"(?m)#.*$").map_err(|e| e.to_string())?;
        let stripped = comment_re.replace_all(&stripped, " ").to_string();

        let usage_re = Regex::new(
            r"\b([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)::([A-Za-z_][A-Za-z0-9_]*)",
        )
        .map_err(|e| e.to_string())?;
        let mut usage_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for caps in usage_re.captures_iter(&stripped) {
            let module = caps.get(1).unwrap().as_str().to_string();
            let symbol = caps.get(2).unwrap().as_str().to_string();

            if imported_modules.contains(&module) || is_pragma_module(&module) {
                continue;
            }

            usage_map.entry(module).or_default().push(symbol);
        }
        let last_import_line = imports.iter().map(|i| i.line).max().unwrap_or(0);
        let missing_imports = usage_map
            .into_iter()
            .map(|(module, mut symbols)| {
                symbols.sort();
                symbols.dedup();
                MissingImport {
                    module,
                    symbols,
                    suggested_location: last_import_line + 1,
                    confidence: 0.8,
                }
            })
            .collect::<Vec<_>>();

        // Generate organization suggestions
        let mut organization_suggestions = Vec::new();

        // Suggest sorting of import statements
        let module_order: Vec<String> = imports.iter().map(|i| i.module.clone()).collect();
        let mut sorted_order = module_order.clone();
        sorted_order.sort();
        if module_order != sorted_order {
            organization_suggestions.push(OrganizationSuggestion {
                description: "Sort import statements alphabetically".to_string(),
                priority: SuggestionPriority::Low,
            });
        }

        // Suggest removing duplicate imports
        if !duplicate_imports.is_empty() {
            let modules =
                duplicate_imports.iter().map(|d| d.module.clone()).collect::<Vec<_>>().join(", ");
            organization_suggestions.push(OrganizationSuggestion {
                description: format!("Remove duplicate imports for modules: {}", modules),
                priority: SuggestionPriority::Medium,
            });
        }

        // Suggest sorting/deduplicating symbols within imports
        let mut symbols_need_org = false;
        for imp in &imports {
            if imp.symbols.len() > 1 {
                let mut sorted = imp.symbols.clone();
                sorted.sort();
                sorted.dedup();
                if sorted != imp.symbols {
                    symbols_need_org = true;
                    break;
                }
            }
        }
        if symbols_need_org {
            organization_suggestions.push(OrganizationSuggestion {
                description: "Sort and deduplicate symbols within import statements".to_string(),
                priority: SuggestionPriority::Low,
            });
        }

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

        // With improved logic, bare imports without explicit symbols are treated conservatively.
        // Modules with exports are not reported as unused to prevent breaking side effects.
        // Only object-oriented modules (no exports) may be reported as unused.
        assert!(analysis.unused_imports.is_empty());
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
        for m in &analysis.missing_imports {
            assert_eq!(m.suggested_location, 3);
        }
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
    fn test_organization_suggestions() {
        let optimizer = ImportOptimizer::new();
        let content = r#"use warnings;
use strict;
use List::Util qw(max max min);
use Data::Dumper;
use Data::Dumper;  # duplicate
"#;

        let (_temp_dir, file_path) = create_test_file(content);
        let analysis = optimizer.analyze_file(&file_path).expect("Analysis should succeed");

        assert!(
            analysis
                .organization_suggestions
                .iter()
                .any(|s| s.description.contains("Sort import statements"))
        );
        assert!(
            analysis
                .organization_suggestions
                .iter()
                .any(|s| s.description.contains("Remove duplicate imports"))
        );
        assert!(
            analysis
                .organization_suggestions
                .iter()
                .any(|s| s.description.contains("Sort and deduplicate symbols"))
        );
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
    fn test_bare_import_with_exports_detection() {
        let optimizer = ImportOptimizer::new();
        let content = r#"use strict;
use warnings;
use Data::Dumper;  # Used
use JSON;          # Unused - has exports but none are used
use SomeUnknownModule;  # Conservative - not marked as unused

print Dumper(\@ARGV);
"#;

        let (_temp_dir, file_path) = create_test_file(content);
        let analysis = optimizer.analyze_file(&file_path).expect("Analysis should succeed");

        // Data::Dumper should not be unused (Dumper is used)
        assert!(!analysis.unused_imports.iter().any(|u| u.module == "Data::Dumper"));

        // JSON and SomeUnknownModule are treated conservatively - modules with exports
        // are not flagged as unused to prevent breaking side effects.
        assert!(analysis.unused_imports.is_empty());
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
