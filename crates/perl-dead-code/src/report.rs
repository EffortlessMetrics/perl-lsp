use crate::DeadCodeAnalysis;

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
