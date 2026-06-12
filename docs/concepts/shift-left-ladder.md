# Shift-Left Ladder

## The pattern

Every failure class has a set of rungs at which it can be caught. Earlier rungs are cheaper;
later rungs are more expensive and let more waste accumulate first. The ladder, from most
expensive/latest to cheapest/earliest:

```
Post-merge production incident    <- most expensive: users are affected
CI red on main                    <- expensive: every concurrent branch is blocked
Deep-review (expensive model)     <- late: builder has already written the code
Integration / adversarial tests   <- confirms the bug exists; per-PR recurring cost
Spec acceptance criteria          <- prevents the bug from being specified incorrectly
Mechanical lint / static analysis <- one-time elimination per class, near-zero marginal cost
Type / design elimination         <- structural impossibility; cheapest per occurrence
```

The goal is to climb to the highest feasible rung for every **recurring** failure class.
Recurring means the class appeared in at least two independent changes. A single occurrence
is a data point; two or more occurrences is a pattern worth eliminating structurally.

## Why it works

The key insight is that **tests are a recurring cost while types and lints are a one-time
cost**.

A test added to guard a bug class runs on every PR that touches the relevant code. That is
correct and valuable — but it means the test budget should be spent on classes that cannot
be eliminated structurally. For classes that *can* be eliminated (by changing the type, by
adding a lint, by redesigning the interface so the error is not representable), writing a
test is the inferior option: you pay for the test on every PR forever instead of paying
for the structural fix once.

Deep-review passes (expensive model, late in the pipeline) should be a **confirmation net**,
not the primary catcher of recurring classes. If the same class keeps showing up in
deep-review findings, that is evidence the class belongs on a higher rung.

## When to use

Apply this analysis when a failure class recurs across two or more independent changes. Ask:

1. What is the highest rung at which this class can be caught?
2. What is the cost of implementing the catch at that rung?
3. Is there a structural change (type, interface redesign, lint) that makes the failure
   *not representable* rather than *caught after the fact*?

For classes that cannot be eliminated structurally, shift them as high as possible into
spec acceptance criteria and adversarial tests — so cheap early passes (a lightweight
pre-build check) catch them before an expensive builder writes a line of code.

## Tradeoff / caution

Climbing the ladder is itself work. For a failure class that appeared exactly once, the cost
of structural elimination may exceed the expected future value. The shift-left investment
pays off only when the class is genuinely recurring.

Spec acceptance criteria and adversarial tests protect only the spec they live in. They do
not protect other areas of the codebase that carry the same hazard without the criteria.
Structural elimination (type-level or lint-level) is the only mechanism that provides
repo-wide coverage without per-spec maintenance.

## Relation to other patterns

- **Hazard-class invariants** (`hazard-class-invariants.md`) — a concrete set of recurring
  classes with their invariants and adversarial test patterns. Use that doc to identify which
  rung each class belongs on.
- **Multi-angle early spec** (`multi-angle-haiku-early-spec.md`) — how to populate the spec
  acceptance criteria rung cheaply before the builder starts.
