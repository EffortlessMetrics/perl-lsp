# Every Agent Is a Scout

How continuous passive discovery makes codebases self-aware.

## The Problem

In traditional development, bugs and gaps are found by whoever happens to notice them. The developer touching auth code notices that logging doesn't work. The reviewer checking a parser fix sees that the error message is confusing. The CI engineer debugging a flake discovers that test isolation is broken.

These discoveries are accidents. They depend on the right person looking at the right code at the right time. Most gaps are never discovered because nobody happens to work near them.

## The Swarm Approach

In a swarm, every agent that touches the codebase is expected to report what it sees — even when it's outside their current task.

A **builder** implementing a parser fix notices the adjacent module has zero tests. It doesn't stop to write tests (that's scope creep). It files a GitHub issue: "perl-dap-value has 316 LOC and 0 tests. Discovered while working on fix/parser-heredoc." That issue includes enough context that a fresh agent can pick it up without re-investigating.

A **reviewer** checking three PRs in a row notices they all use the same workaround for a confusing API. It messages the docs improver: "Same pattern in 3 PRs — this needs an ADR explaining why `position_to_offset` requires the full document, not just the line."

A **fixer** debugging a CI failure discovers that the test was flaky because of a timing dependency. It writes to `known-pitfalls.md` so no future agent wastes time on the same diagnosis.

## Why This Compounds

Each discovery costs the discovering agent ~10 seconds: file an issue, message a teammate, append to a log. But the value is huge:

- **The issue becomes a task.** Scouts pick it up in the next cycle.
- **The pitfall prevents wasted time.** Every future agent avoids the trap.
- **The pattern becomes an ADR.** The knowledge persists permanently.

After 50 swarm cycles, the codebase has a comprehensive map of its own gaps — built organically by agents who were doing other work.

## What Makes It Work

1. **Low friction.** Filing a GitHub issue is one command: `gh issue create --label swarm-discovered --body "..."`. No tickets, no forms, no approval.

2. **Enough context.** The discovering agent includes file paths, line numbers, error messages, and what they were doing when they noticed. The next agent doesn't re-investigate — they pick up where the discovery left off.

3. **Closed loop.** Scouts actively check `gh issue list --label swarm-discovered` as an input source. Discoveries don't rot in a backlog — they get picked up in the next cycle.

4. **No scope creep.** The discovering agent does NOT try to fix what they found. They report it and continue their task. A fresh agent handles it later, with proper isolation and testing.

## The Result

A codebase under swarm development becomes **self-aware**. It knows where its tests are weak, where its docs are stale, where its error messages are confusing, and where its dead code lives. Not because someone audited it — but because dozens of agents walked through it and reported what they saw.
