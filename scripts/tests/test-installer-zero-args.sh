#!/usr/bin/env bash
# Self-test for the ZERO-ARGUMENT installer bootstrap path.
#
# Why this test exists (EffortlessMetrics/perl-lsp-swarm#5448):
#
# The documented bootstrap is
#
#     curl -fsSL https://raw.githubusercontent.com/.../install.sh | bash
#
# which is a zero-argument invocation of a script read from stdin, running
# under `set -euo pipefail`. Two `set -u` hazards live on exactly that path and
# neither was covered by any gate:
#
#   1. `${BASH_SOURCE[0]}` is UNBOUND when the script is read from stdin. This
#      aborts on every bash version, including current ones — the documented
#      curl-pipe command was broken on all platforms.
#   2. `"${ARGS[@]}"` on an EMPTY array is an unbound-variable error on
#      bash < 4.4. macOS ships /bin/bash 3.2.57, so the zero-argument path
#      aborted there even when invoked as a file.
#
# Hazard 1 reproduces on any bash and is asserted directly below. Hazard 2 does
# NOT reproduce on bash >= 4.4 (the expansion was made legal in bash 4.4), so it
# is covered two ways:
#
#   - a static assertion that every array expansion in the installers uses the
#     `${arr[@]+"${arr[@]}"}` guard rather than a bare `"${arr[@]}"`; and
#   - a real execution under a bash 3.2 container when Docker is available
#     (skipped, not passed, when it is not).
#
# The test never downloads a release artifact: it stubs the canonical installer
# and `curl`.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ROOT_INSTALLER="$ROOT/install.sh"
CANONICAL_INSTALLER="$ROOT/scripts/install.sh"
LEGACY_BASH_IMAGE="${PERL_LSP_LEGACY_BASH_IMAGE:-bash:3.2}"

PASS=0
FAIL=0
SKIP=0
WORKDIR=""

cleanup() {
    if [[ -n "${WORKDIR:-}" && -d "$WORKDIR" ]]; then
        rm -rf "$WORKDIR"
    fi
}
trap cleanup EXIT

pass() {
    printf 'PASS  %s\n' "$1"
    PASS=$((PASS + 1))
}

fail() {
    printf 'FAIL  %s\n' "$1"
    printf '      %s\n' "$2"
    FAIL=$((FAIL + 1))
}

skip() {
    printf 'SKIP  %s\n' "$1"
    SKIP=$((SKIP + 1))
}

WORKDIR="$(mktemp -d)"

# A stub that stands in for scripts/install.sh. It reports how many arguments
# the wrapper forwarded, so a zero-argument bootstrap is observable without
# touching the network.
STUB="$WORKDIR/stub-install.sh"
cat > "$STUB" <<'STUB_EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'STUB argc=%s\n' "$#"
printf 'STUB VERSION=%s INSTALL_DIR=%s\n' "${VERSION:-}" "${INSTALL_DIR:-}"
STUB_EOF
chmod +x "$STUB"

# A stub `curl` that serves the stub installer instead of raw.githubusercontent.
FAKEBIN="$WORKDIR/bin"
mkdir -p "$FAKEBIN"
cat > "$FAKEBIN/curl" <<'CURL_EOF'
#!/usr/bin/env bash
set -euo pipefail
out=""
while [[ "$#" -gt 0 ]]; do
    case "$1" in
        -o) out="$2"; shift 2 ;;
        *) shift ;;
    esac
done
if [[ -z "$out" ]]; then
    echo "fake curl expected -o <path>" >&2
    exit 2
fi
cp "$PERL_LSP_TEST_STUB_INSTALLER" "$out"
CURL_EOF
chmod +x "$FAKEBIN/curl"

# A checkout-shaped tree whose sibling scripts/install.sh is the stub.
CHECKOUT="$WORKDIR/checkout"
mkdir -p "$CHECKOUT/scripts"
cp "$ROOT_INSTALLER" "$CHECKOUT/install.sh"
cp "$STUB" "$CHECKOUT/scripts/install.sh"

