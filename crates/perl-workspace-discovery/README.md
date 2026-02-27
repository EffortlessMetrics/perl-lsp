# perl-workspace-discovery

Git-aware workspace file discovery for Perl projects.

## Scope

- Discover Perl source files (`.pl`, `.pm`, `.t`, `.psgi`) from a workspace root
- Prefer `git ls-files` for speed and `.gitignore` awareness
- Fall back to `WalkDir` when git is unavailable or the root is not a repository
- Consistently skip heavy/non-source directories: `.git`, `.hg`, `.svn`, `target`, `node_modules`, `.cache`

## API

- `discover_perl_files(root)`
- `DiscoveryResult`
- `DiscoveryMethod`
