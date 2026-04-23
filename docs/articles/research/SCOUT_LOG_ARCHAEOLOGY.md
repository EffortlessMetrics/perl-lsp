# Scout Log Archaeology
## How The Current Swarm Preserves Session Research Before It Becomes Doctrine

The current swarm memory model is not only `swarm-state`.

`swarm-state` records durable coordination memory:

- overlap
- pitfalls
- dedup
- stable control-plane findings

The tracked scout logs add a different layer:

- dated session research
- scoped investigation notes
- interim recommendations
- preserved evidence that can later be absorbed into historical docs

That makes them an intermediate memory tier between live coordination state and
fully digested archaeology.

---

## 1. What The Tracked Scout Logs Are

The committed scout-log surface currently contains:

- [2026-03-19-v0.12.0-readiness.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/logs/scouts/2026-03-19-v0.12.0-readiness.md)
- [2026-03-19-install-experience.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/logs/scouts/2026-03-19-install-experience.md)

Both are clearly session-scoped research artifacts:

- they carry a date
- they name a scope
- they summarize findings
- they recommend interpretation or sequencing
- they end with a note that the useful findings were absorbed into tracked
  historical docs

That last point matters. These are not random leftovers. They are preserved
research receipts after the higher-level story has already been written up.

---

## 2. They Are Not The Same Thing As `swarm-state`

[`.claude/swarm-state/README.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/README.md)
is explicit that `swarm-state` is for durable coordination knowledge:

- `swarm-queue.json` for overlap tracking
- `completed-slices.md` for dedup
- `discovered-issues.md` for out-of-scope leads
- `known-pitfalls.md` for reusable traps
- `findings.json` for stable control-plane conclusions

The scout logs serve a different job.

They are not:

- active queue state
- long-lived doctrine
- a product bug tracker
- the final polished article layer

They are preserved research runs.

That makes them more like session evidence than control-plane law.

---

## 3. The Two March 19 Logs Show The Pattern Clearly

The readiness log preserves a shipping assessment:

- current baseline and error buckets
- feature completeness
- test coverage shape
- release truth versus milestone truth
- recommended ship sequence

The install log preserves a launch-surface audit:

- install paths
- verification commands
- downloader and checksum behavior
- first-run UX observations
- documentation coverage

Both logs are rich enough that future sessions do not need to rediscover the
same reasoning from scratch. But both also remain narrower and more provisional
than the final archaeology notes written from them.

This is why they are historically interesting. They capture the swarm thinking
in a recoverable form before everything gets normalized into polished docs.

---

## 4. The Logs Became Tracked On Purpose

Local git history shows the current scout logs were committed in:

- `344c6a591` on `2026-03-19`
  `docs: track scout logs for archaeology context`

That commit message is blunt about intent: the logs are not transient residue.
They were promoted into tracked history specifically because they are useful for
archaeology context.

This is another memory-system escalation:

1. session runs produce scout artifacts
2. some of those artifacts prove worth preserving
3. the repo starts tracking them as recoverable evidence

That is knowledge compounding, not just note hoarding.

---

## 5. The Resulting Memory Stack

With the scout logs included, the current memory model looks more layered than
`swarm-state` alone suggests:

1. `swarm-state` for live durable coordination
2. scout logs for preserved session research
3. archaeology notes for synthesized historical interpretation
4. article indexes and launch pieces for publication-facing narrative

The scout logs therefore sit between runtime memory and polished history.

They are the evidence shelf that lets the repo keep more of its own discovery
process.

---

## 6. Why This Matters

This repository is unusual partly because it keeps trying to reduce how much
important knowledge dies with the session that produced it.

`swarm-state` makes control-plane memory durable.
Tracked scout logs make investigative memory recoverable.
Historical notes make interpretation portable.

That stack is what turns one swarm run into material future runs can build on.

---

## Evidence Pointers

- [`.claude/logs/scouts/2026-03-19-v0.12.0-readiness.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/logs/scouts/2026-03-19-v0.12.0-readiness.md)
- [`.claude/logs/scouts/2026-03-19-install-experience.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/logs/scouts/2026-03-19-install-experience.md)
- [`.claude/swarm-state/README.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/README.md)
- [SWARM_STATE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_STATE_ARCHAEOLOGY.md)
- [SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md)
- [INSTALL_SURFACE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/INSTALL_SURFACE_ARCHAEOLOGY.md)
- [ALPHA_READINESS_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ALPHA_READINESS_ARCHAEOLOGY.md)
- commit `344c6a591`
