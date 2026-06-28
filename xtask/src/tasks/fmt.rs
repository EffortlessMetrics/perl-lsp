//! Format task implementation

use color_eyre::eyre::{Context, Result, eyre};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct CargoPackage {
    id: String,
    name: String,
    manifest_path: String,
}

/// One crate's failure record collected during the per-crate iteration.
///
/// `unformatted_files` is populated in `--check` mode by parsing rustfmt's
/// `Diff in <path>` lines from stdout; in apply mode it stays empty (cargo
/// fmt without `--check` is expected to mutate files and exit zero unless
/// rustfmt itself errored, which we still report as a per-crate failure).
struct CrateFailure {
    manifest_path: String,
    unformatted_files: Vec<String>,
    spawn_error: Option<String>,
}

pub fn run(check: bool, package_filters: Option<Vec<String>>) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );

    let action = if check { "Checking" } else { "Formatting" };
    spinner.set_message(format!("{} code", action));

    let mut failures: Vec<CrateFailure> = Vec::new();

    for manifest_path in workspace_manifest_paths(package_filters.as_deref())? {
        spinner.set_message(format!("{} {}", action, manifest_path));

        let mut args = vec!["fmt".to_string(), "--manifest-path".to_string(), manifest_path];
        if check {
            args.push("--".to_string());
            args.push("--check".to_string());
        }

        // Capture stdout in --check mode so we can name the unformatted files
        // in the aggregate error report. In apply mode we let cargo's output
        // pass through unchanged so users still see rustfmt warnings live.
        let result = if check {
            cmd("cargo", &args).stdout_capture().unchecked().run()
        } else {
            cmd("cargo", &args).unchecked().run()
        };

        match result {
            Ok(output) => {
                if !output.status.success() {
                    let unformatted_files =
                        if check { parse_unformatted_files(&output.stdout) } else { Vec::new() };
                    // In --check mode, echo the captured stdout so the user
                    // still sees the diff context in addition to the summary.
                    if check && !output.stdout.is_empty() {
                        print!("{}", String::from_utf8_lossy(&output.stdout));
                    }
                    failures.push(CrateFailure {
                        manifest_path: args[2].clone(),
                        unformatted_files,
                        spawn_error: None,
                    });
                }
            }
            Err(err) => {
                // Spawn / I/O failures are kept in the aggregate report
                // rather than aborting on the first crate, so a single run
                // still surfaces every per-crate problem.
                failures.push(CrateFailure {
                    manifest_path: args[2].clone(),
                    unformatted_files: Vec::new(),
                    spawn_error: Some(err.to_string()),
                });
            }
        }
    }

    if failures.is_empty() {
        spinner.finish_with_message(format!(
            "✅ Code {} successfully",
            if check { "check passed" } else { "formatted" }
        ));
        return Ok(());
    }

    spinner.finish_with_message(format!(
        "❌ Code {} failed in {} crate(s)",
        if check { "check" } else { "formatting" },
        failures.len()
    ));
    Err(eyre!("{}", format_failure_report(check, &failures)))
}

/// Parse rustfmt's `Diff in <path>` lines from captured stdout.
///
/// rustfmt emits one of these two header shapes per unformatted file:
///   * `Diff in <path> at line N:`  (older / verbose-diff)
///   * `Diff in <path>:<N>:`        (current default on recent toolchains)
///
/// Pulling the path out lets the aggregate error name every offending file,
/// not just the crate that contains them. Returns a deduplicated,
/// insertion-ordered list (rustfmt may repeat a path across multiple hunks).
fn parse_unformatted_files(stdout: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(stdout);
    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Diff in ") {
            let path = extract_diff_path(rest);
            if !path.is_empty() && seen.insert(path.to_string()) {
                files.push(path.to_string());
            }
        }
    }
    files
}

