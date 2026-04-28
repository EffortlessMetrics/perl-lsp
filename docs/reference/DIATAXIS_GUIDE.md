# Diataxis Authoring Guide

Use this guide when adding or moving documentation so users can find the right kind of help quickly.

## Quick classifier

Choose the document type based on user intent, not on technical depth:

| User intent | Diataxis type | Typical opening line |
| --- | --- | --- |
| “Teach me by doing.” | Tutorial | “In this walkthrough, you will build…” |
| “Help me solve a task now.” | How-to | “To accomplish X, do the following…” |
| “Tell me the exact contract.” | Reference | “`setting_name` controls… Default: …” |
| “Help me understand why.” | Explanation | “This design exists because…” |

## Placement in this repository

- `docs/tutorials/` → Tutorials
- `docs/how-to/` → How-to guides
- `docs/reference/` → Reference material
- `docs/explanation/` → Explanations

If a document primarily serves one intent but includes a small supporting section from another type, keep that section short and link to the canonical doc for that type.

## Authoring rules

1. **One primary intent per page.**
   - If a page has multiple long sections with different intents, split it.
2. **Link outward instead of embedding everything.**
   - Tutorials should link to reference tables.
   - How-to guides should link to explanation/background as needed.
3. **Use stable sources for claims.**
   - Release line: `Cargo.toml`
   - Capability catalog: `features.toml`
   - Current truth and metrics: `docs/project/CURRENT_STATUS.md`
   - Roadmap state: `docs/project/ROADMAP.md`
4. **Avoid hybrid titles.**
   - Prefer “X Tutorial” or “X Reference”, not “X Guide” when the type is clear.
5. **Keep project status out of evergreen docs.**
   - Point to `docs/project/CURRENT_STATUS.md` for changing metrics and receipts.

## Rewrite patterns

### Convert mixed tutorial/reference page

- Keep the step-by-step path in `docs/tutorials/`.
- Move API tables, option matrices, and exhaustive lists into `docs/reference/`.
- Add a short “See also” block between the two.

### Convert mixed how-to/explanation page

- Keep operational steps in `docs/how-to/`.
- Move rationale and tradeoffs into `docs/explanation/`.
- Add a “Why this works” link in the how-to page.

## Review checklist

Before merging docs changes:

- [ ] The page has a single primary Diataxis intent.
- [ ] Directory matches that intent.
- [ ] Assertions that can drift link to canonical truth sources.
- [ ] Cross-links exist to adjacent document types where helpful.
- [ ] Navigation pages (`docs/README.md`, `docs/INDEX.md`) include the new page when relevant.

## See also

- [docs/README.md](../README.md)
- [docs/INDEX.md](../INDEX.md)
- [Diátaxis framework](https://diataxis.fr/)
