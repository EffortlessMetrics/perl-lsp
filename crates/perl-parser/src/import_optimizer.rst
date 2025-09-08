//! Import optimization for Perl modules

use regex::Regex;
use std::collections::BTreeMap;
use std::path::Path;
use std::fs::read_to_string;

pub struct ImportAnalysis {
    pub unused_imports: Vec<UnusedImport>,
    pub duplicate_imports: Vec<DuplicateImport>,
    pub imports: Vec<ImportEntry>,
}

#[derive(Clone)]
pub struct UnusedImport {
    pub module: String,
    pub symbols: Vec<String>,
    pub line: usize,
}

#[derive(Clone)]
pub struct DuplicateImport {
    pub module: String,
    pub lines: Vec<usize>,
}

#[derive(Clone)]
pub struct ImportEntry {
    pub module: String,
    pub symbols: Vec<String>,
    pub line: usize,
}

pub struct ImportOptimizer;

impl ImportOptimizer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_file(&self, file_path: &Path) -> Result<ImportAnalysis, String> {
        let content = read_to_string(file_path).map_err(|e| e.to_string())?;

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
            })
            .collect::<Vec<_>>();

        let non_use_content = content
            .lines()
            .filter(|line| 
                !line.trim_start().starts_with("use ") && 
                !line.trim_start().starts_with("#")
            )
            .collect::<Vec<_>>()
            .join("\n");

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
                });
            }
        }

        Ok(ImportAnalysis {
            unused_imports,
            duplicate_imports,
            imports,
        })
    }

    pub fn generate_optimized_imports(&self, analysis: &ImportAnalysis) -> String {
        let mut used_imports: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for imp in &analysis.imports {
            if !imp.symbols.is_empty() {
                let used_symbols: Vec<String> = imp.symbols
                    .iter()
                    .filter(|sym| 
                        !analysis.unused_imports.iter()
                            .any(|u| u.module == imp.module && u.symbols.contains(sym))
                    )
                    .cloned()
                    .collect();

                if !used_symbols.is_empty() {
                    used_imports.insert(imp.module.clone(), used_symbols);
                }
            }
        }

        let bare_imports: Vec<String> = analysis.imports
            .iter()
            .filter(|imp| imp.symbols.is_empty())
            .map(|imp| format!("use {};", imp.module))
            .collect();

        let mut result = Vec::new();
        result.extend(bare_imports);
        result.extend(used_imports.iter().map(|(module, symbols)| 
            format!("use {} qw({});", module, symbols.join(" "))
        ));

        result.join("\n")
    }
}

impl Default for ImportOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
