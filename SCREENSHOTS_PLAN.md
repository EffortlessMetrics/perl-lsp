# Screenshots Plan for VSCode Marketplace

## Setup
- Resolution: 1280x800 (16:10 ratio)
- Theme: Dark+ (consistent across all shots)
- Font size: 14px
- Show activity bar and status bar
- File size limit: Keep each < 1MB

## Screenshot 1: Go to Definition & References
**File: demo1.pl**
```perl
package Calculator;

sub add {
    my ($x, $y) = @_;
    return $x + $y;
}

sub multiply {
    my ($x, $y) = @_;
    return $x * $y;
}

# Usage
my $result = Calculator::add(5, 3);
print "Sum: $result\n";

$result = Calculator::multiply(4, 7);
print "Product: $result\n";
```
- Ctrl+click on `Calculator::add` to show go-to-definition
- Right-click → Find All References on `add` function
- Show references panel at bottom

## Screenshot 2: Rich Hover Documentation
**File: demo2.pl**
```perl
use strict;
use warnings;

# Calculate factorial recursively
sub factorial {
    my $n = shift;
    return 1 if $n <= 1;
    return $n * factorial($n - 1);
}

# Hover over 'shift' to see built-in docs
my @numbers = (1..5);
my @results = map { factorial($_) } @numbers;

# Hover over 'map' for signature help
print "Factorials: @results\n";
```
- Hover over `shift` showing parameter documentation
- Show tooltip with rich markdown formatting

## Screenshot 3: Multi-file Rename
**File: demo3a.pl**
```perl
package UserManager;

sub create_user {
    my ($username, $email) = @_;
    # Implementation
}

sub delete_user {
    my $username = shift;
    # Implementation
}

1;
```
**File: demo3b.pl**
```perl
use UserManager;

UserManager::create_user("alice", "alice@example.com");
UserManager::delete_user("bob");
```
- F2 on `create_user` showing rename preview
- Show both files with highlighted changes

## Screenshot 4: Diagnostics & Problems Panel
**File: demo4.pl**
```perl
use strict;
use warnings;

my $name = "Alice";
my $naem = "Bob";  # Typo - unused variable

sub greet {
    my $person = shift;
    print "Hello, $peson\n";  # Undefined variable
}

greet($name);
print "Count: $count\n";  # Undefined under strict

# Missing semicolon
my $x = 42
my $y = 7;
```
- Show red squiggles under errors
- Problems panel showing all issues
- Quick fix suggestions on hover

## Screenshot 5: Semantic Highlighting
**File: demo5.pl**
```perl
package ColorDemo;
use constant PI => 3.14159;

my $global = "global variable";

sub process_data {
    my ($self, $data) = @_;
    
    # Different colors for different token types
    my $local = "local variable";
    our $package_var = "package variable";
    
    foreach my $item (@$data) {
        print "Processing: $item\n" if $item;
    }
    
    return PI * length($local);
}

# Method call vs function call
my $obj = ColorDemo->new();
$obj->process_data([1, 2, 3]);
process_data($obj, [4, 5, 6]);
```
- Show distinct colors for:
  - Keywords (blue)
  - Variables (light blue)  
  - Functions (yellow)
  - Constants (green)
  - Strings (orange)
  - Comments (green italic)

## Screenshot 6: Settings & Auto-download
**Settings UI showing:**
- perl-lsp.autoDownload: ✓ (checked)
- perl-lsp.serverPath: "perl-lsp"
- perl-lsp.enableDiagnostics: ✓
- perl-lsp.enableSemanticTokens: ✓
- perl-lsp.trace.server: "off"

**Notification showing:**
"Downloading Perl LSP server v0.8.1 for linux-x64..."
Progress bar at 75%

## Capture Tips
1. Use VSCode's built-in screenshot tool: Cmd+Shift+P → "Developer: Toggle Screencast Mode"
2. Clean workspace - hide unnecessary files
3. Consistent cursor position
4. No personal information visible
5. Save as PNG with compression
6. Name files: screenshot-1-definition.png, etc.