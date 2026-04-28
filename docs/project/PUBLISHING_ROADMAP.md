# Publishing Roadmap

> Machine-executable release-day playbook. Every step is a command or a binary pass/fail check.
> Use with `RELEASE.md` for release mechanics and `RELEASE_CHECKLIST.md` for the preflight gate.
> Replace `NEW_VERSION` and `PREV_VERSION` throughout with the actual semver strings for the cut.

---

## Part 1: Pre-Release Checklist

### 1.1 Set environment

```bash
export CARGO_TARGET_DIR="/tmp/release-preflight-target"
export NEW_VERSION="NEW_VERSION"
export PREV_VERSION="PREV_VERSION"
```

### 1.2 Verify all version strings agree

```bash
# Workspace Cargo.toml
grep '^version' Cargo.toml | head -1
# Expected: version = "NEW_VERSION"

# VSCode extension
node -p "require('./vscode-extension/package.json').version"
# Expected: NEW_VERSION

# features.toml
grep '^version' features.toml | head -1
# Expected: version = "NEW_VERSION"

# Automated check (must print nothing)
cargo xtask check-version-sync
```

Fail if any string mismatches. Fix with:

```bash
gh workflow run version-bump.yml --field version=NEW_VERSION
# Merge the resulting PR before proceeding.
```

### 1.3 Verify CHANGELOG has a dated entry (not just [Unreleased])

```bash
grep "## \[${NEW_VERSION}\]" CHANGELOG.md
# Must match: ## [NEW_VERSION] - YYYY-MM-DD
```

Fail if missing. To promote [Unreleased] to a dated release:

```bash
# Edit CHANGELOG.md: rename ## [Unreleased] to ## [NEW_VERSION] - $(date +%F)
# Add a new empty ## [Unreleased] section above it.
git add CHANGELOG.md
git commit -m "chore: finalize CHANGELOG for v${NEW_VERSION}"
```

CHANGELOG section structure (required):

```markdown
## [NEW_VERSION] - YYYY-MM-DD

### Added
- ...

### Fixed
- ...

### Changed
- ...
```

### 1.4 Run CI gate (required — must be green)

```bash
nix develop -c just ci-gate
# OR without nix:
just ci-gate
```

All checks must pass. Common failures and fixes:

| Failure | Fix |
|---------|-----|
| `cargo fmt --check` fails | `cargo fmt --all` then commit |
| clippy error | Fix lint, commit |
| stale `.snap.new` files | `cargo insta accept && git add crates/perl-lsp-rs/tests/snapshots/ && git commit -m "test: accept snapshots"` |
| test failure | Fix the test, do not skip |

### 1.5 Run release-check gate (superset of ci-gate)

```bash
just release-check
```

This runs: ci-gate + release-build + sbom-verify + version-check + semver-check + no-panic-check + changelog-check + cargo-publish-dry-run.

### 1.6 Verify corpus ratchet

```bash
just cpan-corpus-sweep
# Review output for regressions.

just cpan-corpus-ratchet
# Auto-promotes clean modules to the known-clean manifest.
# Commit any manifest changes:
git add .ci/cpan-corpus-baseline.json
git commit -m "chore: ratchet CPAN corpus manifest for v${NEW_VERSION}"
```

Fail if `just cpan-corpus-check` reports regressions after ratchet.

### 1.7 Verify no existing tag for this version

```bash
git fetch --tags
git tag | grep "v${NEW_VERSION}"
# Must return nothing.
```

### 1.8 Verify all publishable crate versions match

```bash
cargo metadata --format-version=1 --no-deps | python3 -c '
import json, sys
meta = json.load(sys.stdin)
ws = set(meta["workspace_members"])
for pkg in meta["packages"]:
    if pkg["id"] in ws and pkg.get("publish") != []:
        if pkg["version"] != "'${NEW_VERSION}'":
            print(f"MISMATCH: {pkg[\"name\"]}@{pkg[\"version\"]}")
'
# Must print nothing.
```

