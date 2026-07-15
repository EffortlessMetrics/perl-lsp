# Bundled source-map policy

The development Rolldown bundle retains `out/extension.js.map` with embedded
source content. The VSIX deliberately excludes `**/*.map`; source maps are
diagnostic evidence, not shipped product assets.

`npm run check:source-map` validates the exact bundle/map pair, resolves a
known generated frame back to `src/workspaceTopology.ts`, records bundle and
map SHA-256 values, and writes:

```text
target/receipts/vscode-source-map/extension.js
target/receipts/vscode-source-map/extension.js.map
target/receipts/vscode-source-map/source-map-receipt.json
```

The publishing workflow archives those files beside the exact VSIX build. The
receipt also records the extension version, source revision, VSIX hash when a
single VSIX is present, and Rolldown version. Archive retention is CI artifact
retention; no source map is uploaded to the Marketplace or Open VSX by this
contract.

The check proves source-map resolution for a known bundled frame and hash
correspondence. It does not claim a production crash, automatic crash upload,
or public access to embedded source content.
