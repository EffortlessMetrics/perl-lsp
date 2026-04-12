# VSCode Marketplace + Open VSX Punch List — v0.13.0 Launch

Reference: `vscode-extension/package.json`, `vscode-extension/README.md`,
`vscode-extension/CHANGELOG.md`, `vscode-extension/.vscodeignore`.

Companion docs: `vscode-extension/PUBLISHING.md`, `docs/project/RELEASE_CHECKLIST.md`,
`docs/project/PUBLISHING_ROADMAP.md`.

---

## 1. Current State Audit

### Metadata

| Field | Status | Notes |
|-------|--------|-------|
| `displayName` | ✓ | "Perl Language Server (perl-lsp)" — clear, searchable |
| `description` | ✓ | Mentions features, Rust, zero deps. Could punch harder for v0.13.0 (see Section 3). |
| `version` | ✓ | 0.12.2 — matches workspace `Cargo.toml`. Bump to 0.13.0 is a separate step. |
| `publisher` | ✓ | `EffortlessMetrics` |
| `engines.vscode` | ✓ | `^1.88.0` — reasonable minimum (released April 2024) |
| `categories` | ✓ | Programming Languages, Linters, Formatters, Debuggers, Testing — all five are accurate and valid |
| `keywords` | ⚠ | 10 keywords present. **Marketplace enforces a 5-keyword limit** for search ranking. You have double the allowed count; extras are silently dropped. Trim to the 5 highest-value terms (recommendation in Section 3). |
| `homepage` | ⚠ | Points to GitHub `#readme`, not a standalone project page. Acceptable for now; upgrade to a dedicated landing page before public GA if one exists. |
| `repository` | ✓ | GitHub URL set |
| `bugs` | ✓ | GitHub issues URL set |
| `license` | ⚠ | Extension says `MIT`. Workspace Cargo crates say `MIT OR Apache-2.0`. Inconsistency is fine (extension JS is MIT-only), but be intentional about it. There is no root `LICENSE` file at the repo root — only `vscode-extension/LICENSE`. |
| `preview: true` | ✓ | Correct for public alpha |

### Visual Assets

| Asset | Status | Notes |
|-------|--------|-------|
| `icon.png` (128×128) | ⚠ | File exists at `vscode-extension/icon.png`. Dimensions are **256×256**, not 128×128. Marketplace accepts up to 256×256 PNG — this is fine technically, but the spec says 128×128. Verify rendering at small sizes. |
| `galleryBanner` color + theme | ✓ | `#1e3a8a` dark blue, `dark` theme — set and will render on the marketplace listing header |
| Marketplace screenshots in README | ✗ | **No screenshots or GIFs embedded in README.md.** The README has zero `<img>` tags and no image links (beyond badges). Marketplace listing pages that lack screenshots convert significantly worse. |
| Walkthrough SVGs (in-extension) | ✓ | All 7 SVGs referenced in `package.json` walkthroughs exist under `media/walkthrough/`. These show inside VS Code's walkthrough panel, not on the marketplace page. |
| Walkthrough GIFs (in-extension) | ✗ | SVG storyboards exist; the **actual recorded GIFs do not exist yet**. See `media/walkthrough/README.md` for the 3 planned GIFs: `install-health.gif`, `find-references.gif`, `extract-variable.gif`. The render helper script exists at `scripts/marketing/render-walkthrough-gif.py`. |
| Demo workspace for recording | ✓ | `demo_workspace/main.pl`, `lib/Utils.pm`, `lib/Database.pm` — all present |

### Listing Content (README.md)

| Section | Status | Notes |
|---------|--------|-------|
| Quick-start / installation | ✓ | Clear install commands, platform table, manual install instructions |
| Feature list | ✓ | Comprehensive, organized by category, at top of file |
| Requirements (Perl interpreter) | ✓ | Covered in Configuration and Troubleshooting sections |
| Optional deps (perltidy, perlcritic) | ✓ | perltidy mentioned; perlcritic referenced in `.perl-lsp.toml` description but not in its own explicit "Requirements" heading |
| Extension settings table | ✓ | Present and complete |
| Keyboard shortcuts | ✓ | Table present. Note: `Shift+Alt+F` for Format Document appears in the table but is NOT registered in `contributes.keybindings` — it comes from VS Code's built-in format handler. Worth a footnote to avoid confusion. |
| Known issues | ✗ | No "Known Issues" section. Marketplace listing guidelines recommend one (even if it just says "see GitHub issues"). |
| Release notes / CHANGELOG link | ✓ | Link present in Resources section |
| `preview` alpha disclaimer | ✓ | Present at top of README (references 0.12.2; update to 0.13.0 before publishing) |
| Walkthrough storyboard section | ⚠ | The README currently exposes internal storyboard links under "Walkthrough Previews." This section is developer-facing boilerplate that should be **removed or moved** before the public v0.13.0 publish — it tells users these are placeholder SVGs, not real recordings. |

