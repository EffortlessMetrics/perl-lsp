//! Import optimization for Perl modules (stub implementation)
//!
//! This module analyzes import statements and usage to optimize imports.
//! Currently a stub implementation to demonstrate the architecture.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Result of import analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportAnalysis {
    pub unused_imports: Vec<UnusedImport>,
    pub missing_imports: Vec<MissingImport>,
    pub duplicate_imports: Vec<DuplicateImport>,
    pub organization_suggestions: Vec<OrganizationSuggestion>,
    /// All imports discovered in the file
    pub imports: Vec<ImportEntry>,
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

/// Single import statement discovered during analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportEntry {
    pub module: String,
    pub symbols: Vec<String>,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestionPriority {
    High,
    Medium,
    Low,
}

/// Import optimizer
pub struct ImportOptimizer;

impl ImportOptimizer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze imports in a file. This is a very small-scale parser that
    /// understands a subset of `use` statements of the form
    /// `use Module qw(sym1 sym2);` and tracks symbol usage.
    pub fn analyze_file(&self, file_path: &Path) -> Result<ImportAnalysis, String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| e.to_string())?;

        // Regex for basic `use` statement parsing
        let re_use =
            Regex::new(r"^\s*use\s+([A-Za-z0-9_:]+)(?:\s+qw\(([^)]*)\))?\s*;")
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
                imports.push(ImportEntry {
                    module,
                    symbols,
                    line: idx + 1,
                });
            }
        }

        // Build map for duplicate detection
        let mut module_to_lines: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for imp in &imports {
            module_to_lines
                .entry(imp.module.clone())
                .or_default()
                .push(imp.line);
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
            .filter(|line| !line.trim_start().starts_with("use "))
            .collect::<Vec<_>>()
            .join("\n");

        // Determine unused symbols for each import entry
        let mut unused_imports = Vec::new();
        for imp in &imports {
            let mut unused_symbols = Vec::new();
            for sym in &imp.symbols {
                let re = Regex::new(&format!(r"\b{}\b", regex::escape(sym)))
                    .map_err(|e| e.to_string())?;
                if !re.is_match(&non_use_content) {
                    unused_symbols.push(sym.clone());
                }
            }
            if !unused_symbols.is_empty() {
                unused_imports.push(UnusedImport {
                    module: imp.module.clone(),
                    symbols: unused_symbols,
                    line: imp.line,
                    reason: "unused".to_string(),
                });
            }
        }

        Ok(ImportAnalysis {
            unused_imports,
            missing_imports: vec![],
            duplicate_imports,
            organization_suggestions: vec![],
            imports,
        })
    }

    /// Generate optimized import statements by consolidating duplicate
    /// imports and removing unused symbols.
    pub fn generate_optimized_imports(&self, analysis: &ImportAnalysis) -> String {
        // Map module -> set of used symbols
        let mut modules: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for imp in &analysis.imports {
            // Determine which symbols are unused for this particular line
            let unused = analysis
                .unused_imports
                .iter()
                .find(|u| u.module == imp.module && u.line == imp.line);

            let mut symbols = imp.symbols.clone();
            if let Some(u) = unused {
                symbols.retain(|s| !u.symbols.contains(s));
            }

            if symbols.is_empty() && unused.is_some() {
                // Entire import unused – skip
                continue;
            }

            modules
                .entry(imp.module.clone())
                .or_default()
                .extend(symbols.into_iter());
        }

        // Build final import statements sorted by module name
        let mut result = String::new();
        for (module, symbols) in modules {
            if symbols.is_empty() {
                result.push_str(&format!("use {};&\n", module));
            } else {
                let sym_vec: Vec<_> = symbols.into_iter().collect();
                result.push_str(&format!("use {} qw({});\n", module, sym_vec.join(" ")));
            }
        }
        result
    }
}

impl Default for ImportOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
