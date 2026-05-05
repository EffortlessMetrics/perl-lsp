//! Dead code detection for Perl codebases (stub implementation)
//!
//! This module identifies unused code including unreachable code and unused symbols.
//! Currently a stub implementation to demonstrate the architecture.

use perl_workspace::workspace_index::{SymbolKind, WorkspaceIndex, fs_path_to_uri, uri_to_fs_path};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

mod dead_branches;
mod report;

pub use report::generate_report;

use crate::dead_branches::detect_dead_branches;

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
    /// Total lines of dead code identified
    pub total_dead_lines: usize,
}

/// Dead code detector
pub struct DeadCodeDetector {
    workspace_index: WorkspaceIndex,
    entry_points: HashSet<PathBuf>,
}

impl DeadCodeDetector {
    /// Create a new dead code detector with the given workspace index
    ///
    /// # Arguments
    /// * `workspace_index` - Indexed workspace containing symbol definitions and references
    pub fn new(workspace_index: WorkspaceIndex) -> Self {
        Self { workspace_index, entry_points: HashSet::new() }
    }

    /// Add an entry point (main script)
    pub fn add_entry_point(&mut self, path: PathBuf) {
        self.entry_points.insert(path);
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
        dead.extend(self.detect_unreachable_ranges(file_path, &text));

        // Dead branch detection: scan for constant-condition patterns.
        detect_dead_branches(file_path, &text, &mut dead);

        Ok(dead)
    }

    fn detect_unreachable_ranges(&self, file_path: &Path, text: &str) -> Vec<DeadCode> {
        let mut findings = Vec::new();
        let mut depth: i32 = 0;
        let mut terminator: Option<(usize, String, i32)> = None;
        let mut unreachable_start: Option<usize> = None;
        let mut unreachable_end: Option<usize> = None;
        let mut in_pod = false;

        for (i, line) in text.lines().enumerate() {
            let line_no = i + 1;
            let trimmed = line.trim();

            if trimmed.starts_with("=") && !trimmed.starts_with("=cut") {
                in_pod = true;
            }
            if trimmed == "=cut" {
                in_pod = false;
                continue;
            }
            if in_pod {
                continue;
            }

            let close_count = trimmed.chars().filter(|c| *c == '}').count() as i32;
            let open_count = trimmed.chars().filter(|c| *c == '{').count() as i32;
            let depth_before = depth;
            depth -= close_count;

            let is_comment = trimmed.starts_with('#');
            let is_structural = trimmed.is_empty() || is_comment || trimmed == "{" || trimmed == "}";

            if let Some((term_line, term_kw, term_depth)) = &terminator {
                if depth_before < *term_depth {
                    if let (Some(start), Some(end)) = (unreachable_start.take(), unreachable_end.take()) {
                        findings.push(DeadCode {
                            code_type: DeadCodeType::UnreachableCode,
                            name: None,
                            file_path: file_path.to_path_buf(),
                            start_line: start,
                            end_line: end,
                            reason: format!("Code is unreachable after `{}` on line {}", term_kw, term_line),
                            confidence: 0.8,
                            suggestion: Some("Remove or restructure this code".to_string()),
                        });
                    }
                    terminator = None;
                } else if !is_structural {
                    unreachable_start.get_or_insert(line_no);
                    unreachable_end = Some(line_no);
                }
            }

            if terminator.is_none() && Self::is_terminator(trimmed) {
                if let Some(first_word) = trimmed.split_whitespace().next() {
                    terminator = Some((line_no, first_word.trim_end_matches(';').to_string(), depth_before));
                }
            }

            depth += open_count;
        }

        if let Some((term_line, term_kw, _)) = terminator
            && let (Some(start), Some(end)) = (unreachable_start, unreachable_end)
        {
            findings.push(DeadCode {
                code_type: DeadCodeType::UnreachableCode,
                name: None,
                file_path: file_path.to_path_buf(),
                start_line: start,
                end_line: end,
                reason: format!("Code is unreachable after `{}` on line {}", term_kw, term_line),
                confidence: 0.8,
                suggestion: Some("Remove or restructure this code".to_string()),
            });
        }

        findings
    }

    fn is_terminator(trimmed: &str) -> bool {
        ["return", "die", "exit", "goto", "last", "next", "redo"]
            .iter()
            .any(|kw| {
                trimmed == *kw
                    || trimmed.starts_with(&format!("{kw} "))
                    || trimmed.starts_with(&format!("{kw};"))
            })
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
                    dead_code.append(&mut file_dead);
                }
            }
        }

        // Unused symbols across workspace
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
