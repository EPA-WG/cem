import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { chmodSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { dirname, relative, resolve, sep } from 'node:path';
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
if (packageMetadata.bin?.['cem-ml'] !== './dist/bin.js') {
    throw new Error('CEM-ML CLI must publish the version-synchronized cem-ml executable');
}

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
chmodSync(resolve(projectRoot, 'dist/bin.js'), 0o755);

const runtimeManifest = readJson(resolve(workspaceRoot, 'packages/cem-ml-npm/dist/cem-ml-runtime.json'));
if (runtimeManifest.commonVersion !== cargoVersion) {
    throw new Error('low-level runtime manifest drifted from the CLI package version');
}
writeJson(resolve(projectRoot, 'dist/cem-ml-cli-runtime.json'), {
    schemaVersion: 1,
    package: { name: packageMetadata.name, version: packageMetadata.version },
    commonVersion: cargoVersion,
    runtimeDependency: {
        name: runtimeMetadata.name,
        version: runtimeMetadata.version,
        integritySha256: sha256File(resolve(workspaceRoot, 'packages/cem-ml-npm/dist/integrity.json')),
    },
    abi: runtimeManifest.abi,
    runtimeIdentities: ['wasm-browser-worker', 'wasm-node'],
    targetIdentities: ['wasm32-unknown-unknown:web', 'wasm32-unknown-unknown:nodejs'],
    capabilities: runtimeManifest.capabilities,
    commandSchema: {
        schemaVersion: commandSchema.schemaVersion,
        sha256: sha256File(commandSchemaPath),
    },
});
const integrityFiles = listFiles(resolve(projectRoot, 'dist'))
    .filter((path) => !path.endsWith('/integrity.json'))
    .map((path) => {
        const bytes = readFileSync(resolve(projectRoot, 'dist', path));
        return { path, byteLength: bytes.byteLength, sha256: createHash('sha256').update(bytes).digest('hex') };
    });
writeJson(resolve(projectRoot, 'dist/integrity.json'), {
    schemaVersion: 1,
    algorithm: 'sha256',
    commonVersion: cargoVersion,
    files: integrityFiles,
});

console.log(
    `Built ${packageMetadata.name}@${packageMetadata.version} browser/Node hosts with command schema v${commandSchema.schemaVersion}.`,
);

function readJson(path) {
    return JSON.parse(readFileSync(path, 'utf8'));
}

function writeJson(path, value) {
    writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function sha256File(path) {
    return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function listFiles(root) {
    const files = [];
    const visit = (directory) => {
        for (const entry of readdirSync(directory, { withFileTypes: true })) {
            const path = resolve(directory, entry.name);
            if (entry.isDirectory()) visit(path);
            else if (entry.isFile() && statSync(path).isFile()) files.push(relative(root, path).split(sep).join('/'));
        }
    };
    visit(root);
    return files.sort();
}
