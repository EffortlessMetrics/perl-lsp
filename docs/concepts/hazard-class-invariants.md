# Hazard-Class Invariants

## The pattern

Certain failure classes recur across unrelated changes and unrelated teams. Rather than
discovering each instance at deep-review, they can be front-loaded as explicit acceptance
criteria in the spec and as adversarial tests written before implementation begins.
This shifts catching from the expensive late rung (deep-review) to the cheap early rung
(spec + red-TDD).

The six classes below are the most commonly recurring in agentic, protocol-implementing,
or tool-infrastructure codebases. Each class has a generic invariant and an adversarial
test pattern. Apply them to the specific surface area of the change.

---

## Class 1: ID / Reference-Space Collision

**What recurs**: A subsystem allocates numeric identifiers in a range that overlaps with
identifiers already allocated by another subsystem. The overlap is silent at allocation
time but causes incorrect behavior when a lookup uses the wrong table.

**The invariant**: Newly allocated numeric ranges must be provably disjoint from all
existing ranges in the same process. Document the range boundaries as named constants.

**Adversarial test pattern**: Allocate one ID from the new range and one from each
adjacent existing range. Assert no two IDs are equal. Assert that a lookup with an ID
from range A into table B returns an error or empty result, not a stale entry.

**When to add this criterion**: Any change that introduces a new pool of numeric IDs
(references, handles, tokens, frame IDs, variable references, request IDs).

---

## Class 2: Bounds / Overflow

**What recurs**: A subsystem accepts a numeric index or count from an untrusted source
(a protocol message, a client request, user input) and uses it without range validation.
When the value is negative, zero, or beyond the current collection size, the behavior
is a panic, a wrap, or a silent wrong result.

**The invariant**: Client-supplied numeric indices use checked or saturating arithmetic.
Any value outside the valid range produces a safe empty/error result, never a crash or
undefined behavior.

**Adversarial test pattern**: Supply the minimum representable value, negative one,
zero, and one beyond the maximum valid index. Assert none panic. Assert each out-of-range
value returns the specified safe result, not an arbitrary value.

**When to add this criterion**: Any change that indexes into a collection using a value
from a protocol message, a configuration value, or user input.

---

## Class 3: Protocol-Safety (Invalid Input)

**What recurs**: A protocol handler receives a request with an unknown method, a missing
required field, a stale session reference, or a value from a session that has since been
invalidated. The handler crashes, panics, or silently evaluates with incorrect state
instead of returning an honest error.

**The invariant**: Invalid or unknown input always produces an honest empty or error
response. No code path returns a fabricated result, silently ignores the problem, or
crashes.

**Adversarial test pattern**: Send a request with each known-invalid input category:
unknown method name, missing required field, ID referencing a non-existent session,
ID invalidated by a lifecycle event. Assert each returns the expected error response.

**When to add this criterion**: Any change that adds or modifies a protocol handler,
request dispatcher, or session-stateful operation.

---

## Class 4: Scanner Literal / Comment Blindness

**What recurs**: A byte or character scanner that counts delimiters or special tokens
does not account for the same characters appearing inside string literals, character
literals, comments, or raw-string regions. The scanner miscounts and operates on the
wrong source extent.

**The invariant**: Every scanner that counts or matches delimiter characters must skip
content inside string literals, character literals, comments, and raw/verbatim string
regions. The scanner behavior on source without literals is not sufficient proof.

**Adversarial test pattern**: Supply input where the target delimiter appears exclusively
inside a string literal, a character literal, and a block comment. Assert the scanner
treats all three as if the character were absent. Also supply input where the character
appears both inside a literal and legitimately outside -- assert only the outside
occurrence is counted.

**When to add this criterion**: Any change that introduces or modifies a scanner that
parses source text character-by-character to count, locate, or extract structural tokens.

---

## Class 5: Test Encodes the Bug

**What recurs**: A pre-existing test was written against incorrect behavior and asserts
the defect as the expected result. A fix then has to fight the test suite: the fix is
correct but the test fails. This is discovered late, causing confusion about whether the
fix or the test is wrong.

**The invariant**: When modifying an existing test expected value or behavior, verify
the old assertion was NOT asserting the defect. Confirm the old expected value tested
correct behavior, not an artifact of the bug.

**Adversarial test pattern**: Before fixing the bug, read each test whose assertion you
are changing. For each: articulate what the old assertion tested and confirm it tested
correct behavior. If the old assertion tested incorrect behavior, mark the test as
"was testing the bug" in the commit message.

**When to add this criterion**: Any change that fixes a behavioral bug AND modifies an
existing test. The risk is highest when the bug incorrect output is plausible.

---

## Class 6: Coverage / Measurement Integrity

**What recurs**: A tool that transforms coverage data accidentally excludes or drops
production lines. The coverage percentage appears healthy but does not reflect actual
production-code coverage. The transformation correctness is not tested.

**The invariant**: Coverage transformations must never drop or exclude production source
lines. Test that a known production line number survives the transformation. Test that a
known excluded line does not appear in the output.

**Adversarial test pattern**: Construct a synthetic coverage record containing exactly
one production line and one test-only line. Run the transformation. Assert the production
line is present and the test-only line is absent. Assert the line count equals the number
of production lines in the input.

**When to add this criterion**: Any change that adds, modifies, or replaces a coverage
filter, post-processor, coverage routing script, or coverage gate.

---

## Using these classes in a spec

When writing a spec for a change, scan the six classes above. For each class where
the change touches the relevant surface area, add a row to the acceptance criteria:

  Hazard class: [name]
  Surface: [what the change touches that is in-scope for this class]
  Invariant: [the invariant, stated specifically for this change]
  Test: [what the adversarial test must assert]

A lightweight pre-build pass can then verify that the applicable classes have been
addressed before the builder writes any code.

## Relation to other patterns

- **Shift-left ladder** (shift-left-ladder.md) -- these six classes belong on the
  "spec acceptance criteria" rung; front-loading them moves catching from deep-review
  to pre-build.
- **Multi-angle early spec** (multi-angle-haiku-early-spec.md) -- one of the six
  fan-out angles is "hazard enumeration": systematically check each class for applicability.
