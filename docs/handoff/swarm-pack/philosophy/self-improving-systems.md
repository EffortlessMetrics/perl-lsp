# Self-Improving Systems

How a swarm learns from its own failures and gets better each cycle.

## The Problem with Static Agents

Most agent systems are static: you write the agent definition once, and it runs the same way forever. If the agent makes a mistake, it makes the same mistake next time. If the agent discovers something useful, the discovery dies when the agent exits.

## Five Learning Loops

The swarm has five mechanisms that make it smarter over time:

### 1. Known Pitfalls (Fixer → Scout/Builder)

When a fixer diagnoses a failure, it writes the lesson to `known-pitfalls.md`:
```
### 2026-03-15 — parser
**Pitfall**: Tests for perl-parser-core flake above RUST_TEST_THREADS=2
**Fix**: Always use RUST_TEST_THREADS=2 for parser tests
```

Every future scout includes relevant pitfalls in handoff files. Every future builder reads pitfalls before starting. The mistake happens once; the lesson persists permanently.

### 2. Discovered Issues (All Agents → Scout)

Every agent files GitHub issues for problems they notice outside their scope. Scouts check `gh issue list --label swarm-discovered` as a primary input source. Discoveries are pre-investigated leads with full context — the next agent doesn't start from scratch.

### 3. Metrics Analysis (All Agents → Strategist → Scouts)

Every agent writes performance metrics. The strategist analyzes patterns:
- "parser-fix-engine has 90% green rate, but dap-test has 30%"
- "The swarm has merged 20 P3 PRs but only 3 P1 PRs in the last cycle"

The strategist sends priority steering messages to scouts: "Focus on P1 parser work, pause P3 cleanup." The swarm redirects its effort based on data.

### 4. Agent Patches (Failing Agents → Bootstrapper)

When an agent hits friction caused by its own definition, it writes a patch proposal:
```
# Patch: parser-fix-engine
## Problem: doesn't know about heredoc-specific parser state
## Suggested Change: add heredoc section to agent definition
## Evidence: 3 failed builds on heredoc-related slices
```

The bootstrapper integrates validated patches during `--refresh`. The agent definitions evolve based on field experience, not just initial design.

### 5. Handoff → ADR Pipeline (Builders → Improver-docs)

Builders write "Key Decisions" in handoff files. The docs improver reads these and crystallizes them into Architecture Decision Records. Decisions that would otherwise be lost when agents exit become permanent project documentation.

## What This Looks Like Over Time

**Cycle 1**: The swarm is clumsy. Agents make avoidable mistakes. Handoffs are sparse. No pitfalls documented.

**Cycle 5**: 15 pitfalls documented. Scouts avoid known traps. Agent patches have fixed 3 definition issues. 10 discovered issues have been resolved.

**Cycle 20**: The pitfalls file is comprehensive. Agent definitions are field-tested and improved. The strategist has steered scouts away from low-impact work twice. 50+ discovered issues have been resolved. The improvement rate is visibly higher than cycle 1.

**Cycle 50**: The swarm is a mature system. It finds gaps quickly (scouts are well-calibrated), builds confidently (pitfalls prevent known failures), validates thoroughly (metrics show which checks catch real issues), and stays aligned (strategist keeps priorities current).

## The Key Insight

The swarm isn't just parallel agents. It's a system with **feedback loops**. Outputs from one cycle become inputs to the next. Failures become knowledge. Discoveries become work items. Decisions become documentation. Performance data becomes strategic steering.

A static system does the same thing forever. A self-improving system does the right thing better each time.
