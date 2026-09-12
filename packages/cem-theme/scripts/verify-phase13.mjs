/**
 * Phase 13 cross-phase verifier.
 *
 * Runs the checks that can be made deterministic from built CEM theme output:
 * - manifest coverage and CSS parse validity for each canonical generator
 * - browser rendering smoke for every generator page
 * - theme-mode custom-property resolution
 * - forced-colors and reduced-motion media fallbacks
 * - cross-spec semantic/numeric invariants
 * - adapter-only/deprecated token absence in default CSS output
 *
 * Usage:
 *   node scripts/verify-phase13.mjs
 */

import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { spawn } from "node:child_process";

const packageRoot = process.cwd();
const workspaceRoot = path.resolve(packageRoot, "../..");
const docRoot = path.parse(workspaceRoot).root;

const SPECS = [
    { name: "cem-colors", token: "--cem-palette-comfort" },
    { name: "cem-dimension", token: "--cem-dim-x-small" },
    { name: "cem-timing", token: "--cem-duration-noticeable" },
    { name: "cem-breakpoints", token: "--cem-bp-width-compact-max" },
    { name: "cem-coupling", token: "--cem-coupling-guard-min" },
    { name: "cem-controls", token: "--cem-control-height" },
    { name: "cem-shape", token: "--cem-bend-smooth" },
    { name: "cem-stroke", token: "--cem-stroke-focus" },
    { name: "cem-layering", token: "--cem-layer-work" },
    { name: "cem-voice-fonts-typography", token: "--cem-typography-reading-line-height" },
];

/**
 * Human-facing presentation contract for each Markdown-backed generator. A source table must
 * either appear in one of these projections or be named in `generatorOnly`; the latter is still
 * checked against generated CSS. Numeric projection entries are zero-based Markdown columns.
 */
const PRESENTATION_PROTOCOLS = {
    "cem-breakpoints": {
        generatorOnly: ["cem-bp-active", "cem-bp-media-ranges", "cem-bp-height-ranges"],
        tables: [
            {
                id: "cem-bp-basis",
                caption: "cem-bp-basis — width + epsilon basis tokens",
                headers: ["Token", "Value", "Tier", "Description"],
                sources: ["cem-bp-basis"],
                projection: [0, 1, 3, 2],
            },
            {
                id: "cem-bp-height",
                caption: "cem-bp-height — height basis tokens",
                headers: ["Token", "Value", "Tier", "Description"],
                sources: ["cem-bp-height"],
                projection: [0, 1, 3, 2],
            },
            {
                id: "cem-bp-cq",
                caption: "cem-bp-cq — container query reference values",
                headers: ["Token", "Value", "Tier", "Description"],
                sources: ["cem-bp-cq"],
                projection: [0, 1, 3, 2],
            },
        ],
    },
    "cem-controls": {
        generatorOnly: [],
        tables: [
            {
                id: "cem-controls-geometry",
                caption: "cem-controls-geometry — visual control geometry (balanced baseline)",
                headers: ["Token", "Value", "Tier", "Swatch", "Description"],
                sources: ["cem-controls-geometry"],
                projection: [0, 1, 3, "blank", 2],
                preview: "width",
            },
            {
                id: "cem-progress-geometry",
                caption: "cem-progress-geometry — non-interactive progress graphic geometry",
                headers: ["Token", "Value", "Tier", "Swatch", "Description"],
                sources: ["cem-progress-geometry"],
                projection: [0, 1, 3, "blank", 2],
                preview: "width",
            },
            {
                id: "cem-slider-geometry",
                caption: "cem-slider-geometry — slider visual geometry",
                headers: ["Token", "Value", "Tier", "Swatch", "Description"],
                sources: ["cem-slider-geometry"],
                projection: [0, 1, 3, "blank", 2],
                preview: "width",
            },
            {
                id: "cem-controls-geometry-overrides",
                caption: "cem-controls-geometry-overrides — coupling mode overrides (visual geometry only)",
                headers: ["Token", "forgiving", "compact"],
                sources: ["cem-controls-geometry-overrides"],
                projection: [0, 1, 2],
            },
        ],
    },
    "cem-coupling": {
        generatorOnly: [],
        tables: [
            {
                id: "cem-coupling-minimums",
                caption: "cem-coupling-minimums — hard safety minimums (mode-invariant)",
                headers: ["Token", "Value", "Tier", "Swatch", "Description"],
                sources: ["cem-coupling-minimums"],
                projection: [0, 1, 3, "blank", 2],
                preview: "width",
            },
            {
                id: "cem-coupling-halo-overrides",
                caption: "cem-coupling-halo-overrides — coupling mode halo overrides (visuals owned by D2c Controls)",
                headers: ["Token", "forgiving", "compact"],
                sources: ["cem-coupling-halo-overrides"],
                projection: [0, 1, 2],
            },
        ],
    },
    "cem-dimension": {
        generatorOnly: ["cem-dim-rhythm-reading", "cem-dim-rhythm-data"],
        tables: [
            {
                id: "cem-dim-scale",
                caption: "cem-dim-scale — dimension scale",
                headers: ["Token", "Value", "Description"],
                sources: ["cem-dim-scale"],
                projection: [0, "blank", "value-description"],
                preview: "height",
            },
            {
                id: "cem-dim-spacing-endpoints",
                caption: "cem-dim-gaps + cem-dim-insets — semantic spacing endpoints",
                headers: ["Token", "Value", "Swatch", "Description"],
                sources: ["cem-dim-gaps", "cem-dim-insets"],
                projection: [0, 1, "blank", 2],
                preview: "width",
                previewColumn: 2,
            },
            {
                id: "cem-dim-layout",
                caption: "cem-dim-layout — layout rhythm tokens",
                headers: ["Token", "Value", "Tier", "Description"],
                sources: ["cem-dim-layout"],
                projection: [0, 1, 3, 2],
            },
            {
                id: "cem-dim-spacing-overrides",
                caption: "cem-dim-spacing-overrides — density mode overrides",
                headers: ["Token", "dense", "sparse"],
                sources: ["cem-dim-spacing-overrides"],
                projection: [0, 1, 2],
            },
        ],
    },
    "cem-layering": {
        generatorOnly: [],
        tables: [
            {
                id: "cem-layering-rungs",
                caption: "cem-layering-rungs — signed-depth ladder (box-shadow recipes)",
                headers: ["Token", "Value", "Tier", "Sample", "Description"],
                sources: ["cem-layering-rungs"],
                projection: [0, 1, 3, "blank", 2],
                preview: "shadow",
            },
            {
                id: "cem-layering-semantic-table",
                caption: "cem-layering-semantic — required semantic endpoints",
                headers: ["Token", "Value", "Tier", "Description"],
                sources: ["cem-layering-semantic", "cem-layering-semantic-optional"],
                projection: [0, 1, 3, 2],
            },
            {
                id: "cem-layering-rungs-forced",
                caption: "cem-layering-rungs-forced — forced-colors fallback",
                headers: ["Token", "Forced-colors value"],
                sources: ["cem-layering-rungs-forced"],
                projection: [0, 1],
            },
        ],
    },
    "cem-shape": {
        generatorOnly: ["cem-shape-action-bindings"],
        tables: [
            {
                id: "cem-shape-basis",
                caption: "cem-shape-basis — bend basis tokens",
                headers: ["Token", "Value", "Tier", "Sample", "Description"],
                sources: ["cem-shape-basis"],
                projection: [0, 1, 3, "blank", 2],
                preview: "radius",
            },
            {
                id: "cem-shape-semantic",
                caption: "cem-shape-semantic — role-first semantic endpoints",
                headers: ["Token", "Value", "Tier", "Sample", "Description"],
                sources: ["cem-shape-semantic"],
                projection: [0, 1, 3, "blank", 2],
                preview: "radius",
            },
            {
                id: "cem-shape-pattern",
                caption: "cem-shape-pattern — asymmetric attachment patterns",
                headers: ["Token", "Value", "Tier", "Description"],
                sources: ["cem-shape-pattern"],
                projection: [0, 1, 3, 2],
            },
            {
                id: "cem-shape-mode-sharp",
                caption: "cem-shape-mode-sharp — `data-cem-shape=\"sharp\"` overrides",
                headers: ["Token", "Override value"],
                sources: ["cem-shape-mode-sharp"],
                projection: [0, 1],
            },
            {
                id: "cem-shape-mode-round",
                caption: "cem-shape-mode-round — `data-cem-shape=\"round\"` overrides",
                headers: ["Token", "Override value"],
                sources: ["cem-shape-mode-round"],
                projection: [0, 1],
            },
        ],
    },
    "cem-stroke": {
        generatorOnly: [],
        tables: [
            {
                id: "cem-stroke-basis",
                caption: "cem-stroke-basis — stroke basis",
                headers: ["Token", "Value", "Tier", "Sample", "Description"],
                sources: ["cem-stroke-basis"],
                projection: [0, 1, 3, "blank", 2],
                preview: "stroke",
            },
            {
                id: "cem-stroke-indicator-appearance",
                caption: "cem-stroke-indicator-appearance — outline/underline geometry selectors",
                headers: ["Token", "Value", "Tier", "Description"],
                sources: ["cem-stroke-indicator-appearance"],
                projection: [0, 1, 3, 2],
            },
            {
                id: "cem-stroke-semantic",
                caption: "cem-stroke-semantic — semantic stroke endpoints",
                headers: ["Token", "Value", "Tier", "Sample", "Description"],
                sources: ["cem-stroke-semantic"],
                projection: [0, 1, 3, "blank", 2],
                preview: "stroke",
            },
            {
                id: "cem-stroke-zebra-rings",
                caption: "cem-stroke-zebra-pattern + cem-stroke-rings — zebra-pattern geometry and ring recipes",
                headers: ["Token", "Value", "Tier", "Description"],
                sources: ["cem-stroke-zebra-pattern", "cem-stroke-rings"],
                projection: [0, 1, 3, 2],
            },
            {
                id: "cem-stroke-rings-forced",
                caption: "cem-stroke-rings-forced — forced-colors fallback",
                headers: ["Token", "Forced-colors value"],
                sources: ["cem-stroke-rings-forced"],
                projection: [0, 1],
            },
        ],
    },
    "cem-timing": {
        generatorOnly: [],
        tables: [
            {
                id: "cem-timing-durations",
                caption: "cem-timing-durations — duration scale",
                headers: ["Token", "Value", "Tier", "Demo", "Description"],
                sources: ["cem-timing-durations"],
                projection: [0, 1, 3, "blank", 2],
                preview: "duration",
            },
            {
                id: "cem-timing-easings",
                caption: "cem-timing-easings — easing curves",
                headers: ["Token", "Value", "Tier", "Demo", "Description"],
                sources: ["cem-timing-easings"],
                projection: [0, 1, 3, "blank", 2],
                preview: "easing",
            },
            {
                id: "cem-timing-reduced-motion",
                caption: "cem-timing-reduced-motion — prefers-reduced-motion overrides",
                headers: ["Token", "Reduced value", "Description"],
                sources: ["cem-timing-reduced-motion"],
                projection: [0, 1, 2],
            },
        ],
    },
    "cem-voice-fonts-typography": {
        generatorOnly: [
            "cem-typography-voice-ink-thickness-dark",
            "cem-typography-voice-ink-thickness-contrast",
        ],
        tables: [
            {
                id: "cem-typography-fontography",
                caption: "cem-typography-fontography — semantic family stacks",
                headers: ["Token", "Value", "Sample", "Tier", "Description"],
                sources: ["cem-typography-fontography"],
                projection: [0, 1, "sample", 3, 2],
                preview: "fontography",
            },
            {
                id: "cem-typography-primitives",
                caption: "thickness, size, line-height, letter-spacing, feature, reading-ergonomics",
                headers: ["Token", "Value", "Sample", "Tier", "Description"],
                sources: [
                    "cem-typography-thickness",
                    "cem-typography-size",
                    "cem-typography-line-height",
                    "cem-typography-letter-spacing",
                    "cem-typography-feature",
                    "cem-typography-reading-ergonomics",
                ],
                projection: [0, 1, "sample", 3, 2],
                preview: "primitives",
            },
            {
                id: "cem-typography-voice-channels",
                caption: "voice channels — 7 voices × 6 channels (ink + speech)",
                headers: ["Token", "Value", "Sample", "Tier", "Description"],
                sources: [
                    "cem-typography-voice-ink-thickness",
                    "cem-typography-voice-icon-stroke-multiplier",
                    "cem-typography-voice-speech-volume",
                    "cem-typography-voice-speech-rate",
                    "cem-typography-voice-speech-pitch",
                    "cem-typography-voice-ssml-emphasis",
                ],
                projection: [0, 1, "sample", 3, 2],
                preview: "voice",
            },
            {
                id: "cem-typography-roles",
                caption: "semantic role endpoints — 8 roles × ~9-10 properties",
                headers: ["Token", "Value", "Sample", "Tier"],
                sources: ["cem-typography-roles"],
                projection: [0, 1, "sample", 2],
                preview: "roles",
            },
        ],
    },
};

