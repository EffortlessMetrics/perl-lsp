# Context Efficiency: The Handoff Protocol

Why how you pass information between agents matters more than how many agents you have.

## The Problem

You have 30 agents working in parallel. Agent A reads 10 files to understand a bug. Agent B needs to fix the bug. If Agent B re-reads the same 10 files, you've wasted half of the work.

Scale this to a swarm: scouts read source files, builders re-read them, reviewers re-read the diff, improvers re-read the handoff... each stage duplicates the context-gathering of the previous stage. The agents are fast, but the context re-reading is O(n) per stage.

## The Hierarchy

There are three models for multi-agent context sharing, from best to worst:

### 1. Effective Handoffs (Best)

Agent A condenses its findings into a handoff file: code excerpts, error messages, fix strategy, test template. Agent B reads ONLY the handoff — 1 file instead of 10. Agent B appends its own findings for Agent C.

**Cost**: Agent A spends 30 seconds writing the handoff. Agent B saves 5 minutes of re-reading. Net: massive savings that compound with every stage.

### 2. Shared Context Window (Good)

One agent with skills and cache efficiency. It reads the files once and uses Claude Code skills to load rules on demand. No handoff overhead, but also no parallelism — it's one agent doing everything sequentially.

### 3. Independent Re-reading (Worst)

Each agent reads everything from scratch. 10 files × 4 stages = 40 file reads instead of 10 + 3 handoff reads. This is the default when you don't design for context efficiency.

## How the Handoff Protocol Works

```
Scout reads 10 source files
  │ writes handoff: code excerpts, test template, fix strategy
  ▼
Builder reads 1 handoff file
  │ appends: what changed, key decisions, reviewer watch-list
  ▼
Reviewer reads the builder briefing + focused diff
  │ creates PR
  ▼
Improvers read "Lesson Learned" sections → ADRs, friction log
```

Each stage reads **only what the previous stage condensed**. The handoff file grows as it passes through the pipeline, accumulating context from each stage.

## What Goes In a Handoff

The scout's handoff to the builder:
- **Code excerpts** (10-30 lines of relevant code, so the builder doesn't re-read the file)
- **Error output** (actual error message, so the builder doesn't re-run to see it)
- **Fix strategy** (specific steps, so the builder doesn't re-analyze the options)
- **Test template** (pre-filled skeleton, so the builder starts from the scout's investigation)
- **Known pitfalls** (relevant failure patterns, so the builder doesn't repeat known mistakes)

The builder's addition for the reviewer:
- **What changed and why** (so the reviewer reads the briefing, not the cold diff)
- **Key decisions** (non-obvious choices explained)
- **What to watch for** (specific areas of uncertainty flagged)

## Minimal Subagent Prompts

The handoff protocol also applies to how coordinators spawn subagents. Instead of pasting 100 lines of instructions into the prompt:

```
"Read .ops/handoffs/<branch>.md for context.
 Read .claude/swarm-state/known-pitfalls.md for traps.
 Invoke /swarm-protocol and /coding-standards.
 Branch: X. Package: Y. Verify: fmt && lint && test.
 Append reviewer briefing to handoff."
```

7 lines. The handoff file has the context. The skills have the rules. No context wasted on inline instructions.

## Skills Over File Reads

Protocol, standards, and priorities are Claude Code skills (`/swarm-protocol`, `/coding-standards`, `/swarm-priorities`), not files that need a Read tool call. The skill loads directly into the agent's context — no tool call overhead, no "I read the file but let me check what it said" re-scanning.

## The Compound Effect

In a 50-cycle swarm that processes 500 slices:
- **Without handoffs**: 500 × 10 source file reads = 5,000 file reads
- **With handoffs**: 500 × 1 handoff read + 500 × 1 handoff write = 1,000 operations

5x reduction in context gathering. And the handoffs are more useful than raw source files because they contain condensed analysis, not just code.
