import { createHash, randomUUID } from 'node:crypto';
import { copyFile, lstat, mkdir, readFile, readdir, rename, unlink, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';

export const DEMO_INVENTORY = 'demo-deployment.json';
export const DEMO_FILE_LIMIT = 999;
export const DEMO_FILE_SIZE_LIMIT = 25 * 1024 * 1024;
const producer = 'cem-demo-site';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const missing = error => { if (error.code === 'ENOENT') return undefined; throw error; };

function safePath(path) {
    if (typeof path !== 'string' || /[\\:\x00-\x1f\x7f]/u.test(path)
        || path.split('/').some(part => !part || part === '.' || part === '..') || path === DEMO_INVENTORY)
        throw new Error(`Invalid managed deployment path: ${path}`);
    return path;
}

export async function listDemoFiles(root, path = '') {
    const entries = await readdir(join(root, path), { withFileTypes: true }).catch(missing);
    const files = [];
    for (const entry of entries ?? []) {
        const relative = path ? `${path}/${entry.name}` : entry.name;
        if (entry.isDirectory()) files.push(...await listDemoFiles(root, relative));
        else files.push(relative); // Count unrelated symlinks but never traverse them.
    }
    return files.sort();
}

export async function readDemoInventory(root) {
    const info = await lstat(join(root, DEMO_INVENTORY)).catch(missing);
    if (!info) return undefined;
    if (!info.isFile()) throw new Error('Demo deployment inventory must be a regular file.');
    const bytes = await readFile(join(root, DEMO_INVENTORY)).catch(missing);
    if (!bytes) return undefined;
    if (bytes.length > 2 * 1024 * 1024) throw new Error('Demo deployment inventory is too large.');
    const inventory = JSON.parse(bytes);
    if (inventory.schemaVersion !== 1 || inventory.producer !== producer || !Array.isArray(inventory.files)
        || inventory.files.length > 10000 || inventory.fileCount !== inventory.files.length + 1)
        throw new Error('Invalid demo deployment inventory.');
    const paths = new Set();
    for (const entry of inventory.files) {
        safePath(entry.path);
        if (paths.has(entry.path) || !Number.isSafeInteger(entry.bytes) || entry.bytes < 0
            || !/^[a-f0-9]{64}$/u.test(entry.sha256)) throw new Error('Invalid demo deployment inventory entry.');
        paths.add(entry.path);
    }
    return inventory;
}

/** Publish explicit build outputs; only matching previously owned bytes may be retired. */
export async function publishDemoDeployment({ destination, entries, legacyEntries = [], fileLimit = DEMO_FILE_LIMIT, fileSizeLimit = DEMO_FILE_SIZE_LIMIT }) {
    destination = resolve(destination);
    if (!Number.isSafeInteger(fileLimit) || fileLimit < 1) throw new Error('Invalid demo file limit.');
    if (!Number.isSafeInteger(fileSizeLimit) || fileSizeLimit < 1) throw new Error('Invalid demo file size limit.');
    const checkSize = (path, size) => {
        if (size >= fileSizeLimit) throw new Error(`Demo file ${path} has ${size} bytes (must be below ${fileSizeLimit}). Destination was not updated.`);
    };
    const checked = new Set();
    async function checkPath(path) {
        if (checked.has(path)) return;
        const parent = dirname(path);
        if (parent !== path) await checkPath(parent);
        const info = await lstat(path).catch(missing);
        if (info?.isSymbolicLink()) throw new Error(`Destination contains a symbolic link: ${path}`);
        checked.add(path);
    }
    async function checkFile(path) {
        await checkPath(join(destination, path));
        const info = await lstat(join(destination, path)).catch(missing);
        if (info && !info.isFile()) throw new Error(`Expected a regular destination file: ${path}`);
    }
    await checkFile(DEMO_INVENTORY);
    const previous = await readDemoInventory(destination);
    const paths = new Set();
    const records = [];
    async function record(entry) {
        safePath(entry.path);
        if ((entry.source === undefined) === (entry.content === undefined)) throw new Error(`Expected one deployment source: ${entry.path}`);
        if (entry.source !== undefined && !(await lstat(entry.source)).isFile()) throw new Error(`Expected a regular source file: ${entry.source}`);
        const bytes = entry.source !== undefined ? await readFile(entry.source) : Buffer.from(entry.content);
        return { path: entry.path, bytes: bytes.length, sha256: digest(bytes) };
    }
    for (const entry of entries) {
        if (paths.has(entry.path)) throw new Error(`Duplicate deployment path: ${entry.path}`);
        paths.add(entry.path); records.push(await record(entry)); await checkFile(entry.path);
        checkSize(entry.path, records.at(-1).bytes);
    }
    const oldRecords = previous?.files ?? await Promise.all(legacyEntries.map(record));
    const ownership = new Map();
    for (const entry of oldRecords) {
        if (paths.has(entry.path)) continue;
        await checkFile(entry.path);
        const hashes = ownership.get(entry.path) ?? new Set(); hashes.add(entry.sha256); ownership.set(entry.path, hashes);
    }
    const removals = [], preserved = [];
    for (const [path, hashes] of ownership) {
        const bytes = await readFile(join(destination, path)).catch(missing);
        if (!bytes) continue;
        const sha256 = digest(bytes);
        if (hashes.has(sha256)) removals.push({ path, sha256 });
        else preserved.push(path);
    }
    const projected = new Set(await listDemoFiles(destination));
    for (const entry of removals) projected.delete(entry.path);
    for (const path of paths) projected.add(path);
    projected.add(DEMO_INVENTORY);
    if (projected.size > fileLimit) throw new Error(`Demo would contain ${projected.size} files (limit ${fileLimit}); unrelated and modified files were preserved. Destination was not updated.`);
    for (const path of projected) {
        if (paths.has(path) || path === DEMO_INVENTORY) continue;
        const info = await lstat(join(destination, path));
        if (info.isFile()) checkSize(path, info.size);
    }
    records.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
    const inventory = { schemaVersion: 1, producer, fileCount: records.length + 1, files: records };
    const content = `${JSON.stringify(inventory, null, 2)}\n`;
    checkSize(DEMO_INVENTORY, Buffer.byteLength(content));

    await mkdir(destination, { recursive: true });
    for (const entry of entries) {
        const target = join(destination, entry.path);
        await mkdir(dirname(target), { recursive: true });
        if (entry.source !== undefined) await copyFile(entry.source, target);
        else await writeFile(target, entry.content);
    }
    for (const entry of records) {
        if (digest(await readFile(join(destination, entry.path))) !== entry.sha256)
            throw new Error(`Deployment source changed while publishing: ${entry.path}`);
    }
    for (const entry of removals) {
        const bytes = await readFile(join(destination, entry.path)).catch(missing);
        if (!bytes) continue;
        if (digest(bytes) !== entry.sha256) throw new Error(`Obsolete deployment file changed while publishing: ${entry.path}`);
        await unlink(join(destination, entry.path));
    }
    const totalFileCount = new Set([...(await listDemoFiles(destination)), DEMO_INVENTORY]).size;
    if (totalFileCount > fileLimit) throw new Error('Demo file count changed during publication.');
    const temporary = join(destination, `.demo-deployment-${randomUUID()}.tmp`);
    let created = false;
    try {
        await writeFile(temporary, content, { flag: 'wx' }); created = true;
        await rename(temporary, join(destination, DEMO_INVENTORY)); created = false;
    } finally { if (created) await unlink(temporary).catch(missing); }
    return { inventory, totalFileCount, removed: removals.map(entry => entry.path), preserved,
        bytes: records.reduce((sum, entry) => sum + entry.bytes, Buffer.byteLength(content)) };
}
