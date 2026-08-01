#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/mathml/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const packageLabel = 'MathML Schema Package';
const schema = 'https://cem.dev/ns/data/mathml/1';

function mathmlConvertCase({ id, file, contentType }) {
    const path = `packages/cem_ml/schema-packages/mathml/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `mathml ${file}`,
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

function mathmlValidateCase({ id, file, contentType }) {
    const path = `packages/cem_ml/schema-packages/mathml/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `mathml validate ${file}`,
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

const generic = 'application/mathml+xml';
const content = 'application/mathml-content+xml';
const cases = [
    mathmlConvertCase({
        id: 'basic-presentation',
        file: 'basic-presentation.mml',
        contentType: generic,
    }),
    mathmlConvertCase({
        id: 'content-expression',
        file: 'content-expression.mathml',
        contentType: content,
    }),
    mathmlConvertCase({
        id: 'semantics-external-annotation',
        file: 'semantics-external-annotation.mml',
        contentType: generic,
    }),
    mathmlValidateCase({
        id: 'invalid-missing-namespace',
        file: 'invalid-missing-namespace.mml',
        contentType: generic,
    }),
    mathmlValidateCase({
        id: 'invalid-root-not-math',
        file: 'invalid-root-not-math.mml',
        contentType: generic,
    }),
    mathmlValidateCase({
        id: 'invalid-content-profile-presentation-only',
        file: 'invalid-content-profile-presentation-only.mml',
        contentType: content,
    }),
    mathmlValidateCase({
        id: 'invalid-not-well-formed',
        file: 'invalid-not-well-formed.mml',
        contentType: generic,
    }),
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'MathML',
    refreshCommand:
        'node packages/cem_ml/schema-packages/mathml/v1/scripts/verify-previews.mjs --update',
});
