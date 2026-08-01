#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/svg/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const packageLabel = 'SVG Resource Schema Package';
const schema = 'https://cem.dev/ns/data/svg/1';

function svgConvertCase({ id, file }) {
    const path = `packages/cem_ml/schema-packages/svg/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `svg ${file}`,
        renderer: 'html',
        expectedStatus: 'success',
        width: 980,
        minHeight: 190,
        args: [
            'convert',
            '--input-spec',
            `uri=${path},contentType=image/svg+xml,schema=${schema}`,
            '--to-content-type',
            'image/svg+xml',
            '--to-schema',
            schema,
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-color-profile',
            'html',
        ],
    };
}

function svgValidateCase({ id, file }) {
    const path = `packages/cem_ml/schema-packages/svg/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `svg validate ${file}`,
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
            'image/svg+xml',
            '--schema',
            schema,
            path,
        ],
    };
}

const cases = [
    svgConvertCase({ id: 'basic-icon', file: 'basic-icon.svg' }),
    svgConvertCase({ id: 'bar-chart', file: 'bar-chart.svg' }),
    svgConvertCase({ id: 'unnamed-icon', file: 'unnamed-icon.svg' }),
    svgValidateCase({
        id: 'invalid-missing-namespace',
        file: 'invalid-missing-namespace.svg',
    }),
    svgValidateCase({ id: 'invalid-script', file: 'invalid-script.svg' }),
    svgValidateCase({
        id: 'invalid-external-image',
        file: 'invalid-external-image.svg',
    }),
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'SVG',
    refreshCommand:
        'node packages/cem_ml/schema-packages/svg/v1/scripts/verify-previews.mjs --update',
});
