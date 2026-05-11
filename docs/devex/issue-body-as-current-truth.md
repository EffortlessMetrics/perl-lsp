# Issue body = current truth; comments = research log

## Rule

When a perl-lsp issue is filed from a wrong premise (stale checkout,
misread function signature, incorrect scope, etc.):

1. **Do NOT delete the history.** Wrong premises are durable project memory.
2. **Add a short "Current truth" / "Correction" section at the top of the
   issue body.** Replace the wrong-premise framing with the verified state.
3. **Move detailed research, prior false premise, and planning debate
   into comments.** The issue's comment trail carries the chain of
   evidence.
4. **Keep the main body small enough that a builder can implement from it
   without reading the whole comment log.** If a builder has to scroll
   through 800 lines of body to find the implementation map, the body is
   too big.
5. **Link the correction comment from the body** if the correction is
   materially important (e.g. the body looked very different an hour
   before).

For newly-filed issues (no prior premise to correct), the same body-shape
applies: compact current truth + implementation map, with research and
alternatives in comments rather than inlined.

Tracking: **#8554**.

---

## Suggested issue-body template

```markdown
## Current truth

One paragraph describing the current verified state.

## Required change

Small implementation map. File paths, function names, what to add/remove.

## Acceptance

Commands and expected behavior. Copy-pasteable.

## Notes

- Prior framing was superseded by <comment link>.
- Research log and alternatives are in comments.
```

---

## Why this shape

- **Builders read the body, not the comments.** A long body with
  superseded premises gives the builder the wrong context. A small body
  with current truth gives the builder exactly what they need.
- **Reviewers read the comments.** When a verifier checks why the scope is
  what it is, the comment trail is the durable evidence.
- **The body changes; the comments don't.** Editing the body in place
  preserves the URL and the issue number while letting the spec evolve.
  Comments preserve "how we got here."

The body is a moving picture of current spec. The comments are the still
photographs of the path that produced it.

---

## Worked example: #8485

#8485 was filed claiming `perl5lib_paths_for_completion` did not exist on
master. The local checkout was 5 commits behind `origin/master`. The
function existed; the bug was real but the scope was different. After
architectural review caught the stale-premise error:

- The body was **rewritten** to describe the corrected scope (the actual
  `PERL5LIB` completion-gate bug + startup env-strip work).
- The original wrong-premise framing was preserved in the issue's
  comment history.
- The implementation work that followed (PR #8493) referenced the
  corrected body, not the original framing.

Without the body rewrite, the builder spawned for #8485 would have read
the wrong premise and implemented the wrong thing. Without the comment
trail, a future reviewer would not be able to see why the scope changed.

---

## Anti-patterns

| Anti-pattern | Failure mode |
|---|---|
| **Delete-and-replace** — strip the wrong premise without preserving it | Future agents can't see what was wrong; the lesson is lost. |
| **Append-correction-to-bottom-of-body** — leave the wrong premise, add a "Correction:" section at the end | Body grows long; current truth is hard to find; builders inherit confusion. |
| **Comment-only correction** — body unchanged, correction lives only in a comment | First-time readers of the issue see the wrong thing. |
| **Multi-section body with "Phase 1: original / Phase 2: revised"** | Body becomes a changelog instead of a spec. |

---

## Scout agent guidance

Scouts (`scout`, `scout-parser`, `scout-lsp`, `scout-dap`) filing issues
should:

1. Use the template above.
2. Keep the body terse — body length should not grow over the issue's life.
3. Post research, alternatives considered, and decision rationale as
   comments, NOT inline in the body.
4. When the body needs correction (e.g. after research-verifier flags a
   stale-checkout claim), edit the body to current truth; post the
   correction note as a comment with `## Correction` heading.

---

## Related

- **#8554** — tracking issue.
- **`feedback_issue_correction_record.md`** (orchestrator memory) — the
  internal behavioral rule.
- **`feedback_stale_checkout.md`** (orchestrator memory) — the upstream
  prevention rule for the most common cause of bad premises.
- **#8546** — freshness-check tooling that catches the upstream cause.

## Claim boundary

Docs only. No code change, no test change, no behavior change. Does not
enforce — provides the rule and template. Future tooling (scout agent
prompt updates, GitHub issue templates, body-length lint) can enforce
later.