# ── 1. curl-pipe, zero arguments (the exact documented command) ────────────────

test_curl_pipe_zero_args() {
    local label="curl-pipe zero-arg bootstrap succeeds under set -u"
    local out="$WORKDIR/out-pipe.txt"
    local status=0

    set +e
    env PATH="$FAKEBIN:$PATH" PERL_LSP_TEST_STUB_INSTALLER="$STUB" \
        bash < "$ROOT_INSTALLER" >"$out" 2>&1
    status=$?
    set -e

    if [[ "$status" -ne 0 ]]; then
        fail "$label" "expected exit 0, got $status; output: $(cat "$out")"
        return
    fi

    if ! grep -q 'STUB argc=0' "$out"; then
        fail "$label" "expected the wrapper to forward zero arguments; output: $(cat "$out")"
        return
    fi

    pass "$label"
}

# ── 2. file invocation, zero arguments, local canonical installer present ──────

test_file_zero_args() {
    local label="file zero-arg invocation forwards no arguments"
    local out="$WORKDIR/out-file.txt"
    local status=0

    set +e
    bash "$CHECKOUT/install.sh" >"$out" 2>&1
    status=$?
    set -e

    if [[ "$status" -ne 0 ]]; then
        fail "$label" "expected exit 0, got $status; output: $(cat "$out")"
        return
    fi

    if ! grep -q 'STUB argc=0' "$out"; then
        fail "$label" "expected argc=0; output: $(cat "$out")"
        return
    fi

    pass "$label"
}

# ── 3. positional arguments still map to VERSION / INSTALL_DIR ────────────────

test_positional_args_still_work() {
    local label="positional version and install dir still become env vars"
    local out="$WORKDIR/out-args.txt"
    local status=0

    set +e
    bash "$CHECKOUT/install.sh" 0.17.0 /tmp/perl-lsp-selftest-bin >"$out" 2>&1
    status=$?
    set -e

    if [[ "$status" -ne 0 ]]; then
        fail "$label" "expected exit 0, got $status; output: $(cat "$out")"
        return
    fi

    if ! grep -q 'STUB VERSION=0.17.0 INSTALL_DIR=/tmp/perl-lsp-selftest-bin' "$out"; then
        fail "$label" "positional mapping regressed; output: $(cat "$out")"
        return
    fi

    if ! grep -q 'STUB argc=0' "$out"; then
        fail "$label" "consumed positionals must not be forwarded; output: $(cat "$out")"
        return
    fi

    pass "$label"
}

# ── 4. flag arguments are forwarded verbatim ─────────────────────────────────

test_flag_args_forwarded() {
    local label="flag arguments are forwarded to the canonical installer"
    local out="$WORKDIR/out-flag.txt"
    local status=0

    set +e
    bash "$CHECKOUT/install.sh" --print-target >"$out" 2>&1
    status=$?
    set -e

    if [[ "$status" -ne 0 ]]; then
        fail "$label" "expected exit 0, got $status; output: $(cat "$out")"
        return
    fi

    if ! grep -q 'STUB argc=1' "$out"; then
        fail "$label" "expected argc=1; output: $(cat "$out")"
        return
    fi

    pass "$label"
}

# ── 5. static guard: no bare "${arr[@]}" expansion in the installers ──────────
#
# bash >= 4.4 accepts a bare `"${arr[@]}"` on an empty array, so no amount of
# running these scripts on a modern bash can catch a regression of the macOS
# bash 3.2 abort. Assert the guarded spelling directly.

