{
  description = "Perl LSP - Lightning-fast Language Server Protocol implementation for Perl";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # MSRV: Rust 1.89.0 - pinned for OpenAI Codex compatibility
        # This matches rust-toolchain.toml and CI workflows
        rustVersion = "1.89.0";
        rustToolchain = pkgs.rust-bin.stable.${rustVersion}.default.override {
          extensions = [ "rust-src" "clippy" "rustfmt" ];
          targets = [ "wasm32-unknown-unknown" ];  # For WASM determinism checks
        };

        # Common build inputs (libraries needed for compilation)
        buildInputs = with pkgs; [
          rustToolchain
          pkg-config
          openssl
        ] ++ lib.optionals stdenv.isDarwin [
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.SystemConfiguration
        ];

        # CI tools available in dev shell
        ciTools = with pkgs; [
          just              # Command runner (justfile)
          cargo-nextest     # Fast test runner (used in CI)
          cargo-audit       # Security vulnerability scanner
          gh                # GitHub CLI for PR operations
          jq                # JSON processing for scripts
          (python3.withPackages (ps: [ ps.pyyaml ]))  # Used by CI scripts
        ];

        # Optional expensive CI tools (available but not required)
        optionalCiTools = with pkgs; [
          cargo-mutants     # Mutation testing (ci:mutation label)
        ];

      in {
        # Development shell - provides all tools needed for local CI
        devShells.default = pkgs.mkShell {
          inherit buildInputs;
          nativeBuildInputs = ciTools ++ optionalCiTools ++ [ rustToolchain ];

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

          # Ensure consistent behavior with CI
          CARGO_TERM_COLOR = "always";

          shellHook = ''
            echo ""
            echo "Perl LSP development environment (Rust ${rustVersion})"
            echo "============================================"
            echo ""
            echo "Toolchain: $(rustc --version)"
            echo "Targets:   native, wasm32-unknown-unknown"
            echo ""
            echo "CI Commands (local-first development):"
            echo "  just ci-gate      # Fast merge gate (~2-5 min) - REQUIRED before push"
            echo "  just ci-full      # Full CI pipeline (~10-20 min)"
            echo "  just ci-gate-msrv # Explicit MSRV validation"
            echo ""
            echo "Individual Gates:"
            echo "  just ci-format    # Check formatting"
            echo "  just ci-clippy-lib # Clippy (libraries only)"
            echo "  just ci-test-lib  # Library tests"
            echo "  just ci-lsp-def   # LSP semantic tests"
            echo ""
            echo "Quality Tools:"
            echo "  cargo audit       # Security vulnerability scan"
            echo "  cargo nextest run # Fast parallel tests"
            echo ""
            echo "Documentation: docs/CI_LOCAL_VALIDATION.md"
            echo ""
          '';
        };

        # Minimal shell for CI runners (no optional tools)
        devShells.ci = pkgs.mkShell {
          inherit buildInputs;
          nativeBuildInputs = ciTools ++ [ rustToolchain ];
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          CARGO_TERM_COLOR = "always";
        };

        # Checks - mirror CI gates for `nix flake check`
        # Note: These run in the Nix sandbox without network access.
        # For full CI simulation with network, use: nix develop -c just ci-gate
        checks = {
          # Format check (fast fail, ~5 seconds)
          format = pkgs.runCommand "check-format" {
            buildInputs = [ rustToolchain ];
            src = self;
          } ''
            cd $src
            cargo fmt --check --all
            touch $out
          '';

          # Clippy lint - libraries only (matches ci-clippy-lib, ~30-60 seconds)
          clippy-lib = pkgs.runCommand "check-clippy-lib" {
            buildInputs = buildInputs ++ [ rustToolchain ];
            src = self;
          } ''
            cd $src
            cargo clippy --workspace --lib --locked -- -D warnings -A missing_docs
            touch $out
          '';

          # Production panic safety - no unwrap/expect in shipped code (Issue #143)
          clippy-prod-no-unwrap = pkgs.runCommand "check-clippy-prod-no-unwrap" {
            buildInputs = buildInputs ++ [ rustToolchain ];
            src = self;
          } ''
            cd $src
            cargo clippy --workspace --lib --bins --no-deps --locked -- -D clippy::unwrap_used -D clippy::expect_used
            touch $out
          '';

          # Library tests (fast, essential, ~1-2 minutes)
          # Uses cargo test directly since nextest requires network for download
          test-lib = pkgs.runCommand "check-test-lib" {
            buildInputs = buildInputs ++ [ rustToolchain ];
            src = self;
          } ''
            cd $src
            cargo test --workspace --lib --locked
            touch $out
          '';

          # WASM32 determinism check - ensures parser works in browser contexts
          wasm-check = pkgs.runCommand "check-wasm" {
            buildInputs = buildInputs ++ [ rustToolchain ];
            src = self;
          } ''
            cd $src
            RUSTFLAGS="-D warnings" cargo check --locked -p perl-parser --target wasm32-unknown-unknown
            touch $out
          '';

          # Policy checks (simplified for nix sandbox - no git available)
          policy = pkgs.runCommand "check-policy" {
            buildInputs = [ pkgs.bash pkgs.gnugrep pkgs.findutils ];
            src = self;
          } ''
            cd $src
            # Check for direct ExitStatus::from_raw() usage (without helper)
            # This is the same policy as .ci/scripts/check-from-raw.sh but without git
            viol=$(find crates xtask -name '*.rs' -exec grep -l 'ExitStatus::from_raw(' {} \; 2>/dev/null \
              | xargs -r grep -nE 'ExitStatus::from_raw\(' 2>/dev/null \
              | grep -Ev '::from_raw\([[:space:]]*raw[_ ]?exit[[:space:]]*\(' || true)
            if [ -n "$viol" ]; then
              echo "Policy violation: direct ExitStatus::from_raw() usage found:"
              echo "$viol"
              exit 1
            fi
            echo "Policy check passed"
            touch $out
          '';

          # Check for nested Cargo.lock files (footgun prevention)
          no-nested-lock = pkgs.runCommand "check-no-nested-lock" {
            buildInputs = [ pkgs.bash pkgs.findutils ];
            src = self;
          } ''
            cd $src
            if find . -name 'Cargo.lock' -type f 2>/dev/null | grep -v '^\./Cargo\.lock$' | grep -q .; then
              echo "ERROR: Nested Cargo.lock detected!"
              find . -name 'Cargo.lock' -type f 2>/dev/null | grep -v '^\./Cargo\.lock$'
              exit 1
            fi
            echo "No nested lockfiles"
            touch $out
          '';
        };

        # Expose check as a derivation for CI scripts
        # Usage: nix build .#checks.x86_64-linux.all
        # Note: This aggregates all checks into one target

        # Packages
        packages = {
          default = self.packages.${system}.perl-lsp;

          perl-lsp = pkgs.rustPlatform.buildRustPackage {
            pname = "perl-lsp";
            version = "0.9.0";  # Keep in sync with CLAUDE.md
            src = self;
            cargoLock.lockFile = ./Cargo.lock;

            inherit buildInputs;
            nativeBuildInputs = with pkgs; [ pkg-config ];

            buildAndTestSubdir = "crates/perl-lsp";

            # Skip tests during package build (run via checks)
            doCheck = false;

            meta = with pkgs.lib; {
              description = "Lightning-fast Perl LSP server";
              homepage = "https://github.com/EffortlessMetrics/tree-sitter-perl-rs";
              license = licenses.mit;
              mainProgram = "perl-lsp";
            };
          };
        };

        # Apps
        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.${system}.perl-lsp;
        };

        # Convenience script for running all checks
        # Usage: nix run .#ci-simulate
        apps.ci-simulate = {
          type = "app";
          program = toString (pkgs.writeShellScript "ci-simulate" ''
            set -euo pipefail
            echo "Running CI simulation via Nix..."
            echo ""
            echo "This is equivalent to: nix develop -c just ci-gate"
            echo "For full CI with network access, use that command instead."
            echo ""
            exec ${pkgs.lib.getExe pkgs.just} ci-gate
          '');
        };
      }
    );
}
