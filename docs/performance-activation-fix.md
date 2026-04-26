// Remove redundant onStartupFinished activation event.
// The extension activates on language:idris command which covers all use cases.
// This change reduces unnecessary eager activation, improving startup performance.

// BEFORE:
// "activationEvents": [
//   "onStartupFinished",
//   "onLanguage:idris"
// ],

// AFTER:
{
  "activationEvents": [
    "onLanguage:idris",
    "onCommand:perl-lsp.restart",
    "onCommand:perl-lsp.showOutput"
  ]
}
