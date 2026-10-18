import { createHash } from 'node:crypto';
import { readFile, stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { extname, resolve, sep } from 'node:path';
import { pathToFileURL } from 'node:url';

const projectRoot = resolve(import.meta.dirname, '..');
const prefix = '/__cem_preview__/';

/** Keep browser data on its original origin; only deployment asset URLs change. */
export function createCemStudioPreviewHandler({ root = resolve(projectRoot, 'dist/static'), buildReport = resolve(projectRoot, '../../dist/reports/cem-studio/build.json') } = {}) {
    return async (request, response) => {
        try {
            const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
            const asset = pathname.startsWith(prefix) ? pathname.match(/^\/__cem_preview__\/[a-f0-9]{16}\/(.*)$/)?.[1] : pathname.slice(1);
            if (asset === undefined) { response.writeHead(404).end('Not found'); return; }
            const candidate = resolve(root, asset || 'index.html');
            if (!candidate.startsWith(`${resolve(root)}${sep}`)) { response.writeHead(403).end('Forbidden'); return; }
            const metadata = await stat(candidate);
            const filename = metadata.isDirectory() ? resolve(candidate, 'index.html') : candidate;
            let bytes = await readFile(filename);
            const security = JSON.parse(await readFile(resolve(root, 'security-headers.json'), 'utf8'));
            if (filename === resolve(root, 'index.html')) {
                const report = JSON.parse(await readFile(buildReport, 'utf8'));
                const manifest = findModuleAssetManifest(report);
                if (!manifest?.hash) throw new Error('Build report is missing the module asset fingerprint');
                const revision = createHash('sha256').update(bytes).update(manifest.hash).update(JSON.stringify(security)).digest('hex').slice(0, 16);
                // This changes URL resolution only. Studio markup is still the graph-emitted app.
                bytes = Buffer.from(bytes.toString().replace(/<base href="\.\/" data-cem-studio-scope\s*\/?>/, `<base href="${prefix}${revision}/" data-cem-studio-scope>`));
            }
            response.writeHead(200, { ...security.headers, 'content-type': contentType(filename), 'cache-control': 'no-store', 'service-worker-allowed': '/' });
            response.end(bytes);
        } catch (error) {
            response.writeHead(error.code === 'ENOENT' ? 404 : 503, { 'cache-control': 'no-store' }).end('Studio preview is unavailable. Finish the Studio build and reload.');
        }
    };
}

function findModuleAssetManifest(value) {
    if (!value || typeof value !== 'object') return undefined;
    if (value.hashScheme === 'sha256' && Array.isArray(value.assets)) return value;
    for (const child of Object.values(value)) { const result = findModuleAssetManifest(child); if (result) return result; }
}
function contentType(filename) {
    return { '.html': 'text/html; charset=utf-8', '.xhtml': 'application/xhtml+xml', '.js': 'text/javascript', '.json': 'application/json', '.css': 'text/css', '.wasm': 'application/wasm', '.svg': 'image/svg+xml', '.webmanifest': 'application/manifest+json' }[extname(filename)] ?? 'application/octet-stream';
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
    const port = Number(process.env.CEM_STUDIO_PREVIEW_PORT ?? '8080');
    const server = createServer(createCemStudioPreviewHandler());
    server.listen(port, '127.0.0.1', () => console.log(`CEM Studio preview: http://localhost:${port}/`));
}
