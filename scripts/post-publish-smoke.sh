#!/usr/bin/env bash
# post-publish-smoke.sh — verify that published crates are installable and functional
#
# Usage:
#   scripts/post-publish-smoke.sh <version>
#
# Example:
#   scripts/post-publish-smoke.sh 0.12.2
#   scripts/post-publish-smoke.sh 0.13.0
#
# What this checks (the "indexed != installable" gap):
#   1. Binary crates can be installed via `cargo install`
#   2. Installed binaries execute and report the expected version
#   3. A downstream project can depend on library crates via `cargo add`
#   4. The tree-sitter-perl-c facade works end-to-end in a consumer project
#   5. perl-parser public API compiles and produces non-empty ASTs
#
# Scope: top-10 highest-value crates (binaries + key libraries).  Not every
# crate in the publish allowlist — that would take too long.
#
# Exit codes:
#   0  — all checks passed
#   1  — one or more checks failed (failures are accumulated; all checks run)

set -uo pipefail

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RESET='\033[0m'

PASS_COUNT=0
FAIL_COUNT=0
FAILURES=()

pass() {
    PASS_COUNT=$((PASS_COUNT + 1))
    printf "${GREEN}OK${RESET}  %s\n" "$*"
}

fail() {
    FAIL_COUNT=$((FAIL_COUNT + 1))
    FAILURES+=("$*")
    printf "${RED}FAIL${RESET} %s\n" "$*"
}

section() {
    printf "\n${BLUE}=== %s ===${RESET}\n" "$*"
}

die() {
    printf "${RED}error: %s${RESET}\n" "$*" >&2
    exit 1
}

usage() {
    cat <<'USAGE'
Usage:
  scripts/post-publish-smoke.sh <version>

Arguments:
  version    Semver string (e.g. 0.12.2, 0.13.0)

Environment variables:
  SMOKE_INSTALL_ROOT    Override install dir  (default: auto tempdir)
  SMOKE_WORK_DIR        Override work dir     (default: auto tempdir)
  SKIP_INSTALL          Set to 1 to skip cargo install steps (faster local iteration)
  CARGO_INSTALL_OPTS    Extra flags passed to every `cargo install` call

Examples:
  scripts/post-publish-smoke.sh 0.12.2
  SKIP_INSTALL=1 scripts/post-publish-smoke.sh 0.12.2    # assume already installed
USAGE
}

# ---------------------------------------------------------------------------
# Argument validation
# ---------------------------------------------------------------------------

