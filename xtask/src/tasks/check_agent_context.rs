//! Package-local agent-context coverage gate (M6, issue #3848 / epic #3612).
//!
//! Every workspace member (as reported by `cargo metadata --no-deps`) must be
//! accounted for as exactly one of:
//!   - **has-context**: a `CLAUDE.md` file exists in the crate directory.
//!   - **exempt**: a genuine infra/test-only crate with no product surface to
//!     document, listed in `.ci/policies/agent-context-policy.toml`. Permanent.
//!   - **needs_context**: a core-product crate that should have package-local
//!     context but does not yet, also listed in the policy file. This is
//!     explicit tracked debt -- the validator prints it loudly on every run
//!     and never treats it as satisfying the gate silently.
//!
//! A member found in none of the three buckets is **unaccounted** and fails
//! the gate. This is deliberate: it is the difference between "every crate
//! is accounted for, and the gap is visible" (honest) and "the check is
//! green because we quietly exempted the crates that should have context"
//! (gamed). See the M4-advisory lesson referenced in the M6 spec.

use crate::utils::{project_root, run_cargo_metadata};
use color_eyre::eyre::{Result, bail, eyre};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const POLICY_PATH: &str = ".ci/policies/agent-context-policy.toml";
const CONTEXT_FILE_NAME: &str = "CLAUDE.md";

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<MetadataPackage>,
}

#[derive(Debug, Deserialize)]
struct MetadataPackage {
    name: String,
    manifest_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct AgentContextPolicy {
    version: u64,
    #[serde(default)]
    exempt: Vec<PolicyEntry>,
    #[serde(default)]
    needs_context: Vec<PolicyEntry>,
}

#[derive(Debug, Deserialize)]
struct PolicyEntry {
    name: String,
    reason: String,
    #[serde(default)]
    tracking_issue: Option<u64>,
}

/// A workspace member reduced to what classification needs: its name, and
/// whether a package-local `CLAUDE.md` already exists for it.
#[derive(Debug, Clone)]
struct Member {
    name: String,
    has_context: bool,
}

/// Result of classifying every workspace member against the policy file.
#[derive(Debug, Default)]
struct Coverage {
    total: usize,
    has_context: Vec<String>,
    exempt: Vec<String>,
    needs_context: Vec<String>,
    unaccounted: Vec<String>,
}

pub fn run() -> Result<()> {
    let root = project_root()?;
    let policy = load_policy(&root.join(POLICY_PATH))?;
    let members = load_members(&root)?;
    let coverage = classify(&members, &policy);

    print_report(&coverage, &policy);

    if !coverage.unaccounted.is_empty() {
        bail!(
            "agent-context gate: {} workspace member(s) unaccounted (neither {CONTEXT_FILE_NAME}, exempt, nor needs_context in {POLICY_PATH}): {}",
            coverage.unaccounted.len(),
            coverage.unaccounted.join(", ")
        );
    }

    Ok(())
}

fn print_report(coverage: &Coverage, policy: &AgentContextPolicy) {
    let accounted =
        coverage.has_context.len() + coverage.exempt.len() + coverage.needs_context.len();
    println!(
        "agent-context coverage: {accounted}/{} workspace members accounted for",
        coverage.total
    );
    println!("  has {CONTEXT_FILE_NAME}:  {}", coverage.has_context.len());
    println!("  exempt (infra/test-only): {}", coverage.exempt.len());
    println!("  needs_context (core-product debt): {}", coverage.needs_context.len());

    if !coverage.needs_context.is_empty() {
        println!();
        println!(
            "TRACKED CONTEXT DEBT -- M6 (issue #3848) is NOT complete until this list is empty:"
        );
        for name in &coverage.needs_context {
            let issue = policy
                .needs_context
                .iter()
                .find(|entry| entry.name == *name)
                .and_then(|entry| entry.tracking_issue);
            match issue {
                Some(number) => println!("  - {name} (tracked in #{number})"),
                None => println!("  - {name}"),
            }
        }
    }

    if !coverage.unaccounted.is_empty() {
        println!();
        println!("UNACCOUNTED -- add {CONTEXT_FILE_NAME}, or classify in {POLICY_PATH}:");
        for name in &coverage.unaccounted {
            println!("  - {name}");
        }
    }
}

fn load_policy(path: &Path) -> Result<AgentContextPolicy> {
    let content = fs::read_to_string(path)
        .map_err(|error| eyre!("failed to read {}: {error}", path.display()))?;
    let policy: AgentContextPolicy = toml::from_str(&content)
        .map_err(|error| eyre!("failed to parse {}: {error}", path.display()))?;
    validate_policy(&policy, path)?;
    Ok(policy)
}

fn validate_policy(policy: &AgentContextPolicy, path: &Path) -> Result<()> {
    if policy.version != 1 {
        bail!("{} version must be 1", path.display());
    }

    let mut seen = BTreeSet::new();
    for entry in policy.exempt.iter().chain(policy.needs_context.iter()) {
        if entry.name.trim().is_empty() {
            bail!("{} has an entry with an empty name", path.display());
        }
        if entry.reason.trim().is_empty() {
            bail!("{} entry {} must have a non-empty reason", path.display(), entry.name);
        }
        if !seen.insert(entry.name.as_str()) {
            bail!(
                "{} lists {} more than once (exempt and needs_context must be disjoint, and each list must be duplicate-free)",
                path.display(),
                entry.name
            );
        }
    }

    Ok(())
}

fn load_members(root: &Path) -> Result<Vec<Member>> {
    let bytes = run_cargo_metadata(true)?;
    let metadata: CargoMetadata = serde_json::from_slice(&bytes)
        .map_err(|error| eyre!("failed to parse cargo metadata: {error}"))?;

    let mut members = Vec::with_capacity(metadata.packages.len());
    for package in metadata.packages {
        let crate_dir = package
            .manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.to_path_buf());
        let has_context = crate_dir.join(CONTEXT_FILE_NAME).is_file();
        members.push(Member { name: package.name, has_context });
    }
    Ok(members)
}

