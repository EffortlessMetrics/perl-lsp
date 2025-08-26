//! Import optimization for Perl modules
//!
//! The previous version of this module only contained empty stubs.  The
//! implementation below performs a very small amount of static analysis on a
//! source file: duplicate and unused imports are detected, missing imports are
//! suggested and a basic suggestion for reorganisation is produced.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Result of import analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportAnalysis {
    pub unused_imports: Vec<UnusedImport>,
    pub missing_imports: Vec<MissingImport>,
    pub duplicate_imports: Vec<DuplicateImport>,
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

    /// Analyze imports in a file.
    pub fn analyze_file(&self, file_path: &Path) -> Result<ImportAnalysis, String> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?;

        let lines: Vec<&str> = content.lines().collect();
        let use_re = Regex::new(r"^\s*use\s+([A-Za-z0-9_:]+)(?:\s+qw\(([^)]*)\))?")
            .map_err(|e| e.to_string())?;

        let mut module_lines: HashMap<String, Vec<usize>> = HashMap::new();
        let mut module_symbols: HashMap<String, Vec<String>> = HashMap::new();

        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = use_re.captures(line) {
                let module = caps.get(1).unwrap().as_str().to_string();
                let symbols = caps
                    .get(2)
                    .map(|m| {
                        m.as_str().split_whitespace().map(|s| s.to_string()).collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                module_lines.entry(module.clone()).or_default().push(i + 1);
                if !symbols.is_empty() {
                    module_symbols.insert(module.clone(), symbols);
                }
            }
        }

        // Detect duplicate imports
        let mut duplicate_imports = Vec::new();
        for (module, lines) in &module_lines {
            if lines.len() > 1 {
                duplicate_imports.push(DuplicateImport {
                    module: module.clone(),
                    lines: lines.clone(),
                    can_merge: true,
                });
            }
        }

        // Detect unused imports by searching for module or imported symbols
        let mut unused_imports = Vec::new();
        for (module, lines_vec) in &module_lines {
            let mut used = false;
            let module_re = Regex::new(&format!(r"\b{}::", regex::escape(module))).unwrap();
            if module_re.is_match(&content) {
                for m in module_re.find_iter(&content) {
                    // Ignore the import lines themselves
                    let line = content[..m.start()].lines().count() + 1;
                    if !lines_vec.contains(&line) {
                        used = true;
                        break;
                    }
                }
            }

            if !used {
                if let Some(symbols) = module_symbols.get(module) {
                    let symbol_used = symbols.iter().any(|s| {
                        Regex::new(&format!(r"\\b{}\\b", regex::escape(s)))
                            .unwrap()
                            .is_match(&content)
                    });
                    used = symbol_used;
                }
            }

            if !used {
                unused_imports.push(UnusedImport {
                    module: module.clone(),
                    symbols: module_symbols.get(module).cloned().unwrap_or_default(),
                    line: lines_vec[0],
                    reason: "Import not used".into(),
                });
            }
        }

        // Detect missing imports by scanning for module paths followed by `::symbol`
        let usage_re = Regex::new(r"([A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)*)::[A-Za-z0-9_]+").unwrap();
        let mut imported: HashSet<String> = module_lines.keys().cloned().collect();
        let mut missing_imports = Vec::new();
        for cap in usage_re.captures_iter(&content) {
            let module = cap[1].to_string();
            if !imported.contains(&module) {
                imported.insert(module.clone());
                missing_imports.push(MissingImport {
                    module,
                    symbols: vec![],
                    suggested_location: 1,
                    confidence: 0.5,
                });
            }
        }

        let organization_suggestions = if !duplicate_imports.is_empty()
            || !unused_imports.is_empty()
            || !missing_imports.is_empty()
        {
            vec![OrganizationSuggestion {
                description: "Sort and deduplicate imports".into(),
                priority: SuggestionPriority::Medium,
            }]
        } else {
            Vec::new()
        };

        Ok(ImportAnalysis {
            unused_imports,
            missing_imports,
            duplicate_imports,
            organization_suggestions,
        })
    }

    /// Generate optimized import statements based on analysis.
    ///
    /// The returned string contains one `use` line per module, sorted
    /// alphabetically. Unused imports are omitted and missing imports are
    /// included.
    pub fn generate_optimized_imports(&self, analysis: &ImportAnalysis) -> String {
        let mut modules: HashSet<String> =
            analysis.duplicate_imports.iter().map(|d| d.module.clone()).collect();

        for miss in &analysis.missing_imports {
            modules.insert(miss.module.clone());
        }

        // Unused imports are intentionally not added back
        let mut modules: Vec<String> = modules.into_iter().collect();
        modules.sort();
        modules.into_iter().map(|m| format!("use {}\n", m)).collect()
    }
}

impl Default for ImportOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
