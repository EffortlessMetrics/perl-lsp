# Adversarial Design Findings — work-23431b76

## Current Approach

The plan proposes to add bareword validation in `NodeKind::FunctionCall` for qualified names (containing `::`) when `strict_subs_mode` is active. The approach extracts the identifier part (e.g., `bar` from `Foo::bar`) and applies the same exclusion checks used for unqualified barewords: not a known function, not a builtin import, not in hash key context. If all exclusions pass, report `IssueKind::UnquotedBareword`.

The approach was chosen because it reuses existing infrastructure, is consistent with unqualified bareword handling, and avoids the complexity of cross-file module resolution.

## Alternative Approaches

### Alternative 1: Full Qualified-Name Resolution
Check if the entire `Foo::bar` path resolves to an existing symbol in the workspace index or on disk, rather than checking only the identifier part.

**Core idea:** When `strict_subs_mode` is active and `name.contains("::")`, look up the fully-qualified name in the workspace index or try to find the corresponding `.pm` file to determine if the package/subroutine actually exists.

**Why it might be better:**
- Would catch the actual semantic error: calling a function in a non-existent package
- Would NOT false-negative on `Foo::print()` where `Foo` doesn't exist (current approach misses this because `print` is a known builtin)
- Would provide more meaningful error messages ("Package Foo does not exist" vs "Bareword not allowed")
- Consistent with how `strict_subs` actually works in Perl — it's about *existence*, not *name shape*

**Why it might be worse:**
- Requires cross-file module existence checking, which has performance implications
- Would need to handle namespace cases: `Foo::Bar::baz` where `Foo/Bar.pm` may or may not be in `@INC`
- More complex implementation; potentially slower for large workspaces

**What it sacrifices:** The simplicity of "name shape" checking. The current approach is fast (just string matching) and doesn't require filesystem access or workspace index traversal.

---

### Alternative 2: Separate Qualified and Unqualified Handling with Different Semantics
Treat qualified barewords (`Foo::bar()`) differently from unqualified ones, with a separate code path that applies stricter or different validation rules.

**Core idea:** Rather than applying the same `is_known_function` / `has_builtin_import` exclusions that work for unqualified barewords, apply a stricter check for qualified names: if the qualified name cannot be resolved to an existing symbol, flag it.

**Why it might be better:**
- Would properly handle the `Foo::print()` case: even though `print` is a known builtin, `Foo::print()` is NOT the builtin and should be flagged if `Foo` doesn't exist
- Would avoid the semantic inconsistency where `FOO()` is always flagged but `Foo::bar()` where `bar` is a known builtin is never flagged
- Makes the diagnostic more useful: tells users their package-qualified call is invalid, not just that a bareword looks funny

**Why it might be worse:**
- Requires determining what "known" means for qualified names — existing infrastructure only handles unqualified names
- More complex: need to decide if we're checking the package, the function, or both
- Potential for false negatives if we can't definitively determine the package doesn't exist (e.g., runtime-loaded modules)

**What it sacrifices:** Consistency with unqualified bareword handling. The current approach is "consistent" in a shallow way (same exclusion pattern), but this sacrifices semantic correctness.

---

### Alternative 3: Pragmatic Middle Ground — Flag ALL Qualified Calls Under strict_subs
Since the whole point of `strict_subs` is to prevent typos and bad references, and since qualified calls are inherently more risky (you might mean a module but typo the name), simply flag ALL qualified function calls when `strict_subs` is active, with NO exclusions for known builtins.

**Core idea:** When `strict_subs_mode` is active and `name.contains("::")`, always report `IssueKind::UnquotedBareword` without checking if the identifier part is a known function.

**Why it might be better:**
- Simplest implementation: no need to check `is_known_function` or `has_builtin_import` for qualified names
- Most conservative: catches the most potential errors
- Eliminates the semantic inconsistency where `Foo::print()` (not a builtin call) isn't flagged
- The exclusion for builtins doesn't make semantic sense for qualified calls — `Foo::print()` is not calling the builtin `print`

**Why it might be worse:**
- False positives for legitimate qualified calls to actual defined functions in existing packages (e.g., `DBI::connect()`, `Moose::import()`)
- Requires users to either use indirect object notation, predeclare, or import explicitly to silence warnings
- May be considered too aggressive — many legitimate Perl programs use qualified function calls to core modules

