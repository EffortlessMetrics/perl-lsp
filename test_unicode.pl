#!/usr/bin/env perl
# Test Unicode handling
my $emoji = "✅";
print "Unicode test: $emoji\n";

# More Unicode
my $unicode = "Hello 世界 🌍";
print "Mixed: $unicode\n";

# In comments too
# This has emoji: 🎯

# In strings
my $str = <<'EOF';
Unicode heredoc ✅
With emojis 🎉
EOF

print $str;