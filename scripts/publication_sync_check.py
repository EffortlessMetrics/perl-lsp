#!/usr/bin/env python3
"""Publication Sync Contract validator for the audited 0.18.0-rc.1 join.

Implements EffortlessMetrics/perl-lsp-swarm#12231 in perl-lsp. Two modes:

    ordinary PR                    -> deterministic ``not_applicable`` success
    explicit publication-sync PR   -> fail closed unless the exact packet,
                                      ancestry, tree, and no-publish contract
                                      all pass

Sync mode requires BOTH repository-owned markers:

1. the committed packet at ``.github/publication-sync/packet.yaml`` in the
   PR head tree, and
2. the dedicated PR-template field ``- Publication-sync PR (yes/no): yes``
   in the PR body.

A title substring is not authority. A packet without the field, or the field
without a packet, fails closed: sync content may not slip through undeclared,
and a declared sync may not arrive without its packet.

Commit-identity design (why ``sync_join_sha`` is the *core* join):

The packet is committed inside the PR head J, so it cannot literally name
J's own commit id (a commit cannot contain its own hash). The producer
therefore builds two commits with identical parents ``[R, S]``, message, and
author/committer identity:

    J0 = core join: tree is exactly the reviewed projection
    J  = PR head:   tree is the projection plus the control-plane directory
                    .github/publication-sync/ (packet, ledger, manifest)

``sync_join_sha`` names J0. Proof 2 re-derives J0 from J (same metadata and
declared parents, tree = projection) and requires an exact match, and proof 7
requires the derived projection tree to equal ``expected_projected_tree``.
Together they prove the PR head is precisely "audited join + control files"
and nothing else. ``git diff S..J`` is likewise compared against the manifest
with the control directory excluded.

The packet bytes must be JSON-parseable (JSON is valid YAML 1.2) so this
validator stays deterministic on the Python standard library. The durable
contract is ``schemas/publication_sync.v2.schema.json``.

This script holds no publication authority: it runs only git inspection
plumbing plus local object construction (``mktree``/``commit-tree`` write
loose objects into the local object store and mutate neither refs, index,
nor working tree), and it uses no credential. The permission/command ratchet
lives in ``scripts/tests/test-publication-sync-contract.py``.

Subcommands:

    pr          Pre-merge contract on exact PR base/head (proofs 1-11 of the
                controlling issue).
    post-merge  Bounded verifier for the wrapper merge M: J is an ancestor
                of M, tree(M) == tree(J), R and S are ancestors of M, and
                current master == M. A squash/rebase landing fails.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, NoReturn, Optional, Sequence, Tuple

PACKET_PATH = ".github/publication-sync/packet.yaml"
LEDGER_PATH = ".github/publication-sync/reconciliation-ledger.json"
MANIFEST_PATH = ".github/publication-sync/projection-manifest.json"
# Control-plane directory: packet, ledger, and manifest live here. These
# files are the contract, not the projection, so they are excluded from the
# projection-tree derivation and from the S..J path-set comparison.
CONTROL_DIR = ".github/publication-sync/"
CONTROL_DIR_PARENT = ".github"
CONTROL_DIR_NAME = "publication-sync"

SCHEMA_VERSION = "v0.18_publication_sync.v2"
LEDGER_SCHEMA_VERSION = "v0.18_reconciliation.v1"
MANIFEST_SCHEMA_VERSION = "v0.18_publication_sync_manifest.v1"
RELEASE = "0.18.0-rc.1"

HEX40 = re.compile(r"^[0-9a-f]{40}$")
DIGEST = re.compile(r"^sha256:[0-9a-f]{64}$")
REPO_PATH = re.compile(r"^[A-Za-z0-9_.@+-][A-Za-z0-9_.@+/-]{0,255}$")
MARKER_FIELD = re.compile(
    r"^[ \t]*-[ \t]*Publication-sync PR[^:\n]*:[ \t]*(yes|no)[ \t]*$",
    re.IGNORECASE | re.MULTILINE,
)
IDENT = re.compile(r"^(.*?) <(.*?)> (\d+) ([+-]\d{4})$")
REQUIRED_INVARIANT_IDS = (
    "destination-context",
    "version",
    "topology",
    "artifact-reachability",
    "effective-identity",
)

PACKET_KEYS = {
    "schema_version",
    "release",
    "release_base_sha",
    "prepared_swarm_sha",
    "sync_join_sha",
    "expected_join_parents",
    "reconciliation_digest",
    "publication_sync_manifest_digest",
    "expected_projected_tree",
    "published_channels",
    "release_cut",
}


class ContractFailure(Exception):
    """A named contract proof failed. Never a success path."""


def fail(message: str) -> NoReturn:
    raise ContractFailure(message)


def run_git(repo: Path, args: Sequence[str], *, input_text: Optional[str] = None,
            env: Optional[Dict[str, str]] = None) -> str:
    """Run a git command; instrument failure is not success."""
    proc = subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        input=input_text,
        env=env,
        check=False,
    )
    if proc.returncode != 0:
        fail(f"git {' '.join(args)} failed: {proc.stderr.strip() or proc.stdout.strip()}")
    return proc.stdout


def git_ok(repo: Path, args: Sequence[str]) -> bool:
    proc = subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        check=False,
    )
    return proc.returncode == 0


def tree_file(repo: Path, commit: str, path: str) -> Optional[bytes]:
    """Bytes of ``path`` at ``commit``; None when absent."""
    proc = subprocess.run(
        ["git", "-C", str(repo), "show", f"{commit}:{path}"],
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        return None
    return proc.stdout


def sha256_prefixed(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def load_json_bytes(data: bytes, what: str) -> Any:
    try:
        return json.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        fail(f"{what} is not valid JSON-shaped YAML: {exc}")


def require_hex40(value: Any, what: str) -> str:
    if not isinstance(value, str) or not HEX40.match(value):
        fail(f"{what} must be a 40-char lowercase hex sha, got {value!r}")
    return value


def check_marker_field(body: str) -> Optional[bool]:
    """Parse the dedicated PR-template marker field.

    Returns True (yes), False (no), or None (absent). Conflicting values
    fail closed: an ambiguous declaration is not authority either.
    """
    values = {m.group(1).lower() for m in MARKER_FIELD.finditer(body)}
    if not values:
        return None
    if len(values) > 1:
        fail("PR body carries conflicting Publication-sync PR marker values")
    return values.pop() == "yes"


def validate_packet_shape(packet: Any) -> dict:
    """Hand-rolled strict validation against schemas/publication_sync.v2.schema.json."""
    if not isinstance(packet, dict):
        fail("packet must be a JSON object")
    keys = set(packet)
    missing = PACKET_KEYS - keys
    extra = keys - PACKET_KEYS
    if missing:
        fail(f"packet missing required keys: {sorted(missing)}")
    if extra:
        fail(f"packet carries unknown keys: {sorted(extra)}")
    if packet["schema_version"] != SCHEMA_VERSION:
        fail(f"packet schema_version must be {SCHEMA_VERSION}, got {packet['schema_version']!r}")
    if packet["release"] != RELEASE:
        fail(f"packet release must be {RELEASE}, got {packet['release']!r}")
    require_hex40(packet["release_base_sha"], "release_base_sha")
    require_hex40(packet["prepared_swarm_sha"], "prepared_swarm_sha")
    require_hex40(packet["sync_join_sha"], "sync_join_sha")
    require_hex40(packet["expected_projected_tree"], "expected_projected_tree")
    parents = packet["expected_join_parents"]
    if (
        not isinstance(parents, list)
        or len(parents) != 2
        or any(not isinstance(p, str) or not HEX40.match(p) for p in parents)
    ):
        fail("expected_join_parents must be exactly two 40-char hex shas")
    for key in ("reconciliation_digest", "publication_sync_manifest_digest"):
        if not isinstance(packet[key], str) or not DIGEST.match(packet[key]):
            fail(f"{key} must match sha256:<64 hex>, got {packet[key]!r}")
    if packet["published_channels"] != []:
        fail(f"no-publish contract: published_channels must be [], got {packet['published_channels']!r}")
    if packet["release_cut"] is not False:
        fail(f"no-publish contract: release_cut must be false, got {packet['release_cut']!r}")
    return packet


def validate_ledger(ledger: Any) -> None:
    allowed = {"schema_version", "release", "decisions", "blockers"}
    if not isinstance(ledger, dict):
        fail("reconciliation ledger must be a JSON object")
    if set(ledger) - allowed:
        fail(f"reconciliation ledger carries unknown keys: {sorted(set(ledger) - allowed)}")
    if ledger.get("schema_version") != LEDGER_SCHEMA_VERSION:
        fail(f"ledger schema_version must be {LEDGER_SCHEMA_VERSION}, got {ledger.get('schema_version')!r}")
    if ledger.get("release") != RELEASE:
        fail(f"ledger release must be {RELEASE}, got {ledger.get('release')!r}")
    if ledger.get("blockers") != []:
        fail(f"reconciliation ledger is not blocker-free: {ledger.get('blockers')!r}")
    decisions = ledger.get("decisions")
    if not isinstance(decisions, list):
        fail("reconciliation ledger decisions must be an array")
    for decision in decisions:
        if not isinstance(decision, dict) or not isinstance(decision.get("id"), str):
            fail("each reconciliation decision needs a string id")
        # Fail closed: only an explicit resolved decision is accepted. A
        # blocked or not_proven decision can never read as pass (proof 11).
        if decision.get("status") != "resolved":
            fail(
                f"reconciliation decision {decision.get('id')!r} has status "
                f"{decision.get('status')!r}; only 'resolved' is acceptable"
            )


def validate_manifest(manifest: Any) -> dict:
    allowed = {"schema_version", "release", "translations", "exclusions", "required_invariants"}
    if not isinstance(manifest, dict):
        fail("projection manifest must be a JSON object")
    if set(manifest) - allowed:
        fail(f"projection manifest carries unknown keys: {sorted(set(manifest) - allowed)}")
    if manifest.get("schema_version") != MANIFEST_SCHEMA_VERSION:
        fail(f"manifest schema_version must be {MANIFEST_SCHEMA_VERSION}, got {manifest.get('schema_version')!r}")
    if manifest.get("release") != RELEASE:
        fail(f"manifest release must be {RELEASE}, got {manifest.get('release')!r}")
    translations = manifest.get("translations")
    exclusions = manifest.get("exclusions")
    if not isinstance(translations, list) or not isinstance(exclusions, list):
        fail("manifest translations and exclusions must be arrays")
    listed_paths = set()
    for translation in translations:
        if not isinstance(translation, dict):
            fail("each translation must be an object with from/to")
        for key in ("from", "to"):
            value = translation.get(key)
            if not isinstance(value, str) or not REPO_PATH.match(value) or ".." in value.split("/"):
                fail(f"translation {key} must be a safe repo-relative path, got {value!r}")
            listed_paths.add(value)
    for path in exclusions:
        if not isinstance(path, str) or not REPO_PATH.match(path) or ".." in path.split("/"):
            fail(f"exclusion must be a safe repo-relative path, got {path!r}")
        listed_paths.add(path)
    invariants = manifest.get("required_invariants")
    if not isinstance(invariants, list):
        fail("manifest required_invariants must be an array")
    seen = set()
    for invariant in invariants:
        if not isinstance(invariant, dict) or not isinstance(invariant.get("id"), str):
            fail("each required invariant needs a string id")
        seen.add(invariant["id"])
        # Fail closed: required uncertainty (not_proven or anything else) is
        # never a pass (proof 11).
        if invariant.get("status") != "pass":
            fail(
                f"required invariant {invariant['id']!r} has status "
                f"{invariant.get('status')!r}; only 'pass' is acceptable"
            )
    missing_ids = [i for i in REQUIRED_INVARIANT_IDS if i not in seen]
    if missing_ids:
        fail(f"manifest omits required invariants: {missing_ids}")
    return {"listed_paths": listed_paths}


def ls_tree(repo: Path, treeish: str) -> List[str]:
    """Non-recursive ls-tree lines (``mode type sha\\tname``), or [] if absent."""
    proc = subprocess.run(
        ["git", "-C", str(repo), "ls-tree", treeish],
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        return []
    return [line for line in proc.stdout.splitlines() if line]


def mktree(repo: Path, entries: List[str]) -> str:
    """Build a tree object from ls-tree-format lines (git sorts them)."""
    return run_git(repo, ["mktree"], input_text="\n".join(entries) + ("\n" if entries else "")).strip()


def projected_tree(repo: Path, commit: str) -> str:
    """Tree of ``commit`` with the control-plane directory excluded.

    Removes ``.github/publication-sync``; drops ``.github`` entirely when it
    becomes empty (git trees conventionally omit empty subtrees).
    """
    root_entries = ls_tree(repo, commit)
    if not root_entries:
        fail(f"{commit} has no readable root tree")
    rebuilt_root = []
    for entry in root_entries:
        meta, _, name = entry.partition("\t")
        if name != CONTROL_DIR_PARENT:
            rebuilt_root.append(entry)
            continue
        sub_entries = [
            sub
            for sub in ls_tree(repo, f"{commit}:{CONTROL_DIR_PARENT}")
            if sub.partition("\t")[2] != CONTROL_DIR_NAME
        ]
        if sub_entries:
            new_sub = mktree(repo, sub_entries)
            mode = meta.split(" ", 1)[0]
            rebuilt_root.append(f"{mode} tree {new_sub}\t{CONTROL_DIR_PARENT}")
        # else: .github held only the control directory; drop it.
    return mktree(repo, rebuilt_root)


def commit_metadata(repo: Path, commit: str) -> Tuple[Dict[str, str], str]:
    """(author/committer env, message bytes) of a commit."""
    raw = run_git(repo, ["cat-file", "commit", commit])
    header, sep, message = raw.partition("\n\n")
    if not sep:
        fail(f"{commit} has a malformed commit object")
    env = dict(os.environ)
    found = set()
    for line in header.splitlines():
        for role, prefix in (("AUTHOR", "author "), ("COMMITTER", "committer ")):
            if line.startswith(prefix):
                match = IDENT.match(line[len(prefix):])
                if not match:
                    fail(f"{commit} has an unparseable {prefix.strip()} ident: {line!r}")
                name, email, ts, tz = match.groups()
                env[f"GIT_{role}_NAME"] = name
                env[f"GIT_{role}_EMAIL"] = email
                env[f"GIT_{role}_DATE"] = f"{ts} {tz}"
                found.add(role)
    if found != {"AUTHOR", "COMMITTER"}:
        fail(f"{commit} is missing author/committer idents")
    return env, message


def derive_core_join(repo: Path, commit: str, parents: Sequence[str]) -> str:
    """Rebuild the core join: same metadata/message/parents, projection tree."""
    env, message = commit_metadata(repo, commit)
    tree = projected_tree(repo, commit)
    args = ["commit-tree", tree]
    for parent in parents:
        args += ["-p", parent]
    return run_git(repo, args, input_text=message, env=env).strip()


def check_pr(args: argparse.Namespace) -> int:
    repo = Path(args.repo).resolve()
    require_hex40(args.base_sha, "--base-sha")
    require_hex40(args.head_sha, "--head-sha")
    body = Path(args.pr_body_file).read_text(encoding="utf-8") if args.pr_body_file else ""
    body = body.replace("\r\n", "\n").replace("\r", "\n")  # GitHub bodies may be CRLF
    marker = check_marker_field(body)

    packet_bytes = tree_file(repo, args.head_sha, PACKET_PATH)

    # Mode decision. Both markers are repository-owned; a title substring is
    # never consulted.
    if packet_bytes is None and marker is not True:
        # Deterministic non-sync success (positive control 1).
        print("not_applicable: no committed packet and marker field is not 'yes'")
        print("publication-sync: not_applicable")
        return 0
    if packet_bytes is None:
        fail("declared publication-sync PR (marker field 'yes') without a committed packet")
    if marker is not True:
        fail(
            "committed publication-sync packet present without the PR-template "
            "marker field set to 'yes'; sync content may not arrive undeclared"
        )

    packet = validate_packet_shape(load_json_bytes(packet_bytes, "publication-sync packet"))
    base_r = packet["release_base_sha"]
    swarm_s = packet["prepared_swarm_sha"]
    core_join = packet["sync_join_sha"]

    # Packet self-consistency: declared parents are exactly [R, S].
    if packet["expected_join_parents"] != [base_r, swarm_s]:
        fail("expected_join_parents must equal [release_base_sha, prepared_swarm_sha]")

    # Proof 1: PR base equals declared R.
    if args.base_sha != base_r:
        fail(f"proof 1: PR base {args.base_sha} != declared release_base_sha {base_r}")
    print(f"PASS proof 1: PR base == R ({base_r})")

    # Proof 2: PR head re-derives exactly the declared core join J0 (same
    # parents, message, and identity; tree = projection without control
    # files). See the module docstring for why J0 is the core join.
    try:
        derived = derive_core_join(repo, args.head_sha, [base_r, swarm_s])
    except ContractFailure as exc:
        fail(f"proof 2: {exc}")
    if derived != core_join:
        fail(
            f"proof 2: PR head re-derives core join {derived}, "
            f"declared sync_join_sha is {core_join}"
        )
    print(f"PASS proof 2: PR head is exactly declared join {core_join} plus control files")

    # Proof 3: J has exactly two ordered parents [R, S].
    parents_line = run_git(repo, ["rev-list", "--parents", "-n", "1", args.head_sha]).split()
    if parents_line != [args.head_sha, base_r, swarm_s]:
        fail(f"proof 3: J parents are {parents_line[1:]}, expected [{base_r}, {swarm_s}]")
    print("PASS proof 3: J has exactly two ordered parents [R, S]")

    # Proof 4: S resolves to the exact prepared swarm commit and is reachable from J.
    if not git_ok(repo, ["cat-file", "-e", f"{swarm_s}^{{commit}}"]):
        fail(f"proof 4: prepared swarm commit {swarm_s} does not resolve")
    if not git_ok(repo, ["merge-base", "--is-ancestor", swarm_s, args.head_sha]):
        fail(f"proof 4: S {swarm_s} is not reachable from J {args.head_sha}")
    print("PASS proof 4: S resolves and is reachable from J")

    # Proof 5: final reconciliation ledger is valid, blocker-free, digest-matched.
    ledger_bytes = tree_file(repo, args.head_sha, LEDGER_PATH)
    if ledger_bytes is None:
        fail(f"proof 5: reconciliation ledger missing at {LEDGER_PATH} in tree(J)")
    if sha256_prefixed(ledger_bytes) != packet["reconciliation_digest"]:
        fail("proof 5: reconciliation ledger digest mismatch")
    try:
        validate_ledger(load_json_bytes(ledger_bytes, "reconciliation ledger"))
    except ContractFailure as exc:
        fail(f"proof 5: {exc}")
    print("PASS proof 5: reconciliation ledger valid, blocker-free, digest-matched")

    # Proof 6: publication projection manifest is valid and digest-matched.
    manifest_bytes = tree_file(repo, args.head_sha, MANIFEST_PATH)
    if manifest_bytes is None:
        fail(f"proof 6: projection manifest missing at {MANIFEST_PATH} in tree(J)")
    if sha256_prefixed(manifest_bytes) != packet["publication_sync_manifest_digest"]:
        fail("proof 6: projection manifest digest mismatch")
    try:
        manifest = validate_manifest(load_json_bytes(manifest_bytes, "projection manifest"))
    except ContractFailure as exc:
        fail(f"proof 6: {exc}")
    print("PASS proof 6: projection manifest valid and digest-matched")

    # Proof 7: tree(J) with the control directory excluded equals
    # expected_projected_tree.
    actual_tree = projected_tree(repo, args.head_sha)
    if actual_tree != packet["expected_projected_tree"]:
        fail(
            f"proof 7: projected tree(J) {actual_tree} != expected_projected_tree "
            f"{packet['expected_projected_tree']}"
        )
    print(f"PASS proof 7: tree(J) == expected_projected_tree ({actual_tree})")

    # Proof 8: git diff S..J contains exactly manifest-listed
    # translations/exclusions (control-plane directory excluded).
    diff_output = run_git(repo, ["diff", "--name-only", swarm_s, args.head_sha])
    changed = {
        line.strip()
        for line in diff_output.splitlines()
        if line.strip() and not line.strip().startswith(CONTROL_DIR)
    }
    listed = manifest["listed_paths"]
    unlisted = sorted(changed - listed)
    missing = sorted(listed - changed)
    if unlisted:
        fail(f"proof 8: paths differ from S without a manifest entry: {unlisted}")
    if missing:
        fail(f"proof 8: manifest-listed paths absent from the S..J diff: {missing}")
    print(f"PASS proof 8: S..J diff is exactly the {len(listed)} manifest-listed path(s)")

    # Proof 9: every named invariant passed (enforced inside validate_manifest).
    print("PASS proof 9: destination-context, version, topology, artifact-reachability, "
          "effective-identity invariants all pass")

    # Proof 10: no-publish contract (shape-validated above; restated for the log).
    print("PASS proof 10: published_channels == [] and release_cut == false; "
          "check holds read-only permissions and no publication credential")

    # Proof 11: enforced structurally — every missing, unknown, or uncertain
    # input above is a hard failure, never a success.
    print("PASS proof 11: no missing/skipped/uncertain evidence was accepted as success")

    print("publication-sync: pass")
    return 0


def check_post_merge(args: argparse.Namespace) -> int:
    repo = Path(args.repo).resolve()
    merge_m = require_hex40(args.merge_sha, "--merge-sha")
    master_ref = args.master_ref

    packet_bytes = tree_file(repo, merge_m, PACKET_PATH)
    if packet_bytes is None:
        fail(f"no publication-sync packet at {PACKET_PATH} in tree(M); nothing to verify")
    packet = validate_packet_shape(load_json_bytes(packet_bytes, "publication-sync packet"))
    base_r = packet["release_base_sha"]
    swarm_s = packet["prepared_swarm_sha"]

    # The PR head J is re-derived from the packet-declared core join the same
    # way as in pre-merge mode, then the wrapper M must preserve J itself.
    # J is the commit whose tree contains this packet; locate it as the
    # second parent of M first (GitHub wrapper merges carry the PR head as
    # second parent), then verify.
    parents_line = run_git(repo, ["rev-list", "--parents", "-n", "1", merge_m]).split()
    if len(parents_line) < 2:
        fail(f"post-merge: M {merge_m} has no parents; not a wrapper merge")
    join_j = parents_line[2] if len(parents_line) == 3 else None
    if join_j is None:
        fail(
            f"post-merge: M {merge_m} has parents {parents_line[1:]}; "
            "expected exactly two (master tip, PR head J) — squash/rebase landing?"
        )

    # J is an ancestor of M (direct second parent here) and must re-derive
    # the declared core join, binding M to the audited packet.
    derived = derive_core_join(repo, join_j, [base_r, swarm_s])
    if derived != packet["sync_join_sha"]:
        fail(
            f"post-merge: second parent of M re-derives {derived}, "
            f"declared sync_join_sha is {packet['sync_join_sha']}"
        )
    if not git_ok(repo, ["merge-base", "--is-ancestor", join_j, merge_m]):
        fail(f"post-merge: J {join_j} is not an ancestor of M {merge_m} (squash/rebase landing?)")
    print(f"PASS post-merge: J ({join_j}) is an ancestor of M ({merge_m})")

    # tree(M) == tree(J): the wrapper added nothing.
    tree_m = run_git(repo, ["rev-parse", f"{merge_m}^{{tree}}"]).strip()
    tree_j = run_git(repo, ["rev-parse", f"{join_j}^{{tree}}"]).strip()
    if tree_m != tree_j:
        fail(f"post-merge: tree(M) {tree_m} != tree(J) {tree_j}")
    print(f"PASS post-merge: tree(M) == tree(J) ({tree_j})")

    for name, sha in (("R", base_r), ("S", swarm_s)):
        if not git_ok(repo, ["merge-base", "--is-ancestor", sha, merge_m]):
            fail(f"post-merge: {name} {sha} is not an ancestor of M {merge_m}")
        print(f"PASS post-merge: {name} ({sha}) is an ancestor of M")

    current = run_git(repo, ["rev-parse", master_ref]).strip()
    if current != merge_m:
        fail(f"post-merge: current {master_ref} is {current}, expected M {merge_m}")
    print(f"PASS post-merge: current {master_ref} == M ({merge_m})")

    print("publication-sync post-merge: pass")
    return 0


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)

    pr = sub.add_parser("pr", help="pre-merge publication-sync contract")
    pr.add_argument("--repo", required=True)
    pr.add_argument("--base-sha", required=True)
    pr.add_argument("--head-sha", required=True)
    pr.add_argument("--pr-body-file", default=None)

    pm = sub.add_parser("post-merge", help="post-merge wrapper verification")
    pm.add_argument("--repo", required=True)
    pm.add_argument("--merge-sha", required=True)
    pm.add_argument("--master-ref", default="refs/heads/master")

    args = parser.parse_args(argv)
    try:
        if args.command == "pr":
            return check_pr(args)
        return check_post_merge(args)
    except ContractFailure as exc:
        # GitHub Actions annotation syntax; also readable in local output.
        print(f"::error::Publication Sync Contract FAILED: {exc}")
        return 1
    except Exception as exc:  # instrument failure is never success
        print(f"::error::Publication Sync Contract instrument failure: {exc!r}")
        return 2


if __name__ == "__main__":
    sys.exit(main())