/// Extract the file path from a rustfmt `Diff in ...` line tail.
///
/// Handles both the `<path> at line N:` and `<path>:<N>:` shapes, plus
/// Windows paths like `\\?\C:\...\lib.rs:11:` where the trailing `:line:`
/// must not be confused with the drive-letter colon earlier in the path.
fn extract_diff_path(rest: &str) -> &str {
    // Verbose-diff shape: `<path> at line N:` — split on the literal marker
    // and ignore the trailing line/column completely.
    if let Some(idx) = rest.rfind(" at line ") {
        return rest[..idx].trim();
    }
    // Default shape: `<path>:<line>:` — strip the trailing `:`, then strip the
    // trailing digit run (line number), then strip the separator `:`. Keeping
    // this lexical (rather than regex) matches the rest of the xtask style.
    let mut s = rest.trim().trim_end_matches(':');
    let stripped = s.trim_end_matches(|c: char| c.is_ascii_digit());
    if stripped.len() < s.len() && stripped.ends_with(':') {
        s = stripped.trim_end_matches(':');
    }
    s.trim()
}

/// Build the aggregate error message that lists every failing crate.
///
/// In `--check` mode the report names each unformatted file under the crate
/// that owns it, replacing the historical generic "Failed to format
/// Cargo.toml" message that masked per-PR drift as a master cascade.
fn format_failure_report(check: bool, failures: &[CrateFailure]) -> String {
    let mut report = String::new();
    let header = if check {
        format!("cargo fmt --check found unformatted files in {} crate(s):", failures.len())
    } else {
        format!("cargo fmt failed in {} crate(s):", failures.len())
    };
    report.push_str(&header);
    for failure in failures {
        report.push_str("\n  - ");
        report.push_str(&failure.manifest_path);
        if let Some(spawn_error) = &failure.spawn_error {
            report.push_str(" (spawn failed: ");
            report.push_str(spawn_error);
            report.push(')');
        }
        for file in &failure.unformatted_files {
            report.push_str("\n      ");
            report.push_str(file);
        }
    }
    report
}

fn workspace_manifest_paths(package_filters: Option<&[String]>) -> Result<Vec<String>> {
    let metadata_json = cmd("cargo", ["metadata", "--format-version", "1", "--no-deps"])
        .read()
        .context("Failed to query cargo metadata for workspace formatting")?;
    let metadata: CargoMetadata =
        serde_json::from_str(&metadata_json).context("Failed to parse cargo metadata JSON")?;

    collect_workspace_manifest_paths(&metadata, package_filters)
}

fn collect_workspace_manifest_paths(
    metadata: &CargoMetadata,
    package_filters: Option<&[String]>,
) -> Result<Vec<String>> {
    let package_by_id: HashMap<_, _> = metadata
        .packages
        .iter()
        .map(|package| (package.id.as_str(), package.manifest_path.clone()))
        .collect();
    let member_name_to_manifest: HashMap<_, _> = metadata
        .packages
        .iter()
        .filter(|package| metadata.workspace_members.iter().any(|member| member == &package.id))
        .map(|package| (package.name.as_str(), package.manifest_path.clone()))
        .collect();

    if let Some(filters) = package_filters {
        let mut selected = Vec::with_capacity(filters.len());
        for package_name in filters {
            if let Some(manifest_path) = member_name_to_manifest.get(package_name.as_str()) {
                selected.push(manifest_path.clone());
            } else {
                // Sort the available list so the error message is stable across runs.
                let mut available: Vec<_> = member_name_to_manifest.keys().copied().collect();
                available.sort_unstable();
                return Err(eyre!(
                    "Unknown package `{package_name}`. Available workspace packages: {}",
                    available.join(", ")
                ));
            }
        }
        return Ok(dedup_preserve_order(selected));
    }

    metadata
        .workspace_members
        .iter()
        .map(|member_id| {
            package_by_id
                .get(member_id.as_str())
                .cloned()
                .ok_or_else(|| eyre!("Workspace member not found in cargo metadata: {member_id}"))
        })
        .collect()
}

fn dedup_preserve_order(paths: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::with_capacity(paths.len());
    let mut deduped = Vec::with_capacity(paths.len());
    for path in paths {
        if seen.insert(path.clone()) {
            deduped.push(path);
        }
    }
    deduped
}

#[cfg(test)]
mod tests {
    use super::{
        CargoMetadata, CargoPackage, CrateFailure, collect_workspace_manifest_paths,
        format_failure_report, parse_unformatted_files,
    };
    use color_eyre::eyre::Result;
    use std::fs;
    use std::path::Path;

