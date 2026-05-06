//! Dead code detection for Perl codebases (stub implementation)
//!
//! This module identifies unused code including unreachable code and unused symbols.
//! Currently a stub implementation to demonstrate the architecture.

use perl_workspace::workspace_index::{fs_path_to_uri, uri_to_fs_path, SymbolKind, WorkspaceIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

mod dead_branches;
mod report;

pub use report::generate_report;

use crate::dead_branches::detect_dead_branches;

fn has_statement_modifier(line: &str) -> bool {
    // Perl statement modifiers keep flow-control statements conditional,
    // e.g. `next if $skip;` should not terminate all subsequent flow.
    [" if ", " unless ", " while ", " until ", " for ", " foreach ", " when "]
        .iter()
        .any(|modifier| line.contains(modifier))
}

fn detect_terminator_keyword(line: &str) -> Option<&'static str> {
    ["return", "die", "exit", "goto", "last", "next", "redo"]
        .iter()
        .find(|kw| line.starts_with(**kw))
        .copied()
        .filter(|_| !has_statement_modifier(line))
}

/// Types of dead code detected during Perl script analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeadCodeType {
    /// Subroutine defined but never called
    UnusedSubroutine,
    /// Variable declared but never used
    UnusedVariable,
    /// Constant defined but never referenced
    UnusedConstant,
    /// Package declared but never used
    UnusedPackage,
    /// Code that can never be executed
    UnreachableCode,
    /// Conditional branch that is never taken
    DeadBranch,
    /// Module imported but never used
    UnusedImport,
    /// Function exported but never used externally
    UnusedExport,
}

/// A piece of dead code detected during analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadCode {
    /// Type of dead code (subroutine, variable, etc.)
    pub code_type: DeadCodeType,
    /// Name of the dead code element if available
    pub name: Option<String>,
    /// File path where the dead code is located
    pub file_path: PathBuf,
    /// Starting line number (1-based)
    pub start_line: usize,
    /// Ending line number (1-based)
    pub end_line: usize,
    /// Human-readable explanation of why this is considered dead code
    pub reason: String,
    /// Confidence level (0.0-1.0) in the detection accuracy
    pub confidence: f32,
    /// Optional suggestion for fixing the dead code
    pub suggestion: Option<String>,
}

/// Dead code analysis result for a Perl workspace
#[derive(Debug, Serialize, Deserialize)]
pub struct DeadCodeAnalysis {
    /// List of all dead code instances found
    pub dead_code: Vec<DeadCode>,
    /// Statistical summary of dead code analysis
    pub stats: DeadCodeStats,
    /// Number of files analyzed in the workspace
    pub files_analyzed: usize,
    /// Total lines of code analyzed
    pub total_lines: usize,
}

/// Statistical summary of dead code analysis results
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DeadCodeStats {
    /// Number of unused subroutines detected
    pub unused_subroutines: usize,
    /// Number of unused variables detected
    pub unused_variables: usize,
    /// Number of unused constants detected
    pub unused_constants: usize,
    /// Number of unused packages detected
    pub unused_packages: usize,
    /// Number of unreachable code statements
    pub unreachable_statements: usize,
    /// Number of dead conditional branches
    pub dead_branches: usize,
    /// Number of unused imports detected
    pub unused_imports: usize,
    /// Number of unused exports detected
    pub unused_exports: usize,
    /// Total lines of dead code identified
    pub total_dead_lines: usize,
}

/// Dead code detector
pub struct DeadCodeConfig {
    pub min_confidence: f32,
    pub include_unreachable: bool,
    pub include_unused_symbols: bool,
    pub include_unused_imports: bool,
    pub include_unused_exports: bool,
    pub entry_points: Vec<PathBuf>,
    pub public_api_patterns: Vec<String>,
}

impl Default for DeadCodeConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.0,
            include_unreachable: true,
            include_unused_symbols: true,
            include_unused_imports: true,
            include_unused_exports: true,
            entry_points: Vec::new(),
            public_api_patterns: Vec::new(),
        }
    }
}

/// Dead code detector
pub struct DeadCodeDetector {
    workspace_index: WorkspaceIndex,
    entry_points: HashSet<PathBuf>,
    config: DeadCodeConfig,
}

impl DeadCodeDetector {
    /// Create a new dead code detector with the given workspace index
    ///
    /// # Arguments
    /// * `workspace_index` - Indexed workspace containing symbol definitions and references
    pub fn new(workspace_index: WorkspaceIndex) -> Self {
        Self::with_config(workspace_index, DeadCodeConfig::default())
    }

    /// Create a dead code detector with explicit configuration.
    pub fn with_config(workspace_index: WorkspaceIndex, config: DeadCodeConfig) -> Self {
        let mut entry_points = HashSet::new();
        for entry in &config.entry_points {
            entry_points.insert(entry.clone());
        }
        Self { workspace_index, entry_points, config }
    }

    /// Add an entry point (main script)
    pub fn add_entry_point(&mut self, path: PathBuf) {
        self.entry_points.insert(path.clone());
        self.config.entry_points.push(path);
    }