const THEME_MODES = [
    "cem-theme-native",
    "cem-theme-light",
    "cem-theme-dark",
    "cem-theme-contrast-light",
    "cem-theme-contrast-dark",
];

const MIME_TYPES = {
    ".html": "text/html",
    ".xhtml": "application/xhtml+xml",
    ".js": "application/javascript",
    ".mjs": "application/javascript",
    ".wasm": "application/wasm",
    ".css": "text/css",
    ".json": "application/json",
    ".xml": "application/xml",
    ".svg": "image/svg+xml",
    ".png": "image/png",
    ".jpg": "image/jpeg",
    ".jpeg": "image/jpeg",
    ".gif": "image/gif",
    ".webp": "image/webp",
};

function logOk(message) {
    console.log(`✓ ${message}`);
}

function fail(message) {
    throw new Error(message);
}

async function fileExists(filePath) {
    try {
        await fs.access(filePath);
        return true;
    } catch {
        return false;
    }
}

function run(command, args, options = {}) {
    return new Promise((resolve, reject) => {
        const child = spawn(command, args, {
            cwd: packageRoot,
            stdio: ["ignore", "pipe", "pipe"],
            ...options,
        });
        let stdout = "";
        let stderr = "";
        child.stdout.on("data", (chunk) => { stdout += chunk; });
        child.stderr.on("data", (chunk) => { stderr += chunk; });
        child.on("error", reject);
        child.on("close", (code) => {
            if (code === 0) {
                resolve({ stdout, stderr });
            } else {
                reject(new Error(`${command} ${args.join(" ")} failed with ${code}\n${stdout}\n${stderr}`));
            }
        });
    });
}

async function runManifestAndCssChecks() {
    for (const spec of SPECS) {
        const xhtml = `dist/lib/tokens/${spec.name}.xhtml`;
        const css = `dist/lib/css/${spec.name}.css`;
        if (!(await fileExists(path.join(packageRoot, xhtml)))) fail(`Missing token XHTML: ${xhtml}`);
        if (!(await fileExists(path.join(packageRoot, css)))) fail(`Missing generated CSS: ${css}`);
        await run("node", ["scripts/validate-manifest.mjs", "--hard", xhtml, css]);
    }
    logOk(`manifest coverage and CSS validity green for ${SPECS.length} specs`);
}

async function markdownFilesUnder(directory) {
    const files = [];
    for (const entry of await fs.readdir(directory, { withFileTypes: true })) {
        const entryPath = path.join(directory, entry.name);
        if (entry.isDirectory()) files.push(...await markdownFilesUnder(entryPath));
        else if (entry.isFile() && entry.name.endsWith(".md")) files.push(entryPath);
    }
    return files;
}

async function runMarkdownDocumentInventoryChecks() {
    const sourceRoot = path.join(packageRoot, "src");
    const sourceFiles = await markdownFilesUnder(sourceRoot);
    if (sourceFiles.length === 0) fail("No theme Markdown source documents were found");

    for (const sourcePath of sourceFiles) {
        const relativePath = path.relative(sourceRoot, sourcePath);
        const publishedMarkdownPath = path.join(packageRoot, "dist", relativePath);
        const xhtmlPath = publishedMarkdownPath.replace(/\.md$/, ".xhtml");
        const source = await fs.readFile(sourcePath);
        const published = await fs.readFile(publishedMarkdownPath).catch(() => null);
        if (!published) fail(`${relativePath}: published Markdown copy is missing`);
        if (!source.equals(published)) fail(`${relativePath}: published Markdown copy differs from its source`);

        const xhtml = await fs.readFile(xhtmlPath, "utf8").catch(() => "");
        if (!/<html(?:\s|>)/.test(xhtml) || !/<body(?:\s|>)/.test(xhtml) || !/<\/html>/.test(xhtml)) {
            fail(`${relativePath}: compiled XHTML document is missing or incomplete`);
        }
    }

    logOk(`${sourceFiles.length} theme Markdown documents compile to XHTML with byte-identical published sources`);
}

