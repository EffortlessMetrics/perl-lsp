/**
 * Unit tests for buildDisabledFeaturesFromConfig — the pure function that
 * merges enable* settings into the disabledFeatures list before it is sent
 * as initializationOptions to the language server.
 */

jest.mock('vscode-languageclient/node', () => ({
    LanguageClient: class {},
    Trace: { Off: 'off', Messages: 'messages', Verbose: 'verbose' },
    TransportKind: { stdio: 0 },
}));

import { buildDisabledFeaturesFromConfig } from '../extension';

function makeConfig(vals: Record<string, unknown>) {
    return {
        get: <T>(key: string, def: T): T =>
            key in vals ? (vals[key] as T) : def,
    };
}

describe('buildDisabledFeaturesFromConfig', () => {
    test('returns base when both enable* settings are true (defaults)', () => {
        const cfg = makeConfig({ disabledFeatures: [] });
        expect(buildDisabledFeaturesFromConfig(cfg)).toEqual([]);
    });

    test('adds lsp.semantic_tokens when enableSemanticTokens is false', () => {
        const cfg = makeConfig({ disabledFeatures: [], enableSemanticTokens: false });
        expect(buildDisabledFeaturesFromConfig(cfg)).toContain('lsp.semantic_tokens');
    });

    test('adds lsp.formatting when enableFormatting is false', () => {
        const cfg = makeConfig({ disabledFeatures: [], enableFormatting: false });
        expect(buildDisabledFeaturesFromConfig(cfg)).toContain('lsp.formatting');
    });

    test('adds both IDs when both enable* settings are false', () => {
        const cfg = makeConfig({ disabledFeatures: [], enableSemanticTokens: false, enableFormatting: false });
        const result = buildDisabledFeaturesFromConfig(cfg);
        expect(result).toContain('lsp.semantic_tokens');
        expect(result).toContain('lsp.formatting');
    });

    test('preserves existing entries in disabledFeatures', () => {
        const cfg = makeConfig({ disabledFeatures: ['lsp.hover'], enableFormatting: false });
        const result = buildDisabledFeaturesFromConfig(cfg);
        expect(result).toContain('lsp.hover');
        expect(result).toContain('lsp.formatting');
    });

    test('does not duplicate lsp.semantic_tokens if already in disabledFeatures', () => {
        const cfg = makeConfig({ disabledFeatures: ['lsp.semantic_tokens'], enableSemanticTokens: false });
        const result = buildDisabledFeaturesFromConfig(cfg);
        expect(result.filter(x => x === 'lsp.semantic_tokens')).toHaveLength(1);
    });

    test('does not duplicate lsp.formatting if already in disabledFeatures', () => {
        const cfg = makeConfig({ disabledFeatures: ['lsp.formatting'], enableFormatting: false });
        const result = buildDisabledFeaturesFromConfig(cfg);
        expect(result.filter(x => x === 'lsp.formatting')).toHaveLength(1);
    });

    test('does not mutate the original disabledFeatures array', () => {
        const orig = ['lsp.hover'];
        const cfg = makeConfig({ disabledFeatures: orig, enableFormatting: false });
        buildDisabledFeaturesFromConfig(cfg);
        expect(orig).toEqual(['lsp.hover']);
    });

    test('does not add semantic_tokens when enableSemanticTokens is true (default)', () => {
        const cfg = makeConfig({ disabledFeatures: [], enableSemanticTokens: true });
        expect(buildDisabledFeaturesFromConfig(cfg)).not.toContain('lsp.semantic_tokens');
    });

    test('does not add formatting when enableFormatting is true (default)', () => {
        const cfg = makeConfig({ disabledFeatures: [], enableFormatting: true });
        expect(buildDisabledFeaturesFromConfig(cfg)).not.toContain('lsp.formatting');
    });
});
