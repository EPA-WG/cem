import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, resolve, sep } from 'node:path';

export const workspaceRoot = resolve(import.meta.dirname, '../../..');
export const presenterPrivacy = Object.freeze({ enteredCredentials: 'block-until-explicitly-cleared',
    automaticRecovery: false, aggregateCheckpointBytes: 4 * 1024 * 1024, handoffTimeoutMs: 5000,
    inputs: 'checked-in-project-only', browserStorage: 'never-read' });
export const presenterHash = bytes => createHash('sha256').update(bytes).digest('hex');
export const presenterJson = value => `${JSON.stringify(value, null, 2)}\n`;
export const presenterText = bytes => new TextDecoder('utf-8', { fatal: true }).decode(bytes);
export function presenterFile(root, name) {
    const path = resolve(root, name);
    assert.ok(path.startsWith(`${root}${sep}`), `Presenter path escapes its directory: ${name}`);
    return path;
}
export function presenterCommand(bundle, entry) {
    const resource = bundle.project.resources.find(item => item.id === entry.runConfigResourceId);
    const config = JSON.parse(presenterText(bundle.contents[resource.id]));
    return config.argv ?? ['validate', '--config', resource.path, '--format', 'json', '--no-color'];
}
/** Materialize original project bytes only; native import/execution owns all semantics. */
export async function withPresenterProject(bundle, prefix, run) {
    const directory = await mkdtemp(resolve(tmpdir(), prefix));
    try {
        for (const resource of bundle.project.resources) {
            const path = presenterFile(directory, resource.path);
            await mkdir(dirname(path), { recursive: true });
            await writeFile(path, bundle.contents[resource.id]);
        }
        return await run(directory);
    } finally { await rm(directory, { recursive: true, force: true }); }
}
