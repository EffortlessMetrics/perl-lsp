# Memory Promotion Guide

How to classify and promote knowledge discovered during development sessions.

## Promotion Taxonomy

| Level | What It Is | Where It Goes | Example |
|-------|-----------|---------------|---------|
| **Pitfall** | Something that burned us | feedback memory file | "Don't broadcast shutdown" |
| **Finding** | Something discovered but not yet actionable | project memory file | "semantic.rs is 3,256 lines" |
| **Issue Seed** | Finding ready for a GitHub issue | `gh issue create` | "Wire dead code detector" |
| **Article Evidence** | Finding ready for publication | docs/articles/research/ | "8:1 test-to-code ratio" |
| **Archaeology** | Historical context worth preserving | docs/articles/research/ | "Five eras of development" |
| **Durable Rule** | Proven pattern worth enforcing | CLAUDE.md or skill | "Scout before building" |

## Promotion Flow

```
Discovery -> Memory (pitfall/finding)
           -> Issue Seed -> GitHub Issue -> Builder PR
           -> Article Evidence -> Article Draft -> Publication
           -> Durable Rule -> CLAUDE.md/skill update
```

A single discovery can follow multiple paths. A finding about god file sizes might become both an issue seed (to split the file) and article evidence (for a complexity analysis article).

## When to Promote

- **If it burned you twice** -> pitfall (immediate). Save as a feedback memory file so agents never repeat the mistake.
- **If it's actionable by a builder** -> issue seed. Create a GitHub issue with root cause, reproduction, and fix direction.
- **If it's quotable with a number** -> article evidence. Save to `docs/articles/research/` with source and date.
- **If it applies to ALL future sessions** -> durable rule. Update CLAUDE.md or add enforcement to a skill.

## When NOT to Promote

- **If it's ephemeral session state** -> don't save. Current agent roster, in-progress task lists, and temporary workaround details belong in the conversation, not in memory.
- **If it's already in git history** -> don't duplicate. Commit messages, PR descriptions, and code comments are already permanent. Memory should point to them, not copy them.
- **If it contradicts a newer memory** -> update the old one. Don't create competing memories; find and revise or remove the stale entry.

## Promotion Checklist

Before promoting, verify:

1. **Is it already captured?** Check existing memory files and CLAUDE.md.
2. **Is it still true?** Verify against current codebase state, not the state when it was discovered.
3. **Does it generalize?** A fix for one file is a commit. A pattern across ten files is a finding.
4. **Who benefits?** If only this session, don't save. If future agents, promote.

## Examples

### Finding -> Issue Seed

```
# Discovery during scout
"postfix.rs has 847 lines and handles 12 different operator types"

# Promoted to issue seed
gh issue create --title "Split postfix.rs operator dispatch into per-operator modules" \
  --body "postfix.rs is 847 lines handling 12 operator types. Extract each into its own module under expressions/postfix/."
```

### Finding -> Article Evidence

```
# Discovery during metrics review
"Test-to-code ratio is 8:1 across the workspace"

# Saved to docs/articles/research/test-ratio-analysis.md
# with measurement methodology and date
```

### Pitfall -> Durable Rule

```
# First burn: agent broadcast shutdown crashed 3 teammates
# Second burn: same pattern in cycle 4

# Promoted from feedback memory to CLAUDE.md:
# "Never broadcast shutdown messages; send targeted messages to specific agents"
```
