# Perl::Tidy Integration Options Analysis

## Current Implementation
- Shells out to `perltidy` command
- Requires users to install perltidy separately
- Gracefully handles missing perltidy

## Integration Options

### 1. Keep Current Approach (Recommended) ✅
**Pros:**
- Simple and maintainable
- Users get latest perltidy version
- No licensing concerns
- Supports custom perltidy installations
- Zero binary size increase

**Cons:**
- Requires separate installation
- Platform-dependent availability

### 2. Create Rust Bindings
**Pros:**
- Direct integration
- No subprocess overhead

**Cons:**
- Complex FFI implementation
- Perl::Tidy is pure Perl (would need Perl interpreter)
- Maintenance burden
- No existing crate available

### 3. Bundle Perltidy Binary
**Pros:**
- Works out of the box
- No installation required

**Cons:**
- Large binary size (4.5MB+)
- Platform-specific builds needed
- Version management complexity
- Licensing considerations
- No official standalone binaries

### 4. Implement Basic Formatter in Rust
**Pros:**
- No external dependencies
- Fast and integrated
- Consistent behavior

**Cons:**
- Massive implementation effort
- Won't match perltidy's 20+ years of refinement
- Incompatible with existing .perltidyrc files

## Recommendation

Stay with the current approach (Option 1) because:

1. **Industry Standard**: Perl developers already use perltidy
2. **Configuration**: Respects existing .perltidyrc files
3. **Simplicity**: Clean separation of concerns
4. **Updates**: Users control their perltidy version
5. **Size**: Keeps our binary small and focused

## Improvements to Current Approach

1. **Better Discovery**:
   ```rust
   // Check multiple locations
   - PATH environment
   - Common Perl locations (/usr/bin/perl, /usr/local/bin)
   - perlbrew/plenv installations
   ```

2. **Installation Helper**:
   ```rust
   // Provide helpful error messages
   if perltidy_not_found {
       return Err("Perltidy not found. Install with:
         - cpan Perl::Tidy
         - apt-get install perltidy (Debian/Ubuntu)
         - yum install perltidy (RedHat/Fedora)
         - brew install perltidy (macOS)");
   }
   ```

3. **Fallback Formatter**:
   ```rust
   // Basic indentation-only formatter
   // When perltidy is not available
   ```

## Decision

Keep the current subprocess approach. It's the most maintainable and aligns with how other language servers handle external formatters (e.g., rustfmt, prettier, black).