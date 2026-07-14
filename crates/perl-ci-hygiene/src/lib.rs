//! Re-export binary functionality for testing.
//!
//! This module is primarily used to enable `cargo test --lib` CI runs.
//! The primary entry point is the binary in `main.rs`.

#![warn(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub mod version_sync;

/// Cargo package name for this crate, used to locate its build artifacts and source tree.
pub const PACKAGE_NAME: &str = "perl-ci-hygiene";

/// Resolve the on-disk path of this crate's debug binary inside `root`'s `target/` directory.
///
/// Appends the platform executable extension on Windows.
#[must_use]
pub fn binary_path(root: &Path) -> PathBuf {
    let mut path = root.join("target").join("debug").join(PACKAGE_NAME);
    if cfg!(windows) {
        path.set_extension(std::env::consts::EXE_EXTENSION);
    }
    path
}

/// Returns `true` if `path` should be treated as a Rust source file.
///
/// A file is a Rust source only when it has a `.rs` extension.  Files with no
/// extension (e.g. `README`, `Makefile`, `LICENSE`) and files with any other
/// extension (e.g. `.toml`, `.md`) return `false` and are excluded from
/// Rust-source walks.
///
/// This is the canonical include-predicate used by all Rust-source walks in the
/// CI hygiene tool.  Using a positive include test (`ext == "rs"`) rather than a
/// negative skip test (`ext != "rs"`) avoids the subtle bug where
/// `is_some_and(|ext| ext != "rs")` returns `false` for extensionless files,
/// accidentally admitting them into the walk.
#[must_use]
pub fn is_rust_source_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "rs")
}

/// Walk `root` recursively and return the paths of all `.rs` files found.
///
/// Non-file entries (directories, symlinks) and files that do not have a `.rs`
/// extension — including extensionless files like `README`, `Makefile`, and
/// `LICENSE` — are silently excluded.
///
/// This is the canonical directory walker used by all Rust-source scans in the
/// CI hygiene tool.  Using [`is_rust_source_file`] as the include predicate here
/// ensures every walk site enforces the same extension check, and the check is
/// covered by `--lib` tests even though the individual walk functions live in the
/// binary entry point.
#[must_use]
pub fn walk_rs_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if entry.file_type().is_file() && is_rust_source_file(path) {
                Some(path.to_path_buf())
            } else {
                None
            }
        })
        .collect()
}

/// Collect the `Cargo.toml` plus every Rust source file shipped by this crate,
/// sorted lexicographically.
///
/// `root` is the workspace root; the crate is located at `root/crates/perl-ci-hygiene`.
#[must_use]
pub fn source_paths(root: &Path) -> Vec<PathBuf> {
    let crate_root = root.join("crates").join(PACKAGE_NAME);
    let mut paths = vec![crate_root.join("Cargo.toml")];
    collect_rust_sources(&crate_root.join("src"), &mut paths);
    paths.sort();
    paths
}

fn collect_rust_sources(dir: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, paths);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            paths.push(path);
        }
    }
}

