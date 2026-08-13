import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const packageMetadata = readJson(resolve(projectRoot, 'package.json'));
const runtimeMetadata = readJson(resolve(workspaceRoot, 'packages/cem-ml-npm/package.json'));
const cargoManifest = readFileSync(resolve(workspaceRoot, 'packages/cem_ml/Cargo.toml'), 'utf8');
const cargoVersion = cargoManifest.match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1];

if (!cargoVersion) throw new Error('Could not read the common cem_ml Cargo version');
if (packageMetadata.version !== cargoVersion || runtimeMetadata.version !== cargoVersion) {
    throw new Error('CEM-ML CLI, WASM npm, and common Cargo versions must match exactly');
}
if (packageMetadata.dependencies?.['@epa-wg/cem-ml'] !== cargoVersion) {
    throw new Error('CEM-ML CLI must depend on the exact common @epa-wg/cem-ml version');
}
if ('bin' in packageMetadata) throw new Error('The npm executable requires the later command-service slice');

const commandSchemaPath = resolve(
    workspaceRoot,
    'dist/generated/cem-ml-command-schema/command-schema.json',
);
const commandSchema = readJson(commandSchemaPath);
if (commandSchema.schemaVersion !== 1 || commandSchema.commonVersion !== cargoVersion) {
    throw new Error('Generated command schema version/common version does not match the npm deployment');
}

const generatedRoot = resolve(projectRoot, 'src/generated');
const generatedModule = resolve(generatedRoot, 'command-schema.ts');
rmSync(resolve(projectRoot, 'dist'), { recursive: true, force: true });
mkdirSync(generatedRoot, { recursive: true });
writeFileSync(
    generatedModule,
    `import type { SharedCommandSchema } from '../command.js';\n\nexport const generatedCommandSchema = ${JSON.stringify(commandSchema, null, 2)} as const satisfies SharedCommandSchema;\n`,
);
try {
    const result = spawnSync(
        process.platform === 'win32' ? 'yarn.cmd' : 'yarn',
        ['tsc', '--build', 'tsconfig.lib.json', '--pretty', 'false'],
        {
            cwd: projectRoot,
            stdio: 'inherit',
        },
    );
    if (result.status !== 0) throw new Error(`TypeScript build failed with status ${result.status}`);
} finally {
    rmSync(generatedRoot, { recursive: true, force: true });
}

console.log(
    `Built ${packageMetadata.name}@${packageMetadata.version} browser/Node hosts with command schema v${commandSchema.schemaVersion}.`,
);

function readJson(path) {
    return JSON.parse(readFileSync(path, 'utf8'));
}