### 1.9 Cargo publish dry-run

```bash
cargo publish --dry-run -p perl-parser
cargo publish --dry-run -p perl-lsp-rs
cargo publish --dry-run -p perllsp
# All three must succeed.
```

### 1.10 Check GitHub secrets exist

```bash
gh secret list
# Must show: CARGO_REGISTRY_TOKEN, VSCE_PAT, OVSX_PAT, DOCKER_USERNAME, DOCKER_PASSWORD
```

### 1.11 CI green on master HEAD

```bash
gh run list --branch master --limit 3
# Most recent run must show: completed / success
```

---

## Part 2: Smoke Test Protocol

Run these tests manually against the release binary before tagging. Build the binary first:

```bash
cargo build -p perllsp --release
BINARY="./target/release/perllsp"
$BINARY --version    # Must print: perllsp NEW_VERSION
$BINARY --health     # Must print healthy status for all subsystems
```

### Test project 1: Moose (object-oriented Perl)

```bash
# Clone a Moose-heavy CPAN module
git clone https://github.com/moose/Moose /tmp/smoke-moose
$BINARY --stdio &
LSP_PID=$!
```

Open `/tmp/smoke-moose/lib/Moose.pm` in VSCode (or use `perl-lsp` test harness).

| Check | Pass criteria |
|-------|---------------|
| Hover on `extends` | Shows documentation card |
| Hover on `has` attribute | Shows attribute type, accessor names |
| Go-to-definition on `$self->method` | Navigates to method in correct file |
| Completions on `$self->` | Returns method list |
| Diagnostics panel | No phantom errors on clean Moose syntax |

```bash
kill $LSP_PID
```

### Test project 2: DBI (database interface)

```bash
git clone https://github.com/perl5-dbi/dbi /tmp/smoke-dbi
```

Open `/tmp/smoke-dbi/lib/DBI.pm`.

| Check | Pass criteria |
|-------|---------------|
| File opens without crash | No exit / no hang |
| Hover on `sub` definition | Shows sub signature |
| Document symbols outline | Lists all subs and packages |
| Completions on `$dbh->` | Returns method candidates |
| No parse-error diagnostics on clean file | Zero (ERROR) nodes for known-clean DBI files |

### Test project 3: Try::Tiny (exception handling)

```bash
cpanm --look Try::Tiny   # or: git clone https://github.com/p5sagit/Try-Tiny /tmp/smoke-trytiny
```

Open the main `.pm` file.

| Check | Pass criteria |
|-------|---------------|
| Hover on `try`, `catch`, `finally` | Shows documentation |
| Go-to-definition on exported subs | Navigates to definition |
| Rename symbol | Applies change across all uses in file |
| No hang on file open | Opens in <2 seconds |

### Test project 4: Perl core script (CGI-style)

Create `/tmp/smoke-script.pl`:

```perl
#!/usr/bin/perl
use strict;
use warnings;
use POSIX qw(strftime);

my $name = $ARGV[0] // 'world';
my $time = strftime("%Y-%m-%d", localtime);
print "Hello, $name! Today is $time.\n";

sub greet {
    my ($who) = @_;
    return "Hello, $who";
}

my $msg = greet($name);
print "$msg\n";
```

| Check | Pass criteria |
|-------|---------------|
| Hover on `strftime` | Shows builtin/POSIX doc |
| Hover on `$name` | Shows inferred type or variable info |
| Go-to-definition on `greet` | Jumps to sub definition |
| Completion after `$name->` | Gracefully returns empty or scalar methods, no crash |
| Rename `greet` | Renames both definition and call site |
| Diagnostics | `use strict` / `use warnings` respected, no false positives |

### Test project 5: Heredoc and regex-heavy code

Create `/tmp/smoke-heredoc.pl`:

