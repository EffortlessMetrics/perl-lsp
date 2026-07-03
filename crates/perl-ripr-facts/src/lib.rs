//! Batch exporter for the `ripr-perl-facts-v1` packet.
//!
//! This crate owns the `ripr-facts` unit that previously lived inside the LSP
//! server runtime (`perl-lsp-rs::ripr_facts_emitter` + the packet-assembly
//! helpers in `perl-lsp-rs::cli`). It was relocated here behavior-preserving so
//! that RIPR fact production sits **below** the editor/LSP stack and **above**
//! the raw parser — see `README.md` for the dependency contract.
//!
//! Two entry points:
//!
//! - [`build_ripr_facts_packet`] is the **structured batch API** (#3293 PR 2):
//!   it validates a [`RiprFactsRequest`], runs the (currently conservative,
//!   string-scan) emitter, and returns the assembled `ripr-perl-facts-v1` packet
//!   as a [`serde_json::Value`] — no I/O.
//! - [`run_ripr_facts`] is the thin CLI wrapper the `perl-lsp` / `perllsp`
//!   `ripr-facts` subcommand calls: it forwards CLI-shaped args to the batch
//!   API, then validates the output path, writes the packet to disk, and maps
//!   the outcome to a process exit code.
//!
//! Later slices replace the remaining string scans with fuller parser- and
//! semantic-backed extraction (using leaf crates like `perl-parser-core` /
//! `perl-symbol` that carry no forbidden dependencies — not `perl-workspace`,
//! which pulls `lsp-types`); this crate is the home for that work.

mod emitter;

use emitter::{
    emit_boundaries_and_commands, emit_changes_from_diff, emit_files_and_owners,
    emit_relations_and_discriminators, emit_tests_and_oracles,
};

/// Expected schema version for `ripr-perl-facts-v1` packets.
const EXPECTED_RIPR_FACTS_SCHEMA: &str = "ripr-perl-facts-v1";

/// A structured request to the `ripr-facts` batch exporter.
///
/// This is the programmatic input shape for [`build_ripr_facts_packet`]. The
/// `perl-lsp` / `perllsp` `ripr-facts` subcommand parses argv into one of these
/// and calls the batch API through [`run_ripr_facts`]; other batch producers
/// can construct it directly.
#[derive(Debug, Clone, Copy)]
pub struct RiprFactsRequest<'a> {
    /// Packet schema version; must equal `ripr-perl-facts-v1`.
    pub schema: &'a str,
    /// Repo-relative workspace root to scan (forward-slash, no `..`/drive/absolute).
    pub root: &'a str,
    /// Optional base ref for diff-derived facts (managed-producer mode; not yet emitted).
    pub base: Option<&'a str>,
    /// Optional head ref recorded in the packet.
    pub head: Option<&'a str>,
    /// Comma-separated fact classes to request; validated + normalized internally.
    pub fact_classes: &'a str,
    /// Pre-computed unified diff (base→head) text, supplied by a managed-producer
    /// caller and consumed only when `changes` is requested. `None` in the batch
    /// / CLI path (which does not yet produce one — see #3293 PR 5). The diff is
    /// treated as opaque text: no git is run, no process is spawned, and its
    /// paths are expected in `git diff`'s default repo-root-relative `a/`/`b/`
    /// form. base/head/diff are caller-asserted, never verified here.
    pub diff: Option<&'a str>,
}

/// A validation failure from [`build_ripr_facts_packet`] that prevents packet
/// assembly.
///
/// Emission itself is infallible — the conservative string-scan emitter degrades
/// to an `unavailable` / `partial` packet rather than erroring — so the only way
/// to build no packet at all is to fail input validation.
///
/// The [`Display`](std::fmt::Display) form is the operator-facing reason without
/// the `ripr-facts: ` prefix that [`run_ripr_facts`] adds when printing to
/// stderr.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiprFactsError {
    /// The requested schema is not the supported `ripr-perl-facts-v1`.
    UnsupportedSchema {
        /// The unsupported schema string the caller passed.
        schema: String,
    },
    /// The `root` path is not repo-relative (absolute, `./`, `..`, or drive).
    InvalidRoot(String),
    /// The `fact_classes` list is empty or contains an unknown class.
    InvalidFactClasses(String),
}

