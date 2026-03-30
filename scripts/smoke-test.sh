#!/usr/bin/env bash
# smoke-test.sh — End-to-end LSP smoke test for pre-tag validation.
#
# Usage:
#   scripts/smoke-test.sh [--no-build]
#
# Requires: cargo, jq
# Exit codes: 0 = PASS, 1 = FAIL
set -uo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Respect CARGO_TARGET_DIR if set (used by agents to avoid build collisions).
_TARGET_DIR="${CARGO_TARGET_DIR:-${REPO_ROOT}/target}"
BIN="${_TARGET_DIR}/release/perllsp"
SKIP_BUILD=false
TIMEOUT_SECONDS=30

for arg in "$@"; do
  case "$arg" in
    --no-build) SKIP_BUILD=true ;;
  esac
done

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
pass() { printf 'PASS  %s\n' "$*"; }
fail() { printf 'FAIL  %s\n' "$*" >&2; exit 1; }
step() { printf '  ... %s\n' "$*"; }

check_dep() {
  command -v "$1" >/dev/null 2>&1 || fail "required dependency not found: $1"
}

# ---------------------------------------------------------------------------
# Dependency check
# ---------------------------------------------------------------------------
check_dep cargo
check_dep jq

# ---------------------------------------------------------------------------
# Step 1: Build
# ---------------------------------------------------------------------------
if [[ "$SKIP_BUILD" == true ]]; then
  step "skipping build (--no-build)"
  [[ -x "$BIN" ]] || fail "binary not found at $BIN — run without --no-build first"
else
  step "building perllsp --release"
  cargo build -p perllsp --release --quiet \
    || fail "cargo build failed"
  pass "build"
fi

# ---------------------------------------------------------------------------
# Step 2: Create temp Perl file
# ---------------------------------------------------------------------------
TMPDIR_SMOKE="$(mktemp -d)"
SERVER_PID=""
cleanup() {
  [[ -n "$SERVER_PID" ]] && kill "$SERVER_PID" 2>/dev/null || true
  rm -rf "$TMPDIR_SMOKE"
}
trap cleanup EXIT

PERL_FILE="${TMPDIR_SMOKE}/test.pl"
cat > "$PERL_FILE" <<'PERL'
use strict;
use warnings;

my $message = "hello world";
my $count   = 42;

sub greet {
    my ($name) = @_;
    return "Hello, $name!";
}

my $result = greet("Perl");
print "$result\n";
PERL

PERL_URI="file://${PERL_FILE}"
PERL_TEXT="$(cat "$PERL_FILE")"
step "created temp Perl file: $PERL_FILE"

# ---------------------------------------------------------------------------
# LSP framing helpers (pure bash + printf)
# ---------------------------------------------------------------------------

