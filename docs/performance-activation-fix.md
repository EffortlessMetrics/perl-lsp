// Remove redundant onStartupFinished activation event.
// The extension activates on language:perl command which covers all use cases.
// This change reduces unnecessary eager activation, improving startup performance.

// BEFORE:
// "activationEvents": [
//   "onStartupFinished",
//   "onLanguage:perl"
// ],

// AFTER:
{
  "activationEvents": [
    "onLanguage:perl",
    "onCommand:perl-lsp.restart",
    "onCommand:perl-lsp.showOutput"
  ]
}
