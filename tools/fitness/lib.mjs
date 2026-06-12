// Shared helpers for the evolutionary fitness-function checks (FF-5, FF-6).
// See docs/fitness-functions.md. Plain Node ESM, no dependencies.
import { readdir, readFile } from 'node:fs/promises';
import { join, sep } from 'node:path';

/** Posix-normalize a path so registry patterns are OS-independent. */
export function toPosix(p) {
    return p.split(sep).join('/');
}

/** Recursively collect files under rootDir matching `extensions`, skipping `ignoreDirs` by name. */
export async function walkFiles(rootDir, { extensions, ignoreDirs }) {
    const ignore = new Set(ignoreDirs);
    const out = [];
    async function walk(dir) {
        let entries;
        try {
            entries = await readdir(dir, { withFileTypes: true });
        } catch {
            return; // missing root is not an error; the registry may list optional roots
        }
        for (const entry of entries) {
            const full = join(dir, entry.name);
            if (entry.isDirectory()) {
                if (!ignore.has(entry.name)) await walk(full);
            } else if (entry.isFile() && extensions.some((ext) => entry.name.endsWith(ext))) {
                out.push(full);
            }
        }
    }
    await walk(rootDir);
    return out;
}

/** Match a posix relPath against a registry allowlist/ignore pattern (exact, `dir/**`, `prefix*`, or substring). */
export function pathMatches(relPath, pattern) {
    const p = toPosix(relPath);
    if (pattern.endsWith('/**')) {
        const base = pattern.slice(0, -3);
        return p === base || p.startsWith(base + '/');
    }
    if (pattern.endsWith('*')) return p.startsWith(pattern.slice(0, -1));
    return p === pattern || p.includes(pattern);
}

/** Return `{ file, line }` hits where `pattern` (substring) occurs in the file. */
export async function findSubstringHits(absPath, relPath, pattern) {
    const content = await readFile(absPath, 'utf8');
    if (!content.includes(pattern)) return [];
    const rel = toPosix(relPath);
    const hits = [];
    const lines = content.split(/\r?\n/);
    for (let i = 0; i < lines.length; i++) {
        if (lines[i].includes(pattern)) hits.push({ file: rel, line: i + 1 });
    }
    return hits;
}

/** Practical SemVer 2.0 check (MAJOR.MINOR.PATCH with optional prerelease/build). */
export function isSemver(value) {
    return (
        typeof value === 'string' &&
        /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(value.trim())
    );
}

/**
 * Resolve a version string from a registry locator.
 * `{ type: "json", file, path }`  — JSON.parse + dot-path.
 * `{ type: "regex", file, pattern, flags }` — capture group 1 (or whole match).
 * Returns `{ found, value, error }`.
 */
export async function readVersionFromLocator(workspaceRoot, locator) {
    const abs = join(workspaceRoot, locator.file);
    let content;
    try {
        content = await readFile(abs, 'utf8');
    } catch {
        return { found: false, value: null, error: `file not found: ${locator.file}` };
    }
    if (locator.type === 'json') {
        let data;
        try {
            data = JSON.parse(content);
        } catch {
            return { found: false, value: null, error: `invalid JSON: ${locator.file}` };
        }
        const value = locator.path.split('.').reduce((o, k) => (o == null ? undefined : o[k]), data);
        if (value === undefined) {
            return { found: false, value: null, error: `path '${locator.path}' absent in ${locator.file}` };
        }
        return { found: true, value: String(value), error: null };
    }
    if (locator.type === 'regex') {
        const match = new RegExp(locator.pattern, locator.flags ?? '').exec(content);
        if (!match) return { found: false, value: null, error: `pattern not found in ${locator.file}` };
        return { found: true, value: match[1] ?? match[0], error: null };
    }
    return { found: false, value: null, error: `unknown locator type: ${locator.type}` };
}