### CHANGELOG.md

| Item | Status | Notes |
|------|--------|-------|
| `[0.12.2]` entry | ✓ | Present, dated, complete |
| `[0.13.0]` entry | ✗ | Not yet written — TBD as part of v0.13.0 release preparation |
| Emoji use in older entries | ⚠ | Entries 0.6.0–0.9.0 use emoji bullets; newer entries don't. Not a blocking issue, but inconsistent for a polished listing. |

### Contributions (what ships in the .vsix)

| Contribution | Status | Notes |
|--------------|--------|-------|
| `contributes.languages` | ✓ | `.pl`, `.pm`, `.pod`, `.t`, `.psgi`; shebang detection; language config |
| `contributes.grammars` | ✓ | TextMate grammar registered |
| `contributes.configuration` | ✓ | 4 groups, well-documented |
| `contributes.commands` | ✓ | 23 commands with icons and categories |
| `contributes.keybindings` | ✓ | 5 default keybindings |
| `contributes.menus` | ✓ | commandPalette, editor/title, editor/context |
| `contributes.snippets` | ✓ | `perl.json` and `launch.json` present |
| `contributes.debuggers` | ✓ | DAP registered with launch + attach configs |
| `contributes.walkthroughs` | ✓ | 7-step walkthrough with SVG media (GIF replacements pending) |
| `contributes.taskDefinitions` | ✓ | Perl task type defined |

### Quality / Publish

| Item | Status | Notes |
|------|--------|-------|
| `.vscodeignore` present | ✓ | Excludes src, tests, maps, scripts, lock files, dev docs |
| `node_modules` excluded | ✓ | vsce excludes automatically; not needed in `.vscodeignore` |
| `media/` ships with .vsix | ✓ | Not excluded — correct, walkthrough needs it |
| `**/*.md` excluded EXCEPT README + CHANGELOG | ✓ | Correct pattern |
| `icon.svg` excluded | ✓ | Source SVG excluded, only PNG ships |
| `npm run verify:marketplace` script | ✓ | Compiles, bundles, packages — run this before push |
| `npm run publish:openvsx` script | ✓ | Uses same .vsix artifact |
| Stale `.vsix` artifact in repo | ⚠ | `perl-lsp-rs-0.12.0.vsix` is committed to `vscode-extension/`. This does not ship (`.vscodeignore` has `*.vsix`) but it bloats the repo. Consider deleting it. |

---

## 2. Visual Assets — What You Need to Gather

These are the items that require a human with a running editor. The storyboard references
and render helper already exist — you just need the recordings.

### BLOCKING — icon dimensions

The icon is 256×256 but marketplace recommendations say 128×128. The current file
will publish fine (vsce accepts up to 256×256), but verify it looks sharp at 28px
(the size shown in the Extensions panel sidebar). If the current design has fine
detail that disappears at small sizes, create a simplified 128×128 variant.

**Recommended action**: open `icon.png` at 28px zoom and check readability. If it
reads cleanly — no action needed. If it blurs — create `icon-128.png` from `icon.svg`.

### Screenshots for README.md (highest ROI)

The marketplace listing README shows inline images. You currently have zero. Add
3–5 in the README by hosting them in `media/` or as GitHub raw links.

Recommended scenarios:

1. **Go to definition** — hover over a sub name, see the definition preview, F12 to jump. Shows the "it just works" moment. File: `demo_workspace/main.pl` → `lib/Utils.pm`.
2. **Diagnostics in action** — a `.pl` file with a visible red squiggle on an undefined variable, showing the error tooltip. Quick, convincing.
3. **Debug session** — breakpoint hit, Variables panel showing Perl values, call stack visible. This is the strongest differentiator vs. other Perl extensions.
4. **Completions** — Ctrl+Space triggered in a Perl file, showing module/function/variable suggestions with documentation.
5. **Refactoring** — `Extract Variable` code action active on a selected expression. (Optional, but shows depth.)

**Format**: PNG or GIF. PNG for static, GIF for animated. Keep GIFs under 5MB for
fast GitHub rendering. Use the `render-walkthrough-gif.py` helper at
`scripts/marketing/render-walkthrough-gif.py` — pass `--max-bytes 5000000`.

**Where to put them**: `vscode-extension/media/screenshots/` and reference with
relative paths in README.md, e.g.:

```markdown
![Go to Definition](media/screenshots/goto-definition.png)
```

### Walkthrough GIFs (in-extension, not marketplace)

These appear in VS Code's "Get Started" walkthrough panel, not on the marketplace page.
Three recordings planned per `media/walkthrough/README.md`:

