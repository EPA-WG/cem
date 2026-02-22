/**
 * Minimal static file server for local development and debugging.
 *
 * Usage:
 *   node tools/scripts/dev-server.mjs [path-to-open]
 *
 * Examples:
 *   node tools/scripts/dev-server.mjs packages/cem-theme/dist/lib/tokens/index.xhtml
 *   node tools/scripts/dev-server.mjs packages/cem-theme/src/lib/css-generators/cem-colors.html
 *
 * The server serves from the filesystem root so all relative paths in HTML resolve correctly
 * (same approach as capture-xpath-text.mjs).
 */
import fs from "node:fs/promises";
import path from "node:path";
import http from "node:http";
import { exec } from "node:child_process";

const MIME_TYPES = {
    '.html':  'text/html',
    '.xhtml': 'application/xhtml+xml',
    '.js':    'application/javascript',
    '.mjs':   'application/javascript',
    '.css':   'text/css',
    '.json':  'application/json',
    '.xml':   'application/xml',
    '.svg':   'image/svg+xml',
    '.png':   'image/png',
    '.jpg':   'image/jpeg',
    '.gif':   'image/gif',
    '.ico':   'image/x-icon',
    '.woff':  'font/woff',
    '.woff2': 'font/woff2',
};

const PORT = parseInt(process.env.PORT, 10) || 3000;
const docRoot = path.parse(process.cwd()).root;

const server = http.createServer(async (req, res) => {
    const filePath = path.join(docRoot, decodeURIComponent(req.url.split('?')[0]));
    try {
        const data = await fs.readFile(filePath);
        const ext = path.extname(filePath).toLowerCase();
        res.writeHead(200, { 'Content-Type': MIME_TYPES[ext] || 'application/octet-stream' });
        res.end(data);
    } catch {
        res.writeHead(404);
        res.end('Not found');
    }
});

server.listen(PORT, '127.0.0.1', () => {
    const openPath = process.argv[2] || 'packages/cem-theme/dist/lib/tokens/index.xhtml';
    const absPath = path.resolve(openPath);
    const relativePath = path.relative(docRoot, absPath);
    const url = `http://127.0.0.1:${PORT}/${relativePath}`;

    console.log(`Serving from: ${docRoot}`);
    console.log(`Open: ${url}`);
    console.log('Press Ctrl+C to stop.\n');

    // Open browser (best-effort, works on Linux/WSL/macOS)
    const commands = [
        `xdg-open "${url}"`,              // Linux
        `cmd.exe /c start "${url}"`,       // WSL → Windows browser
        `open "${url}"`,                   // macOS
    ];
    for (const cmd of commands) {
        try { exec(cmd); break; } catch { /* try next */ }
    }
});
