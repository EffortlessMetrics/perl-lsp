use std::fs;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir
}

fn committed_metric_baselines() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let root = project_root();
    let baseline_dir = root.join(".ci/metrics/baselines");
    let mut baselines = Vec::new();

    for entry in fs::read_dir(baseline_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("baseline path has non-utf8 file stem: {}", path.display()))?;
        baselines.push(stem.to_string());
    }

    baselines.sort();
    Ok(baselines)
}

#[test]
fn ci_metrics_ratchet_recipe_checks_every_committed_baseline()
-> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let justfile = fs::read_to_string(root.join("justfile"))?;

    for subsystem in committed_metric_baselines()? {
        let command = format!("metrics ratchet-check {subsystem}");
        assert!(
            justfile.contains(&command),
            "just ci-metrics-ratchet must check committed baseline `{subsystem}`"
        );
    }

    Ok(())
}

#[test]
fn ratchet_guide_lists_every_committed_baseline() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let guide = fs::read_to_string(root.join("docs/project/metrics/RATCHET.md"))?;

    for subsystem in committed_metric_baselines()? {
        let path = format!(".ci/metrics/baselines/{subsystem}.json");
        assert!(guide.contains(&path), "RATCHET.md must list `{path}`");
    }

    Ok(())
}

#[test]
fn nightly_ratchet_job_is_label_gated_and_bootstrap_safe() -> Result<(), Box<dyn std::error::Error>>
{
    let root = project_root();
    let workflow = fs::read_to_string(root.join(".github/workflows/ci-nightly.yml"))?;

    assert!(
        workflow.contains("scorecard-ratchet-check:"),
        "ci-nightly.yml must define the scorecard ratchet job"
    );
    assert!(
        workflow.contains("ci:metrics-ratchet"),
        "scorecard ratchet job must stay label-gated for PRs"
    );
    assert!(
        workflow.contains("just ci-metrics-ratchet"),
        "scorecard ratchet job must use the shared just recipe"
    );
    assert!(
        workflow.contains("bootstrap-safe"),
        "workflow should document that missing receipts pass in bootstrap mode"
    );

    Ok(())
}
