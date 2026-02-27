class PerlLsp < Formula
  desc "Fast, reliable Perl language server with 100% syntax coverage"
  homepage "https://github.com/EffortlessMetrics/perl-lsp"
  version "0.8.3"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v0.8.3/perl-lsp-v0.8.3-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_AARCH64_DARWIN"
    else
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v0.8.3/perl-lsp-v0.8.3-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_X86_64_DARWIN"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v0.8.3/perl-lsp-v0.8.3-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_AARCH64_LINUX"
    else
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v0.8.3/perl-lsp-v0.8.3-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "8af781a0e0aed47f22517ab15cce80dbf78e7bcafb62e1eed5ab236b481b920d"
    end
  end

  def install
    # Find the extracted directory (should be perl-lsp-v0.8.3-{target})
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