**What it sacrifices:** Compatibility with existing patterns. Many Perl programs use qualified calls to core/stdlib modules (e.g., `Scalar::Util::blessed()`) which would be flagged.

---

## Strongest Argument Against Current Approach

The current approach's use of `is_known_function(identifier_part)` to suppress warnings is **semantically wrong for qualified calls**.

Consider: `Foo::print()` where `Foo` doesn't exist.

Under `strict_subs`, this is an error because `Foo` doesn't exist. The plan says "Foo::print() will NOT be flagged because `print` is a known builtin." But this reasoning is flawed:

- `Foo::print()` is NOT a call to the builtin `print` — it's a call to `print` in the `Foo` package
- The fact that `print` happens to be a known builtin function name is **irrelevant** to whether `Foo::print()` is valid
- The `is_known_function` check was designed for unqualified barewords like `print()` where the bareword itself IS the builtin — not for qualified calls

The same flaw applies to all builtins: `Foo::delete()` is NOT the hash builtin `delete`, it's a function call that requires `Foo` to exist. Using `is_known_function` to suppress these warnings means the fix will have false negatives for ANY qualified call to a builtin-named function, even when the package doesn't exist.

The consequence: a user with `use strict 'subs'` who types `Foo::prnt()` (typo of a non-builtin function) will get flagged, but `Foo::print()` (a genuinely invalid call to a non-existent package) will NOT be flagged — even though the latter is the more serious error.

## Recommended Action

**Modify** the current approach. The key change: do NOT apply the `is_known_function` exclusion when checking qualified barewords. Instead:

1. For unqualified barewords (`FOO`): keep using `is_known_function` to suppress warnings (current behavior — correct)
2. For qualified barewords (`Foo::bar`): apply ONLY the hash-key context and builtin-import exclusions. Do NOT suppress based on `is_known_function(identifier_part)` because that check is semantically incorrect for qualified calls.

This means:
- `Foo::bar()` where `Foo` doesn't exist → flagged (correct)
- `Foo::print()` where `Foo` doesn't exist → flagged (correct — current approach would NOT flag this)
- `print()` → NOT flagged (known builtin — correct)
- `Foo::print()` where `Foo` exports a `print` function → NOT flagged (this is a legitimate qualified call)

The exclusion for `has_builtin_import` should be kept because if someone has explicitly imported `print` into their package, `Foo::print()` might be legitimate even if `Foo` doesn't export it directly.

## Long-Term Cost Assessment

**If we do it the current way (checking `is_known_function` for qualified calls):**

- **6 months**: Users will file bugs like "Foo::print() isn't flagged even though Foo doesn't exist" and "Foo::delete() isn't flagged." These are true bugs that erode trust in the diagnostic. We'll either need to explain the limitation (leading to frustration) or add a "known limitation" disclaimer.

- **2 years**: The inconsistency becomes architectural debt. New engineers see `is_known_function` used in the `Identifier` handler and assume it applies correctly to `FunctionCall`. The semantic mismatch is buried in the ADR. Documentation will need to explain why qualified and unqualified barewords are treated differently. Feature requests to "fix the qualified bareword check" will pile up.

- **Long-term**: The fix partially addresses the reported bug but introduces a new category of false negatives that will themselves become bugs. The complexity of explaining the behavior (and its limitations) to users is ongoing cost. The "minimal change" becomes a maintenance burden as edge cases accumulate.

**If we modify as recommended (don't apply `is_known_function` to qualified calls):**

- **6 months**: Fewer false negatives. Users with `Foo::print()` to non-existent packages get warned correctly. Legitimate uses of qualified calls to non-builtin functions get warned, which may feel aggressive but is consistent with `strict_subs` intent.

- **2 years**: The behavior is more consistent and easier to explain. The exclusion logic is: hash-key context and explicit imports only — not "does the identifier part look like a known builtin." Simpler mental model.

- **Long-term**: Lower maintenance burden. No need to track "known builtin but qualified" exceptions. Diagnostic is more conservative but more correct, which is appropriate for a strict mode.
