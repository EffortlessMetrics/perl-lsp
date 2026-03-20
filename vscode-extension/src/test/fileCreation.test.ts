/**
 * Unit tests for smart file creation with package boilerplate.
 *
 * Tests cover the pure logic exported from fileCreation.ts:
 *   - inferPackageName: derive Perl package name from file path
 *   - generateBoilerplate: produce correct file content for .pm and .t files
 *
 * Issue: #2056
 */

import {
    inferPackageName,
    generateBoilerplate,
    FileKind,
} from '../fileCreation';

// ---------------------------------------------------------------------------
// inferPackageName
// ---------------------------------------------------------------------------
describe('inferPackageName', () => {
    test('derives package from lib/ path', () => {
        expect(inferPackageName('/project/lib/Foo/Bar.pm')).toBe('Foo::Bar');
    });

    test('derives package from nested lib/ path', () => {
        expect(inferPackageName('/project/lib/My/Long/Module.pm')).toBe('My::Long::Module');
    });

    test('derives package from top-level lib/ module', () => {
        expect(inferPackageName('/project/lib/Foo.pm')).toBe('Foo');
    });

    test('derives package from lib/ with Windows-style separators', () => {
        // path.sep may be / on Linux; test cross-platform via explicit slashes
        expect(inferPackageName('C:\\project\\lib\\Foo\\Bar.pm')).toBe('Foo::Bar');
    });

    test('falls back to basename without extension when no lib/ anchor', () => {
        expect(inferPackageName('/project/Foo/Bar.pm')).toBe('Bar');
    });

    test('returns null for .t files (no package name)', () => {
        expect(inferPackageName('/project/t/foo.t')).toBeNull();
    });

    test('returns null for non-.pm files', () => {
        expect(inferPackageName('/project/lib/Foo.pl')).toBeNull();
    });

    test('handles deep lib anchor correctly', () => {
        expect(inferPackageName('/home/user/myapp/lib/App/Controller/Root.pm')).toBe(
            'App::Controller::Root'
        );
    });
});

// ---------------------------------------------------------------------------
// generateBoilerplate — .pm files
// ---------------------------------------------------------------------------
describe('generateBoilerplate for .pm files', () => {
    test('returns FileKind.Module for .pm extension', () => {
        const result = generateBoilerplate('/project/lib/Foo/Bar.pm')!;
        expect(result.kind).toBe(FileKind.Module);
    });

    test('includes correct package declaration', () => {
        const result = generateBoilerplate('/project/lib/Foo/Bar.pm')!;
        expect(result.content).toContain('package Foo::Bar;');
    });

    test('includes use strict', () => {
        const result = generateBoilerplate('/project/lib/Foo/Bar.pm')!;
        expect(result.content).toContain('use strict;');
    });

    test('includes use warnings', () => {
        const result = generateBoilerplate('/project/lib/Foo/Bar.pm')!;
        expect(result.content).toContain('use warnings;');
    });

    test('ends with 1;', () => {
        const result = generateBoilerplate('/project/lib/Foo/Bar.pm')!;
        expect(result.content.trimEnd()).toMatch(/1;$/);
    });

    test('package declaration appears before use strict', () => {
        const result = generateBoilerplate('/project/lib/Foo/Bar.pm')!;
        const pkgIdx = result.content.indexOf('package');
        const strictIdx = result.content.indexOf('use strict');
        expect(pkgIdx).toBeLessThan(strictIdx);
    });

    test('uses fallback basename when no lib/ anchor', () => {
        const result = generateBoilerplate('/project/MyModule.pm')!;
        expect(result.content).toContain('package MyModule;');
    });
});

// ---------------------------------------------------------------------------
// generateBoilerplate — .t files
// ---------------------------------------------------------------------------
describe('generateBoilerplate for .t files', () => {
    test('returns FileKind.Test for .t extension', () => {
        const result = generateBoilerplate('/project/t/foo.t')!;
        expect(result.kind).toBe(FileKind.Test);
    });

    test('includes use strict', () => {
        const result = generateBoilerplate('/project/t/foo.t')!;
        expect(result.content).toContain('use strict;');
    });

    test('includes use warnings', () => {
        const result = generateBoilerplate('/project/t/foo.t')!;
        expect(result.content).toContain('use warnings;');
    });

    test('includes use Test::More', () => {
        const result = generateBoilerplate('/project/t/foo.t')!;
        expect(result.content).toContain('use Test::More;');
    });

    test('ends with done_testing', () => {
        const result = generateBoilerplate('/project/t/foo.t')!;
        expect(result.content.trimEnd()).toMatch(/done_testing;$/);
    });

    test('does not include a package declaration', () => {
        const result = generateBoilerplate('/project/t/foo.t')!;
        expect(result.content).not.toContain('package ');
    });

    test('does not include 1; terminator', () => {
        const result = generateBoilerplate('/project/t/foo.t')!;
        expect(result.content).not.toContain('\n1;');
    });
});

// ---------------------------------------------------------------------------
// generateBoilerplate — unsupported extensions
// ---------------------------------------------------------------------------
describe('generateBoilerplate for unsupported files', () => {
    test('returns null for .pl files', () => {
        expect(generateBoilerplate('/project/script.pl')).toBeNull();
    });

    test('returns null for .pod files', () => {
        expect(generateBoilerplate('/project/doc.pod')).toBeNull();
    });
});
