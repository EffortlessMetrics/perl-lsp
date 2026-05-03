# Release Proof Protocol

What "this release shipped correctly" means as evidence, channel by channel. Distinguishes executor-verifiable proof (CI runs, API queries, checksum verification) from human-verifiable proof (manual UX smoke). Captures the seam-fragilities that have actually surfaced in this repo's release pipeline.

> Companion docs: [FAILURE_CLASSIFICATION.md](FAILURE_CLASSIFICATION.md), [AGENT_HANDOFF_PROTOCOL.md](AGENT_HANDOFF_PROTOCOL.md), [../articles/RELEASES_FAIL_AT_SEAMS.md](../articles/RELEASES_FAIL_AT_SEAMS.md), [../articles/EVIDENCE_DURABILITY_TIERS.md](../articles/EVIDENCE_DURABILITY_TIERS.md).

## The four-tier proof stack

A release is proven only when each tier shows green:

| Tier | What it proves | Where it runs | Substrate |
|---|---|---|---|
| 1. Hosted CI | Basic compatibility on managed runners | GitHub Actions | Cloud (with AV exclusions, pre-warmed caches) |
| 2. Local repro | Realism — what real user environments look like | A real Windows / macOS / Linux machine | User-grade environment (Defender on, no exclusions) |
| 3. Published smoke | Actual distribution channel works end-to-end | Hosted CI installing the *published* artifact | Bridges hosted CI and real distribution |
| 4. Manual UX smoke | The user-facing flow actually works | A human, in the actual editor / shell | Human-verifiable only |

Hosted CI greenness is necessary but **not sufficient**. Real Windows machines reproduce failure modes hosted runners mask (most notably Defender first-time-scan source-side `EBUSY`). See [../articles/CI_VS_REAL_USER_PARITY.md](../articles/CI_VS_REAL_USER_PARITY.md).

## Release proof packet

Used by executor to report release-cycle status:

```markdown
RELEASE PROOF PACKET

Version:                       0.13.3
Tag:                           v0.13.3
Tag commit:                    06fc1443
Release URL:                   https://github.com/<owner>/<repo>/releases/tag/v0.13.3
Published at:                  2026-05-03T08:37:57Z
isPrerelease:                  false

Asset count:                   10
Asset list:                    [perllsp-0.13.3-x86_64-pc-windows-msvc.zip, ..., SHA256SUMS, sbom-spdx.json, perl-lsp-rs-0.13.3.vsix]
SHA256SUMS verified locally:   pass / fail
Chooser body present:          pass / fail
gnu / musl / windows-msvc rows: pass / fail

crates.io:
  - perllsp 0.13.3:            published, not yanked
  - perl-lsp-rs 0.13.3:        published, not yanked

Docker Hub:
  - 0.13.3-perl (runtime):     published (multi-arch)
  - 0.13.3 (builder/slim):     <state>

GHCR:                          <state>

Marketplace:                   indexed / validated; install endpoint verified
Open VSX:                      indexed / validated; install endpoint verified

Marketplace published smoke:   windows-latest / macos-latest / ubuntu-latest — all SUCCESS
Open VSX published smoke:      windows-latest / macos-latest / ubuntu-latest — all SUCCESS

Homebrew tap PR:               <url> — merged
Public tap smoke:              formula tests pass (macos-latest / ubuntu-latest)

Manual Windows Stable smoke:   <yours>
Manual Windows Insiders smoke: <yours>

Known skips / non-blocking:
  - Docker builder linux/arm64: still in QEMU build at receipt time

Receipt path:                  target/receipts/release-install-surface-v0.13.3.md
Remaining blockers:            none / list
```

The receipt is the **local forensic artifact**; the packet is the **shareable summary**.

## Channel verification commands

### GitHub release

```bash
gh release view v0.13.3 --repo <owner>/<repo> --json tagName,name,isPrerelease,assets,body \
  --jq '{tagName, name, isPrerelease, assets: [.assets[].name], hasChooser: (.body | contains("Which file should I download?")), hasGnu: (.body | contains("x86_64-unknown-linux-gnu")), hasMusl: (.body | contains("x86_64-unknown-linux-musl")), hasWindows: (.body | contains("x86_64-pc-windows-msvc"))}'
```

### Checksum verification

```bash
mkdir -p /tmp/verify-v0.13.3 && cd /tmp/verify-v0.13.3
gh release download v0.13.3 --repo <owner>/<repo> --pattern 'SHA256SUMS' --pattern 'perllsp-0.13.3-*' --clobber --dir .
sha256sum -c SHA256SUMS
```

All seven binary archives must report `OK`.

### crates.io

```bash
curl -s "https://crates.io/api/v1/crates/perllsp/0.13.3"   | jq '.version | {num, yanked}'
curl -s "https://crates.io/api/v1/crates/perl-lsp-rs/0.13.3" | jq '.version | {num, yanked}'
```

Both must show `num: "0.13.3"` and `yanked: false`.

### Docker

```bash
curl -s "https://hub.docker.com/v2/repositories/effortlessmetrics/perl-lsp/tags/0.13.3-perl" | jq '.name, .tag_status'
curl -s "https://hub.docker.com/v2/repositories/effortlessmetrics/perl-lsp/tags/0.13.3"      | jq '.name, .tag_status'
```

The `-perl` runtime tag is what most users pull. The plain tag (builder image) may lag behind because of arm64 QEMU build time — that's non-blocking, not a failure.

### Marketplace + Open VSX

