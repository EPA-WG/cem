#!/usr/bin/env node

import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { readFile, realpath, stat } from 'node:fs/promises';
import { homedir } from 'node:os';
import { extname, join, resolve, sep } from 'node:path';
import { pathToFileURL } from 'node:url';
import { readDemoInventory } from './demo-deployment.mjs';

const defaultRoot = join(homedir(), 'cem-bin');
const studioPreviewPrefix = '/studio/__cem_preview__/';
const contentTypes = {
    '.html': 'text/html; charset=utf-8', '.xhtml': 'application/xhtml+xml; charset=utf-8',
    '.js': 'application/javascript; charset=utf-8', '.mjs': 'application/javascript; charset=utf-8',
    '.css': 'text/css; charset=utf-8', '.json': 'application/json; charset=utf-8',
    '.wasm': 'application/wasm', '.webmanifest': 'application/manifest+json',
    '.svg': 'image/svg+xml', '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg',
    '.webp': 'image/webp', '.gif': 'image/gif', '.ico': 'image/x-icon',
    '.woff': 'font/woff', '.woff2': 'font/woff2', '.xml': 'application/xml; charset=utf-8',
    '.xsl': 'application/xml; charset=utf-8', '.md': 'text/markdown; charset=utf-8',
    '.cem': 'application/cem+xml; charset=utf-8', '.cemt': 'application/vnd.cem.template+cem; charset=utf-8',
    '.csv': 'text/csv; charset=utf-8', '.yaml': 'application/yaml; charset=utf-8', '.yml': 'application/yaml; charset=utf-8',
};
const within = (root, candidate) => candidate === root || candidate.startsWith(`${root}${sep}`);

/** Serve the assembled deployment; Studio keeps its own headers and worker scope. */
export function createDemoSiteHandler({ root = defaultRoot } = {}) {
    root = resolve(root);
    return async (request, response) => {
        if (!['GET', 'HEAD'].includes(request.method)) {
            response.writeHead(405, { allow: 'GET, HEAD' }).end('Method not allowed');
            return;
        }
        let url, pathname;
        try {
            url = new URL(request.url, 'http://localhost');
            pathname = decodeURIComponent(url.pathname);
        } catch {
            response.writeHead(400).end('Invalid URL');
            return;
        }
        const isStudioPreview = pathname.startsWith(studioPreviewPrefix);
        if (isStudioPreview) {
            const asset = pathname.match(/^\/studio\/__cem_preview__\/[a-f0-9]{16}\/(.*)$/)?.[1];
            if (asset === undefined) { response.writeHead(404).end('Not found'); return; }
            pathname = `/studio/${asset}`;
        }
        const candidate = resolve(root, `.${pathname}`);
        if (!within(root, candidate) || isStudioPreview && !within(join(root, 'studio'), candidate)) {
            response.writeHead(403).end('Forbidden'); return;
        }
        try {
            const info = await stat(candidate);
            const filename = info.isDirectory() ? join(candidate, 'index.html') : candidate;
            if (!within(await realpath(root), await realpath(filename))) {
                response.writeHead(403).end('Forbidden'); return;
            }
            if (isStudioPreview && !within(await realpath(join(root, 'studio')), await realpath(filename))) {
                response.writeHead(403).end('Forbidden'); return;
            }
            if (info.isDirectory() && !url.pathname.endsWith('/')) {
                response.writeHead(308, { location: `${url.pathname}/${url.search}` }).end(); return;
            }
            let bytes = await readFile(filename);
            if (filename === join(root, 'studio/index.html') && !isStudioPreview) {
                const base = await studioPreviewBase(root);
                response.writeHead(307, { location: `${base}${url.search}`, 'cache-control': 'no-store' }).end(); return;
            }
            if (filename === join(root, 'studio/index.html')) {
                const base = url.pathname.slice(0, url.pathname.lastIndexOf('/') + 1);
                bytes = Buffer.from(bytes.toString().replace(/<base href="\.\/" data-cem-studio-scope\s*\/?>/,
                    `<base href="${base}" data-cem-studio-scope>`));
            } else if (filename === join(root, 'index.html')) {
                const base = await studioPreviewBase(root);
                bytes = Buffer.from(bytes.toString().replace(/href="\.\/studio\/"/g, `href="${base}"`));
            }
            const security = pathname.startsWith('/studio/')
                ? JSON.parse(await readFile(join(root, 'studio/security-headers.json'), 'utf8')).headers : {};
            response.writeHead(200, { ...security, 'cache-control': 'no-store',
                'content-type': contentTypes[extname(filename)] ?? 'application/octet-stream',
                'content-length': bytes.length });
            response.end(request.method === 'HEAD' ? undefined : bytes);
        } catch (error) {
            response.writeHead(['ENOENT', 'ENOTDIR'].includes(error.code) ? 404 : 500).end('Not found');
        }
    };
}

async function studioPreviewBase(root) {
    const inventory = await readDemoInventory(root);
    const files = inventory?.files.filter(file => file.path.startsWith('studio/'));
    if (!files?.length) throw new Error('Demo deployment inventory is missing Studio files. Run yarn copy:demo.');
    const revision = createHash('sha256').update(JSON.stringify(files.sort((a, b) => a.path.localeCompare(b.path)))).digest('hex').slice(0, 16);
    return `${studioPreviewPrefix}${revision}/`;
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
    const args = process.argv.slice(2);
    if (args.length === 1 && ['--help', '-h'].includes(args[0])) {
        console.log('Usage: yarn serve:demo [destination]\nServe the copy:demo output (default ~/cem-bin) at http://localhost:8081/. Set PORT to choose another port.');
    } else if (args.length > 1 || args.some(arg => arg.startsWith('-'))) {
        console.error('Usage: yarn serve:demo [destination]'); process.exitCode = 1;
    } else {
        const root = resolve(args[0] ?? defaultRoot);
        await stat(join(root, 'index.html'));
        await stat(join(root, 'studio/index.html'));
        const port = Number(process.env.PORT ?? '8081');
        if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error('PORT must be between 1 and 65535.');
        const server = createServer(createDemoSiteHandler({ root }));
        const studio = await studioPreviewBase(root);
        server.listen(port, '127.0.0.1', () => console.log(`CEM demo site: http://localhost:${port}/\nCEM Studio: http://localhost:${port}${studio}\nServing ${root}`));
    }
}