async function readCombinedCss() {
    const parts = [];
    for (const spec of SPECS) {
        const cssPath = path.join(packageRoot, `dist/lib/css/${spec.name}.css`);
        parts.push(`/* ${spec.name} */\n${await fs.readFile(cssPath, "utf8")}`);
    }
    return parts.join("\n\n");
}

async function startServer() {
    const server = http.createServer(async (req, res) => {
        const pathname = decodeURIComponent(req.url.split("?")[0]);
        const filePath = path.join(docRoot, pathname);
        try {
            const data = await fs.readFile(filePath);
            const contentType = MIME_TYPES[path.extname(filePath).toLowerCase()] || "application/octet-stream";
            res.writeHead(200, { "Content-Type": contentType });
            res.end(data);
        } catch {
            res.writeHead(404);
            res.end("Not found");
        }
    });

    await new Promise((resolve, reject) => {
        server.once("error", reject);
        server.listen(0, "127.0.0.1", resolve);
    });
    return server;
}

async function withBrowser(callback) {
    const { chromium } = await import("playwright");
    const server = await startServer();
    const browser = await chromium.launch({ headless: true });
    try {
        const port = server.address().port;
        const baseUrl = `http://127.0.0.1:${port}`;
        await callback(browser, baseUrl);
    } finally {
        await browser.close();
        server.close();
    }
}

