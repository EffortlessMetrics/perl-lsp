#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <crate> [crate ...]" >&2
  exit 1
fi

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

cd "${WORKSPACE_ROOT}"

PATCH_OUTPUT="$(
  cargo metadata --format-version=1 --no-deps | python3 -c '
import json
import os
import sys

meta = json.load(sys.stdin)
workspace_members = set(meta["workspace_members"])
workspace_root = meta["workspace_root"]

for pkg in sorted(meta["packages"], key=lambda p: p["name"]):
    if pkg["id"] not in workspace_members:
        continue
    # Include publish-disabled workspace crates in patch config to allow
    # local workspace dependency resolution during dry-run packaging.
    crate_dir = os.path.dirname(pkg["manifest_path"])
    rel_path = os.path.relpath(crate_dir, workspace_root)
    print("--config=patch.crates-io.{}.path=\"{}\"".format(pkg["name"], rel_path))
'
)"

mapfile -t PATCH_ARGS <<< "${PATCH_OUTPUT}"

NO_VERIFY="${CARGO_PACKAGE_NO_VERIFY:-0}"

for crate in "$@"; do
  echo "==> cargo package -p ${crate}"
  CMD=(cargo package -p "${crate}" "${PATCH_ARGS[@]}")
  if [[ "${NO_VERIFY}" == "1" ]]; then
    # --allow-dirty is required here because the publish-dry-run gate strips
    # dev-dependencies from Cargo.toml before packaging (to avoid resolution
    # failures on workspace-sibling dev-deps not yet on crates.io). The strip
    # leaves the file modified but not staged, so cargo package would fail
    # without this flag. This is safe: we're just verifying the package
    # structure, not actually publishing.
    CMD+=(--no-verify --allow-dirty)
  fi
  "${CMD[@]}"
done