| Target file | Storyboard | What to record |
|-------------|-----------|----------------|
| `install-health.gif` | `install-health.svg` | Fresh install + `Perl: Run Health Check` command |
| `find-references.gif` | `find-references.svg` | F12 go-to-definition + Find All References in demo workspace |
| `extract-variable.gif` | `extract-variable.svg` | Select expression → code action → Extract Variable |

Render command:
```bash
python scripts/marketing/render-walkthrough-gif.py \
  --input recordings/<name>.mp4 \
  --output vscode-extension/media/walkthrough/<name>.gif \
  --max-bytes 8000000
```

---

## 3. Draft Copy — Options to Choose From

### Option A: `description` in package.json (shown in marketplace search results)

Current: "Fast, native Perl 5 language server: go-to-definition, completions, diagnostics,
refactoring, debugging, and 98 LSP/DAP features. Zero runtime dependencies."

**Option A-1** (feature breadth, current tone):
> Native Rust LSP + DAP for Perl 5. Go-to-definition, completions, semantic highlighting,
> refactoring, step debugging, and Test Explorer. Zero runtime dependencies — installs in seconds.

**Option A-2** (lead with the differentiator):
> The only Perl LSP written in Rust. Instant startup, 98 LSP/DAP features,
> and a full step debugger that works without babysitting. Zero runtime dependencies.

**Option A-3** (dev-experience angle):
> IDE-quality Perl development in VS Code: real-time diagnostics, smart completions,
> step debugging, and refactoring — all driven by a native Rust server with no runtime deps.

Recommendation: A-1 is the safest update (closest to current, adds DAP/Test callout).
A-2 is the most assertive. Choose based on tone you want for the public alpha.

### Option B: README intro paragraph (first paragraph after the badges)

Current: "A fast, native Perl 5 language server extension. Written in Rust for speed
and reliability. No runtime dependencies -- just install and code."

**Option B-1** (minimal change, update version ref):
> A fast, native Perl 5 language server. Written in Rust for speed and reliability.
> No runtime dependencies — install the extension and start coding.

