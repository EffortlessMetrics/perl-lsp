# perl-lsp Documentation Site

This directory contains the mdBook-based documentation site for perl-lsp.

## Quick Start

### Building the Documentation

```bash
# From repository root
just docs-build
```

### Serving Locally

```bash
# From repository root
just docs-serve
```

This will start a local server at `http://localhost:3000` with live reload.

### Cleaning Build Artifacts

```bash
# From repository root
just docs-clean
```

## Structure

```
book/
├── book.toml           # mdBook configuration
├── src/                # Source markdown files
│   ├── SUMMARY.md      # Table of contents
│   ├── introduction.md
│   ├── quick-start.md
│   ├── getting-started/
│   ├── user-guides/
│   ├── architecture/
│   ├── developer/
│   ├── lsp/
│   ├── advanced/
│   ├── reference/
│   ├── dap/
│   ├── ci/
│   ├── process/
│   └── resources/
└── book/               # Build output (generated)
```

## Documentation Organization

The documentation follows the [Diataxis framework](https://diataxis.fr/):

- **Tutorials**: Getting Started section
- **How-to Guides**: User Guides and Developer Guides
- **Explanations**: Architecture and Advanced Topics
- **Reference**: API documentation and specifications

## Source Files

Most documentation files are automatically copied from the `docs/` directory during build.
The `scripts/populate-book.sh` script handles this synchronization.

Key files are created specifically for the book:
- `src/introduction.md`
- `src/quick-start.md`
- `src/SUMMARY.md`

## Deployment

The documentation site is automatically deployed to GitHub Pages on push to `master` branch.

See `.github/workflows/docs-deploy.yml` for the deployment workflow.

## Local Development

For rapid iteration:

1. Make changes to source files in `docs/` or `book/src/`
2. Run `just docs-serve` to see changes with live reload
3. The site will automatically rebuild on file changes

## Configuration

The mdBook configuration is in `book.toml`. Key settings:

- **Search**: Enabled with optimized settings
- **Git Integration**: Links to GitHub repository
- **Theme**: Light mode default with navy dark theme
- **Print Support**: PDF-friendly output enabled

## Troubleshooting

### mdBook Not Found

Install mdBook:

```bash
cargo install mdbook
```

### Build Fails

Ensure all source documentation exists:

```bash
bash scripts/populate-book.sh
```

### Serve Port Conflict

Change the port in the serve command:

```bash
mdbook serve book --port 3001
```

## Contributing

When adding new documentation:

1. Add the markdown file to `docs/` (for project docs) or `book/src/` (for book-specific pages)
2. Update `book/src/SUMMARY.md` to include it in the navigation
3. Update `scripts/populate-book.sh` if the file should be auto-copied
4. Test locally with `just docs-serve`

## Resources

- [mdBook Documentation](https://rust-lang.github.io/mdBook/)
- [Diataxis Framework](https://diataxis.fr/)
- [GitHub Pages](https://pages.github.com/)
