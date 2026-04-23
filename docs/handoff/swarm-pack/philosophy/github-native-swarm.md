# GitHub-Native Swarm Operations

Why the swarm uses GitHub as its source of truth instead of flat files.

## The Temptation

It's easy to build swarm coordination with flat files: a JSON queue, a markdown log, a YAML status tracker. Files are simple, fast, and don't require external services.

But flat files fail at exactly the things that matter for swarm coordination:
- **Visibility**: nobody sees what's in `.ops/swarm-queue.json` unless they look
- **Search**: you can't query a markdown file for "all parser-related discoveries"
- **Persistence**: flat files live in a worktree that might get pruned
- **Collaboration**: flat files don't have comments, reactions, or assignees
- **Audit trail**: flat files don't track who changed what when

## The GitHub-Native Approach

The swarm uses GitHub for everything that benefits from visibility, persistence, and searchability:

### Work Items Are Issues
When an agent discovers something, it creates a GitHub issue — not a line in a file:
```bash
gh issue create --title "test: perl-dap-value has 0 tests for 316 LOC" \
  --label "swarm-discovered" --body "..."
```

Issues are searchable, commentable, assignable, closeable. They show up in project boards. They have timestamps. They link to PRs. They survive worktree pruning.

### Work Products Are PRs
Every piece of work becomes a PR with a label:
- `swarm-core` for primary task work
- `swarm-improve-docs` for documentation
- `swarm-improve-tests` for test quality
- `swarm-improve-infra` for infrastructure

Labels make it trivial to query what the swarm has been doing:
```bash
gh pr list --state merged --label "swarm-improve-tests" --limit 50
```

### State Queries Use `gh`
The swarm checks its own state through GitHub, not flat files:
```bash
gh pr list --state open              # What's in progress?
gh issue list --label swarm-discovered  # What's been found?
gh run list --status failure          # What's broken?
```

### Auto-Merge Reduces Bottlenecks
Small, well-tested PRs enable auto-merge:
```bash
gh pr merge <N> --auto --squash --delete-branch
```
The PR merges when checks pass. No human or merger agent needs to poll.

## What Stays in Files

Not everything belongs in GitHub. Some things are better as local files:

| Use Case | GitHub or File? | Why |
|----------|----------------|-----|
| Work items | GitHub issues | Searchable, persistent, commentable |
| Work products | GitHub PRs | Labeled, reviewable, mergeable |
| CI state | GitHub runs | Authoritative, observable |
| Handoff context | File (`.ops/handoffs/`) | Ephemeral, per-branch, too granular for issues |
| Failure knowledge | File (`.claude/swarm-state/`) | Append-only, machine-read, cross-references needed |
| Performance metrics | File (`.ops/swarm-metrics.jsonl`) | High-frequency append, analyzed in bulk |
| Agent patches | File (`.ops/agent-patches/`) | Temporary proposals, applied then deleted |

The rule: **visible, persistent, searchable things go to GitHub. Machine-internal, ephemeral, high-frequency things stay in files.**

## The Result

An observer can understand what the swarm is doing entirely through GitHub:
- Open PRs show what's being built and reviewed
- Merged PRs show what was shipped
- Issues labeled `swarm-discovered` show what was found
- Issues labeled `swarm-architectural` show what needs human decision
- PR labels show the capacity allocation (core vs improvement)
- CI runs show what's passing and failing

No need to SSH in and `cat .ops/swarm-queue.json`. The dashboard is GitHub itself.