async function runColorGeneratorProtocolChecks(page) {
    const failures = await page.evaluate(async () => {
        const errors = [];
        const normalize = (value) => (value ?? "").replace(/\s+/g, " ").trim();
        const texts = (root, selector) => Array.from(root.querySelectorAll(selector), (node) => normalize(node.textContent));
        const rows = (table) => Array.from(table?.querySelectorAll("tbody tr") ?? [], (row) =>
            Array.from(row.children, (cell) => normalize(cell.textContent))
        );
        const expect = (condition, message) => {
            if (!condition) errors.push(message);
        };
        const expectEqual = (actual, expected, message) => {
            if (JSON.stringify(actual) !== JSON.stringify(expected)) {
                errors.push(`${message}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
            }
        };

        const template = document.querySelector('template[id="cem-colors-template"]');
        const tokenUrl = template?.getAttribute("data-token-url");
        expect(Boolean(tokenUrl), "color generator template must declare data-token-url");
        if (!tokenUrl) return errors;

        const response = await fetch(new URL(tokenUrl, document.baseURI));
        expect(response.ok, `color token source must load (${response.status})`);
        if (!response.ok) return errors;
        const source = new DOMParser().parseFromString(await response.text(), "text/html");

        const sourceTable = (id) => {
            let sibling = source.getElementById(id)?.nextElementSibling ?? null;
            while (sibling && sibling.localName !== "table") sibling = sibling.nextElementSibling;
            return sibling;
        };
        const sourceRows = (id) => rows(sourceTable(id));
        const renderedTable = (id) => document.querySelector(`main table#${CSS.escape(id)}`);
        const tableHeaders = (table) => texts(table, "thead th");
        const wrapCssReference = (value) => value.startsWith("--") ? `var(${value})` : value;

        const hueSource = sourceRows("cem-color-hue-variant");
        const hue = renderedTable("cem-color-hue-variant");
        expect(Boolean(hue), "branded-color presentation table is missing");
        if (hue) {
            expectEqual(tableHeaders(hue), ["Token", "Hue", "Swatch", "Label", "Value"], "branded-color headers");
            expectEqual(
                rows(hue),
                hueSource.map((row) => [row[0], row[1], "", row[4], row[3]]),
                "branded-color rows must project Markdown token, hue, label, and value columns"
            );
            const renderedRows = Array.from(hue.querySelectorAll("tbody tr"));
            renderedRows.forEach((row, index) => {
                const sourceRow = hueSource[index];
                const swatch = row.children[2];
                expect(swatch?.style.backgroundColor === `var(${sourceRow[0]})`, `branded row ${index + 1} swatch token`);
                expect(normalize(swatch?.title).includes(normalize(sourceRow[5])), `branded row ${index + 1} intended-use tooltip`);
                expect(swatch?.getAttribute("data-cem-value") === `${sourceRow[0]}: ${sourceRow[3]}`, `branded row ${index + 1} selectable value`);
                expect(getComputedStyle(swatch).backgroundColor !== "rgba(0, 0, 0, 0)", `branded row ${index + 1} computed swatch`);
            });
        }

        const nativeSource = sourceRows("cem-color-native");
        const native = renderedTable("cem-color-native-table");
        expect(Boolean(native), "native-color presentation table is missing");
        if (native) {
            expectEqual(tableHeaders(native), ["Color", "Light", "Dark", "Description"], "native-color headers");
            expectEqual(
                rows(native),
                nativeSource.map((row) => [row[0], row[0], row[0], row[2]]),
                "native-color rows must project every Markdown row"
            );
            Array.from(native.querySelectorAll("tbody tr")).forEach((row, index) => {
                const sourceRow = nativeSource[index];
                expect(row.children[1]?.style.colorScheme === "light", `native row ${index + 1} light preview`);
                expect(row.children[2]?.style.colorScheme === "dark", `native row ${index + 1} dark preview`);
                expect(row.children[1]?.getAttribute("data-cem-value") === sourceRow[0], `native row ${index + 1} selectable value`);
            });
        }

        const shiftSource = sourceRows("cem-palette-emotion-shift");
        const baseEmotions = shiftSource.filter((row) => !row[0].includes("-text") && !row[0].includes("-x"));
        const emotion = renderedTable("cem-palette-emotion-shift");
        expect(Boolean(emotion), "emotional-palette presentation table is missing");
        if (emotion) {
            expectEqual(
                tableHeaders(emotion),
                ["emotion token", "light", "dark", "native light", "native dark"],
                "emotional-palette headers"
            );
            const actual = rows(emotion);
            const expected = baseEmotions.flatMap((row) => [
                [row[0], `light ⬤ ${row[7]}`, `dark ⬤ ${row[7]}`, `native light ⬤ ${row[7]}`, `native dark ⬤ ${row[7]}`],
                [`${row[0]}-x`, `light ⬤ ${row[7]}-x`, `dark ⬤ ${row[7]}-x`, `native light ⬤ ${row[7]}-x`, `native dark ⬤ ${row[7]}-x`],
            ]);
            expectEqual(actual, expected, "emotional-palette base and extreme rows");
            Array.from(emotion.querySelectorAll("tbody tr")).forEach((row, index) => {
                const token = expected[index][0];
                expect(row.children[1]?.style.backgroundColor === `var(${token})`, `emotion row ${index + 1} light swatch`);
                expect(row.children[2]?.style.colorScheme === "dark", `emotion row ${index + 1} dark swatch`);
                expect(row.children[3]?.classList.contains("cem-theme-native"), `emotion row ${index + 1} native-light scope`);
                expect(row.children[4]?.classList.contains("cem-theme-native"), `emotion row ${index + 1} native-dark scope`);
                if (index % 2 === 0) {
                    expect(row.children[3]?.getAttribute("data-selected-token")?.includes(".cem-theme-native"), `emotion row ${index + 1} native-light selector`);
                    expect(row.children[4]?.getAttribute("data-selected-token")?.includes(".cem-theme-native"), `emotion row ${index + 1} native-dark selector`);
                }
            });
        }

        const modeSource = sourceRows("cem-theme-mode");
        const intentSource = sourceRows("cem-action-intent-emotion");
        const stateSource = sourceRows("cem-action-state-color");
        const actionTables = Array.from(document.querySelectorAll("main table#cem-action-state-color"));
        expect(actionTables.length === modeSource.length, "one action matrix must render per Markdown theme mode");
        actionTables.forEach((table, modeIndex) => {
            const mode = modeSource[modeIndex];
            expect(table.closest("section")?.classList.contains(`cem-theme-${mode[0]}`), `action matrix ${mode[0]} theme scope`);
            expect(normalize(table.querySelector("caption b")?.textContent) === mode[0], `action matrix ${mode[0]} caption`);
            expect(normalize(table.querySelector("caption sub")?.textContent) === mode[2], `action matrix ${mode[0]} description`);
            expect(table.getAttribute("data-selected-token")?.includes('[data-theme*="cem-theme-"]'), `action matrix ${mode[0]} selectable theme scope`);
            expectEqual(tableHeaders(table), ["State \\ intent:", ...intentSource.map((row) => row[0])], `action matrix ${mode[0]} headers`);
            expectEqual(
                rows(table),
                stateSource.map((state) => [state[0], ...intentSource.map(() => "Aa")]),
                `action matrix ${mode[0]} state × intent cells`
            );
            Array.from(table.querySelectorAll("tbody tr")).forEach((row, stateIndex) => {
                const state = stateSource[stateIndex];
                intentSource.forEach((intent, intentIndex) => {
                    const cell = row.children[intentIndex + 1];
                    const prefix = `--cem-action-${intent[0]}-${state[0]}`;
                    const expectedValue = `${prefix}-background: ${state[1].replaceAll("[emotion]", intent[1])}; ${prefix}-text: ${state[2].replaceAll("[emotion]", intent[1])}`;
                    expect(cell?.style.backgroundColor === `var(${prefix}-background)`, `${mode[0]} ${state[0]} ${intent[0]} background preview`);
                    expect(cell?.style.color === `var(${prefix}-text)`, `${mode[0]} ${state[0]} ${intent[0]} text preview`);
                    expect(cell?.getAttribute("data-cem-value") === expectedValue, `${mode[0]} ${state[0]} ${intent[0]} selectable formulas`);
                });
            });
        });

        const zmapSource = sourceRows("cem-zebra-mode-mapping");
        const zebraTables = Array.from(document.querySelectorAll("main table")).filter((table) =>
            tableHeaders(table).join("|") === "State|color-0|color-1|color-2|color-3|Ring"
        );
        expect(zebraTables.length === modeSource.length, "one zebra matrix must render per Markdown theme mode");
        const zebraStates = ["none", "focus", "selected", "target", "focus+selected", "focus+selected+target"];
        zebraTables.forEach((table, modeIndex) => {
            const mode = modeSource[modeIndex];
            const mapping = zmapSource.find((row) => row[0] === mode[0]);
            expect(Boolean(mapping), `zebra mapping for ${mode[0]}`);
            if (!mapping) return;
            expect(table.closest("section")?.classList.contains(`cem-theme-${mode[0]}`), `zebra matrix ${mode[0]} theme scope`);
            const selectedScope = table.getAttribute("data-selected-token") ?? "";
            if (["native", "light", "dark"].includes(mode[0])) {
                expect(selectedScope.includes(`cem-theme-${mode[0]}`), `zebra matrix ${mode[0]} selectable theme scope`);
            } else {
                expect(selectedScope === "", `zebra matrix ${mode[0]} must not invent a selector override`);
            }
            expectEqual(rows(table).map((row) => row[0]), zebraStates, `zebra matrix ${mode[0]} states`);
            const valueRows = [
                [mapping[1], mapping[1], mapping[1], mapping[1]],
                [mapping[1], mapping[2], mapping[1], mapping[1]],
                [mapping[1], mapping[1], mapping[1], mapping[4]],
                [mapping[1], mapping[1], mapping[3], mapping[1]],
                [mapping[1], mapping[2], mapping[1], mapping[4]],
                [mapping[1], mapping[2], mapping[3], mapping[4]],
            ];
            Array.from(table.querySelectorAll("tbody tr")).forEach((row, rowIndex) => {
                if (rowIndex > 0) expect(row.getAttribute("data-selected-token")?.endsWith("{}"), `zebra ${mode[0]} ${zebraStates[rowIndex]} selectable state scope`);
                valueRows[rowIndex].forEach((value, colorIndex) => {
                    const cell = row.children[colorIndex + 1];
                    expect(
                        cell?.getAttribute("data-cem-value") === `--cem-zebra-color-${colorIndex}: ${wrapCssReference(value)}`,
                        `zebra ${mode[0]} ${zebraStates[rowIndex]} color-${colorIndex} source mapping`
                    );
                });
                expect(row.lastElementChild?.querySelector("div")?.style.boxShadow.length > 0, `zebra ${mode[0]} ${zebraStates[rowIndex]} ring preview`);
            });
        });

        const simpleGroups = [
            ["cem-input-indicator-colors", [0, 1, 2, 4, 3], ["Token", "Value type", "Formula", "Tier", "Notes"]],
            ["cem-select-state-colors", [0, 1, 2, 4, 3], ["Token", "Value type", "Formula", "Tier", "Notes"]],
            ["cem-navigation-item-state-colors", [0, 1, 2, 4, 3], ["Token", "Value type", "Formula", "Tier", "Notes"]],
            ["cem-content-interaction-state-colors", [0, 1, 2, 4, 3], ["Token", "Value type", "Formula", "Tier", "Notes"]],
            ["cem-separator-colors", [0, 1, 2, 3, 5, 4], ["Token", "Value type", "Formula", "Forced colors", "Tier", "Notes"]],
            ["cem-progress-indicator-colors", [0, 1, 2, 3, 5, 4], ["Token", "Value type", "Formula", "Forced colors", "Tier", "Notes"]],
            ["cem-workflow-step-colors", [0, 1, 2, 3, 5, 4], ["Token", "Value type", "Formula", "Forced colors", "Tier", "Notes"]],
            ["cem-slider-colors", [0, 1, 2, 3, 5, 4], ["Token", "Value type", "Formula", "Forced colors", "Tier", "Notes"]],
        ];
        simpleGroups.forEach(([id, projection, headers]) => {
            const table = renderedTable(id);
            expect(Boolean(table), `${id} presentation table is missing`);
            if (!table) return;
            expectEqual(tableHeaders(table), headers, `${id} headers`);
            expectEqual(rows(table), sourceRows(id).map((row) => projection.map((index) => row[index])), `${id} complete Markdown column projection`);
        });

        const expectedTableCount = 3 + (modeSource.length * 2) + simpleGroups.length;
        expect(document.querySelectorAll("main table").length === expectedTableCount, "color presentation table inventory must be source-complete");
        const generatedCss = document.querySelector("main code[data-generated-css]")?.textContent ?? "";
        sourceRows("cem-zebra-tokens").forEach((row) => {
            expect(generatedCss.includes(`${row[0]}: ${row[2]};`), `generated CSS must derive ${row[0]} from cem-zebra-tokens`);
        });
        expect(document.querySelectorAll("main code[data-generated-css]").length === 1, "color generator must expose exactly one generated CSS block");

        return errors;
    });

    if (failures.length) fail(`cem-colors Markdown-driven presentation protocol:\n${failures.join("\n")}`);
    logOk("cem-colors Markdown-driven presentation protocol is source-complete");
}

async function runGeneratorPresentationProtocolChecks(page, specName) {
    const protocol = PRESENTATION_PROTOCOLS[specName];
    if (!protocol) fail(`${specName}: missing Markdown presentation protocol`);

    const failures = await page.evaluate(async ({ protocol, specName }) => {
        const errors = [];
        const normalize = (value) => (value ?? "").replace(/\s+/g, " ").trim();
        const expect = (condition, message) => {
            if (!condition) errors.push(message);
        };
        const expectEqual = (actual, expected, message) => {
            if (JSON.stringify(actual) !== JSON.stringify(expected)) {
                errors.push(`${message}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
            }
        };
        const rowCells = (row) => Array.from(row?.children ?? [], (cell) => normalize(cell.textContent));
        const tableRows = (table) => Array.from(table?.querySelectorAll("tbody tr") ?? [], rowCells);
        const tableHeaders = (table) =>
            Array.from(table?.querySelectorAll("thead th") ?? [], (cell) => normalize(cell.textContent));

        const template = document.querySelector('template[data-token-url][data-slices]');
        const tokenUrl = template?.getAttribute("data-token-url");
        expect(Boolean(tokenUrl), `${specName} template must declare data-token-url`);
        if (!tokenUrl) return errors;

        const response = await fetch(new URL(tokenUrl, document.baseURI));
        expect(response.ok, `${specName} token source must load (${response.status})`);
        if (!response.ok) return errors;
        const source = new DOMParser().parseFromString(await response.text(), "text/html");
        const sourceTable = (id) => {
            let sibling = source.getElementById(id)?.nextElementSibling ?? null;
            while (sibling && sibling.localName !== "table") sibling = sibling.nextElementSibling;
            return sibling;
        };
        const sourceRows = (id) => tableRows(sourceTable(id));
        const sourceHeaders = (id) => tableHeaders(sourceTable(id));

        const configuredSources = (template.getAttribute("data-slices") ?? "")
            .split(/\s+/)
            .filter(Boolean)
            .map((entry) => entry.split("=")[1])
            .filter(Boolean);
        const presentedSources = protocol.tables.flatMap((table) => table.sources);
        expectEqual(
            [...new Set([...presentedSources, ...protocol.generatorOnly])].sort(),
            [...new Set(configuredSources)].sort(),
            `${specName} configured source tables must be explicitly presented or generator-only`
        );

        const sampleText = (kind, token, value) => {
            if (kind === "fontography") return "Reading UI 123";
            if (kind === "primitives") {
                if (token.includes("thickness")) return "Weight Aa 123";
                if (token.includes("-size-")) return "Size Aa 123";
                if (token.includes("line-height")) return "Line height examplesecond line";
                if (token.includes("letter-spacing")) return "TRACKING 123";
                if (token.includes("numeric")) return "1111 8888 12.34";
                if (token.includes("ligatures")) return "office affine ffi";
                if (token.includes("optical-sizing")) return "Optical size";
                if (token.includes("measure")) {
                    return "Readable measure example wraps only when the available table space is narrower than the token.";
                }
                if (token.includes("paragraph-gap")) return "Paragraph one. Paragraph two.";
                return "Aa 123";
            }
            if (kind === "voice") {
                if (token.includes("ink-thickness")) return "Voice ink Aa 123";
                if (token.includes("icon-stroke-multiplier")) return "";
                if (token.includes("speech-volume")) return `Speech volume ${value}`;
                if (token.includes("speech-rate")) return `Speech rate ${value}`;
                if (token.includes("speech-pitch")) return `Speech pitch ${value}`;
                if (token.includes("ssml-emphasis")) return `SSML emphasis: ${value}`;
                return `Voice value ${value}`;
            }
            if (kind === "roles") {
                if (token.includes("font-family")) return "Role family Aa 123";
                if (token.includes("font-size")) return "Role size Aa 123";
                if (token.includes("line-height")) return "Role line heightsecond line";
                if (token.includes("letter-spacing")) return "ROLE TRACKING";
                if (token.includes("font-weight")) return "Role weight Aa 123";
                if (token.includes("font-variant-numeric")) return "1111 8888 12.34";
                if (token.includes("font-variant-ligatures")) return "office affine ffi";
                if (token.includes("text-transform")) return "role text";
                if (token.includes("speech-volume")) return `Speech volume ${value}`;
                if (token.includes("speech-rate")) return `Speech rate ${value}`;
                if (token.includes("speech-pitch")) return `Speech pitch ${value}`;
                if (token.includes("ssml-emphasis")) return `SSML emphasis: ${value}`;
                return `Role value ${value}`;
            }
            return "";
        };

        const projectedCell = (projection, row, preview) => {
            if (Number.isInteger(projection)) return row[projection] ?? "";
            if (projection === "blank") return "";
            if (projection === "value-description") return `${row[1]} — ${row[2]}`;
            if (projection === "sample") return sampleText(preview, row[0], row[1]);
            return "";
        };

        const checkPreview = (kind, renderedRow, sourceRow, rowIndex, tableId, previewColumn) => {
            const token = sourceRow[0];
            let cellIndex = Number.isInteger(previewColumn) ? previewColumn : 3;
            if (kind === "height") cellIndex = 1;
            if (["fontography", "primitives", "voice", "roles"].includes(kind)) cellIndex = 2;
            const cell = renderedRow.children[cellIndex];
            const sample = cell?.firstElementChild;
            expect(Boolean(sample), `${tableId} row ${rowIndex + 1} ${kind} preview is missing`);
            if (!sample) return;

            if (kind === "width") expect(sample.style.width === `var(${token})`, `${tableId} row ${rowIndex + 1} width preview`);
            if (kind === "height") {
                expect(sample.style.height === sourceRow[1], `${tableId} row ${rowIndex + 1} height preview`);
                expect(sample.title === `${token}: ${sourceRow[1]}`, `${tableId} row ${rowIndex + 1} height tooltip`);
            }
            if (kind === "shadow") expect(sample.style.boxShadow === `var(${token})`, `${tableId} row ${rowIndex + 1} shadow preview`);
            if (kind === "radius") expect(sample.style.borderRadius === `var(${token})`, `${tableId} row ${rowIndex + 1} radius preview`);
            if (kind === "stroke") expect(sample.style.borderBlockEnd.includes(`var(${token})`), `${tableId} row ${rowIndex + 1} stroke preview`);
            if (["duration", "easing"].includes(kind)) {
                expect(sample.style.transition.includes(`var(${token})`), `${tableId} row ${rowIndex + 1} timing preview`);
                expect(sample.getAttribute("onmouseenter")?.includes("scale(1.5)"), `${tableId} row ${rowIndex + 1} hover demo`);
            }

            const tokenStyle = (property) => sample.style.getPropertyValue(property) === `var(${token})`;
            if (kind === "fontography") expect(tokenStyle("font-family"), `${tableId} row ${rowIndex + 1} family preview`);
            if (kind === "primitives") {
                if (token.includes("thickness")) expect(tokenStyle("font-weight"), `${tableId} row ${rowIndex + 1} weight preview`);
                else if (token.includes("-size-")) expect(tokenStyle("font-size"), `${tableId} row ${rowIndex + 1} size preview`);
                else if (token.includes("line-height")) expect(tokenStyle("line-height"), `${tableId} row ${rowIndex + 1} line-height preview`);
                else if (token.includes("letter-spacing")) expect(tokenStyle("letter-spacing"), `${tableId} row ${rowIndex + 1} tracking preview`);
                else if (token.includes("numeric")) expect(tokenStyle("font-variant-numeric"), `${tableId} row ${rowIndex + 1} numeric preview`);
                else if (token.includes("ligatures")) expect(tokenStyle("font-variant-ligatures"), `${tableId} row ${rowIndex + 1} ligature preview`);
                else if (token.includes("optical-sizing")) expect(tokenStyle("font-optical-sizing"), `${tableId} row ${rowIndex + 1} optical preview`);
                else if (token.includes("measure")) expect(tokenStyle("max-width"), `${tableId} row ${rowIndex + 1} measure preview`);
                else if (token.includes("paragraph-gap")) {
                    const second = cell.children[1];
                    expect(second?.style.marginTop === `var(${token})`, `${tableId} row ${rowIndex + 1} paragraph-gap preview`);
                }
            }
            if (kind === "voice") {
                if (token.includes("ink-thickness")) expect(tokenStyle("font-weight"), `${tableId} row ${rowIndex + 1} voice weight preview`);
                else if (token.includes("icon-stroke-multiplier")) {
                    expect(sample.style.getPropertyValue("--sample-multiplier") === `var(${token})`, `${tableId} row ${rowIndex + 1} icon stroke preview`);
                } else if (token.includes("speech-volume")) expect(tokenStyle("opacity"), `${tableId} row ${rowIndex + 1} voice volume preview`);
            }
            if (kind === "roles") {
                const property = [
                    "font-family",
                    "font-size",
                    "line-height",
                    "letter-spacing",
                    "font-weight",
                    "font-variant-numeric",
                    "font-variant-ligatures",
                    "text-transform",
                    "speech-volume",
                ].find((candidate) => token.includes(candidate));
                if (property === "speech-volume") expect(tokenStyle("opacity"), `${tableId} row ${rowIndex + 1} role volume preview`);
                else if (property) expect(tokenStyle(property), `${tableId} row ${rowIndex + 1} role ${property} preview`);
            }
        };

        for (const tableContract of protocol.tables) {
            const table = document.querySelector(`main table#${CSS.escape(tableContract.id)}`);
            expect(Boolean(table), `${tableContract.id} presentation table is missing`);
            if (!table) continue;
            expectEqual(tableHeaders(table), tableContract.headers, `${tableContract.id} headers`);
            expect(
                normalize(table.caption?.textContent) === tableContract.caption,
                `${tableContract.id} caption must preserve the established presentation`
            );
            const inputRows = tableContract.sources.flatMap(sourceRows);
            const expectedRows = inputRows.map((row) =>
                tableContract.projection.map((projection) => projectedCell(projection, row, tableContract.preview))
            );
            expectEqual(tableRows(table), expectedRows, `${tableContract.id} Markdown row projection`);
            if (tableContract.preview) {
                Array.from(table.querySelectorAll("tbody tr")).forEach((row, index) =>
                    checkPreview(
                        tableContract.preview,
                        row,
                        inputRows[index],
                        index,
                        tableContract.id,
                        tableContract.previewColumn
                    )
                );
            }
        }

        expect(
            document.querySelectorAll("main table").length === protocol.tables.length,
            `${specName} presentation table inventory must match its protocol`
        );
        const generatedCss = document.querySelector("main code[data-generated-css]")?.textContent ?? "";
        expect(Boolean(generatedCss), `${specName} generated CSS is missing`);
        for (const sourceId of configuredSources) {
            const headers = sourceHeaders(sourceId).map((header) => header.toLowerCase());
            const tierIndex = headers.indexOf("tier");
            const valueIndexes = headers
                .map((header, index) => ({ header, index }))
                .filter(({ header }) => header === "value" || header.endsWith(" value"))
                .map(({ index }) => index);
            for (const [rowIndex, row] of sourceRows(sourceId).entries()) {
                const token = row[0];
                if (!token.startsWith("--") || (tierIndex >= 0 && row[tierIndex] === "deprecated")) continue;
                expect(generatedCss.includes(token), `${sourceId} row ${rowIndex + 1} token must occur in generated CSS`);
                for (const valueIndex of valueIndexes) {
                    expect(
                        generatedCss.includes(`${token}: ${row[valueIndex]};`),
                        `${sourceId} row ${rowIndex + 1} value must drive generated CSS`
                    );
                }
            }
        }

        if (specName === "cem-breakpoints") {
            for (const [sourceId, dimension] of [["cem-bp-media-ranges", "width"], ["cem-bp-height-ranges", "height"]]) {
                for (const [range, minimum, maximum] of sourceRows(sourceId)) {
                    expect(generatedCss.includes(`--cem-bp-active-${dimension}: ${range};`), `${sourceId} ${range} active value`);
                    if (minimum !== "0px") expect(generatedCss.includes(`min-${dimension}: ${minimum}`), `${sourceId} ${range} minimum`);
                    if (maximum !== "—") expect(generatedCss.includes(`max-${dimension}: ${maximum}`), `${sourceId} ${range} maximum`);
                }
            }
        }

        return errors;
    }, { protocol, specName });

    if (failures.length) fail(`${specName} Markdown-driven presentation protocol:\n${failures.join("\n")}`);
    logOk(`${specName} Markdown-driven presentation protocol is source-complete`);
}

async function runGeneratorBrowserChecks(browser, baseUrl) {
    const context = await browser.newContext({ javaScriptEnabled: true, bypassCSP: true });
    const page = await context.newPage();
    const errors = [];
    page.on("pageerror", (err) => errors.push(err.message));

    for (const spec of SPECS) {
        const rel = path.relative(docRoot, path.join(packageRoot, `dist/lib/css-generators/${spec.name}.html`));
        await page.goto(`${baseUrl}/${rel}`, { waitUntil: "networkidle" });
        await page.waitForTimeout(500);
        const result = await page.evaluate((token) => {
            const codeBlocks = Array.from(document.querySelectorAll("code[data-generated-css]"));
            const generatedCss = codeBlocks.map((node) => node.textContent.trim()).join("\n");
            return {
                tableCount: document.querySelectorAll("main table").length,
                tableDataRowCount: document.querySelectorAll("main tbody tr").length,
                codeBlockCount: codeBlocks.length,
                generatedCssBytes: generatedCss.length,
                hasTokenInCss: generatedCss.includes(token),
                rootValue: getComputedStyle(document.documentElement).getPropertyValue(token).trim(),
                loaderCount: document.head.querySelectorAll("style[data-cem-css-loader]").length,
            };
        }, spec.token);

        if (result.tableCount === 0) fail(`${spec.name}: rendered token tables are missing`);
        if (result.tableDataRowCount === 0) fail(`${spec.name}: rendered token tables have no data rows`);
        if (result.codeBlockCount !== 1) fail(`${spec.name}: expected exactly one data-generated-css block`);
        if (result.generatedCssBytes === 0) fail(`${spec.name}: generated CSS block is empty`);
        if (!result.hasTokenInCss) fail(`${spec.name}: generated CSS block does not include ${spec.token}`);
        if (!result.rootValue) fail(`${spec.name}: ${spec.token} did not resolve on :root`);
        if (result.loaderCount === 0) fail(`${spec.name}: no cem-css-loader style was injected`);
        if (spec.name === "cem-colors") await runColorGeneratorProtocolChecks(page);
        else await runGeneratorPresentationProtocolChecks(page, spec.name);
    }

    if (errors.length) fail(`Browser page errors:\n${errors.join("\n")}`);
    await context.close();
    logOk(`browser generator capture green for ${SPECS.length} generators`);
}

async function runThemeModeChecks(browser, combinedCss) {
    const context = await browser.newContext({ javaScriptEnabled: true, bypassCSP: true });
    const page = await context.newPage();
    await page.setContent(`<style>${combinedCss}</style><main id="theme-scope">phase13</main>`);

    const result = await page.evaluate((themeModes) => {
        const rootKeys = [
            "--cem-palette-comfort",
            "--cem-bend-smooth",
            "--cem-stroke-focus",
            "--cem-layer-work",
            "--cem-typography-reading-line-height",
        ];
        const scopedKeys = [
            "--cem-action-explicit-default-background",
        ];
        const scope = document.getElementById("theme-scope");
        const checks = {};
        for (const mode of themeModes) {
            document.documentElement.className = mode;
            document.documentElement.setAttribute("data-theme", mode);
            scope.className = mode;
            scope.setAttribute("data-theme", mode);
            const style = getComputedStyle(document.documentElement);
            const scopedStyle = getComputedStyle(scope);
            checks[mode] = {
                root: Object.fromEntries(rootKeys.map((key) => [key, style.getPropertyValue(key).trim()])),
                scoped: Object.fromEntries(scopedKeys.map((key) => [key, scopedStyle.getPropertyValue(key).trim()])),
            };
        }
        return checks;
    }, THEME_MODES);

    for (const [mode, groups] of Object.entries(result)) {
        for (const [token, value] of Object.entries(groups.root)) {
            if (!value) fail(`${mode}: root ${token} did not resolve`);
        }
        for (const [token, value] of Object.entries(groups.scoped)) {
            if (!value) fail(`${mode}: scoped ${token} did not resolve`);
        }
    }
    await context.close();
    logOk(`theme-mode root resolution green for ${THEME_MODES.length} modes`);
}

async function runForcedColorsChecks(browser, combinedCss) {
    const context = await browser.newContext({ javaScriptEnabled: true, bypassCSP: true, forcedColors: "active" });
    const page = await context.newPage();
    await page.setContent(`<style>${combinedCss}</style><main>phase13</main>`);

    const result = await page.evaluate(() => {
        const style = getComputedStyle(document.documentElement);
        return {
            forcedColors: matchMedia("(forced-colors: active)").matches,
            zebra3: style.getPropertyValue("--cem-ring-zebra-3").trim(),
            zebra4: style.getPropertyValue("--cem-ring-zebra-4").trim(),
            elevation4: style.getPropertyValue("--cem-elevation-4").trim(),
            strokeFocus: style.getPropertyValue("--cem-stroke-focus").trim(),
            bendSmooth: style.getPropertyValue("--cem-bend-smooth").trim(),
            inkRegular: style.getPropertyValue("--cem-voice-regular-ink-thickness").trim(),
        };
    });

    if (!result.forcedColors) fail("forced-colors media query did not activate in browser context");
    if (!result.zebra3.includes("Highlight")) fail("D5 forced-colors zebra-3 fallback is not using Highlight");
    if (!result.zebra4.includes("Highlight")) fail("D5 forced-colors zebra-4 fallback is not using Highlight");
    if (result.elevation4 !== "none") fail("D4 forced-colors elevation rung did not collapse to none");
    for (const token of ["strokeFocus", "bendSmooth", "inkRegular"]) {
        if (!result[token]) fail(`forced-colors smoke: ${token} did not resolve`);
    }
    await context.close();
    logOk("forced-colors smoke green for D3/D4/D5/D6 representative tokens");
}

async function runReducedMotionChecks(browser, combinedCss) {
    const normalContext = await browser.newContext({ javaScriptEnabled: true, bypassCSP: true });
    const reducedContext = await browser.newContext({
        javaScriptEnabled: true,
        bypassCSP: true,
        reducedMotion: "reduce",
    });

    async function readDurations(context) {
        const page = await context.newPage();
        await page.setContent(`<style>${combinedCss}</style>`);
        const values = await page.evaluate(() => {
            const style = getComputedStyle(document.documentElement);
            return {
                reduced: matchMedia("(prefers-reduced-motion: reduce)").matches,
                instant: style.getPropertyValue("--cem-duration-instant").trim(),
                noticeable: style.getPropertyValue("--cem-duration-noticeable").trim(),
                lingering: style.getPropertyValue("--cem-duration-lingering").trim(),
            };
        });
        await page.close();
        return values;
    }

    const normal = await readDurations(normalContext);
    const reduced = await readDurations(reducedContext);
    await normalContext.close();
    await reducedContext.close();

    if (normal.reduced) fail("normal context unexpectedly matches prefers-reduced-motion");
    if (!reduced.reduced) fail("reduced-motion media query did not activate");
    if (reduced.instant !== "0ms") fail("reduced-motion instant duration should be 0ms");
    if (parseMs(reduced.noticeable) >= parseMs(normal.noticeable)) fail("reduced noticeable duration did not shorten");
    if (parseMs(reduced.lingering) >= parseMs(normal.lingering)) fail("reduced lingering duration did not shorten");
    if (!(parseMs(reduced.instant) <= parseMs(reduced.noticeable) && parseMs(reduced.noticeable) <= parseMs(reduced.lingering))) {
        fail("reduced-motion duration ordering is not preserved");
    }
    logOk("reduced-motion durations shorten while preserving ordering");
}

async function runShapeBrowserValidation(browser, combinedCss) {
    const context = await browser.newContext({ javaScriptEnabled: true, bypassCSP: true });
    const page = await context.newPage();
    await page.setContent(`
        <style>
            ${combinedCss}
            .cem-shape-proof {
                inline-size: 10rem;
                block-size: var(--cem-control-height);
                border-radius: var(--cem-bend-round);
                outline: var(--cem-stroke-focus) solid currentColor;
                outline-offset: var(--cem-stroke-indicator-offset);
                overflow: visible;
            }
            .cem-shape-attached {
                inline-size: 6rem;
                block-size: 3rem;
                border-start-start-radius: var(--cem-bend-attached-edge);
                border-end-start-radius: var(--cem-bend-attached-edge);
                border-start-end-radius: var(--cem-bend-free-edge);
                border-end-end-radius: var(--cem-bend-free-edge);
            }
        </style>
        <div id="shape-proof" class="cem-shape-proof" tabindex="0"></div>
        <div id="shape-attached" class="cem-shape-attached"></div>
    `);

    const couplingResults = [];
    for (const mode of ["balanced", "forgiving", "compact"]) {
        await page.evaluate((nextMode) => {
            document.documentElement.setAttribute("data-cem-coupling", nextMode);
        }, mode);
        couplingResults.push(await page.evaluate((nextMode) => {
            const el = document.getElementById("shape-proof");
            const style = getComputedStyle(el);
            return {
                mode: nextMode,
                blockSize: style.blockSize,
                borderRadius: style.borderRadius,
                overflow: style.overflow,
                outlineWidth: style.outlineWidth,
                outlineOffset: style.outlineOffset,
            };
        }, mode));
    }

    for (const result of couplingResults) {
        if (!(parseCssLengthPx(result.blockSize) > 0)) fail(`D3 shape ${result.mode}: control block-size did not resolve`);
        if (!(parseCssLengthPx(result.borderRadius) > 0)) fail(`D3 shape ${result.mode}: round-end radius did not resolve`);
        if (result.overflow !== "visible") fail(`D3 shape ${result.mode}: focus-ring proof is clipped by overflow`);
        if (!(parseCssLengthPx(result.outlineWidth) > 0)) fail(`D3 shape ${result.mode}: focus outline width did not resolve`);
        if (!(parseCssLengthPx(result.outlineOffset) >= 0)) fail(`D3 shape ${result.mode}: focus outline offset did not resolve`);
    }

    const zoomResults = [];
    for (const zoom of [2, 4]) {
        await page.evaluate((nextZoom) => {
            document.body.style.zoom = String(nextZoom);
        }, zoom);
        zoomResults.push(await page.evaluate((nextZoom) => {
            const style = getComputedStyle(document.getElementById("shape-proof"));
            return {
                zoom: nextZoom,
                borderRadius: style.borderRadius,
                outlineWidth: style.outlineWidth,
                scrollWidth: document.documentElement.scrollWidth,
                clientWidth: document.documentElement.clientWidth,
            };
        }, zoom));
    }
    for (const result of zoomResults) {
        if (!(parseCssLengthPx(result.borderRadius) > 0)) fail(`D3 shape ${result.zoom * 100}% zoom: radius collapsed`);
        if (!(parseCssLengthPx(result.outlineWidth) > 0)) fail(`D3 shape ${result.zoom * 100}% zoom: outline collapsed`);
        if (result.scrollWidth > result.clientWidth + 1) fail(`D3 shape ${result.zoom * 100}% zoom: proof surface overflowed viewport`);
    }

    const logicalResults = [];
    for (const dir of ["ltr", "rtl"]) {
        await page.evaluate((nextDir) => {
            document.documentElement.dir = nextDir;
        }, dir);
        logicalResults.push(await page.evaluate((nextDir) => {
            const style = getComputedStyle(document.getElementById("shape-attached"));
            return {
                dir: nextDir,
                topLeft: style.borderTopLeftRadius,
                topRight: style.borderTopRightRadius,
                bottomLeft: style.borderBottomLeftRadius,
                bottomRight: style.borderBottomRightRadius,
            };
        }, dir));
    }
    const ltr = logicalResults.find((result) => result.dir === "ltr");
    const rtl = logicalResults.find((result) => result.dir === "rtl");
    if (!(parseCssLengthPx(ltr.topLeft) === 0 && parseCssLengthPx(ltr.bottomLeft) === 0)) {
        fail("D3 shape LTR logical attached edge did not map to the physical left edge");
    }
    if (!(parseCssLengthPx(ltr.topRight) > 0 && parseCssLengthPx(ltr.bottomRight) > 0)) {
        fail("D3 shape LTR free edge did not map to the physical right edge");
    }
    if (!(parseCssLengthPx(rtl.topRight) === 0 && parseCssLengthPx(rtl.bottomRight) === 0)) {
        fail("D3 shape RTL logical attached edge did not map to the physical right edge");
    }
    if (!(parseCssLengthPx(rtl.topLeft) > 0 && parseCssLengthPx(rtl.bottomLeft) > 0)) {
        fail("D3 shape RTL free edge did not map to the physical left edge");
    }

    await context.close();

    const forcedContext = await browser.newContext({ javaScriptEnabled: true, bypassCSP: true, forcedColors: "active" });
    const forcedPage = await forcedContext.newPage();
    await forcedPage.setContent(`
        <style>
            ${combinedCss}
            .cem-shape-proof {
                inline-size: 10rem;
                block-size: var(--cem-control-height);
                border-radius: var(--cem-bend-round);
                outline: var(--cem-stroke-focus) solid Highlight;
                outline-offset: var(--cem-stroke-indicator-offset);
                overflow: visible;
            }
        </style>
        <div id="shape-proof" class="cem-shape-proof" tabindex="0"></div>
    `);
    const forcedResult = await forcedPage.evaluate(() => {
        const style = getComputedStyle(document.getElementById("shape-proof"));
        return {
            forcedColors: matchMedia("(forced-colors: active)").matches,
            borderRadius: style.borderRadius,
            outlineStyle: style.outlineStyle,
            outlineWidth: style.outlineWidth,
            outlineOffset: style.outlineOffset,
        };
    });
    await forcedContext.close();

    if (!forcedResult.forcedColors) fail("D3 shape forced-colors context did not activate");
    if (!(parseCssLengthPx(forcedResult.borderRadius) > 0)) fail("D3 shape forced-colors radius collapsed");
    if (forcedResult.outlineStyle !== "solid") fail("D3 shape forced-colors outline style did not remain solid");
    if (!(parseCssLengthPx(forcedResult.outlineWidth) > 0)) fail("D3 shape forced-colors outline width collapsed");
    if (!(parseCssLengthPx(forcedResult.outlineOffset) >= 0)) fail("D3 shape forced-colors outline offset did not resolve");

    logOk("D3 shape browser validation green");
}

async function runAccessibilityRegressionChecks(browser, combinedCss) {
    const context = await browser.newContext({ javaScriptEnabled: true, bypassCSP: true });
    const page = await context.newPage();
    await page.setContent(`
        <style>
            ${combinedCss}
            .a11y-scope {
                background: var(--cem-palette-comfort);
                color: var(--cem-palette-comfort-text);
                padding: var(--cem-coupling-guard-min);
            }
            .a11y-action {
                min-inline-size: var(--cem-coupling-zone-min);
                min-block-size: var(--cem-coupling-zone-min);
                background: var(--cem-action-explicit-default-background);
                color: var(--cem-action-explicit-default-text);
                outline: var(--cem-stroke-focus) solid currentColor;
                outline-offset: var(--cem-stroke-indicator-offset);
            }
            .a11y-targets {
                display: flex;
                gap: max(var(--cem-gap-related, 0px), var(--cem-coupling-guard-min));
                overflow: visible;
            }
        </style>
        <main id="scope" class="a11y-scope">
            <p id="text">Readable text</p>
            <div id="targets" class="a11y-targets">
                <button id="button-a" class="a11y-action">One</button>
                <button id="button-b" class="a11y-action">Two</button>
            </div>
        </main>
    `);

    const result = await page.evaluate((themeModes) => {
        function parseRgb(value) {
            const match = value.match(/rgba?\(([^)]+)\)/);
            if (!match) return null;
            const channels = match[1].split(/[,\s/]+/).filter(Boolean).slice(0, 3).map(Number);
            return channels.length === 3 && channels.every(Number.isFinite) ? channels : null;
        }

        function luminance([r, g, b]) {
            return [r, g, b]
                .map((channel) => {
                    const srgb = channel / 255;
                    return srgb <= 0.03928 ? srgb / 12.92 : ((srgb + 0.055) / 1.055) ** 2.4;
                })
                .reduce((sum, channel, index) => sum + channel * [0.2126, 0.7152, 0.0722][index], 0);
        }

        function contrast(foreground, background) {
            const fg = parseRgb(foreground);
            const bg = parseRgb(background);
            if (!fg || !bg) return null;
            const lighter = Math.max(luminance(fg), luminance(bg));
            const darker = Math.min(luminance(fg), luminance(bg));
            return (lighter + 0.05) / (darker + 0.05);
        }

        const scope = document.getElementById("scope");
        const text = document.getElementById("text");
        const targets = document.getElementById("targets");
        const buttonA = document.getElementById("button-a");
        const buttonB = document.getElementById("button-b");
        const checks = [];

        for (const mode of themeModes.filter((name) => name !== "cem-theme-native")) {
            scope.className = `a11y-scope ${mode}`;
            scope.setAttribute("data-theme", mode);
            const scopeStyle = getComputedStyle(scope);
            const textStyle = getComputedStyle(text);
            const buttonStyle = getComputedStyle(buttonA);
            buttonA.focus();
            const buttonRect = buttonA.getBoundingClientRect();
            const buttonBRect = buttonB.getBoundingClientRect();
            checks.push({
                mode,
                textContrast: contrast(textStyle.color, scopeStyle.backgroundColor),
                actionContrast: contrast(buttonStyle.color, buttonStyle.backgroundColor),
                targetInline: buttonRect.width,
                targetBlock: buttonRect.height,
                targetGap: buttonBRect.left - buttonRect.right,
                focusOutlineWidth: Number.parseFloat(buttonStyle.outlineWidth),
                focusOutlineOffset: Number.parseFloat(buttonStyle.outlineOffset),
                targetOverflow: getComputedStyle(targets).overflow,
            });
        }
        return checks;
    }, THEME_MODES);

    for (const check of result) {
        if (!(check.textContrast >= 4.5)) fail(`${check.mode}: text contrast ${check.textContrast} is below 4.5:1`);
        if (!(check.actionContrast >= 4.5)) fail(`${check.mode}: action contrast ${check.actionContrast} is below 4.5:1`);
        if (!(check.targetInline >= 24 && check.targetBlock >= 24)) {
            fail(`${check.mode}: WCAG 2.5.8 target size below 24x24 CSS px`);
        }
        if (!(check.targetGap >= 8)) fail(`${check.mode}: target gap below CEM guard minimum`);
        if (!(check.focusOutlineWidth > 0 && check.focusOutlineOffset >= 0)) {
            fail(`${check.mode}: WCAG 2.4.11 focus outline geometry is not visible`);
        }
        if (check.targetOverflow !== "visible") fail(`${check.mode}: focus proof surface may hide focus outline`);
    }

    await context.close();
    logOk("accessibility regression smoke green for contrast, focus visibility, and target size");
}

function parseCssLengthPx(value, rootPx = 16) {
    if (value === undefined) return Number.NaN;
    const trimmed = value.trim();
    if (trimmed.endsWith("rem")) return Number.parseFloat(trimmed) * rootPx;
    if (trimmed.endsWith("px")) return Number.parseFloat(trimmed);
    if (trimmed === "0") return 0;
    return Number.NaN;
}

function parseMs(value) {
    const trimmed = value.trim();
    if (trimmed.endsWith("ms")) return Number.parseFloat(trimmed);
    if (trimmed.endsWith("s")) return Number.parseFloat(trimmed) * 1000;
    return Number.NaN;
}

async function runCrossSpecChecks(combinedCss) {
    const rootDecls = new Map();
    const declRe = /(--cem-[a-z][a-z0-9-]*)\s*:\s*([^;]+);/g;
    let match;
    while ((match = declRe.exec(combinedCss)) !== null) {
        rootDecls.set(match[1], match[2].trim());
    }

    function resolveValue(token, seen = new Set()) {
        const raw = rootDecls.get(token);
        if (!raw || seen.has(token)) return raw;
        const varMatch = raw.match(/^var\((--cem-[a-z][a-z0-9-]*)\)$/);
        if (!varMatch) return raw;
        seen.add(token);
        return resolveValue(varMatch[1], seen);
    }

    const guard = parseCssLengthPx(resolveValue("--cem-coupling-guard-min"));
    const zebraStrip = parseCssLengthPx(resolveValue("--cem-zebra-strip-size"));
    const indicatorOffset = parseCssLengthPx(resolveValue("--cem-stroke-indicator-offset"));
    const focus = parseCssLengthPx(resolveValue("--cem-stroke-focus"));
    const d5WorstOutset = Math.max(4 * zebraStrip, indicatorOffset + focus);
    if (!(guard >= d5WorstOutset)) {
        fail(`D2 guard (${guard}px) is smaller than D5 worst-case indicator outset (${d5WorstOutset}px)`);
    }

    const controlHeight = parseCssLengthPx(resolveValue("--cem-control-height"));
    let compactControlHeight = Number.NaN;
    const compactBlockRe = /:root\[data-cem-coupling="compact"\]\s*\{([\s\S]*?)\}/g;
    let compactBlockMatch;
    while ((compactBlockMatch = compactBlockRe.exec(combinedCss)) !== null) {
        const controlHeightMatch = compactBlockMatch[1].match(/--cem-control-height:\s*([^;]+);/);
        if (controlHeightMatch) {
            compactControlHeight = parseCssLengthPx(controlHeightMatch[1]);
            break;
        }
    }
    if (!(compactControlHeight > 0 && compactControlHeight <= controlHeight)) {
        fail("D3/D2 compact control-height relationship did not resolve");
    }
    if (!rootDecls.get("--cem-bend-round")?.includes("--cem-control-height")) {
        fail("D3 round-end bend does not reference D2c control height");
    }

    const readingSize = parseCssLengthPx(resolveValue("--cem-typography-size-m"));
    const lineHeight = Number.parseFloat(resolveValue("--cem-typography-line-height-reading"));
    const measure = resolveValue("--cem-typography-reading-measure-max");
    if (!(readingSize >= 16 && lineHeight >= 1.45 && measure === "65ch")) {
        fail("D1/D6 reading rhythm, line-height, and measure defaults are outside expected usable range");
    }

    if (!combinedCss.includes("@media (forced-colors: active)") || !combinedCss.includes("--cem-ring-zebra-3")) {
        fail("D5 forced-colors zebra fallback is absent from output");
    }

    if (/--cem-bend-(none|xs|sm|md|lg|xl|full)\s*:/.test(combinedCss)) {
        fail("D3 adapter-only M3 parity aliases leaked into default output");
    }
    if (/--cem-layout-(inline|block)-/.test(combinedCss)) {
        fail("D1 deprecated layout aliases leaked into default output");
    }

    logOk("cross-spec semantic checks green");
}

async function main() {
    await runMarkdownDocumentInventoryChecks();
    await runManifestAndCssChecks();
    const combinedCss = await readCombinedCss();
    await withBrowser(async (browser, baseUrl) => {
        await runGeneratorBrowserChecks(browser, baseUrl);
        await runThemeModeChecks(browser, combinedCss);
        await runForcedColorsChecks(browser, combinedCss);
        await runReducedMotionChecks(browser, combinedCss);
        await runShapeBrowserValidation(browser, combinedCss);
        await runAccessibilityRegressionChecks(browser, combinedCss);
    });
    await runCrossSpecChecks(combinedCss);
    logOk("Phase 13 verifier complete");
}

main().catch((err) => {
    console.error(`\nPhase 13 verification failed:\n${err.message}`);
    process.exit(1);
});
