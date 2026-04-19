# Justfile for perl-lsp development and CI workflows
# Usage: just <command>
# Install just: cargo install just

# Default recipe (show available commands)
default:
    @just --list

# ============================================================================
# Tiered CI Execution (works locally via Nix and in GitHub Actions)
# ============================================================================
#
# Tier hierarchy:
#   pr-fast    -> Fastest checks for every PR iteration (~1-2 min)
#   merge-gate -> Required before merge to master (~3-5 min)
#   nightly    -> Scheduled comprehensive tests (~15-30 min)
#
# Usage:
#   just pr-fast      # Quick PR validation
#   just merge-gate   # Full pre-merge validation
#   just ci-local     # Same as merge-gate, via Nix
#   nix develop -c just ci-gate  # Canonical local gate

# Helper to time a command and report duration
[private]
_timed name cmd:
    #!/usr/bin/env bash
    set -uo pipefail
    START=$(date +%s)
    echo ">>> Starting {{name}}..."
    {{cmd}}
    RC=$?
    END=$(date +%s)
    DURATION=$((END - START))
    if [ $RC -eq 0 ]; then
        echo "<<< {{name}} completed in ${DURATION}s"
    else
        echo "<<< {{name}} FAILED in ${DURATION}s (exit $RC)"
        exit $RC
    fi

# Tier: PR-fast (required for every PR iteration, must be fast ~1-2 min)
pr-fast: _check-tools-basic
    #!/usr/bin/env bash
    set -uo pipefail
    echo "=============================================="
    echo "  PR-FAST GATE (quick validation)"
    echo "=============================================="
    START=$(date +%s)
    just _timed "fmt-check" "just fmt-check" && \
    just _timed "clippy-core" "just clippy-core" && \
    just _timed "test-core" "just test-core" && \
    just _timed "publish-closure" "just ci-publish-closure" && \
    just _timed "publish-manifest-check" "just ci-publish-manifest-check" && \
    just _timed "layer-check" "just ci-layer-check" && \
    just _timed "published-crate-count" "just ci-published-crate-count"
    RC=$?
    END=$(date +%s)
    echo ""
    echo "=============================================="
    echo "  PR-fast gate complete (total: $((END - START))s)"
    echo "=============================================="
    exit $RC

# Pre-merge guard: verify a PR is not draft, has merge-ready label, and title has (#NNN)
# Usage: just pre-merge-check 3291
pre-merge-check NUMBER:
    bash scripts/pre-merge-check.sh {{NUMBER}}

# Tier: Merge-gate (required before merge to master ~3-5 min)
merge-gate: _check-tools-basic pr-fast
    #!/usr/bin/env bash
    set -uo pipefail
    echo "=============================================="
    echo "  MERGE GATE (full pre-merge validation)"
    echo "=============================================="
    START=$(date +%s)
    just _timed "clippy-full" "just clippy-full" && \
    just _timed "test-full" "just test-full" && \
    just _timed "lsp-smoke" "just lsp-smoke" && \
    just _timed "lsp-microcrates" "just ci-lsp-microcrates" && \
    just _timed "lsp-bdd" "just ci-lsp-bdd" && \
    just _timed "security-audit" "just security-audit" && \
    just _timed "ci-policy" "just ci-policy" && \
    just _timed "ci-v2-bundle-sync" "just ci-v2-bundle-sync" && \
    just _timed "ci-v2-parity" "just ci-v2-parity" && \
    just _timed "ci-lsp-def" "just ci-lsp-def" && \
    just _timed "ci-parser-features-check" "just ci-parser-features-check" && \
    just _timed "ci-features-invariants" "just ci-features-invariants"
    RC=$?
    END=$(date +%s)
    echo ""
    echo "=============================================="
    if [ $RC -eq 0 ]; then
        echo "  Merge gate PASSED (total: $((END - START))s)"
    else
        echo "  Merge gate FAILED (total: $((END - START))s)"
    fi
    echo "=============================================="
    exit $RC

# Tier: Nightly (scheduled, non-blocking comprehensive tests)
nightly: merge-gate
    #!/usr/bin/env bash
    set -uo pipefail
    echo "=============================================="
    echo "  NIGHTLY GATE (comprehensive validation)"
    echo "=============================================="
    START=$(date +%s)
    just _timed "ci-workspace-multiroot" "just ci-workspace-multiroot" && \
    just _timed "mutation-subset" "just mutation-subset" && \
    just _timed "fuzz-bounded" "just fuzz-bounded" && \
    just _timed "benchmarks" "just benchmarks"
    RC=$?
    END=$(date +%s)
    echo ""
    echo "=============================================="
    if [ $RC -eq 0 ]; then
        echo "  Nightly gate PASSED (total: $((END - START))s)"
    else
        echo "  Nightly gate FAILED (total: $((END - START))s)"
    fi
    echo "=============================================="
    exit $RC

# ============================================================================
# Individual Gate Targets
# ============================================================================

# Format check (fast fail)
fmt-check:
    @echo "Checking code formatting..."
    cargo xtask fmt --check
    @echo "Format check passed"

# Developer watch mode using bacon (optional local DX tool)
dev-watch:
    @if command -v bacon >/dev/null 2>&1; then \
        bacon; \
    else \
        echo "SKIP: bacon not installed (run: cargo install --locked bacon)"; \
        exit 1; \
    fi

# Developer watch mode focused on clippy for core crates
dev-watch-clippy:
    @if command -v bacon >/dev/null 2>&1; then \
        bacon clippy-core; \
    else \
        echo "SKIP: bacon not installed (run: cargo install --locked bacon)"; \
        exit 1; \
    fi

# Developer watch mode focused on fast core tests
dev-watch-tests:
    @if command -v bacon >/dev/null 2>&1; then \
        bacon test-core; \
    else \
        echo "SKIP: bacon not installed (run: cargo install --locked bacon)"; \
        exit 1; \
    fi

# Clippy core crates only (fast, for PR iterations)
clippy-core:
    @echo "Running clippy (core crates: perl-parser, perl-lexer)..."
    cargo clippy -p perl-parser -p perl-lexer --locked -- -D warnings -A missing_docs
    @echo "Clippy (core) passed"

# Clippy full workspace (thorough, for merge gate)
clippy-full:
    @echo "Running clippy (full workspace)..."
    cargo clippy --workspace --locked -- -D warnings -A missing_docs
    cargo clippy --workspace --bins --locked --no-deps -- -D clippy::unwrap_used -D clippy::expect_used
    @echo "Clippy (full) passed"

# Test core crates only (fast, for PR iterations)
test-core:
    @echo "Running tests (core crates: perl-parser, perl-lexer)..."
    cargo test -p perl-parser -p perl-lexer --lib --locked
    @echo "Tests (core) passed"

# Test full workspace (thorough, for merge gate)
test-full:
    @echo "Running tests (full workspace)..."
    RUST_TEST_THREADS=2 cargo test --workspace --lib --locked
    @echo "Tests (full) passed"

# LSP smoke test (deterministic, single-threaded)
lsp-smoke:
    @echo "Running LSP smoke tests..."
    cargo test -p perl-lsp-rs --test cli_smoke --locked -- --test-threads=1
    @echo "LSP smoke tests passed"

# Security audit (non-blocking, warns on issues)
# Temporary ignore for rand unsoundness advisory tracked in #4149.
security-audit:
    @echo "Running security audit..."
    @if command -v cargo-audit >/dev/null 2>&1; then \
        cargo audit 2>&1 || echo "Audit warnings (non-blocking)"; \
    else \
        echo "SKIP: cargo-audit not installed (run: cargo install cargo-audit)"; \
    fi

# Production hardening security scan
security-hardening:
    @echo "Running production hardening security scan..."
    @cargo xtask security-hardening

# Production hardening performance scan
performance-hardening:
    @echo "Running production hardening performance scan..."
    @cargo xtask performance-hardening

# Production hardening E2E validation
e2e-validation:
    @echo "Running production hardening E2E validation..."
    @cargo xtask e2e-validate

# Generate Homebrew formula + VS Code asset map from release checksums
inject-sha-assets version owner repo prefix checksums brew_out asset_map_out:
    @cargo xtask inject-sha-assets \
        --version "{{version}}" \
        --owner "{{owner}}" \
        --repo "{{repo}}" \
        --prefix "{{prefix}}" \
        --checksums "{{checksums}}" \
        --brew-out "{{brew_out}}" \
        --asset-map-out "{{asset_map_out}}"

# Generate Homebrew formula from release SHA256SUMS
update-homebrew version:
    @cargo xtask update-homebrew --version "{{version}}"

# Complete production hardening validation
production-hardening: security-hardening performance-hardening e2e-validation
    @echo "✅ Production hardening validation completed"
    @echo "📊 Check generated reports for detailed results"

# Production gates validation
production-gates-validation:
    @echo "Running production gates validation..."
    @cargo xtask production-gates-validation

# Complete Phase 6 production readiness validation
phase6-production-readiness: production-hardening production-gates-validation
    @echo "🎉 Phase 6 Production Hardening completed!"
    @echo "📋 All security, performance, and validation checks complete"
    @echo "🚀 Ready for v1.0 release validation"

# Generate SBOM in SPDX format
sbom-spdx:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Generating SBOM (SPDX format)..."
    cargo sbom --output-format spdx_json_2_3 > sbom-spdx.json
    echo "✓ Generated sbom-spdx.json"
    ls -lh sbom-spdx.json

# Generate SBOM in CycloneDX format
sbom-cyclonedx:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Generating SBOM (CycloneDX format)..."
    cargo sbom --output-format cyclone_dx_json_1_6 > sbom-cyclonedx.json
    echo "✓ Generated sbom-cyclonedx.json"
    ls -lh sbom-cyclonedx.json

# Generate both SBOM formats
sbom: sbom-spdx sbom-cyclonedx
    @echo "✓ Generated both SBOM formats"

# Verify SBOM files
sbom-verify: sbom
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Verifying SBOM files..."
    test -f sbom-spdx.json || (echo "ERROR: sbom-spdx.json not found" && exit 1)
    test -f sbom-cyclonedx.json || (echo "ERROR: sbom-cyclonedx.json not found" && exit 1)
    echo "✓ SBOM files verified"
    ls -lh sbom-*.json

# ============================================================================
# Heavy Jobs (label-gated in CI, for nightly tier)
# ============================================================================

# Mutation testing subset (bounded, ~5-10 min)
mutation-subset:
    @echo "Running mutation testing (subset)..."
    @if command -v cargo-mutants >/dev/null 2>&1; then \
        cargo mutants --workspace -j 2 --timeout 60 2>&1 || echo "Mutation testing completed (some mutants may survive)"; \
    else \
        echo "SKIP: cargo-mutants not installed (run: cargo install cargo-mutants)"; \
        echo "Running mutation regression harnesses instead..."; \
        just mutation-regression; \
    fi

# Mutation regression harness (fast fallback + PR guardrail)
mutation-regression:
    @echo "🧪 Running mutation regression harnesses..."
    @cargo test -p perl-parser --test mutation_hardening_tests
    @cargo test -p perl-parser --test parser_boolean_logic_mutation_hardening
    @cargo test -p perl-lsp-rs --test mutation_survivors_elimination
    @echo "✅ Mutation regression harnesses passed"

# Bounded fuzz run (quick fuzzing for CI/nightly)
fuzz-bounded:
    @echo "🔥 Running bounded fuzz testing (60 seconds per target)..."
    @cargo +nightly fuzz run builtin_functions -- -max_total_time=60 || echo "  Builtin functions fuzzing complete"
    @cargo +nightly fuzz run heredoc_parsing -- -max_total_time=60 || echo "  Heredoc fuzzing complete"
    @cargo +nightly fuzz run lsp_cancellation_registry -- -max_total_time=60 || echo "  LSP cancellation registry fuzzing complete"
    @cargo +nightly fuzz run lsp_navigation -- -max_total_time=60 || echo "  LSP navigation fuzzing complete"
    @cargo +nightly fuzz run parser_integration -- -max_total_time=60 || echo "  Parser integration fuzzing complete"
    @cargo +nightly fuzz run quote_operators -- -max_total_time=60 || echo "  Quote operators fuzzing complete"
    @cargo +nightly fuzz run substitution_parsing -- -max_total_time=60 || echo "  Substitution fuzzing complete"
    @cargo +nightly fuzz run unicode_positions -- -max_total_time=60 || echo "  Unicode positions fuzzing complete"
    @echo "✅ Fuzz testing complete"

# `bench` is the canonical benchmark target; keep this as a compatibility alias.
benchmarks: bench

# ============================================================================
# CI Aliases and Convenience Targets
# ============================================================================

