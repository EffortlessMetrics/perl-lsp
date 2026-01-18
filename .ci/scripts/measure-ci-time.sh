#!/usr/bin/env bash
# Measure runtimes of the local CI lanes (Issue #211).
#
# Goal: baseline timings before changing workflows, so we can prove cost savings.
#
# Output:
#   artifacts/ci-time.json  (machine-readable)
#   artifacts/ci-time.md    (human-readable)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACTS="$ROOT/artifacts"
mkdir -p "$ARTIFACTS"

python_bin="python3"
if ! command -v python3 >/dev/null 2>&1; then
  python_bin="python"
fi

now_iso() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

time_cmd() {
  local name="$1"
  shift
  echo "==> $name" >&2
  "$python_bin" - "$name" "$@" <<'PY'
import json
import os
import subprocess
import sys
import time

name = sys.argv[1]
cmd = sys.argv[2:]

start = time.perf_counter()
proc = subprocess.run(
    cmd,
    cwd=os.environ.get("ROOT"),
    text=True,
    stdout=sys.stderr,
    stderr=sys.stderr,
)
end = time.perf_counter()

sys.stdout.write(json.dumps({"name": name, "seconds": round(end - start, 3), "returncode": proc.returncode}) + "\n")
sys.exit(proc.returncode)
PY
}

export ROOT

tmp_json="$ARTIFACTS/ci-time.ndjson"
rm -f "$tmp_json"

# Lane set: mirrors what actually matters for merges.
# Keep this small and stable; add more lanes only when we decide to pay for them.
time_cmd "ci-format"             just ci-format             | tee -a "$tmp_json"
time_cmd "ci-docs-check"         just ci-docs-check         | tee -a "$tmp_json"
time_cmd "ci-clippy-lib"         just ci-clippy-lib         | tee -a "$tmp_json"
time_cmd "clippy-prod-no-unwrap" just clippy-prod-no-unwrap | tee -a "$tmp_json"
time_cmd "ci-test-lib"           just ci-test-lib           | tee -a "$tmp_json"
time_cmd "ci-lsp-def"            just ci-lsp-def            | tee -a "$tmp_json"
time_cmd "status-check"          just status-check          | tee -a "$tmp_json"

# Build consolidated JSON
"$python_bin" - "$tmp_json" "$ARTIFACTS/ci-time.json" "$(now_iso)" <<'PY'
import json
import sys
from pathlib import Path

ndjson = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
out_path = Path(sys.argv[2])
generated_at = sys.argv[3]

rows = [json.loads(line) for line in ndjson if line.strip()]
total = round(sum(r["seconds"] for r in rows), 3)

payload = {
  "generated_at": generated_at,
  "lanes": rows,
  "total_seconds": total,
}
out_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY

# Build Markdown table
"$python_bin" - "$ARTIFACTS/ci-time.json" "$ARTIFACTS/ci-time.md" <<'PY'
import json
import sys
from pathlib import Path

data = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
out = Path(sys.argv[2])

lines = []
lines.append("# CI Timing Baseline")
lines.append("")
lines.append(f"- Generated at: `{data['generated_at']}`")
lines.append(f"- Total: `{data['total_seconds']}s`")
lines.append("")
lines.append("| Lane | Seconds | RC |")
lines.append("|------|---------|----|")
for r in data["lanes"]:
  lines.append(f"| `{r['name']}` | {r['seconds']} | {r['returncode']} |")
out.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY

echo ""
echo "Wrote:"
echo "  - $ARTIFACTS/ci-time.json"
echo "  - $ARTIFACTS/ci-time.md"