```perl
#!/usr/bin/perl
use strict;
use warnings;

my $text = <<'END';
This is a heredoc block
with multiple lines
END

my $html = <<"HTML";
<html><body>$text</body></html>
HTML

if ($text =~ /(\w+)\s+lines/) {
    print "Matched: $1\n";
}

my $result = $text =~ s/block/section/gr;
```

| Check | Pass criteria |
|-------|---------------|
| File parses without (ERROR) nodes | `$BINARY --parse-check /tmp/smoke-heredoc.pl` exits 0 |
| Hover on heredoc string | No crash |
| Semantic tokens | Regex parts highlighted correctly |
| No hang on open | Opens in <1 second |

### Smoke test pass/fail criteria

**Pass**: All 5 projects open cleanly, no crashes, no hangs >2s, all hover/goto/completion checks return non-empty results.

**Fail**: Any crash, hang, or test project producing (ERROR) parse nodes on known-clean syntax. File an issue immediately, do not release.

---

## Part 3: Release Steps

### 3.1 Trigger the release workflow (automated path)

```bash
gh workflow run release-orchestration.yml \
  --field version=${NEW_VERSION} \
  --field prerelease=false \
  --field skip_crates=false \
  --field skip_extension=false \
  --field skip_docker=false
```

Monitor until complete:

```bash
gh run list --workflow=release-orchestration.yml --limit 5
```

Expected total wall time: 50-90 minutes.

### 3.2 Git tag format

The workflow creates the tag automatically. If manual tag creation is needed:

```bash
git tag -a "v${NEW_VERSION}" -m "Release v${NEW_VERSION}"
git push origin "v${NEW_VERSION}"
```

Tag must be: `v` + semver. Examples: `vNEW_VERSION`, `vNEXT_PATCH_VERSION`. Never: `NEW_VERSION`, `release-NEW_VERSION`.

### 3.3 GitHub release

The workflow creates the release automatically. If verifying or creating manually:

```bash
gh release view "v${NEW_VERSION}"
```

Release must include:

```
perllsp-NEW_VERSION-x86_64-unknown-linux-gnu.tar.gz
perllsp-NEW_VERSION-aarch64-unknown-linux-gnu.tar.gz
perllsp-NEW_VERSION-x86_64-unknown-linux-musl.tar.gz
perllsp-NEW_VERSION-aarch64-unknown-linux-musl.tar.gz
perllsp-NEW_VERSION-x86_64-apple-darwin.tar.gz
perllsp-NEW_VERSION-aarch64-apple-darwin.tar.gz
perllsp-NEW_VERSION-x86_64-pc-windows-msvc.zip
SHA256SUMS
sbom-spdx.json
perl-lsp-rs-NEW_VERSION.vsix
```

Verify binary checksum:

```bash
gh release download "v${NEW_VERSION}" \
  --pattern "perllsp-${NEW_VERSION}-x86_64-unknown-linux-gnu.tar.gz" \
  --pattern SHA256SUMS
sha256sum --check SHA256SUMS --ignore-missing
```

### 3.4 crates.io publishing (automated via workflow)

The `publish-crates.yml` workflow handles topological publish order. If a partial failure requires manual re-publish of a single crate:

```bash
cargo publish -p CRATE_NAME
```

Publish order is in `Cargo.toml` under `[workspace.metadata.publish].allow`. Do not publish out of order — dependencies must exist on crates.io before dependents.

Verify after publish:

```bash
cargo search perl-lsp-rs --limit 1
# Expected: perl-lsp-rs = "NEW_VERSION"
cargo search perllsp --limit 1
# Expected: perllsp = "NEW_VERSION"
```

### 3.5 VSCode extension publishing (automated via workflow)

The `publish-extension.yml` workflow handles both VS Code Marketplace and Open VSX.

Verify:

```
https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs
```

Version shown must be `NEW_VERSION`.

If manual publish is needed:

```bash
cd vscode-extension
npm install
npm run package
vsce publish --pat $VSCE_PAT
ovsx publish perl-lsp-rs-${NEW_VERSION}.vsix --pat $OVSX_PAT
```

