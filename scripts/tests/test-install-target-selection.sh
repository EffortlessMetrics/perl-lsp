#!/usr/bin/env bash
# Self-test for Linux installer target selection.
#
# The installer is intentionally shell because it is the curl-pipe/bootstrap
# entrypoint. This test locks the target-selection contract without downloading
# release artifacts by using --print-target.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ROOT_INSTALLER="$ROOT/install.sh"
CANONICAL_INSTALLER="$ROOT/scripts/install.sh"

PASS=0
FAIL=0
SKIP=0
TMPDIR_BASE=""

cleanup() {
    if [[ -n "${TMPDIR_BASE:-}" && -d "$TMPDIR_BASE" ]]; then
        rm -rf "$TMPDIR_BASE"
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

assert_stdout() {
    local label="$1"
    local expected="$2"
    shift 2

    local output status
    set +e
    output="$("$@" 2>&1)"
    status=$?
    set -e

    if [[ "$status" -ne 0 ]]; then
        fail "$label" "expected exit 0, got $status; output: $output"
        return
    fi

    if [[ "$output" != "$expected" ]]; then
        fail "$label" "expected '$expected', got '$output'"
        return
    fi

    pass "$label"
}

assert_bad_override_fails() {
    local output status
    set +e
    output="$(env PERL_LSP_LINUX_LIBC=bad bash "$CANONICAL_INSTALLER" --print-target 2>&1)"
    status=$?
    set -e

    if [[ "$status" -eq 0 ]]; then
        fail "bad PERL_LSP_LINUX_LIBC override fails" "expected non-zero exit, got 0"
        return
    fi

    if [[ "$output" != *"invalid PERL_LSP_LINUX_LIBC=bad; expected auto, gnu, glibc, or musl"* ]]; then
        fail "bad PERL_LSP_LINUX_LIBC override explains valid values" "unexpected output: $output"
        return
    fi

    pass "bad PERL_LSP_LINUX_LIBC override fails clearly"
}

host_arch() {
    case "$(uname -m)" in
        x86_64|amd64|x64) printf '%s\n' "x86_64" ;;
        aarch64|arm64) printf '%s\n' "aarch64" ;;
        *) return 1 ;;
    esac
}

host_auto_libc() {
    if command -v ldd >/dev/null 2>&1; then
        local ldd_output
        ldd_output="$(ldd --version 2>&1 || true)"
        if printf '%s\n' "$ldd_output" | grep -qi musl; then
            printf '%s\n' "musl"
            return
        fi
        if printf '%s\n' "$ldd_output" | grep -Eqi 'glibc|gnu libc'; then
            printf '%s\n' "gnu"
            return
        fi
    fi

    if command -v getconf >/dev/null 2>&1 && getconf GNU_LIBC_VERSION >/dev/null 2>&1; then
        printf '%s\n' "gnu"
        return
    fi

    if [[ -f /etc/alpine-release ]]; then
        printf '%s\n' "musl"
        return
    fi

    printf '%s\n' "gnu"
}

test_root_wrapper_fetch_fallback() {
    local expected="$1"
    TMPDIR_BASE="$(mktemp -d)"
    local fakebin="$TMPDIR_BASE/bin"
    local isolated="$TMPDIR_BASE/isolated"
    mkdir -p "$fakebin" "$isolated"

    cp "$ROOT_INSTALLER" "$isolated/install.sh"
    cat > "$fakebin/curl" <<'FAKE_CURL'
#!/usr/bin/env bash
set -euo pipefail

out=""
while [[ "$#" -gt 0 ]]; do
    case "$1" in
        -o)
            out="$2"
            shift 2
            ;;
        *)
            shift
            ;;
    esac
done

if [[ -z "$out" ]]; then
    echo "fake curl expected -o <path>" >&2
    exit 2
fi

cp "$PERL_LSP_TEST_CANONICAL_INSTALLER" "$out"
FAKE_CURL
    chmod +x "$fakebin/curl"

    assert_stdout \
        "root install.sh fallback fetches canonical installer outside checkout" \
        "$expected" \
        env PATH="$fakebin:$PATH" PERL_LSP_TEST_CANONICAL_INSTALLER="$CANONICAL_INSTALLER" \
        bash "$isolated/install.sh" --print-target
}

test_alpine_docker() {
    local arch="$1"

    if ! command -v docker >/dev/null 2>&1; then
        skip "Alpine Docker target selection (docker not installed)"
        return
    fi

    if ! docker info >/dev/null 2>&1; then
        skip "Alpine Docker target selection (docker daemon unavailable)"
        return
    fi

    if ! docker pull alpine:latest >/dev/null 2>&1; then
        fail "Alpine Docker target selection" "failed to pull alpine:latest"
        return
    fi

    assert_stdout \
        "Alpine Docker auto-selects musl" \
        "${arch}-unknown-linux-musl" \
        docker run --rm -v "$ROOT:/repo:ro" -w /repo alpine:latest \
            sh -lc 'apk add --no-cache bash >/dev/null && bash scripts/install.sh --print-target'
}

if [[ ! -f "$ROOT_INSTALLER" ]]; then
    fail "root installer exists" "missing $ROOT_INSTALLER"
fi

if [[ ! -f "$CANONICAL_INSTALLER" ]]; then
    fail "canonical installer exists" "missing $CANONICAL_INSTALLER"
fi

if [[ "$(uname -s)" != "Linux" ]]; then
    skip "Linux target-selection checks (host is $(uname -s))"
else
    if arch="$(host_arch)"; then
        auto_libc="$(host_auto_libc)"
        auto_target="${arch}-unknown-linux-${auto_libc}"
        gnu_target="${arch}-unknown-linux-gnu"
        musl_target="${arch}-unknown-linux-musl"

        assert_stdout "root install.sh --print-target follows canonical installer" \
            "$auto_target" bash "$ROOT_INSTALLER" --print-target
        assert_stdout "scripts/install.sh auto target" \
            "$auto_target" bash "$CANONICAL_INSTALLER" --print-target
        assert_stdout "PERL_LSP_LINUX_LIBC=gnu target" \
            "$gnu_target" env PERL_LSP_LINUX_LIBC=gnu bash "$CANONICAL_INSTALLER" --print-target
        assert_stdout "PERL_LSP_LINUX_LIBC=glibc target" \
            "$gnu_target" env PERL_LSP_LINUX_LIBC=glibc bash "$CANONICAL_INSTALLER" --print-target
        assert_stdout "PERL_LSP_LINUX_LIBC=musl target" \
            "$musl_target" env PERL_LSP_LINUX_LIBC=musl bash "$CANONICAL_INSTALLER" --print-target
        assert_bad_override_fails
        test_root_wrapper_fetch_fallback "$auto_target"
        test_alpine_docker "$arch"
    else
        skip "Linux target-selection checks (unsupported host arch: $(uname -m))"
    fi
fi

echo
echo "-- Summary --"
echo "Passed: $PASS"
echo "Failed: $FAIL"
echo "Skipped: $SKIP"

if [[ "$FAIL" -gt 0 ]]; then
    exit 1
fi
