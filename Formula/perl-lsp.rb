class PerlLsp < Formula
  desc "High-performance Perl Language Server with 100% syntax coverage"
  homepage "https://github.com/EffortlessMetrics/perl-lsp"
  # PLACEHOLDER-GUARD: __RELEASE_VERSION__ must be replaced in CI before merge.
  version "__RELEASE_VERSION__"
  # PLACEHOLDER-GUARD: all sha256 values must be replaced in CI before merge.
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v#{version}/perllsp-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "__SHA256_MACOS_AARCH64__"
    else
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v#{version}/perllsp-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "__SHA256_MACOS_X86_64__"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v#{version}/perllsp-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_LINUX_AARCH64__"
    else
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v#{version}/perllsp-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_LINUX_X86_64__"
    end
  end

  def install
    # Expected extracted layout: perllsp-<version>-<target>/{perllsp,perl-dap}
    # If the release packaging layout changes, update this extraction logic with a follow-up.
    extracted_dir = Dir.glob("perllsp-*").find { |dir| Dir.exist?(dir) }
    if extracted_dir
      bin.install "#{extracted_dir}/perllsp"
      bin.install "#{extracted_dir}/perl-dap" if File.exist?("#{extracted_dir}/perl-dap")
    else
      bin.install "perllsp"
      bin.install "perl-dap" if File.exist?("perl-dap")
    end
  end

  test do
    assert_match(/perllsp|Perl LSP/, shell_output("#{bin}/perllsp --version"))
  end
end
