#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <crate> [crate ...]" >&2
  exit 1
fi

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

cd "${WORKSPACE_ROOT}"

NO_VERIFY="${CARGO_PACKAGE_NO_VERIFY:-0}"

for crate in "$@"; do
  PATCH_OUTPUT="$(
    cargo metadata --format-version=1 --all-features | TARGET_CRATE="${crate}" python3 -c '
import json
import os
import sys

meta = json.load(sys.stdin)
target_name = os.environ["TARGET_CRATE"]
workspace_members = set(meta["workspace_members"])
workspace_root = meta["workspace_root"]
packages_by_id = {pkg["id"]: pkg for pkg in meta["packages"]}

resolve = meta.get("resolve") or {}
nodes = resolve.get("nodes") or []

deps_by_id = {
    node["id"]: [dep["pkg"] for dep in node.get("deps", [])]
    for node in nodes
}

start_ids = [
    pkg_id
    for pkg_id in workspace_members
    if packages_by_id[pkg_id]["name"] == target_name
]

if not start_ids:
    print(f"Unknown workspace crate: {target_name}", file=sys.stderr)
    sys.exit(2)

reachable = set()
stack = list(start_ids)
while stack:
    pkg_id = stack.pop()
    if pkg_id in reachable:
        continue
    reachable.add(pkg_id)
    stack.extend(deps_by_id.get(pkg_id, []))

for pkg_id in sorted(reachable, key=lambda i: packages_by_id[i]["name"]):
    if pkg_id not in workspace_members:
        continue
    pkg = packages_by_id[pkg_id]
    publish = pkg.get("publish")
    if publish is not None and len(publish) == 0:
        continue
    crate_dir = os.path.dirname(pkg["manifest_path"])
    rel_path = os.path.relpath(crate_dir, workspace_root)
    print("--config=patch.crates-io.{}.path=\"{}\"".format(pkg["name"], rel_path))
'
  )"

  mapfile -t PATCH_ARGS <<< "${PATCH_OUTPUT}"

  echo "==> cargo package -p ${crate}"
  CMD=(cargo package -p "${crate}" "${PATCH_ARGS[@]}")
  if [[ "${NO_VERIFY}" == "1" ]]; then
    CMD+=(--no-verify)
  fi
  "${CMD[@]}"
done