    fn sample_metadata() -> CargoMetadata {
        CargoMetadata {
            packages: vec![
                CargoPackage {
                    id: "path+file:///repo/xtask#0.1.0".to_string(),
                    name: "xtask".to_string(),
                    manifest_path: "/repo/xtask/Cargo.toml".to_string(),
                },
                CargoPackage {
                    id: "path+file:///repo/crates/perl-parser#0.1.0".to_string(),
                    name: "perl-parser".to_string(),
                    manifest_path: "/repo/crates/perl-parser/Cargo.toml".to_string(),
                },
            ],
            workspace_members: vec![
                "path+file:///repo/xtask#0.1.0".to_string(),
                "path+file:///repo/crates/perl-parser#0.1.0".to_string(),
            ],
        }
    }

    #[test]
    fn package_filters_select_requested_manifest_paths() -> Result<()> {
        let metadata = sample_metadata();
        let filters = vec!["perl-parser".to_string()];
        let manifests = collect_workspace_manifest_paths(&metadata, Some(&filters))?;
        assert_eq!(manifests, vec!["/repo/crates/perl-parser/Cargo.toml".to_string()]);
        Ok(())
    }

    #[test]
    fn package_filters_are_deduplicated() -> Result<()> {
        let metadata = sample_metadata();
        let filters = vec!["xtask".to_string(), "xtask".to_string()];
        let manifests = collect_workspace_manifest_paths(&metadata, Some(&filters))?;
        assert_eq!(manifests, vec!["/repo/xtask/Cargo.toml".to_string()]);
        Ok(())
    }

    #[test]
    fn package_filters_report_unknown_package() -> Result<()> {
        let metadata = sample_metadata();
        let filters = vec!["missing-package".to_string()];
        let message = match collect_workspace_manifest_paths(&metadata, Some(&filters)) {
            Ok(paths) => {
                return Err(color_eyre::eyre::eyre!("expected error, got paths: {paths:?}"));
            }
            Err(err) => format!("{err}"),
        };
        assert!(message.contains("missing-package"));
        assert!(message.contains("Available workspace packages"));
        Ok(())
    }

    #[test]
    fn package_filters_error_lists_packages_in_stable_sorted_order() -> Result<()> {
        let metadata = sample_metadata();
        let filters = vec!["nonexistent".to_string()];
        let message = match collect_workspace_manifest_paths(&metadata, Some(&filters)) {
            Ok(paths) => {
                return Err(color_eyre::eyre::eyre!("expected error, got paths: {paths:?}"));
            }
            Err(err) => format!("{err}"),
        };
        // The available list must be sorted — both packages appear in alphabetical order.
        let perl_pos = message.find("perl-parser").expect("perl-parser in error");
        let xtask_pos = message.find("xtask").expect("xtask in error");
        assert!(perl_pos < xtask_pos, "available packages must be listed in sorted order");
        Ok(())
    }

    #[test]
    fn parse_unformatted_files_extracts_paths_from_verbose_diff_lines() {
        // Older rustfmt and verbose-diff modes emit `<path> at line N:`.
        let stdout = b"Diff in /repo/crates/foo/src/lib.rs at line 12:\n\
             -    let x = 1;\n\
             +    let x = 1;\n\
             Diff in /repo/crates/bar/src/main.rs at line 3:\n\
             -fn main(){}\n";
        let files = parse_unformatted_files(stdout);
        assert_eq!(
            files,
            vec![
                "/repo/crates/foo/src/lib.rs".to_string(),
                "/repo/crates/bar/src/main.rs".to_string(),
            ]
        );
    }

    #[test]
    fn parse_unformatted_files_extracts_paths_from_default_diff_lines() {
        // Current default rustfmt output: `<path>:<line>:` — must not chop the
        // drive-letter colon out of `\\?\C:\...` style Windows paths.
        let stdout = b"Diff in /repo/crates/foo/src/lib.rs:11:\n\
             -    let x = 1;\n\
             Diff in \\\\?\\C:\\repo\\crates\\bar\\src\\main.rs:42:\n\
             -fn main(){}\n";
        let files = parse_unformatted_files(stdout);
        assert_eq!(
            files,
            vec![
                "/repo/crates/foo/src/lib.rs".to_string(),
                "\\\\?\\C:\\repo\\crates\\bar\\src\\main.rs".to_string(),
            ]
        );
    }

