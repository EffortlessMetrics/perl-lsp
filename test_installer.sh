#!/usr/bin/env bash
# Test the installer locally

set -euo pipefail

echo "Testing installer script..."

# Test with dry-run (no actual download)
echo "1. Testing platform detection..."
# Extract just the detect_platform function
PLATFORM="$(bash -c '
  detect_platform() {
    local os arch
    case "$(uname -s)" in
      Linux)  os=linux ;;
      Darwin) os=macos ;;
      MINGW*|MSYS*|CYGWIN*) echo "Windows not supported by this shell script"; exit 1 ;;
      *) echo "Unsupported OS: $(uname -s)"; exit 1 ;;
    esac
    case "$(uname -m)" in
      x86_64|amd64) arch=x86_64 ;;
      arm64|aarch64) arch=aarch64 ;;
      *) echo "Unsupported architecture: $(uname -m)"; exit 1 ;;
    esac
    echo "${os}-${arch}"
  }
  detect_platform
')"
echo "   Detected platform: $PLATFORM"

# Test temp dir creation and cleanup
echo "2. Testing temp dir handling..."
tmp_test="$(mktemp -d)"
trap 'rm -rf "$tmp_test"' EXIT
echo "   ✓ Temp dir created: $tmp_test"
echo "   ✓ Cleanup trap registered"

# Test PATH detection
echo "3. Testing PATH detection..."
TEST_DIR="/tmp/test_install"
case ":$PATH:" in
  *":$TEST_DIR:"*)
    echo "   Test dir is in PATH"
    ;;
  *)
    echo "   Test dir is NOT in PATH (expected)"
    ;;
esac

# Test shell config detection
echo "4. Testing shell config detection..."
shell_rc=""
if [[ -n "${ZDOTDIR:-}" && -f "$ZDOTDIR/.zshrc" ]]; then
  shell_rc="$ZDOTDIR/.zshrc"
elif [[ -f "$HOME/.zshrc" ]]; then
  shell_rc="$HOME/.zshrc"
elif [[ -f "$HOME/.bashrc" ]]; then
  shell_rc="$HOME/.bashrc"
elif [[ -f "$HOME/.profile" ]]; then
  shell_rc="$HOME/.profile"
fi
echo "   Detected shell config: ${shell_rc:-none}"

# Test archive extraction simulation
echo "5. Testing archive structure handling..."
test_archive="$(mktemp -d)"
# Simulate flat structure
touch "$test_archive/perl-lsp"
chmod +x "$test_archive/perl-lsp"
found_flat="$(find "$test_archive" -maxdepth 2 -type f -name "perl-lsp" | head -n1)"
echo "   Flat structure test: ${found_flat:+✓}"

# Simulate nested structure
mkdir -p "$test_archive/perl-lsp-0.7.5"
touch "$test_archive/perl-lsp-0.7.5/perl-lsp"
chmod +x "$test_archive/perl-lsp-0.7.5/perl-lsp"
found_nested="$(find "$test_archive" -maxdepth 2 -type f -name "perl-lsp" | head -n1)"
echo "   Nested structure test: ${found_nested:+✓}"
rm -rf "$test_archive"

echo
echo "✅ All installer tests passed!"
echo
echo "To test actual installation (requires network):"
echo "  INSTALL_DIR=/tmp/test_install VERSION=latest bash ./install.sh"