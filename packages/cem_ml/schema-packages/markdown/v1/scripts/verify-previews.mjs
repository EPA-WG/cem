#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/markdown/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const packageLabel = 'Markdown Resource Schema Package';

function markdownInputSpec(file, contentType) {
    const path = `packages/cem_ml/schema-packages/markdown/v1/examples/${file}`;
    return `uri=${path},contentType=${contentType},schema=https://cem.dev/ns/data/markdown/1`;
}

function markdownCliExampleCase({
    id,
    file,
    contentType = 'text/markdown; charset=utf-8; variant=CommonMark',
    minHeight = 190,
}) {
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `markdown ${file}`,
        renderer: 'html',
        expectedStatus: 'any',
        width: 920,
        minHeight,
        args: [
            'convert',
            '--input-spec',
            markdownInputSpec(file, contentType),
            '--to-content-type',
            'text/markdown',
            '--to-schema',
            'https://cem.dev/ns/data/markdown/1',
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-color-profile',
            'html',
        ],
    };
}

function markdownValidateExampleCase({
    id,
    file,
    contentType = 'text/markdown; charset=utf-8; variant=CommonMark',
    minHeight = 520,
}) {
    const path = `packages/cem_ml/schema-packages/markdown/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `markdown validate ${file}`,
        renderer: 'json',
        expectedStatus: 'success',
        width: 1040,
        minHeight,
        args: [
            'validate',
            '--format',
            'json',
            '--fail-level',
            'parse',
            '--content-type',
            contentType,
            '--schema',
            'https://cem.dev/ns/data/markdown/1',
            path,
        ],
    };
}

const cases = [
    markdownCliExampleCase({ id: 'basic-document', file: 'basic-document.md' }),
    markdownCliExampleCase({
        id: 'gfm-worklog',
        file: 'gfm-worklog.md',
        contentType: 'text/markdown; charset=utf-8; variant=GFM',
        minHeight: 250,
    }),
    markdownValidateExampleCase({
        id: 'invalid-embedded-html',
        file: 'invalid-embedded-html.md',
    }),
    markdownCliExampleCase({
        id: 'unknown-variant',
        file: 'unknown-variant.md',
        contentType: 'text/markdown; charset=utf-8; variant=CustomWiki',
    }),
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'Markdown',
    refreshCommand:
        'node packages/cem_ml/schema-packages/markdown/v1/scripts/verify-previews.mjs --update',
});