test_no_unguarded_array_expansion() {
    local label="installers use \${arr[@]+\"\${arr[@]}\"} (bash 3.2 set -u safety)"
    local offenders=""

    local file
    for file in "$ROOT_INSTALLER" "$CANONICAL_INSTALLER"; do
        # Strip comments first, so documentation of the hazard does not register
        # as the hazard. Then strip the *guarded* spelling
        # `${arr[@]+"${arr[@]}"}` — it legitimately contains a `"${arr[@]}"`
        # substring — so only genuinely bare expansions remain.
        local hits
        hits="$(sed -e 's/[[:space:]]*#.*$//' \
                    -e 's/\${\([A-Za-z_][A-Za-z_0-9]*\)\[@\]+"\${\1\[@\]}"}/<GUARDED>/g' \
                    "$file" \
            | grep -n '"\${[A-Za-z_][A-Za-z_0-9]*\[@\]}"' || true)"
        if [[ -n "$hits" ]]; then
            offenders+="${file}:
${hits}
"
        fi
    done

    if [[ -n "$offenders" ]]; then
        fail "$label" "bare array expansions found (use \${arr[@]+\"\${arr[@]}\"}):
$offenders"
        return
    fi

    pass "$label"
}

# ── 6. static guard: no bare ${BASH_SOURCE[0]} ───────────────────────────────

test_bash_source_guarded() {
    local label="wrapper guards \${BASH_SOURCE[0]} for stdin invocation"

    local hits
    hits="$(sed 's/[[:space:]]*#.*$//' "$ROOT_INSTALLER" \
        | grep -n 'BASH_SOURCE\[0\]}' | grep -v 'BASH_SOURCE\[0\]:-' || true)"

    if [[ -n "$hits" ]]; then
        fail "$label" "unguarded BASH_SOURCE[0] expansion (curl-pipe aborts under set -u):
$hits"
        return
    fi

    pass "$label"
}

# ── 7. real bash 3.2 execution when Docker is available ──────────────────────

test_legacy_bash_container() {
    local label="zero-arg bootstrap runs under bash 3.2 ($LEGACY_BASH_IMAGE)"

    if ! command -v docker >/dev/null 2>&1; then
        skip "$label (docker not installed)"
        return
    fi

    if ! docker info >/dev/null 2>&1; then
        skip "$label (docker daemon unavailable)"
        return
    fi

    if ! docker pull "$LEGACY_BASH_IMAGE" >/dev/null 2>&1; then
        skip "$label (could not pull $LEGACY_BASH_IMAGE)"
        return
    fi

    local out="$WORKDIR/out-legacy.txt"
    local status=0

    set +e
    docker run --rm -v "$CHECKOUT:/checkout:ro" -w /checkout \
        --entrypoint bash "$LEGACY_BASH_IMAGE" \
        -c 'echo "bash=$BASH_VERSION"; bash /checkout/install.sh' >"$out" 2>&1
    status=$?
    set -e

    if [[ "$status" -ne 0 ]]; then
        fail "$label" "expected exit 0 under legacy bash, got $status; output: $(cat "$out")"
        return
    fi

    if ! grep -q 'STUB argc=0' "$out"; then
        fail "$label" "expected argc=0 under legacy bash; output: $(cat "$out")"
        return
    fi

    printf '      %s\n' "$(grep '^bash=' "$out" || true)"
    pass "$label"
}

if [[ ! -f "$ROOT_INSTALLER" ]]; then
    fail "root installer exists" "missing $ROOT_INSTALLER"
elif [[ ! -f "$CANONICAL_INSTALLER" ]]; then
    fail "canonical installer exists" "missing $CANONICAL_INSTALLER"
else
    test_curl_pipe_zero_args
    test_file_zero_args
    test_positional_args_still_work
    test_flag_args_forwarded
    test_no_unguarded_array_expansion
    test_bash_source_guarded
    test_legacy_bash_container
fi

echo
echo "-- Summary --"
echo "Passed: $PASS"
echo "Failed: $FAIL"
echo "Skipped: $SKIP"

if [[ "$FAIL" -gt 0 ]]; then
    exit 1
fi
