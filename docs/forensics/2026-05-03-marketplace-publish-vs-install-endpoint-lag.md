# 2026-05-03 — Marketplace + Open VSX Gallery vs. Install Endpoint Propagation Lag

**Lens**: VS Code Marketplace and Open VSX index new extension versions in their gallery API immediately, but the install endpoints (what `code --install-extension` actually hits) lag by minutes. Smokes that run inside the publish workflow can fail on propagation; the same smokes dispatched 5 minutes later succeed.

## What we hit

Inside `Publish VSCode Extension` workflow run `25274154490`, two jobs failed:

- `Published Marketplace Smoke` — windows + macos + ubuntu all reported "Extension not found" after 12 retries (~4 minutes total).
- `Published Open VSX Smoke` — same shape.

Smoke step:

```
$ code --install-extension EffortlessMetrics.perl-lsp-rs@0.13.3 ...
Installing extensions...
Extension 'effortlessmetrics.perl-lsp-rs@0.13.3' not found.
Make sure you use the full extension ID, including the publisher
Failed Installing Extensions: effortlessmetrics.perl-lsp-rs
```

But at the same moment, the gallery API showed 0.13.3 already indexed:

```bash
$ curl -s -X POST "https://marketplace.visualstudio.com/_apis/public/gallery/extensionquery" \
    -H "Content-Type: application/json" \
    -d '{"filters":[{"criteria":[{"filterType":7,"value":"EffortlessMetrics.perl-lsp-rs"}]}],"flags":914}' \
  | jq '.results[0].extensions[0].versions[0]'
{
  "version": "0.13.3",
  "lastUpdated": "2026-05-03T08:31:16.247Z",
  "flags": "validated"
}
```

Open VSX showed the same — 0.13.3 was in the gallery API, but the install endpoint wasn't serving it yet.

## What's actually happening

VS Code Marketplace and Open VSX have separate paths for:

1. **Gallery query** (`/_apis/public/gallery/extensionquery`) — the search/browse API. Updated near-instantly when a publish completes.
2. **Install** (what `code --install-extension` and the editor's UI use) — separate CDN endpoints with their own caches. Propagation lag is observably 5-15 minutes for new versions.

Marketplace and Open VSX both have this two-tier structure. The lag is independent of the publishing API's `success` response — the publish *succeeded* in both cases (verified by the gallery API showing the version) but the install endpoints hadn't caught up.

## Why this looks like the install-reliability bug

The error message `Failed Installing Extensions: effortlessmetrics.perl-lsp-rs` is identical in shape to the Windows install failures we were trying to fix with v0.13.3. Initial triage briefly conflated the two.

Disambiguating signal:

- **Real install bug**: fails consistently on a fresh test, even after 30+ minutes.
- **Propagation lag**: fails at first; a re-run 5-10 minutes later succeeds without any code change.

Recovery for v0.13.3 was to dispatch the smoke workflow separately *after* publish:

```bash
$ gh workflow run vscode-published-extension-smoke.yml --field version=0.13.3 --field source=marketplace
$ gh workflow run vscode-published-extension-smoke.yml --field version=0.13.3 --field source=open-vsx
```

Both smokes went green on all 3 OSes. Workflow runs `25274789275` (Marketplace) and `25274789657` (Open VSX).

## Why the in-publish smoke is fragile

`Publish VSCode Extension` runs the smoke immediately after the marketplace publish step. The smoke retries 12 times with 20-second intervals, ~4 minutes total. That's apparently not enough margin for cold propagation on either Marketplace or Open VSX.

Two options to fix:

**Option A: don't run the smoke inside the publish workflow.** Have a separate workflow that runs on `release` events with a delay (e.g., 10-minute initial sleep) or runs on a schedule shortly after. This is what the standalone `vscode-published-extension-smoke.yml` does — and dispatching it manually ~5 minutes after publish reliably works.

**Option B: extend the smoke's retry budget significantly.** Push to 30 retries × 20 seconds = 10 minutes. Still gambles on propagation timing.

Option A is structurally cleaner. The in-publish smoke can be removed entirely if the standalone smoke is reliable.

## Detection signal

If `Published Marketplace Smoke` or `Published Open VSX Smoke` fails inside `Publish VSCode Extension`:

1. Verify gallery API shows the version (it almost certainly does).
2. Wait 5-10 minutes.
3. Dispatch `vscode-published-extension-smoke.yml` separately.
4. If *that* succeeds, the publish was correct; the in-workflow smoke was just early.
5. If *that* fails too, escalate — it's a real install issue.

## Lesson

Publishing is asynchronous from a CDN/install perspective even when the publish API call returns `success`. Workflows that gate on "the publish worked" cannot also test "the install works" without giving propagation time.

## Related

- Forensics: `2026-05-03-v0.13.3-windows-install-dual-lock.md` (the real install bug — disambiguated from this propagation lag)
- Articles: `../articles/RELEASES_FAIL_AT_SEAMS.md` (this is a publish-vs-install seam)
- Reference: `../reference/RELEASE_PROOF_PROTOCOL.md` (the protocol now says "dispatch published smokes *after* publish, not as part of it")
- Reference: `../reference/FAILURE_CLASSIFICATION.md` (this is "flake" class — repeatable but transient)
