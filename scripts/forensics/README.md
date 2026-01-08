# Forensics Tooling

Scripts for PR archaeology and fact extraction.

## Purpose

Extract structured data from merged PRs to support forensics analysis:

- **Fact bundles**: Machine-readable PR metadata, commits, and change surfaces
- **Dependency tracking**: Cargo.lock/Cargo.toml diff analysis
- **Verification data**: CI check run results

Output feeds into dossier creation per [`docs/FORENSICS_SCHEMA.md`](../../docs/FORENSICS_SCHEMA.md).

## Tools

### pr-harvest.sh

Extract fact bundle from a merged PR.

```bash
# Output YAML to stdout
./scripts/forensics/pr-harvest.sh 259

# Output to file
./scripts/forensics/pr-harvest.sh 259 -o pr-259-facts.yaml
```

**Prerequisites:**
- `gh` CLI (authenticated)
- `jq` for JSON processing
- Git repository with PR commits available locally

**Output schema:**

```yaml
pr:
  number: <int>
  title: <string>
  url: <string>
  author: <string>
  created_at: <ISO8601>
  merged_at: <ISO8601>
  labels: [<string>]
  reviewers: [<string>]

commits:
  base_sha: <string>
  head_sha: <string>
  merge_commit: <string>
  count: <int>
  history:
    - sha: <string>
      date: <ISO8601>
      author: <string>
      message: <string>

change_surface:
  files_changed: <int>
  insertions: <int>
  deletions: <int>
  hotspots:
    - path: <string>
      insertions: <int>
      deletions: <int>
  crates_touched: [<string>]
  dependency_delta:
    added: [<string>]
    removed: [<string>]
    updated: [<string>]

verification:
  check_runs:
    - name: <string>
      conclusion: <string>

body: <multiline string>
```

## Workflow

1. **Harvest facts**: `./scripts/forensics/pr-harvest.sh <PR>`
2. **Create dossier**: Use output to populate `docs/forensics/pr-<N>.md`
3. **Apply schema**: Follow [`FORENSICS_SCHEMA.md`](../../docs/FORENSICS_SCHEMA.md) for analysis
4. **Extract lessons**: Add findings to [`docs/LESSONS.md`](../../docs/LESSONS.md)

## Idempotency

All scripts are safe to re-run:

- `pr-harvest.sh` produces identical output for the same PR
- No side effects beyond writing to specified output file

## See Also

- [`docs/FORENSICS_SCHEMA.md`](../../docs/FORENSICS_SCHEMA.md) - Dossier schema
- [`docs/forensics/`](../../docs/forensics/) - PR dossiers
- [`docs/LESSONS.md`](../../docs/LESSONS.md) - Aggregated wrongness log