/// Categorize an `#[ignore]` reason/context into a policy bucket.
pub fn categorize_ignore(reason: &str, context: &str) -> String {
    let reason = reason.trim().to_lowercase();
    let context = context.to_lowercase();
    let reason_no_space = reason.replace(' ', "");

    if reason.starts_with("manual:")
        || reason.contains("manual ")
        || reason.contains("regenerate")
        || reason.contains("helper")
    {
        return "manual".to_string();
    }
    if reason.starts_with("stress:")
        || reason.contains("stress test")
        || reason.contains("memory.stress")
        || reason.contains("performance.stress")
        || reason.contains("load.test")
        || reason.contains("stack.overflow")
        || reason.contains("designed.to.fail")
    {
        return "stress".to_string();
    }
    if reason.starts_with("bug:")
        || reason.contains("bug:")
        || reason.contains("known.bug")
        || reason.contains("regression")
        || reason.contains("incorrect.behavior")
        || reason.contains("parser.bug")
        || reason.contains("missing.notification")
        || reason.contains("missing.initialize")
        || reason.contains("server.returns.instead")
        || reason.contains("will.kill")
        || reason.contains("known.inconsistencies")
        || reason.contains("mut_")
        || reason.contains("matching.issue")
        || reason.contains("investigate")
        || reason.contains("instead.of.expected")
        || reason.contains("different.error.format")
        || reason.contains("expects")
    {
        return "bug".to_string();
    }
    if reason.starts_with("todo:")
        || reason_no_space.starts_with("todo(#")
        || reason.starts_with("infra:")
        || reason.contains("infra ")
        || reason.contains("fixme")
        || reason.contains("needs")
        || reason.contains("requires")
        || reason.contains("setup")
        || reason.contains("config")
        || reason.contains("environment")
        || reason.contains("run.with")
        || reason.contains("only.run.after")
        || reason.contains("only.run.when")
    {
        return "infra".to_string();
    }
    if reason.starts_with("feature:")
        || reason.contains("feature ")
        || reason.contains("not.implemented")
        || reason.contains("unimplemented")
        || reason.contains("wip")
        || reason.contains("work.in.progress")
        || reason.contains("pending")
        || reason.contains("when.implemented")
        || reason.contains("remove.when")
        || reason.contains("ac:")
        || reason.contains("ac ")
        || reason.contains("not.yet")
        || reason.contains("tdd.scaffold")
        || reason.contains("scaffold")
        || reason.contains("doesn.t.support")
        || reason.contains("doesn't.support")
        || reason.contains("parser.limitation")
        || reason.contains("expected.to.fail")
        || reason.contains("not.fully.supported")
        || reason.contains("enable.after")
        || reason.contains("after.phase")
        || reason.contains("parser.doesn")
        || reason.contains("tracked in #")
    {
        return "feature".to_string();
    }
    if reason.starts_with("brokenpipe:")
        || reason.contains("brokenpipe ")
        || reason.contains("broken.pipe")
        || reason.contains("transport.error")
        || reason.contains("transport.flake")
        || reason.contains("flaky")
    {
        return "brokenpipe".to_string();
    }
    if reason.contains("protocol")
        || reason.contains("lsp")
        || reason.contains("dap")
        || reason.contains("compliance")
        || reason.contains("specification")
    {
        return "protocol".to_string();
    }
    if reason.contains("tracked in #") {
        return "feature".to_string();
    }
    if reason.contains("doesn.t.have.field")
        || reason.contains("may.not.produce")
        || reason.contains("doesn.t.yet")
        || reason.contains("fewer.than.expected")
    {
        return "feature".to_string();
    }
    if reason.contains("recursion.limit.behavior") || reason.contains("behavior.changed") {
        return "feature".to_string();
    }
    if reason.contains("integration.test.that.spawns")
        || reason.contains("spawns.external")
        || reason.contains("burn.down")
        || reason.contains("mutation.hardening")
    {
        return "infra".to_string();
    }
    if reason.contains("clippy.warnings") || reason.contains("warnings.burn") {
        return "infra".to_string();
    }
    if reason.starts_with("ac:") {
        return "feature".to_string();
    }
    if reason.is_empty() || reason == "ignore" {
        return "bare".to_string();
    }
    if context.contains("ac:") {
        return "feature".to_string();
    }
    "other".to_string()
}

