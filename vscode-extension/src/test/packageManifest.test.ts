import * as fs from 'fs';
import * as path from 'path';

describe('package manifest Perl language registration', () => {
    test('registers common Perl project files by filename', () => {
        const manifestPath = path.resolve(__dirname, '../../package.json');
        const packageJson = JSON.parse(fs.readFileSync(manifestPath, 'utf8')) as {
            contributes?: {
                languages?: Array<{
                    id?: string;
                    filenames?: string[];
                }>;
            };
        };

        const perlLanguage = packageJson.contributes?.languages?.find(language => language.id === 'perl');
        expect(perlLanguage).toBeDefined();

        const filenames = perlLanguage?.filenames ?? [];
        expect(filenames).toEqual(expect.arrayContaining([
            'Makefile.PL',
            'Build.PL',
            'cpanfile',
            'cpanfile.snapshot',
            'dist.ini',
        ]));
    });
});
