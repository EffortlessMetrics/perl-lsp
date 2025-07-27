#!/bin/bash
# Demonstration of the Perl LSP server features

echo "=== Perl Language Server Demo ==="
echo

# Function to send LSP message
send_message() {
    local msg="$1"
    local len=${#msg}
    printf "Content-Length: %d\r\n\r\n%s" "$len" "$msg"
}

# Create a test Perl file
cat > /tmp/lsp_test.pl << 'EOF'
#!/usr/bin/env perl
use strict;
use warnings;

my $name = "World";
print "Hello, $name!\n";

# This will trigger a diagnostic (undefined variable)
$undefined = 42;

sub greet {
    my ($person) = @_;
    print "Hello, $person!\n";
}

greet("Alice");

# Modern Perl features
use feature 'say';
say "Using modern features";

# Complex data structure
my $data = {
    users => [
        { name => "Bob", age => 30 },
        { name => "Carol", age => 25 },
    ]
};

say $data->{users}->[0]->{name};
EOF

echo "Test file created at /tmp/lsp_test.pl"
echo

# Run LSP server and send multiple requests
(
    # Initialize
    send_message '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":"file:///tmp","capabilities":{}}}'
    sleep 0.2
    
    # Initialized notification
    send_message '{"jsonrpc":"2.0","method":"initialized","params":{}}'
    sleep 0.2
    
    # Open document
    DOC_CONTENT=$(cat /tmp/lsp_test.pl | jq -Rs .)
    send_message "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"file:///tmp/lsp_test.pl\",\"languageId\":\"perl\",\"version\":1,\"text\":$DOC_CONTENT}}}"
    sleep 0.2
    
    # Request completion at line 5 (after "my $")
    send_message '{"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///tmp/lsp_test.pl"},"position":{"line":4,"character":5}}}'
    sleep 0.2
    
    # Request hover on "greet" function
    send_message '{"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///tmp/lsp_test.pl"},"position":{"line":10,"character":5}}}'
    sleep 0.2
    
    # Request code actions for undefined variable
    send_message '{"jsonrpc":"2.0","id":4,"method":"textDocument/codeAction","params":{"textDocument":{"uri":"file:///tmp/lsp_test.pl"},"range":{"start":{"line":8,"character":0},"end":{"line":8,"character":15}}}}'
    sleep 0.2
    
    # Shutdown
    send_message '{"jsonrpc":"2.0","id":5,"method":"shutdown"}'
    sleep 0.2
    
) | ./target/release/perl-lsp --stdio 2>&1 | grep -E "(Received|Content-Length|result|capabilities|title|label)" | head -20

echo
echo "=== Demo Complete ==="
echo
echo "The LSP server demonstrated:"
echo "1. Initialization with server capabilities"
echo "2. Document synchronization (didOpen)"
echo "3. Code completion suggestions"
echo "4. Hover information"
echo "5. Code actions for fixes"
echo "6. Clean shutdown"
echo
echo "To use in your editor, configure it to run: perl-lsp --stdio"