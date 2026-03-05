# perl-lsp-document-links

Document link extraction for Perl source files used by LSP document link providers.

## Features

- Detects `use Module::Name` imports
- Detects module-form `require Module::Name` imports
- Detects quoted file `require "path.pm"` / `require 'path.pm'` includes
- Returns deferred-resolution link payloads for `documentLink/resolve`
