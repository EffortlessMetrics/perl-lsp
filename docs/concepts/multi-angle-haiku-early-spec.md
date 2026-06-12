# Multi-Angle Haiku Early Spec

## The pattern

Before a builder writes a single line of implementation code, fan out several cheap model
passes -- each from a distinct analytical angle -- to build a rich, gap-free spec. The
passes are independent of each other (true fan-out) and can run in parallel. Their
outputs are then synthesized into the spec acceptance criteria before the builder starts.

The six angles below have been found to complement each other: each catches a different
category of spec gap that the others tend to miss.

## The six angles

**1. Hazard enumeration**
Scan the spec for applicable failure classes: ID/reference collision, bounds/overflow,
protocol-safety, scanner blindness, test-encodes-bug, coverage integrity. For each that
applies, add an explicit acceptance row and an adversarial test direction. See
hazard-class-invariants.md for the full class definitions.

**2. Contract pointers**
Identify every external contract the change must satisfy: a protocol spec section, a
published interface document, an API stability guarantee, a wire format version. For each,
add a citation (document name + section) to the spec, and a test that confirms the
relevant contract behavior.

**3. Prior-art / duplication check**
Scan the existing codebase for a function or module that already solves the same problem.
If found, the correct answer is "reuse and extend" rather than "add a parallel
implementation." If not found, confirm the new code location is the canonical place
for this kind of logic, so future code finds it rather than duplicating it.

**4. API shape**
Sketch the public interface (function signatures, message types, return types, error
variants) before implementation. The goal is to make the interface correctness properties
visible -- a caller that must supply a valid ID should receive a typed wrapper, not a raw
integer. Confirm the proposed interface makes the class-1 and class-2 hazards structurally
difficult to trigger.

**5. Test grid**
Enumerate the axes of variation in the change behavior (input size, state at call time,
protocol version, presence/absence of optional fields). Produce a matrix of test cases
covering the corners. This prevents the common pattern where tests cover the happy path
and one error path, leaving the adversarial corner uncovered.

**6. Blast radius**
Identify which other subsystems consume or depend on the change. For each dependent,
confirm it will still behave correctly after the change. Look for: callers that assume
a particular error type, downstream processors that assume a particular data shape, tests
in other crates that will need updating. A change with a small diff can have a large
blast radius if it modifies a widely consumed interface.

## Why it works

Each angle has a different failure mode it tends to catch:

- Hazard enumeration catches missing invariants before the builder hard-codes them wrong.
- Contract pointers catch protocol misunderstandings before code is written.
- Prior-art check catches duplicated logic before two implementations diverge.
- API shape catches interface designs that make hazard-class violations easy to trigger.
- Test grid catches coverage gaps before the reviewer has to request them.
- Blast radius catches unintended breakage of dependents before CI red.

Running all six on a warm cache (see cache-aware-agent-lanes.md) costs roughly the same
as one full cold analysis pass. The synthesis then produces a spec that is substantially
richer than a single-angle analysis would produce.

## Tradeoff / caution

The six-angle fan-out is overkill for genuinely trivial changes (a one-line fix to a
constant, a typo correction, a docs update). Apply it to changes that touch a
non-trivial surface: a new feature, a new subsystem, a change to a shared interface,
or a change that fixes a recurring bug class.

Not every angle will produce output for every change. A change with no external
protocol dependency produces nothing from the contract-pointers angle. That is expected
and acceptable -- the absence of findings is itself a useful signal.

## Relation to other patterns

- **Hazard-class invariants** (hazard-class-invariants.md) -- angle 1 (hazard
  enumeration) is a direct application; the six classes are the enumeration target.
- **Shift-left ladder** (shift-left-ladder.md) -- this pattern operates at the
  "spec acceptance criteria" rung; the rich spec produced here eliminates the need
  for deep-review to catch the classes that the spec now explicitly addresses.
- **Cache-aware agent lanes** (cache-aware-agent-lanes.md) -- the fan-out is cheapest
  when all six passes share the same warm context; run them as a batch within one session.
