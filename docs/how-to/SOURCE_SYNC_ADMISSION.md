# Engineering source-sync admission

Controller: EffortlessMetrics/perl-lsp-swarm#17224. Public consumer: #10121.

`Publication Sync Contract` remains the required admission context. The workflow
loads its validator from the exact trusted public base. The closed dispatch policy
in that validator recognizes the historical `v0.18_publication_sync.v2` envelope
and the non-release `source_sync.v1` envelope. Candidate packet fields cannot add
profiles, choose invariant sets, or change that policy. Unsupported and missing
profiles reject a declared transaction.

A legacy schema string alone also cannot select the historical release success
path. `.ci/policies/publication-sync-admission.v1.json` in the trusted R tree must
authorize the exact prepared release subject S. Post-merge reads that policy from
the wrapper's actual first parent, never from the candidate tree. The initial
policy authorizes no new historical release transaction. A candidate-authored
policy, another subject's authorization, or absent/unreadable policy cannot grant
admission. Historical packet/schema/proof interpretation remains intact after
this explicit authorization; unchanged inherited packets remain ordinary.

The source envelope binds semantic boundary B, public base R, exact swarm product
subject S, core join J0, projection P, ordered join parents [R,S], source and
destination identities, and policy/reconciliation/manifest/product-proof digests.
The packet-carrying head J cannot name its own hash; admission must derive J0 from
J with identical metadata and ordered parents after removing the closed three-file
control directory. `expected_projected_tree` names P, not tree(J). A digest binds
bytes and does not establish an evidence producer's authenticity or adequacy.

S is an engineering product subject. It does not claim release preparation, freeze,
release notes, version change, tag/channel selection, release environments or
publication acceptance. Source sync has no publication authority: channels are
empty and `release_cut` is false. Changing a historical release packet's profile
alone cannot make it a source packet; the two closed input shapes differ.

## Current implementation boundary

This is the dispatcher/envelope stage, not successful source admission.
Every declared source transaction returns `source_sync.v1 NOT_PROVEN`, including
a syntactically valid packet and a synthetic protected wrapper. No JSON `pass`
string can enable a source success path. Unchanged inherited control state remains
ordinary; it is not re-admitted as a new transaction. The existing #10123 control
identity repair and #10125 trusted-base repair remain independent prerequisites.

Admission may be enabled only by reviewed trusted-base implementation consuming:

- the landed #14570 model, extended by swarm #17225 rather than copied;
- swarm #17226's complete reconciliation population and required-port survival;
- #7973/#7976's deterministic complete-tree projection, independently derived
  changed-path denominator, exact blob/mode identities, destination context and
  applicable product/build/test qualification;
- authentic/current observations of the protected source transition where needed.
  Release-only live qualification is outside this source catch-up claim.

Product evidence may be reused only for byte-identical covered surfaces. A shared
product transformation needs fresh qualification. Missing, stale, cross-subject or
inaccessible required evidence remains blocked. Producer contract shapes are owned
by those mechanisms; this envelope does not invent a parallel manifest or ledger.

The coordinator owns actual history-preserving joins, serialization against exact
R/J and protected landing proof: wrapper parents [R,J], both histories retained,
tree(M)=tree(J). This stage constructs no live join and changes no protection.

## Focused proof

Run both standard-library suites on real disposable Git repositories:

```sh
python3 -B scripts/tests/test-publication-sync-contract.py
python3 -B scripts/tests/test-source-sync-dispatch.py
```

The dedicated test workflow runs the same two suites. It exercises candidate code
as test input; it does not replace the trusted-base admission workflow or qualify
an actual sync. Historical release success, ordinary behavior and source
NOT_PROVEN are distinct outcomes. No release/tag/package/deployment action belongs
to this stage.
