# Rust 1.95 / 0.14.0 rollout map (historical)

> **This doc is now a pointer.**
>
> When this file was first written, the workspace was on Rust 1.93 and
> the 0.14.0 line had not been planned. The MSRV / toolchain /
> `clippy.toml`-msrv bumps have since shipped (#8509), and the rollout
> has moved on. The **current, canonical post-landing roadmap** lives at:
>
> **[`../development/RUST_1_95_ROLLOUT.md`](../development/RUST_1_95_ROLLOUT.md)**
>
> That doc carries:
>
> 1. Already-landed state (current-state facts + ratchet table).
> 2. Remaining implementation ladder (single canonical PR list).
> 3. Per-rail acceptance contracts (one per row).
> 4. Claude / Codex operating contract.
>
> Sibling rails:
>
> - [`../development/STRONG_CLIPPY_LINTS_ROLLOUT.md`](../development/STRONG_CLIPPY_LINTS_ROLLOUT.md)
>   — strong-clippy-lints activation rail (umbrella #8590).
> - [`../development/RUST_1_95_PROACTIVE_GUARDS.md`](../development/RUST_1_95_PROACTIVE_GUARDS.md)
>   — proactive CI integrity guards rail (umbrella #8662).
>
> Umbrella tracking for the consolidation that produced this pointer:
> **#8663**.

The original 20-step Rust-1.93-framed ladder previously hosted at this
path has been folded into the canonical roadmap with current branch /
issue / objective / acceptance-contract data per row. If you arrived
here via an older commit hash or stale link, follow the link above.