# Canonical local merge gate via Nix (use before merge, not as the push hook)
ci-local:
    @echo "Running merge gate via Nix shell..."
    @if command -v nix >/dev/null 2>&1; then \
        nix develop -c just ci-gate; \
    else \
        echo "ERROR: Nix not found. Install Nix or run 'just ci-gate' directly."; \
        echo "  Install Nix: https://nixos.org/download.html"; \
        exit 1; \
    fi

# Checks that required tools (cargo, rustfmt, rustup, clippy, etc.) are installed.
# For workspace state corruption checks (core.bare, worktree leaks, stale branches),
# use `just doctor` instead.
# Developer environment diagnostics (onboarding/troubleshooting helper)
doctor-env:
    @echo "=============================================="
    @echo "  perl-lsp developer environment doctor"
    @echo "=============================================="
    @cargo xtask devex-doctor

# Short alias for the developer environment quick check
devex: doctor-env

# Run before any agent-spawning session. Safe to run repeatedly (idempotent).
# Checks: core.bare corruption (#3205), stale branches, worktree leaks, orphaned
# worktree dirs, pre-push hook, workspace cleanliness, master fast-forward state.
# Workspace health check — auto-detects and auto-fixes recurring state corruption
doctor:
    #!/usr/bin/env bash
    set -uo pipefail
    echo "🩺 perl-lsp doctor — workspace health check"
    echo

    issues=0
    fixed=0

    # ------------------------------------------------------------------
    # Locate main checkout. NOTE: on Git Bash for Windows, every external
    # command (dirname, sed, awk) costs 0.5-5s due to fork overhead. We use
    # pure bash parameter expansion wherever possible to keep the budget low.
    # ------------------------------------------------------------------
    # One rev-parse call. --git-common-dir is absolute when run from a worktree,
    # relative (".git") when run from the main checkout.
    # Avoid `git rev-parse --show-toplevel` — it takes ~10s on Windows with many
    # registered worktrees. Use $PWD as the fallback for the relative case.
    common_dir=$(git rev-parse --git-common-dir 2>/dev/null || true)
    if [ -z "$common_dir" ]; then
        echo "❌ not inside a git repository"
        exit 1
    fi
    # Resolve to an absolute path with bash builtins (avoid `dirname`).
    case "$common_dir" in
        /*|[A-Za-z]:*) abs_common="$common_dir" ;;
        *) abs_common="$PWD/$common_dir" ;;
    esac
    # main_root = parent of the common .git directory (bash builtin).
    main_root="${abs_common%/*}"
    main_git_dir="$abs_common"
    # Detect whether we are running from the main checkout or a worktree.
    # git --git-common-dir returns a RELATIVE path (".git") only from the main
    # checkout; from any worktree it returns an absolute path. This is reliable
    # and avoids path-prefix comparisons that break when worktrees are nested
    # inside the main checkout directory (e.g. .claude/worktrees/agent-*).
    case "$common_dir" in
        /*|[A-Za-z]:*) running_in_main=0 ;;
        *) running_in_main=1 ;;
    esac

    # ------------------------------------------------------------------
    # Check 1: core.bare = true corruption (#3205)
    # ------------------------------------------------------------------
    bare_value=$(git --git-dir="$main_git_dir" config --local --get core.bare 2>/dev/null || true)
    if [ "$bare_value" = "true" ]; then
        echo "⚠️  git config core.bare = true detected in main checkout (#3205)"
        if git --git-dir="$main_git_dir" config --local --unset core.bare 2>/dev/null; then
            echo "   Auto-fixed: unset core.bare"
            fixed=$((fixed + 1))
        else
            echo "   Fix: git --git-dir=\"$main_git_dir\" config --local --unset core.bare"
            issues=$((issues + 1))
        fi
    else
        echo "✅ git config core.bare unset (was OK)"
    fi

    # ------------------------------------------------------------------
    # One batched ref dump — used by both stale-branch and master-FF checks.
    # ------------------------------------------------------------------
    refs_dump=$(git -C "$main_root" for-each-ref \
        --format='%(refname:short)|%(upstream:short)' \
        refs/heads/ refs/remotes/ 2>/dev/null || true)

    # Parse refs_dump in pure bash (no awk subshell) into two arrays.
    declare -A remote_ref_set=()
    declare -a branch_upstreams=()
    while IFS= read -r line; do
        [ -z "$line" ] && continue
        ref="${line%%|*}"
        up="${line#*|}"
        case "$ref" in
            origin/*) remote_ref_set["$ref"]=1 ;;
            *)
                if [ -n "$up" ] && [ "$up" != "$ref" ]; then
                    branch_upstreams+=("$ref|$up")
                fi
                ;;
        esac
    done <<<"$refs_dump"

    # ------------------------------------------------------------------
    # Check 2: Stale local branches (upstream gone)
    # NOTE: deliberately no `git remote prune` — that contacts the remote.
    # Run `git fetch --prune` manually if you want fresh remote state.
    # ------------------------------------------------------------------
    stale_list=()
    for entry in "${branch_upstreams[@]}"; do
        b="${entry%%|*}"
        u="${entry#*|}"
        case "$b" in master|main|HEAD) continue ;; esac
        if [ -z "${remote_ref_set[$u]:-}" ]; then
            stale_list+=("$b")
        fi
    done
    if [ "${#stale_list[@]}" -eq 0 ]; then
        echo "✅ no stale local branches found"
    else
        echo "⚠️  ${#stale_list[@]} stale local branches (upstream gone)"
        for b in "${stale_list[@]}"; do
            echo "   $b"
        done
        echo "   Fix: git branch -D <branch>   # only after confirming the PR is merged"
        issues=$((issues + 1))
    fi

    # ------------------------------------------------------------------
    # One batched dirty-status check on the main checkout.
    # Used by both leak detection (Check 3) and workspace-clean (Check 6).
    # ------------------------------------------------------------------
    main_dirty_full=$(git -C "$main_root" status --porcelain --untracked-files=no 2>/dev/null || true)
    declare -A main_dirty_set=()
    while IFS= read -r line; do
        [ -z "$line" ] && continue
        # Strip the two-char status prefix and any leading spaces.
        path="${line:3}"
        # Handle rename entries: "old -> new" — take the new path.
        case "$path" in *" -> "*) path="${path##* -> }" ;; esac
        main_dirty_set["$path"]=1
    done <<<"$main_dirty_full"

    # ------------------------------------------------------------------
    # Check 3: Worktree file leaks
    # Only walk worktrees if the main checkout has tracked dirt.
    # ------------------------------------------------------------------
    leak_count=0
    leak_report=""
    if [ -d "$main_root/.claude/worktrees" ] && [ "${#main_dirty_set[@]}" -gt 0 ]; then
        for wt_dir in "$main_root"/.claude/worktrees/agent-*; do
            [ -d "$wt_dir" ] || continue
            wt_status=$(git -C "$wt_dir" status --porcelain --untracked-files=no 2>/dev/null || true)
            [ -z "$wt_status" ] && continue
            wt_overlaps=()
            while IFS= read -r line; do
                [ -z "$line" ] && continue
                p="${line:3}"
                case "$p" in *" -> "*) p="${p##* -> }" ;; esac
                if [ -n "${main_dirty_set[$p]:-}" ]; then
                    wt_overlaps+=("$p")
                fi
            done <<<"$wt_status"
            if [ "${#wt_overlaps[@]}" -gt 0 ]; then
                leak_count=$((leak_count + ${#wt_overlaps[@]}))
                wt_name="${wt_dir##*/}"
                leak_report+=$'\n'"   from $wt_name:"
                for o in "${wt_overlaps[@]}"; do
                    leak_report+=$'\n'"     $o"
                done
            fi
        done
    fi
    if [ "$leak_count" -eq 0 ]; then
        echo "✅ no worktree file leaks"
    else
        echo "⚠️  $leak_count worktree file leaks (files modified in main + agent worktree)"
        echo "$leak_report"
        echo "   Fix: cd \"$main_root\" && git restore <file>   # discard leaked changes"
        issues=$((issues + 1))
    fi

    # ------------------------------------------------------------------
    # Check 4: Orphaned worktree directories
    # ------------------------------------------------------------------
    orphan_count=0
    orphans=""
    if [ -d "$main_root/.claude/worktrees" ]; then
        # Build a set of registered worktree paths (forward-slash normalized).
        registered_dump=$(git -C "$main_root" worktree list --porcelain 2>/dev/null || true)
        declare -A registered_set=()
        while IFS= read -r line; do
            case "$line" in
                "worktree "*)
                    p="${line#worktree }"
                    p="${p//\\//}"
                    registered_set["$p"]=1
                    ;;
            esac
        done <<<"$registered_dump"
        for wt_dir in "$main_root"/.claude/worktrees/*/; do
            [ -d "$wt_dir" ] || continue
            wt_dir_clean="${wt_dir%/}"
            wt_dir_norm="${wt_dir_clean//\\//}"
            if [ -z "${registered_set[$wt_dir_norm]:-}" ]; then
                orphan_count=$((orphan_count + 1))
                orphans+=$'\n'"   $wt_dir_clean (no corresponding git worktree)"
            fi
        done
    fi
    if [ "$orphan_count" -eq 0 ]; then
        echo "✅ no orphaned worktree directories"
    else
        echo "⚠️  $orphan_count orphaned worktree directories under .claude/worktrees/"
        echo "$orphans"
        echo "   Fix: git worktree prune; rm -rf <dir>   # only after confirming it is truly orphaned"
        issues=$((issues + 1))
    fi

    # ------------------------------------------------------------------
    # Check 5: pre-push hook installed
    # ------------------------------------------------------------------
    hook_path="$main_git_dir/hooks/pre-push"
    expected_hook="$main_root/hooks/pre-push"
    if [ -f "$hook_path" ] && [ -x "$hook_path" ]; then
        if [ -f "$expected_hook" ] && diff -q \
            <(awk '{ sub(/\r$/, ""); lines[NR]=$0 } END { last=NR; while (last > 0 && lines[last] == "") last--; for (i = 1; i <= last; i++) print lines[i] }' "$hook_path") \
            <(awk '{ sub(/\r$/, ""); lines[NR]=$0 } END { last=NR; while (last > 0 && lines[last] == "") last--; for (i = 1; i <= last; i++) print lines[i] }' "$expected_hook") >/dev/null; then
            echo "✅ pre-push hook installed and current"
        elif [ -f "$expected_hook" ]; then
            echo "⚠️  pre-push hook installed but stale: $hook_path"
            echo "   Fix: cargo xtask ci-hygiene install-githooks   # refresh from hooks/pre-push"
            issues=$((issues + 1))
        else
            echo "✅ pre-push hook installed"
        fi
    elif [ -f "$hook_path" ]; then
        echo "⚠️  pre-push hook present but not executable: $hook_path"
        echo "   Fix: chmod +x \"$hook_path\""
        issues=$((issues + 1))
    else
        echo "⚠️  pre-push hook not installed"
        echo "   Fix: cargo xtask ci-hygiene install-githooks   # or: bash scripts/install-githooks.sh"
        issues=$((issues + 1))
    fi

    # ------------------------------------------------------------------
    # Check 6: Workspace clean
    # When run from the main checkout, reuse the cached status from Check 3.
    # When run from a worktree, query the worktree's own status.
    # ------------------------------------------------------------------
    if [ "$running_in_main" = "1" ]; then
        dirty="$main_dirty_full"
    else
        dirty=$(git status --porcelain --untracked-files=no 2>/dev/null || true)
    fi
    if [ -z "$dirty" ]; then
        echo "✅ workspace clean"
    else
        # Count lines without spawning wc.
        dirty_count=0
        while IFS= read -r _line; do
            [ -z "$_line" ] && continue
            dirty_count=$((dirty_count + 1))
        done <<<"$dirty"
        echo "⚠️  workspace has $dirty_count uncommitted changes"
        # Print up to the first 10 dirty entries.
        shown=0
        while IFS= read -r line; do
            [ -z "$line" ] && continue
            echo "   $line"
            shown=$((shown + 1))
            [ "$shown" -ge 10 ] && break
        done <<<"$dirty"
        if [ "$dirty_count" -gt 10 ]; then
            echo "   ... and $((dirty_count - 10)) more"
        fi
        echo "   Fix: review with 'git status', then commit or 'git restore' as appropriate"
        issues=$((issues + 1))
    fi

    # ------------------------------------------------------------------
    # Check 7: Master is fast-forward-able
    # Use the cached refs_dump to check origin/master existence (no extra call).
    # ------------------------------------------------------------------
    if [ -n "${remote_ref_set[origin/master]:-}" ]; then
        behind=$(git rev-list --count HEAD..origin/master 2>/dev/null || echo 0)
        if [ "$behind" = "0" ]; then
            echo "✅ master is up to date with origin/master"
        else
            echo "⚠️  HEAD is $behind commits behind origin/master"
            echo "   Fix: git pull --ff-only"
            issues=$((issues + 1))
        fi
    else
        echo "⚠️  origin/master ref not found (cannot check fast-forward state)"
        echo "   Fix: git fetch origin master:refs/remotes/origin/master"
        issues=$((issues + 1))
    fi

    echo
    if [ "$issues" -eq 0 ] && [ "$fixed" -eq 0 ]; then
        echo "✅ all checks passed"
    else
        echo "$issues issues found, $fixed auto-fixed"
    fi
    exit 0

# Targeted checks for changed crates (fast feedback for active branch)
devex-targeted base='origin/master' mode='all':
    @echo "Running targeted checks (base={{base}}, mode={{mode}})..."
    @cargo xtask targeted-checks --base "{{base}}" --mode "{{mode}}"

# Tool availability check (basic tools for PR-fast)
[private]
_check-tools-basic:
    #!/usr/bin/env bash
    set -euo pipefail
    MISSING=""
    if ! command -v cargo >/dev/null 2>&1; then MISSING="$MISSING cargo"; fi
    if ! command -v rustfmt >/dev/null 2>&1; then MISSING="$MISSING rustfmt"; fi
    if ! cargo nextest --version >/dev/null 2>&1; then MISSING="$MISSING cargo-nextest"; fi
    if [ -n "$MISSING" ]; then
        echo "ERROR: Missing required tools:$MISSING"
        echo "  Install Rust: https://rustup.rs"
        echo "  Install nextest: cargo install cargo-nextest --locked"
        exit 1
    fi
    cargo xtask check-toolchain

# ============================================================================
# CI Validation Commands (Issue #211)
# ============================================================================

# MSRV: Rust 1.92 (for OpenAI Codex compatibility)
# The rust-toolchain.toml pins to 1.92.0, so standard commands use MSRV by default.
# Use these recipes to explicitly verify MSRV compliance:

# Phase 0: publish receipts to review/receipts/YYYY-MM-DD/
receipts date='':
    #!/usr/bin/env bash
    set -euo pipefail
    d="{{date}}"
    if [ -z "$d" ]; then
        cargo xtask publish-receipts
    else
        cargo xtask publish-receipts "$d"
    fi

# Issue #211: measure CI lane runtimes locally (baseline before cleanup)
ci-measure:
    @echo "Measuring CI lane runtimes..."
    @cargo xtask ci-measure

# ============================================================================
# UX Regression Tests (first-5-minutes user experience)
# ============================================================================

# Fast merge gate on MSRV (~2-5 min) - proves 1.92 compatibility
ci-gate-msrv:
    @echo "🚪 Running fast merge gate on MSRV (Rust 1.92)..."
    @RUSTUP_TOOLCHAIN=1.92.0 just ci-gate

# Low-memory merge gate - for constrained environments (WSL, CI runners, low-RAM)
# Forces single-threaded builds/tests to prevent OOM crashes
# Key fixes: unset RUSTC_WRAPPER (not empty), --no-deps on clippy
ci-gate-low-mem:
    @echo "🚪 Running low-memory merge gate (sequential, single-threaded)..."
    @echo "   Using CARGO_BUILD_JOBS=1, RUST_TEST_THREADS=1, RUSTC_WRAPPER unset"
    @env -u RUSTC_WRAPPER CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 PROPTEST_CASES=32 \
        sh -c 'just ci-workflow-audit && \
        just ci-check-no-nested-lock && \
        just ci-format && \
        just ci-docs-check && \
        echo "🔍 Running clippy (single-threaded, no-deps)..." && \
        cargo clippy --workspace --lib --locked --no-deps -j1 -- -D warnings -A missing_docs && \
        cargo clippy --workspace --bins --locked --no-deps -j1 -- -D clippy::unwrap_used -D clippy::expect_used && \
        just ci-forbid-fatal && \
        echo "🧪 Running library tests (single-threaded)..." && \
        cargo test --workspace --lib --locked -j1 -- --test-threads=1 && \
        just ci-policy && \
        just ci-lsp-def && \
        just ci-parser-features-check && \
        just ci-features-invariants'
    @echo "✅ Low-memory merge gate passed!"

# Full CI on MSRV (~10-20 min) - proves 1.92 compatibility for releases
ci-full-msrv:
    @echo "🚀 Running full CI on MSRV (Rust 1.92)..."
    @RUSTUP_TOOLCHAIN=1.92.0 just ci-full

# Check for nested Cargo.lock files (footgun prevention)
ci-check-no-nested-lock:
    @echo "🔒 Checking for nested Cargo.lock files..."
    @if find . \( -path './target' -o -path './.runs' -o -path './archive' -o -path './fuzz' -o -path './tree-sitter-perl' -o -path './.claude/worktrees' \) -prune -o -name 'Cargo.lock' -type f -print 2>/dev/null | \
        grep -v '^\./Cargo\.lock$' | grep -q .; then \
        echo "❌ ERROR: Nested Cargo.lock detected! Run gates from repo root only."; \
        find . \( -path './target' -o -path './.runs' -o -path './archive' -o -path './fuzz' -o -path './tree-sitter-perl' -o -path './.claude/worktrees' \) -prune -o -name 'Cargo.lock' -type f -print 2>/dev/null | \
            grep -v '^\./Cargo\.lock$'; \
        exit 1; \
    fi
    @echo "✅ No nested lockfiles"

# Audit workflows for ungated expensive jobs
ci-workflow-audit:
    @cargo xtask ci-audit-workflows

# Fast merge gate (~2-5 min) - REQUIRED for all merges
# The pre-push hook runs `just pr-fast`; this is the broader local merge gate.
ci-gate:
    @echo "Running fast merge gate..."
    just ci-workflow-audit && \
    just ci-check-no-nested-lock && \
    just ci-format && \
    just ci-docs-check && \
    just status-check && \
    just ci-clippy-gate && \
    just ci-unwrap-panic-ratchet && \
    just ci-unsafe-ratchet && \
    just ci-forbid-fatal && \
    just ci-test-lib && \
    just common-corpus-check && \
    just ci-policy && \
    just ci-v2-bundle-sync && \
    just ci-v2-parity && \
    just ci-lsp-def && \
    just ci-lsp-smoke-e2e && \
    just ci-lsp-microcrates && \
    just ci-lsp-bdd && \
    just ci-semantic-frameworks && \
    just ci-dap-smoke-e2e && \
    just ci-parser-features-check && \
    just ci-features-invariants && \
    just hook-check && \
    just hook-registry-check && \
    just hook-tests && \
    just ci-publish-closure && \
    just ci-publish-manifest-check && \
    just ci-layer-check && \
    just ci-published-crate-count
    # @START=$$(date +%s); \

# Gate runner with receipt output (Issue #210)
# Uses xtask gates for structured gate execution with receipt generation
gates tier='merge-gate' *args='':
    @echo "🧾 Running gate runner (tier: {{tier}})..."
    cargo xtask gates --tier {{tier}} --receipt {{args}}

# Run gates with JSON output (for CI)
gates-json tier='merge-gate':
    @cargo xtask gates --tier {{tier}} --format json --receipt

# List available gates
gates-list:
    @cargo xtask gates --list

# Run old shell-based gate runner (deprecated, kept for compatibility)
gates-legacy:
    @echo "🧾 Running legacy gate runner..."
    @cargo xtask gates --tier merge-gate --receipt

# Full CI pipeline (~10-20 min) - RECOMMENDED for large changes
ci-full:
    @echo "🚀 Running full CI pipeline..."
    @just ci-format
    @just ci-docs-check
    @just ci-clippy
    @just ci-test-core
    @just ci-test-lsp
    @just ci-lsp-microcrates
    @just ci-lsp-bdd
    @just ux-tests
    @just ci-docs
    @echo "✅ Full CI passed!"

# Local CI parity with .github/workflows/ci.yml (legacy alias)
# Prefer: nix develop -c just ci-gate
ci-local-full:
    @just ci-full

# Format check (fast fail)
ci-format:
    @echo "📝 Checking code formatting..."
    cargo xtask fmt --check
    @echo "✅ Format check passed"

# Clippy lint (catches common issues, allow missing_docs during systematic resolution)
ci-clippy:
    @echo "🔍 Running clippy (all targets)..."
    cargo clippy --workspace --all-targets -- -D warnings -A missing_docs
    @echo "✅ Clippy passed"

# Clippy libraries only (fast, for merge gate)
ci-clippy-lib:
    @echo "🔍 Running clippy (libraries only)..."
    cargo clippy --workspace --lib --locked -- -D warnings -A missing_docs
    @echo "✅ Clippy (lib) passed"

# Clippy production unwrap/expect gate (Issue #143) - prevents panic-prone code in shipped binaries
clippy-prod-no-unwrap:
    @echo "🔒 Enforcing no unwrap/expect in production code..."
    cargo clippy --workspace --lib --bins --no-deps -- -D clippy::unwrap_used -D clippy::expect_used

# Clippy NO UNWRAP ALL gate - enforces zero unwrap/expect everywhere
clippy-no-unwrap-all:
    @echo "🔒 Enforcing no unwrap/expect everywhere (including tests)..."
    cargo clippy --workspace --all-targets -- -D clippy::unwrap_used -D clippy::expect_used
    @echo "✅ Production code is panic-safe (no unwrap/expect)"

# Consolidated clippy gate (Issue #1909): two passes replacing the three-pass ci-gate sequence.
# Pass 1: lib warnings + unwrap/expect in one invocation (avoids separate ci-clippy-lib + clippy-prod-no-unwrap)
# Pass 2: bins-only unwrap/expect (clippy-no-unwrap-all used --all-targets which trips on test expect() in tests)
ci-clippy-gate:
    @echo "🔍 Running consolidated clippy gate (libs + bins)..."
    cargo clippy --workspace --lib --locked -- -D warnings -A missing_docs -D clippy::unwrap_used -D clippy::expect_used
    cargo clippy --workspace --bins --locked -- -D clippy::unwrap_used -D clippy::expect_used
    @echo "✅ Clippy gate passed"

# Unwrap/panic-family ratchet (production source only)
ci-unwrap-panic-ratchet:
    @echo "🛡️  Checking unwrap/panic-family ratchet..."
    @cargo xtask ci-hygiene check-unwraps-prod
    @echo "✅ Unwrap/panic-family ratchet passed"

# Unsafe syntax ratchet (production source only)
ci-unsafe-ratchet:
    @echo "🛡️  Checking unsafe syntax ratchet..."
    @cargo xtask ci-hygiene check-unsafe-prod
    @echo "✅ Unsafe syntax ratchet passed"

# Forbid fatal constructs gate - catches abort/exit/panic that Clippy misses
ci-forbid-fatal:
    @echo "🚫 Checking for forbidden fatal constructs..."
    @cargo xtask forbid-fatal-constructs -- --verbose
    @echo "✅ No forbidden fatal constructs"

ci-publish-closure:
    @echo "🔐 Checking publish-closure transitive deps..."
    @cargo xtask publish-closure
    @echo "✅ Publish-closure check passed"

# Layer-check gate: enforce crate dependency direction constraints
# Normal (runtime) deps only; dev-deps may cross layers freely.
ci-layer-check:
    @echo "🧱 Checking crate layer constraints..."
    @cargo xtask layer-check
    @echo "✅ Layer-check passed"

# Ratchet: published crate count must not increase above baseline (see #4416)
ci-published-crate-count:
    @echo "🧮 Checking published-crate count ratchet..."
    @cargo xtask published-crate-count
    @echo "✅ Published-crate count ratchet passed"

# Offline manifest validation: allowlist drift + LICENSE present (see #4499)
ci-publish-manifest-check:
    @echo "Checking publish manifest (allowlist drift + LICENSE)..."
    @cargo xtask publish-manifest-check
    @echo "Publish manifest check passed"

# Core tests (fast, essential)
ci-test-core:
    @echo "🧪 Running core tests..."
    cargo test --workspace --lib --bins
    @echo "✅ Core tests passed"

# Library tests only (fastest, for merge gate)
ci-test-lib:
    @echo "🧪 Running library tests..."
    cargo nextest run --workspace --lib --locked --profile ci
    @echo "✅ Library tests passed"

# V2 bundle sync guard (in-crate v2 files must match extracted perl-parser-pest v2 files)
ci-v2-bundle-sync:
    @echo "🔍 Checking v2 bundle sync..."
    cargo xtask ci-hygiene check-v2-bundle-sync
    @echo "✅ V2 bundle sync check passed"

# V2 parser parity guard (in-crate v2 vs extracted perl-parser-pest v2)
ci-v2-parity:
    @echo "🧪 Running v2 parity corpus check..."
    cargo run --locked -p xtask --features legacy -- corpus --scanner v2-parity
    @echo "✅ V2 parity corpus check passed"

# Targeted parser/DAP verification (low-memory, for heredoc/breakpoint changes)
# Key fixes: unset RUSTC_WRAPPER (not empty), --no-deps on clippy, targeted tests
ci-test-parser-dap:
    @echo "🎯 Running targeted parser/DAP tests (single-threaded)..."
    @env -u RUSTC_WRAPPER CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 \
        sh -c 'echo "📦 Building perl-parser-core..." && \
        cargo build -p perl-parser-core --lib -j1 && \
        echo "🧪 Running perl-parser heredoc tests..." && \
        cargo test -p perl-parser -j1 -- --test-threads=1 heredoc && \
        echo "🧪 Running DAP breakpoint tests..." && \
        cargo test -p perl-dap --test dap_breakpoint_matrix_tests -j1 -- --test-threads=1 && \
        echo "🔍 Running clippy on affected crates (no-deps)..." && \
        cargo clippy -p perl-parser-core -p perl-parser -p perl-dap --lib --no-deps -j1 -- -D warnings'
    @echo "✅ Parser/DAP tests passed"

# LSP integration tests (with adaptive threading)
ci-test-lsp:
    @echo "🔌 Running LSP integration tests..."
    RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_comprehensive_e2e_test -- --test-threads=2
    @echo "✅ LSP tests passed"

# LSP semantic definition tests (semantic-aware go-to-definition)
ci-lsp-def:
    @echo "🔎 Running LSP semantic definition tests..."
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-lsp-rs --test semantic_definition -- --test-threads=1
    @echo "✅ LSP semantic definition tests passed"

# LSP process-level smoke receipt (initialize/open/completion/hover/definition/shutdown)
ci-lsp-smoke-e2e:
    @echo "💨 Running LSP stdio smoke E2E test..."
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-lsp-rs --test lsp_smoke_e2e -- --test-threads=1
    @echo "✅ LSP smoke E2E passed"

# UX regression test harness — systematic first-5-minutes scenario testing.
# Runs all default `perl-lsp-ux-tests` scenarios (currently 17 scenario files).
# `ux-tests-full` adds the integration-only 10k-line large-file coverage.
ux-tests:
    @echo "Running UX regression test harness (base scenarios)..."
    @env -u RUSTC_WRAPPER CARGO_BUILD_JOBS=1 \
        cargo build -p perl-lsp-rs --bin perl-lsp
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        PERL_LSP_BIN={{justfile_directory()}}/target/debug/perl-lsp \
        cargo test -p perl-lsp-ux-tests -- --test-threads=1
    @echo "UX tests passed"

# UX regression test harness — full suite including the integration-only
# 10k-line large-file scenario. Slower (~5-10 min). Run before releases or
# after large LSP changes.
ux-tests-full:
    @echo "Running UX regression test harness (full, including large-file)..."
    @env -u RUSTC_WRAPPER CARGO_BUILD_JOBS=1 \
        cargo build -p perl-lsp-rs --bin perl-lsp
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        PERL_LSP_BIN={{justfile_directory()}}/target/debug/perl-lsp \
        cargo test -p perl-lsp-ux-tests --features integration-test -- --test-threads=1
    @echo "UX tests (full) passed"

# @INC consumer-consistency conformance harness.
# Verifies that goto-definition, hover, and PL701 diagnostic agree on module resolution
# across 5 resolution modes: relative includePaths, lexical use lib, no lib cancellation,
# FindBin-relative, and system @INC (PERL5LIB).
metrics-inc-conformance:
    @echo "Running @INC consumer-consistency conformance harness..."
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-lsp-ux-tests --test ux_scenario_14_inc_conformance -- --test-threads=1 --nocapture
    @echo "@INC conformance harness passed"

# LSP BDD workflow tests (serialized to prevent WSL resource exhaustion)
ci-lsp-bdd:
    @echo "🎭 Running LSP BDD workflow tests..."
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-lsp-rs --locked --test lsp_bdd_workflows -- --test-threads=1
    @echo "✅ LSP BDD workflow tests passed"

# LSP microcrate compatibility coverage (feature microcrates + provider re-export contract)
ci-lsp-microcrates:
    @echo "🧩 Running LSP microcrate compatibility tests..."
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-lsp-providers --locked --test microcrate_reexports_compatibility -- --test-threads=1
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-lsp-rs --locked --test lsp_color_tests -- --test-threads=1
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-lsp-rs --locked --test lsp_code_lens_tests -- --test-threads=1
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-lsp-rs --locked --test lsp_inline_completion_tests -- --test-threads=1
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test --locked -p perl-feature-catalog \
                   -p perl-lsp-feature-ids \
                   -p perl-lsp-feature-profile \
                   -p perl-lsp-feature-policy \
                   -p perl-lsp-feature-flags \
                   -p perl-lsp-feature-contracts \
                   -p perl-lsp-feature-grid \
                   -p perl-lsp-feature-governance \
                   -p perl-lsp-launcher
    @echo "✅ LSP microcrate compatibility tests passed"

# Framework semantic depth receipts (Moo/Moose/Class::Accessor)
ci-semantic-frameworks:
    @echo "🧠 Running framework semantic tests..."
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-semantic-analyzer --test frameworks_moo -- --test-threads=1
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-lsp-rs --test moo_semantics_e2e -- --test-threads=1
    @echo "✅ Framework semantic tests passed"

# Multi-root workspace integration tests (timing-sensitive, nightly only)
# Requires PERL_LSP_WORKSPACE=1 and features workspace,expose_lsp_test_api.
# These tests use 15-second adaptive timeouts — proven stable before blocking merges.
ci-workspace-multiroot:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=============================================="
    echo "  Multi-Root Workspace Integration Tests"
    echo "=============================================="
    START=$(date +%s)
    env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        PERL_LSP_WORKSPACE=1 \
        cargo test -p perl-lsp-rs \
            --features workspace,expose_lsp_test_api \
            --test multi_root_workspace_tests \
            -- --test-threads=1
    END=$(date +%s)
    echo ""
    echo "=============================================="
    echo "  Multi-root tests PASSED (total: $((END - START))s)"
    echo "=============================================="

# DAP smoke receipt (launch/breakpoint/step/stack/evaluate/disconnect)
ci-dap-smoke-e2e:
    @echo "🐞 Running DAP smoke E2E test..."
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-dap --test dap_smoke_e2e -- --test-threads=1
    @echo "✅ DAP smoke E2E passed"

# Documentation build (no deps)
ci-docs:
    @echo "📚 Building documentation..."
    cargo doc -p perl-parser -p perl-lsp-rs --no-deps
    @echo "✅ Docs build passed"

# Verify docs.rs builds for all publishable crates
# Usage: just docs-verify [--fast]  (--fast skips large crates)
docs-verify *args:
    @echo "Verifying docs.rs builds for all publishable crates..."
    bash scripts/verify-docs-rs.sh {{args}}

# Mutation testing (expensive, ~15-30 min)
ci-test-mutation:
    @echo "🧬 Running mutation tests..."
    cargo mutants --package perl-parser --timeout 300
    @echo "✅ Mutation tests passed"

# Cost estimation
ci-cost-estimate:
    @echo "💰 Estimating CI costs (essential jobs: ~$0.06-0.08 per PR)"
    @just ci-local

# ============================================================================
# Low-Memory Debugging Commands
# ============================================================================

# Trace a command with /usr/bin/time -v to capture Max RSS (peak memory)
# Usage: just trace 'cargo clippy -p perl-parser --no-deps -j1 -- -D warnings'
trace cmd:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p target/ci-trace
    log=target/ci-trace/trace-$(date +%Y%m%d-%H%M%S).log
    echo "CMD: {{cmd}}" | tee -a "$log"
    /usr/bin/time -v {{cmd}} 2>&1 | tee -a "$log"
    echo "---" | tee -a "$log"
    echo "Log: $log"

# Trace each low-mem step individually to find memory hotspots
trace-lowmem-steps:
    @echo "🔬 Tracing low-memory steps individually..."
    @mkdir -p target/ci-trace
    @echo "Step 1: format check"
    @just trace 'cargo fmt --check --all'
    @echo "Step 2: clippy lib (no-deps)"
    @just trace 'env -u RUSTC_WRAPPER cargo clippy --workspace --lib --locked --no-deps -j1 -- -D warnings -A missing_docs'
    @echo "Step 3: clippy bins (no-deps)"
    @just trace 'env -u RUSTC_WRAPPER cargo clippy --workspace --bins --locked --no-deps -j1 -- -D clippy::unwrap_used -D clippy::expect_used'
    @echo "Step 4: tests lib"
    @just trace 'env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 cargo test --workspace --lib --locked -j1 -- --test-threads=1'
    @echo "📊 Check target/ci-trace/ for Max RSS values"

# Full parser/DAP tests (not just heredoc-targeted) with low-memory settings
ci-test-parser-dap-full:
    @echo "🎯 Running full parser/DAP tests (single-threaded)..."
    @env -u RUSTC_WRAPPER CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 \
        sh -c 'echo "🧪 Running all perl-parser lib tests..." && \
        cargo test -p perl-parser --lib -j1 -- --test-threads=1 && \
        echo "🧪 Running all perl-dap tests..." && \
        cargo test -p perl-dap -j1 -- --test-threads=1 && \
        echo "🔍 Running clippy on affected crates (no-deps)..." && \
        cargo clippy -p perl-parser-core -p perl-parser -p perl-dap --lib --no-deps -j1 -- -D warnings'
    @echo "✅ Full Parser/DAP tests passed"

# ============================================================================
# Development Commands
# ============================================================================

# Build all workspace crates
build:
    cargo build --workspace

# Run all tests
test:
    cargo test --workspace

# Format code
fmt:
    cargo xtask fmt

# Clean build artifacts
clean:
    cargo clean

# Missing docs ratcheting check (Issue #197)
ci-docs-check:
    @echo "📝 Checking missing docs baseline..."
    @cargo xtask ci-hygiene check-missing-docs
    @echo "✅ Missing docs check passed"

# Policy and governance checks
ci-policy:
    @echo "⚖️  Checking project policies..."
    just ci-check-todos
    @cargo xtask check-from-raw
    just version-check
    just ci-doc-claims

# Check article inline claims against PUBLICATION_FACTS_LEDGER.md
ci-doc-claims:
    @echo "📄 Checking article claims against publication ledger..."
    @cargo xtask doc-claims
    @echo "✅ Doc claims check passed"

# Check all registered hook scripts are executable
hook-check:
    @cargo xtask hook-check

# Check hook registry in settings.json matches files on disk
hook-registry-check:
    @cargo xtask hook-registry-check

# Run all hook tests (behavior, registry, executable-bit)
hook-tests:
    @cargo xtask hook-tests

# Show swarm metrics summary
swarm-summary:
    @cargo xtask swarm-summary

# Check for machine-specific paths in documentation
ci-doc-paths:
    @echo "🔍 Checking documentation paths..."
    @cargo xtask ci-hygiene check-doc-paths docs
    @echo "✅ Documentation paths check passed"

# Verify publication facts against live codebase metrics (informational, non-blocking)
# Flags WARNING if delta >5%, ERROR if delta >10%. Use --strict to exit 1 on ERROR.
verify-publication-facts *args='':
    @echo "📊 Verifying publication facts..."
    @cargo xtask verify-publication-facts {{args}}
    @echo "✅ Publication facts verification complete"

# Strict publication facts check for CI (exits 1 on ERROR-level drift)
ci-publication-facts:
    @echo "📊 Checking publication facts (strict mode)..."
    @cargo xtask verify-publication-facts --strict
    @echo "✅ Publication facts check passed"

# Update derived metrics in docs/project/status/ subsystem files and ROADMAP.md.
# Optionally pass a subsystem name to regenerate only that one (e.g. just status-update lsp).
status-update subsystem="":
    @if [ -n "{{subsystem}}" ]; then \
        cargo run -p xtask -- update-status --write --only {{subsystem}}; \
    else \
        cargo run -p xtask -- update-status --write; \
    fi

# Verify docs/project/status/ subsystem files are up-to-date
status-check:
    @cargo run -p xtask -- update-status --check

# ============================================================================
# Corpus Audit Commands
# ============================================================================

# Run corpus audit for coverage analysis
corpus-audit:
    @echo "🔍 Running corpus audit..."
    @cd xtask && cargo run --no-default-features -- corpus-audit

# Run corpus audit in CI check mode (fails if issues found)
corpus-audit-check:
    @echo "🔍 Running corpus audit (CI check mode)..."
    @cd xtask && cargo run --no-default-features -- corpus-audit --check

# Run corpus audit with fresh report regeneration
corpus-audit-fresh:
    @echo "🔍 Running corpus audit (fresh mode)..."
    @cd xtask && cargo run --no-default-features -- corpus-audit --fresh

# ============================================================================
# Diagnostics Metrics Commands
# ============================================================================

# Run gold corpus diagnostics test suite (precision/recall validation)
metrics-diagnostics:
    @echo "📊 Running diagnostics gold suite..."
    @cargo test -p perl-lsp-diagnostics --test diagnostics_gold_suite -- --nocapture

# ============================================================================
# Parser Feature Coverage Commands (Issue #180)
# ============================================================================

# Run parser audit for coverage analysis (detailed report)
parser-audit:
    @echo "📊 Running parser audit..."
    @cargo run -p xtask --no-default-features -- corpus-audit --fresh --corpus-path .
    @echo ""
    @echo "Report written to: corpus_audit_report.json"
    @python3 -c "import json; r=json.load(open('corpus_audit_report.json')); po=r['parse_outcomes']; print(f'Parse success: {po[\"ok\"]}/{po[\"total\"]} files ({100*po[\"ok\"]/po[\"total\"]:.0f}%)')"

# Check parser features baseline (CI mode, fails on regression)
#
# Issue #3202: corpus-audit and check-parse-errors must run as SEPARATE
# top-level cargo invocations (not nested) on Windows. Nesting them caused
# `os error 5: Access is denied` because the inner cargo tried to relink
# the still-running parent xtask.exe.
ci-parser-features-check:
    @echo "🔍 Checking parser features baseline..."
    @echo "  Step 1/2: corpus-audit (regenerates corpus_audit_report.json)"
    @cargo run -p xtask --no-default-features -q -- corpus-audit --fresh --corpus-path . --output corpus_audit_report.json
    @echo "  Step 2/2: check-parse-errors (compares to baseline)"
    @cargo xtask ci-hygiene check-parse-errors

# Check features.toml invariants (GA+advertised must have tests, no duplicates)
ci-features-invariants:
    @echo "🔍 Checking features.toml invariants..."
    @cargo xtask features invariants

# Update parser feature matrix document from audit report
parser-matrix-update:
    @echo "📝 Updating parser feature matrix..."
    @cargo xtask parser-matrix

# ============================================================================
# GitHub Repository Management
# ============================================================================

# Ensure label taxonomy exists (idempotent, safe to rerun)
gh-labels:
    @echo "🏷️  Ensuring label taxonomy..."
    @cargo xtask gh-labels
    @echo "✅ Labels ready"

# Show issues missing required taxonomy labels
gh-triage:
    @echo "🔍 Issues needing taxonomy labels..."
    @cargo xtask gh-triage

# Backfill prefixed labels from legacy labels (dry run)
gh-backfill-dry:
    @echo "🔄 Dry run: showing labels to backfill..."
    @cargo xtask gh-backfill-prefixed-labels

# Backfill prefixed labels from legacy labels (apply)
gh-backfill:
    @echo "🔄 Applying prefixed label backfill..."
    @cargo xtask gh-backfill-prefixed-labels --apply

# ============================================================================
# Bug Tracking (BUG category ignored tests)
# ============================================================================

# Show current bug status
bugs:
    @echo "🐛 Bug Queue Status"
    @echo "==================="
    @VERBOSE=1 cargo xtask ci-hygiene ignored-test-count 2>&1 | sed -n '/=== bug/,/===/p' | head -30

# Wave A: COMPLETE - these were test brittleness issues, not parser bugs
bugs-wave-a:
    @echo "✅ Wave A: Complete (tests were brittle, not bugs)"
    @echo "   - test_word_boundary_qwerty_not_matched: fixed test expectations"
    @echo "   - test_comment_with_qw_in_it: fixed dynamic position calculation"

# Run all Wave B bug tests (substitution)
bugs-wave-b:
    @echo "🌊 Wave B: Substitution Operator Bugs"
    cargo test -p perl-parser --test substitution_operator_tests -- test_substitution_empty_replacement_balanced_delimiters --nocapture --ignored || true
    cargo test -p perl-parser --test substitution_ac_tests -- test_ac2_empty_replacement_balanced_delimiters --nocapture --ignored || true
    cargo test -p perl-parser --test substitution_operator_tests -- test_substitution_invalid_modifier_characters --nocapture --ignored || true
    cargo test -p perl-parser --test substitution_ac_tests -- test_ac2_invalid_flag_combinations --nocapture --ignored || true

# Run all Wave C bug tests (harder semantics)
bugs-wave-c:
    @echo "🌊 Wave C: Semantic Bugs"
    cargo test -p perl-parser --test substitution_ac_tests -- test_ac5_negative_malformed --nocapture --ignored || true
    cargo test -p perl-parser --test prop_whitespace_idempotence -- insertion_safe_is_consistent --nocapture --ignored || true
    cargo test -p perl-parser --test comprehensive_operator_precedence_test -- test_complex_precedence_combinations --nocapture --ignored || true
    cargo test -p perl-parser --test parser_regressions -- print_filehandle_then_variable_is_indirect --nocapture --ignored || true

# ============================================================================
# Roadmap Gate (informational, never blocks merge)
# ============================================================================

# Run feature/infra ignored tests and report progress
roadmap-gate:
    @echo "=== ROADMAP BACKLOG: running ignored feature/infra tests ==="
    -cargo test -p perl-semantic-analyzer -- test_anonymous_subroutine --ignored --nocapture
    -cargo test -p perl-dap -- test_attach_tcp_valid_arguments test_attach_default_values --ignored --nocapture
    -cargo test -p perl-parser -- test_statement_with_or_modifier --ignored --nocapture
    -RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- test_fix_undefined_variable test_user_story_debugging_workflow test_user_story_refactoring_legacy_code --ignored --test-threads=2 --nocapture
    @echo "=== Roadmap gate complete (failures = unimplemented features) ==="

# Health Scoreboard (keep yourself honest)
# ============================================================================

# Show codebase health metrics
health:
    @echo "📊 Codebase Health Scoreboard"
    @echo "=============================="
    @echo ""
    @echo "📝 Ignored Tests by Crate:"
    @echo "  perl-parser: $(grep -r '#\[ignore' crates/perl-parser/tests/ 2>/dev/null | wc -l || echo 0)"
    @echo "  perl-lsp:    $(grep -r '#\[ignore' crates/perl-lsp/tests/ 2>/dev/null | wc -l || echo 0)"
    @echo "  perl-lexer:  $(grep -r '#\[ignore' crates/perl-lexer/tests/ 2>/dev/null | wc -l || echo 0)"
    @echo "  perl-dap:    $(grep -r '#\[ignore' crates/perl-dap/tests/ 2>/dev/null | wc -l || echo 0)"
    @echo ""
    @echo "⚠️  Unwrap/Expect Count (potential panic sites):"
    @echo "  .unwrap():  $(grep -r '\.unwrap()' crates/*/src/ --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo "  .expect(:   $(grep -r '\.expect(' crates/*/src/ --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo ""
    @echo "🖨️  Debug Print Count (should use tracing):"
    @echo "  println!:   $(grep -r 'println!' crates/*/src/ --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo "  eprintln!:  $(grep -r 'eprintln!' crates/*/src/ --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo ""
    @echo "📦 Public Items in perl-parser (API surface):"
    @echo "  pub fn:     $(grep -r '^[[:space:]]*pub fn' crates/perl-parser/src/ --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo "  pub struct: $(grep -r '^[[:space:]]*pub struct' crates/perl-parser/src/ --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo "  pub enum:   $(grep -r '^[[:space:]]*pub enum' crates/perl-parser/src/ --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo ""
    @echo "🔧 LSP Crate Size (crates/perl-lsp/src/):"
    @echo "  Lines:      $(find crates/perl-lsp/src -name '*.rs' | xargs wc -l | tail -n 1 | awk '{print $1}' || echo 'N/A')"
    @echo ""
    @echo "🧹 Dead Code Metrics:"
    @echo "  Unused deps: $(cargo machete 2>&1 | grep -c 'Cargo.toml:' || echo 0) crates affected"
    @echo "  Dead code allows: $(grep -r '#\[allow(dead_code)\]' crates --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo ""
    @echo "💡 Run 'just health-detail' for file-by-file breakdown"

# Detailed health metrics with file breakdown
health-detail:
    @echo "📊 Detailed Health Metrics"
    @echo "=========================="
    @echo ""
    @echo "🔴 Top 10 files with most .unwrap() calls:"
    @grep -r '\.unwrap()' crates/*/src/ --include='*.rs' -c 2>/dev/null | sort -t: -k2 -nr | head -10 || echo "  None found"
    @echo ""
    @echo "🟡 Top 10 files with most eprintln! calls:"
    @grep -r 'eprintln!' crates/*/src/ --include='*.rs' -c 2>/dev/null | sort -t: -k2 -nr | head -10 || echo "  None found"
    @echo ""
    @echo "📁 Largest source files (by lines):"
    @find crates/*/src -name '*.rs' -exec wc -l {} \; 2>/dev/null | sort -nr | head -10 || echo "  None found"

# Show ignored test counts (categorised summary with baseline delta)
ignored-tests:
    cargo xtask ignored-tests

# Show ignored test counts with per-test detail
ignored-tests-verbose:
    cargo xtask ignored-tests --verbose

# Update ignored test baseline after intentional changes
ignored-tests-update:
    cargo xtask ignored-tests --update

# ============================================================================
# Milestone Verification
# ============================================================================

# Verify v0.9.0 release exit criteria (mechanical checks)
milestone-v0_9-check:
    @echo "🎯 Verifying v0.9.0 exit criteria..."
    @echo ""
    @echo "📋 Step 1: Running ci-gate..."
    @just ci-gate
    @echo ""
    @echo "📋 Step 2: Checking ignored test breakdown..."
    @cargo xtask ci-hygiene ignored-test-count
    @echo ""
    @echo "📋 Step 3: Verifying metrics consistency..."
    @just status-check
    @echo ""
    @echo "✅ v0.9.0 exit criteria check complete!"
    @echo "   Next: Manual review of BUG=0, MANUAL≤1 from test count output above"

# ============================================================================
# Forensics (post-hoc PR archaeology)
# ============================================================================

# Harvest raw facts from a merged PR
forensics-harvest pr:
    @echo "🔬 Harvesting raw facts from PR {{pr}}..."
    @cargo xtask forensics-harvest {{pr}}
    @echo "✅ Harvest complete"

# Compute temporal topology (convergence, friction, oscillations)
forensics-temporal pr:
    @echo "⏱️  Computing temporal topology for PR {{pr}}..."
    @cargo xtask forensics-temporal {{pr}}
    @echo "✅ Temporal analysis complete"

# Run static analysis deltas (quick mode)
forensics-telemetry-quick pr:
    @echo "📊 Running quick telemetry for PR {{pr}}..."
    @cargo xtask forensics-telemetry-quick {{pr}}
    @echo "✅ Quick telemetry complete"

# Run static analysis deltas (full mode with exhibit-grade tools)
forensics-telemetry-full pr:
    @echo "📊 Running full telemetry for PR {{pr}}..."
    @cargo xtask forensics-telemetry-full {{pr}}
    @echo "✅ Full telemetry complete"

# Generate complete dossier (runs full pipeline)
forensics-dossier pr:
    @echo "📁 Generating complete dossier for PR {{pr}}..."
    @cargo xtask forensics-dossier {{pr}}
    @echo "✅ Dossier generation complete"

# Render dossier markdown from existing YAML outputs
forensics-render pr format='full':
    @echo "📝 Rendering dossier for PR {{pr}} (format: {{format}})..."
    @cargo xtask forensics-render {{pr}} --format {{format}}
    @echo "✅ Rendering complete"

# ============================================================================
# Benchmark Infrastructure
# ============================================================================
# Run performance benchmarks with structured output.
# See benchmarks/README.md for documentation.

# Run all benchmarks
bench:
    @echo "📊 Running full benchmark suite..."
    @mkdir -p benchmarks/results
    @cargo xtask bench-run --output benchmarks/results/latest.json
    @echo ""
    @echo "Results saved to benchmarks/results/latest.json"
    @cargo xtask bench-format

# Quick smoke benchmarks (fast, ~30s)
bench-quick:
    @echo "⚡ Running quick benchmark smoke test..."
    @mkdir -p benchmarks/results
    @cargo xtask bench-run --quick --output benchmarks/results/latest.json
    @echo ""
    @cargo xtask bench-format --receipt

# Compare current results against baseline
bench-compare:
    @echo "📈 Comparing against baseline..."
    @cargo xtask bench-compare

# Compare with failure on regression (for CI)
bench-compare-strict:
    @echo "📈 Comparing against baseline (strict mode)..."
    @cargo xtask bench-compare --fail-on-regression

# Save current results as a new baseline
bench-baseline version='':
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Saving benchmark baseline..."
    mkdir -p benchmarks/baselines
    if [ -z "{{version}}" ]; then
        VERSION="v$(date +%Y%m%d)"
    else
        VERSION="{{version}}"
    fi
    if [ ! -f benchmarks/results/latest.json ]; then
        echo "No results found. Running benchmarks first..."
        just bench
    fi
    cp benchmarks/results/latest.json "benchmarks/baselines/$VERSION.json"
    echo "Baseline saved to benchmarks/baselines/$VERSION.json"

# Run parser benchmarks only
bench-parser:
    @echo "📊 Running parser benchmarks..."
    @cargo xtask bench-run --category parser --output benchmarks/results/latest.json

# Run lexer benchmarks only
bench-lexer:
    @echo "📊 Running lexer benchmarks..."
    @cargo xtask bench-run --category lexer --output benchmarks/results/latest.json

# Run LSP benchmarks only
bench-lsp:
    @echo "📊 Running LSP benchmarks..."
    @cargo xtask bench-run --category lsp --output benchmarks/results/latest.json

# Run workspace index benchmarks only
bench-index:
    @echo "📊 Running workspace index benchmarks..."
    @cargo xtask bench-run --category index --output benchmarks/results/latest.json

# Format benchmark results as receipt
bench-receipt:
    @echo "📋 Generating benchmark receipt..."
    @cargo xtask bench-format --receipt

# Format benchmark results as markdown
bench-markdown:
    @echo "📋 Generating benchmark markdown..."
    @cargo xtask bench-format --markdown

# Generate performance regression alerts (terminal)
bench-alert:
    @echo "📊 Checking for performance regressions..."
    @cargo xtask bench-alert

# Generate performance regression alerts (markdown for PR)
bench-alert-md:
    @echo "📊 Generating performance alert (markdown)..."
    @cargo xtask bench-alert --format markdown

# Check for critical performance regressions (exits non-zero)
bench-alert-check:
    @echo "🔍 Checking for critical regressions..."
    @cargo xtask bench-alert --check

# Extract Criterion benchmark output from target/criterion artifacts
bench-extract:
    @echo "📊 Extracting benchmark results from Criterion..."
    @cargo xtask bench-extract

# Run benchmark alert regression self-checks
bench-alert-test:
    @echo "🧪 Running benchmark alert regression tests..."
    @cargo xtask bench-alert-test


# Run all performance benchmarks and save baseline for 0.12.0
perf-baseline:
    @echo "Running performance baseline benchmarks..."
    cargo bench -p perl-parser --bench parser_benchmark --locked
    cargo bench -p perl-lexer --bench lexer_benchmarks --locked
    cargo bench -p perl-lsp-completion --bench completion_benchmark --locked
    cargo bench -p perl-lsp-navigation --bench navigation_benchmark --locked
    cargo bench -p perl-workspace-index --bench workspace_index_benchmark --locked
    cargo bench -p perl-lsp-rs --bench rope_performance_benchmark --locked
    cargo bench -p perl-lsp-tooling --bench cache_benchmark --locked
    @echo "Baseline complete. See docs/project/PERFORMANCE_BASELINES.md"

# ============================================================================
# Code Coverage (Issue #276)
# ============================================================================
# Generate and analyze code coverage reports using cargo-llvm-cov.
# See codecov.yml for service configuration.

# Generate local HTML coverage report
coverage:
    @echo "📊 Generating coverage report..."
    @if [[ ! -x "$HOME/.cargo/bin/cargo-llvm-cov" ]]; then \
        echo "❌ cargo-llvm-cov not found. Installing..."; \
        "$HOME/.cargo/bin/rustup" run nightly cargo install cargo-llvm-cov --locked; \
    fi
    @"$HOME/.cargo/bin/rustup" run nightly cargo llvm-cov -p perl-parser --lib --locked --branch --html --output-dir target/coverage \
        --ignore-filename-regex '(^|/)(archive|tests|benches|examples)(/|$)|(^|/)build\.rs$|(^|/)crates/tree-sitter-perl-c/'
    @echo "✅ Coverage report: target/coverage/index.html"
    @echo "📈 Opening report in browser..."
    @command -v xdg-open >/dev/null 2>&1 && xdg-open target/coverage/index.html || \
     command -v open >/dev/null 2>&1 && open target/coverage/index.html || \
     echo "⚠️  Please open target/coverage/index.html manually"

# Generate coverage report (lcov format for CI)
coverage-lcov:
    @echo "📊 Generating coverage (lcov format)..."
    @if [[ ! -x "$HOME/.cargo/bin/cargo-llvm-cov" ]]; then \
        echo "❌ cargo-llvm-cov not found. Installing..."; \
        "$HOME/.cargo/bin/rustup" run nightly cargo install cargo-llvm-cov --locked; \
    fi
    @"$HOME/.cargo/bin/rustup" run nightly cargo llvm-cov -p perl-parser --lib --locked --branch --lcov --output-path lcov.info \
        --ignore-filename-regex '(^|/)(archive|tests|benches|examples)(/|$)|(^|/)build\.rs$|(^|/)crates/tree-sitter-perl-c/'
    @echo "✅ Coverage: lcov.info"

# Show coverage summary (terminal)
coverage-summary:
    @echo "📊 Coverage Summary"
    @echo "==================="
    @if [[ ! -x "$HOME/.cargo/bin/cargo-llvm-cov" ]]; then \
        echo "❌ cargo-llvm-cov not found. Installing..."; \
        "$HOME/.cargo/bin/rustup" run nightly cargo install cargo-llvm-cov --locked; \
    fi
    @"$HOME/.cargo/bin/rustup" run nightly cargo llvm-cov -p perl-parser --lib --locked --branch \
        --ignore-filename-regex '(^|/)(archive|tests|benches|examples)(/|$)|(^|/)build\.rs$|(^|/)crates/tree-sitter-perl-c/'

# Generate branch coverage and fail if it regresses against the baseline policy
coverage-branch-gate:
    @echo "📊 Generating branch coverage gate data..."
    @if [[ ! -x "$HOME/.cargo/bin/cargo-llvm-cov" ]]; then \
        echo "❌ cargo-llvm-cov not found. Installing..."; \
        "$HOME/.cargo/bin/rustup" run nightly cargo install cargo-llvm-cov --locked; \
    fi
    @"$HOME/.cargo/bin/rustup" run nightly cargo llvm-cov -p perl-parser --lib --locked --branch --lcov --output-path lcov.info \
        --ignore-filename-regex '(^|/)(archive|tests|benches|examples)(/|$)|(^|/)build\.rs$|(^|/)crates/tree-sitter-perl-c/'
    @bash ./scripts/check-coverage-baseline.sh lcov.info .ci/coverage-baseline.txt

# Refresh the checked-in coverage baseline from a fresh parser coverage snapshot.
coverage-baseline-refresh:
    @just coverage-lcov
    @bash ./scripts/update-coverage-baseline.sh lcov.info .ci/coverage-baseline.txt

# ============================================================================
# Technical Debt Tracking (Issue #XXX)
# ============================================================================
# Track flaky tests, known issues, and technical debt with budgets.
# See .ci/debt-ledger.yaml for configuration.

# Show current debt status report
debt-report:
    @cargo xtask debt-report

# CI gate: fail if debt budget exceeded or quarantines expired
debt-check:
    @echo "🔍 Checking debt budget compliance..."
    @cargo xtask debt-report --check

# Show only expired quarantines (quick check)
debt-expired:
    @cargo xtask debt-report --expired

# Output debt report as JSON (for receipt integration)
debt-json:
    @cargo xtask debt-report --json

# Add a flaky test to quarantine (interactive helper)
debt-quarantine name issue days="14":
    @echo "Adding {{name}} to quarantine for {{days}} days..."
    @echo ""
    @echo "To complete this action, add the following to .ci/debt-ledger.yaml"
    @echo "under the 'flaky_tests:' section:"
    @echo ""
    @echo "  - name: \"{{name}}\""
    @echo "    added: \"$(date -u +%Y-%m-%d)\""
    @echo "    issue: \"{{issue}}\""
    @echo "    tier: \"quarantine\""
    @echo "    quarantine_days: {{days}}"
    @echo "    expires: \"$(date -u -d '+{{days}} days' +%Y-%m-%d 2>/dev/null || date -v+{{days}}d -u +%Y-%m-%d)\""
    @echo "    notes: \"<describe the failure pattern>\""
    @echo ""
    @echo "Then run: just debt-report"

# Remove a test from quarantine (interactive helper)
debt-unquarantine name:
    @echo "To remove {{name}} from quarantine:"
    @echo ""
    @echo "1. Remove the entry from .ci/debt-ledger.yaml 'flaky_tests:' section"
    @echo "2. Optionally add a 'resolved' entry to the 'history.resolved:' section:"
    @echo ""
    @echo "  - type: \"flaky_test\""
    @echo "    name: \"{{name}}\""
    @echo "    resolved: \"$(date -u +%Y-%m-%d)\""
    @echo "    resolution: \"<describe the fix>\""
    @echo "    pr: \"#XXX\""
    @echo ""
    @echo "3. Run: just debt-report"

# Show debt summary suitable for PR comments
debt-pr-summary:
    @echo "## Technical Debt Status"
    @echo ""
    @cargo xtask debt-report --summary

# ============================================================================
# CI Guardrail Tests (Issue #364)
# ============================================================================
# Tests for automated ignored test monitoring and governance.
# Tests are in xtask/tests/ci_guardrail_ignored_test_monitoring_tests.rs

# Run guardrail tests (shows ignored status)
guardrail-tests:
    @echo "🔍 Running CI guardrail tests (scaffolding)..."
    cargo test -p xtask --test ci_guardrail_ignored_test_monitoring_tests

# Check guardrail test status
guardrail-status:
    @echo "📊 CI Guardrail Test Status"
    @echo "==========================="
    @echo ""
    @cargo test -p xtask --test ci_guardrail_ignored_test_monitoring_tests 2>&1 | grep -E "(test .*ignored|test result)"
    @echo ""
    @echo "Note: These tests are scaffolding for Issue #364"
    @echo "They will be enabled as features are implemented (AC13-AC15)"

# Try running guardrail tests (will fail until features implemented)
guardrail-run-ignored:
    @echo "⚠️  Attempting to run ignored guardrail tests..."
    @echo "Note: Some tests expected to fail pending feature implementation"
    @cargo test -p xtask --test ci_guardrail_ignored_test_monitoring_tests -- --ignored || true


# crates.io launch dry-run checks
prep-crates-io-launch mode='core':
    @echo "🚀 Running crates.io launch prep (mode={{mode}})..."
    @if [ "{{mode}}" = "--all" ] || [ "{{mode}}" = "all" ]; then \
        cargo xtask prep-crates-io-launch --mode all; \
    elif [ "{{mode}}" = "--core" ] || [ "{{mode}}" = "core" ]; then \
        cargo xtask prep-crates-io-launch --mode core; \
    else \
        echo "Invalid mode '{{mode}}' (expected core/all or --core/--all)"; \
        exit 1; \
    fi

# ============================================================================
# SemVer Breaking Change Detection (Issue #277)
# ============================================================================
# Automated semantic versioning validation to prevent accidental breaking changes.
# Uses cargo-semver-checks to compare against baseline (last release tag).

# Check for breaking changes against last release
semver-check:
    @echo "🔍 Checking for SemVer breaking changes..."
    @just _semver-check-install
    @just _semver-check-run

# Check specific package for breaking changes
semver-check-package package:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "🔍 Checking {{package}} for SemVer breaking changes..."
    just _semver-check-install
    BASELINE="$(git tag | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -1)"
    cargo semver-checks check-release -p {{package}} --baseline-rev "$BASELINE"

# Check all published packages
semver-check-all:
    @echo "🔍 Checking all published packages for SemVer breaking changes..."
    @just _semver-check-install
    @just semver-check-package perl-parser
    @just semver-check-package perl-lexer
    @just semver-check-package perl-parser-core
    @just semver-check-package perl-lsp-rs
    @just semver-check-package perllsp

# Generate breaking changes report
semver-report:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "📊 Generating SemVer breaking changes report..."
    just _semver-check-install
    mkdir -p target/semver-reports
    BASELINE="$(git tag | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -1)"
    cargo semver-checks check-release --workspace --baseline-rev "$BASELINE" \
        --output-format json > target/semver-reports/breaking-changes.json || true
    echo "Report saved to: target/semver-reports/breaking-changes.json"

# List all available baseline tags
semver-list-baselines:
    @echo "📋 Available baseline tags:"
    @git tag | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -10

# Show what changed in public API since last release
semver-diff package='perl-parser':
    #!/usr/bin/env bash
    set -euo pipefail
    echo "📝 Public API changes in {{package}} since last release:"
    just _semver-check-install
    BASELINE="$(git tag | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -1)"
    cargo semver-checks check-release -p {{package}} --baseline-rev "$BASELINE" || true

# Private helper: install cargo-semver-checks if missing
[private]
_semver-check-install:
    @if ! command -v cargo-semver-checks >/dev/null 2>&1; then \
        echo "📦 Installing cargo-semver-checks..."; \
        cargo install cargo-semver-checks --locked; \
    fi

# Private helper: run semver checks on core packages
[private]
_semver-check-run:
    #!/usr/bin/env bash
    set -euo pipefail
    BASELINE="$(git tag | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -1)"
    EXIT_CODE=0
    echo "Using baseline: $BASELINE"
    echo
    echo "Checking perl-parser..."
    cargo semver-checks check-release -p perl-parser --baseline-rev "$BASELINE" || EXIT_CODE=1
    echo
    echo "Checking perl-lexer..."
    cargo semver-checks check-release -p perl-lexer --baseline-rev "$BASELINE" || EXIT_CODE=1
    echo
    echo "Checking perl-parser-core..."
    cargo semver-checks check-release -p perl-parser-core --baseline-rev "$BASELINE" || EXIT_CODE=1
    exit "$EXIT_CODE"

# Private helper: get baseline tag for comparison
[private]
_semver-baseline-tag:
    @git tag | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -1

# ============================================================================
# Fuzzing (cargo-fuzz integration)
# ============================================================================

# Run fuzzing on specific target (default: 60 seconds)
fuzz target='substitution_parsing' duration='60':
    @echo "🔥 Fuzzing {{target}} for {{duration}} seconds..."
    @cargo +nightly fuzz run {{target}} -- -max_total_time={{duration}}

# List available fuzz targets
fuzz-list:
    @echo "📋 Available fuzz targets:"
    @cargo +nightly fuzz list

# Run continuous fuzzing (for local development, Ctrl+C to stop)
fuzz-continuous target='substitution_parsing':
    @echo "🔥 Running continuous fuzzing on {{target}} (Ctrl+C to stop)..."
    @echo "📊 Corpus: fuzz/corpus/{{target}}"
    @echo "💥 Crashes: fuzz/artifacts/{{target}}"
    @cargo +nightly fuzz run {{target}}

# Check fuzz corpus coverage for a target
fuzz-coverage target='substitution_parsing':
    @echo "📊 Checking coverage for {{target}}..."
    @cargo +nightly fuzz coverage {{target}}
    @echo ""
    @echo "To view coverage report, open: fuzz/coverage/{{target}}/coverage/index.html"

# Minimize a crash case to smallest reproducing input
fuzz-minimize target crash:
    @echo "🔍 Minimizing crash case for {{target}}..."
    @cargo +nightly fuzz cmin {{target}} {{crash}}

# Check for crash artifacts (fails if crashes found)
fuzz-check-crashes:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Checking for crash artifacts..."
    if [ -d fuzz/artifacts ]; then
        CRASHES=$(find fuzz/artifacts -type f 2>/dev/null | wc -l)
        if [ $CRASHES -gt 0 ]; then
            echo "Found $CRASHES crash artifacts:"
            find fuzz/artifacts -type f 2>/dev/null
            exit 1
        else
            echo "No crashes found"
        fi
    else
        echo "No artifacts directory (no crashes)"
    fi

# Run all fuzz targets for regression testing (short duration)
fuzz-regression duration='30':
    @echo "🔥 Running fuzz regression tests ({{duration}}s per target)..."
    @just fuzz builtin_functions {{duration}} || true
    @just fuzz heredoc_parsing {{duration}} || true
    @just fuzz lsp_cancellation_registry {{duration}} || true
    @just fuzz parser_integration {{duration}} || true
    @just fuzz quote_operators {{duration}} || true
    @just fuzz substitution_parsing {{duration}} || true
    @just fuzz lsp_navigation {{duration}} || true
    @just fuzz unicode_positions {{duration}} || true
    @just fuzz-check-crashes
    @echo "✅ Fuzz regression testing complete"

# ============================================================================
# Documentation Site (mdBook)
# ============================================================================

# Build documentation site with mdBook
docs-build:
    @echo "📖 Building mdBook documentation site..."
    @cargo xtask populate-book
    mdbook build book
    @echo "✅ Documentation site built successfully"
    @echo "📂 Output: book/book/index.html"

# Serve documentation site locally
docs-serve:
    @echo "📖 Serving mdBook documentation site..."
    @cargo xtask populate-book
    @echo "🌐 Starting local server at http://localhost:3000"
    @echo "Press Ctrl+C to stop"
    mdbook serve book --port 3000 --open

# Clean documentation build artifacts
docs-clean:
    @echo "🧹 Cleaning documentation build artifacts..."
    rm -rf book/book
    rm -rf book/src/getting-started
    rm -rf book/src/user-guides
    rm -rf book/src/architecture
    rm -rf book/src/developer
    rm -rf book/src/lsp
    rm -rf book/src/advanced
    rm -rf book/src/reference
    rm -rf book/src/dap
    rm -rf book/src/ci
    rm -rf book/src/process
    rm -rf book/src/resources
    @echo "✅ Documentation artifacts cleaned"

# ============================================================================
# Changelog Generation (Issue #280)
# ============================================================================
# Automated changelog generation using git-cliff.
# See cliff.toml for configuration.

# Generate full changelog (overwrites CHANGELOG.md)
changelog:
    @echo "📝 Generating changelog..."
    @if command -v git-cliff >/dev/null 2>&1; then \
        git-cliff --output CHANGELOG.md; \
        echo "✅ Changelog generated: CHANGELOG.md"; \
    else \
        echo "ERROR: git-cliff not installed."; \
        echo "  Install via: cargo install git-cliff"; \
        echo "  Or: brew install git-cliff (macOS)"; \
        echo "  Or: nix-shell -p git-cliff (Nix)"; \
        exit 1; \
    fi

# Generate changelog for unreleased changes only (preview mode)
changelog-preview:
    @echo "📋 Previewing unreleased changes..."
    @if command -v git-cliff >/dev/null 2>&1; then \
        git-cliff --unreleased; \
    else \
        echo "ERROR: git-cliff not installed. Run: cargo install git-cliff"; \
        exit 1; \
    fi

# Generate changelog for a specific version range
changelog-range from to:
    @echo "📋 Generating changelog from {{from}} to {{to}}..."
    @if command -v git-cliff >/dev/null 2>&1; then \
        git-cliff {{from}}..{{to}}; \
    else \
        echo "ERROR: git-cliff not installed. Run: cargo install git-cliff"; \
        exit 1; \
    fi

# Generate changelog for latest tag only
changelog-latest:
    @echo "📋 Generating changelog for latest tag..."
    @if command -v git-cliff >/dev/null 2>&1; then \
        git-cliff --latest; \
    else \
        echo "ERROR: git-cliff not installed. Run: cargo install git-cliff"; \
        exit 1; \
    fi

# Append unreleased changes to existing CHANGELOG.md (for releases)
changelog-append:
    @echo "📝 Appending unreleased changes to CHANGELOG.md..."
    @if command -v git-cliff >/dev/null 2>&1; then \
        git-cliff --unreleased --prepend CHANGELOG.md; \
        echo "✅ Changelog updated with unreleased changes"; \
    else \
        echo "ERROR: git-cliff not installed. Run: cargo install git-cliff"; \
        exit 1; \
    fi

# ============================================================================
# Dead Code Detection (Issue #284)
# ============================================================================
# Detect unused dependencies, dead code, and unused imports/variables.
# Uses cargo-udeps and clippy dead_code lints.

# Run dead code detection (local check)
dead-code:
    @echo "🔍 Running dead code detection..."
    @cargo xtask dead-code check

# Generate dead code baseline
dead-code-baseline:
    @echo "📝 Generating dead code baseline..."
    @cargo xtask dead-code baseline

# Generate dead code report (JSON)
dead-code-report:
    @echo "📊 Generating dead code report..."
    @cargo xtask dead-code report

# Run dead code detection in strict mode (fail on any increase)
dead-code-strict:
    @echo "🔍 Running dead code detection (strict mode)..."
    @cargo xtask dead-code check --strict

# CI gate: fail if dead code exceeds baseline
ci-dead-code:
    @echo "🔍 Checking dead code baseline..."
    @cargo xtask dead-code check

# ============================================================================
# Scan for built-but-not-wired crates (issue #2667)
# Finds crates with tests but zero direct dependency from perl-lsp.

# Scan for unwired infrastructure (human-readable report)
unwired-scan:
    @cargo xtask unwired-scan

# Scan for unwired infrastructure (JSON output)
unwired-scan-json:
    @cargo xtask unwired-scan --json

# CI gate: exit 1 if any unwired crates are found
ci-unwired-scan:
    @cargo xtask unwired-scan --check

# ============================================================================
# CI Gate Execution with Receipt Generation (Issue #210)
# ============================================================================

# CI gate: check unlinked-item compliance
ci-check-todos:
    @cargo xtask ci-hygiene check-todos

# Fast merge gate with receipt generation
ci-gate-with-receipts:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Running fast merge gate with receipts..."
    DATE=$(date +%Y%m%d)
    mkdir -p ".receipts/$DATE"
    for gate in workflow-audit no-nested-lock format clippy-lib test-lib policy lsp-definition; do
        cargo xtask gates --gate "$gate" --receipt --receipt-path ".receipts/$DATE/$gate.json" || exit 1
    done
    echo "Merge gate passed with receipts!"
    echo "Receipts: .receipts/$DATE/"

# Gate execution for individual gate (with receipt)
gate-execute gate_id:
    @cargo xtask gates --gate "{{gate_id}}" --receipt --receipt-path ".receipts/$(date +%Y%m%d)/{{gate_id}}.json"

# Show gate registry
gate-list:
    @cargo xtask gates --list

# ============================================================================
# Release Gate (Slice C: Release candidate validation)
# ============================================================================

# Release build (locked, optimized)
release-build:
    @echo "Building release binary..."
    cargo build -p perllsp --release --locked
    @echo "Release build complete: target/release/perllsp"

# Version sync check (Slice B: single source of version truth)
version-check:
    @cargo xtask check-version-sync

# Bump the workspace version across every tracked site.
#
# Usage: just bump-version 0.13.0
#
# Updates in one pass (via `cargo xtask bump-version`):
#   - [workspace.package] version in Cargo.toml
#   - All [workspace.dependencies] version fields in Cargo.toml
#   - vscode-extension/package.json (and package-lock.json if present)
#   - features.toml [meta] version
#   - Documentation version references (README.md, CLAUDE.md, ROADMAP.md)
#
# Crate-level Cargo.toml files use `version.workspace = true` and are
# updated automatically through the workspace, so they are not touched.
#
# Does NOT commit or push. Review the diff and commit when satisfied.
# Then tag the release and push — CI runs the publish workflow.
bump-version VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Bumping workspace version to {{VERSION}} ..."
    cargo xtask bump-version "{{VERSION}}"
    echo ""
    echo "Running cargo check to regenerate Cargo.lock ..."
    cargo check --workspace --quiet
    echo ""
    echo "Version bump complete. Changed files:"
    git diff --name-only
    echo ""
    echo "Next steps:"
    echo "  1. Review the diff: git diff"
    echo "  2. Commit: git commit -am 'chore(release): bump version to {{VERSION}}'"
    echo "  3. Push and open PR"
    echo "  4. After merge, tag: git tag v{{VERSION}} && git push origin v{{VERSION}}"
    echo "  5. CI publish workflow triggers automatically on the tag"

# Turnkey PR-driven release orchestrator for 0.x.y releases.
release-turnkey VERSION *ARGS="":
    @cargo xtask release-turnkey "{{VERSION}}" {{ARGS}}

# Dispatch publish-to-crates.io workflow for a release version.
publish-release VERSION *ARGS="":
    @cargo xtask publish-release "{{VERSION}}" {{ARGS}}

# Manually publish the 4 new crates for v0.12.2 one at a time with 10-min gaps.
# Use when the automated workflow is blocked by crates.io's new-crate rate limit
# (burst=5, refill=1/10 min — separate from the update limit fixed in #3307).
# See docs/reference/MANUAL_PUBLISH_NEW_CRATES.md for full context.
#
# Dry run (safe, no actual publishing):
#   DRY_RUN=true just publish-new-crates
# Live run:
#   CARGO_REGISTRY_TOKEN=<token> just publish-new-crates
publish-new-crates:
    bash scripts/publish-new-crates-manually.sh

# Check that the hand-maintained publish allowlist ([workspace.metadata.publish.allow])
# matches the set of crates that cargo metadata considers publishable.
# Run this after adding a new crate to catch drift before pushing.
publish-allowlist-check:
    cargo xtask publish-manifest-check

# Dry-run publish gate: package every allowlisted crate in topological order.
# Mirrors the dev-dep strip and packaging steps from publish-crates.yml.
# Runs automatically in CI on every PR that touches Cargo.toml.
# Use this locally to verify your change won't break the publish workflow.
publish-dry-run:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== Publish dry-run gate ==="
    echo "Running publish-topo unit tests..."
    python3 scripts/tests/test-publish-topo.py -v

    echo ""
    echo "Computing topological publish order..."
    cargo metadata --format-version=1 --no-deps \
      | python3 scripts/publish-topo.py > /tmp/publish-dry-run-crates.json

    COUNT=$(python3 -c 'import json; print(len(json.load(open("/tmp/publish-dry-run-crates.json"))))')
    echo "  ${COUNT} crates in publish allowlist"

    STRIP_SCRIPT="$(mktemp --suffix=.py)"
    cat > "$STRIP_SCRIPT" << 'STRIP_SCRIPT_EOF'
    import re, sys, pathlib
    path = pathlib.Path(sys.argv[1])
    text = path.read_text(encoding="utf-8")
    new_text = re.sub(
        r"^\[dev-dependencies\].*?(?=^\[|\Z)",
        "",
        text,
        flags=re.MULTILINE | re.DOTALL,
    )
    path.write_text(new_text, encoding="utf-8")
    STRIP_SCRIPT_EOF

    MANIFEST_MAP="$(mktemp)"
    cargo metadata --format-version=1 --no-deps | python3 -c '
    import json, sys
    meta = json.load(sys.stdin)
    ws = set(meta["workspace_members"])
    for pkg in meta["packages"]:
        if pkg["id"] in ws:
            print("{} {}".format(pkg["name"], pkg["manifest_path"]))
    ' > "$MANIFEST_MAP"

    FAILED=""
    CRATES_LIST="$(mktemp)"
    python3 -c \
      'import json,sys; [print("{} {}".format(c["name"],c["version"])) for c in json.load(open("/tmp/publish-dry-run-crates.json"))]' \
      > "$CRATES_LIST"

    while read -r CRATE VERSION; do
      echo ""
      echo "===> Packaging $CRATE@$VERSION"
      MANIFEST_PATH="$(grep "^${CRATE} " "$MANIFEST_MAP" | awk '{print $2}')"
      if [ -z "$MANIFEST_PATH" ]; then
        echo "  ERROR: Could not find manifest path"
        FAILED="${FAILED} ${CRATE}"
        continue
      fi
      MANIFEST_BACKUP="$(mktemp)"
      cp "$MANIFEST_PATH" "$MANIFEST_BACKUP"
      # shellcheck disable=SC2064
      trap "cp '$MANIFEST_BACKUP' '$MANIFEST_PATH'; rm -f '$MANIFEST_BACKUP'" EXIT
      python3 "$STRIP_SCRIPT" "$MANIFEST_PATH"
      TARGET_DIR="/tmp/cargo-package-dry-run-${CRATE//\//-}"
      rm -rf "$TARGET_DIR"
      if CARGO_TARGET_DIR="$TARGET_DIR" CARGO_PACKAGE_NO_VERIFY=1 \
         bash scripts/cargo-package-workspace-dry-run.sh "$CRATE"; then
        echo "  OK"
      else
        echo "  FAILED"
        FAILED="${FAILED} ${CRATE}"
      fi
      cp "$MANIFEST_BACKUP" "$MANIFEST_PATH"
      rm -f "$MANIFEST_BACKUP"
      trap - EXIT
    done < "$CRATES_LIST"

    echo ""
    if [ -n "$FAILED" ]; then
      echo "ERROR: Packaging failed for:${FAILED}"
      echo "These crates would break the publish workflow. Fix before merging."
      exit 1
    fi
    echo "=== All ${COUNT} crates packaged successfully. ==="

# Run post-release installed-binary smoke test for a release version.
smoke-test-release VERSION:
    @cargo xtask smoke-test-release "{{VERSION}}"

# Verify a published version is installable and functional end-to-end.
# Installs binary crates, exercises library crates in a fresh downstream project,
# and runs tree-sitter-perl-c / perl-parser integration tests.
# Uses a clean tempdir — no Docker required.
#
# Example: just smoke-test 0.12.2
smoke-test VERSION:
    @bash scripts/post-publish-smoke.sh "{{VERSION}}"

# Release gate: full validation for release candidates (~10 min)
# Composes: ci-gate + release-specific checks
release-gate: ci-gate release-build sbom-verify version-check
    @echo "=============================================="
    @echo "  RELEASE GATE PASSED"
    @echo "=============================================="

# Extended release check: release-gate + semver + changelog + publish dry-run + panic audit
# Use before cutting a release tag. See docs/project/RELEASE_CHECKLIST.md
release-check: release-gate semver-check
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== Extended release checks ==="
    echo "Checking CHANGELOG.md for release section..."
    WORKSPACE_VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
    if ! grep -q "## \[${WORKSPACE_VERSION}\]" CHANGELOG.md; then
      echo "ERROR: CHANGELOG.md missing section for [${WORKSPACE_VERSION}]"
      echo "  Found [Unreleased] but need a versioned section before release."
      exit 1
    fi
    echo "  CHANGELOG.md has [${WORKSPACE_VERSION}] section"
    echo "Checking for banned panic constructs in production code..."
    PANIC_HITS=$(grep -rn --include='*.rs' \
      -e '\.unwrap()' -e '\.expect(' -e 'panic!(' \
      -e 'todo!(' -e 'unimplemented!(' -e 'dbg!(' \
      crates/*/src/ \
      --exclude-dir='tests' --exclude-dir='benches' \
      -- | grep -v '#\[allow' | grep -v '// allow' \
           | grep -v 'crates/perl-lsp/src/util/uri.rs' \
           | grep -v '#\[cfg(test)\]' || true)
    if [ -n "$PANIC_HITS" ]; then
      echo "WARNING: Potential panic constructs found in production code:"
      echo "$PANIC_HITS" | head -20
      echo "  (showing first 20 matches -- review before release)"
    else
      echo "  No banned panic constructs found"
    fi
    echo "Running crates.io launch prep..."
    cargo xtask prep-crates-io-launch --mode core
    echo "  Crates.io launch prep passed"
    echo "Building perl-dap release binary..."
    cargo build -p perl-dap --release --locked
    echo "  perl-dap release build passed"
    echo "=============================================="
    echo "  RELEASE CHECK PASSED"
    echo "  See docs/project/RELEASE_CHECKLIST.md for"
    echo "  manual verification steps before tagging."
    echo "=============================================="

# ============================================================================
# LSP Test Tiering (Slice D: tiered test execution)
# ============================================================================

# Tier A: fast smoke tests for perl-lsp (<30s)
# Run on every PR for quick feedback
lsp-tier-a:
    @echo "Running LSP Tier A (smoke tests)..."
    cargo test -p perl-lsp-rs --test cli_smoke --test lsp_capabilities_snapshot --test lsp_capabilities_contract --test lsp_protocol_tests --locked -- --test-threads=1
    @echo "LSP Tier A passed"

# Tier B: core behavior tests for perl-lsp (~2-5 min)
# Run at merge gate for thorough validation
lsp-tier-b: lsp-tier-a
    @echo "Running LSP Tier B (core behavior)..."
    env RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs \
        --test semantic_definition \
        --test lsp_completion_tests \
        --test lsp_unhappy_paths \
        --test lsp_code_actions_test \
        --test execute_command_security_tests \
        --test lsp_behavioral_tests \
        --test lsp_workspace_index_e2e \
        --locked -- --test-threads=2
    @echo "LSP Tier B passed"

# ============================================================================
# Parser Corpus Sweep
# ============================================================================

# Sweep system Perl corpus and print results
corpus-sweep:
    cargo run -p xtask -- parser-corpus-sweep

# Check corpus against baseline (fails on regression)
corpus-sweep-check:
    cargo run -p xtask -- parser-corpus-sweep \
        --baseline .ci/parser-corpus-baseline.json --enforce --receipt

# Check common-files corpus (strict: 0 errors, PR gate)
common-corpus-check:
    cargo run -p xtask -- parser-corpus-sweep \
        --manifest .ci/common-corpus-manifest.txt --enforce --receipt

# Update corpus baseline with current results
corpus-sweep-update:
    cargo run -p xtask -- parser-corpus-sweep \
        --output .ci/parser-corpus-baseline.json

# ============================================================================
# CPAN Corpus Management
# ============================================================================

# Fetch CPAN top 1000 distribution list from MetaCPAN
cpan-corpus-fetch:
    cargo run -p xtask -- cpan-corpus fetch-list

# Install CPAN top 1000 distributions locally
cpan-corpus-install:
    cargo run -p xtask -- cpan-corpus install

# Sweep CPAN corpus and print results
cpan-corpus-sweep:
    cargo run -p xtask -- cpan-corpus sweep

# Bootstrap/update the committed CPAN corpus baseline
cpan-corpus-baseline-update:
    cargo run -p xtask -- cpan-corpus sweep --output .ci/cpan-corpus-baseline.json

# Check CPAN corpus against baseline and known-clean manifest (fails on regression)
cpan-corpus-check:
    cargo run -p xtask -- cpan-corpus sweep --enforce

# Auto-add newly-clean CPAN modules to known-clean manifest
cpan-corpus-ratchet:
    cargo run -p xtask -- cpan-corpus ratchet

# ============================================================================
# Scorecard Metrics Ratchet
# ============================================================================

# Check scorecard floor metrics for a single subsystem (soft gate: warn, not block)
ci-metrics-ratchet-check subsystem:
    @echo "Checking scorecard floor metrics for {{subsystem}}..."
    cargo run -p xtask -- metrics ratchet-check {{subsystem}}
    @echo "Scorecard ratchet passed for {{subsystem}}"

# Check all committed scorecard baselines (parser + engineering_health).
# Soft gate: run after ci-gate, warns on violation, does not block PR in v1.
ci-metrics-ratchet:
    @echo "Checking scorecard floor metrics..."
    cargo run -p xtask -- metrics ratchet-check parser
    cargo run -p xtask -- metrics ratchet-check engineering_health
    @echo "Scorecard ratchet passed"

# Tier C: full suite (nightly, all integration tests)
lsp-tier-c:
    @echo "Running LSP Tier C (full suite)..."
    env RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --locked -- --test-threads=2
    @echo "LSP Tier C passed"

# ============================================================================
# Worktree Cleanup
# ============================================================================

# Clean up stale agent worktrees (safe — only removes worktrees with no uncommitted changes and no open PR)
clean-worktrees:
    #!/usr/bin/env bash
    set -euo pipefail
    repo_name=$(basename "$PWD")
    managed_root="$(dirname "$PWD")/${repo_name}-worktrees"
    echo "Pruning unreferenced worktrees..."
    git worktree prune
    echo "Checking ${managed_root}/ for stale entries..."
    removed=0
    kept=0
    for wt in "${managed_root}"/*/; do
        [ -d "$wt" ] || continue
        name=$(basename "$wt")
        # Check for uncommitted changes
        if [ -n "$(git -C "$wt" status --porcelain 2>/dev/null)" ]; then
            echo "  KEEP $name (has uncommitted changes)"
            kept=$((kept + 1))
            continue
        fi
        # Check for open PR
        branch=$(git -C "$wt" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
        if [ -n "$branch" ] && [ "$branch" != "HEAD" ]; then
            pr_count=$(gh pr list --head "$branch" --state open --json number --jq length 2>/dev/null || echo "0")
            if [ "$pr_count" -gt 0 ]; then
                echo "  KEEP $name (has open PR on $branch)"
                kept=$((kept + 1))
                continue
            fi
        fi
        echo "  REMOVE $name"
        git worktree remove --force "$wt" 2>/dev/null || rm -rf "$wt"
        removed=$((removed + 1))
    done
    git worktree prune
    echo "Done: removed $removed, kept $kept"

# Query, allocate, release, or clean up reusable worktree slots
worktree-manager *ARGS:
    python3 scripts/worktree-manager.py {{ARGS}}
