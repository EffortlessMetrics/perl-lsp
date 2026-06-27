# Issue-Scout Protocol

> **Post where the work lives. Verify in opposition. Land only on convergence.**

## The model

```
scouts post on the issue
oppositional scouts verify on the issue
the issue converges
the coordinator only routes from the converged state
```

The GitHub issue thread is **not a report destination**. It is the database, the
whiteboard, the audit log, and the convergence rail. No agent's — and not the
orchestrator's — private context is where findings become real. The issue is.
The coordinator's job is to keep the system pointed at the durable layer, not to
*become* the durable layer.

This corrects an earlier, still-too-centralized model in which scouts briefed the
orchestrator and the orchestrator held the analysis in context. That recreates the
human-RAM bottleneck, serializes on one actor, and loses the analysis to
compaction. Convergence must happen on the durable layer instead.

## Rules

- Scouts post findings **directly on the GitHub issue**. Do not return substantive
  issue analysis only to the orchestrator.
- Every scout comment must carry seven fields: **current state · evidence
  (file:line / tests / PRs / commands) · opposing checks (what could refute this,
  and what was checked) · verdict · updated plan · acceptance criteria · residual
  uncertainty.**
- Oppositional verifiers post `CONFIRMED` / `REFUTED` / `CORRECTED` on the *same*
  issue, with evidence and the exact correction when the prior claim was
  path-mis-scoped.
- Posting may tolerate the expected ~12% hallucination rate — convergence corrects
  it. **Closing, merging, and `builder-ready` routing require a converged verdict**
  from an oppositional pass. A single scout's "dual evidence" is not enough if the
  evidence may be path-mis-scoped.
- **A real test is not enough if it exercises the wrong code path.** This is the
  #3106 lesson, and it is the same disease as NodeKind blindness: proof for one
  shape does not prove the semantic case.
- The coordinator does not centralize raw analysis. It reads the converged issue
  state, routes labels / follow-up work, and gates landing actions.

## The operating split

| Layer | Error tolerance | Authority |
|---|---:|---|
| Scout comments | High | May be wrong; useful raw convergence input |
| Oppositional comments | Medium | Corrects direction and narrows scope |
| Issue convergence | Low | Basis for planning and builder routing |
| Close / merge / builder-ready | Very low | Requires a converged verdict |

This avoids both bad extremes — too strict (scouts report to the coordinator, the
coordinator bottlenecks everything) and too loose (a single scout posts and closes;
wrong-path evidence removes real work). The right design is asymmetric:

```
parallelize posting
parallelize opposition
serialize landing
```

## The failure case that proves the design (#3106)

\#3106 was correctly filed as a post-edit staleness gap, then **wrongly closed**
because another scout cited a *real* test that covered the *wrong* path — the
diagnostics pull-paths, not the query providers (definition / references / hover /
completion), which have no generation check. The oppositional verifier caught the
path mismatch, refuted the already-done claim, and the issue was reopened from that
converged verdict.

That is the system working — but only because convergence happened on GitHub, not
in anyone's private context. The two durable lessons: a **close needs a converged
verdict** beyond the originator, and **"dual evidence" must be path-scope-matched**
(a test that exists but exercises different code paths is not evidence the issue's
paths are handled).

## Prompt templates

### Scout

```
Walk these issues one at a time. For each issue, research against current main and
post an audit-ready GitHub comment directly on the issue.

For each issue include:
1. Current state.
2. Evidence with file:line, tests, PRs, or commands.
3. Opposing checks: what could refute this, and what you checked.
4. Verdict: CONFIRMED / REFUTED / CORRECTED / ALREADY-DONE / DUPLICATE / NEEDS-REPRO.
5. Updated plan.
6. Acceptance criteria.
7. Residual uncertainty.

Do not save substantive findings for the final response. The final response should
only list issue URLs touched and any gh errors.
```

### Opposition

```
Try to refute the latest scout comment on this issue. Check whether cited tests
exercise the same code path, whether file:line evidence is current on main, whether
the issue is already fixed, and whether the proposed plan solves the stated problem.

Post your verdict directly on the issue:
CONFIRMED / REFUTED / CORRECTED

Include evidence and the exact correction if the prior claim was path-mis-scoped.
```

## See also

- [PIPELINE_GATES.md](PIPELINE_GATES.md) — the gate model the convergence cascade
  runs inside (accuracy-scout → research-verifier → oppositional-planner →
  architecture-reviewer → maintainer-issue → plan-reviewer).
- [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md) — live-truth signals vs.
  authoritative-only labels; the close/merge gate reads converged labels.
