#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/relax-ng/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const packageLabel = 'RELAX NG Schema Package';
const schema = 'https://cem.dev/ns/data/relax-ng/1';

function relaxNgConvertCase({ id, file, contentType }) {
    const path = `packages/cem_ml/schema-packages/relax-ng/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `relax-ng ${file}`,
        renderer: 'html',
        expectedStatus: 'success',
        width: 980,
        minHeight: 190,
        args: [
            'convert',
            '--input-spec',
            `uri=${path},contentType=${contentType},schema=${schema}`,
            '--to-content-type',
            contentType,
            '--to-schema',
            schema,
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-color-profile',
            'html',
        ],
    };
}

function relaxNgValidateCase({ id, file, contentType }) {
    const path = `packages/cem_ml/schema-packages/relax-ng/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `relax-ng validate ${file}`,
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
            contentType,
            '--schema',
            schema,
            path,
        ],
    };
}

const xmlContentType = 'application/relax-ng+xml';
const compactContentType = 'application/relax-ng-compact-syntax';
const cases = [
    relaxNgConvertCase({ id: 'basic-schema-xml', file: 'basic-schema.rng', contentType: xmlContentType }),
    relaxNgConvertCase({ id: 'datatype-schema', file: 'datatype-schema.rng', contentType: xmlContentType }),
    relaxNgConvertCase({
        id: 'basic-schema-compact',
        file: 'basic-schema.rnc',
        contentType: compactContentType,
    }),
    relaxNgValidateCase({
        id: 'invalid-missing-start',
        file: 'invalid-missing-start.rng',
        contentType: xmlContentType,
    }),
    relaxNgValidateCase({
        id: 'invalid-unknown-element',
        file: 'invalid-unknown-element.rng',
        contentType: xmlContentType,
    }),
    relaxNgValidateCase({
        id: 'invalid-unclosed-compact',
        file: 'invalid-unclosed-compact.rnc',
        contentType: compactContentType,
    }),
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'RELAX NG',
    refreshCommand: 'node packages/cem_ml/schema-packages/relax-ng/v1/scripts/verify-previews.mjs --update',
});
