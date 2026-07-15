# Workspace capability matrix

The extension manifest declares a workspace extension that supports virtual and
untrusted workspaces. The runtime now records a `workspace_topology.v1` object
in client trust reports and first-hour receipts so those claims can be checked
against the host state that was actually exercised.

| Host/workspace mode        | Contract                                                                                  | Current evidence                                 | Claim boundary                                                                        |
| -------------------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------- |
| Trusted local single-root  | Supported                                                                                 | Exact-source smoke and topology contract tests   | Exercises one file-backed folder                                                      |
| Trusted local multi-root   | Supported                                                                                 | Exact-source capability smoke and topology tests | Exercises two file-backed folders in a real host                                      |
| Untrusted file workspace   | Supported with host restrictions                                                          | Exact-source capability smoke and topology tests | Exercises a genuinely untrusted file workspace; virtual/remote claims remain separate |
| Virtual workspace/document | Degraded for file-backed operations; language-server document handling remains observable | URI classification tests                         | No claim that a local Rust process can start for every virtual host                   |
| Untitled Perl document     | Degraded for file-backed operations                                                       | Untitled URI classification tests                | No workspace-root path is invented                                                    |
| Remote workspace host      | Not promoted by local proof                                                               | Remote-host classification test                  | Requires a scheduled real remote-host receipt                                         |

The classifier records workspace mode, trust state, host kind, folder count,
URI schemes, untitled/virtual document counts, capability statuses, and
limitations. It intentionally does not claim server initialization, provider
success, remote process execution, or filesystem access for unsupported URI
schemes. Those behaviors require the corresponding host receipt.
