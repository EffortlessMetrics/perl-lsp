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

        # Use Rust 1.89 (MSRV) from rust-overlay
        rustToolchain = pkgs.rust-bin.stable."1.89.0".default.override {
          extensions = [ "rust-src" "clippy" "rustfmt" ];
        };

        # Common build inputs
        buildInputs = with pkgs; [
          rustToolchain
          pkg-config
          openssl
        ] ++ lib.optionals stdenv.isDarwin [
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.SystemConfiguration
        ];

        nativeBuildInputs = with pkgs; [
          just
          cargo-nextest
        ];

      in {
        # Development shell
        devShells.default = pkgs.mkShell {
          inherit buildInputs;
          nativeBuildInputs = nativeBuildInputs ++ [ rustToolchain ];

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

          shellHook = ''
            echo "🦀 Perl LSP development environment"
            echo "   Rust: $(rustc --version)"
            echo ""
            echo "Commands:"
            echo "  just ci-gate      # Fast merge gate (~2-5 min)"
            echo "  just ci-full      # Full CI pipeline (~10-20 min)"
            echo "  nix flake check   # Run all checks"
          '';
        };

        # Checks - the single source of truth before pushing
        checks = {
          # Format check (fast fail)
          format = pkgs.runCommand "check-format" {
            buildInputs = [ rustToolchain ];
            src = self;
          } ''
            cd $src
            cargo fmt --check --all
            touch $out
          '';

          # Clippy lint (catches common issues)
          clippy = pkgs.runCommand "check-clippy" {
            buildInputs = buildInputs ++ [ rustToolchain ];
            src = self;
          } ''
            cd $src
            cargo clippy --workspace --lib --locked -- -D warnings -A missing_docs
            touch $out
          '';

          # Library tests (fast, essential) - uses nextest for consistency with CI
          test-lib = pkgs.runCommand "check-test-lib" {
            buildInputs = buildInputs ++ [ rustToolchain pkgs.cargo-nextest ];
            src = self;
          } ''
            cd $src
            cargo nextest run --workspace --lib --locked --hide-progress-bar
            touch $out
          '';

          # WASM32 determinism check
          wasm-check = pkgs.runCommand "check-wasm" {
            buildInputs = buildInputs ++ [
              (rustToolchain.override { targets = [ "wasm32-unknown-unknown" ]; })
            ];
            src = self;
          } ''
            cd $src
            RUSTFLAGS="-D warnings" cargo check --locked -p perl-parser --target wasm32-unknown-unknown
            touch $out
          '';

          # LSP integration tests (thread-constrained, matches CI)
          test-lsp = pkgs.runCommand "check-test-lsp" {
            buildInputs = buildInputs ++ [ rustToolchain pkgs.cargo-nextest ];
            src = self;
            RUST_TEST_THREADS = "2";
          } ''
            cd $src
            cargo nextest run -p perl-lsp --locked --hide-progress-bar --test-threads=2
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
            echo "✅ Policy check passed"
            touch $out
          '';
        };

        # Packages
        packages = {
          default = self.packages.${system}.perl-lsp;

          perl-lsp = pkgs.rustPlatform.buildRustPackage {
            pname = "perl-lsp";
            version = "0.8.8";
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
      }
    );
}
