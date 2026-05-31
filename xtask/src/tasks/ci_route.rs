use color_eyre::eyre::{Result, bail};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CiRouteArgs {
    pub base: String,
    pub head: String,
    pub receipt: PathBuf,
    pub changed_files: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct CiRouteReceipt {
    schema_version: &'static str,
    provider_action: &'static str,
    claim_boundary: &'static str,
    base: String,
    head: String,
    changed_files: Vec<String>,
    changed_surfaces: Vec<String>,
    required_proof_packs: Vec<ProofPackReceipt>,
    skipped_by_policy: BTreeMap<String, String>,
    coverage_pack_selector: Vec<String>,
    estimated_lem: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct ProofPackReceipt {
    id: String,
    commands: Vec<String>,
}

#[derive(Debug, Clone)]
struct ProofPack {
    id: &'static str,
    commands: &'static [&'static str],
}

const PREFLIGHT_PACK: ProofPack = ProofPack {
    id: "preflight",
    commands: &[
        "cargo xtask pr title-check --no-gh",
        "cargo fmt -p xtask -- --check",
        "git diff --check",
    ],
};

const DOCS_PACK: ProofPack = ProofPack {
    id: "docs-focused",
    commands: &["cargo xtask check-devex-docs", "cargo xtask doc-claims"],
};

const XTASK_SEMANTIC_INLINE_PACK: ProofPack = ProofPack {
    id: "xtask-semantic-inline-receipts",
    commands: &[
        "cargo test -p xtask --bin xtask --profile agent --locked semantic_inline_receipts -- --nocapture",
        "cargo test -p xtask --test semantic_inline_receipts_cli --profile agent --locked -- --nocapture",
    ],
};

const XTASK_SUPPORTED_EDITOR_INLINE_PACK: ProofPack = ProofPack {
    id: "xtask-supported-editor-inline-smoke",
    commands: &[
        "cargo test -p xtask --bin xtask --profile agent --locked supported_editor_inline_smoke -- --nocapture",
        "cargo test -p xtask --test supported_editor_inline_smoke_cli --profile agent --locked -- --nocapture",
        "cargo test -p xtask --bin xtask --profile agent --locked semantic_inline_receipts -- --nocapture",
    ],
};

const INLINE_CORE_PACK: ProofPack = ProofPack {
    id: "inline-core",
    commands: &[
        "cargo test -p perl-lsp-rs-core --lib --profile agent --locked inline_completion -- --nocapture",
        "cargo run -p xtask --profile agent --locked -- inline-completion-quality --receipt target/receipts/inline-completion-quality.json",
    ],
};

const UX_SCENARIO_PACK: ProofPack = ProofPack {
    id: "ux-scenario-focused",
    commands: &[
        "cargo test -p perl-lsp-ux-tests --profile agent --locked -- --nocapture",
        "python -m json.tool crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json",
    ],
};

const CI_POLICY_PACK: ProofPack = ProofPack {
    id: "ci-policy-focused",
    commands: &[
        "cargo xtask workflow-trigger-lint --policy .ci/policies/required-checks.toml --receipt target/receipts/workflow-trigger-lint.json",
        "cargo test -p xtask --test quality_ci_wiring_policy --profile agent --locked -- --nocapture",
    ],
};

const CI_ROUTE_PACK: ProofPack = ProofPack {
    id: "ci-route-receipt",
    commands: &[
        "cargo test -p xtask --bin xtask --profile agent --locked ci_route -- --nocapture",
        "cargo test -p xtask --test ci_route_cli --profile agent --locked -- --nocapture",
        "cargo run -p xtask --profile agent --locked -- ci route --base origin/main --head HEAD --receipt target/receipts/ci-route.json",
    ],
};

const GENERAL_RUST_PACK: ProofPack = ProofPack {
    id: "rust-focused",
    commands: &["cargo check --workspace --all-targets --profile agent --locked"],
};

pub fn run(args: CiRouteArgs) -> Result<()> {
    let changed_files = if args.changed_files.is_empty() {
        git_changed_files(&args.base, &args.head)?
    } else {
        normalize_changed_files(args.changed_files)
    };
    let receipt = route_receipt(&args.base, &args.head, changed_files);
    write_receipt(&args.receipt, &receipt)?;
    println!(
        "ci route receipt OK: {} changed files, {} proof packs, {}",
        receipt.changed_files.len(),
        receipt.required_proof_packs.len(),
        args.receipt.display()
    );
    Ok(())
}

fn git_changed_files(base: &str, head: &str) -> Result<Vec<String>> {
    let output =
        Command::new("git").args(["diff", "--name-only", &format!("{base}...{head}")]).output()?;
    if !output.status.success() {
        bail!(
            "git diff --name-only {base}...{head} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let raw = String::from_utf8(output.stdout)?;
    Ok(normalize_changed_files(raw.lines().map(ToString::to_string).collect()))
}

fn normalize_changed_files(files: Vec<String>) -> Vec<String> {
    files
        .into_iter()
        .map(|file| file.replace('\\', "/"))
        .filter(|file| !file.trim().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn route_receipt(base: &str, head: &str, changed_files: Vec<String>) -> CiRouteReceipt {
    let mut route = RouteBuilder::default();
    route.add_pack(PREFLIGHT_PACK);

    if changed_files.is_empty() {
        route.add_surface("no_changes");
    }

    let docs_only =
        !changed_files.is_empty() && changed_files.iter().all(|file| is_docs_file(file));
    if docs_only {
        route.add_surface("docs");
        route.add_pack(DOCS_PACK);
        route.skip("full-ux-regression", "docs-only change");
        route.skip("release-smoke", "no release surface changed");
        route.skip("docker", "no docker or release workflow changed");
        route.skip("codecov-patch-95", "docs-only change");
    } else {
        for file in &changed_files {
            route_file(file, &mut route);
        }
        route.skip("release-smoke", "no release surface changed");
        route.skip("docker", "no docker or release workflow changed");
    }

    let estimated_lem = route.estimated_lem();

    CiRouteReceipt {
        schema_version: "ci-route.v1",
        provider_action: "changed_file_proof_pack_route",
        claim_boundary: "advisory changed-file proof routing only; does not weaken required checks or skip branch protection",
        base: base.to_string(),
        head: head.to_string(),
        changed_files,
        changed_surfaces: route.surfaces.into_iter().collect(),
        required_proof_packs: route
            .proof_packs
            .into_values()
            .map(|pack| ProofPackReceipt {
                id: pack.id.to_string(),
                commands: pack.commands.iter().map(|command| (*command).to_string()).collect(),
            })
            .collect(),
        skipped_by_policy: route.skipped_by_policy,
        coverage_pack_selector: route.coverage_pack_selector.into_iter().collect(),
        estimated_lem,
    }
}

fn route_file(file: &str, route: &mut RouteBuilder) {
    if file == "xtask/src/tasks/supported_editor_inline_smoke.rs"
        || file == "xtask/tests/supported_editor_inline_smoke_cli.rs"
    {
        route.add_surface("xtask-supported-editor-inline-smoke");
        route.add_pack(XTASK_SUPPORTED_EDITOR_INLINE_PACK);
        route.add_coverage_pack("patch-coverage-xtask-supported-editor-inline-smoke");
        route.skip("full-ux-regression", "supported-editor smoke receipt change");
        return;
    }

    if file == "xtask/src/tasks/semantic_inline_receipts.rs"
        || file == "xtask/src/tasks/semantic_inline_next_edit.rs"
        || file == "xtask/tests/semantic_inline_receipts_cli.rs"
        || file == "xtask/tests/semantic_inline_next_edit_cli.rs"
    {
        route.add_surface("xtask-semantic-inline-receipts");
        route.add_pack(XTASK_SEMANTIC_INLINE_PACK);
        route.add_coverage_pack("patch-coverage-xtask-semantic-inline");
        route.skip("full-ux-regression", "semantic receipt/dashboard change");
        return;
    }

    if file.starts_with("crates/perl-lsp-rs-core/src/providers/inline_completion/") {
        route.add_surface("inline-core");
        route.add_pack(INLINE_CORE_PACK);
        route.add_coverage_pack("patch-coverage-inline-core");
        return;
    }

    if file.starts_with("crates/perl-lsp-ux-tests/tests/ux_scenario_") {
        route.add_surface("ux-scenario");
        route.add_pack(UX_SCENARIO_PACK);
        route.add_coverage_pack("patch-coverage-ux-scenario");
        return;
    }

    if file.starts_with(".github/workflows/")
        || file.starts_with(".ci/")
        || file.starts_with("policy/")
    {
        route.add_surface("ci-policy");
        route.add_pack(CI_POLICY_PACK);
        route.add_coverage_pack("patch-coverage-ci-policy");
        return;
    }

    if file == "xtask/src/tasks/ci_route.rs" || file == "xtask/tests/ci_route_cli.rs" {
        route.add_surface("ci-routing");
        route.add_pack(CI_ROUTE_PACK);
        route.add_coverage_pack("patch-coverage-ci-route");
        return;
    }

    if is_docs_file(file) {
        route.add_surface("docs");
        route.add_pack(DOCS_PACK);
        return;
    }

    if file.ends_with(".rs") {
        route.add_surface("rust");
        route.add_pack(GENERAL_RUST_PACK);
        route.add_coverage_pack("patch-coverage-rust-focused");
    } else {
        route.add_surface("misc");
    }
}

fn is_docs_file(file: &str) -> bool {
    file.starts_with("docs/")
        || file == "README.md"
        || file == "CHANGELOG.md"
        || file.ends_with(".md")
}

fn write_receipt(path: &Path, receipt: &CiRouteReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(receipt)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

#[derive(Default)]
struct RouteBuilder {
    surfaces: BTreeSet<String>,
    proof_packs: BTreeMap<&'static str, ProofPack>,
    skipped_by_policy: BTreeMap<String, String>,
    coverage_pack_selector: BTreeSet<String>,
}

impl RouteBuilder {
    fn add_surface(&mut self, surface: impl Into<String>) {
        self.surfaces.insert(surface.into());
    }

    fn add_pack(&mut self, pack: ProofPack) {
        self.proof_packs.insert(pack.id, pack);
    }

    fn skip(&mut self, pack: impl Into<String>, reason: impl Into<String>) {
        self.skipped_by_policy.entry(pack.into()).or_insert_with(|| reason.into());
    }

    fn add_coverage_pack(&mut self, pack: impl Into<String>) {
        self.coverage_pack_selector.insert(pack.into());
    }

    fn estimated_lem(&self) -> u64 {
        let pack_cost = u64::try_from(self.proof_packs.len()).unwrap_or(u64::MAX);
        let coverage_cost = u64::try_from(self.coverage_pack_selector.len()).unwrap_or(u64::MAX);
        2 + pack_cost.saturating_mul(3) + coverage_cost.saturating_mul(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::eyre;
    use serde_json::Value;
    use tempfile::TempDir;

    #[test]
    fn route_receipt_maps_supported_editor_smoke_to_focused_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["xtask/src/tasks/supported_editor_inline_smoke.rs".to_string()],
        );

        assert_eq!(receipt.changed_surfaces, vec!["xtask-supported-editor-inline-smoke"]);
        assert!(proof_pack_ids(&receipt).contains(&"xtask-supported-editor-inline-smoke"));
        assert!(
            receipt
                .coverage_pack_selector
                .iter()
                .any(|pack| pack == "patch-coverage-xtask-supported-editor-inline-smoke")
        );
        assert_eq!(
            receipt.skipped_by_policy.get("full-ux-regression").map(String::as_str),
            Some("supported-editor smoke receipt change")
        );
        Ok(())
    }

    #[test]
    fn route_receipt_maps_semantic_inline_receipts_to_dashboard_pack() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["xtask/src/tasks/semantic_inline_receipts.rs".to_string()],
        );

        assert_eq!(receipt.changed_surfaces, vec!["xtask-semantic-inline-receipts"]);
        assert!(proof_pack_ids(&receipt).contains(&"xtask-semantic-inline-receipts"));
        assert!(
            receipt
                .coverage_pack_selector
                .iter()
                .any(|pack| pack == "patch-coverage-xtask-semantic-inline")
        );
        Ok(())
    }

    #[test]
    fn route_receipt_skips_coverage_for_docs_only_changes() -> Result<()> {
        let receipt = route_receipt(
            "origin/main",
            "HEAD",
            vec!["docs/development/INLINE_COMPLETION_ROADMAP.md".to_string()],
        );

        assert_eq!(receipt.changed_surfaces, vec!["docs"]);
        assert!(proof_pack_ids(&receipt).contains(&"docs-focused"));
        assert!(receipt.coverage_pack_selector.is_empty());
        assert_eq!(
            receipt.skipped_by_policy.get("codecov-patch-95").map(String::as_str),
            Some("docs-only change")
        );
        Ok(())
    }

    #[test]
    fn route_receipt_maps_ci_route_files_to_route_pack() -> Result<()> {
        let receipt =
            route_receipt("origin/main", "HEAD", vec!["xtask/src/tasks/ci_route.rs".to_string()]);

        assert_eq!(receipt.changed_surfaces, vec!["ci-routing"]);
        assert!(proof_pack_ids(&receipt).contains(&"ci-route-receipt"));
        assert!(
            receipt.coverage_pack_selector.iter().any(|pack| pack == "patch-coverage-ci-route")
        );
        Ok(())
    }

    #[test]
    fn route_command_writes_receipt_from_explicit_changed_files() -> Result<()> {
        let temp = TempDir::new()?;
        let receipt_path = temp.path().join("ci-route.json");

        run(CiRouteArgs {
            base: "origin/main".to_string(),
            head: "HEAD".to_string(),
            receipt: receipt_path.clone(),
            changed_files: vec!["xtask\\src\\tasks\\supported_editor_inline_smoke.rs".to_string()],
        })?;

        let value: Value = serde_json::from_str(&fs::read_to_string(receipt_path)?)?;
        assert_eq!(
            value
                .get("schema_version")
                .and_then(Value::as_str)
                .ok_or_else(|| eyre!("missing schema_version"))?,
            "ci-route.v1"
        );
        assert_eq!(
            value
                .pointer("/changed_files/0")
                .and_then(Value::as_str)
                .ok_or_else(|| eyre!("missing changed file"))?,
            "xtask/src/tasks/supported_editor_inline_smoke.rs"
        );
        assert_eq!(
            value
                .pointer("/coverage_pack_selector/0")
                .and_then(Value::as_str)
                .ok_or_else(|| eyre!("missing coverage pack"))?,
            "patch-coverage-xtask-supported-editor-inline-smoke"
        );
        Ok(())
    }

    fn proof_pack_ids(receipt: &CiRouteReceipt) -> Vec<&str> {
        receipt.required_proof_packs.iter().map(|pack| pack.id.as_str()).collect()
    }
}
