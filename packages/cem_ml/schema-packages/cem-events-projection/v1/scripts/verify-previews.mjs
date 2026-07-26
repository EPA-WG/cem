#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(
    workspaceRoot,
    'packages/cem_ml/schema-packages/cem-events-projection/v1',
);
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');

const commonArgs = ['validate', '--format', 'json'];
const schemaArgs = ['--schema', 'https://cem.dev/ns/projection/events/1'];

const cases = [
    {
        id: 'basic-events-binary-validate',
        preview: 'basic-events-binary-validate.svg',
        title: 'CEM events binary validation command preview',
        description:
            'Terminal-style preview of the JSON validation report for the basic binary CEM events projection example.',
        terminalTitle: 'validate basic-events.cem-bin',
        renderer: 'json',
        args: [
            ...commonArgs,
            '--content-type',
            'application/vnd.cem.events+cem-bin',
            ...schemaArgs,
            'packages/cem_ml/schema-packages/cem-events-projection/v1/examples/basic-events.cem-bin',
        ],
    },
    {
        id: 'basic-events-json-validate',
        preview: 'basic-events-json-validate.svg',
        title: 'CEM events JSON validation command preview',
        description:
            'Terminal-style preview of the JSON validation report for the basic CEM events JSON debug view example.',
        terminalTitle: 'validate basic-events.events.json',
        renderer: 'json',
        args: [
            ...commonArgs,
            '--content-type',
            'application/vnd.cem.events+json',
            ...schemaArgs,
            'packages/cem_ml/schema-packages/cem-events-projection/v1/examples/basic-events.events.json',
        ],
    },
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'CEM events projection',
    refreshCommand:
        'node packages/cem_ml/schema-packages/cem-events-projection/v1/scripts/verify-previews.mjs --update',
});