fn classify(members: &[Member], policy: &AgentContextPolicy) -> Coverage {
    let exempt_names: BTreeSet<&str> =
        policy.exempt.iter().map(|entry| entry.name.as_str()).collect();
    let needs_context_names: BTreeSet<&str> =
        policy.needs_context.iter().map(|entry| entry.name.as_str()).collect();

    let mut coverage = Coverage { total: members.len(), ..Coverage::default() };

    for member in members {
        if member.has_context {
            coverage.has_context.push(member.name.clone());
        } else if exempt_names.contains(member.name.as_str()) {
            coverage.exempt.push(member.name.clone());
        } else if needs_context_names.contains(member.name.as_str()) {
            coverage.needs_context.push(member.name.clone());
        } else {
            coverage.unaccounted.push(member.name.clone());
        }
    }

    coverage.has_context.sort();
    coverage.exempt.sort();
    coverage.needs_context.sort();
    coverage.unaccounted.sort();

    coverage
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(exempt: &[&str], needs_context: &[&str]) -> AgentContextPolicy {
        AgentContextPolicy {
            version: 1,
            exempt: exempt
                .iter()
                .map(|name| PolicyEntry {
                    name: (*name).to_string(),
                    reason: "test reason".to_string(),
                    tracking_issue: None,
                })
                .collect(),
            needs_context: needs_context
                .iter()
                .map(|name| PolicyEntry {
                    name: (*name).to_string(),
                    reason: "test reason".to_string(),
                    tracking_issue: Some(3874),
                })
                .collect(),
        }
    }

    fn member(name: &str, has_context: bool) -> Member {
        Member { name: name.to_string(), has_context }
    }

    #[test]
    fn member_with_context_is_accounted_for() {
        let members = vec![member("has-context-crate", true)];
        let policy = policy(&[], &[]);

        let coverage = classify(&members, &policy);

        assert_eq!(coverage.has_context, vec!["has-context-crate".to_string()]);
        assert!(coverage.exempt.is_empty());
        assert!(coverage.needs_context.is_empty());
        assert!(coverage.unaccounted.is_empty());
    }

    #[test]
    fn exempt_member_without_context_is_accounted_for() {
        let members = vec![member("perl-test-must", false)];
        let policy = policy(&["perl-test-must"], &[]);

        let coverage = classify(&members, &policy);

        assert!(coverage.has_context.is_empty());
        assert_eq!(coverage.exempt, vec!["perl-test-must".to_string()]);
        assert!(coverage.needs_context.is_empty());
        assert!(coverage.unaccounted.is_empty());
    }

    #[test]
    fn needs_context_member_passes_with_debt_printed() {
        let members = vec![member("perllsp", false)];
        let policy = policy(&[], &["perllsp"]);

        let coverage = classify(&members, &policy);

        assert!(coverage.has_context.is_empty());
        assert!(coverage.exempt.is_empty());
        assert_eq!(coverage.needs_context, vec!["perllsp".to_string()]);
        assert!(coverage.unaccounted.is_empty());
        // needs_context does not fail the gate on its own -- only unaccounted does.
    }

    #[test]
    fn truly_unaccounted_member_fails() {
        let members = vec![member("brand-new-crate", false)];
        let policy = policy(&["perl-test-must"], &["perllsp"]);

        let coverage = classify(&members, &policy);

        assert_eq!(coverage.unaccounted, vec!["brand-new-crate".to_string()]);
    }

    #[test]
    fn mixed_membership_classifies_each_bucket_independently() {
        let members = vec![
            member("perl-ast", true),
            member("perl-test-must", false),
            member("perllsp", false),
            member("mystery-crate", false),
        ];
        let policy = policy(&["perl-test-must"], &["perllsp"]);

        let coverage = classify(&members, &policy);

        assert_eq!(coverage.has_context, vec!["perl-ast".to_string()]);
        assert_eq!(coverage.exempt, vec!["perl-test-must".to_string()]);
        assert_eq!(coverage.needs_context, vec!["perllsp".to_string()]);
        assert_eq!(coverage.unaccounted, vec!["mystery-crate".to_string()]);
    }

    #[test]
    fn validate_policy_rejects_wrong_version() {
        let mut bad = policy(&[], &[]);
        bad.version = 2;

        let result = validate_policy(&bad, Path::new("policy.toml"));

        assert!(result.is_err());
    }

    #[test]
    fn validate_policy_rejects_duplicate_names_across_lists() -> Result<()> {
        let bad = policy(&["dup-crate"], &["dup-crate"]);

        let result = validate_policy(&bad, Path::new("policy.toml"));

        let Err(error) = result else {
            bail!("duplicate name across exempt/needs_context should be rejected");
        };
        assert!(error.to_string().contains("more than once"));
        Ok(())
    }

    #[test]
    fn validate_policy_rejects_empty_reason() {
        let mut bad = policy(&[], &[]);
        bad.exempt.push(PolicyEntry {
            name: "some-crate".to_string(),
            reason: String::new(),
            tracking_issue: None,
        });

        let result = validate_policy(&bad, Path::new("policy.toml"));

        assert!(result.is_err());
    }

    #[test]
    fn real_policy_file_parses_and_is_internally_consistent() -> Result<()> {
        let root = project_root()?;
        let policy = load_policy(&root.join(POLICY_PATH))?;

        assert!(!policy.exempt.is_empty(), "expected at least one exempt entry");
        assert!(!policy.needs_context.is_empty(), "expected at least one needs_context entry");
        Ok(())
    }

    #[test]
    fn real_workspace_has_no_unaccounted_members() -> Result<()> {
        let root = project_root()?;
        let policy = load_policy(&root.join(POLICY_PATH))?;
        let members = load_members(&root)?;

        let coverage = classify(&members, &policy);

        assert!(
            coverage.unaccounted.is_empty(),
            "unaccounted workspace members found (add {CONTEXT_FILE_NAME} or classify in {POLICY_PATH}): {:?}",
            coverage.unaccounted
        );
        Ok(())
    }
}
