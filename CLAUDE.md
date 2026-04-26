# Claude Code Instructions

## Project overview

CEM (Consumer-Experience Model) is a semantic design token framework using `@epa-wg/custom-element` for declarative web
components. No shadow DOM is used -- all content renders in the light DOM.

### Key paths

| Purpose                         | Path                                               |
|---------------------------------|----------------------------------------------------|
| Token specs (markdown)          | `packages/cem-theme/src/lib/tokens/*.md`           |
| Token specs (built XHTML)       | `packages/cem-theme/dist/lib/tokens/*.xhtml`       |
| CSS generators (HTML templates) | `packages/cem-theme/src/lib/css-generators/*.html` |
| Generated CSS                   | `packages/cem-theme/dist/lib/css/*.css`            |

### Build

```bash
yarn build          # build everything
yarn build:css      # generate CSS only
yarn build:theme    # build theme package
```

## Dev server

```bash
yarn start                                                                      # opens dist/lib/tokens/index.xhtml
yarn start packages/cem-theme/src/lib/css-generators/cem-colors.html            # opens a specific file
PORT=8080 yarn start packages/cem-theme/dist/lib/tokens/cem-colors.xhtml        # custom port
```

The server serves from filesystem root so all relative paths in HTML resolve correctly. Files must be served over
HTTP -- `file://` protocol breaks `fetch()` / `<http-request>` in the custom-element templates.

## Debugging DOM and CSS with headless browser

Use Playwright (already a project dependency) to inspect the live DOM and computed CSS. This is the same Chromium used
by the build pipeline (`capture-xpath-text.mjs`).

### Inline one-shot script

Run a quick inspection directly from the command line:

```bash
node -e "
const fs = require('fs/promises'), path = require('path'), http = require('http');
const docRoot = path.parse(process.cwd()).root;
const srv = http.createServer(async (q,r) => {
  try { const d = await fs.readFile(path.join(docRoot, decodeURIComponent(q.url.split('?')[0])));
    r.writeHead(200); r.end(d); } catch { r.writeHead(404); r.end(); }
});
srv.listen(0, '127.0.0.1', async () => {
  const { chromium } = await import('playwright');
  const port = srv.address().port;
  const url = \`http://127.0.0.1:\${port}/${path.relative(docRoot, path.resolve('packages/cem-theme/src/lib/css-generators/cem-colors.html'))}\`;
  const browser = await chromium.launch({ headless: true });
  const page = await (await browser.newContext({ javaScriptEnabled: true, bypassCSP: true })).newPage();
  await page.goto(url, { waitUntil: 'networkidle' });
  await page.waitForTimeout(2000);

  // ---- EDIT THIS SECTION TO INSPECT WHATEVER YOU NEED ----
  const result = await page.evaluate(() => {
    const root = document.documentElement;
    const info = {};

    // Example: check a CSS variable on :root
    info.rootComfort = getComputedStyle(root).getPropertyValue('--cem-palette-comfort') || '(empty)';

    // Example: find elements by selector and read computed styles
    const tables = document.querySelectorAll('table.cem-theme-dark');
    info.darkTables = Array.from(tables).map(t => ({
      computedBg:          getComputedStyle(t).backgroundColor,
      computedColorScheme: getComputedStyle(t).colorScheme,
      varComfort:          getComputedStyle(t).getPropertyValue('--cem-palette-comfort'),
    }));

    // Example: check injected style elements
    info.headStyles = Array.from(document.head.querySelectorAll('style')).map(s => ({
      id: s.id, length: s.textContent.length,
    }));

    return info;
  });

  console.log(JSON.stringify(result, null, 2));
  await browser.close(); srv.close();
});
" 2>&1
```

### What to inspect

Common `page.evaluate()` patterns:

```js
// CSS variable on any element
getComputedStyle( el ).getPropertyValue( '--cem-palette-comfort' )

// Computed property
getComputedStyle( el ).backgroundColor   // resolved color e.g. "rgb(0, 16, 16)"
getComputedStyle( el ).colorScheme       // "light", "dark", or "normal"

// Check if cem-css-loader injected its style
document.querySelectorAll( 'style[data-cem-css-loader]' )

// Search for a CSS variable definition in the injected stylesheet
document.querySelector( 'style[data-cem-css-loader]' )?.textContent.includes( '--cem-palette-comfort' )

// All tables with a specific inline style
document.querySelectorAll( 'table[style*="--cem-palette-comfort"]' )

// DOM tree context of an element
el.parentElement.tagName
el.getRootNode().constructor.name   // "HTMLDocument" = light DOM, "ShadowRoot" = shadow DOM
el.closest( '.cem-theme-dark' )       // find ancestor with class
```

### Writing a standalone debug script

For more complex investigations, create a temporary `.mjs` file:

```bash
cat > /tmp/debug-cem.mjs << 'SCRIPT'
import fs from "node:fs/promises";
import path from "node:path";
import http from "node:http";

const docRoot = path.parse(process.cwd()).root;
const server = http.createServer(async (req, res) => {
    try {
        const data = await fs.readFile(path.join(docRoot, decodeURIComponent(req.url.split('?')[0])));
        res.writeHead(200); res.end(data);
    } catch { res.writeHead(404); res.end(); }
});

server.listen(0, '127.0.0.1', async () => {
    const { chromium } = await import("playwright");
    const port = server.address().port;
    const file = process.argv[2] || "packages/cem-theme/src/lib/css-generators/cem-colors.html";
    const url = `http://127.0.0.1:${port}/${path.relative(docRoot, path.resolve(file))}`;

    const browser = await chromium.launch({ headless: true });
    const page = await (await browser.newContext({ javaScriptEnabled: true, bypassCSP: true })).newPage();
    page.on('console', msg => console.error(`[browser ${msg.type()}]`, msg.text()));
    page.on('pageerror', err => console.error(`[page error]`, err.message));

    await page.goto(url, { waitUntil: "networkidle" });
    await page.waitForTimeout(2000);

    // --- your inspection logic here ---
    const result = await page.evaluate(() => {
        return {}; // fill in
    });

    console.log(JSON.stringify(result, null, 2));
    await browser.close();
    server.close();
});
SCRIPT
node /tmp/debug-cem.mjs
```
