//! Shared edit validation helpers.

use crate::refactor::edit_plan::{RefactorDiagnostic, RefactorPlan};

/// Validate common edit invariants and return any non-fatal diagnostics.
pub fn validate_refactor_plan(plan: &RefactorPlan) -> Vec<RefactorDiagnostic> {
    let mut diagnostics = Vec::new();

    for file_edit in &plan.edits {
        let mut previous_end = 0usize;

        for edit in &file_edit.edits {
            if edit.start > edit.end {
                diagnostics.push(RefactorDiagnostic {
                    message: format!(
                        "Invalid edit range in {}: start {} is after end {}",
                        file_edit.file_path.display(),
                        edit.start,
                        edit.end
                    ),
                });
                continue;
            }

            if edit.start < previous_end {
                diagnostics.push(RefactorDiagnostic {
                    message: format!(
                        "Overlapping edit range in {}: start {} overlaps previous end {}",
                        file_edit.file_path.display(),
                        edit.start,
                        previous_end
                    ),
                });
            }

            previous_end = edit.end;
        }
    }

    diagnostics
}
