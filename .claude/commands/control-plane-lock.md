---
description: Advisory single-writer lock for control-plane files (.claude/agents/, .claude/commands/, CLAUDE.md)
user-invocable: true
---

# Control-Plane Lock

Advisory lock that prevents multiple agents from editing control-plane files simultaneously.

**Protected paths:** `.claude/agents/`, `.claude/commands/`, `CLAUDE.md`

**Lock file:** `.ops-perl-lsp/control-plane.lock`

**TTL:** 30 minutes (stale locks are auto-expired)

## Usage

### Acquire before editing control-plane files

```bash
scripts/control-plane-lock.sh acquire <your-agent-id>
```

Returns `OK: lock acquired by '<agent-id>'` on success.
Returns non-zero exit code with an error message if the lock is held.

### Release when done

```bash
scripts/control-plane-lock.sh release <your-agent-id>
```

Always release when you finish — even if your edit failed.

### Check current state

```bash
scripts/control-plane-lock.sh status
```

Reports: `unlocked`, `locked: holder='...' age=Xs remaining=Ys`, or `stale (expired): ...`

### Emergency release (orchestrator only)

```bash
scripts/control-plane-lock.sh force-release
```

Clears the lock regardless of who holds it. Use only when an agent crashed and left the lock held.

## Protocol for agents editing control-plane files

1. `scripts/control-plane-lock.sh acquire <agent-id>`
2. Edit `.claude/agents/<file>` or `.claude/commands/<file>` or `CLAUDE.md`
3. `scripts/control-plane-lock.sh release <agent-id>`

If acquire fails, do not retry in a loop. File your safe edits (per-crate CLAUDE.md, issue comments) and report contention to the orchestrator.

## Important notes

- This is an **advisory** lock. It coordinates willing agents; it does not block filesystem access.
- Lock is scoped to the repository — one writer at a time across all agents in the worktree.
- If a lock is stale (> 30 min old), `acquire` will clear it automatically with a warning.
- Never leave the lock acquired across a long-running operation. Acquire -> edit -> release should be fast.
