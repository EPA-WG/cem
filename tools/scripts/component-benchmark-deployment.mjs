// Benchmark-only assembly of already built artifacts, using the demo's shared
// CEM-QL adapter contract. This neither rebuilds nor updates the user's demo.
import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import { resolve, join, extname } from 'node:path';
import { publishDemoDeployment } from './demo-deployment.mjs';

export async function prepareComponentBenchmark(root, destination) {
    const vendor = 'packages/cem-elements/dist/lib/internal/runtime-support/vendor';
    const core = 'packages/cem_ql/dist/wasm';
    for (const name of ['cem_ql.js', 'cem_ql_bg.wasm', 'cem_ql_code.js']) {
        assert((await readFile(resolve(root, core, name))).equals(await readFile(resolve(root, vendor, name))),
            `mixed benchmark glue/binary cohort: ${name}; rebuild cem-elements before measuring`);
    }
    const extensions = new Set(['.js', '.mjs', '.wasm', '.json', '.css', '.woff', '.woff2', '.svg', '.xhtml', '.cem', '.cemt']);
    const files = [];
    async function collect(path) {
        for (const entry of await readdir(resolve(root, path), { withFileTypes: true })) {
            const name = `${path}/${entry.name}`;
            if (entry.isDirectory()) await collect(name);
            else if (entry.isFile() && extensions.has(extname(name)) && !name.endsWith('.d.ts')) files.push(name);
        }
    }
    for (const path of ['packages/cem-elements/dist', 'packages/cem-components/dist',
        'packages/cem-components/src/components', 'packages/cem-theme/dist/lib/css', core]) await collect(path);
    const replacements = new Map([
        [`${core}/runtime.js`, `export * from './cem_ql.js';\nimport init from './cem_ql.js';\nlet ready;\nexport default function initialize(options) { return ready ??= init(options); }\n`],
        [`${vendor}/cem_ql.js`, `export * from '../../../../../../cem_ql/dist/wasm/runtime.js';\nexport {default} from '../../../../../../cem_ql/dist/wasm/runtime.js';\n`],
        [`${vendor}/cem_ql_factory.js`, `export * from '../../../../../../cem_ql/dist/wasm/cem_ql_factory.js';\n`],
        [`${vendor}/cem_ql_code.js`, `export * from '../../../../../../cem_ql/dist/wasm/cem_ql_code.js';\n`],
    ]);
    return publishDemoDeployment({ destination, entries: [
        ...files.filter(file => file !== `${vendor}/cem_ql_bg.wasm` && !replacements.has(file))
            .map(file => ({ path: file, source: join(root, file) })),
        ...[...replacements].map(([path, content]) => ({ path, content })),
    ] });
}
