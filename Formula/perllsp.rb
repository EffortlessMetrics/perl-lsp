class Perllsp < Formula
  desc "Native Rust language server and debug adapter for Perl"
  homepage "https://github.com/EffortlessMetrics/perl-lsp"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v__RELEASE_VERSION__/perllsp-__RELEASE_VERSION__-aarch64-apple-darwin.tar.gz"
      sha256 "__SHA256_MACOS_AARCH64__"
    else
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v__RELEASE_VERSION__/perllsp-__RELEASE_VERSION__-x86_64-apple-darwin.tar.gz"
      sha256 "__SHA256_MACOS_X86_64__"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v__RELEASE_VERSION__/perllsp-__RELEASE_VERSION__-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_LINUX_AARCH64__"
    else
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v__RELEASE_VERSION__/perllsp-__RELEASE_VERSION__-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_LINUX_X86_64__"
    end
  end

  def install
    extracted_dir = Dir.glob("perllsp-#{version}-*").find { |path| File.directory?(path) }
    package_dir = extracted_dir || "."

    bin.install "#{package_dir}/perllsp"
    bin.install "#{package_dir}/perl-dap"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/perllsp --version")
    assert_match version.to_s, shell_output("#{bin}/perl-dap --version")
  end
end
