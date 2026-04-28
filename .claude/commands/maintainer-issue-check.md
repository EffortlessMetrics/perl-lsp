---
description: Maintainer vision (issue) step 2 — evaluate alignment with project vision
user-invocable: false
---

# Maintainer Issue: Check

Evaluate whether this issue aligns with perl-lsp's goals and current priorities.

## Synthesize with prior agents (do this BEFORE evaluating criteria)

You run after accuracy, research, oppositional, diaboli, and architecture. Their comments are on the issue. Your verdict must *engage* with theirs — your job is to add the project-vision lens, not echo what earlier agents already said.

For each prior agent comment:

- **accuracy-scout** — facts corrected? If claims were corrected, evaluate the issue against the *corrected* facts.
- **research-verifier** — external claims verified or debunked? If Perl/LSP/crate claims were debunked, the premise may have shifted.
- **oppositional-planner** — alternative scopes or approaches surfaced? If a scope-pivot was proposed, evaluate project-fit of the pivot, not just the original.
- **architecture-reviewer** — ALIGNED / CONCERN / FAIL? If ALIGNED, the structural case is made; your lens is direction-fit only.
- **advocatus-diaboli** — BUILD / DEFER / CLOSE? Diaboli's scope is PREMISE (is the work right in principle?); yours is PROJECT DIRECTION. If diaboli returned DEFER citing priority/timing, that's diaboli straying into *your* lane — your verdict should stand on project-direction grounds, not concur with diaboli's out-of-lane reasoning.

**If your honest verdict matches diaboli's, explain what additional project-vision angle you contribute.** If you can't name the additional angle, you're probably just echoing — re-examine.

**If the issue is part of a committed tracker / ADR / roadmap milestone:** the project's decision has been made. Start at ALIGNED and look for NEW information that would shift it — don't re-litigate the original decision from scratch.

## Evaluation criteria

Weight these against the committed roadmap. A work item implementing decided roadmap direction starts at ALIGNED; the criteria check whether new information changes that default.

1. **Roadmap alignment** — Does this advance a current priority from ROADMAP.md, or implement a decided tracker direction?
2. **User impact (in principle)** — Which Perl developers benefit? Is the benefit real, even if modest? (Not "is this higher impact than other work" — that's priority, handled by labels.)
3. **Maintenance fit** — Can the project sustain this surface long-term?
4. **Scope fit** — LSP server, or belongs in a separate tool?
5. **Framework scope** — Moose/Moo/Dancer/Mojo = in scope; niche <1K-user modules = out unless demonstrating general pattern
6. **Experimental features** — must be real (research-verified) and used before investing

*Not in this list intentionally:* "opportunity cost" / "more important than queued work." Priority is a queue-management concern handled via size + priority labels, not a verdict concern. We have capacity to queue valid lower-priority work.
