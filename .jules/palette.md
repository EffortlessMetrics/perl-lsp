## 2025-02-27 - Startup Notifications vs Logs
**Learning:** Users perceive extensions that notify on every startup as "spammy". Moving successful startup messages to the Output Channel respects user attention and aligns with the "Good UX is invisible" philosophy.
**Action:** Audit `activate()` functions for unnecessary `showInformationMessage` calls and replace them with `outputChannel.appendLine`.
