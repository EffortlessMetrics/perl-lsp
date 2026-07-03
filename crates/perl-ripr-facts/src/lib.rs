//! Batch exporter for the `ripr-perl-facts-v1` packet.
//!
//! This crate owns the `ripr-facts` unit that previously lived inside the LSP
//! server runtime (`perl-lsp-rs::ripr_facts_emitter` + the packet-assembly
//! helpers in `perl-lsp-rs::cli`). It was relocated here behavior-preserving so
//! that RIPR fact production sits **below** the editor/LSP stack and **above**
//! the raw parser — see `README.md` for the dependency contract.
//!
//! The public entry point is [`run_ripr_facts`]: it validates the CLI-shaped
//! inputs, runs the (currently conservative, string-scan) emitter, assembles
//! the `ripr-perl-facts-v1` packet, and writes it to disk. The `perl-lsp` /
//! `perllsp` binaries call it directly as a thin wrapper.
//!
//! Subsequent slices replace the string-scan emitter with `perl-workspace` /
//! `perl-semantic-facts`-backed extraction and add a structured batch API; this
//! crate is the home for that work.

mod emitter;

use emitter::{
    emit_boundaries_and_commands, emit_relations_and_discriminators, emit_tests_and_oracles,
};

/// Expected schema version for `ripr-perl-facts-v1` packets.
const EXPECTED_RIPR_FACTS_SCHEMA: &str = "ripr-perl-facts-v1";

/// Run the `ripr-facts` exporter (Campaign 31, ripr-swarm#1379).
///
/// Validates the CLI surface + arg constraints, runs the emitter, assembles the
/// packet, and writes it to `out`. Returns the process exit code (`0` on
/// success, `1` on any validation or write failure).
///
/// Relocated from `perl-lsp-rs::cli::run_ripr_facts` behavior-preserving; the
/// emitter body (mapping the workspace fact substrate into the packet shape)
/// is upgraded in later slices.
#[expect(
    clippy::print_stderr,
    reason = "ripr-facts is a batch CLI unit — user-facing diagnostics intentionally use stderr"
)]
pub fn run_ripr_facts(
    schema: &str,
    root: &str,
    base: Option<&str>,
    head: Option<&str>,
    fact_classes: &str,
    out: &str,
) -> i32 {
    // Validate schema version.
    if schema != EXPECTED_RIPR_FACTS_SCHEMA {
        eprintln!(
            "ripr-facts: unsupported schema `{schema}`; expected `{EXPECTED_RIPR_FACTS_SCHEMA}`"
        );
        return 1;
    }

    // Validate root is repo-relative (forward-slash, no host/drive/temp).
    if let Err(reason) = validate_ripr_facts_path(root, "root") {
        eprintln!("ripr-facts: {reason}");
        return 1;
    }

    // Validate out path.
    if let Err(reason) = validate_ripr_facts_path(out, "out") {
        eprintln!("ripr-facts: {reason}");
        return 1;
    }

    // Validate + normalize fact classes.
    let normalized_classes = match normalize_fact_classes(fact_classes) {
        Ok(classes) => classes,
        Err(reason) => {
            eprintln!("ripr-facts: {reason}");
            return 1;
        }
    };

    // Emit the packet. PR 6 (perl-lsp-swarm#2593) adds test + oracle emission;
    // files/owners/changes (PR 5) + relations/discriminators (PR 7) + boundaries
    // (PR 8) land in subsequent PRs. When tests are found, upgrade packet_status
    // from `unavailable` to `partial` (some fact classes are populated).
    let (tests, oracles) = emit_tests_and_oracles(root);
    let has_test_facts = !tests.is_empty();

    // PR 7 (perl-lsp-swarm#2594): emit relations + concrete discriminators +
    // observed-sink facts.
    let (relations, _changed_observables, _observed_sinks) =
        emit_relations_and_discriminators(root, &tests, &oracles);
    let has_relation_facts = !relations.is_empty();

    // PR 8 (perl-lsp-swarm#2595): emit dynamic boundaries + limitations +
    // typed verify-command candidates.
    let (boundaries, boundary_limitations, verify_commands) = emit_boundaries_and_commands(root);
    let has_boundary_facts = !boundaries.is_empty();

    // P2 (Campaign 31): emit diff-derived changes with concrete discriminators.
    // For now, scan .pm files for changed lines (no git diff available in
    // batch mode; future managed-producer mode will supply a real diff).
    // Emit empty changes[] when no diff is available — the packet stays partial.
    let changes = if base.is_some() {
        // In managed mode, a diff would be available. For now emit empty.
        Vec::new()
    } else {
        Vec::new()
    };
    let has_change_facts = !changes.is_empty();

    let mut packet = build_unavailable_packet(schema, root, base, head, &normalized_classes);

    // Populate tests + oracles arrays.
    packet["tests"] = serde_json::Value::Array(tests);
    packet["oracles"] = serde_json::Value::Array(oracles);

    // Populate relations array (PR 7).
    packet["relations"] = serde_json::Value::Array(relations);

    // Populate changes array (P2).
    packet["changes"] = serde_json::Value::Array(changes);

    // Populate dynamic_boundaries + verify_commands arrays (PR 8).
    packet["dynamic_boundaries"] = serde_json::Value::Array(boundaries);
    packet["verify_commands"] = serde_json::Value::Array(verify_commands);

    // Upgrade status + merge limitations if we found any facts.
    if has_test_facts || has_relation_facts || has_boundary_facts || has_change_facts {
        packet["packet_status"] = serde_json::json!("partial");
        // Merge boundary limitations with the emitter-partial limitation.
        let mut all_limitations = boundary_limitations;
        all_limitations.push(serde_json::json!({
            "limitation_id": "emitter-partial",
            "kind": "partial_emitter",
            "message": "PRs 6-8 landed (tests/oracles, relations/discriminators, boundaries/commands). Files/owners/changes (PR 5) still not yet emitted.",
            "evidence_refs": []
        }));
        packet["limitations"] = serde_json::Value::Array(all_limitations);
    }

    // Write the packet to the output path.
    if let Err(error) = write_packet(out, &packet) {
        eprintln!("ripr-facts: failed to write packet to `{out}`: {error}");
        return 1;
    }

    let status = packet["packet_status"].as_str().unwrap_or("unknown");
    eprintln!("ripr-facts: wrote {status} packet to `{out}`");
    0
}

