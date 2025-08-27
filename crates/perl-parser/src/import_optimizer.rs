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

        // Regex to capture simple `use Module qw(foo bar);` statements.
        let import_re =
            Regex::new(r"(?m)^[ \t]*use\s+([A-Za-z_][\w:]*)\s*(?:qw\(([^)]*)\))?;\s*$")
                .unwrap();
        // Regex to capture `Module::symbol` references
        let module_ref_re =
            Regex::new(r"([A-Za-z_][\w:]*)::([A-Za-z_][A-Za-z0-9_]*)").unwrap();

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
            let symbols = caps
                .get(2)
                .map(|m| {
                    m.as_str()
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();
            let line = text[..m.start()].lines().count() + 1;
            imports.push(ImportLine {
                stmt: ImportStatement {
                    module,
                    symbols,
                    line,
                },
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

        // Detect duplicates
        let mut module_lines: HashMap<String, Vec<usize>> = HashMap::new();
        for imp in &imports {
            module_lines
                .entry(imp.stmt.module.clone())
                .or_default()
                .push(imp.stmt.line);
        }
        let mut duplicate_imports = Vec::new();
        for (module, lines) in module_lines {
            if lines.len() > 1 {
                duplicate_imports.push(DuplicateImport {
                    module,
                    lines,
                    can_merge: true,
                });
            }
        }

        // Detect unused imports
        let mut unused_imports = Vec::new();
        for imp in &imports {
            let mut unused_symbols = Vec::new();
            if imp.stmt.symbols.is_empty() {
                let re = Regex::new(&format!("\\b{}\\b", regex::escape(&imp.stmt.module)))
                    .unwrap();
                if !re.is_match(&text_without_imports) {
                    unused_imports.push(UnusedImport {
                        module: imp.stmt.module.clone(),
                        symbols: vec![],
                        line: imp.stmt.line,
                        reason: "Module not used".to_string(),
                    });
                }
            } else {
                for sym in &imp.stmt.symbols {
                    let re =
                        Regex::new(&format!("\\b{}\\b", regex::escape(sym))).unwrap();
                    if !re.is_match(&text_without_imports) {
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

        // Detect missing imports based on Module::symbol references
        let mut missing_map: HashMap<(String, String), usize> = HashMap::new();
        for caps in module_ref_re.captures_iter(&text_without_imports) {
            let module = caps[1].to_string();
            let symbol = caps[2].to_string();
            let pos = caps.get(0).unwrap().start();
            let line = text_without_imports[..pos].lines().count() + 1;
            let import = imports.iter().find(|i| i.stmt.module == module);
            if let Some(imp) = import {
                if !imp.stmt.symbols.is_empty() && !imp.stmt.symbols.contains(&symbol) {
                    missing_map.entry((module.clone(), symbol.clone())).or_insert(line);
                }
            } else {
                missing_map.entry((module.clone(), symbol.clone())).or_insert(line);
            }
        }

        let mut missing_imports = Vec::new();
        for ((module, symbol), line) in missing_map {
            missing_imports.push(MissingImport {
                module,
                symbols: vec![symbol],
                suggested_location: line,
                confidence: 0.5,
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
            let entry = module_map
                .entry(missing.module.clone())
                .or_default();
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
