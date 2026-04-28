# Install Surface Archaeology
## How March 2026 Turned Install And First-Run Into Part Of The Launch Story

This note tracks the install and first-run surface as it existed in March 2026.
The important historical point is not just that the repo had installers. It is
that install, health checks, binary discovery, and editor startup behavior had
become part of the public-alpha trust story.

By this point, "can someone get the binary, verify it, and attach an editor
without guesswork?" was no longer a side concern. It was launch material.

All claims below were checked against tracked repo files on `2026-03-19`.

---

## 1. The Install Surface Was Layered, Not Singular

The repository exposed multiple installation paths in parallel:

- release archive installers via [`install.sh`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/install.sh) and [`install.ps1`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/install.ps1)
- manual/editor-facing setup via [`docs/how-to/INSTALLATION.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/how-to/INSTALLATION.md)
- editor-specific deep dives via [`docs/how-to/EDITOR_SETUP.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/how-to/EDITOR_SETUP.md), [`docs/EDITORS/NEOVIM_SETUP.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/EDITORS/NEOVIM_SETUP.md), and [`docs/EDITORS/HELIX_SETUP.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/EDITORS/HELIX_SETUP.md)
- VS Code managed-binary discovery in [`vscode-extension/src/extension.ts`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/vscode-extension/src/extension.ts) and [`vscode-extension/src/downloader.ts`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/vscode-extension/src/downloader.ts)

That matters because the launch surface is not just `cargo install`. It is a
stack:

1. obtain a binary
2. verify the binary responds correctly
3. resolve which binary the editor should run
4. confirm first-run behavior is legible enough to trust

---

## 2. CLI Verification Was Already A First-Class Contract

The CLI contract in [`crates/perl-lsp-launcher/src/lib.rs`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/crates/perl-lsp-launcher/src/lib.rs)
already treats install verification as part of the product surface:

- `--health` prints `ok <version>`
- `--info` prints version, git tag, executable path, feature profile, and coverage summary
- `--version` prints version and git tag
- `--feature-profile` exposes the runtime capability profile directly in the CLI

The user-facing docs mirror that. [`docs/how-to/INSTALLATION.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/how-to/INSTALLATION.md)
explicitly tells users to run:

- `perl-lsp --health`
- `perl-lsp --info`

and documents the expected behavior:

- `--health` prints `ok <version>`
- `--info` prints build and feature-profile details

That is historically useful because it shows install trust moving away from
"binary exists on PATH" toward "binary proves what it is."

---

## 3. First-Run Was Quiet By Design, Which Created A UX Tension

The runtime path is also revealing.

[`crates/perl-lsp-rs/src/main.rs`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/crates/perl-lsp-rs/src/main.rs)
uses the launcher crate to decide between `--health`, `--info`, `--check`,
and server startup. [`crates/perl-lsp-launcher/src/lib.rs`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/crates/perl-lsp-launcher/src/lib.rs)
shows that normal server startup only emits explicit startup logs when logging
is enabled.

So the March 2026 install surface had an important tension:

- verification commands were explicit and strong
- normal `--stdio` startup was intentionally quiet unless logging was enabled

That is good protocol behavior but a weaker human first-run signal. The repo
compensates by pushing verification into `--health`, `--info`, and editor-side
health checks instead of relying on visible startup chatter.

---

## 4. VS Code Discovery Order Encoded A Trust Policy

[`vscode-extension/src/extension.ts`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/vscode-extension/src/extension.ts)
implements a four-step discovery order:

1. user-configured `perl-lsp.serverPath`
2. bundled extension binary
3. binary found on `PATH`
4. auto-download if enabled

After discovery, the extension immediately runs `--health` and refuses to start
the language client if the binary does not return output beginning with `ok`.

That is not just convenience logic. It is a launch-time trust policy:

- explicit user choice wins
- known bundled binary is preferred next
- ambient system binary is acceptable
- managed download is fallback, not silent magic
- every path still has to pass the same health probe

The extension therefore treats binary discovery and binary validation as
separate concerns.

---

## 5. Managed Download Included Provenance Hooks

The installer scripts and extension downloader also show that the repo was
trying to make installation auditable rather than purely convenient.

[`install.sh`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/install.sh)
and [`install.ps1`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/install.ps1)
both:

- resolve a release tag
- download a platform-specific archive
- fetch `SHA256SUMS`
- verify the checksum when possible
- install the binary into a user-local path
- verify the install via `--version`

The VS Code downloader in
[`vscode-extension/src/downloader.ts`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/vscode-extension/src/downloader.ts)
extends the same pattern with managed release discovery, optional internal
download base URLs, checksum verification, and HTTPS-only protections for
remote downloads.

That makes the install surface historically relevant to the repo's broader
"trusted change" theme. Even installation is treated as something that should
have provenance and verification, not just happy-path ergonomics.

---

## 6. Why This Became Launch Material

The March 2026 launch story was not only about parser coverage and swarm
history. It was also about whether the public-alpha experience looked
intentional:

- install docs exist and cover multiple editors
- the binary can prove it is healthy
- the extension can discover or fetch the binary predictably
- downloads can be checksum-verified
- feature profile and build posture are surfaced in `--info`

That is why install UX belongs in the archaeology. By March 2026, the repo was
treating installation, first-run verification, and editor attach as part of the
same public-facing trust boundary as CI gates and receipts.

---

## Evidence Pointers

- [`install.sh`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/install.sh)
- [`install.ps1`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/install.ps1)
- [`docs/how-to/INSTALLATION.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/how-to/INSTALLATION.md)
- [`docs/how-to/EDITOR_SETUP.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/how-to/EDITOR_SETUP.md)
- [`docs/EDITORS/NEOVIM_SETUP.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/EDITORS/NEOVIM_SETUP.md)
- [`docs/EDITORS/HELIX_SETUP.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/EDITORS/HELIX_SETUP.md)
- [`crates/perl-lsp-launcher/src/lib.rs`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/crates/perl-lsp-launcher/src/lib.rs)
- [`crates/perl-lsp-rs/src/main.rs`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/crates/perl-lsp-rs/src/main.rs)
- [`vscode-extension/src/extension.ts`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/vscode-extension/src/extension.ts)
- [`vscode-extension/src/downloader.ts`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/vscode-extension/src/downloader.ts)
