#!/usr/bin/env python3
"""
publish-topo.py — Compute topological publish order for a Cargo workspace.

Reads `cargo metadata` JSON on stdin and prints a JSON array of
{"name": ..., "version": ...} objects in the order they should be published.

Implements Tarjan SCC to break dev-dependency cycles:
- Normal dep edges are always kept.
- Dev-dep edges that cross SCC boundaries are kept (ordering constraint).
- Dev-dep edges within an SCC are dropped (they are the only edges that can
  form cycles, e.g. crate A dev-depends on B while B normally depends on A).

This is the shared topo-sort helper used by:
- publish-crates.yml  (the actual publish workflow)
- publish-dry-run.yml  (the PR gate that catches breakage before merge)

Usage:
    # Default: read publish list from [workspace.metadata.publish.allow]
    cargo metadata --format-version=1 --no-deps | python3 scripts/publish-topo.py

    # Auto-derive: derive publishable crates from cargo metadata (no manual list)
    cargo metadata --format-version=1 --no-deps | python3 scripts/publish-topo.py --from-metadata

    # Check drift: compare hand-maintained allowlist against metadata-derived list
    cargo metadata --format-version=1 --no-deps | python3 scripts/publish-topo.py --check-drift

Returns exit code 1 if a cycle is detected in normal deps, or if the
publish allowlist is missing / contains invalid entries, or (with
--check-drift) if the allowlist diverges from the metadata-derived list.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict


def tarjan_sccs(graph: dict[str, set[str]], nodes: list[str]) -> list[list[str]]:
    """Return a list of SCCs in reverse topological order (Tarjan's algorithm)."""
    index_counter = [0]
    stack: list[str] = []
    lowlink: dict[str, int] = {}
    index: dict[str, int] = {}
    on_stack: dict[str, bool] = {}
    sccs: list[list[str]] = []

    def strongconnect(v: str) -> None:
        index[v] = index_counter[0]
        lowlink[v] = index_counter[0]
        index_counter[0] += 1
        stack.append(v)
        on_stack[v] = True
        for w in graph.get(v, set()):
            if w not in index:
                strongconnect(w)
                lowlink[v] = min(lowlink[v], lowlink[w])
            elif on_stack.get(w):
                lowlink[v] = min(lowlink[v], index[w])
        if lowlink[v] == index[v]:
            scc: list[str] = []
            while True:
                w = stack.pop()
                on_stack[w] = False
                scc.append(w)
                if w == v:
                    break
            sccs.append(scc)

    sys.setrecursionlimit(max(10000, len(nodes) * 2))
    for v in nodes:
        if v not in index:
            strongconnect(v)
    return sccs


def derive_publish_list(meta: dict) -> list[str]:
    """
    Derive the list of publishable crate names from cargo metadata.

    A crate is publishable iff:
    - It is a workspace member, AND
    - Its ``publish`` field is absent (None) or non-empty.

    In cargo metadata, ``publish = false`` and ``publish = []`` both produce
    ``"publish": []`` in the JSON output.  A non-empty list like
    ``["crates-io"]`` means the crate is published to that specific registry
    (treated as publishable here since it has at least one target).

    Returns a sorted list of crate names for deterministic output.
    """
    workspace_members = set(meta["workspace_members"])
    names: list[str] = []
    for pkg in meta["packages"]:
        if pkg["id"] not in workspace_members:
            continue
        publish_field = pkg.get("publish")
        # publish_field is None  -> no restriction (publishable)
        # publish_field is []    -> publish = false (not publishable)
        # publish_field is [...]  -> registry-restricted (publishable to those)
        if publish_field is not None and len(publish_field) == 0:
            continue
        names.append(pkg["name"])
    return sorted(names)


def check_allowlist_drift(meta: dict) -> tuple[set[str], set[str]]:
    """
    Compare the hand-maintained allowlist against the metadata-derived list.

    Returns ``(missing, extra)`` where:
    - ``missing`` — crates in metadata-derived list but absent from allowlist
    - ``extra``   — crates in allowlist but NOT in the metadata-derived list
                    (e.g. they have ``publish = false`` or were removed)

    Both sets are empty when there is no drift.
    """
    derived = set(derive_publish_list(meta))
    workspace_meta = meta.get("metadata") or {}
    publish_meta = workspace_meta.get("publish") or {}
    allowlist_raw = publish_meta.get("allow", [])
    if not isinstance(allowlist_raw, list):
        allowlist_raw = []
    allowlist = {str(c) for c in allowlist_raw if isinstance(c, str)}

    missing = derived - allowlist
    extra = allowlist - derived
    return missing, extra


def compute_publish_order(
    meta: dict, from_metadata: bool = False
) -> list[dict[str, str]]:
    """
    Given parsed cargo metadata, return the publish order.

    Parameters
    ----------
    meta:
        Parsed ``cargo metadata --no-deps`` JSON.
    from_metadata:
        When True, derive the publish list from the metadata ``publish`` field
        instead of reading ``[workspace.metadata.publish.allow]``.  Crates
        with ``publish = false`` / ``publish = []`` are excluded automatically.

    Raises SystemExit(1) on error (cycle, bad allowlist, etc.).
    """
    workspace_members = set(meta["workspace_members"])

    # Build name -> package info map (only workspace members).
    packages: dict[str, dict] = {}
    for pkg in meta["packages"]:
        if pkg["id"] in workspace_members:
            packages[pkg["name"]] = pkg

    # Build separate normal and dev dependency graphs (only internal deps).
    normal_deps: dict[str, set[str]] = defaultdict(set)
    dev_deps: dict[str, set[str]] = defaultdict(set)
    for name, pkg in packages.items():
        for dep in pkg["dependencies"]:
            if dep["name"] not in packages:
                continue
            if dep.get("kind") == "dev":
                dev_deps[name].add(dep["name"])
            else:
                normal_deps[name].add(dep["name"])

    # Tarjan SCC on the full graph (normal + dev edges).
    full_graph = {name: normal_deps[name] | dev_deps[name] for name in packages}
    sccs = tarjan_sccs(full_graph, list(packages.keys()))
    node_to_scc: dict[str, int] = {}
    for i, scc in enumerate(sccs):
        for node in scc:
            node_to_scc[node] = i

    # Build final dep graph: normal edges always included; dev edges only
    # when they cross SCC boundaries (intra-SCC dev edges are dropped to
    # break cycles).
    deps: dict[str, set[str]] = {}
    for name in packages:
        deps[name] = set(normal_deps[name])
        for dep in dev_deps[name]:
            if node_to_scc.get(dep) != node_to_scc.get(name):
                deps[name].add(dep)

    # Topological sort (Kahn algorithm).
    in_degree = {name: len(d) for name, d in deps.items()}
    queue = sorted([n for n, d in in_degree.items() if d == 0])
    order: list[str] = []

    while queue:
        node = queue.pop(0)
        order.append(node)
        for name, d in deps.items():
            if node in d:
                in_degree[name] -= 1
                if in_degree[name] == 0:
                    queue.append(name)
                    queue.sort()

    if len(order) != len(packages):
        print("ERROR: cycle detected in dependency graph", file=sys.stderr)
        sys.exit(1)

    # Build the set of crates to publish.
    #
    # Two modes:
    #  from_metadata=True  — derive from the ``publish`` field in cargo metadata.
    #                         Crates with ``publish = []`` (i.e. publish = false)
    #                         are excluded automatically.  No hand-maintained list
    #                         is required.
    #  from_metadata=False — read the explicit [workspace.metadata.publish.allow]
    #                         list.  This is the legacy behaviour (default) kept
    #                         for backwards compatibility and as a safety net
    #                         during the transition period.
    if from_metadata:
        derived = derive_publish_list(meta)
        allowed_set = set(derived)
        if len(allowed_set) == 0:
            print(
                "ERROR: No publishable crates found in workspace metadata. "
                "All workspace members have publish = false / publish = [].",
                file=sys.stderr,
            )
            sys.exit(1)
    else:
        workspace_meta = meta.get("metadata") or {}
        publish_meta = workspace_meta.get("publish") or {}
        allowlist = publish_meta.get("allow", [])
        if not isinstance(allowlist, list):
            print(
                "ERROR: Workspace publish allowlist must be a list at "
                "[workspace.metadata.publish.allow].",
                file=sys.stderr,
            )
            sys.exit(1)

        allowed: list[str] = []
        for crate_name in allowlist:
            if not isinstance(crate_name, str):
                print(
                    f"ERROR: Invalid publish allowlist entry (not a string): {crate_name}",
                    file=sys.stderr,
                )
                sys.exit(1)

            if crate_name in allowed:
                continue

            if crate_name not in packages:
                print(
                    f"ERROR: Crate in publish allowlist is not a workspace member: {crate_name}",
                    file=sys.stderr,
                )
                sys.exit(1)

            allowed.append(crate_name)

        if len(allowed) == 0:
            print(
                "ERROR: Publish allowlist is empty. Set [workspace.metadata.publish.allow] "
                "in workspace Cargo.toml.",
                file=sys.stderr,
            )
            sys.exit(1)

        allowed_set = set(allowed)

    result = []
    for name in order:
        if name not in allowed_set:
            continue
        pkg = packages[name]
        result.append({"name": name, "version": pkg["version"]})

    return result


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Compute topological publish order for a Cargo workspace.",
        epilog=(
            "Reads cargo metadata JSON from stdin.  "
            "Without flags, uses [workspace.metadata.publish.allow].  "
            "With --from-metadata, derives the list automatically."
        ),
    )
    parser.add_argument(
        "--from-metadata",
        action="store_true",
        default=False,
        help=(
            "Derive the publish list from cargo metadata instead of the "
            "hand-maintained [workspace.metadata.publish.allow] allowlist.  "
            "Crates with publish = false / publish = [] are excluded automatically."
        ),
    )
    parser.add_argument(
        "--check-drift",
        action="store_true",
        default=False,
        help=(
            "Check whether [workspace.metadata.publish.allow] matches the "
            "metadata-derived list.  Exits 1 with a diff if gaps exist, 0 if clean.  "
            "Useful as a CI guard during the transition period."
        ),
    )
    args = parser.parse_args()

    meta = json.load(sys.stdin)

    if args.check_drift:
        missing, extra = check_allowlist_drift(meta)
        if not missing and not extra:
            print("Allowlist matches metadata-derived list. No drift detected.")
            sys.exit(0)
        if missing:
            print(
                "ERROR: The following crates are publishable (no publish=false) "
                "but are MISSING from [workspace.metadata.publish.allow]:",
                file=sys.stderr,
            )
            for name in sorted(missing):
                print(f"  + {name}", file=sys.stderr)
        if extra:
            print(
                "ERROR: The following crates are in [workspace.metadata.publish.allow] "
                "but have publish=false in their Cargo.toml (or are no longer workspace members):",
                file=sys.stderr,
            )
            for name in sorted(extra):
                print(f"  - {name}", file=sys.stderr)
        print(
            "\nFix: update [workspace.metadata.publish.allow] in root Cargo.toml "
            "to match the metadata-derived list, or set publish = false in the "
            "relevant Cargo.toml files.",
            file=sys.stderr,
        )
        sys.exit(1)

    result = compute_publish_order(meta, from_metadata=args.from_metadata)
    print(json.dumps(result))


if __name__ == "__main__":
    main()
