# Adversarial Design Findings — work-aa31d3df

## Current Approach

The plan addresses a read-only end-of-session security sweep (issue #4141) that produced five findings (3 medium, 2 low) across eight security checks. Sprint-scale work is limited to adding a `RUSTSEC-2026-0097` ignore entry in `deny.toml`. Two backlog items are proposed: (1) regex-based identifier validation for `run_test_sub`'s `sub_name` parameter, and (2) renaming `validate_expression` to `reject_multiline_expression` with a doc comment. The SBOM regeneration task (Finding 8) was struck because the SBOM files don't exist in the repository. The `cargo machete` false positive is deferred pending CI confirmation.

---

## Alternative Approaches

### Alternative 1: Treat `run_test_sub` as a "sandboxed namespace" problem, not an identifier-validation problem

**Core idea:** The proposed regex validation (`^[A-Za-z_][A-Za-z0-9_]*(::[A-Za-z_][A-Za-z0-9_]*)*$`) is the wrong fix because `no strict 'refs'` + `&$sub()` after `do $file` can call ANY symbol table entry accessible after the file is loaded — including `CORE::system`, `CORE::open`, or subs defined in the loaded file — regardless of identifier shape. The real issue is that `do $file` loads the file into the current package namespace, making all its subs globally accessible. Instead of validating the identifier, wrap the execution in a safe Perl dispatcher or use a private package scope to isolate the loaded file.

**Why it might be better:**
- Addresses the actual attack surface rather than a narrow slice of it
- Would actually restrict `run_test_sub` to the intended contract (calling only the named sub)
- Doesn't add friction for valid use cases while genuinely hardening the security boundary
- A scoped namespace approach (e.g., loading the file into a temp package via `eval` in a dedicated package) is idiomatic Perl and doesn't require enumerating dangerous identifiers

**Why it might be worse:**
- Requires changing the Perl execution model, which is more complex than adding a regex check
- Could break edge cases if test files rely on specific package-level interactions
- Sprints to completion later than a simple regex

**What it sacrifices:**
- The simple, incremental nature of the proposed regex fix
- The ability to call `Package::Sub` style subs (which would be blocked by the namespace isolation approach unless explicitly allowed)

---

### Alternative 2: Explicit threat-model-first triage instead of sweep-based backlog

**Core idea:** Instead of treating all five findings as equally valid inputs to a sprint/backlog split, establish an explicit threat model — specifically: "is the LSP client trusted or untrusted?" — and use it to immediately deprioritize findings that only matter if the client is untrusted. If the client is trusted (which is the de facto assumption in most LSP implementations), then `run_test_sub` identifier validation and `validate_expression` hardening are defense-in-depth against a compromised-client scenario that may be out of scope for this project's threat model.

**Why it might be better:**
- Prevents spending security engineering budget on threats that the project doesn't actually defend against
- The `run_test_sub` finding disappears as a security issue if the client is trusted — it becomes a robustness improvement, not a security fix
- Forces the team to make an explicit decision about threat model, which is currently absent from the ADR
- Simpler triage: if SBOMs don't exist in the repo and aren't tracked, Finding 8 is N/A, not a sprint item

**Why it might be worse:**
- If the threat model IS "untrusted client" (e.g., LSP server exposed to third-party clients), this deprioritization is dangerous
- Removes the "just in case" defense-in-depth that some organizations prefer
- Requires an uncomfortable explicit decision that may reveal disagreements within the team

**What it sacrifices:**
- The comprehensive coverage of a sweep (every finding gets a backlog slot)
- Defense-in-depth for findings that don't meet the bar for immediate action

---

### Alternative 3: Investigate the SBOM claim before striking Finding 8

**Core idea:** Before striking Finding 8 as "factually incorrect," establish what the perl-lsp release process actually does with SBOMs. If SBOMs are generated on-demand during release and shipped as release artifacts (GitHub release assets, container image layers, etc.), then "the files don't exist in the repo" is true but the concern is still valid — there is still no CI gate to ensure SBOM freshness at release time. The fix is not "regenerate SBOMs" (since they're generated ad-hoc) but "add a CI check that runs `cargo sbom` and fails if the generated output would differ from what's in the last release."

**Why it might be better:**
- Preserves a legitimate supply-chain hygiene concern that was dismissed
- Would catch dependency changes that aren't reflected in any shipped artifact
- The CI gate (mentioned in the research analysis but then struck) might be the right answer, just not wired to the right trigger

**Why it might be worse:**
- If the release process truly doesn't use SBOMs at all, this is wasted effort
- SBOM generation can be slow and adds CI time

**What it sacrifices:**
- The clean "this is struck, don't revisit" decision

---

## Strongest Argument Against Current Approach

The proposed `run_test_sub` identifier validation is security theater that addresses the wrong layer of the attack. The research analysis explicitly states that `sub_name` is passed via `@ARGV` (not string interpolation), and the existing security test (`test_run_test_sub_subname_injection`) already proves that string-interpolation injection doesn't work — the literal string is looked up as a sub name and fails. The ADR's proposed fix (regex validation) addresses an injection vector that doesn't exist.

What DOES exist — and is acknowledged in the research analysis — is that `do $file` followed by `no strict 'refs'; &$sub()` allows calling any sub from the loaded file's namespace, including `CORE::system`. The ADR calls this "safe enough given the threat model" and puts it in the backlog, but offers no evidence for the threat model claim. If the LSP client is untrusted, `CORE::system` is callable with a valid Perl identifier like `CORE::system`. The regex doesn't stop it. The spec itself says "This does not eliminate the `no strict 'refs'` + `&$sub()` pattern" — so the fundamental unsafe pattern is acknowledged but deferred.

The ADR's own reasoning for calling this "safe enough" is that "run_tests already runs arbitrary code." But `run_tests` runs all top-level code in a file (which includes `BEGIN` blocks, `use` statements, etc.). `run_test_sub` is positioned as a more surgical alternative — it should only run one sub. If it can be made to call `CORE::system` via a valid identifier, it fails at its own security contract. Putting regex validation in the backlog without also noting that the underlying unsafe pattern is the real issue creates a false sense of progress.

---

## Recommended Action

**Modify** — two specific changes:

1. **Augment the `run_test_sub` backlog item with a note about the architectural concern.** The backlog item should explicitly state that regex validation is a partial mitigation (blocks obvious non-identifier injection attempts) but does not address the `no strict 'refs'` + `&$sub()` pattern. A separate, harder look at namespace isolation for `run_test_sub` should be scheduled. This prevents future maintainers from believing the regex check is a complete fix.

2. **Add an explicit threat model section to the ADR.** Before the backlog sprint items are refined, the ADR should state whether the LSP client is trusted or untrusted. This is the single most important input to evaluating whether `run_test_sub` identifier validation and `validate_expression` hardening belong in sprint (if untrusted client) or are best-effort defense-in-depth (if trusted client). Without this, the backlog is built on an unstated assumption that future maintainers will fill in differently.

**Keep as-is:** The `deny.toml` `RUSTSEC-2026-0097` fix. This is a well-scoped, low-risk change that fixes a broken CI gate. No changes needed.

**Strike cleanly:** The Finding 8 / SBOM task. The files don't exist in the repo and the "13-day drift" finding was based on a false premise. The alternative approach (investigation-first) is not worth the additional work given that the release process details are already known.

---

## Long-Term Cost Assessment

**If the regex `run_test_sub` validation lands without architectural follow-up:**

- **6 months:** A future maintainer sees the regex check and believes `run_test_sub` is now secure against injection. They extend the function or add new commands using the same pattern. The architectural debt compounds.
- **2 years:** The regex is maintained, extended, and documented as a security control. Nobody revisits the `no strict 'refs'` pattern because it's "already handled." If the threat model ever shifts (e.g., the LSP server is exposed to untrusted clients in a new deployment), the gap is invisible and the regex check creates false confidence.

**If the SBOM Finding 8 was struck prematurely and SBOMs are actually part of the release:**
- **6 months:** A release is made without verifying SBOM freshness. Downstream consumers receive SBOMs that don't reflect actual dependencies. Compliance audit fails. Retroactive investigation required.
- **2 years:** The release process is known to be "cargo sbom on demand" but nobody owns the freshness check. Each release requires manual SBOM verification or downstream complaints surface.

**If the threat model is never clarified:**
- **6 months:** Backlog refinement for `run_test_sub` proceeds under an implicit "untrusted client" assumption in some tickets and "trusted client" assumption in others. Security findings are inconsistently prioritized.
- **2 years:** The codebase accumulates defense-in-depth measures that protect against threats outside the actual threat model, while genuine threats from the actual threat model go unaddressed because they weren't "findings" in any sweep.