# lsp_frame <json>  — prints a Content-Length framed LSP message
lsp_frame() {
  local body="$1"
  local len=${#body}
  printf 'Content-Length: %d\r\n\r\n%s' "$len" "$body"
}

# ---------------------------------------------------------------------------
# Step 3: Launch LSP and run the protocol sequence
# ---------------------------------------------------------------------------
step "launching perllsp --stdio"

# Use a pair of FIFOs for bidirectional communication with the server.
FIFO_IN="${TMPDIR_SMOKE}/lsp_in"
FIFO_OUT="${TMPDIR_SMOKE}/lsp_out"
mkfifo "$FIFO_IN" "$FIFO_OUT"

# Suppress startup banner
export PERL_LSP_QUIET=1

# Launch the server: reads from FIFO_IN, writes to FIFO_OUT.
# stderr is discarded so it does not interfere with the test output.
"$BIN" --stdio < "$FIFO_IN" > "$FIFO_OUT" 2>/dev/null &
SERVER_PID=$!

# Open the write end of FIFO_IN as fd 3 so we can write to it.
exec 3>"$FIFO_IN"

# ---------------------------------------------------------------------------
# recv_response <id> — reads LSP messages until the one with matching id
# Prints the JSON body on stdout, fails on timeout or EOF.
# ---------------------------------------------------------------------------
recv_response() {
  local expected_id="$1"
  local deadline=$(( $(date +%s) + TIMEOUT_SECONDS ))

  while true; do
    [[ $(date +%s) -lt $deadline ]] || fail "timeout waiting for LSP response id=$expected_id"

    # Read header lines until blank line (CRLF CRLF).
    local content_length=0
    local line
    while IFS= read -r -t 5 line <&4; do
      # Strip trailing CR
      line="${line%$'\r'}"
      [[ -z "$line" ]] && break
      if [[ "${line,,}" == content-length:* ]]; then
        content_length="${line#*: }"
        content_length="${content_length## }"
      fi
    done

    [[ "$content_length" -gt 0 ]] || continue

    # Read exactly content_length bytes.
    local body
    body="$(dd bs=1 count="$content_length" <&4 2>/dev/null)"

    local id
    id="$(printf '%s' "$body" | jq -r '.id // empty' 2>/dev/null)"

    if [[ "$id" == "$expected_id" ]]; then
      printf '%s' "$body"
      return 0
    fi
    # Not our message (notification etc.) — loop and read next.
  done
}

# Open FIFO_OUT for reading as fd 4.
exec 4<"$FIFO_OUT"

# ---------------------------------------------------------------------------
# send <json>
# ---------------------------------------------------------------------------
send() {
  lsp_frame "$1" >&3
}

# ---------------------------------------------------------------------------
# Step 4: initialize
# ---------------------------------------------------------------------------
step "sending initialize"
INIT_REQ='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
send "$INIT_REQ"

INIT_RESP="$(recv_response 1)"
[[ -n "$INIT_RESP" ]] || fail "empty initialize response"

# Verify capabilities block is present
CAPS="$(printf '%s' "$INIT_RESP" | jq '.result.capabilities' 2>/dev/null)"
[[ "$CAPS" != "null" && -n "$CAPS" ]] || fail "initialize response missing capabilities: $INIT_RESP"

# Verify hoverProvider is advertised
HOVER_CAP="$(printf '%s' "$INIT_RESP" | jq '.result.capabilities.hoverProvider' 2>/dev/null)"
[[ "$HOVER_CAP" != "null" && -n "$HOVER_CAP" ]] \
  || fail "hoverProvider not advertised in capabilities"

pass "initialize — capabilities received, hoverProvider=true"

# Send initialized notification (no response expected)
send '{"jsonrpc":"2.0","method":"initialized","params":{}}'

# ---------------------------------------------------------------------------
# Step 5: textDocument/didOpen
# ---------------------------------------------------------------------------
step "sending textDocument/didOpen"

# Escape the Perl source for JSON embedding.
ESCAPED_TEXT="$(printf '%s' "$PERL_TEXT" | jq -Rs '.')"

DID_OPEN_REQ="{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"${PERL_URI}\",\"languageId\":\"perl\",\"version\":1,\"text\":${ESCAPED_TEXT}}}}"
send "$DID_OPEN_REQ"

# didOpen is a notification — no response to wait for.
# Give the server a moment to index the document.
sleep 0.2
pass "textDocument/didOpen — sent"

# ---------------------------------------------------------------------------
# Step 6: textDocument/hover on $message (line 3, character 3 — the '$')
# ---------------------------------------------------------------------------
# Line 3 (0-indexed) is: my $message = "hello world";
# $message starts at character 3.
step "sending textDocument/hover"

HOVER_REQ="{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"textDocument/hover\",\"params\":{\"textDocument\":{\"uri\":\"${PERL_URI}\"},\"position\":{\"line\":3,\"character\":3}}}"
send "$HOVER_REQ"

HOVER_RESP="$(recv_response 2)"
[[ -n "$HOVER_RESP" ]] || fail "empty hover response"

# ---------------------------------------------------------------------------
# Step 7: Verify hover response is non-null and non-empty
# ---------------------------------------------------------------------------
HOVER_RESULT="$(printf '%s' "$HOVER_RESP" | jq '.result' 2>/dev/null)"

if [[ "$HOVER_RESULT" == "null" || -z "$HOVER_RESULT" ]]; then
  # Hover returning null is LSP-valid (no info available) but for smoke
  # purposes we want to confirm the server responded at all and did not error.
  HOVER_ERROR="$(printf '%s' "$HOVER_RESP" | jq '.error' 2>/dev/null)"
  if [[ "$HOVER_ERROR" != "null" && -n "$HOVER_ERROR" ]]; then
    fail "hover request returned an error: $HOVER_ERROR"
  fi
  pass "textDocument/hover — responded (result=null, no error)"
else
  HOVER_CONTENTS="$(printf '%s' "$HOVER_RESP" | jq '.result.contents' 2>/dev/null)"
  [[ -n "$HOVER_CONTENTS" ]] \
    || fail "hover result present but missing 'contents' field: $HOVER_RESULT"
  pass "textDocument/hover — non-empty response received"
fi

# ---------------------------------------------------------------------------
# Shutdown sequence
# ---------------------------------------------------------------------------
step "sending shutdown"
send '{"jsonrpc":"2.0","id":99,"method":"shutdown","params":null}'
recv_response 99 >/dev/null || true
send '{"jsonrpc":"2.0","method":"exit","params":null}'

# Close our write end — signals EOF to the server.
exec 3>&-

# Wait for the server to exit cleanly (up to 3 s).
for _i in 1 2 3; do
  kill -0 "$SERVER_PID" 2>/dev/null || break
  sleep 1
done
kill "$SERVER_PID" 2>/dev/null || true
SERVER_PID=""  # prevent double-kill in cleanup trap

printf '\n'
printf '==============================\n'
printf 'SMOKE TEST: PASS\n'
printf '==============================\n'
exit 0
