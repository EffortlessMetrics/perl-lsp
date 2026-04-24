class PerlLsp < Formula
  desc "Fast, reliable Perl language server with 100% syntax coverage"
  homepage "https://github.com/EffortlessMetrics/perl-lsp"
  version "0.11.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v0.11.0/perllsp-0.11.0-aarch64-apple-darwin.tar.gz"
      sha256 "90ec6dfcb71882ff6e33ff3b4fcf410d4e92d7eb31cd7b37872148e854ec3b2d"
    else
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v0.11.0/perllsp-0.11.0-x86_64-apple-darwin.tar.gz"
      sha256 "50edd25dc077cecf2227103b15598d2fd48aa7c94a33b591b37eff96e0342407"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v0.11.0/perllsp-0.11.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "a4d72384c0bb3b7b3fbec2c7b967579e49145844f8835230757d532281cb21c0"
    else
      url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v0.11.0/perllsp-0.11.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "a9124a347bec0de26e8b01a1b9d5630c856611a01a454e68b8f586f258485fba"
    end
  end

  def install
    # Find the extracted directory (should be perllsp-v0.11.0-{target})
    extracted_dir = Dir.glob("perllsp-*").first
    if extracted_dir && File.directory?(extracted_dir)
      bin.install "#{extracted_dir}/perllsp"
    else
      # Fallback: binary might be in the root
      bin.install "perllsp"
    end
  end

  def caveats
    <<~EOS
      To use perl-lsp with your editor:

      VS Code:
        Install the "Perl Language Server" extension from the marketplace

      Neovim (with lspconfig):
        require('lspconfig').perl_lsp.setup{
          cmd = {'#{opt_bin}/perllsp', '--stdio'}
        }

      Emacs (with lsp-mode):
        (lsp-register-client
         (make-lsp-client :new-connection (lsp-stdio-connection '("#{opt_bin}/perllsp" "--stdio"))
                          :activation-fn (lsp-activate-on "perl")
                          :server-id 'perl-lsp))
    EOS
  end

  test do
    # Test that the binary runs and responds to version request
    assert_match(/perllsp|Perl LSP/, shell_output("#{bin}/perllsp --version"))
    
    # Test LSP initialization
    input = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
    output = pipe_output("#{bin}/perllsp --stdio", input, 0)
    assert_match(/Content-Length/, output)
  end
end