#[cfg(test)]
mod tests {
    use super::{categorize_ignore, source_paths};
    use crate::version_sync::{is_pre_release, validate_version_format};
    use std::error::Error;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_repo_dir(label: &str) -> Result<PathBuf, Box<dyn Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let dir = std::env::temp_dir()
            .join(format!("perl-ci-hygiene-lib-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn contains_path(paths: &[PathBuf], suffix: &Path) -> bool {
        paths.iter().any(|path| path.ends_with(suffix))
    }

    #[test]
    fn categorize_ignore_maps_documented_policy_buckets() -> Result<(), String> {
        let cases = [
            ("manual: regenerate snapshots", "", "manual"),
            ("stress: memory.stress load.test", "", "stress"),
            ("bug: parser.bug missing.initialize", "", "bug"),
            ("infra: requires setup", "", "infra"),
            ("feature: not implemented", "", "feature"),
            ("brokenpipe: transport.flake", "", "brokenpipe"),
            ("LSP protocol compliance fixture", "", "protocol"),
            ("ignore", "", "bare"),
            ("", "", "bare"),
            ("needs triage", "AC: parser support", "infra"),
            ("unknown skip reason", "AC: parser support", "feature"),
            ("unknown skip reason", "ordinary context", "other"),
        ];

        for (reason, context, expected) in cases {
            let actual = categorize_ignore(reason, context);
            if actual != expected {
                return Err(format!(
                    "categorize_ignore({reason:?}, {context:?}) returned {actual:?}, expected {expected:?}"
                ));
            }
        }

        Ok(())
    }

    #[test]
    fn categorize_ignore_handles_punctuation_normalized_legacy_reasons() -> Result<(), String> {
        let cases = [
            ("todo(#1234)", "infra"),
            ("doesn.t.have.field", "feature"),
            ("may.not.produce", "feature"),
            ("recursion.limit.behavior", "feature"),
            ("integration.test.that.spawns", "infra"),
            ("clippy.warnings", "infra"),
        ];

        for (reason, expected) in cases {
            let actual = categorize_ignore(reason, "");
            if actual != expected {
                return Err(format!(
                    "categorize_ignore({reason:?}, \"\") returned {actual:?}, expected {expected:?}"
                ));
            }
        }

        Ok(())
    }

    #[test]
    fn validate_version_format_accepts_stable_and_pre_release_versions() -> Result<(), String> {
        for version in ["0.15.0", "1.2.3-rc1", "1.2.3-beta.2", "10.20.30-alpha-1"] {
            validate_version_format(version)
                .map_err(|err| format!("{version:?} should be valid: {err}"))?;
        }

        Ok(())
    }

    #[test]
    fn validate_version_format_rejects_malformed_versions() -> Result<(), String> {
        for version in ["1", "1.2", "1.2.3.4", "1.2.x", "1.2.3-", "1.2.3+build"] {
            if validate_version_format(version).is_ok() {
                return Err(format!("{version:?} should be rejected"));
            }
        }

        Ok(())
    }

    #[test]
    fn is_pre_release_only_tracks_dash_suffixes() -> Result<(), String> {
        let cases = [("0.15.0", false), ("0.15.0-rc1", true), ("0.15.0-beta.2", true)];

        for (version, expected) in cases {
            let actual = is_pre_release(version);
            if actual != expected {
                return Err(format!(
                    "is_pre_release({version:?}) returned {actual}, expected {expected}"
                ));
            }
        }

        Ok(())
    }

    #[test]
    fn source_paths_include_split_rust_modules() -> Result<(), Box<dyn Error>> {
        let root = unique_temp_repo_dir("source-paths")?;
        let crate_root = root.join("crates").join("perl-ci-hygiene");
        let commands_dir = crate_root.join("src").join("commands");
        fs::create_dir_all(&commands_dir)?;
        fs::write(crate_root.join("Cargo.toml"), "")?;
        fs::write(crate_root.join("src").join("main.rs"), "")?;
        fs::write(crate_root.join("src").join("process.rs"), "")?;
        fs::write(commands_dir.join("mod.rs"), "")?;

        let paths = source_paths(&root);
        assert!(contains_path(&paths, Path::new("crates/perl-ci-hygiene/Cargo.toml")));
        assert!(contains_path(&paths, Path::new("crates/perl-ci-hygiene/src/main.rs")));
        assert!(contains_path(&paths, Path::new("crates/perl-ci-hygiene/src/process.rs")));
        assert!(contains_path(&paths, Path::new("crates/perl-ci-hygiene/src/commands/mod.rs")));

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    /// Regression guard for #2074: extensionless files (README, Makefile, LICENSE)
    /// must NOT be included in Rust-source walks.
    ///
    /// Before the fix, `is_some_and(|ext| ext != "rs")` was used as a skip
    /// predicate: it returns `false` for extensionless files, so they slipped
    /// through into the Rust-source walk.  The correct predicate is a positive
    /// inclusion test: `is_some_and(|ext| ext == "rs")`.
    #[test]
    fn is_rust_source_file_excludes_extensionless_files() -> Result<(), Box<dyn Error>> {
        use super::is_rust_source_file;
        use std::path::Path;

        // Must include .rs files.
        assert!(
            is_rust_source_file(Path::new("lib.rs")),
            "lib.rs must be included as a Rust source"
        );
        assert!(
            is_rust_source_file(Path::new("src/main.rs")),
            "src/main.rs must be included as a Rust source"
        );

        // Must exclude extensionless files (#2074 regression guard).
        assert!(
            !is_rust_source_file(Path::new("README")),
            "README (no extension) must not be treated as a Rust source"
        );
        assert!(
            !is_rust_source_file(Path::new("Makefile")),
            "Makefile (no extension) must not be treated as a Rust source"
        );
        assert!(
            !is_rust_source_file(Path::new("LICENSE")),
            "LICENSE (no extension) must not be treated as a Rust source"
        );

        // Must exclude other-extension files.
        assert!(
            !is_rust_source_file(Path::new("Cargo.toml")),
            "Cargo.toml must not be treated as a Rust source"
        );
        assert!(
            !is_rust_source_file(Path::new("script.py")),
            "script.py must not be treated as a Rust source"
        );
        Ok(())
    }

    /// Regression guard for #2074: `walk_rs_files` must return only `.rs` files.
    ///
    /// Exercises the `else { None }` branch of `walk_rs_files` with extensionless
    /// files (`README`, `Makefile`, `LICENSE`) and non-`.rs` files (`Cargo.toml`,
    /// `build.sh`), confirming those are excluded and only `.rs` files are returned.
    #[test]
    fn walk_rs_files_excludes_extensionless_and_non_rs_files() -> Result<(), Box<dyn Error>> {
        use super::walk_rs_files;

        let root = unique_temp_repo_dir("walk-rs-files")?;

        // Create .rs files that must be included.
        let src = root.join("src");
        fs::create_dir_all(&src)?;
        fs::write(src.join("main.rs"), "fn main() {}")?;
        fs::write(src.join("lib.rs"), "")?;

        // Create extensionless files that must be excluded (#2074 regression guard).
        fs::write(root.join("README"), "readme text")?;
        fs::write(root.join("Makefile"), "all:\n\t@echo ok")?;
        fs::write(root.join("LICENSE"), "")?;

        // Create other-extension files that must be excluded.
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"")?;
        fs::write(root.join("build.sh"), "#!/bin/bash")?;

        let rs_files = walk_rs_files(&root);

        assert_eq!(
            rs_files.len(),
            2,
            "only main.rs and lib.rs should be returned; got: {rs_files:?}"
        );
        assert!(rs_files.iter().any(|p| p.ends_with("main.rs")), "main.rs must be included");
        assert!(rs_files.iter().any(|p| p.ends_with("lib.rs")), "lib.rs must be included");
        assert!(
            !rs_files.iter().any(|p| p.ends_with("README")),
            "README must not be included (extensionless — #2074)"
        );
        assert!(
            !rs_files.iter().any(|p| p.ends_with("Makefile")),
            "Makefile must not be included (extensionless — #2074)"
        );
        assert!(
            !rs_files.iter().any(|p| p.ends_with("Cargo.toml")),
            "Cargo.toml must not be included"
        );

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn binary_path_points_at_debug_artifact_named_for_the_package() {
        use super::{PACKAGE_NAME, binary_path};

        let path = binary_path(Path::new("/tmp/example-root"));

        // Always resolves under <root>/target/debug/.
        assert!(
            path.starts_with(Path::new("/tmp/example-root/target/debug")),
            "binary lives under target/debug: {path:?}"
        );
        // The artifact stem is the package name on every platform.
        assert_eq!(path.file_stem().and_then(|stem| stem.to_str()), Some(PACKAGE_NAME));

        // Windows carries the .exe extension; other platforms carry none.
        #[cfg(windows)]
        assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("exe"));
        #[cfg(not(windows))]
        assert!(path.extension().is_none(), "no extension off Windows: {path:?}");
    }
}
