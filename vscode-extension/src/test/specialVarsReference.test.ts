/**
 * Unit tests for specialVarsReference.
 *
 * Tests cover:
 * - SPECIAL_VARS covers all 27 known variables
 * - buildSpecialVarsHtml returns valid HTML with expected content
 * - all entries have non-empty name, description, and example
 */

import { SPECIAL_VARS, buildSpecialVarsHtml } from '../specialVarsReference';

describe('specialVarsReference', () => {
    test('SPECIAL_VARS covers the 27 known variables', () => {
        const names = SPECIAL_VARS.map(v => v.name);
        expect(names).toContain('$_');
        expect(names).toContain('@_');
        expect(names).toContain('%ENV');
        expect(names).toContain('$!');
        expect(names).toContain('$?');
        expect(names).toContain('@ARGV');
        expect(names).toContain('%SIG');
        expect(names).toContain('$^V');
        expect(names).toContain('$^A');
        expect(names).toContain('$^T');
        expect(SPECIAL_VARS.length).toBeGreaterThanOrEqual(27);
    });

    test('buildSpecialVarsHtml returns valid HTML', () => {
        const html = buildSpecialVarsHtml();
        expect(html).toContain('<!DOCTYPE html>');
        expect(html).toContain('$_');
        expect(html).toContain('%ENV');
        expect(html).toContain('perldoc.perl.org/perlvar');
        expect(html).not.toContain('<script');  // enableScripts: false
    });

    test('all SPECIAL_VARS entries have non-empty name, description, and example', () => {
        for (const v of SPECIAL_VARS) {
            expect(v.name.length).toBeGreaterThan(0);
            expect(v.description.length).toBeGreaterThan(0);
            expect(v.example.length).toBeGreaterThan(0);
        }
    });
});
