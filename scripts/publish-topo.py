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
    cargo metadata --format-version=1 --no-deps | python3 scripts/publish-topo.py

Returns exit code 1 if a cycle is detected in normal deps, or if the
publish allowlist is missing / contains invalid entries.
"""

from __future__ import annotations

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


def compute_publish_order(meta: dict) -> list[dict[str, str]]:
    """
    Given parsed cargo metadata, return the publish order.

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

    # Filter to only crates listed in the workspace publish allowlist.
    allowlist = meta.get("metadata", {}).get("publish", {}).get("allow", [])
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

    result = []
    for name in order:
        if name not in allowed:
            continue
        pkg = packages[name]
        result.append({"name": name, "version": pkg["version"]})

    return result


def main() -> None:
    meta = json.load(sys.stdin)
    result = compute_publish_order(meta)
    print(json.dumps(result))


if __name__ == "__main__":
    main()