/// Validate a path is repo-relative: forward-slash, no host/drive/temp prefix,
/// no `..` escape, no leading `/` or `./`.
fn validate_ripr_facts_path(path: &str, field: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err(format!("`{field}` must not be empty"));
    }
    if path.starts_with('/') {
        return Err(format!("`{field}` must be repo-relative, not absolute: `{path}`"));
    }
    if path.starts_with("./") {
        return Err(format!("`{field}` must not start with `./`: `{path}`"));
    }
    if path.contains("..") {
        return Err(format!("`{field}` must not contain `..` (path escape): `{path}`"));
    }
    // Reject Windows drive letters (e.g. `C:\`) and UNC paths.
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        return Err(format!("`{field}` must be repo-relative, not a drive path: `{path}`"));
    }
    Ok(())
}

/// The closed vocabulary of fact classes the producer can emit.
const VALID_FACT_CLASSES: &[&str] = &[
    "files",
    "owners",
    "changes",
    "tests",
    "oracles",
    "relations",
    "dynamic_boundaries",
    "verify_commands",
    "limitations",
    "provenance",
];

/// Parse + deduplicate + deterministically order the comma-separated
/// fact-class list.
fn normalize_fact_classes(raw: &str) -> Result<Vec<String>, String> {
    let mut seen: Vec<String> = Vec::new();
    for class in raw.split(',').map(str::trim).filter(|c| !c.is_empty()) {
        if !VALID_FACT_CLASSES.contains(&class) {
            return Err(format!(
                "unknown fact class `{class}`; valid: {}",
                VALID_FACT_CLASSES.join(", ")
            ));
        }
        if !seen.iter().any(|s| s == class) {
            seen.push(class.to_string());
        }
    }
    // Deterministic order: canonical VALID_FACT_CLASSES order.
    seen.sort_by_key(|c| {
        VALID_FACT_CLASSES.iter().position(|v| *v == c.as_str()).unwrap_or(usize::MAX)
    });
    if seen.is_empty() {
        return Err("fact_classes must not be empty".to_string());
    }
    Ok(seen)
}

/// Build a schema-valid `unavailable` packet (the honest state until the full
/// emitter body lands).
fn build_unavailable_packet(
    schema: &str,
    root: &str,
    base: Option<&str>,
    head: Option<&str>,
    fact_classes: &[String],
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": schema,
        // M1 contract convergence: deterministic packet ID (no timestamp).
        // The ID is derived from the schema + root + fact_classes so the same
        // input always produces the same packet ID.
        "packet_id": format!(
            "perl-lsp-ripr-facts-{schema}-{root}-{}",
            fact_classes.join(",")
        ),
        "packet_status": "unavailable",
        "packet_fingerprint": null,
        "producer": {
            "name": "perl-lsp",
            "version": env!("CARGO_PKG_VERSION"),
            "capabilities": fact_classes,
        },
        "root": {
            "repo_relative": root,
            "vcs_head": head,
            "path_style": "posix",
        },
        "input": {
            "base": base,
            "head": head,
            "diff_id": null,
            "requested_fact_classes": fact_classes,
        },
        "files": [],
        "owners": [],
        "changes": [],
        "tests": [],
        "oracles": [],
        "relations": [],
        "dynamic_boundaries": [],
        "verify_commands": [],
        "limitations": [{
            "limitation_id": "emitter-not-yet-implemented",
            "kind": "missing_emitter",
            "message": "The ripr-facts emitter body lands in PRs 5-8 (perl-lsp-swarm#2592-#2595). Today every call produces an unavailable packet.",
            "evidence_refs": []
        }],
        "provenance": [{
            "provenance_id": "cli-surface",
            "source": "operator_config",
            "confidence": "high"
        }]
    })
}

