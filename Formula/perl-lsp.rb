class PerlLsp < Formula
  desc "Lightning-fast Perl LSP server with 26+ IDE features"
  homepage "https://github.com/EffortlessMetrics/perl-lsp"
  version "0.8.0"
  
  if OS.mac? && Hardware::CPU.arm?
    url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v#{version}/perl-lsp-v#{version}-aarch64-apple-darwin.tar.gz"
    sha256 "PLACEHOLDER_SHA256_AARCH64_DARWIN"
  elsif OS.mac?
    url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v#{version}/perl-lsp-v#{version}-x86_64-apple-darwin.tar.gz"
    sha256 "PLACEHOLDER_SHA256_X86_64_DARWIN"
  elsif OS.linux?
    url "https://github.com/EffortlessMetrics/perl-lsp/releases/download/v#{version}/perl-lsp-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "PLACEHOLDER_SHA256_X86_64_LINUX"
  end

  def install
    bin.install "perl-lsp"
    bin.install "perl-parse" if File.exist?("perl-parse")
  end

  test do
    require "open3"
    json = <<~JSON
      {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
          "capabilities": {}
        }
      }
    JSON
    
    Open3.popen3("#{bin}/perl-lsp", "--stdio") do |stdin, stdout, _, _|
      stdin.write "Content-Length: #{json.bytesize}\r\n\r\n#{json}"
      stdin.close
      assert_match(/\"capabilities\"/, stdout.read)
    end
  end
end