### 3.6 Announcement channels

Execute in this order on release day:

**Day 1 — immediate:**

1. Reddit r/perl

   ```
   Title: perl-lsp vNEW_VERSION — Rust-native Perl language server

   Body: [paste from docs/project/LAUNCH_PLAN.md § Reddit r/perl template, update version]
   URL: https://github.com/EffortlessMetrics/perl-lsp
   ```

2. Hacker News Show HN

   ```
   Title: Show HN: perl-lsp – A Rust-native Perl language server (vNEW_VERSION)
   URL: https://github.com/EffortlessMetrics/perl-lsp
   ```

3. Reddit r/rust (Rust implementation angle)

4. X / Twitter thread — tag #Perl #LSP #RustLang

**Day 1 — submit for publication:**

5. Perl Weekly newsletter submission — editors@perlweekly.com

   ```
   Subject: New Perl tooling: perl-lsp vNEW_VERSION (Rust-native LSP)
   Body: 2-3 sentence summary + GitHub link
   ```

6. Lobsters submission

**Week 1:**

7. PerlMonks article — https://www.perlmonks.org/?node=Perl+News

8. This Week in Rust submission — https://this-week-in-rust.org/

9. Rust Users Forum announcement thread — https://users.rust-lang.org/

10. TPRC 2026 lightning talk submission (June 26-28, Greenville SC) — if timeline aligns

**Week 2:**

