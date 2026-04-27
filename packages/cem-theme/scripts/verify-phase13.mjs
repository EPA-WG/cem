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
                codeBlockCount: codeBlocks.length,
                generatedCssBytes: generatedCss.length,
                hasTokenInCss: generatedCss.includes(token),
                rootValue: getComputedStyle(document.documentElement).getPropertyValue(token).trim(),
                loaderCount: document.head.querySelectorAll("style[data-cem-css-loader]").length,
            };
        }, spec.token);

        if (result.codeBlockCount !== 1) fail(`${spec.name}: expected exactly one data-generated-css block`);
        if (result.generatedCssBytes === 0) fail(`${spec.name}: generated CSS block is empty`);
        if (!result.hasTokenInCss) fail(`${spec.name}: generated CSS block does not include ${spec.token}`);
        if (!result.rootValue) fail(`${spec.name}: ${spec.token} did not resolve on :root`);
        if (result.loaderCount === 0) fail(`${spec.name}: no cem-css-loader style was injected`);
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
    await runManifestAndCssChecks();
    const combinedCss = await readCombinedCss();
    await withBrowser(async (browser, baseUrl) => {
        await runGeneratorBrowserChecks(browser, baseUrl);
        await runThemeModeChecks(browser, combinedCss);
        await runForcedColorsChecks(browser, combinedCss);
        await runReducedMotionChecks(browser, combinedCss);
    });
    await runCrossSpecChecks(combinedCss);
    logOk("Phase 13 verifier complete");
}

main().catch((err) => {
    console.error(`\nPhase 13 verification failed:\n${err.message}`);
    process.exit(1);
});
