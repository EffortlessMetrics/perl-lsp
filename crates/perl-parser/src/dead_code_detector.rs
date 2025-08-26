//! Dead code detection for Perl codebases
//!
//! This module performs a lightweight analysis to locate unused subroutines in
//! a workspace.  The original version only contained stubs; the implementation
//! below uses the [`WorkspaceIndex`]'s document store to search for subroutine
//! definitions that are never called.

use crate::workspace_index::{fs_path_to_uri, uri_to_fs_path, WorkspaceIndex};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Types of dead code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeadCodeType {
    UnusedSubroutine,
    UnusedVariable,
    UnusedConstant,
    UnusedPackage,
    UnreachableCode,
    DeadBranch,
    UnusedImport,
    UnusedExport,
}

/// A piece of dead code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadCode {
    pub code_type: DeadCodeType,
    pub name: Option<String>,
    pub file_path: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub reason: String,
    pub confidence: f32,
    pub suggestion: Option<String>,
}

/// Dead code analysis result
#[derive(Debug, Serialize, Deserialize)]
pub struct DeadCodeAnalysis {
    pub dead_code: Vec<DeadCode>,
    pub stats: DeadCodeStats,
    pub files_analyzed: usize,
    pub total_lines: usize,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DeadCodeStats {
    pub unused_subroutines: usize,
    pub unused_variables: usize,
    pub unused_constants: usize,
    pub unused_packages: usize,
    pub unreachable_statements: usize,
    pub dead_branches: usize,
    pub total_dead_lines: usize,
}

/// Dead code detector
pub struct DeadCodeDetector {
    _workspace_index: WorkspaceIndex,
    entry_points: HashSet<PathBuf>,
}

impl DeadCodeDetector {
    pub fn new(workspace_index: WorkspaceIndex) -> Self {
        Self { _workspace_index: workspace_index, entry_points: HashSet::new() }
    }

    /// Add an entry point (main script)
    pub fn add_entry_point(&mut self, path: PathBuf) {
        self.entry_points.insert(path);
    }

    /// Analyze a single file for dead code.
    ///
    /// Currently only unused subroutines are detected.  The implementation is
    /// intentionally conservative and based on simple text matching.
    pub fn analyze_file(&self, file_path: &Path) -> Result<Vec<DeadCode>, String> {
        // Try to get file content from the document store first
        let uri = fs_path_to_uri(file_path).unwrap_or_else(|_| file_path.to_string_lossy().into());
        let content = if let Some(doc) = self._workspace_index.document_store().get(&uri) {
            doc.text
        } else {
            std::fs::read_to_string(file_path)
                .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?
        };

        let mut results = Vec::new();
        let sub_re = Regex::new(r"(?m)^sub\s+([A-Za-z_][A-Za-z0-9_]*)").unwrap();
        for caps in sub_re.captures_iter(&content) {
            let name = caps[1].to_string();
            let start_line = content[..caps.get(0).unwrap().start()].lines().count() + 1;

            // Search the entire workspace for calls to this subroutine
            let call_re = Regex::new(&format!(r"\b{}\s*\(", regex::escape(&name))).unwrap();
            let mut used = false;
            for doc in self._workspace_index.document_store().all_documents() {
                if call_re.is_match(&doc.text) {
                    used = true;
                    break;
                }
            }

            if !used {
                results.push(DeadCode {
                    code_type: DeadCodeType::UnusedSubroutine,
                    name: Some(name),
                    file_path: file_path.to_path_buf(),
                    start_line,
                    end_line: start_line,
                    reason: "Subroutine is never used".into(),
                    confidence: 0.8,
                    suggestion: Some("Consider removing the subroutine".into()),
                });
            }
        }

        Ok(results)
    }

    /// Analyze entire workspace for dead code.
    pub fn analyze_workspace(&self) -> DeadCodeAnalysis {
        let documents = self._workspace_index.document_store().all_documents();
        let mut dead_code = Vec::new();
        let mut stats = DeadCodeStats::default();
        let mut total_lines = 0;

        for doc in &documents {
            total_lines += doc.text.lines().count();
            if let Some(path) = uri_to_fs_path(&doc.uri) {
                match self.analyze_file(&path) {
                    Ok(mut dc) => {
                        stats.unused_subroutines += dc
                            .iter()
                            .filter(|d| d.code_type == DeadCodeType::UnusedSubroutine)
                            .count();
                        for item in &dc {
                            stats.total_dead_lines +=
                                item.end_line.saturating_sub(item.start_line) + 1;
                        }
                        dead_code.append(&mut dc);
                    }
                    Err(_) => {}
                }
            }
        }

        DeadCodeAnalysis { dead_code, stats, files_analyzed: documents.len(), total_lines }
    }
}

/// Generate a report from dead code analysis
pub fn generate_report(analysis: &DeadCodeAnalysis) -> String {
    let mut report = String::new();

    report.push_str("=== Dead Code Analysis Report ===\n\n");

    report.push_str(&format!("Files analyzed: {}\n", analysis.files_analyzed));
    report.push_str(&format!("Total lines: {}\n", analysis.total_lines));
    report.push_str(&format!("Dead code items: {}\n\n", analysis.dead_code.len()));

    report.push_str("Statistics:\n");
    report.push_str(&format!("  Unused subroutines: {}\n", analysis.stats.unused_subroutines));
    report.push_str(&format!("  Unused variables: {}\n", analysis.stats.unused_variables));
    report.push_str(&format!("  Unused constants: {}\n", analysis.stats.unused_constants));
    report.push_str(&format!("  Unused packages: {}\n", analysis.stats.unused_packages));
    report.push_str(&format!(
        "  Unreachable statements: {}\n",
        analysis.stats.unreachable_statements
    ));
    report.push_str(&format!("  Dead branches: {}\n", analysis.stats.dead_branches));
    report.push_str(&format!("  Total dead lines: {}\n", analysis.stats.total_dead_lines));

    report
}
