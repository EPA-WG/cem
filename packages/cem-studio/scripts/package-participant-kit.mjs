import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { readLoginPresenter, presenterRoot, workspaceRoot } from './login-presenter.mjs';
import { participantFiles, verifyParticipantFiles } from './participant-kit-launcher.mjs';

const root = resolve(workspaceRoot, 'dist/packages/cem-studio-login-kit');
const archive = `${root}.tgz`;
const site = resolve(workspaceRoot, 'packages/cem-studio/dist/static');
const { manifest } = await readLoginPresenter();
assert.deepEqual(await readFile(resolve(site, 'samples/examples/login.cem-studio.json')),
    await readFile(resolve(presenterRoot, manifest.archive.file)), 'Deployed Login differs from the frozen presenter project');
await rm(root, { recursive: true, force: true });
await mkdir(root, { recursive: true });
await cp(site, resolve(root, 'site'), { recursive: true });
await cp(presenterRoot, resolve(root, 'docs/declarative-amsterdam-2026/login-presenter'), { recursive: true });
await cp(resolve(workspaceRoot, 'docs/declarative-amsterdam-2026/login-walkthrough.md'),
    resolve(root, 'docs/declarative-amsterdam-2026/login-walkthrough.md'));
await mkdir(resolve(root, 'examples/projects/login'), { recursive: true });
await cp(resolve(presenterRoot, 'example.json'), resolve(root, 'examples/projects/login/example.json'));
await cp(resolve(import.meta.dirname, 'serve-preview.mjs'), resolve(root, 'serve-preview.mjs'));
await cp(resolve(import.meta.dirname, 'participant-kit-launcher.mjs'), resolve(root, 'start.mjs'));
await cp(resolve(workspaceRoot, 'docs/declarative-amsterdam-2026/participant-setup.md'), resolve(root, 'README.md'));
const files = [];
for (const path of await participantFiles(root)) {
    const bytes = await readFile(resolve(root, path));
    files.push({ path, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') });
}
await writeFile(resolve(root, 'kit-manifest.json'), `${JSON.stringify({ schemaVersion: 1,
    sourceRevision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: workspaceRoot, encoding: 'utf8' }).trim(),
    loginVersion: manifest.projectVersion, frozenSourceRevision: manifest.sourceRevision,
    privacy: manifest.privacy, nodeEngines: '^22.12.0 || ^24.0.0', files }, null, 2)}\n`);
await verifyParticipantFiles(root);
execFileSync('tar', ['-czf', archive, '-C', resolve(root, '..'), 'cem-studio-login-kit']);
console.log(`Packaged ${files.length + 1} files: ${archive}`);
