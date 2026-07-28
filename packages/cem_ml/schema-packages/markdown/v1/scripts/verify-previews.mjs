#!/usr/bin/env node

import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from '../../../scripts/readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/markdown/v1');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const packageLabel = 'Markdown Resource Schema Package';

function markdownSourceExampleCase({ id, file }) {
    const path = `packages/cem_ml/schema-packages/markdown/v1/examples/${file}`;
    return {
        id: `${id}-preview`,
        preview: `${file}.svg`,
        html: `${file}.html`,
        title: `${packageLabel} ${id} example preview`,
        description: `Preview of examples/${file} from package.cem example metadata.`,
        terminalTitle: `source ${file}`,
        renderer: 'text',
        sourcePath: path,
        width: 920,
        minHeight: 190,
    };
}

const cases = [
    markdownSourceExampleCase({ id: 'basic-document', file: 'basic-document.md' }),
    markdownSourceExampleCase({ id: 'gfm-worklog', file: 'gfm-worklog.md' }),
    markdownSourceExampleCase({ id: 'invalid-embedded-html', file: 'invalid-embedded-html.md' }),
    markdownSourceExampleCase({ id: 'unknown-variant', file: 'unknown-variant.md' }),
];

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update,
    cases,
    packageLabel: 'Markdown',
    refreshCommand:
        'node packages/cem_ml/schema-packages/markdown/v1/scripts/verify-previews.mjs --update',
});