    /// Analyze a single file for dead code
    pub fn analyze_file(&self, file_path: &Path) -> Result<Vec<DeadCode>, String> {
        let uri = fs_path_to_uri(file_path).map_err(|e| e.to_string())?;
        let text = self
            .workspace_index
            .document_store()
            .get_text(&uri)
            .ok_or_else(|| "file not indexed".to_string())?;

        let mut dead = Vec::new();
        let mut brace_depth: usize = 0;
        let mut block_terminator: Option<(usize, String, usize)> = None;

        for (i, line) in text.lines().enumerate() {
            let line_no = i + 1;
            let trimmed = line.trim();

            if let Some((term_line, term_kw, term_depth)) = &block_terminator {
                if brace_depth == *term_depth
                    && !trimmed.is_empty()
                    && trimmed != "}"
                    && !trimmed.starts_with('#')
                {
                    let start_line = line_no;
                    let mut end_line = line_no;
                    for (j, next_line) in text.lines().enumerate().skip(i + 1) {
                        let next_trimmed = next_line.trim();
                        if next_trimmed == "}" {
                            end_line = j;
                            break;
                        }
                        if !next_trimmed.is_empty() && !next_trimmed.starts_with('#') {
                            end_line = j + 1;
                        }
                    }
                    dead.push(DeadCode {
                        code_type: DeadCodeType::UnreachableCode,
                        name: None,
                        file_path: file_path.to_path_buf(),
                        start_line,
                        end_line,
                        reason: format!(
                            "Code is unreachable after `{}` on line {}",
                            term_kw, term_line
                        ),
                        confidence: 0.9,
                        suggestion: Some("Remove or restructure this code".to_string()),
                    });
                    break;
                }
                if brace_depth < *term_depth {
                    block_terminator = None;
                }
            }

            if let Some(terminator) = detect_terminator_keyword(trimmed) {
                block_terminator = Some((line_no, terminator.to_string(), brace_depth));
            }

            brace_depth += line.chars().filter(|&c| c == '{').count();
            brace_depth = brace_depth.saturating_sub(line.chars().filter(|&c| c == '}').count());
        }

        // Dead branch detection: scan for constant-condition patterns.
        detect_dead_branches(file_path, &text, &mut dead);

        Ok(dead)
    }

    /// Analyze entire workspace for dead code
    pub fn analyze_workspace(&self) -> DeadCodeAnalysis {
        let docs = self.workspace_index.document_store().all_documents();
        let mut dead_code = Vec::new();
        let mut total_lines = 0;

        // Per-file unreachable code
        for doc in &docs {
            total_lines += doc.text.lines().count();
            if let Some(path) = uri_to_fs_path(&doc.uri) {
                if let Ok(mut file_dead) = self.analyze_file(&path) {
                    if self.config.include_unreachable {
                        dead_code.append(&mut file_dead);
                    }
                }
            }
        }

        // Unused symbols across workspace
        if self.config.include_unused_symbols {
            for sym in self.workspace_index.find_unused_symbols() {
                let code_type = match sym.kind {
                    SymbolKind::Subroutine => DeadCodeType::UnusedSubroutine,
                    SymbolKind::Variable(_) => DeadCodeType::UnusedVariable,
                    SymbolKind::Constant => DeadCodeType::UnusedConstant,
                    SymbolKind::Package => DeadCodeType::UnusedPackage,
                    _ => continue,
                };

                let file_path = uri_to_fs_path(&sym.uri).unwrap_or_else(|| PathBuf::from(&sym.uri));

                dead_code.push(DeadCode {
                    code_type,
                    name: Some(sym.name.clone()),
                    file_path,
                    start_line: sym.range.start.line as usize + 1,
                    end_line: sym.range.end.line as usize + 1,
                    reason: "Symbol is never used".to_string(),
                    confidence: 0.9,
                    suggestion: Some("Remove or use this symbol".to_string()),
                });
            }
        }

        dead_code.retain(|item| item.confidence >= self.config.min_confidence);
        dead_code.sort_by(|a, b| {
            a.file_path
                .cmp(&b.file_path)
                .then(a.start_line.cmp(&b.start_line))
                .then_with(|| {
                    let ak = format!("{:?}", a.code_type);
                    let bk = format!("{:?}", b.code_type);
                    ak.cmp(&bk)
                })
                .then_with(|| a.name.as_deref().unwrap_or("").cmp(b.name.as_deref().unwrap_or("")))
        });

        // Compute stats
        let mut stats = DeadCodeStats::default();
        for item in &dead_code {
            let lines = item.end_line.saturating_sub(item.start_line) + 1;
            stats.total_dead_lines += lines;
            match item.code_type {
                DeadCodeType::UnusedSubroutine => stats.unused_subroutines += 1,
                DeadCodeType::UnusedVariable => stats.unused_variables += 1,
                DeadCodeType::UnusedConstant => stats.unused_constants += 1,
                DeadCodeType::UnusedPackage => stats.unused_packages += 1,
                DeadCodeType::UnreachableCode => stats.unreachable_statements += 1,
                DeadCodeType::DeadBranch => stats.dead_branches += 1,
                _ => {}
            }
        }

        DeadCodeAnalysis { dead_code, stats, files_analyzed: docs.len(), total_lines }
    }
}

#[cfg(test)]
mod tests {
    use super::{detect_terminator_keyword, has_statement_modifier};

    #[test]
    fn statement_modifiers_are_detected() {
        assert!(has_statement_modifier("next if $skip;"));
        assert!(has_statement_modifier("last unless $ok;"));
        assert!(!has_statement_modifier("next;"));
    }

    #[test]
    fn conditional_flow_control_is_not_treated_as_terminator() {
        assert_eq!(detect_terminator_keyword("next if $skip;"), None);
        assert_eq!(detect_terminator_keyword("last unless $done;"), None);
        assert_eq!(detect_terminator_keyword("redo while $retry;"), None);
        assert_eq!(detect_terminator_keyword("next;"), Some("next"));
        assert_eq!(detect_terminator_keyword("return $value;"), Some("return"));
    }
}
