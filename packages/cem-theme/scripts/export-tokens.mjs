/**
 * Token export pipeline — orchestrates Phases A through E.
 *
 * Stage 1 (Phase A): Extract token records from compiled XHTML specs into an
 *   in-memory intermediate model and emit cem.tokens.intermediate.json.
 * Stage 2 (Phase B): Resolve CSS custom property values for all supported modes
 *   via headless Chromium and emit cem.tokens.resolved.json.
 *
 * Usage:
 *   node packages/cem-theme/scripts/export-tokens.mjs [--with-optional] [--with-adapter] [--with-deprecated]
 *
 * Depends on: dist/lib/tokens/*.xhtml and dist/lib/css/*.css (produced by build:css)
 */

import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { SPEC_ORDER, deriveManifestForSpec } from "./manifest-utils.mjs";
import { deriveTokensForSpec } from "./derive-tokens.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PACKAGE_ROOT = path.resolve(__dirname, "..");
const DIST_TOKENS = path.join(PACKAGE_ROOT, "dist/lib/tokens");

const VALID_TIERS = new Set(["required", "recommended", "optional", "adapter", "deprecated"]);

// Supported color-scheme modes
const MODES = ["light", "dark", "contrast-light", "contrast-dark", "native"];

// Tokens overridden by data-cem-spacing (from cem-dimension.css parallel group rules)
const SPACING_TOKENS = new Set([
    "--cem-gap-related", "--cem-gap-group", "--cem-gap-block", "--cem-gap-section", "--cem-gap-page",
    "--cem-inset-control", "--cem-inset-container", "--cem-inset-surface",
    "--cem-layout-gutter", "--cem-layout-gutter-wide", "--cem-layout-gutter-max",
    "--cem-rhythm-reading-paragraph", "--cem-rhythm-reading-section",
    "--cem-rhythm-data-row", "--cem-rhythm-data-group",
]);

// Tokens overridden by data-cem-coupling (from cem-coupling.css parallel group rules)
const COUPLING_TOKENS = new Set(["--cem-coupling-halo"]);

// Tokens overridden by data-cem-shape (from cem-shape.css parallel group rules)
const SHAPE_TOKENS = new Set(["--cem-bend", "--cem-bend-control", "--cem-bend-surface", "--cem-bend-overlay"]);

// CSS system color keywords — resolved values containing these are platform-specific
const CSS_SYSTEM_COLORS = new Set([
    "canvas", "canvastext", "linktext", "visitedtext", "activetext",
    "buttonface", "buttontext", "buttonborder", "field", "fieldtext",
    "highlight", "highlighttext", "selecteditem", "selecteditemtext",
    "mark", "marktext", "graytext", "accentcolor", "accentcolortext",
]);

// MIME types for the in-process HTTP server
const MIME_TYPES = {
    ".html": "text/html",
    ".css": "text/css",
    ".mjs": "application/javascript",
    ".js": "application/javascript",
    ".json": "application/json",
};

async function readPackageVersion() {
    const pkgPath = path.join(PACKAGE_ROOT, "package.json");
    try {
        const pkg = JSON.parse(await fs.readFile(pkgPath, "utf8"));
        return pkg.version ?? "0.0.0";
    } catch {
        return "0.0.0";
    }
}

// ---------------------------------------------------------------------------
// Stage 1 — Token extraction
// ---------------------------------------------------------------------------

