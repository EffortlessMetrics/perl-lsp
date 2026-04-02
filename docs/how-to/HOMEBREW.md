# Homebrew Installation and Formula Maintenance

This document covers installing `perllsp` via Homebrew and maintaining the
formula in the release pipeline.

## Installing via Homebrew

```bash
brew install perl-lsp
```

After installation, verify the binary:

```bash
perllsp --version
perllsp --health
```

Platform support:

| Platform | Architecture |
| --- | --- |
| macOS | Intel (x86_64) |
| macOS | Apple Silicon (aarch64) |
| Linux (Linuxbrew) | x86_64 |
| Linux (Linuxbrew) | aarch64 |

## Shell Completions

The formula does not auto-install shell completions. Install them manually:

```bash
# bash
perllsp --completion bash > "$(brew --prefix)/etc/bash_completion.d/_perllsp"

# zsh
perllsp --completion zsh > "$(brew --prefix)/share/zsh/site-functions/_perllsp"

# fish
perllsp --completion fish > "$(brew --prefix)/share/fish/vendor_completions.d/perllsp.fish"
```

## Formula Auto-Bump Workflow

The workflow at `.github/workflows/brew-bump.yml` runs automatically on each
`release.published` event and opens a PR to `Homebrew/homebrew-core` with the
updated version and SHA256 checksums.

It can also be triggered manually:

```
Actions → Homebrew Auto-Bump → Run workflow → enter the release tag (e.g. v0.12.0)
```

### What the workflow does

1. Resolves the release tag (from the event or the manual input).
2. Waits up to 10 minutes for release assets to appear on the GitHub release.
3. Downloads all four platform tarballs: `perllsp-{version}-{target}.tar.gz`.
4. Computes SHA256 for each tarball.
5. Patches `Formula/perl-lsp.rb` with the new version and checksums.
6. Validates that no `__RELEASE_VERSION__` or `__SHA256_` placeholders remain.
7. Opens a PR to `Homebrew/homebrew-core`.

### Release artifact naming

The workflow and formula expect tarballs named with the `perllsp-` prefix, which
matches what `release.yml` produces:

```
perllsp-{version}-x86_64-apple-darwin.tar.gz
perllsp-{version}-aarch64-apple-darwin.tar.gz
perllsp-{version}-x86_64-unknown-linux-gnu.tar.gz
perllsp-{version}-aarch64-unknown-linux-gnu.tar.gz
```

If the release packaging changes this naming, update the `--pattern` argument in
the "Download and compute SHA256" step of `brew-bump.yml` to match.

### Formula files

Two formula files are kept in sync:

| File | Purpose |
| --- | --- |
| `Formula/perl-lsp.rb` | Used by `brew-bump.yml` — patched on each release |
| `distribution/homebrew/perl-lsp.rb` | Distribution reference copy |

Both must use the `perllsp-` prefix in URLs and the `Dir.glob("perllsp-*")`
extraction glob, matching the actual release tarball names.

## Troubleshooting

**Workflow downloads no assets** — the release tarballs may not yet be uploaded
when the workflow starts. The wait loop retries for up to 10 minutes. If assets
are still missing after that, trigger the workflow manually once they appear.

**SHA256 mismatch** — re-run `brew-bump.yml` manually against the correct tag.
The workflow recomputes checksums from the actual uploaded assets each run.

**`brew install` fails with "no bottle"** — the Homebrew PR to homebrew-core
may not have merged yet. Wait for the PR to land in homebrew-core, then
`brew update && brew install perl-lsp`.
