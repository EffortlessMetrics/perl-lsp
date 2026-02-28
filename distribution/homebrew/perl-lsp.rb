class PerlLsp < Formula
  desc "High-performance Perl Language Server with 100% syntax coverage"
  homepage "https://github.com/EffortlessMetrics/perl-lsp"
  # PLACEHOLDER-GUARD: __RELEASE_VERSION__ and all sha placeholders must be replaced in CI.
  version "__RELEASE_VERSION__"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v#{version}/perl-lsp-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "__SHA256_MACOS_AARCH64__"
    else
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v#{version}/perl-lsp-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "__SHA256_MACOS_X86_64__"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v#{version}/perl-lsp-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_LINUX_AARCH64__"
    else
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v#{version}/perl-lsp-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_LINUX_X86_64__"
    end
  end

  def install
    # Expected extracted layout: perl-lsp-v<version>-<target>/perl-lsp
    # If the release packaging layout changes, update this extraction logic with a follow-up.
    extracted_dir = Dir.glob("perl-lsp-v*").find { |dir| Dir.exist?(dir) }
    if extracted_dir
      bin.install "#{extracted_dir}/perl-lsp"
    else
      bin.install "perl-lsp"
    end
  end

  test do
    assert_match(/perl-lsp|Perl LSP/, shell_output("#{bin}/perl-lsp --version"))
  end
end
