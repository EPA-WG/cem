#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/cem-dom-projection/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');

const commonArgs = ['validate', '--format', 'json'];
const schemaArgs = ['--schema', 'https://cem.dev/ns/projection/dom/1'];

const cases = [
    {
        id: 'basic-dom-binary-validate',
        preview: 'basic-dom-binary-validate.svg',
        title: 'CEM DOM binary validation command preview',
        description:
            'Terminal-style preview of the JSON validation report for the basic binary CEM DOM projection example.',
        terminalTitle: 'validate basic-dom.cem-bin',
        renderer: 'json',
        args: [
            ...commonArgs,
            '--content-type',
            'application/vnd.cem.dom+cem-bin',
            ...schemaArgs,
            'packages/cem_ml/schema-packages/cem-dom-projection/v1/examples/basic-dom.cem-bin',
        ],
    },
    {
        id: 'basic-dom-json-validate',
        preview: 'basic-dom-json-validate.svg',
        title: 'CEM DOM JSON validation command preview',
        description:
            'Terminal-style preview of the JSON validation report for the basic CEM DOM JSON debug view example.',
        terminalTitle: 'validate basic-dom.dom.json',
        renderer: 'json',
        args: [
            ...commonArgs,
            '--content-type',
            'application/vnd.cem.dom+json',
            ...schemaArgs,
            'packages/cem_ml/schema-packages/cem-dom-projection/v1/examples/basic-dom.dom.json',
        ],
    },
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'CEM DOM projection',
    refreshCommand:
        'node packages/cem_ml/schema-packages/cem-dom-projection/v1/scripts/verify-previews.mjs --update',
});