    #[test]
    fn parse_unformatted_files_deduplicates_repeated_paths() {
        let stdout = b"Diff in /repo/a/src/lib.rs at line 1:\n\
             Diff in /repo/a/src/lib.rs at line 42:\n";
        let files = parse_unformatted_files(stdout);
        assert_eq!(files, vec!["/repo/a/src/lib.rs".to_string()]);
    }

    #[test]
    fn parse_unformatted_files_returns_empty_for_clean_output() {
        let files = parse_unformatted_files(b"");
        assert!(files.is_empty());
        let files = parse_unformatted_files(b"some unrelated cargo output\n");
        assert!(files.is_empty());
    }

    #[test]
    fn format_failure_report_lists_every_failing_crate_in_check_mode() {
        let failures = vec![
            CrateFailure {
                manifest_path: "crates/foo/Cargo.toml".to_string(),
                unformatted_files: vec!["crates/foo/src/lib.rs".to_string()],
                spawn_error: None,
            },
            CrateFailure {
                manifest_path: "crates/bar/Cargo.toml".to_string(),
                unformatted_files: vec![
                    "crates/bar/src/lib.rs".to_string(),
                    "crates/bar/src/util.rs".to_string(),
                ],
                spawn_error: None,
            },
        ];
        let report = format_failure_report(true, &failures);
        // Both crates and every unformatted file must appear so operators can
        // distinguish per-PR drift from a shared master cascade in one read.
        assert!(report.contains("2 crate(s)"));
        assert!(report.contains("crates/foo/Cargo.toml"));
        assert!(report.contains("crates/bar/Cargo.toml"));
        assert!(report.contains("crates/foo/src/lib.rs"));
        assert!(report.contains("crates/bar/src/lib.rs"));
        assert!(report.contains("crates/bar/src/util.rs"));
        // Regression: never emit the original misleading generic message.
        assert!(!report.contains("Failed to format Cargo.toml"));
    }

    #[test]
    fn format_failure_report_surfaces_spawn_errors_inline() {
        let failures = vec![CrateFailure {
            manifest_path: "crates/foo/Cargo.toml".to_string(),
            unformatted_files: Vec::new(),
            spawn_error: Some("rustfmt not found".to_string()),
        }];
        let report = format_failure_report(false, &failures);
        assert!(report.contains("cargo fmt failed"));
        assert!(report.contains("crates/foo/Cargo.toml"));
        assert!(report.contains("rustfmt not found"));
    }

    #[test]
    fn xtask_tasks_do_not_shell_out_to_workspace_wide_cargo_fmt_all() -> Result<()> {
        let xtask_tasks = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("tasks");
        let mut offenders = Vec::new();
        collect_workspace_fmt_all_offenders(&xtask_tasks, &mut offenders)?;

        assert!(
            offenders.is_empty(),
            "repo-owned xtask gates must route formatting through fmt::run, not raw workspace fmt: {offenders:?}"
        );
        Ok(())
    }

    fn collect_workspace_fmt_all_offenders(dir: &Path, offenders: &mut Vec<String>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_workspace_fmt_all_offenders(&path, offenders)?;
                continue;
            }

            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }

            let source = fs::read_to_string(&path)?;
            if path.file_name().and_then(|name| name.to_str()) == Some("fmt.rs") {
                continue;
            }

            let argv_pattern = ["\"fmt\"", ", \"--all\""].concat();
            let shell_pattern = ["cargo fmt", " --all"].concat();
            let args_pattern = ["args([\"fmt\"", ", \"--all\""].concat();
            if source.contains(&argv_pattern)
                || source.contains(&shell_pattern)
                || source.contains(&args_pattern)
            {
                offenders.push(path.display().to_string());
            }
        }

        Ok(())
    }
}
