class Perllsp < Formula
  desc "Native Rust language server and debug adapter for Perl"
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
    extracted_dir = Dir.glob("perllsp-#{version}-*").find { |path| File.directory?(path) }
    raise "expected release archive directory perllsp-#{version}-<target>" unless extracted_dir

    bin.install "#{extracted_dir}/perllsp"
    bin.install "#{extracted_dir}/perl-dap"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/perllsp --version")
    assert_match version.to_s, shell_output("#{bin}/perl-dap --version")
  end
end
