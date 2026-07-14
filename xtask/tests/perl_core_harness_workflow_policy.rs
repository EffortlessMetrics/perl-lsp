//! Contract tests for the advisory Perl core harness workflow.

use std::{fs, path::PathBuf};

use perl_tdd_support::{must, must_some};

#[test]
fn perl_core_harness_workflow_is_schedule_or_manual_only() {
    let root = repo_root();
    let workflow = must(fs::read_to_string(root.join(".github/workflows/perl-core-harness.yml")));

    assert!(workflow.contains("name: Perl Core Harness"));
    assert!(workflow.contains("schedule:"));
    assert!(workflow.contains("workflow_dispatch:"));
    assert!(
        !workflow.contains("pull_request:") && !workflow.contains("merge_group:"),
        "Perl core harness workflow must stay advisory and must not run on PR or merge queue"
    );
    assert!(
        workflow.contains("perl-core-harness prepare")
            && workflow.contains("perl-core-harness smoke")
            && workflow.contains("target/perl-core/"),
        "workflow must prepare upstream Perl, run smoke, and upload receipts"
    );
}

fn repo_root() -> PathBuf {
    must_some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent()).to_path_buf()
}
