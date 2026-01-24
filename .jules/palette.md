## 2025-02-27 - Startup Notifications vs Logs
**Learning:** Users perceive extensions that notify on every startup as "spammy". Moving successful startup messages to the Output Channel respects user attention and aligns with the "Good UX is invisible" philosophy.
**Action:** Audit `activate()` functions for unnecessary `showInformationMessage` calls and replace them with `outputChannel.appendLine`.
## 2025-05-22 - Extension Restart Logic
**Learning:** Re-running `activate()` to restart an extension can duplicate UI elements (Status Bar Items, Output Channels) if they are not singleton-managed or disposed.
**Action:** Ensure global UI elements are checked for existence before creation in `activate()` to support soft restarts.
