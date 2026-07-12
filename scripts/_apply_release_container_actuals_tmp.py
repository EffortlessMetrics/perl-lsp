#!/usr/bin/env python3
"""One-shot exact edit for the audited container-channel backfill.

Temporary release-maintenance helper. Validate every edit before writing any file.
"""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "policy" / "release-container-actuals.json"


def newline_for(data: bytes) -> bytes:
    return b"\r\n" if b"\r\n" in data else b"\n"


def block(value: str, newline: bytes) -> bytes:
    return value.replace("\n", newline.decode("ascii")).encode("utf-8")


def replace_once(data: bytes, old: str, new: str, *, label: str) -> bytes:
    newline = newline_for(data)
    old_bytes = block(old, newline)
    new_bytes = block(new, newline)
    count = data.count(old_bytes)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact match, found {count}")
    return data.replace(old_bytes, new_bytes, 1)


def update_docker_frontmatter(data: bytes, value: str, *, label: str) -> bytes:
    newline = newline_for(data)
    delimiter = b"---" + newline
    if not data.startswith(delimiter):
        raise SystemExit(f"{label}: missing opening frontmatter delimiter")
    end = data.find(delimiter, len(delimiter))
    if end < 0:
        raise SystemExit(f"{label}: missing closing frontmatter delimiter")
    front_end = end + len(delimiter)
    front = data[:front_end]
    tail = data[front_end:]
    lines = front.splitlines(keepends=True)
    matches = [index for index, line in enumerate(lines) if line.startswith(b"  docker:")]
    if len(matches) != 1:
        raise SystemExit(f"{label}: expected one docker channel line, found {len(matches)}")
    index = matches[0]
    ending = newline if lines[index].endswith(newline) else b""
    lines[index] = b'  docker: "' + value.encode("utf-8") + b'"' + ending
    return b"".join(lines) + tail


def main() -> None:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    note_values = {
        record["version"]: record["note_channel_value"]
        for record in manifest["releases"]
    }

    updates: dict[Path, bytes] = {}

    for version, value in note_values.items():
        path = ROOT / "docs" / "releases" / f"v{version}.md"
        data = path.read_bytes()
        updates[path] = update_docker_frontmatter(
            data,
            value,
            label=f"v{version} frontmatter",
        )

    pending_paragraph = """Channels remain recorded as pending in the repository release ledger. This note
describes the tagged source tree and does not independently assert publication to
every distribution channel."""
    reconstructed_paragraph = """Container actuals were reconstructed after the cut: Docker Hub builder and
runtime tags expose both `linux/amd64` and `linux/arm64`; the corresponding GHCR
tags exist but expose only `linux/arm64` as an application platform. Other
unverified channels remain pending. This note describes the tagged source tree
and does not independently assert publication to every distribution channel."""
    for version in ("0.16.0", "0.17.0"):
        path = ROOT / "docs" / "releases" / f"v{version}.md"
        updates[path] = replace_once(
            updates[path],
            pending_paragraph,
            reconstructed_paragraph,
            label=f"v{version} release-path paragraph",
        )

    note_152 = ROOT / "docs" / "releases" / "v0.15.2.md"
    updates[note_152] = replace_once(
        updates[note_152],
        """- Docker publish completed successfully. Docker Hub manifests were verified.
  GHCR public manifest verification returned `denied` with the available token,
  so that boundary remains explicit in the closeout receipt.""",
        """- Docker Hub builder and runtime tags expose both `linux/amd64` and
  `linux/arm64`. The corresponding GHCR tags exist, but authenticated manifest
  inspection shows only `linux/arm64`; `linux/amd64` is missing from both live
  indexes.""",
        label="v0.15.2 container verification",
    )

    receipt = ROOT / "docs" / "releases" / "0.15.2-closeout-audit.md"
    receipt_data = receipt.read_bytes()
    receipt_data = replace_once(
        receipt_data,
        """| GHCR | workflow-succeeded, public check blocked | Docker workflow succeeded, but `docker manifest inspect ghcr.io/effortlessmetrics/perl-lsp:0.15.2` and `ghcr.io/effortlessmetrics/perl-lsp-perl:0.15.2` returned `denied` with the available token. |""",
        """| GHCR | published but incomplete | Authenticated package and manifest probes show that both `0.15.2` tags exist, but each live index contains only `linux/arm64` plus provenance or attestation metadata; `linux/amd64` is missing. |""",
        label="v0.15.2 GHCR receipt row",
    )
    receipt_data = replace_once(
        receipt_data,
        """Local `docker pull` verification was not run because the Docker Desktop Linux
engine was unavailable on this machine.""",
        """The original closeout could not verify GHCR with the available token. A later
authenticated audit resolved the live state: both `0.15.2` GHCR package versions
exist, but each immutable tag contains only `linux/arm64` plus provenance or
attestation metadata. `linux/amd64` is absent.""",
        label="v0.15.2 GHCR follow-up",
    )
    receipt_data = replace_once(
        receipt_data,
        """`v0.15.2` is the recommended version for `cargo install` users. Docker Hub,
GitHub Release, crates.io, VS Code Marketplace, and Open VSX are verified.
GHCR publish completed in CI, but public/local manifest verification was not
confirmed from this machine.""",
        """`v0.15.2` is the recommended version for `cargo install` users. Docker Hub,
GitHub Release, crates.io, VS Code Marketplace, and Open VSX are verified.
GHCR was published but is incomplete: both builder and runtime tags are
`linux/arm64`-only, with `linux/amd64` missing from the live indexes.""",
        label="v0.15.2 channel state",
    )
    receipt_data = replace_once(
        receipt_data,
        """- Original closeout issue: #9616
- Original merged closeout PR: #9617
- Canonical GitHub Release: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.15.2""",
        """- Original closeout issue: #9616
- Original merged closeout PR: #9617
- Container actuals manifest: `policy/release-container-actuals.json`
- Read-only registry evidence: workflow runs `29192188862` and `29192323459`
- GHCR multi-architecture defect and forward fix: #9971 / PR #9972
- Canonical GitHub Release: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.15.2""",
        label="v0.15.2 provenance",
    )
    updates[receipt] = receipt_data

    history_check = ROOT / "scripts" / "check_release_history.sh"
    history_data = history_check.read_bytes()
    history_data = replace_once(
        history_data,
        """# ── Exit ──────────────────────────────────────────────────────────────────────""",
        """# ── Check 7: Audited container actuals must not regress ───────────────────────

if ! python3 scripts/check_release_container_actuals.py; then
    error "Release-container actuals drift check failed"
fi

if ! python3 scripts/tests/test_release_container_actuals.py; then
    error "Release-container actuals validator tests failed"
fi

# ── Exit ──────────────────────────────────────────────────────────────────────""",
        label="release-history container gate",
    )
    updates[history_check] = history_data

    for path, data in updates.items():
        path.write_bytes(data)
        print(path.relative_to(ROOT))


if __name__ == "__main__":
    main()
