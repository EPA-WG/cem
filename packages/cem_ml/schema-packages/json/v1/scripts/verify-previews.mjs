#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/json/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const packageLabel = 'JSON Resource Schema Package';

const jsonOutputArgs = [
    '--to-content-type',
    'application/json',
    '--to-schema',
    'https://cem.dev/ns/data/json/1',
    '--cemt-formatter-profile',
    'tabular',
    '--cemt-color-profile',
    'terminal',
    '--output-color-type',
    'ansi-256',
];

function jsonExampleCase({ id, file, contentType = 'application/json' }) {
    const path = `packages/cem_ml/schema-packages/json/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `json ${file}`,
        renderer: 'ansi',
        expectedStatus: 'any',
        fallbackSourcePath: path,
        width: 780,
        minHeight: 190,
        args: [
            'convert',
            '--input-spec',
            `uri=${path},contentType=${contentType},schema=https://cem.dev/ns/data/json/1`,
            ...jsonOutputArgs,
        ],
    };
}

const cases = [
    jsonExampleCase({ id: 'basic-object', file: 'basic-object.json' }),
    jsonExampleCase({ id: 'nested-data', file: 'nested-data.json' }),
    jsonExampleCase({ id: 'invalid-trailing-comma', file: 'invalid-trailing-comma.json' }),
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'JSON',
    refreshCommand:
        'node packages/cem_ml/schema-packages/json/v1/scripts/verify-previews.mjs --update',
});
