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
        let mut block_depth = 0usize;
        let mut terminator: Option<(usize, usize, String)> = None;

        for (i, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            let current_depth = block_depth;

            if let Some((term_line, term_depth, term_kw)) = &terminator {
                if current_depth < *term_depth {
                    terminator = None;
                } else if current_depth == *term_depth
                    && !trimmed.is_empty()
                    && !trimmed.starts_with('#')
                    && !is_structural_line(trimmed)
                {
                    dead.push(DeadCode {
                        code_type: DeadCodeType::UnreachableCode,
                        name: None,
                        file_path: file_path.to_path_buf(),
                        start_line: i + 1,
                        end_line: i + 1,
                        reason: format!(
                            "Code is unreachable after `{}` on line {}",
                            term_kw, term_line
                        ),
                        confidence: 0.9,
                        suggestion: Some("Remove or restructure this code".to_string()),
                    });
                    break;
                }
            }

            if let Some(term_kw) = detect_unconditional_terminator(trimmed) {
                terminator = Some((i + 1, current_depth, term_kw.to_string()));
            }

            block_depth += line.chars().filter(|&ch| ch == '{').count();
            block_depth = block_depth.saturating_sub(line.chars().filter(|&ch| ch == '}').count());
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

fn is_structural_line(trimmed: &str) -> bool {
    !trimmed.is_empty() && trimmed.chars().all(|ch| ch == '}' || ch == ';')
}

fn detect_unconditional_terminator(trimmed: &str) -> Option<&str> {
    const TERMINATORS: [&str; 4] = ["return", "die", "exit", "CORE::exit"];

    let first = trimmed
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ';' | '('))
        .next()
        .unwrap_or_default();
    if !TERMINATORS.contains(&first) {
        return None;
    }

    let after_terminator = &trimmed[first.len()..];
    let remainder = match after_terminator.split_once('#') {
        Some((before_comment, _)) => before_comment,
        None => after_terminator,
    }
    .trim_start();
    if contains_postfix_condition(remainder) {
        return None;
    }

    Some(first)
}

fn contains_postfix_condition(remainder: &str) -> bool {
    const CONDITIONS: [&str; 7] = ["if", "unless", "when", "while", "until", "for", "foreach"];
    CONDITIONS.iter().any(|keyword| contains_keyword(remainder, keyword))
}

fn contains_keyword(text: &str, keyword: &str) -> bool {
    text.match_indices(keyword).any(|(idx, _)| {
        let before = text[..idx].chars().next_back();
        let after = text[idx + keyword.len()..].chars().next();
        is_keyword_boundary(before) && is_keyword_boundary(after)
    })
}

fn is_keyword_boundary(ch: Option<char>) -> bool {
    ch.is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
}
