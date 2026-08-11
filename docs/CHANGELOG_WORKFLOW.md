# Changelog workflow

`perl-lsp` uses [Changie](https://changie.dev/) for new changelog fragments and
release batching. `CHANGELOG.md` remains the published, human-readable history.

The migration is intentionally forward-only:

- the curated pre-Changie sections in `CHANGELOG.md` remain an immutable legacy
  tail;
- new PRs add structured files under `.changes/unreleased/`;
- release automation runs `changie batch` and inserts that version immediately
  after `## [Unreleased]`;
- the adapter does not regenerate or reinterpret older release prose.

This boundary matters because some historical source-tag ranges are incomplete
or inflated by cross-repository synchronization. A new renderer must not erase
or silently rewrite the corrected archive.

## Contributor flow

### Add a fragment

Run:

```bash
changie new
```

Choose the narrowest applicable kind and describe the user-observable change.
A useful fragment says what changed, when it activates, and—where material—what
fallback or failure behavior remains.

Commit the generated YAML file with the implementation PR:

```bash
git add .changes/unreleased
```

A fragment is normally required for:

- user-visible LSP, DAP, parser, formatter, CLI, extension, or install changes;
- protocol and configuration changes;
- security fixes;
- deprecations, removals, and breaking public API changes.

A fragment is normally unnecessary for:

- test-only receipts with no behavior change;
- internal refactors that preserve externally visible behavior;
- formatting-only changes;
- generated-file refreshes.

When classification is uncertain, prefer a short `Internal` fragment over
silence. Release reviewers can omit or reclassify it with an explicit reason.

### Non-interactive fragment creation

CI and scripted work may pass values directly:

```bash
changie new --kind fixed --body \
  'Prevent stale completion results after the document generation advances.'
```

Fragment files have this shape:

```yaml
kind: fixed
body: Prevent stale completion results after the document generation advances.
time: 2026-07-12T05:45:00Z
```

Do not hand-author timestamps or filenames when `changie new` is available.

## Release flow

The version-bump workflow resolves the workspace version, then runs:

```bash
changie batch vX.Y.Z
python3 scripts/changie_merge_legacy.py vX.Y.Z
```

`changie batch` consumes unreleased fragments into `.changes/vX.Y.Z.md`.
The adapter inserts that rendered version after `[Unreleased]` while proving the
existing historical tail is copied without textual transformation.

The workflow fails when:

- no unreleased fragments exist;
- the expected `.changes/vX.Y.Z.md` file is absent;
- the version heading does not match the requested version;
- the changelog already contains that version;
- `[Unreleased]` is missing or duplicated;
- the legacy-tail renderer tests fail.

The release PR must review both files:

```text
.changes/vX.Y.Z.md
CHANGELOG.md
```

The version file is the structured release input. `CHANGELOG.md` is the rendered
public history.

## Release-note provenance is separate

Changie solves fragment capture; it does not determine the true release
boundary. Release notes still begin from the logical `perl-lsp-swarm`
first-parent range and are verified against the final `perl-lsp` release tree.

Use the release checklist and release-note guidance to record:

- previous and new immutable swarm RC SHAs;
- the two-parent source sync commit;
- documented source-only exclusions;
- user-visible changes represented or explicitly excluded;
- claim boundaries for disabled, shadow-only, and proof-only work.

A complete fragment set is necessary but not sufficient evidence that the
release note is complete.

## Configuration

Changie is configured by [`.changie.yaml`](../.changie.yaml). The active kinds
are:

| Key | Rendered section | Automatic SemVer signal |
|---|---|---|
| `added` | Added | minor |
| `changed` | Changed | minor |
| `deprecated` | Deprecated | minor |
| `removed` | Removed | minor (0.x breaking change) |
| `fixed` | Fixed | patch |
| `security` | Security | patch |
| `documentation` | Documentation | patch |
| `internal` | Internal | patch |

The release workflow pins Changie `v1.24.0`. Local installation:

```bash
GOBIN="$HOME/.local/bin" go install github.com/miniscruff/changie@v1.24.0
```

## Validation

Run the adapter tests directly:

```bash
python3 scripts/tests/test_changie_merge_legacy.py
```

Inspect pending fragments without changing them:

```bash
find .changes/unreleased -maxdepth 1 -type f -name '*.yaml' -print -exec cat {} \;
```

Test a batch only on a disposable branch or worktree: batching consumes the
unreleased YAML files.

## Migration boundary

`cliff.toml` and the old `just changelog*` shortcuts remain transitional files
until their dedicated cleanup lands. They are no longer used by the version-bump
workflow and must not be used to prepare a release.

Do not run `changie merge` against the repository yet. The historical versions
have not been converted into native Changie version files; a raw merge would
omit the legacy tail. Use `scripts/changie_merge_legacy.py` through the release
workflow.

A later cleanup may import old releases into `.changes/v*.md` after a separate,
byte-for-byte reconciliation. That is archival maintenance, not a release
prerequisite.