/// Write a JSON packet to the output path, creating parent directories.
fn write_packet(out: &str, packet: &serde_json::Value) -> std::io::Result<()> {
    let path = std::path::Path::new(out);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(packet)?;
    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::{
        build_unavailable_packet, normalize_fact_classes, run_ripr_facts, validate_ripr_facts_path,
        write_packet,
    };

    // ── ripr-facts command tests (Campaign 31 PR 4, perl-lsp-swarm#2591) ──

    #[test]
    fn ripr_facts_validates_schema_version() {
        let rc = run_ripr_facts(
            "wrong-schema",
            ".",
            None,
            None,
            "owners,changes",
            "target/ripr/test-wrong-schema.json",
        );
        assert_eq!(rc, 1, "wrong schema must exit 1");
    }

    #[test]
    fn ripr_facts_rejects_absolute_root() {
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            "/absolute/path",
            None,
            None,
            "owners",
            "target/ripr/test-abs-root.json",
        );
        assert_eq!(rc, 1, "absolute root must exit 1");
    }

    #[test]
    fn ripr_facts_rejects_path_escape() {
        let rc =
            run_ripr_facts("ripr-perl-facts-v1", ".", None, None, "owners", "../../../etc/passwd");
        assert_eq!(rc, 1, "path escape must exit 1");
    }

    #[test]
    fn ripr_facts_rejects_unknown_fact_class() {
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            ".",
            None,
            None,
            "owners,bogus_class",
            "target/ripr/test-bad-class.json",
        );
        assert_eq!(rc, 1, "unknown fact class must exit 1");
    }

    #[test]
    fn ripr_facts_rejects_drive_path() {
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            "C:/repo",
            None,
            None,
            "owners",
            "target/ripr/test-drive.json",
        );
        assert_eq!(rc, 1, "Windows drive path must exit 1");
    }

    #[test]
    fn ripr_facts_rejects_dot_slash_root() {
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            "./repo",
            None,
            None,
            "owners",
            "target/ripr/test-dot-slash.json",
        );
        assert_eq!(rc, 1, "./ prefix must exit 1");
    }

    #[test]
    fn ripr_facts_rejects_empty_fact_classes() {
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            ".",
            None,
            None,
            "",
            "target/ripr/test-empty-classes.json",
        );
        assert_eq!(rc, 1, "empty fact_classes must exit 1");
    }

    #[test]
    fn ripr_facts_accepts_valid_invocation() {
        let out = "target/ripr/test-valid-invocation.json";
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            ".",
            Some("origin/main"),
            Some("HEAD"),
            "files,owners,changes,tests,oracles",
            out,
        );
        assert_eq!(rc, 0, "valid invocation must exit 0");
        let written = std::fs::read_to_string(out).expect("packet must be written");
        let parsed: serde_json::Value =
            serde_json::from_str(&written).expect("packet must be JSON");
        assert_eq!(parsed["packet_status"], "unavailable");
        // Clean up.
        let _ = std::fs::remove_file(out);
    }

    /// Call-observation test for the success-*with-facts* path.
    ///
    /// The other `run_ripr_facts` tests hit early-return validation (rc==1) or
    /// the empty-root success path (root=".", which finds no `.t` files and
    /// stays `unavailable`). This drives the full fact-producing chain end to
    /// end — the emitter discovers a real `.t` file, detects the framework,
    /// upgrades the packet to `partial`, and writes it — so the emitter seams
    /// are observed via a real call. It pairs with the string-scan ripr
    /// suppression in `policy/ripr-suppressions.toml` (ripr#1429 class): RIPR's
    /// static tracer cannot follow the string scans, but this observably
    /// exercises them.
    #[test]
    fn ripr_facts_success_with_test_facts_writes_partial_packet() -> std::io::Result<()> {
        // `run_ripr_facts` resolves `root`/`out` relative to the process CWD (the
        // crate dir under `cargo test`), so keep them repo-relative to pass the
        // path validator. A unique subdir avoids collision with the other tests.
        let root = "target/ripr-facts-selftest";
        let t_dir = format!("{root}/t");
        std::fs::create_dir_all(&t_dir)?;
        std::fs::write(
            format!("{t_dir}/basic.t"),
            "use Test::More;\nok(1, 'truthy');\nis(1, 1, 'one equals one');\ndone_testing;\n",
        )?;
        let out = format!("{root}/packet.json");

        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            root,
            None,
            None,
            "tests,oracles,relations,limitations",
            &out,
        );
        assert_eq!(rc, 0, "valid invocation with a .t file must exit 0");

        let written = std::fs::read_to_string(&out)?;
        let parsed: serde_json::Value = serde_json::from_str(&written)?;
        // Discovering a `.t` file upgrades the packet from `unavailable` to `partial`.
        assert_eq!(
            parsed["packet_status"], "partial",
            "a discovered .t file must yield a partial packet"
        );
        let tests = parsed["tests"].as_array().expect("tests[] is an array");
        assert!(!tests.is_empty(), "the discovered .t file must produce a test fact");
        assert_eq!(
            tests[0]["framework"], "Test::More",
            "framework must be detected from `use Test::More`"
        );

        // Clean up the synthetic tree.
        let _ = std::fs::remove_dir_all(root);
        Ok(())
    }

    #[test]
    fn ripr_facts_deduplicates_and_orders_fact_classes() {
        let normalized = normalize_fact_classes("changes,owners,owners,changes,tests")
            .expect("valid classes normalize");
        // Canonical order (VALID_FACT_CLASSES order): files, owners, changes, tests, ...
        assert_eq!(normalized, vec!["owners", "changes", "tests"]);
    }

    #[test]
    fn ripr_facts_unavailable_packet_has_correct_shape() {
        let packet = build_unavailable_packet(
            "ripr-perl-facts-v1",
            ".",
            Some("origin/main"),
            Some("HEAD"),
            &["owners".to_string(), "changes".to_string()],
        );
        assert_eq!(packet["schema_version"], "ripr-perl-facts-v1");
        assert_eq!(packet["packet_status"], "unavailable");
        assert_eq!(packet["producer"]["name"], "perl-lsp");
        assert_eq!(packet["root"]["repo_relative"], ".");
        assert_eq!(packet["input"]["base"], "origin/main");
        assert_eq!(packet["input"]["head"], "HEAD");
        assert_eq!(
            packet["input"]["requested_fact_classes"],
            serde_json::json!(["owners", "changes"])
        );
        // The limitation explains why the packet is unavailable.
        assert_eq!(packet["limitations"][0]["kind"], "missing_emitter");
        // All fact arrays are empty (unavailable).
        for key in [
            "files",
            "owners",
            "changes",
            "tests",
            "oracles",
            "relations",
            "dynamic_boundaries",
            "verify_commands",
        ] {
            assert!(packet[key].as_array().unwrap().is_empty(), "array {key} should be empty");
        }
    }

    #[test]
    fn ripr_facts_writes_unavailable_packet_to_disk() -> std::io::Result<()> {
        let out = "target/ripr/test-ripr-facts-write.json";
        let packet = build_unavailable_packet(
            "ripr-perl-facts-v1",
            ".",
            None,
            None,
            &["owners".to_string()],
        );
        write_packet(out, &packet)?;
        let written = std::fs::read_to_string(out)?;
        let parsed: serde_json::Value = serde_json::from_str(&written)?;
        assert_eq!(parsed["schema_version"], "ripr-perl-facts-v1");
        assert_eq!(parsed["packet_status"], "unavailable");
        Ok(())
    }

    #[test]
    fn ripr_facts_rejects_empty_root() {
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            "",
            None,
            None,
            "owners",
            "target/ripr/test-empty-root.json",
        );
        assert_eq!(rc, 1, "empty root must exit 1");
    }

    #[test]
    fn ripr_facts_rejects_empty_out() {
        let rc = run_ripr_facts("ripr-perl-facts-v1", ".", None, None, "owners", "");
        assert_eq!(rc, 1, "empty out path must exit 1");
    }

    #[test]
    fn ripr_facts_validates_path_helper_directly() {
        // Directly test the path validator for all branches.
        assert!(validate_ripr_facts_path(".", "test").is_ok());
        assert!(validate_ripr_facts_path("target/ripr/x.json", "test").is_ok());
        assert!(validate_ripr_facts_path("", "test").is_err());
        assert!(validate_ripr_facts_path("/abs", "test").is_err());
        assert!(validate_ripr_facts_path("./rel", "test").is_err());
        assert!(validate_ripr_facts_path("../escape", "test").is_err());
        assert!(validate_ripr_facts_path("C:/drive", "test").is_err());
    }
}
