import { createHash } from 'node:crypto';
import { readFile, readdir } from 'node:fs/promises';
import { resolve } from 'node:path';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');

/** Build identity only: all input bytes were emitted by the native graph. */
export async function computeStudioDeploymentRevision(root) {
    async function filesAt(prefix = '') {
        const files = [];
        const entries = await readdir(resolve(root, prefix), { withFileTypes: true });
        entries.sort((a, b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
        for (const entry of entries) {
            const path = prefix ? `${prefix}/${entry.name}` : entry.name;
            if (entry.isDirectory()) files.push(...await filesAt(path));
            else {
                if (!entry.isFile()) throw Error(`Unsupported deployment entry: ${path}`);
                const bytes = await readFile(resolve(root, path));
                files.push({ path, bytes: bytes.length, sha256: hash(bytes) });
            }
        }
        return files;
    }
    const files = await filesAt();
    return { schemaVersion: 1, revision: hash(JSON.stringify(files)), files };
}

/** A bounded generated build input; the native graph emits the module. */
export function deploymentRevisionModule(revision) {
    if (revision !== '' && !/^[a-f0-9]{64}$/.test(revision)) throw Error('Invalid deployment revision');
    return `export const deploymentRevision = '${revision}';\n`;
}