**Option B-2** (lead with the preview notice update for v0.13.0):
> Perl Language Server brings IDE-quality Perl development to VS Code and VSCodium.
> Built on a native Rust parser and language server — no Perl modules required,
> no Node.js middleware. Just install and code.
>
> **v0.13.0 Public Alpha** — active development. [Report issues](https://github.com/EffortlessMetrics/perl-lsp/issues/new/choose).

**Option B-3** (tightest, punchy):
> Native Rust LSP + DAP for Perl 5. No runtime dependencies. Install and go.
>
> **v0.13.0 Public Alpha** — [report issues here](https://github.com/EffortlessMetrics/perl-lsp/issues/new/choose).

Recommendation: B-2 gives the most context for new visitors arriving from the marketplace.
B-3 works if you expect the screenshot grid to do the heavy lifting.

### Option C: Keywords (trim to 5)

Current 10 keywords (marketplace silently drops extras beyond 5):
`perl, perl5, language-server, lsp, debugger, refactoring, code-completion, diagnostics, vscodium, open-vsx`

Recommended top-5 (prioritize what users search for):
```json
["perl", "perl5", "debugger", "lsp", "language-server"]
```

Alternative if you want to capture VSCodium users:
```json
["perl", "perl5", "debugger", "vscodium", "language-server"]
```

`open-vsx` as a keyword is redundant once the extension is listed there.
`refactoring` and `code-completion` are less common search terms than `debugger`.

---

## 4. Marketplace-Specific Gotchas

**Keywords hard cap is 5, not a soft suggestion.** The marketplace [documentation](https://code.visualstudio.com/api/references/extension-manifest) states: "Keywords to make it easier to find the extension. These are included with other extension Tags on the Marketplace. Limit to 5." The current 10-keyword list means the last 5 entries (`refactoring`, `code-completion`, `diagnostics`, `vscodium`, `open-vsx`) are silently dropped.

**Do not use Shields for VS Marketplace install counts.** Shields deprecated the live VS Marketplace install badge route; use the repo-owned static badge (`docs/assets/badges/vscode-marketplace-installs.svg`) and update it manually from publisher metrics when needed. Other Shields badges in README remain fine.

**`"markdown": "github"` is already set** in package.json. This tells the marketplace to render README.md with GitHub-Flavored Markdown, which enables tables, checkboxes, and fenced code blocks. No action needed.

**Gallery banner color matters.** The `#1e3a8a` dark blue is set. This is the header background on the marketplace extension page. It looks fine against a dark theme. Confirm it does not clash with the icon colors before publishing.

**Icon file format**: PNG only, no JPEG. You have a PNG. The `icon.svg` source is correctly excluded from the .vsix via `.vscodeignore`.

**Open VSX image rendering**: Open VSX renders the README identically for hosted images. GitHub raw URLs (`https://raw.githubusercontent.com/...`) work. Relative paths within the .vsix (e.g., `media/screenshots/foo.png`) also work if the image is included in the package — and `media/` is not excluded in `.vscodeignore`.

**Gallery image max size**: VS Marketplace does not publish a hard limit for README images, but the practical limit for fast rendering is 1MB per image. Keep screenshots under 500KB; animated GIFs under 5MB.

**`preview: true` flag**: The extension is correctly marked as preview. This adds a "Preview" badge on the listing. Remove this flag in `package.json` when you move from public alpha to stable. Do not remove it for v0.13.0.

**`.vsix` file committed to the repo**: `vscode-extension/perl-lsp-rs-0.12.0.vsix` is a stale build artifact checked into git. `.vscodeignore` excludes `*.vsix` from packaging (correct), but the file is still in the git history. It does not affect publishing, but it bloats clones. Consider removing it with a follow-up commit.

**`Shift+Alt+F` keybinding discrepancy**: The README keyboard shortcuts table lists `Shift+Alt+F` for Format Document, but this shortcut is not registered in `contributes.keybindings`. VS Code assigns `Shift+Alt+F` to its built-in `editor.action.formatDocument` command. The shortcut works because of VS Code's default binding, not the extension's. Either register it explicitly or add a footnote in the README explaining this.

---

## 5. When You're Ready to Publish

Prerequisites: `VSCE_PAT` and `OVSX_PAT` set as environment variables (or GitHub secrets for CI).

```bash
# 1. From repo root — confirm workspace version is 0.13.0
cargo metadata --format-version=1 --no-deps | python3 -c "
import json,sys; m=json.load(sys.stdin)
[print(p['name'],p['version']) for p in m['packages'] if p['name']=='perl-lsp-rs']
"

# 2. From vscode-extension/
cd vscode-extension
npm install

# 3. Build + package (runs TypeScript compile, bundles binary, creates .vsix)
npm run verify:marketplace
# This produces perl-lsp-rs-0.13.0.vsix

# 4. Smoke test locally (optional but recommended)
code --install-extension perl-lsp-rs-0.13.0.vsix

# 5. Publish to VS Marketplace
npx @vscode/vsce publish --pat "$VSCE_PAT"
# Or: npm run publish -- --pat "$VSCE_PAT"

# 6. Publish to Open VSX (same .vsix)
npx ovsx publish perl-lsp-rs-0.13.0.vsix --pat "$OVSX_PAT"
# Or: npm run publish:openvsx -- perl-lsp-rs-0.13.0.vsix --pat "$OVSX_PAT"

# 7. Verify both listings
open https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs
open https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs
```

For CI-driven publish (GitHub Actions), see `RELEASE_CHECKLIST.md` for the full
secret-check preflight and `PUBLISHING_ROADMAP.md` for the release-day sequence.

---

## 6. Prioritized Action List

### Blocking (will cause publish failure or a broken listing)

- [ ] **Trim keywords to 5** in `package.json`. Current 10 silently drops 5 entries.
- [ ] **Write `[0.13.0]` CHANGELOG entry** before publishing 0.13.0.
- [ ] **Update the `preview` version reference in README.md** from `0.12.2 Public Alpha` to `0.13.0 Public Alpha`.

### High value (do before v0.13.0 announce)

- [ ] **Add 3–5 screenshots to README.md**. Zero images is the most impactful gap for marketplace conversion. Capture from `demo_workspace/`.
- [ ] **Remove the "Walkthrough Previews" section from README.md** (lines 53–63). It exposes internal dev notes to marketplace visitors.
- [ ] **Add a "Known Issues" section** to README.md (even one sentence pointing to GitHub issues).
- [ ] **Pick and apply the 5-keyword list** from the options in Section 3.

### Nice to have (can do after announce)

- [ ] **Record the 3 walkthrough GIFs** using `scripts/marketing/render-walkthrough-gif.py`. These improve the in-extension first-run experience, not the marketplace listing itself.
- [ ] **Delete the stale `perl-lsp-rs-0.12.0.vsix`** from `vscode-extension/`. It does not block publishing but bloats git history.
- [ ] **Verify icon at 28px** — confirm it reads at sidebar icon size.
- [ ] **Add perltidy/perlcritic as an explicit "Requirements" section** in README.md.
- [ ] **Resolve the `Shift+Alt+F` footnote** in the keyboard shortcuts table.
- [ ] **Consider root `LICENSE` file** — the repo root has no LICENSE file (only `vscode-extension/LICENSE`). GitHub shows the license badge from the root. Worth adding a root `LICENSE` or symlink.
