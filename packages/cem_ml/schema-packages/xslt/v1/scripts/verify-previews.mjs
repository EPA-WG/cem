#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/xslt/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const packageLabel = 'XSLT Schema Package v1';
const schema = 'https://cem.dev/ns/transform/xslt/1';

function xsltConvertCase({ id, file, contentType }) {
    const path = `packages/cem_ml/schema-packages/xslt/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `xslt ${file}`,
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

function xsltCompatibilityCase({ id, file }) {
    const path = `packages/cem_ml/schema-packages/xslt/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `xslt compatibility ${file}`,
        renderer: 'ansi',
        expectedStatus: 'success',
        width: 980,
        minHeight: 190,
        args: [
            'convert',
            '--input-spec',
            `uri=${path},contentType=custom-element-xslt,schema=${schema}`,
            '--to-content-type',
            'application/cem',
            '--to-schema',
            'https://cem.dev/ns/cem-ml/1',
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-color-profile',
            'terminal',
            '--output-color-type',
            'ansi-256',
        ],
    };
}

function xsltValidateCase({ id, file }) {
    const path = `packages/cem_ml/schema-packages/xslt/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `xslt validate ${file}`,
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
            'application/xslt+xml',
            '--schema',
            schema,
            path,
        ],
    };
}

const cases = [
    xsltConvertCase({ id: 'basic-stylesheet', file: 'basic-stylesheet.xsl', contentType: 'application/xslt+xml' }),
    xsltConvertCase({ id: 'named-template', file: 'named-template.xslt', contentType: 'text/xsl' }),
    xsltCompatibilityCase({ id: 'legacy-custom-element-stylesheet', file: 'legacy-custom-element-stylesheet.xsl' }),
    xsltCompatibilityCase({ id: 'legacy-custom-element-fragment', file: 'legacy-custom-element-fragment.html' }),
    xsltConvertCase({ id: 'unsupported-extension-warning', file: 'unsupported-extension-warning.xsl', contentType: 'application/xslt+xml' }),
    xsltValidateCase({ id: 'invalid-missing-namespace', file: 'invalid-missing-namespace.xsl' }),
    xsltValidateCase({ id: 'invalid-missing-version', file: 'invalid-missing-version.xsl' }),
    xsltValidateCase({ id: 'invalid-external-include', file: 'invalid-external-include.xsl' }),
    xsltValidateCase({ id: 'invalid-missing-entrypoint', file: 'invalid-missing-entrypoint.xsl' }),
    xsltValidateCase({ id: 'invalid-not-well-formed', file: 'invalid-not-well-formed.xsl' }),
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'XSLT',
    refreshCommand:
        'node packages/cem_ml/schema-packages/xslt/v1/scripts/verify-previews.mjs --update',
});
