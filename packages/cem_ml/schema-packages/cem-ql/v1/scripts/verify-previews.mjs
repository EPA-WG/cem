#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/cem-ql/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');

const packageLabel = 'CEM-QL Query Resource Schema Package';
const invalidUtf8Path =
    'packages/cem_ml/schema-packages/cem-ql/v1/examples/invalid-utf8.cemql';
const contentType = 'application/vnd.cem.query+cem-ql';
const schema = 'https://cem.dev/ns/query/cem-ql/1';

const cases = [
    {
        id: 'invalid-utf8-preview',
        preview: 'invalid-utf8.cemql.svg',
        html: 'invalid-utf8.cemql.html',
        title: `${packageLabel} invalid-utf8 example preview`,
        description:
            'Preview of examples/invalid-utf8.cemql from package.cem example metadata.',
        terminalTitle: 'tabular invalid-utf8.cemql',
        renderer: 'ansi',
        expectedStatus: 'any',
        args: [
            'convert',
            '--input-spec',
            `uri=${invalidUtf8Path},contentType=${contentType},schema=${schema}`,
            '--to-content-type',
            contentType,
            '--to-schema',
            schema,
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-color-profile',
            'terminal',
            '--output-color-type',
            'ansi-256',
        ],
        fallbackSourcePath: invalidUtf8Path,
        width: 980,
        minHeight: 190,
    },
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel,
    refreshCommand: 'node packages/cem_ml/schema-packages/cem-ql/v1/scripts/verify-previews.mjs --update',
});
