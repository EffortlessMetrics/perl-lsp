//! Shared validation primitives for refactor plans and edit batches.

use crate::refactor::refactor_plan::{RefactorDiagnostic, RefactorPlan};
use crate::workspace_refactor::TextEdit;

/// Validate per-file edit ordering and overlap invariants.
pub fn validate_plan(plan: &RefactorPlan) -> Vec<RefactorDiagnostic> {
    let mut diagnostics = Vec::new();
    for file_edit in &plan.edits {
        if file_edit.edits.is_empty() {
            continue;
        }

        if !is_sorted_non_overlapping(&file_edit.edits) {
            diagnostics.push(RefactorDiagnostic {
                code: "edit_overlap_or_order".to_string(),
                message: format!(
                    "Edits must be sorted and non-overlapping: {}",
                    file_edit.file_path.display()
                ),
            });
        }
    }
    diagnostics
}

fn is_sorted_non_overlapping(edits: &[TextEdit]) -> bool {
    let mut previous_end = 0usize;
    for (index, edit) in edits.iter().enumerate() {
        if edit.start > edit.end {
            return false;
        }
        if index > 0 && edit.start < previous_end {
            return false;
        }
        previous_end = edit.end;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::validate_plan;
    use crate::refactor::refactor_plan::{
        RefactorConfidence, RefactorOperationKind, RefactorPlan, RefactorSafety, RefactorStats,
    };
    use crate::workspace_refactor::{FileEdit, TextEdit};
    use std::path::PathBuf;

    #[test]
    fn validate_plan_accepts_non_overlapping_sorted_edits() -> Result<(), Box<dyn std::error::Error>>
    {
        let plan = RefactorPlan {
            operation: RefactorOperationKind::OptimizeImports,
            edits: vec![FileEdit {
                file_path: PathBuf::from("a.pm"),
                edits: vec![
                    TextEdit { start: 0, end: 1, new_text: "x".to_string() },
                    TextEdit { start: 1, end: 2, new_text: "y".to_string() },
                ],
            }],
            diagnostics: vec![],
            confidence: RefactorConfidence::Exact,
            safety: RefactorSafety::Safe,
            stats: RefactorStats { files_changed: 1, edits_count: 2 },
        };
        assert!(validate_plan(&plan).is_empty());
        Ok(())
    }

    #[test]
    fn validate_plan_rejects_overlapping_edits() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RefactorPlan {
            operation: RefactorOperationKind::OptimizeImports,
            edits: vec![FileEdit {
                file_path: PathBuf::from("a.pm"),
                edits: vec![
                    TextEdit { start: 0, end: 3, new_text: "x".to_string() },
                    TextEdit { start: 2, end: 4, new_text: "y".to_string() },
                ],
            }],
            diagnostics: vec![],
            confidence: RefactorConfidence::Exact,
            safety: RefactorSafety::Safe,
            stats: RefactorStats { files_changed: 1, edits_count: 2 },
        };
        assert_eq!(validate_plan(&plan).len(), 1);
        Ok(())
    }

    #[test]
    fn validate_plan_accepts_empty_edit_batches() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RefactorPlan {
            operation: RefactorOperationKind::OptimizeImports,
            edits: vec![FileEdit { file_path: PathBuf::from("empty.pm"), edits: vec![] }],
            diagnostics: vec![],
            confidence: RefactorConfidence::Exact,
            safety: RefactorSafety::Safe,
            stats: RefactorStats { files_changed: 0, edits_count: 0 },
        };
        assert!(validate_plan(&plan).is_empty());
        Ok(())
    }

    #[test]
    fn validate_plan_rejects_reversed_edit_ranges() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RefactorPlan {
            operation: RefactorOperationKind::OptimizeImports,
            edits: vec![FileEdit {
                file_path: PathBuf::from("reversed.pm"),
                edits: vec![TextEdit { start: 4, end: 2, new_text: "x".to_string() }],
            }],
            diagnostics: vec![],
            confidence: RefactorConfidence::Exact,
            safety: RefactorSafety::Safe,
            stats: RefactorStats { files_changed: 1, edits_count: 1 },
        };
        let diagnostics = validate_plan(&plan);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "edit_overlap_or_order");
        assert!(diagnostics[0].message.contains("reversed.pm"));
        Ok(())
    }

    #[test]
    fn validate_plan_accepts_adjacent_zero_length_edits() -> Result<(), Box<dyn std::error::Error>>
    {
        let plan = RefactorPlan {
            operation: RefactorOperationKind::OptimizeImports,
            edits: vec![FileEdit {
                file_path: PathBuf::from("zero-length.pm"),
                edits: vec![
                    TextEdit { start: 0, end: 0, new_text: "use strict;\n".to_string() },
                    TextEdit { start: 0, end: 0, new_text: "use warnings;\n".to_string() },
                    TextEdit { start: 3, end: 3, new_text: "# marker".to_string() },
                ],
            }],
            diagnostics: vec![],
            confidence: RefactorConfidence::Exact,
            safety: RefactorSafety::Safe,
            stats: RefactorStats { files_changed: 1, edits_count: 3 },
        };
        assert!(validate_plan(&plan).is_empty());
        Ok(())
    }
}