```bash
# Marketplace gallery query
curl -s -X POST "https://marketplace.visualstudio.com/_apis/public/gallery/extensionquery" \
  -H "Accept: application/json;api-version=3.0-preview.1" \
  -H "Content-Type: application/json" \
  -d '{"filters":[{"criteria":[{"filterType":7,"value":"EffortlessMetrics.perl-lsp-rs"}]}],"flags":914}'   | jq '.results[0].extensions[0].versions[0]'

# Open VSX
curl -s "https://open-vsx.org/api/EffortlessMetrics/perl-lsp-rs/0.13.3" | jq '.version, .files.download'
```

Both must show 0.13.3 with `flags: "validated"` (Marketplace) or a download URL (Open VSX).

**Important**: gallery indexing is fast; install-endpoint propagation is slow. See `docs/forensics/2026-05-03-marketplace-publish-vs-install-endpoint-lag.md`.

### Published-extension smokes

These are the canonical install-reliability proof. Dispatch *after* publish, not as part of it:

```bash
gh workflow run vscode-published-extension-smoke.yml --repo <owner>/<repo> \
  --field version=0.13.3 --field source=marketplace
gh workflow run vscode-published-extension-smoke.yml --repo <owner>/<repo> \
  --field version=0.13.3 --field source=open-vsx
```

Required: `windows-latest` + `macos-latest` + `ubuntu-latest` all SUCCESS for both sources.

The smokes assert reinstall-twice with the binary held by a spawned process during the second pass — exercising the destination-side lock condition that's the canonical Windows install regression. See `docs/forensics/2026-05-03-windows-defender-source-side-ebusy.md` for the source-side counterpart.

## Manual proof (CTO / human only)

The executor cannot produce these. They go in the proof packet under "Manual Windows ... smoke: <yours>".

**VS Code Stable + Insiders, both clean profile and existing-0.13.x profile:**

```
Install or update EffortlessMetrics.perl-lsp-rs
Open a Perl file
Perl: Reinstall Server Binary
Perl: Run Health Check
Perl: Reinstall Server Binary  (second pass — the regression-locked one)
Perl: Run Health Check          (second pass)
Perl: Show Output
```

Expected:

```
Found matching asset: perllsp-0.13.3-x86_64-pc-windows-msvc.zip
Checksum verified successfully
Binary installed under <versioned managed dir>
perllsp --health passes
no EBUSY / EPERM / EACCES
no "Failed to obtain a Perl LSP binary"
```

The "existing-0.13.x profile" run exercises the legacy migration path (flat layout → versioned). The unit-level migration test in `vscode-extension/src/test/downloader.test.ts` covers the same code path; the manual run proves it works in the actual VS Code lifecycle.

## Pre-release checks (run before dispatching orchestration)

These run on `origin/master` immediately before release-orchestration dispatch:

```bash
git fetch origin master
git switch --detach origin/master

cargo xtask check-version-sync           # all version sites agree
cargo xtask install-surface-check        # installer files / configs match
bash scripts/check_release_history.sh    # RELEASE_HISTORY.md current

cargo xtask release-notes --tag "v$VERSION" --output "/tmp/v$VERSION-body.md"
grep -q 'Which file should I download?'          "/tmp/v$VERSION-body.md"
grep -q 'x86_64-unknown-linux-gnu'               "/tmp/v$VERSION-body.md"
grep -q 'x86_64-unknown-linux-musl'              "/tmp/v$VERSION-body.md"
grep -q 'x86_64-pc-windows-msvc'                 "/tmp/v$VERSION-body.md"

git diff --check
```

If `check_release_history.sh` fails with "Missing RELEASE_HISTORY.md entry for $VERSION", the release-prep PR did not include the ledger row. Land the row before re-dispatching. See `docs/forensics/2026-05-03-release-history-ledger-drift.md`.

## Known seam-fragilities

These have actually surfaced in this pipeline. Each has its own forensic note.

| Seam | Failure | Note |
|---|---|---|
| `Validate Release` vs. master CI on squash-merge | `pending` combined-status → validation fails | `docs/forensics/2026-05-03-validate-release-squash-timing-race.md` |
| `Attach VSIX to GitHub Release` vs. release creation | Race; "release not available yet" | `docs/forensics/2026-05-03-release-orchestration-attach-vsix-race.md` |
| Smokes in publish workflow vs. install-endpoint propagation | "Extension not found" within minutes of publish | `docs/forensics/2026-05-03-marketplace-publish-vs-install-endpoint-lag.md` |
| Release-prep version bump vs. RELEASE_HISTORY.md row | Master CI breaks for *all* downstream PRs | `docs/forensics/2026-05-03-release-history-ledger-drift.md` |

The follow-up work to harden these seams is tracked in their respective forensic notes.

## Recovery patterns

When a publish step fails after the release tag exists, **do not re-run the orchestration blindly** — that risks duplicate publishes. Instead:

| Failed step | Recovery |
|---|---|
| Attach VSIX | Workflow stored VSIX as artifact; `gh release upload` manually with `--clobber` |
| In-orchestration smoke | Wait for propagation; dispatch the smoke workflow separately |
| Validate Release | Wait for master CI on the merged commit to report; re-dispatch orchestration |
| RELEASE_HISTORY drift | File a small fix PR adding the row; land before any other PRs |

The recovery pattern depends on artifacts existing at every join-point. See [../articles/EVIDENCE_DURABILITY_TIERS.md](../articles/EVIDENCE_DURABILITY_TIERS.md).

## Provenance

Codified during the v0.13.3 install-reliability release closeout (2026-05-03). The receipt at `target/receipts/release-install-surface-v0.13.3.md` is the worked example.