async function stage1Extract(opts) {
    const { withOptional, withAdapter, withDeprecated } = opts;
    const allTokens = [];
    const allWarnings = [];

    for (const { name: specName } of SPEC_ORDER) {
        const xhtmlPath = path.join(DIST_TOKENS, `${specName}.xhtml`);
        let xhtml;
        try {
            xhtml = await fs.readFile(xhtmlPath, "utf8");
        } catch {
            allWarnings.push(`[${specName}] XHTML not found: ${xhtmlPath} — run build:css first`);
            continue;
        }

        const { tokens, warnings } = deriveTokensForSpec(specName, xhtml);
        for (const w of warnings) allWarnings.push(`[${specName}] ${w}`);
        allTokens.push(...tokens);
    }

    return { tokens: allTokens, warnings: allWarnings };
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

function validateIntermediate(tokens, manifestBySpec, warnings) {
    const errors = [];

    // No duplicate CSS token names
    const seen = new Map();
    for (const t of tokens) {
        if (seen.has(t.name)) {
            errors.push(`Duplicate token name: ${t.name} (from ${t.spec}/${t.sourceTable} and ${seen.get(t.name)})`);
        } else {
            seen.set(t.name, `${t.spec}/${t.sourceTable}`);
        }
    }

    // No unknown tier values
    for (const t of tokens) {
        if (!VALID_TIERS.has(t.tier)) {
            errors.push(`Invalid tier "${t.tier}" on token ${t.name} (spec: ${t.spec})`);
        }
    }

    // Every token has source spec and sourceTable
    for (const t of tokens) {
        if (!t.spec || !t.sourceTable) {
            errors.push(`Token ${t.name} missing spec or sourceTable`);
        }
    }

    // Every manifest-derived token must appear in the intermediate model
    const tokenNames = new Set(tokens.map((t) => t.name));
    for (const [specName, manifest] of Object.entries(manifestBySpec)) {
        for (const { name, tier } of manifest) {
            if (!tokenNames.has(name)) {
                errors.push(`Manifest token missing from intermediate: ${name} [${tier}] (spec: ${specName})`);
            }
        }
    }

    return errors;
}

// ---------------------------------------------------------------------------
// Intermediate JSON emission
// ---------------------------------------------------------------------------

async function emitIntermediate(tokens, warnings, errors, version, opts) {
    const outPath = path.join(DIST_TOKENS, "cem.tokens.intermediate.json");
    await fs.mkdir(DIST_TOKENS, { recursive: true });

    const output = {
        $debug: true,
        $contract: "non-contract debug output — do not consume in production",
        $generated: {
            timestamp: new Date().toISOString(),
            packageVersion: version,
            sourceSpecs: SPEC_ORDER.map((s) => s.name),
            generator: "packages/cem-theme/scripts/export-tokens.mjs",
            stage: "1-extract",
            options: opts,
        },
        stats: {
            total: tokens.length,
            byTier: Object.fromEntries(
                [...VALID_TIERS].map((tier) => [tier, tokens.filter((t) => t.tier === tier).length])
            ),
            bySpec: Object.fromEntries(
                SPEC_ORDER.map(({ name }) => [name, tokens.filter((t) => t.spec === name).length])
            ),
        },
        warnings,
        errors,
        tokens: tokens.map(({ row: _row, ...rest }) => rest),
    };

    await fs.writeFile(outPath, JSON.stringify(output, null, 2), "utf8");
    return outPath;
}

// ---------------------------------------------------------------------------
// Value-type classification
// ---------------------------------------------------------------------------

// Classify a token's primary value type.
//   valueRaw:      raw spec-table value (used to detect alias references)
//   resolvedLight: CSS computed value in light mode after var() substitution
//                  (used to detect mode/expression/platform-note types)
function classifyValueType(valueRaw, resolvedLight) {
    // Alias: raw spec value is a token reference — resolved value has been substituted away
    const rawV = (valueRaw ?? "").trim().toLowerCase();
    if (rawV.startsWith("var(--cem-") || rawV.startsWith("var( --cem-")) return "alias";

    // Use the CSS-level computed value for all other classifications
    const v = (resolvedLight ?? valueRaw ?? "").trim().toLowerCase();
    if (!v) return "literal";
    if (CSS_SYSTEM_COLORS.has(v)) return "platform-note";
    if (v.includes("light-dark(")) return "mode";
    if (v.includes("calc(") || v.includes("color-mix(") || v.includes("env(")) return "css-expression";
    return "literal";
}

// ---------------------------------------------------------------------------
// Stage 2 — CSS-backed value resolution
// ---------------------------------------------------------------------------

async function stage2Resolve(tokens) {
    const warnings = [];
    const tokenNames = tokens.map((t) => t.name);
    const CSS_ABS_PATH = path.join(PACKAGE_ROOT, "dist/lib/css/cem.css");
    const docRoot = path.parse(process.cwd()).root;

    // Fixture HTML: minimal page that loads the full CEM stylesheet
    const FIXTURE_URL_PATH = "/__cem-token-fixture.html";
    const fixtureHtml = [
        "<!DOCTYPE html>",
        "<html>",
        "<head>",
        '  <meta charset="utf-8">',
        `  <link rel="stylesheet" href="${CSS_ABS_PATH}">`,
        "</head>",
        "<body></body>",
        "</html>",
    ].join("\n");

    // HTTP server: serves the in-memory fixture + all files from filesystem root.
    // Uses filesystem root as doc-root so @import paths in cem.css resolve correctly.
    const server = http.createServer(async (req, res) => {
        const urlPath = decodeURIComponent(req.url.split("?")[0]);
        if (urlPath === FIXTURE_URL_PATH) {
            res.writeHead(200, { "Content-Type": "text/html" });
            res.end(fixtureHtml);
            return;
        }
        const filePath = path.join(docRoot, urlPath);
        try {
            const data = await fs.readFile(filePath);
            const ext = path.extname(filePath).toLowerCase();
            res.writeHead(200, { "Content-Type": MIME_TYPES[ext] || "application/octet-stream" });
            res.end(data);
        } catch {
            res.writeHead(404);
            res.end("Not found");
        }
    });

    await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
    const { port } = server.address();

    const { chromium } = await import("playwright");
    const browser = await chromium.launch({ headless: true });
    const context = await browser.newContext({ javaScriptEnabled: true, bypassCSP: true });
    const page = await context.newPage();
    page.on("console", (msg) => console.warn(`  [browser] ${msg.text()}`));
    page.on("pageerror", (err) => console.warn(`  [page error] ${err.message}`));

    try {
        await page.goto(`http://127.0.0.1:${port}${FIXTURE_URL_PATH}`, { waitUntil: "networkidle" });
        await page.waitForTimeout(500);

        // Read all token values under a given mode class and optional data attributes.
        //
        // The CEM CSS uses CSS nesting: theme selectors (.cem-theme-light etc.) are
        // nested inside :root {}, so they apply to descendants of :root, not :root
        // itself. The theme class goes on <body>; data attributes go on <html> (:root).
        // Values are read from <body> so that:
        //   - color-scheme on <body>.cem-theme-* resolves light-dark() correctly
        //   - action/palette tokens declared on the themed element are accessible
        //   - :root-level dimension/shape tokens are inherited by <body>
        //
        // Data attributes not in dataAttrs are cleared to avoid cross-pass contamination.
        const readValues = (modeClass, dataAttrs) =>
            page.evaluate(
                ({ names, modeClass, dataAttrs }) => {
                    const root = document.documentElement;
                    const themed = document.body;
                    // Data attributes on :root (dimension, coupling, shape parallels)
                    root.removeAttribute("data-cem-spacing");
                    root.removeAttribute("data-cem-coupling");
                    root.removeAttribute("data-cem-shape");
                    for (const [attr, val] of Object.entries(dataAttrs)) root.setAttribute(attr, val);
                    // Theme class on the descendant element so nested CSS rules fire
                    themed.className = modeClass;
                    const cs = getComputedStyle(themed);
                    return Object.fromEntries(names.map((n) => [n, cs.getPropertyValue(n).trim()]));
                },
                { names: tokenNames, modeClass, dataAttrs }
            );

        // --- Base mode passes: one per color-scheme mode ---
        const modeResults = {};
        for (const mode of MODES) {
            console.log(`  resolving mode: ${mode}`);
            modeResults[mode] = await readValues(`cem-theme-${mode}`, {});
        }

        // --- Parallel group variant passes (light mode as reference base) ---
        console.log("  resolving spacing/coupling/shape variants");
        const spacingDense      = await readValues("cem-theme-light", { "data-cem-spacing": "dense" });
        const spacingSparse     = await readValues("cem-theme-light", { "data-cem-spacing": "sparse" });
        const couplingForgiving = await readValues("cem-theme-light", { "data-cem-coupling": "forgiving" });
        const couplingCompact   = await readValues("cem-theme-light", { "data-cem-coupling": "compact" });
        const shapeSharp        = await readValues("cem-theme-light", { "data-cem-shape": "sharp" });
        const shapeRound        = await readValues("cem-theme-light", { "data-cem-shape": "round" });

        // --- Enrich token records ---
        const enriched = tokens.map((t) => {
            const valueByMode = {};
            for (const mode of MODES) {
                valueByMode[mode] = modeResults[mode][t.name] ?? "";
            }
            // Append parallel group variant keys only for affected tokens
            if (SPACING_TOKENS.has(t.name)) {
                valueByMode["spacing-dense"]  = spacingDense[t.name]  ?? "";
                valueByMode["spacing-sparse"] = spacingSparse[t.name] ?? "";
            }
            if (COUPLING_TOKENS.has(t.name)) {
                valueByMode["coupling-forgiving"] = couplingForgiving[t.name] ?? "";
                valueByMode["coupling-compact"]   = couplingCompact[t.name]   ?? "";
            }
            if (SHAPE_TOKENS.has(t.name)) {
                valueByMode["shape-sharp"] = shapeSharp[t.name] ?? "";
                valueByMode["shape-round"] = shapeRound[t.name] ?? "";
            }

            if (valueByMode["light"] === "") {
                const msg = `Token not resolved in CSS: ${t.name} (spec: ${t.spec}, tier: ${t.tier})`;
                warnings.push(msg);
            }

            return { ...t, valueByMode, valueType: classifyValueType(t.valueRaw, valueByMode["light"]) };
        });

        // --- 5-token spot-check against known tokens ---
        const SPOT_CHECKS = [
            { name: "--cem-palette-comfort",  mode: "light" },
            { name: "--cem-gap-related",      mode: "light" },
            { name: "--cem-duration-instant", mode: "light" },
            { name: "--cem-bend",             mode: "light" },
            { name: "--cem-control-height",   mode: "light" },
        ];
        for (const { name, mode } of SPOT_CHECKS) {
            const val = modeResults[mode]?.[name] ?? "";
            if (val === "") warnings.push(`Spot-check failed: ${name} resolved to empty in ${mode} mode`);
        }

        return { tokens: enriched, warnings };
    } finally {
        await browser.close();
        await new Promise((resolve) => server.close(resolve));
    }
}

// ---------------------------------------------------------------------------
// Resolved JSON emission
// ---------------------------------------------------------------------------

async function emitResolved(tokens, warnings, version, opts) {
    const outPath = path.join(DIST_TOKENS, "cem.tokens.resolved.json");
    await fs.mkdir(DIST_TOKENS, { recursive: true });

    const byValueType = {};
    let unresolvedCount = 0;
    for (const t of tokens) {
        byValueType[t.valueType] = (byValueType[t.valueType] ?? 0) + 1;
        if ((t.valueByMode?.light ?? "") === "") unresolvedCount++;
    }

    const output = {
        $debug: true,
        $contract: "non-contract debug output — do not consume in production",
        $generated: {
            timestamp: new Date().toISOString(),
            packageVersion: version,
            sourceSpecs: SPEC_ORDER.map((s) => s.name),
            generator: "packages/cem-theme/scripts/export-tokens.mjs",
            stage: "2-resolve",
            options: opts,
            nativeModeCaveat:
                "native mode values are Chromium-computed browser-reference system colors " +
                "(Canvas, ButtonFace, Highlight, etc.) and are not iOS/Android system color equivalents",
        },
        stats: {
            total: tokens.length,
            byValueType,
            unresolvedCount,
        },
        warnings,
        tokens: tokens.map(({ row: _row, ...rest }) => rest),
    };

    await fs.writeFile(outPath, JSON.stringify(output, null, 2), "utf8");
    return outPath;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main(argv) {
    const args = argv.slice(2);
    const opts = {
        withOptional: args.includes("--with-optional"),
        withAdapter: args.includes("--with-adapter"),
        withDeprecated: args.includes("--with-deprecated"),
    };

    const version = await readPackageVersion();

    console.log("export-tokens: Stage 1 — extraction");

    // Build manifest map for cross-validation
    const manifestBySpec = {};
    for (const { name: specName } of SPEC_ORDER) {
        const xhtmlPath = path.join(DIST_TOKENS, `${specName}.xhtml`);
        try {
            const xhtml = await fs.readFile(xhtmlPath, "utf8");
            const { tokens } = deriveManifestForSpec(specName, xhtml);
            manifestBySpec[specName] = tokens;
        } catch {
            // warned during stage1Extract
        }
    }

    const { tokens, warnings } = await stage1Extract(opts);

    if (warnings.length) {
        for (const w of warnings) console.warn(`  warn: ${w}`);
    }

    const errors = validateIntermediate(tokens, manifestBySpec, warnings);

    const intermediatePath = await emitIntermediate(tokens, warnings, errors, version, opts);
    const intermediateRel = path.relative(process.cwd(), intermediatePath);

    console.log(`  extracted ${tokens.length} tokens from ${SPEC_ORDER.length} specs`);
    console.log(`  warnings: ${warnings.length}  errors: ${errors.length}`);
    console.log(`  → ${intermediateRel}`);

    if (errors.length) {
        for (const e of errors) console.error(`  error: ${e}`);
        process.exit(1);
    }

    // Stage 2 — CSS-backed value resolution
    console.log("export-tokens: Stage 2 — CSS-backed value resolution");
    const { tokens: resolvedTokens, warnings: resolveWarnings } = await stage2Resolve(tokens);

    if (resolveWarnings.length) {
        for (const w of resolveWarnings) console.warn(`  warn: ${w}`);
    }

    const resolvedPath = await emitResolved(resolvedTokens, resolveWarnings, version, opts);
    const resolvedRel = path.relative(process.cwd(), resolvedPath);

    // Deprecated tokens that are absent from CSS are expected; only non-deprecated failures are hard errors.
    const unresolvedAll = resolvedTokens.filter((t) => (t.valueByMode?.light ?? "") === "");
    const unresolvedHard = unresolvedAll.filter((t) => t.tier !== "deprecated");
    const unresolvedDeprecated = unresolvedAll.length - unresolvedHard.length;

    console.log(`  resolved ${resolvedTokens.length} tokens across ${MODES.length} modes`);
    console.log(`  unresolved: ${unresolvedHard.length} (+ ${unresolvedDeprecated} deprecated, expected)`);
    console.log(`  warnings: ${resolveWarnings.length}`);
    console.log(`  → ${resolvedRel}`);

    if (unresolvedHard.length > 0) {
        for (const t of unresolvedHard) console.error(`  error: not resolved in CSS: ${t.name} (${t.tier})`);
        console.error(`  ${unresolvedHard.length} token(s) not resolved — run build:css first`);
        process.exit(1);
    }
}

main(process.argv).catch((err) => {
    console.error(err);
    process.exit(2);
});
