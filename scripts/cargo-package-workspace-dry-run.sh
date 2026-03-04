#!/usr/bin/env bash
set -euo pipefail

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
    publish = pkg.get("publish")
    if publish is not None and len(publish) == 0:
        continue
    crate_dir = os.path.dirname(pkg["manifest_path"])
    rel_path = os.path.relpath(crate_dir, workspace_root)
    print("--config=patch.crates-io.{}.path=\"{}\"".format(pkg["name"], rel_path))
'
)"

mapfile -t PATCH_ARGS <<< "${PATCH_OUTPUT}"

ALLOWLIST_OUTPUT="$(
  cargo metadata --format-version=1 --no-deps | python3 -c '
import json
import sys

meta = json.load(sys.stdin)
allowlist = meta.get("metadata", {}).get("publish", {}).get("allow")

if not isinstance(allowlist, list) or not allowlist:
    raise SystemExit("[workspace.metadata.publish.allow] is missing or empty in Cargo.toml")

for crate in allowlist:
    if not isinstance(crate, str):
        raise SystemExit("publish allowlist contains a non-string value")
    print(crate)
'
)"

mapfile -t ALLOWLIST_CRATES <<< "${ALLOWLIST_OUTPUT}"

if [[ $# -eq 0 ]]; then
  echo "No crates provided. Using [workspace.metadata.publish.allow] from Cargo.toml."
  CRATES=("${ALLOWLIST_CRATES[@]}")
else
  CRATES=("$@")
fi

NO_VERIFY="${CARGO_PACKAGE_NO_VERIFY:-0}"

for crate in "${CRATES[@]}"; do
  echo "==> cargo package -p ${crate}"
  CMD=(cargo package -p "${crate}" "${PATCH_ARGS[@]}")
  if [[ "${NO_VERIFY}" == "1" ]]; then
    CMD+=(--no-verify)
  fi
  "${CMD[@]}"
done