11. blogs.perl.org post (mirror of blog post #1 from LAUNCH_PLAN.md)

12. Dev.to cross-post

### 3.7 Prepare next development cycle

```bash
gh workflow run version-bump.yml \
  --field bump_type=minor
# OR:
gh workflow run version-bump.yml \
  --field version=NEW_VERSION

# Merge the resulting version-bump PR.
```

This opens a PR that bumps `Cargo.toml`, `vscode-extension/package.json`, `features.toml`, and adds an empty `## [Unreleased]` section to `CHANGELOG.md`.

---

## Part 4: Post-Release

### 4.1 Distribution channel verification (within 2 hours of release)

```bash
# GitHub release artifacts
gh release view "v${NEW_VERSION}"

# crates.io
cargo search perllsp --limit 1

# Docker Hub
docker pull effortlessmetrics/perl-lsp:${NEW_VERSION}
docker run --rm effortlessmetrics/perl-lsp:${NEW_VERSION} perllsp --version

# GHCR
docker pull ghcr.io/effortlessmetrics/perl-lsp:${NEW_VERSION}

# VSCode Marketplace (browser check)
# https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs

# Open VSX (browser check)
# https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs

# Homebrew (automated — verify brew-bump.yml succeeded)
gh run list --workflow=brew-bump.yml --limit 3
```

### 4.2 Install path spot-check (within 24 hours)

```bash
# From crates.io
cargo install perllsp --version ${NEW_VERSION}
perllsp --version     # Must print: perllsp NEW_VERSION
perllsp --health      # Must show healthy

# From install script
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash
perllsp --version
```

### 4.3 Monitoring plan (Week 1)

Check daily:

```bash
# Open issues (bug reports = users)
gh issue list --state open --label bug --limit 20

# New crash/hang reports (high priority)
gh issue list --state open --label "P0" --limit 10
gh issue list --state open --label "crash" --limit 10

# Download count (crates.io)
curl -s https://crates.io/api/v1/crates/perllsp | python3 -c \
  'import json,sys; d=json.load(sys.stdin)["crate"]; print(f"downloads: {d[\"downloads\"]}, recent: {d[\"recent_downloads\"]}")'
```

Triage SLA:

| Severity | Response time |
|----------|---------------|
| Crash or hang | Same day — file P0 issue, assign to next build cycle |
| Parse error on valid Perl | 48 hours — file issue, label `parser-corpus` |
| Feature gap | 1 week — triage to the next release milestone |
| Enhancement | Triage to roadmap, no SLA |

### 4.4 Current release issue triage (Week 1 — after release)

```bash
# List all open issues
gh issue list --state open --limit 200

# Label new issues from post-release feedback
# Use these labels: bug, parser-corpus, lsp-feature, enhancement, good-first-issue
gh issue edit ISSUE_NUMBER --add-label "parser-corpus"
gh issue edit ISSUE_NUMBER --milestone "NEW_VERSION"
```

Milestone priorities for NEW_VERSION (from ROADMAP.md):

1. Diagnostic hardening: `strict`, `warnings`, dead-code signals
2. CPAN corpus clean-parse rate to 95%+
3. Moo/Moose/Class::Accessor semantic coverage hardening
4. Cross-file `use parent` / `use base` inheritance resolution
5. Auto-import completions

```bash
# Seed the builder queue
gh issue list --state open --label "builder-ready" --limit 20
```

### 4.5 Corpus ratchet after post-release fixes

After merging parser fix PRs from user-reported issues:

```bash
export CARGO_TARGET_DIR="/tmp/post-release-target"
just cpan-corpus-sweep
just cpan-corpus-ratchet
git add .ci/cpan-corpus-baseline.json
git commit -m "chore: ratchet corpus post-v${NEW_VERSION} fixes"
```

### 4.6 Success metrics — Week 1 targets

| Metric | Target | Check |
|--------|--------|-------|
| crates.io downloads | 100+ | `cargo search perllsp` |
| VSCode installs | 200+ | Marketplace dashboard |
| GitHub stars | 50+ | GitHub repo page |
| Crash reports | 0 critical | `gh issue list --label crash` |
| P0 issues | 0 open | `gh issue list --label P0` |

---

## Rollback

Situations and commands:

```bash
# Delete GitHub release (keeps tag, safe to redo)
gh release delete "v${NEW_VERSION}" --yes

# Delete tag locally and remotely
git tag -d "v${NEW_VERSION}"
git push origin ":refs/tags/v${NEW_VERSION}"

# Yank a specific crate from crates.io (irreversible — cannot delete, only yank)
cargo yank --version ${NEW_VERSION} perllsp

# Yank all published crates at once
VERSION=${NEW_VERSION}
cargo metadata --format-version=1 --no-deps | python3 -c '
import json, sys
meta = json.load(sys.stdin)
for name in meta.get("metadata", {}).get("publish", {}).get("allow", []):
    print(name)
' | while read crate; do
  cargo yank --version "$VERSION" "$crate" || true
done

# VSCode Marketplace: cannot delete — publish patch release NEW_VERSION.1 to supersede
# Open VSX: same — publish patch release

# Re-run release after partial failure (skip stages already completed)
gh workflow run release-orchestration.yml \
  --field version=${NEW_VERSION} \
  --field skip_crates=true \
  --field skip_extension=false \
  --field skip_docker=false
```

---

## Files That Need Version Bumps (summary)

| File | Field | Tool |
|------|-------|------|
| `Cargo.toml` | `[workspace.package].version` | `gh workflow run version-bump.yml` |
| `Cargo.toml` | All `version = "X.Y.Z"` in `[workspace.dependencies]` | Same workflow |
| `vscode-extension/package.json` | `"version"` | Same workflow |
| `features.toml` | `[meta].version` | Same workflow |
| `CHANGELOG.md` | Promote `[Unreleased]` to `[NEW_VERSION] - DATE` | Manual edit |
| `docs/project/status/index.md` | Release posture narrative | Manual edit after ship |

`cargo xtask check-version-sync` validates all of the above except `CHANGELOG.md`.

---

*This document is version-agnostic — substitute NEW_VERSION and PREV_VERSION at use time.*
*Authoritative mechanics: `RELEASE.md`. Authoritative feature catalog: `features.toml`. Authoritative status: `docs/project/status/`.*
