#!/usr/bin/env node

import { createServer } from 'node:http';
import { readFile, realpath, stat } from 'node:fs/promises';
import { homedir } from 'node:os';
import { extname, join, resolve, sep } from 'node:path';
import { pathToFileURL } from 'node:url';

const defaultRoot = join(homedir(), 'cem-bin');
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
        const candidate = resolve(root, `.${pathname}`);
        if (!within(root, candidate)) { response.writeHead(403).end('Forbidden'); return; }
        try {
            const info = await stat(candidate);
            const filename = info.isDirectory() ? join(candidate, 'index.html') : candidate;
            if (!within(await realpath(root), await realpath(filename))) {
                response.writeHead(403).end('Forbidden'); return;
            }
            if (info.isDirectory() && !url.pathname.endsWith('/')) {
                response.writeHead(308, { location: `${url.pathname}/${url.search}` }).end(); return;
            }
            const bytes = await readFile(filename);
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
        server.listen(port, '127.0.0.1', () => console.log(`CEM demo site: http://localhost:${port}/\nCEM Studio: http://localhost:${port}/studio/\nServing ${root}`));
    }
}
