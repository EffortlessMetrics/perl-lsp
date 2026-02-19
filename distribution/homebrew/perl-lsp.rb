class PerlLsp < Formula
  desc "High-performance Perl Language Server with 100% syntax coverage"
  homepage "https://github.com/tree-sitter/tree-sitter-perl"
  url "https://github.com/EffortlessMetrics/perl-lsp/archive/v1.0.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256"  # Update with actual SHA256 after release
  license "MIT"
  head "https://github.com/EffortlessMetrics/perl-lsp.git", branch: "main"

  depends_on "rust" => :build

  def install
    # Build the LSP server
    system "cargo", "build", "--release", "--bin", "perl-lsp", "-p", "perl-parser"
    
    # Install the binary
    bin.install "target/release/perl-lsp"
    
    # Install completion scripts if available
    bash_completion.install "completions/perl-lsp.bash" if File.exist?("completions/perl-lsp.bash")
    zsh_completion.install "completions/_perl-lsp" if File.exist?("completions/_perl-lsp")
    fish_completion.install "completions/perl-lsp.fish" if File.exist?("completions/perl-lsp.fish")
  end

  test do
    # Test that the server starts and responds to initialization
    require "open3"
    require "json"

    json_request = {
      jsonrpc: "2.0",
      id: 1,
      method: "initialize",
      params: {
        processId: Process.pid,
        rootUri: nil,
        capabilities: {}
      }
    }.to_json

    content_length = json_request.bytesize
    input = "Content-Length: #{content_length}\r\n\r\n#{json_request}"

    output = ""
    Open3.popen3("#{bin}/perl-lsp", "--stdio") do |stdin, stdout, stderr, wait_thr|
      stdin.write(input)
      stdin.close
      
      # Read response headers
      while line = stdout.gets
        break if line == "\r\n"
      end
      
      # Read response body
      if stdout.gets =~ /Content-Length: (\d+)/
        length = $1.to_i
        stdout.read(2)  # Skip \r\n
        output = stdout.read(length)
      end
    end

    response = JSON.parse(output)
    assert_equal "2.0", response["jsonrpc"]
    assert response["result"]["capabilities"]
    assert response["result"]["serverInfo"]["name"]
  end
end