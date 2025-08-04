# 🚀 Announcing tree-sitter-perl v0.5.0: Modern IDE Support for Perl

**TL;DR**: Perl now has a world-class Language Server with VSCode extension. Install with one click and get features like workspace-wide symbol search, inline test running, and more.

---

Dear Perl Community,

We're thrilled to announce the release of **tree-sitter-perl v0.5.0**, featuring a complete Language Server Protocol (LSP) implementation that brings modern IDE capabilities to Perl development.

## What's New?

### 🎯 Workspace Symbols (`Ctrl+T`)
Jump to any symbol across your entire project instantly. No more grep!

### 🏃 Code Lens 
See "▶ Run Test" buttons above test functions. Click to run. See reference counts inline.

### 📦 One-Click Install
```bash
code --install-extension perl-language-server-0.5.0.vsix
```

### ⚡ Blazing Fast
- Parser: 4-19x faster than before
- Symbol search: <1ms 
- 100% Perl 5 syntax coverage

## Why This Matters

For too long, Perl developers have lacked the modern tooling available to other languages. This release changes that. You now get the same IDE experience as Rust, Go, or TypeScript developers—but for Perl.

## Quick Demo

```perl
package MyApp;  # ← "3 references" appears above

sub process_data {  # ← "▶ Run Test | 5 references"
    my ($self, $data) = @_;
    # Ctrl+T → "process_data" → jump here instantly
}
```

## Get Started

1. **Download**: [perl-language-server-0.5.0.vsix](https://github.com/tree-sitter/tree-sitter-perl/releases)
2. **Install**: `code --install-extension perl-language-server-0.5.0.vsix`
3. **Enjoy**: Open any Perl project and experience the difference

## What's Next?

- v0.6.0: Semantic tokens, call hierarchy, inlay hints
- Test runner integration
- Debugging support
- Refactoring tools

## Feedback

Try it out and let us know what you think! Report issues at:
https://github.com/tree-sitter/tree-sitter-perl/issues

## Spread the Word

If you find this useful, please share with your team and the Perl community. Together, we can modernize Perl development.

---

**Happy coding!** 🐪

The tree-sitter-perl team