if [[ $# -eq 0 || "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    usage
    [[ $# -eq 0 ]] && exit 1
    exit 0
fi

VERSION="$1"
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
    die "invalid semver: '$VERSION'. Expected form like '0.12.2'."
fi

printf "post-publish smoke test — version %s\n" "$VERSION"
printf "Started: %s\n\n" "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"

# ---------------------------------------------------------------------------
# Directory setup
# ---------------------------------------------------------------------------

CLEANUP_DIRS=()

setup_dirs() {
    if [[ -n "${SMOKE_INSTALL_ROOT:-}" ]]; then
        INSTALL_ROOT="$SMOKE_INSTALL_ROOT"
        printf "Using provided INSTALL_ROOT: %s\n" "$INSTALL_ROOT"
    else
        INSTALL_ROOT="$(mktemp -d)"
        CLEANUP_DIRS+=("$INSTALL_ROOT")
    fi

    if [[ -n "${SMOKE_WORK_DIR:-}" ]]; then
        WORK_DIR="$SMOKE_WORK_DIR"
        printf "Using provided WORK_DIR: %s\n" "$WORK_DIR"
    else
        WORK_DIR="$(mktemp -d)"
        CLEANUP_DIRS+=("$WORK_DIR")
    fi

    mkdir -p "$INSTALL_ROOT/bin"
    export PATH="$INSTALL_ROOT/bin:$PATH"

    # Isolate cargo home so we don't pollute the developer's cache
    export CARGO_HOME="$WORK_DIR/cargo-home"
    mkdir -p "$CARGO_HOME"

    printf "INSTALL_ROOT: %s\n" "$INSTALL_ROOT"
    printf "WORK_DIR:     %s\n" "$WORK_DIR"
    printf "CARGO_HOME:   %s\n\n" "$CARGO_HOME"
}

cleanup() {
    for dir in "${CLEANUP_DIRS[@]:-}"; do
        if [[ -d "$dir" ]]; then
            rm -rf "$dir"
        fi
    done
}
trap cleanup EXIT

setup_dirs

SKIP_INSTALL="${SKIP_INSTALL:-0}"
CARGO_INSTALL_OPTS="${CARGO_INSTALL_OPTS:-}"

# ---------------------------------------------------------------------------
# Section 1: Binary crates — install + version check
# ---------------------------------------------------------------------------

section "Binary crate installation"

# Target binaries and their expected --version output pattern (ERE regex).
# Pattern must match against the first line of `<binary> --version`.
declare -A BINARY_CRATES=(
    ["perllsp"]="perllsp"
    ["perl-dap"]="perl-dap"
)
declare -A BINARY_VERSION_PATTERN=(
    ["perllsp"]="${VERSION}"
    ["perl-dap"]="${VERSION}"
)
declare -A BINARY_HELP_CHECK=(
    ["perllsp"]="1"
    ["perl-dap"]="1"
)

for BIN_CRATE in "${!BINARY_CRATES[@]}"; do
    BIN_NAME="${BINARY_CRATES[$BIN_CRATE]}"
    PATTERN="${BINARY_VERSION_PATTERN[$BIN_CRATE]}"

    if [[ "$SKIP_INSTALL" != "1" ]]; then
        printf "Installing %s@%s...\n" "$BIN_CRATE" "$VERSION"
        # shellcheck disable=SC2086
        if cargo install "$BIN_CRATE" \
            --version "$VERSION" \
            --locked \
            --root "$INSTALL_ROOT" \
            $CARGO_INSTALL_OPTS \
            2>&1; then
            pass "cargo install $BIN_CRATE@$VERSION"
        else
            fail "cargo install $BIN_CRATE@$VERSION failed"
            continue
        fi
    fi

    # Version output check
    if command -v "$BIN_NAME" >/dev/null 2>&1; then
        BIN_VERSION_OUTPUT="$("$BIN_NAME" --version 2>&1 | head -n 1)"
        if [[ "$BIN_VERSION_OUTPUT" =~ $PATTERN ]]; then
            pass "$BIN_NAME --version matches '$PATTERN' (got: $BIN_VERSION_OUTPUT)"
        else
            fail "$BIN_NAME --version did not match '$PATTERN' (got: $BIN_VERSION_OUTPUT)"
        fi

        # Help flag sanity check
        if [[ "${BINARY_HELP_CHECK[$BIN_CRATE]:-0}" == "1" ]]; then
            if "$BIN_NAME" --help >/dev/null 2>&1; then
                pass "$BIN_NAME --help exits cleanly"
            else
                fail "$BIN_NAME --help exited non-zero"
            fi
        fi
    else
        fail "$BIN_NAME not found in PATH after install"
    fi
done

# ---------------------------------------------------------------------------
# Section 2: Library consumption — cargo add + build
# ---------------------------------------------------------------------------

section "Library crate consumption"

# The 10 highest-value crates to verify as downstream dependencies.
# For each: the crate name on crates.io and a minimal Rust snippet that
# exercises the primary public API.  The snippet is compiled in a fresh
# `cargo init --lib` project.
#
# Crates covered:
#   tree-sitter-perl-c  — C-backed tree-sitter grammar facade
#   perl-parser         — Native v3 recursive descent parser
#   perl-lexer          — Context-aware lexer
#   perl-token          — Token type definitions
#   perl-ast            — AST node types
#   perl-error          — Structured error types
#   perl-lsp-protocol   — LSP protocol data structures
#   perl-lsp-transport  — LSP message transport layer
#   perl-parser-core    — Core parsing infrastructure
#   perl-position-tracking — Source position utilities

declare -a LIB_CRATES=(
    "tree-sitter-perl-c"
    "perl-parser"
    "perl-lexer"
    "perl-token"
    "perl-ast"
    "perl-error"
    "perl-lsp-protocol"
    "perl-lsp-transport"
    "perl-parser-core"
    "perl-position-tracking"
)

# Minimal smoke code per crate.  Must compile and, if it contains a #[test],
# must pass `cargo test`.  Keep these tiny — we're checking API surface, not
# correctness.
declare -A LIB_SMOKE_CODE

LIB_SMOKE_CODE["tree-sitter-perl-c"]='
#[test]
fn smoke_tree_sitter_perl_c() {
    let mut parser = tree_sitter_perl_c::try_create_parser()
        .expect("language loading failed");
    let tree = parser.parse("my $x = 42;", None)
        .expect("parse returned None");
    let root = tree.root_node();
    assert!(!root.to_sexp().is_empty(), "sexp must not be empty");
}
'

LIB_SMOKE_CODE["perl-parser"]='
#[test]
fn smoke_perl_parser() {
    let code = r#"my $x = 42;"#;
    let mut parser = perl_parser::Parser::new(code);
    let ast = parser.parse().expect("parse failed");
    assert!(!ast.to_sexp().is_empty(), "AST sexp must not be empty");
}
'

LIB_SMOKE_CODE["perl-lexer"]='
#[test]
fn smoke_perl_lexer() {
    let mut lexer = perl_lexer::PerlLexer::new("my $x = 42;");
    let tokens = lexer.collect_tokens();
    assert!(!tokens.is_empty(), "lexer produced no tokens");
}
'

LIB_SMOKE_CODE["perl-token"]='
#[test]
fn smoke_perl_token() {
    // TokenKind is the central type; ensure it can be constructed and compared.
    // Use Eof — always present and a stable variant in the public API.
    use perl_token::TokenKind;
    let kind = TokenKind::Eof;
    assert_eq!(kind.display_name(), "end of input");
}
'

LIB_SMOKE_CODE["perl-ast"]='
#[test]
fn smoke_perl_ast() {
    use perl_ast::{Node, NodeKind, SourceLocation};
    let loc = SourceLocation { start: 0, end: 2 };
    let node = Node::new(NodeKind::Number { value: "42".to_string() }, loc);
    assert_eq!(node.kind.kind_name(), "Number");
}
'

LIB_SMOKE_CODE["perl-error"]='
#[test]
fn smoke_perl_error() {
    use perl_error::ParseError;
    // ParseError is an enum; construct a variant and confirm Display works
    let e = ParseError::UnexpectedEof;
    assert!(!format!("{}", e).is_empty());
}
'

LIB_SMOKE_CODE["perl-lsp-protocol"]='
#[test]
fn smoke_perl_lsp_protocol() {
    use perl_lsp_protocol::JsonRpcResponse;
    // JsonRpcResponse is the primary serialisable outbound type
    let resp = JsonRpcResponse::success(Some(serde_json::Value::Number(1.into())), serde_json::json!({}));
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains("2.0"), "should contain jsonrpc version");
}
'

LIB_SMOKE_CODE["perl-lsp-transport"]='
#[test]
fn smoke_perl_lsp_transport() {
    // Verify the crate loads: frame() wraps a byte slice in Content-Length framing
    use perl_lsp_transport::frame;
    let payload = b"{}";
    let framed = frame(payload);
    // framed must start with "Content-Length:" header
    assert!(
        framed.starts_with(b"Content-Length:"),
        "frame output must start with Content-Length header"
    );
    assert!(framed.len() > payload.len(), "framed output must be longer than payload");
}
'

LIB_SMOKE_CODE["perl-parser-core"]='
#[test]
fn smoke_perl_parser_core() {
    use perl_parser_core::{Parser, NodeKind};
    let mut parser = Parser::new("my $x = 42;");
    let ast = parser.parse().expect("should parse");
    assert!(matches!(ast.kind, NodeKind::Program { .. }));
}
'

LIB_SMOKE_CODE["perl-position-tracking"]='
#[test]
fn smoke_perl_position_tracking() {
    use perl_position_tracking::{Position, SourceLocation};
    let _loc = SourceLocation { start: 0, end: 5 };
    let pos = Position { byte: 0, line: 0, column: 0 };
    assert_eq!(pos.line, 0);
}
'

# Run each library smoke in an isolated cargo project
for CRATE in "${LIB_CRATES[@]}"; do
    CRATE_PROJ="$WORK_DIR/smoke_${CRATE//-/_}"
    mkdir -p "$CRATE_PROJ"

    # Create a minimal Cargo project
    (
        cd "$CRATE_PROJ" || exit 1
        cargo init --lib --quiet --name "smoke_consumer" 2>/dev/null
    )

    # Add the crate at the target version (plus any required companions)
    if [[ "$SKIP_INSTALL" != "1" ]]; then
        if ! (cd "$CRATE_PROJ" && cargo add "${CRATE}@${VERSION}" --quiet 2>&1); then
            fail "cargo add $CRATE@$VERSION failed"
            continue
        fi
        # perl-lsp-protocol smoke uses serde_json directly for round-trip test
        if [[ "$CRATE" == "perl-lsp-protocol" ]]; then
            (cd "$CRATE_PROJ" && cargo add serde_json --quiet 2>&1) || true
        fi
    fi

    # Write the smoke test to src/lib.rs
    SMOKE_CODE="${LIB_SMOKE_CODE[$CRATE]:-}"
    if [[ -z "$SMOKE_CODE" ]]; then
        # Fallback: just check it compiles with a trivial use statement
        CRATE_IDENT="${CRATE//-/_}"
        SMOKE_CODE="
#[test]
fn smoke_compile() {
    let _ = stringify!(${CRATE_IDENT});
}
"
    fi

    cat > "$CRATE_PROJ/src/lib.rs" <<EOF
// Auto-generated smoke test for $CRATE@$VERSION
$SMOKE_CODE
EOF

    # Compile + run the test
    if (cd "$CRATE_PROJ" && cargo test --quiet 2>&1); then
        pass "library smoke: $CRATE@$VERSION"
    else
        fail "library smoke: $CRATE@$VERSION (cargo test failed — see output above)"
    fi
done

# ---------------------------------------------------------------------------
# Section 3: tree-sitter-perl-c functional integration check
# ---------------------------------------------------------------------------

section "tree-sitter-perl-c functional integration"

TS_PROJ="$WORK_DIR/ts_integration"
mkdir -p "$TS_PROJ"
(
    cd "$TS_PROJ" || exit 1
    cargo init --lib --quiet --name "ts_integration" 2>/dev/null
)

if [[ "$SKIP_INSTALL" != "1" ]]; then
    (cd "$TS_PROJ" && cargo add "tree-sitter-perl-c@${VERSION}" --quiet 2>&1) || \
        fail "cargo add tree-sitter-perl-c@$VERSION for integration test" || true
    # tree-sitter is a direct dependency: the integration test uses tree_sitter::Tree
    # and tree_sitter::Parser types directly.  tree-sitter-perl-c re-exports none of
    # them, so the consumer project must declare it explicitly.
    (cd "$TS_PROJ" && cargo add tree-sitter --quiet 2>&1) || \
        fail "cargo add tree-sitter for ts_integration project" || true
fi

cat > "$TS_PROJ/src/lib.rs" <<'TSEOF'
// Integration smoke for tree-sitter-perl-c: parse several Perl constructs
// and verify the root node kind and that the parse has no errors.

#[cfg(test)]
mod tests {
    fn parse(code: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter_perl_c::try_create_parser()
            .expect("language loading failed");
        parser.parse(code, None).expect("parse returned None")
    }

    fn has_error(tree: &tree_sitter::Tree) -> bool {
        let root = tree.root_node();
        root.has_error()
    }

    #[test]
    fn smoke_scalar_assignment() {
        let tree = parse("my $x = 42;\n");
        let root = tree.root_node();
        assert!(!root.to_sexp().is_empty());
        // Root kind varies by grammar version; just check it's non-empty
        assert!(root.kind().len() > 0, "root kind must be non-empty");
    }

    #[test]
    fn smoke_subroutine() {
        let tree = parse("sub greet { my ($name) = @_; print \"Hello, $name\\n\"; }\n");
        assert!(!has_error(&tree), "subroutine parse should have no errors");
    }

    #[test]
    fn smoke_use_statement() {
        let tree = parse("use strict;\nuse warnings;\n");
        assert!(!has_error(&tree), "use statement parse should have no errors");
    }

    #[test]
    fn smoke_empty_program() {
        let tree = parse("");
        let root = tree.root_node();
        // Empty program must still produce a valid root node
        assert!(!root.to_sexp().is_empty());
    }
}
TSEOF

if (cd "$TS_PROJ" && cargo test --quiet 2>&1); then
    pass "tree-sitter-perl-c integration: all parse checks passed"
else
    fail "tree-sitter-perl-c integration: one or more parse checks failed"
fi

# ---------------------------------------------------------------------------
# Section 4: perl-parser functional integration check
# ---------------------------------------------------------------------------

section "perl-parser functional integration"

PP_PROJ="$WORK_DIR/pp_integration"
mkdir -p "$PP_PROJ"
(
    cd "$PP_PROJ" || exit 1
    cargo init --lib --quiet --name "pp_integration" 2>/dev/null
)

if [[ "$SKIP_INSTALL" != "1" ]]; then
    (cd "$PP_PROJ" && cargo add "perl-parser@${VERSION}" --quiet 2>&1) || \
        fail "cargo add perl-parser@$VERSION for integration test" || true
fi

cat > "$PP_PROJ/src/lib.rs" <<'PPEOF'
#[cfg(test)]
mod tests {
    use perl_parser::Parser;

    fn parse_code(code: &str) -> perl_parser::ast::Node {
        let mut parser = Parser::new(code);
        parser.parse().expect("parse failed")
    }

    #[test]
    fn smoke_scalar_assignment() {
        let ast = parse_code("my $x = 42;");
        assert!(!ast.to_sexp().is_empty(), "AST sexp must not be empty");
    }

    #[test]
    fn smoke_subroutine_definition() {
        let ast = parse_code("sub greet { return \"hello\"; }");
        assert!(!ast.to_sexp().is_empty());
    }

    #[test]
    fn smoke_node_count() {
        let ast = parse_code("my $a = 1; my $b = 2;");
        assert!(ast.count_nodes() > 0, "must have at least one node");
    }
}
PPEOF

if (cd "$PP_PROJ" && cargo test --quiet 2>&1); then
    pass "perl-parser integration: all AST checks passed"
else
    fail "perl-parser integration: one or more AST checks failed"
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

TOTAL=$((PASS_COUNT + FAIL_COUNT))

printf "\n"
printf "================================================================\n"
printf "  Post-publish smoke test — version %s\n" "$VERSION"
printf "  Completed: %s\n" "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
printf "================================================================\n"
printf "  Passed:  %d / %d\n" "$PASS_COUNT" "$TOTAL"
printf "  Failed:  %d / %d\n" "$FAIL_COUNT" "$TOTAL"

if [[ "${GITHUB_STEP_SUMMARY:-}" != "" ]]; then
    {
        echo "## Post-publish smoke test — v${VERSION}"
        echo ""
        echo "| Result | Check |"
        echo "|--------|-------|"
        echo "| Passed | ${PASS_COUNT}/${TOTAL} checks |"
        echo "| Failed | ${FAIL_COUNT}/${TOTAL} checks |"
        echo ""
        if [[ ${#FAILURES[@]} -gt 0 ]]; then
            echo "### Failed checks"
            echo ""
            for F in "${FAILURES[@]}"; do
                echo "- \`${F}\`"
            done
        else
            echo "All checks passed. Crates are installable and functional."
        fi
    } >> "$GITHUB_STEP_SUMMARY"
fi

if [[ $FAIL_COUNT -gt 0 ]]; then
    printf "\n${RED}FAILED checks:${RESET}\n"
    for F in "${FAILURES[@]}"; do
        printf "  - %s\n" "$F"
    done
    printf "\n"
    printf "${RED}Smoke test FAILED.${RESET} %d check(s) did not pass.\n" "$FAIL_COUNT"
    printf "\nInterpretation guide:\n"
    printf "  Binary install failure  -> crate not yet indexed or --locked checksum mismatch\n"
    printf "  Version mismatch        -> binary reports wrong version; possibly stale install\n"
    printf "  cargo add failure       -> crate not yet in sparse index; wait ~30s and retry\n"
    printf "  cargo test failure      -> API changed or public types renamed; check diff\n"
    exit 1
else
    printf "\n${GREEN}All smoke checks passed.${RESET} v%s is installable and functional.\n" "$VERSION"
    exit 0
fi
