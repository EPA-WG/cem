#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/cem-transform/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');

const commonInputArgs = [
    'packages/cem_ml/schema-packages/cem-transform/v1/examples/basic-transform.cemt',
    '--content-type',
    'application/vnd.cem.transform+cem',
    '--schema',
    'https://cem.dev/ns/transform/cem/1',
];

const cases = [
    {
        id: 'basic-transform-validate',
        preview: 'basic-transform-validate.svg',
        title: 'CEM transform validation command preview',
        description:
            'Terminal-style preview of the JSON validation report for the basic CEM transform example.',
        terminalTitle: 'validate basic-transform.cemt',
        renderer: 'json',
        args: ['validate', '--format', 'json', ...commonInputArgs],
    },
    {
        id: 'basic-transform-pretty-terminal',
        preview: 'basic-transform-pretty-terminal.svg',
        title: 'CEM transform pretty formatter terminal preview',
        description:
            'Terminal-style preview of colored pretty CEM transform output for the basic transform example.',
        terminalTitle: 'pretty + terminal color',
        renderer: 'ansi',
        args: [
            'convert',
            ...commonInputArgs,
            '--to-content-type',
            'application/vnd.cem.transform+cem',
            '--to-schema',
            'https://cem.dev/ns/transform/cem/1',
            '--cemt-formatter-profile',
            'pretty',
            '--cemt-color-profile',
            'terminal',
            '--output-color-type',
            'ansi-256',
        ],
    },
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'CEM transform',
    refreshCommand:
        'node packages/cem_ml/schema-packages/cem-transform/v1/scripts/verify-previews.mjs --update',
});
