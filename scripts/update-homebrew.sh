#!/usr/bin/env bash
set -euo pipefail

# Update Homebrew formula with actual SHA256 checksums after release
# Usage: ./scripts/update-homebrew.sh v0.8.3

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 v0.8.3"
    exit 1
fi

# Remove 'v' prefix if present
VERSION_NUM="${VERSION#v}"

echo "Updating Homebrew formula for version $VERSION_NUM..."

# Download SHA256SUMS file
SUMS_URL="https://github.com/EffortlessMetrics/perl-lsp/releases/download/${VERSION}/SHA256SUMS"
echo "Downloading checksums from $SUMS_URL"

if ! curl -sSfL "$SUMS_URL" -o /tmp/SHA256SUMS; then
    echo "Error: Could not download SHA256SUMS file"
    echo "Make sure the release $VERSION exists"
    exit 1
fi

# Extract checksums for each platform
get_sha256() {
    local pattern="$1"
    grep "$pattern" /tmp/SHA256SUMS | awk '{print $1}'
}

SHA_AARCH64_DARWIN=$(get_sha256 "perl-lsp-v${VERSION_NUM}-aarch64-apple-darwin.tar.gz")
SHA_X86_64_DARWIN=$(get_sha256 "perl-lsp-v${VERSION_NUM}-x86_64-apple-darwin.tar.gz")
SHA_AARCH64_LINUX=$(get_sha256 "perl-lsp-v${VERSION_NUM}-aarch64-unknown-linux-gnu.tar.gz")
SHA_X86_64_LINUX=$(get_sha256 "perl-lsp-v${VERSION_NUM}-x86_64-unknown-linux-gnu.tar.gz")

if [ -z "$SHA_AARCH64_DARWIN" ] || [ -z "$SHA_X86_64_DARWIN" ] || [ -z "$SHA_AARCH64_LINUX" ] || [ -z "$SHA_X86_64_LINUX" ]; then
    echo "Error: Could not extract all required checksums"
    echo "Available checksums:"
    cat /tmp/SHA256SUMS
    exit 1
fi

# Create updated formula
cat > homebrew/perl-lsp.rb << EOF
class PerlLsp < Formula
  desc "Fast, reliable Perl language server with 100% syntax coverage"
  homepage "https://github.com/EffortlessMetrics/perl-lsp"
  version "${VERSION_NUM}"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/${VERSION}/perl-lsp-v${VERSION_NUM}-aarch64-apple-darwin.tar.gz"
      sha256 "${SHA_AARCH64_DARWIN}"
    else
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/${VERSION}/perl-lsp-v${VERSION_NUM}-x86_64-apple-darwin.tar.gz"
      sha256 "${SHA_X86_64_DARWIN}"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/${VERSION}/perl-lsp-v${VERSION_NUM}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "${SHA_AARCH64_LINUX}"
    else
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/${VERSION}/perl-lsp-v${VERSION_NUM}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "${SHA_X86_64_LINUX}"
    end
  end

  def install
    # Find the extracted directory (should be perl-lsp-v${VERSION_NUM}-{target})
    extracted_dir = Dir.glob("perl-lsp-v*").first
    if extracted_dir && File.directory?(extracted_dir)
      bin.install "#{extracted_dir}/perl-lsp"
    else
      # Fallback: binary might be in the root
      bin.install "perl-lsp"
    end
  end

  def caveats
    <<~EOS
      To use perl-lsp with your editor:

      VS Code:
        Install the "Perl Language Server" extension from the marketplace

      Neovim (with lspconfig):
        require('lspconfig').perl_lsp.setup{
          cmd = {'#{opt_bin}/perl-lsp', '--stdio'}
        }

      Emacs (with lsp-mode):
        (lsp-register-client
         (make-lsp-client :new-connection (lsp-stdio-connection '("#{opt_bin}/perl-lsp" "--stdio"))
                          :activation-fn (lsp-activate-on "perl")
                          :server-id 'perl-lsp))
    EOS
  end

  test do
    # Test that the binary runs and responds to version request
    assert_match(/perl-lsp|Perl LSP/, shell_output("#{bin}/perl-lsp --version"))
    
    # Test LSP initialization
    input = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
    output = pipe_output("#{bin}/perl-lsp --stdio", input, 0)
    assert_match(/Content-Length/, output)
  end
end
EOF

echo "✅ Homebrew formula updated for version $VERSION_NUM"
echo ""
echo "Checksums:"
echo "  macOS ARM64:  $SHA_AARCH64_DARWIN"
echo "  macOS x86_64: $SHA_X86_64_DARWIN"
echo "  Linux ARM64:  $SHA_AARCH64_LINUX"
echo "  Linux x86_64: $SHA_X86_64_LINUX"
echo ""
echo "Next steps:"
echo "1. Review the formula: cat homebrew/perl-lsp.rb"
echo "2. Copy to your homebrew-tap repository"
echo "3. Commit and push the updated formula"
echo ""
echo "Users can then install with:"
echo "  brew tap effortlesssteven/tap"
echo "  brew install perl-lsp"