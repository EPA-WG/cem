#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/xhtml/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const packageLabel = 'XHTML Resource Schema Package';
const schema = 'https://cem.dev/ns/data/xhtml/1';

function xhtmlConvertCase({ id, file }) {
    const path = `packages/cem_ml/schema-packages/xhtml/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `xhtml ${file}`,
        renderer: 'html',
        expectedStatus: 'success',
        width: 980,
        minHeight: 190,
        args: [
            'convert',
            '--input-spec',
            `uri=${path},contentType=application/xhtml+xml,schema=${schema}`,
            '--to-content-type',
            'application/xhtml+xml',
            '--to-schema',
            schema,
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-color-profile',
            'html',
        ],
    };
}

function xhtmlValidateCase({ id, file }) {
    const path = `packages/cem_ml/schema-packages/xhtml/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `xhtml validate ${file}`,
        renderer: 'json',
        expectedStatus: 'success',
        width: 1040,
        minHeight: 520,
        args: [
            'validate',
            '--format',
            'json',
            '--fail-level',
            'parse',
            '--content-type',
            'application/xhtml+xml',
            '--schema',
            schema,
            path,
        ],
    };
}

const cases = [
    xhtmlConvertCase({ id: 'basic-document', file: 'basic-document.xhtml' }),
    xhtmlConvertCase({ id: 'form-page', file: 'form-page.xhtml' }),
    xhtmlValidateCase({
        id: 'invalid-missing-namespace',
        file: 'invalid-missing-namespace.xhtml',
    }),
    xhtmlValidateCase({
        id: 'invalid-body-before-head',
        file: 'invalid-body-before-head.xhtml',
    }),
    xhtmlValidateCase({
        id: 'invalid-not-well-formed',
        file: 'invalid-not-well-formed.xhtml',
    }),
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'XHTML',
    refreshCommand:
        'node packages/cem_ml/schema-packages/xhtml/v1/scripts/verify-previews.mjs --update',
});
