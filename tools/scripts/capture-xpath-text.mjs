const helpString = `
Capture text content from elements matching a given XPath across local HTML files.

Usage: node tools/scripts/capture-xpath-text.mjs <path-mask> <xpath> <output-mask>

Mask examples:
  node tools/scripts/capture-xpath-text.mjs "packages/cem-theme/src/lib/css-generators/*.html" "//code[@data-generated-css]" "tmp/*.css"
  node tools/scripts/capture-xpath-text.mjs "packages/**/colors.html" "//*[@code='language-css']" "dist/lib/css/*-generated.css"

Output mask rules:
  - Use * to substitute the input file name without extension.
  - If multiple input files match, output mask must include * to avoid overwriting.
`;

import fs from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import http from "node:http";

function hasGlobMagic(mask) {
    return /[*?[\]{}()!]/.test(mask);
}

async function expandInputFiles(mask) {
    if (!hasGlobMagic(mask)) {
        const candidatePath = path.resolve(mask);
        if (!existsSync(candidatePath)) {
            throw new Error(`Input file not found: ${candidatePath}`);
        }
        return [candidatePath];
    }

    const matches = await fs.glob(mask, { withFileTypes: false });
    if (matches.length === 0) {
        throw new Error(`No input files matched mask: ${mask}`);
    }
    const files = await Array.fromAsync(fs.glob(mask, { withFileTypes: false }));

    return files;// matches.map((match) => path.resolve(match));
}

function resolveOutputPath(mask, inputPath) {
    if (!mask.includes("*")) {
        return path.resolve(mask);
    }
    const name = path.parse(inputPath).name;
    return path.resolve(mask.split("*").join(name));
}

const MIME_TYPES = {
    '.html': 'text/html',
    '.xhtml': 'application/xhtml+xml',
    '.js': 'application/javascript',
    '.mjs': 'application/javascript',
    '.css': 'text/css',
    '.json': 'application/json',
    '.xml': 'application/xml',
    '.svg': 'image/svg+xml',
    '.png': 'image/png',
    '.jpg': 'image/jpeg',
    '.gif': 'image/gif',
};

/** Start a static file server rooted at `docRoot`, returns { url, close } */
function startStaticServer(docRoot) {
    return new Promise((resolve, reject) => {
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
        server.listen(0, '127.0.0.1', () => {
            const { port } = server.address();
            resolve({
                url: `http://127.0.0.1:${port}`,
                close: () => new Promise(r => server.close(r)),
            });
        });
        server.on('error', reject);
    });
}

async function main(urlMask, xpath, outputMask) {
    if (!urlMask || !xpath || !outputMask) {
        console.error('Usage: node tools/scripts/capture-xpath-text.mjs <path-mask> <xpath> <output-mask>', helpString);
        process.exit(1);
    }

    const inputFiles = await expandInputFiles(urlMask);
    if (inputFiles.length > 1 && !outputMask.includes("*")) {
        throw new Error("Output mask must include * when multiple input files match.");
    }

    let chromium;
    try {
        ({ chromium } = await import("playwright"));
    } catch (error) {
        console.error(
            "Playwright is required. Install with: yarn add -D playwright && npx playwright install chromium",
        );
        throw error;
    }

    // Serve files over HTTP so fetch() works (file:// protocol blocks Fetch API).
    // Serve from filesystem root so all relative paths in HTML resolve correctly.
    const docRoot = path.parse(process.cwd()).root;
    const server = await startStaticServer(docRoot);

    let browser;
    try {
        browser = await chromium.launch({
            headless: true,
        });
        const context = await browser.newContext({
            javaScriptEnabled: true,
            bypassCSP: true,
        });
        const page = await context.newPage();

        for (const inputFile of inputFiles) {
            const relativePath = path.relative(docRoot, inputFile);
            const targetUrl = `${server.url}/${relativePath}`;
            await page.goto(targetUrl, { waitUntil: "networkidle" });
            const locator = page.locator(`xpath=${xpath}`);
            await locator
                .first()
                .waitFor({ state: "attached", timeout: 10000 })
                .catch(() => {});
            const count = await locator.count();
            if (count === 0) {
                throw new Error(`No elements matched XPath: ${xpath} for ${inputFile}`);
            }

            const texts = await locator.allTextContents();
            const outputPath = resolveOutputPath(outputMask, inputFile);
            const outputDir = path.dirname(outputPath);
            await fs.mkdir(outputDir, { recursive: true });
            await fs.writeFile(outputPath, texts.join("\n"), "utf8");
            console.log(`${inputFile} ➤ ${outputPath}`);
        }
    } finally {
        if (browser) {
            await browser.close();
        }
        await server.close();
    }
}

const [urlMask, xpath, outputMask] = process.argv.slice(2);

main(urlMask, xpath, outputMask).catch((error) => {
    console.error(error);
    process.exit(1);
});
