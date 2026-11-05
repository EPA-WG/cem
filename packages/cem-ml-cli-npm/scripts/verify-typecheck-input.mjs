import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { stripVTControlCharacters } from 'node:util';
import ts from 'typescript';

const project = resolve(import.meta.dirname, '..'), workspace = resolve(project, '../..');
const generated = resolve(project, 'src/generated'), distribution = resolve(project, 'dist');
const schemaPath = resolve(workspace, 'dist/generated/cem-ml-command-schema/command-schema.json');
const schemaBytes = readFileSync(schemaPath), schema = JSON.parse(schemaBytes);
const cache = mkdtempSync(resolve(tmpdir(), 'cem-cli-typecheck-cache-'));
const env = { ...process.env, NX_CACHE_DIRECTORY: resolve(cache, 'outputs'),
    NX_WORKSPACE_DATA_DIRECTORY: resolve(cache, 'workspace-data'), NX_DAEMON: 'false', NX_SKIP_NX_CACHE: 'false' };
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const source = () => readFileSync(resolve(generated, 'command-schema.ts'));
const emitted = () => readFileSync(resolve(distribution, 'generated/command-schema.js'));

function nx(args, success = true) {
    const run = spawnSync(process.platform === 'win32' ? 'yarn.cmd' : 'yarn', ['nx', ...args], {
        cwd: workspace, env, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024,
    });
    if (run.error) throw run.error;
    const output = stripVTControlCharacters(`${run.stdout}\n${run.stderr}`);
    if (success) assert.equal(run.status, 0, output);
    else assert.notEqual(run.status, 0, output);
    return output;
}
const build = () => nx(['run', '@epa-wg/cem-ml-cli:build', '--excludeTaskDependencies']);
const removeOutputs = () => {
    rmSync(generated, { recursive: true, force: true });
    rmSync(distribution, { recursive: true, force: true });
};
function checkRuntimeDeclarations() {
    // Use the real solution builder with tsc --build's options. Capture emitted
    // declarations/build-info in memory so this integration gate does not
    // overwrite other packages' post-processed publication artifacts.
    const outputs = new Map(), directories = new Set(), diagnostics = [];
    const system = { ...ts.sys,
        writeFile(file, text) {
            outputs.set(resolve(file), text);
            for (let directory = dirname(resolve(file)); !directories.has(directory); directory = dirname(directory)) {
                directories.add(directory); if (directory === dirname(directory)) break;
            }
        },
        readFile: file => outputs.get(resolve(file)) ?? ts.sys.readFile(file),
        fileExists: file => outputs.has(resolve(file)) || ts.sys.fileExists(file),
        directoryExists: directory => directories.has(resolve(directory)) || ts.sys.directoryExists(directory),
        realpath: file => outputs.has(resolve(file)) ? resolve(file) : ts.sys.realpath(file),
        getModifiedTime: file => outputs.has(resolve(file)) ? new Date() : ts.sys.getModifiedTime(file),
        createDirectory: directory => { directories.add(resolve(directory)); },
        setModifiedTime() { /* On-disk timestamps belong to their package builds. */ },
        deleteFile: file => { outputs.delete(resolve(file)); },
    };
    const host = ts.createSolutionBuilderHost(system, undefined,
        diagnostic => diagnostics.push(ts.flattenDiagnosticMessageText(diagnostic.messageText, '\n')),
        () => { /* Compiler errors are collected; routine status needs no output. */ });
    const status = ts.createSolutionBuilder(host, [resolve(workspace, 'packages/cem-elements/tsconfig.json')],
        { force: true, emitDeclarationOnly: true }).build();
    assert.equal(status, 0, diagnostics.join('\n'));
    assert.ok(outputs.has(resolve(distribution, 'generated/command-schema.d.ts')), 'CLI source must actually be recompiled');
    assert.ok(outputs.has(resolve(workspace, 'packages/cem-elements/dist/lib/cem-elements.d.ts')), 'runtime source must actually be recompiled');
}
let restoreSchema = false;
try {
    assert.doesNotMatch(build(), /\[local cache\]/, 'a private empty cache must execute the cold build');
    assert.deepEqual(readdirSync(generated).sort(), ['command-schema.ts']);
    const originalSource = source(), originalEmitted = emitted();
    const module = await import(pathToFileURL(resolve(distribution, 'generated/command-schema.js')).href);
    assert.deepEqual(module.generatedCommandSchema, schema, 'compiled schema must equal the native emitter input');

    removeOutputs();
    const replay = build();
    assert.match(replay, /\[local cache\]/, 'deleted source/distribution must be restored from the private Nx cache');
    assert.equal(sha256(source()), sha256(originalSource));
    assert.equal(sha256(emitted()), sha256(originalEmitted));

    // Exercise the version guard uncached. This focused leaf invocation skips
    // the native emitter; configured builds hash its declared task outputs.
    restoreSchema = true;
    writeFileSync(schemaPath, JSON.stringify({ ...schema, commonVersion: 'fixture-invalid' }));
    const refused = nx(['run', '@epa-wg/cem-ml-cli:build', '--excludeTaskDependencies', '--skip-nx-cache'], false);
    assert.match(refused, /Generated command schema version\/common version does not match/);
    assert.doesNotMatch(refused, /\[local cache\]/);
    writeFileSync(schemaPath, schemaBytes);

    const buildInfo = readFileSync(resolve(distribution, 'tsconfig.lib.tsbuildinfo'));
    checkRuntimeDeclarations();
    assert.equal(sha256(source()), sha256(originalSource), 'runtime typechecking must retain the exact generated input');
    assert.equal(sha256(emitted()), sha256(originalEmitted));
    assert.equal(sha256(readFileSync(resolve(distribution, 'tsconfig.lib.tsbuildinfo'))), sha256(buildInfo));
    console.log('Verified CLI cold build, exact cached source restoration, native version refusal and standalone runtime typecheck.');
} finally {
    try {
        if (restoreSchema) writeFileSync(schemaPath, schemaBytes);
    } finally { rmSync(cache, { recursive: true, force: true }); }
}