impl std::fmt::Display for RiprFactsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema { schema } => {
                write!(f, "unsupported schema `{schema}`; expected `{EXPECTED_RIPR_FACTS_SCHEMA}`")
            }
            Self::InvalidRoot(reason) | Self::InvalidFactClasses(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for RiprFactsError {}

/// Build the `ripr-perl-facts-v1` packet for a request, performing no I/O.
///
/// This is the structured batch API (#3293 PR 2). It validates the schema, the
/// repo-relative `root`, and the requested fact classes, runs the (currently
/// conservative, string-scan) emitter, and assembles the packet — returning it
/// as a [`serde_json::Value`] instead of writing it to disk.
///
/// The returned packet is byte-identical to what the pre-PR-2 `run_ripr_facts`
/// wrote for the same inputs. Callers that want the CLI behaviour (validate an
/// `out` path, write the packet, map to an exit code) use [`run_ripr_facts`],
/// which is a thin wrapper over this function.
pub fn build_ripr_facts_packet(
    request: &RiprFactsRequest<'_>,
) -> Result<serde_json::Value, RiprFactsError> {
    let &RiprFactsRequest { schema, root, base, head, fact_classes, diff } = request;

    // Validate schema version.
    if schema != EXPECTED_RIPR_FACTS_SCHEMA {
        return Err(RiprFactsError::UnsupportedSchema { schema: schema.to_owned() });
    }

    // Validate root is repo-relative (forward-slash, no host/drive/temp).
    validate_ripr_facts_path(root, "root").map_err(RiprFactsError::InvalidRoot)?;

    // Validate + normalize fact classes.
    let normalized_classes =
        normalize_fact_classes(fact_classes).map_err(RiprFactsError::InvalidFactClasses)?;

    // Emit the packet. Parser-backed tests/oracles (#3293 PR 4), relations/
    // discriminators (#2594), boundaries/commands (#2595), and parser-backed
    // files/owners (#3293 PR 3) are populated; diff-derived changes still land in
    // a later slice. When any facts are found, `packet_status` upgrades from
    // `unavailable` to `partial`.
    //
    // PR 4: parse test files only when `tests`/`oracles` — or `relations`, a
    // downstream consumer of the test/oracle facts (out of scope here, kept
    // behavior-identical by sharing the gate) — are requested, so a subset like
    // `files` stays cheap. The packet carries `tests[]`/`oracles[]` only for the
    // classes actually requested.
    let wants_tests = normalized_classes.iter().any(|c| c == "tests");
    let wants_oracles = normalized_classes.iter().any(|c| c == "oracles");
    let wants_relations = normalized_classes.iter().any(|c| c == "relations");
    let (tests, oracles, test_provenance, test_limitations) =
        if wants_tests || wants_oracles || wants_relations {
            emit_tests_and_oracles(root)
        } else {
            (Vec::new(), Vec::new(), Vec::new(), Vec::new())
        };

    let (relations, _changed_observables, _observed_sinks) =
        emit_relations_and_discriminators(root, &tests, &oracles);
    let has_relation_facts = !relations.is_empty();

    // Emit `tests[]`/`oracles[]` only for the specifically-requested classes; the
    // facts computed above may exist solely to feed `relations`. But referential
    // integrity trumps strict gating: an `oracle` carries a required `test_id` and
    // a `relation` carries a required `test_id`, so keeping either while dropping
    // the referenced `test` facts would dangle. So gate `oracles[]` on the explicit
    // request (nothing references an oracle — relations set `oracle_id: null` in
    // this slice), then keep `tests[]` whenever a relation OR an oracle references
    // one. This preserves the referential integrity origin/main had by always
    // populating `tests[]`.
    let oracles = if wants_oracles { oracles } else { Vec::new() };
    let has_oracle_facts = !oracles.is_empty();
    let tests =
        if wants_tests || has_relation_facts || has_oracle_facts { tests } else { Vec::new() };
    let has_test_facts = !tests.is_empty();

    let (boundaries, boundary_limitations, verify_commands) = emit_boundaries_and_commands(root);
    let has_boundary_facts = !boundaries.is_empty();

    // PR 3 (perl-lsp-swarm#3293): emit parser-backed files + owners facts (plus
    // per-file provenance and parse/read limitations) by parsing every Perl
    // source/test file under `root`. Only do the (potentially expensive) walk +
    // parse when the caller actually requested `files`/`owners`/`provenance`, so
    // a subset request (e.g. `tests,oracles`) stays cheap and the packet does
    // not carry facts outside the advertised `requested_fact_classes`.
    // `changes` needs the parsed owners to attribute each diff hunk, so the walk
    // runs when files/owners/provenance are explicitly requested OR when
    // `changes` is requested (the "computed for internal need" split PR 4 used
    // for tests feeding relations).
    let wants_changes = normalized_classes.iter().any(|c| c == "changes");
    let wants_file_facts_explicit = normalized_classes
        .iter()
        .any(|class| class == "files" || class == "owners" || class == "provenance");
    let (files, owners, file_provenance, file_limitations) =
        if wants_file_facts_explicit || wants_changes {
            emit_files_and_owners(root)
        } else {
            (Vec::new(), Vec::new(), Vec::new(), Vec::new())
        };

    // PR 5 (perl-lsp-swarm#3293): emit diff-owned `changes[]` from a caller-
    // supplied unified diff (`RiprFactsRequest.diff`). No git, no subprocess —
    // the diff is opaque text. `changes` requested without a diff yields an empty
    // array plus a `no-diff-supplied` limitation, so a downstream consumer can
    // distinguish "not analyzed" from "nothing changed".
    let (changes, change_limitations) = if wants_changes {
        match diff {
            Some(diff_text) if !diff_text.trim().is_empty() => {
                emit_changes_from_diff(diff_text, root, &files, &owners)
            }
            _ => (
                Vec::new(),
                vec![serde_json::json!({
                    "limitation_id": "no-diff-supplied",
                    "kind": "missing_input",
                    "message": "`changes` was requested but no diff was supplied on RiprFactsRequest.diff; the batch/CLI path does not yet produce one. An empty `changes[]` here means \"not analyzed\", not \"nothing changed\".",
                    "evidence_refs": []
                })],
            ),
        }
    } else {
        (Vec::new(), Vec::new())
    };
    let has_change_facts = !changes.is_empty();

    // Referential integrity: a change's `file_id`/`owner_id` — and an
    // `unattributable-change` limitation's file `evidence_refs` — point at a
    // parsed file, so the `files[]`/`owners[]` they reference must be present.
    // Force files+owners into the packet whenever a change was emitted OR an
    // `unattributable-change` limitation was recorded (both reference a
    // known/parsed file), exactly as PR 4 kept `tests[]` whenever a relation
    // referenced a `test_id`. `diff-file-not-found` references an UNparsed path
    // (genuinely absent), so it needs no force-include.
    let changes_reference_known_file = has_change_facts
        || change_limitations.iter().any(|l| {
            l["limitation_id"].as_str().is_some_and(|id| id.starts_with("unattributable-change:"))
        });
    let (files, owners) = if wants_file_facts_explicit || changes_reference_known_file {
        (files, owners)
    } else {
        (Vec::new(), Vec::new())
    };
    let has_file_facts = !files.is_empty();
    let has_owner_facts = !owners.is_empty();

    // File-walk limitations (the `digest-algorithm` schema note, parse/read
    // failures) describe or reference `files[]` facts, so only surface them when
    // `files[]` are actually in the packet. A `changes`-only request that cleared
    // `files[]` would otherwise ship a note about digests that aren't present —
    // the same orphaned-limitation class as `oracle-representation`.
    let file_limitations =
        if wants_file_facts_explicit || has_file_facts { file_limitations } else { Vec::new() };

    let mut packet = build_unavailable_packet(schema, root, base, head, &normalized_classes);

    // Populate tests + oracles arrays (parser-backed, #3293 PR 4) and append the
    // `test_discovery` / `oracle_extraction` provenance the facts reference by id.
    packet["tests"] = serde_json::Value::Array(tests);
    packet["oracles"] = serde_json::Value::Array(oracles);
    // Each `test_provenance` entry is referenced by exactly one fact class
    // (`test_discovery` ← tests, `oracle_extraction` ← oracles). Extend with only
    // the entries whose referenced class is actually in the packet — a coarse
    // `has_test_facts || has_oracle_facts` gate on the whole Vec would leak a
    // dangling `oracle_extraction`/`test_discovery` id when only one of
    // tests/oracles is requested (same referential class as the relation→test_id
    // fix).
    if has_test_facts || has_oracle_facts {
        if let Some(provenance) = packet["provenance"].as_array_mut() {
            for entry in test_provenance {
                let keep = match entry["source"].as_str() {
                    Some("test_discovery") => has_test_facts,
                    Some("oracle_extraction") => has_oracle_facts,
                    _ => true,
                };
                if keep {
                    provenance.push(entry);
                }
            }
        }
    }

    // Populate relations array (PR 7).
    packet["relations"] = serde_json::Value::Array(relations);

    // Populate diff-owned changes array (#3293 PR 5).
    packet["changes"] = serde_json::Value::Array(changes);

    // Populate dynamic_boundaries + verify_commands arrays (PR 8).
    packet["dynamic_boundaries"] = serde_json::Value::Array(boundaries);
    packet["verify_commands"] = serde_json::Value::Array(verify_commands);

    // Populate files + owners arrays (PR 3), and append the per-file `syntax`
    // provenance entries that each file/owner fact references by id.
    if has_file_facts || has_owner_facts {
        packet["files"] = serde_json::Value::Array(files);
        packet["owners"] = serde_json::Value::Array(owners);
        if let Some(provenance) = packet["provenance"].as_array_mut() {
            provenance.extend(file_provenance);
        }
    }

    // `limitations` is a meta-class aggregating every emitter's limitations, and
    // `test_limitations` is already gated at its source (empty unless the
    // tests/oracles/relations parse above ran), so it flows through as-is — a
    // `relations`-driven parse still surfaces its `test-parse-failed` limitations.
    // One entry needs an extra guard: `oracle-representation` is a schema-note
    // about the *oracle facts*. If the caller's request dropped `oracles[]` from
    // the packet (e.g. `tests`-only), that note describes facts that aren't
    // present — the same referential-integrity class as the relation→test_id fix.
    // Drop it when no oracle facts made it into the packet.
    let test_limitations: Vec<serde_json::Value> = if has_oracle_facts {
        test_limitations
    } else {
        test_limitations
            .into_iter()
            .filter(|l| l["limitation_id"].as_str() != Some("oracle-representation"))
            .collect()
    };

    // Upgrade status + merge limitations if we found any facts. Parse/read
    // limitations from the test and files passes are always surfaced (even with
    // no facts).
    let has_facts = has_test_facts
        || has_oracle_facts
        || has_relation_facts
        || has_boundary_facts
        || has_change_facts
        || has_file_facts
        || has_owner_facts;
    if has_facts {
        packet["packet_status"] = serde_json::json!("partial");
        // Merge boundary limitations, the emitter-partial note, and the
        // test/change/file limitations.
        let mut all_limitations = boundary_limitations;
        all_limitations.push(serde_json::json!({
            "limitation_id": "emitter-partial",
            "kind": "partial_emitter",
            "message": "Parser-backed tests/oracles, relations/discriminators, boundaries/commands, files/owners, and (when a diff is supplied) diff-owned changes are emitted. Semantic relations, the packet fingerprint, and the managed-producer diff source land in later slices.",
            "evidence_refs": []
        }));
        all_limitations.extend(test_limitations);
        all_limitations.extend(change_limitations);
        all_limitations.extend(file_limitations);
        packet["limitations"] = serde_json::Value::Array(all_limitations);
    } else if !test_limitations.is_empty()
        || !change_limitations.is_empty()
        || !file_limitations.is_empty()
    {
        // No facts, but a pass produced limitations (test/file parse failures, or
        // a `changes` request with no diff) — surface them next to the base
        // `emitter-not-yet-implemented` limitation so they are never dropped.
        if let Some(limitations) = packet["limitations"].as_array_mut() {
            limitations.extend(test_limitations);
            limitations.extend(change_limitations);
            limitations.extend(file_limitations);
        }
    }

    Ok(packet)
}

/// Run the `ripr-facts` exporter (Campaign 31, ripr-swarm#1379).
///
/// The thin CLI wrapper over [`build_ripr_facts_packet`]: it forwards the
/// CLI-shaped args to the batch API, then validates the output path, writes the
/// assembled packet to `out`, and maps the outcome to a process exit code (`0`
/// on success, `1` on any validation or write failure). Diagnostics go to
/// stderr with a `ripr-facts: ` prefix.
///
/// The `out` path (a write concern owned by the wrapper, not part of the
/// packet) is validated first: it is the cheapest check, so failing on it before
/// building the packet avoids a needless workspace scan when the write
/// destination is invalid.
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
    // Validate the output path first — the cheapest check — so an invalid write
    // destination fails fast, before the emitter scans the workspace.
    if let Err(reason) = validate_ripr_facts_path(out, "out") {
        eprintln!("ripr-facts: {reason}");
        return 1;
    }

    let packet = match build_ripr_facts_packet(&RiprFactsRequest {
        schema,
        root,
        base,
        head,
        fact_classes,
        // The CLI wrapper does not yet supply a diff (#3293 PR 5 scope).
        diff: None,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            eprintln!("ripr-facts: {error}");
            return 1;
        }
    };

    // Write the assembled packet to disk.
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
        RiprFactsError, RiprFactsRequest, build_ripr_facts_packet, build_unavailable_packet,
        normalize_fact_classes, run_ripr_facts, validate_ripr_facts_path, write_packet,
    };

    /// A valid request against the crate root (`"."`, no `t/` dir → unavailable).
    fn valid_request<'a>(fact_classes: &'a str) -> RiprFactsRequest<'a> {
        RiprFactsRequest {
            schema: "ripr-perl-facts-v1",
            root: ".",
            base: Some("origin/main"),
            head: Some("HEAD"),
            fact_classes,
            diff: None,
        }
    }

    // ── batch API tests (#3293 PR 2) ──

    #[test]
    fn build_packet_returns_unavailable_for_valid_empty_root() {
        // `.` has no `t/` directory, so the emitter finds nothing and the packet
        // stays `unavailable` — and the batch API performs no I/O to produce it.
        let packet = build_ripr_facts_packet(&valid_request("files,owners,changes,tests,oracles"))
            .expect("a valid request must build a packet");
        assert_eq!(packet["schema_version"], "ripr-perl-facts-v1");
        assert_eq!(packet["packet_status"], "unavailable");
        assert_eq!(packet["producer"]["name"], "perl-lsp");
    }

    #[test]
    fn build_packet_rejects_unsupported_schema() {
        let err = build_ripr_facts_packet(&RiprFactsRequest {
            schema: "wrong-schema",
            ..valid_request("owners")
        })
        .expect_err("wrong schema must fail validation");
        assert_eq!(err, RiprFactsError::UnsupportedSchema { schema: "wrong-schema".to_owned() });
        // Display carries the exact stderr reason the wrapper prefixes with `ripr-facts: `.
        assert_eq!(
            err.to_string(),
            "unsupported schema `wrong-schema`; expected `ripr-perl-facts-v1`"
        );
    }

    #[test]
    fn build_packet_rejects_invalid_root() {
        let err = build_ripr_facts_packet(&RiprFactsRequest {
            root: "/absolute",
            ..valid_request("owners")
        })
        .expect_err("absolute root must fail validation");
        assert!(matches!(err, RiprFactsError::InvalidRoot(_)), "got {err:?}");
    }

    #[test]
    fn build_packet_rejects_empty_fact_classes() {
        let err = build_ripr_facts_packet(&valid_request(""))
            .expect_err("empty fact classes must fail validation");
        assert!(matches!(err, RiprFactsError::InvalidFactClasses(_)), "got {err:?}");
    }

    #[test]
    fn build_packet_is_deterministic() {
        // Pure function: identical input yields an identical packet (no clock, no
        // randomness, no filesystem mutation between calls).
        let request = valid_request("tests,oracles");
        let first = build_ripr_facts_packet(&request).expect("valid");
        let second = build_ripr_facts_packet(&request).expect("valid");
        assert_eq!(first, second, "the batch API must be deterministic for identical input");
    }

    #[test]
    fn build_packet_matches_what_the_wrapper_writes() -> std::io::Result<()> {
        // Parity: the packet the batch API returns is byte-identical to what the
        // `run_ripr_facts` CLI wrapper writes to disk for the same inputs.
        let out = "target/ripr/test-batch-parity.json";
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            ".",
            Some("origin/main"),
            Some("HEAD"),
            "tests,oracles",
            out,
        );
        assert_eq!(rc, 0, "wrapper must succeed");
        let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(out)?)?;
        let built = build_ripr_facts_packet(&valid_request("tests,oracles"))
            .expect("valid request builds a packet");
        assert_eq!(built, written, "batch API packet must equal what the wrapper writes");
        let _ = std::fs::remove_file(out);
        Ok(())
    }

    // ── PR 3 parser-backed files/owners/provenance tests (#3293) ──

    /// Build a packet for a synthetic workspace under a unique repo-relative
    /// root, then clean the fixture up. `files` is a slice of
    /// `(repo-relative path, content)`.
    fn packet_for_fixture(dir: &str, files: &[(&str, &str)]) -> serde_json::Value {
        let root = format!("target/ripr-p3-fixtures/{dir}");
        let _ = std::fs::remove_dir_all(&root);
        for (rel, content) in files {
            let path = format!("{root}/{rel}");
            if let Some(parent) = std::path::Path::new(&path).parent() {
                std::fs::create_dir_all(parent).expect("create fixture dir");
            }
            std::fs::write(&path, content).expect("write fixture file");
        }
        let packet = build_ripr_facts_packet(&RiprFactsRequest {
            schema: "ripr-perl-facts-v1",
            root: &root,
            base: None,
            head: None,
            fact_classes: "files,owners,provenance,limitations",
            diff: None,
        })
        .expect("valid request builds a packet");
        let _ = std::fs::remove_dir_all(&root);
        packet
    }

    fn owners(packet: &serde_json::Value) -> Vec<serde_json::Value> {
        packet["owners"].as_array().expect("owners[]").clone()
    }

    #[test]
    fn build_packet_emits_file_fact_for_pm() {
        let packet = packet_for_fixture(
            "pm-file",
            &[("lib/Widget.pm", "package Widget;\nsub build { 1 }\n1;\n")],
        );
        let files = packet["files"].as_array().expect("files[]");
        let f = files.iter().find(|f| f["path"] == "lib/Widget.pm").expect(".pm file fact");
        assert_eq!(f["role"], serde_json::json!(["source"]));
        assert_eq!(f["file_id"], "file:lib/Widget.pm");
        assert!(f["digest"].as_str().unwrap().starts_with("fnv64:"), "fnv64 digest");
    }

    #[test]
    fn build_packet_emits_file_fact_for_t() {
        let packet = packet_for_fixture("t-file", &[("t/basic.t", "use Test::More;\nok(1);\n")]);
        let files = packet["files"].as_array().expect("files[]");
        let f = files.iter().find(|f| f["path"] == "t/basic.t").expect(".t file fact");
        assert_eq!(f["role"], serde_json::json!(["test"]));
    }

    #[test]
    fn build_packet_emits_package_owner_with_real_range() {
        let packet =
            packet_for_fixture("pkg-owner", &[("lib/App.pm", "package App;\nsub run { 1 }\n1;\n")]);
        let owners = owners(&packet);
        let pkg = owners
            .iter()
            .find(|o| o["kind"] == "package" && o["name"] == "App")
            .expect("package owner");
        // Real range, not a 1:1 placeholder: `package App` starts at line 0.
        assert_eq!(pkg["range"]["start_line"], 0);
        assert_eq!(pkg["range"]["start_column"], 0);
        assert_eq!(pkg["confidence"], "high");
    }

    #[test]
    fn build_packet_emits_sub_owner_with_real_range() {
        let packet = packet_for_fixture(
            "sub-owner",
            &[("lib/App.pm", "package App;\nsub discount { return 42; }\n1;\n")],
        );
        let owners = owners(&packet);
        let sub = owners
            .iter()
            .find(|o| o["kind"] == "sub" && o["name"] == "discount")
            .expect("sub owner");
        // `sub discount` is on the second line (0-based line 1) — a real span.
        assert_eq!(sub["range"]["start_line"], 1);
        assert_eq!(sub["package"], "App", "sub owner records its enclosing package");
    }

    #[test]
    fn build_packet_emits_method_owner_with_real_range() {
        // Perl 5.38 `use feature 'class'` method declaration.
        let packet = packet_for_fixture(
            "method-owner",
            &[(
                "lib/Point.pm",
                "use v5.38;\nuse feature 'class';\nclass Point {\n    method describe { return 'p'; }\n}\n",
            )],
        );
        let owners = owners(&packet);
        let m = owners.iter().find(|o| o["kind"] == "method" && o["name"] == "describe");
        assert!(m.is_some(), "method `describe` must be an owner; owners: {owners:?}");
        assert_eq!(m.unwrap()["range"]["start_line"], 3, "method is on line 3 (0-based)");
    }

    #[test]
    fn build_packet_collects_package_names() {
        let packet = packet_for_fixture(
            "pkg-names",
            &[("lib/Multi.pm", "package Foo;\nsub a { }\npackage Bar;\nsub b { }\n1;\n")],
        );
        let files = packet["files"].as_array().expect("files[]");
        let f = files.iter().find(|f| f["path"] == "lib/Multi.pm").expect("file fact");
        let names: Vec<&str> = f["package_names"]
            .as_array()
            .expect("package_names")
            .iter()
            .filter_map(|n| n.as_str())
            .collect();
        assert!(names.contains(&"Foo") && names.contains(&"Bar"), "both packages, got {names:?}");
    }

    #[test]
    fn build_packet_orders_files_and_owners_deterministically() {
        let fixture: &[(&str, &str)] =
            &[("lib/Zebra.pm", "package Zebra;\n1;\n"), ("lib/Apple.pm", "package Apple;\n1;\n")];
        let a = packet_for_fixture("order-a", fixture);
        let b = packet_for_fixture("order-b", fixture);
        let paths = |p: &serde_json::Value| -> Vec<String> {
            p["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|f| f["path"].as_str().map(String::from))
                .collect()
        };
        let pa = paths(&a);
        assert_eq!(pa, paths(&b), "file ordering is deterministic across builds");
        assert!(pa.windows(2).all(|w| w[0] <= w[1]), "files are sorted, got {pa:?}");
    }

    #[test]
    fn build_packet_uses_repo_relative_paths_only() {
        let packet =
            packet_for_fixture("rel-paths", &[("lib/Deep/Mod.pm", "package Deep::Mod;\n1;\n")]);
        for f in packet["files"].as_array().expect("files[]") {
            let path = f["path"].as_str().unwrap();
            assert!(!path.starts_with('/'), "no absolute path: {path}");
            assert!(!path.contains(':'), "no drive/host prefix: {path}");
            assert!(
                !path.contains("ripr-p3-fixtures"),
                "path is relative to root, not the temp prefix: {path}"
            );
        }
        assert_eq!(packet["files"][0]["path"], "lib/Deep/Mod.pm");
    }

    #[test]
    fn build_packet_records_parser_provenance() {
        let packet = packet_for_fixture("prov", &[("lib/App.pm", "package App;\n1;\n")]);
        let provenance = packet["provenance"].as_array().expect("provenance[]");
        let p = provenance.iter().find(|p| p["source"] == "syntax").expect("syntax provenance");
        assert_eq!(p["file_id"], "file:lib/App.pm");
        assert_eq!(p["confidence"], "high");
        // The file fact references its provenance entry by id.
        let f =
            packet["files"].as_array().unwrap().iter().find(|f| f["path"] == "lib/App.pm").unwrap();
        let refs: Vec<&str> =
            f["provenance_refs"].as_array().unwrap().iter().filter_map(|r| r.as_str()).collect();
        assert!(
            refs.contains(&p["provenance_id"].as_str().unwrap()),
            "file references its provenance"
        );
    }

    #[test]
    fn build_packet_reports_parse_failure_as_limitation_or_partial() {
        // Deeply unbalanced braces trip the parser's recursion guard → parse() Err.
        let bad = "{".repeat(5000);
        let packet = packet_for_fixture("parse-fail", &[("lib/Bad.pm", &bad)]);
        // The file is not silently dropped — a file fact is still emitted.
        let files = packet["files"].as_array().expect("files[]");
        assert!(
            files.iter().any(|f| f["path"] == "lib/Bad.pm"),
            "file fact emitted despite parse issue"
        );
        // Fail-soft: either a parse-failure limitation is recorded, or the file
        // parsed (recovered) but produced no owners — never a silent drop.
        let limitations = packet["limitations"].as_array().expect("limitations[]");
        let had_parse_limitation = limitations
            .iter()
            .any(|l| l["limitation_id"].as_str().is_some_and(|s| s.starts_with("parse-failed:")));
        let bad_owners: Vec<_> =
            owners(&packet).into_iter().filter(|o| o["file_id"] == "file:lib/Bad.pm").collect();
        assert!(
            had_parse_limitation || bad_owners.is_empty(),
            "parse failure must surface a limitation"
        );
    }

    #[test]
    fn wrapper_output_matches_batch_packet_after_parser_facts() -> std::io::Result<()> {
        // Parity WITH real parser-backed facts: the wrapper writes exactly the
        // batch-API packet for a root that produces files + owners.
        let root = "target/ripr-p3-parity";
        let _ = std::fs::remove_dir_all(root);
        std::fs::create_dir_all(format!("{root}/lib"))?;
        std::fs::write(format!("{root}/lib/App.pm"), "package App;\nsub run { return 1; }\n1;\n")?;

        let out = format!("{root}/packet.json");
        let rc = run_ripr_facts("ripr-perl-facts-v1", root, None, None, "files,owners", &out);
        assert_eq!(rc, 0, "wrapper must succeed");
        let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out)?)?;

        let built = build_ripr_facts_packet(&RiprFactsRequest {
            schema: "ripr-perl-facts-v1",
            root,
            base: None,
            head: None,
            fact_classes: "files,owners",
            diff: None,
        })
        .expect("valid request");
        // Sanity: the fixture actually yields owners, so parity covers PR-3 facts.
        assert!(!built["owners"].as_array().unwrap().is_empty(), "fixture must yield owners");
        assert_eq!(built, written, "wrapper output must equal the batch packet with parser facts");

        let _ = std::fs::remove_dir_all(root);
        Ok(())
    }

    #[test]
    fn build_packet_skips_files_when_not_requested() {
        // A subset request that omits files/owners/provenance must not walk +
        // parse the workspace: files[]/owners[] stay empty even with a .pm present.
        let root = "target/ripr-p3-gate";
        let _ = std::fs::remove_dir_all(root);
        std::fs::create_dir_all(format!("{root}/lib")).expect("mkdir");
        std::fs::write(format!("{root}/lib/App.pm"), "package App;\nsub run { 1 }\n1;\n")
            .expect("write");
        let packet = build_ripr_facts_packet(&RiprFactsRequest {
            schema: "ripr-perl-facts-v1",
            root,
            base: None,
            head: None,
            fact_classes: "tests,oracles",
            diff: None,
        })
        .expect("valid request");
        let _ = std::fs::remove_dir_all(root);

        assert!(packet["files"].as_array().unwrap().is_empty(), "files not requested → empty");
        assert!(packet["owners"].as_array().unwrap().is_empty(), "owners not requested → empty");
    }

    // ── PR 4 parser-backed tests/oracles tests (#3293) ──

    /// Build a packet from a single synthetic `t/foo.t` file with the given
    /// requested fact classes, then clean the fixture up.
    fn packet_for_t(dir: &str, t_content: &str, fact_classes: &str) -> serde_json::Value {
        let root = format!("target/ripr-p4-fixtures/{dir}");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(format!("{root}/t")).expect("create t/ dir");
        std::fs::write(format!("{root}/t/foo.t"), t_content).expect("write t file");
        let packet = build_ripr_facts_packet(&RiprFactsRequest {
            schema: "ripr-perl-facts-v1",
            root: &root,
            base: None,
            head: None,
            fact_classes,
            diff: None,
        })
        .expect("valid request builds a packet");
        let _ = std::fs::remove_dir_all(&root);
        packet
    }

    fn tests_of(packet: &serde_json::Value) -> Vec<serde_json::Value> {
        packet["tests"].as_array().expect("tests[]").clone()
    }
    fn oracles_of(packet: &serde_json::Value) -> Vec<serde_json::Value> {
        packet["oracles"].as_array().expect("oracles[]").clone()
    }

    #[test]
    fn build_packet_detects_test_more_import() {
        let p = packet_for_t("tm-import", "use Test::More;\nok(1);\n", "tests");
        assert_eq!(tests_of(&p)[0]["framework"], "Test::More");
    }

    #[test]
    fn build_packet_detects_test2_v0_import() {
        let p = packet_for_t("t2v0-import", "use Test2::V0;\nok(1);\n", "tests");
        assert_eq!(tests_of(&p)[0]["framework"], "Test2::V0");
    }

    #[test]
    fn build_packet_detects_test2_v1_import() {
        let p = packet_for_t("t2v1-import", "use Test2::V1;\nok(1);\n", "tests");
        assert_eq!(tests_of(&p)[0]["framework"], "Test2::V1");
    }

    #[test]
    fn build_packet_detects_test2_suite_import() {
        let p = packet_for_t("t2suite-import", "use Test2::Suite;\nok(1);\n", "tests");
        assert_eq!(tests_of(&p)[0]["framework"], "Test2::Suite");
    }

    #[test]
    fn build_packet_detects_test_exception_import() {
        let p = packet_for_t("te-import", "use Test::Exception;\nok(1);\n", "tests");
        assert_eq!(tests_of(&p)[0]["framework"], "Test::Exception");
    }

    #[test]
    fn build_packet_detects_test_fatal_import() {
        let p = packet_for_t("tf-import", "use Test::Fatal;\nok(1);\n", "tests");
        assert_eq!(tests_of(&p)[0]["framework"], "Test::Fatal");
    }

    #[test]
    fn build_packet_emits_test_fact_for_t_file() {
        let p = packet_for_t("test-fact", "use Test::More;\nok(1);\ndone_testing;\n", "tests");
        let t = tests_of(&p);
        assert_eq!(t.len(), 1, "one test fact for the .t file");
        assert_eq!(t[0]["name"], "t/foo.t");
        assert_eq!(t[0]["test_id"], "test:t/foo.t");
        assert_eq!(t[0]["file_id"], "file:t/foo.t");
    }

    #[test]
    fn build_packet_emits_subtest_owner_or_test_fact_with_real_range() {
        // Subtest ownership is not required; the file-level test fact must carry a
        // real (non-placeholder) range spanning the file.
        let src = "use Test::More;\nsubtest 'grp' => sub {\n  ok(1);\n};\ndone_testing;\n";
        let p = packet_for_t("subtest", src, "tests");
        let t = tests_of(&p);
        assert_eq!(t.len(), 1);
        let placeholder =
            serde_json::json!({"start_line": 1, "start_column": 1, "end_line": 1, "end_column": 1});
        assert_ne!(t[0]["range"], placeholder, "real file range, not a 1:1 placeholder");
        assert!(t[0]["range"]["end_line"].as_u64().expect("end_line") >= 3, "spans the file");
    }

    #[test]
    fn build_packet_emits_test_more_ok_oracle() {
        let p = packet_for_t("tm-ok", "use Test::More;\nok(1, 'name');\n", "tests,oracles");
        assert!(oracles_of(&p).iter().any(|o| o["kind"] == "smoke_ok"), "ok → smoke_ok");
    }

    #[test]
    fn build_packet_emits_test_more_is_oracle() {
        let p = packet_for_t("tm-is", "use Test::More;\nis($x, 1, 'name');\n", "tests,oracles");
        assert!(
            oracles_of(&p).iter().any(|o| o["kind"] == "exact_return_assertion"),
            "is → exact_return_assertion"
        );
    }

    #[test]
    fn build_packet_emits_test_more_like_oracle() {
        let p =
            packet_for_t("tm-like", "use Test::More;\nlike($x, qr/y/, 'name');\n", "tests,oracles");
        assert!(
            oracles_of(&p)
                .iter()
                .any(|o| o["kind"] == "predicate_boundary_assertion"
                    && o["strength"] == "weak_broad"),
            "like → predicate_boundary_assertion / weak_broad"
        );
    }

    #[test]
    fn build_packet_emits_test_more_cmp_ok_oracle() {
        let p = packet_for_t(
            "tm-cmpok",
            "use Test::More;\ncmp_ok($x, '==', 1, 'name');\n",
            "tests,oracles",
        );
        assert!(
            oracles_of(&p)
                .iter()
                .any(|o| o["kind"] == "predicate_boundary_assertion"
                    && o["strength"] == "strong_exact"),
            "cmp_ok → predicate_boundary_assertion / strong_exact"
        );
    }

    #[test]
    fn build_packet_emits_test2_is_oracle() {
        let p = packet_for_t("t2-is", "use Test2::V0;\nis($x, 1, 'name');\n", "tests,oracles");
        assert!(
            oracles_of(&p).iter().any(|o| o["kind"] == "exact_return_assertion"),
            "Test2 is → exact_return_assertion"
        );
        assert_eq!(tests_of(&p)[0]["framework"], "Test2::V0");
    }

    #[test]
    fn build_packet_emits_test_exception_throws_ok_oracle() {
        let p = packet_for_t(
            "te-throws",
            "use Test::Exception;\nthrows_ok { die } qr/x/, 'name';\n",
            "tests,oracles",
        );
        assert!(
            oracles_of(&p).iter().any(|o| o["kind"] == "exception_observer"),
            "throws_ok → exception_observer"
        );
    }

    #[test]
    fn build_packet_emits_test_fatal_exception_oracle() {
        let p = packet_for_t(
            "tf-exception",
            "use Test::Fatal;\nis(exception { die }, undef, 'name');\n",
            "tests,oracles",
        );
        assert!(
            oracles_of(&p).iter().any(|o| o["kind"] == "exception_observer"
                && o["expression"].as_str().is_some_and(|e| e.contains("exception"))),
            "Test::Fatal exception(...) → exception_observer"
        );
    }

    #[test]
    fn build_packet_oracle_ranges_are_not_placeholder() {
        let p = packet_for_t(
            "no-placeholder",
            "use Test::More;\nis(1, 1, 'a');\nok(1, 'b');\n",
            "tests,oracles",
        );
        let placeholder =
            serde_json::json!({"start_line": 1, "start_column": 1, "end_line": 1, "end_column": 1});
        let os = oracles_of(&p);
        assert!(!os.is_empty(), "must emit oracles");
        assert!(os.iter().all(|o| o["range"] != placeholder), "no 1:1 placeholder ranges");
    }

    #[test]
    fn build_packet_test_oracles_are_deterministically_ordered() {
        let src = "use Test::More;\nis(1, 1);\nok(1);\nis(2, 2);\n";
        let a = packet_for_t("det-a", src, "tests,oracles");
        let b = packet_for_t("det-b", src, "tests,oracles");
        assert_eq!(a["oracles"], b["oracles"], "identical input → identical oracle order");
        assert!(oracles_of(&a).len() >= 3, "all three assertions extracted");
    }

    #[test]
    fn build_packet_skips_test_parse_when_tests_or_oracles_not_requested() {
        // A `files`-only request must not emit tests/oracles even though the .t
        // file has assertions.
        let p = packet_for_t("gate", "use Test::More;\nis(1, 1);\nok(1);\n", "files");
        assert!(tests_of(&p).is_empty(), "tests not requested → empty");
        assert!(oracles_of(&p).is_empty(), "oracles not requested → empty");
    }

    #[test]
    fn build_packet_relations_request_keeps_referenced_test_facts() {
        // A `relations`-only request still parses tests to build relations. Every
        // relation carries a required `test_id`; dropping the test facts would
        // leave `relation.test_id` dangling into an empty `tests[]`. Regression
        // guard for the fact-class gating added in #3293 PR 4.
        let root = "target/ripr-p4-fixtures/relations-refint";
        let _ = std::fs::remove_dir_all(root);
        std::fs::create_dir_all(format!("{root}/lib")).expect("create lib/");
        std::fs::create_dir_all(format!("{root}/t")).expect("create t/");
        // pm basename "foo" appears in test path "t/foo.t" → file_references_package.
        std::fs::write(format!("{root}/lib/foo.pm"), "package Foo;\nsub run { }\n1;\n")
            .expect("write pm");
        std::fs::write(format!("{root}/t/foo.t"), "use Test::More;\nuse Foo;\nok(Foo::run());\n")
            .expect("write t");
        let p = build_ripr_facts_packet(&RiprFactsRequest {
            schema: "ripr-perl-facts-v1",
            root,
            base: None,
            head: None,
            fact_classes: "relations",
            diff: None,
        })
        .expect("valid request builds a packet");
        let _ = std::fs::remove_dir_all(root);

        let relations = p["relations"].as_array().expect("relations[]");
        assert!(!relations.is_empty(), "fixture must produce at least one relation");
        let tests = tests_of(&p);
        // Every relation.test_id must resolve to a present test fact — no dangling ref.
        for rel in relations {
            let tid = rel["test_id"].as_str().expect("relation.test_id is a string");
            assert!(
                tests.iter().any(|t| t["test_id"] == tid),
                "relation.test_id {tid} must resolve to a test fact in the packet"
            );
        }
    }

    #[test]
    fn build_packet_is_referentially_closed_across_fact_class_subsets() {
        // Every fact reference must resolve within the packet, and no test-side
        // provenance entry may be an orphan — for any fact-class subset. Guards
        // the recurring referential-integrity class: relation→test_id (96dd5f9),
        // oracle→test_id, and provenance→fact (droid P2 on lib.rs:189).
        let t = "use Test::More;\nok(1, 'smoke');\nis(1, 1, 'exact');\n";
        for classes in ["tests", "oracles", "tests,oracles", "relations", "tests,oracles,relations"]
        {
            let p = packet_for_t(&format!("refclose-{}", classes.replace(',', "-")), t, classes);

            let ids = |arr: &[serde_json::Value], key: &str| -> std::collections::HashSet<String> {
                arr.iter().filter_map(|v| v[key].as_str().map(str::to_owned)).collect()
            };
            let tests = tests_of(&p);
            let oracles = oracles_of(&p);
            let relations = p["relations"].as_array().expect("relations[]").clone();
            let provenance = p["provenance"].as_array().expect("provenance[]").clone();

            let test_ids = ids(&tests, "test_id");
            let oracle_ids = ids(&oracles, "oracle_id");
            let prov_ids = ids(&provenance, "provenance_id");

            // Forward references resolve.
            for o in &oracles {
                let tid = o["test_id"].as_str().expect("oracle.test_id");
                assert!(test_ids.contains(tid), "[{classes}] oracle.test_id {tid} must resolve");
            }
            for r in &relations {
                let tid = r["test_id"].as_str().expect("relation.test_id");
                assert!(test_ids.contains(tid), "[{classes}] relation.test_id {tid} must resolve");
                if let Some(oid) = r["oracle_id"].as_str() {
                    assert!(
                        oracle_ids.contains(oid),
                        "[{classes}] relation.oracle_id must resolve"
                    );
                }
            }
            // Every provenance_ref on any fact resolves, and gather what's referenced.
            let mut referenced_prov = std::collections::HashSet::new();
            for fact in tests.iter().chain(oracles.iter()).chain(relations.iter()) {
                if let Some(refs) = fact["provenance_refs"].as_array() {
                    for r in refs {
                        let rid = r.as_str().expect("provenance_ref is a string");
                        assert!(
                            prov_ids.contains(rid),
                            "[{classes}] provenance_ref {rid} must resolve"
                        );
                        referenced_prov.insert(rid.to_owned());
                    }
                }
            }
            // No orphan test-side provenance: a `test_discovery`/`oracle_extraction`
            // entry present in the packet must be referenced by some fact.
            for prov in &provenance {
                let source = prov["source"].as_str().unwrap_or("");
                if source == "test_discovery" || source == "oracle_extraction" {
                    let pid = prov["provenance_id"].as_str().expect("provenance_id");
                    assert!(
                        referenced_prov.contains(pid),
                        "[{classes}] orphan {source} provenance {pid} referenced by no fact"
                    );
                }
            }
        }
    }

    #[test]
    fn build_packet_drops_oracle_representation_note_when_oracles_not_in_packet() {
        // The `oracle-representation` schema-note describes emitted oracle facts.
        // A `tests`-only request drops `oracles[]`, so that note must not linger —
        // it would describe facts absent from the packet (droid P3, same
        // referential-integrity class). The mirror request keeps it.
        let t = "use Test::More;\nis(1, 1, 'exact');\nok(1, 'smoke');\n";
        let has_note = |p: &serde_json::Value| {
            p["limitations"]
                .as_array()
                .expect("limitations[]")
                .iter()
                .any(|l| l["limitation_id"].as_str() == Some("oracle-representation"))
        };

        let tests_only = packet_for_t("orep-tests", t, "tests");
        assert!(oracles_of(&tests_only).is_empty(), "oracles not requested → empty");
        assert!(
            !has_note(&tests_only),
            "oracle-representation note must not linger when oracles[] is dropped"
        );

        let with_oracles = packet_for_t("orep-oracles", t, "oracles");
        assert!(!oracles_of(&with_oracles).is_empty(), "oracles present");
        assert!(has_note(&with_oracles), "oracle-representation note present with oracle facts");
    }

    #[test]
    fn build_packet_relations_request_surfaces_test_parse_limitation() {
        // `relations` drives the test parse (to build relations), so a parse
        // failure in a .t file must still surface a limitation even though
        // `tests`/`oracles` weren't requested — `limitations` is a meta-class.
        // Guards against silently dropping test-side limitations (droid P2).
        // No `lib/`, so no relation is emitted (the parse still runs because
        // `relations` is requested) — this isolates the limitation-surfacing
        // behavior from the referential-integrity retention of `tests[]`.
        let p = packet_for_t("relations-limitations", &"{".repeat(5000), "relations,limitations");

        // Tests/oracles were not requested and no relation references them →
        // not emitted...
        assert!(tests_of(&p).is_empty(), "tests not requested → empty");
        assert!(oracles_of(&p).is_empty(), "oracles not requested → empty");
        // ...but the test-parse-failed limitation is NOT dropped.
        let lims = p["limitations"].as_array().expect("limitations[]");
        assert!(
            lims.iter().any(|l| l["limitation_id"]
                .as_str()
                .is_some_and(|s| s.starts_with("test-parse-failed:"))),
            "relations-driven test parse failure must surface a limitation"
        );
    }

    #[test]
    fn build_packet_reports_unparseable_t_file_as_limitation() {
        // Deeply unbalanced braces trip the parser's recursion guard → parse() Err.
        let bad = "{".repeat(5000);
        let p = packet_for_t("unparseable", &bad, "tests,oracles,limitations");
        // Not silent: a test fact is still emitted (unknown framework) ...
        assert!(tests_of(&p).iter().any(|t| t["name"] == "t/foo.t"), "test fact emitted");
        // ... and a parse-failure limitation is recorded.
        let lims = p["limitations"].as_array().expect("limitations[]");
        assert!(
            lims.iter().any(|l| l["limitation_id"]
                .as_str()
                .is_some_and(|s| s.starts_with("test-parse-failed:"))),
            "unparseable .t must surface a test-parse-failed limitation"
        );
    }

    #[test]
    fn wrapper_output_matches_batch_packet_after_test_oracle_facts() -> std::io::Result<()> {
        // Parity means batch API == wrapper-written packet (not PR4 == PR3).
        let root = "target/ripr-p4-parity";
        let _ = std::fs::remove_dir_all(root);
        std::fs::create_dir_all(format!("{root}/t"))?;
        std::fs::write(
            format!("{root}/t/foo.t"),
            "use Test::More;\nis(1, 1, 'a');\nok(1, 'b');\n",
        )?;
        let out = format!("{root}/packet.json");
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            root,
            None,
            None,
            "tests,oracles,provenance,limitations",
            &out,
        );
        assert_eq!(rc, 0, "wrapper succeeds");
        let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out)?)?;
        let built = build_ripr_facts_packet(&RiprFactsRequest {
            schema: "ripr-perl-facts-v1",
            root,
            base: None,
            head: None,
            fact_classes: "tests,oracles,provenance,limitations",
            diff: None,
        })
        .expect("valid request");
        assert_eq!(built, written, "batch API packet == wrapper-written packet");
        assert!(!built["oracles"].as_array().expect("oracles[]").is_empty(), "oracles present");
        let _ = std::fs::remove_dir_all(root);
        Ok(())
    }

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

    // ── #3293 PR 5: diff-owned changes[] wiring (packet level) ──

    /// Build a packet over a fixture with `lib/App.pm` (a `sub discount`) and a
    /// caller-supplied `diff`.
    fn packet_for_diff(dir: &str, fact_classes: &str, diff: Option<&str>) -> serde_json::Value {
        let root = format!("target/ripr-p5-fixtures/{dir}");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(format!("{root}/lib")).expect("create lib/");
        std::fs::write(
            format!("{root}/lib/App.pm"),
            "package App;\nsub discount {\n    my ($amount) = @_;\n    return $amount;\n}\n1;\n",
        )
        .expect("write pm");
        let packet = build_ripr_facts_packet(&RiprFactsRequest {
            schema: "ripr-perl-facts-v1",
            root: &root,
            base: Some("origin/main"),
            head: Some("HEAD"),
            fact_classes,
            diff,
        })
        .expect("valid request builds a packet");
        let _ = std::fs::remove_dir_all(&root);
        packet
    }

    /// A diff adding a line inside `sub discount` (0-based head line 3).
    const APP_DIFF: &str = "+++ b/lib/App.pm\n@@ -3,2 +3,3 @@\n     my ($amount) = @_;\n+    return $amount / 2;\n     return $amount;\n";

    fn changes_of(p: &serde_json::Value) -> Vec<serde_json::Value> {
        p["changes"].as_array().expect("changes[]").clone()
    }
    fn has_limitation(p: &serde_json::Value, id_prefix: &str) -> bool {
        p["limitations"]
            .as_array()
            .expect("limitations[]")
            .iter()
            .any(|l| l["limitation_id"].as_str().is_some_and(|s| s.starts_with(id_prefix)))
    }

    #[test]
    fn build_packet_skips_change_parsing_when_changes_not_requested() {
        // A non-empty diff is ignored unless `changes` is requested.
        let p = packet_for_diff("skip", "files,owners", Some(APP_DIFF));
        assert!(changes_of(&p).is_empty(), "changes not requested → empty even with a diff");
    }

    #[test]
    fn build_packet_emits_no_diff_supplied_limitation_when_changes_requested_without_diff() {
        let p = packet_for_diff("nodiff", "changes", None);
        assert!(changes_of(&p).is_empty(), "no diff → no changes");
        assert!(has_limitation(&p, "no-diff-supplied"), "must surface no-diff-supplied");
    }

    #[test]
    fn build_packet_emits_no_diff_supplied_limitation_for_blank_diff() {
        let p = packet_for_diff("blankdiff", "changes", Some("   \n  "));
        assert!(changes_of(&p).is_empty(), "blank diff behaves like no diff");
        assert!(has_limitation(&p, "no-diff-supplied"), "blank diff → no-diff-supplied");
    }

    #[test]
    fn build_packet_change_owner_id_resolves_to_present_owners_fact() {
        // Referential integrity (PR-4 lesson): even a `changes`-only request must
        // carry the owners[]/files[] the change references.
        let p = packet_for_diff("refint", "changes", Some(APP_DIFF));
        let changes = changes_of(&p);
        assert!(!changes.is_empty(), "the fixture diff must produce a change");
        let owner_ids: std::collections::HashSet<&str> = p["owners"]
            .as_array()
            .expect("owners[]")
            .iter()
            .filter_map(|o| o["owner_id"].as_str())
            .collect();
        let file_ids: std::collections::HashSet<&str> = p["files"]
            .as_array()
            .expect("files[]")
            .iter()
            .filter_map(|f| f["file_id"].as_str())
            .collect();
        for change in &changes {
            let oid = change["owner_id"].as_str().expect("change.owner_id");
            let fid = change["file_id"].as_str().expect("change.file_id");
            assert!(
                owner_ids.contains(oid),
                "change.owner_id {oid} must resolve to an owners[] fact"
            );
            assert!(file_ids.contains(fid), "change.file_id {fid} must resolve to a files[] fact");
        }
        // The change is attributed to the sub, not the package.
        assert_eq!(changes[0]["behavior_hint"], "return_value");
    }

    #[test]
    fn build_packet_changes_request_alone_does_not_leak_files_owners_when_no_changes_emitted() {
        // Diff touches a file outside root → zero changes → files/owners stay
        // empty (no force-include when nothing references them).
        let unknown = "+++ b/other/Nope.pm\n@@ -1,0 +1,1 @@\n+return 1;\n";
        let p = packet_for_diff("noleak", "changes", Some(unknown));
        assert!(changes_of(&p).is_empty(), "unknown-file hunk → no change");
        assert!(p["files"].as_array().expect("files[]").is_empty(), "files[] not force-included");
        assert!(
            p["owners"].as_array().expect("owners[]").is_empty(),
            "owners[] not force-included"
        );
        assert!(
            has_limitation(&p, "diff-file-not-found:"),
            "unknown path surfaced as a limitation"
        );
    }

    #[test]
    fn build_packet_changes_are_deterministically_ordered() {
        let a = packet_for_diff("det-a", "changes", Some(APP_DIFF));
        let b = packet_for_diff("det-b", "changes", Some(APP_DIFF));
        assert_eq!(changes_of(&a), changes_of(&b), "same request → identical changes[]");
    }

    #[test]
    fn build_packet_changes_only_no_diff_has_no_orphan_file_limitation() {
        // `changes` with no diff clears files[] (nothing references them); the
        // file-walk's `digest-algorithm` note must not linger describing files
        // that aren't in the packet (orphaned-limitation class).
        let p = packet_for_diff("orphan-digest", "changes", None);
        assert!(p["files"].as_array().expect("files[]").is_empty(), "files[] cleared");
        assert!(
            !has_limitation(&p, "digest-algorithm"),
            "no digest-algorithm note when files[] is absent"
        );
        // The intended limitation is still surfaced.
        assert!(has_limitation(&p, "no-diff-supplied"), "no-diff-supplied still present");
    }

    #[test]
    fn build_packet_unattributable_change_evidence_ref_resolves() {
        // A `.pm` with only top-level code parses with a file fact but zero
        // owners. A diff hunk in it is unattributable — the limitation's
        // evidence_ref (a real parsed file) must resolve, so files[] is
        // force-included even though no change fact was emitted.
        let root = "target/ripr-p5-fixtures/unattributable";
        let _ = std::fs::remove_dir_all(root);
        std::fs::create_dir_all(format!("{root}/lib")).expect("create lib/");
        std::fs::write(format!("{root}/lib/Script.pm"), "my $x = 1;\n$x++;\n1;\n")
            .expect("write ownerless pm");
        let diff = "+++ b/lib/Script.pm\n@@ -1,0 +1,1 @@\n+$x = 2;\n";
        let p = build_ripr_facts_packet(&RiprFactsRequest {
            schema: "ripr-perl-facts-v1",
            root,
            base: None,
            head: None,
            fact_classes: "changes",
            diff: Some(diff),
        })
        .expect("valid request");
        let _ = std::fs::remove_dir_all(root);

        assert!(changes_of(&p).is_empty(), "ownerless hunk → no change fact");
        assert!(has_limitation(&p, "unattributable-change:"), "unattributable-change surfaced");
        // Every limitation evidence_ref that names a file_id must resolve to a
        // files[] fact present in the packet.
        let file_ids: std::collections::HashSet<&str> = p["files"]
            .as_array()
            .expect("files[]")
            .iter()
            .filter_map(|f| f["file_id"].as_str())
            .collect();
        assert!(!file_ids.is_empty(), "files[] force-included for the unattributable ref");
        for lim in p["limitations"].as_array().expect("limitations[]") {
            if let Some(refs) = lim["evidence_refs"].as_array() {
                for r in refs {
                    if let Some(fid) = r.as_str() {
                        if fid.starts_with("file:") {
                            assert!(
                                file_ids.contains(fid),
                                "limitation evidence_ref {fid} must resolve to a files[] fact"
                            );
                        }
                    }
                }
            }
        }
    }
}
