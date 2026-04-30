/**
 * Token export pipeline — orchestrates Phases A through E.
 *
 * Stage 1 (Phase A): Extract token records from compiled XHTML specs into an
 *   in-memory intermediate model and emit cem.tokens.intermediate.json.
 * Stage 2 (Phase B): Resolve CSS custom property values for all supported modes
 *   via headless Chromium and emit cem.tokens.resolved.json.
 * Stage 3 (Phase C): Emit canonical DTCG-compatible JSON (cem.tokens.json,
 *   cem.voice.tokens.json) and reports (cem.tokens.report.{md,json}).
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
    // css-expression before mode: color-mix(in srgb, light-dark(...) ...) is css-expression, not mode
    if (v.includes("calc(") || v.includes("color-mix(") || v.includes("env(")) return "css-expression";
    if (v.includes("light-dark(")) return "mode";
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
// Stage 3 constants
// ---------------------------------------------------------------------------

// Source tables that produce voice/audio tokens — routed to cem.voice.tokens.json
const VOICE_TABLES = new Set([
    "cem-typography-voice-ink-thickness",
    "cem-typography-voice-icon-stroke-multiplier",
    "cem-typography-voice-speech-volume",
    "cem-typography-voice-speech-rate",
    "cem-typography-voice-speech-pitch",
    "cem-typography-voice-ssml-emphasis",
]);

// Specs whose tokens are dimension-typed by default (for bare-0 handling etc.)
const DIMENSION_SPECS = new Set([
    "cem-dimension", "cem-breakpoints", "cem-coupling", "cem-controls", "cem-shape", "cem-stroke",
]);

// ---------------------------------------------------------------------------
// DTCG path and value helpers
// ---------------------------------------------------------------------------

// --cem-palette-comfort → ["cem", "palette", "comfort"]
function cssNameToDtcgPath(cssName) {
    return cssName.replace(/^--/, "").split("-");
}

// var(--cem-dim-x-small) → "{cem.dim.x.small}"
function varToDtcgRef(varExpression) {
    const m = (varExpression ?? "").match(/^var\(\s*--([a-z0-9-]+)\s*\)$/);
    if (!m) return null;
    return `{${m[1].split("-").join(".")}}`;
}

// Extract the light (first) branch from light-dark(X, Y) → X.
// Returns null if extraction is not possible.
function extractLightBranch(value) {
    const trimmed = (value ?? "").trim();
    if (!trimmed.toLowerCase().startsWith("light-dark(")) return null;
    const inner = trimmed.slice("light-dark(".length, trimmed.length - 1);
    let depth = 0;
    for (let i = 0; i < inner.length; i++) {
        if (inner[i] === "(") depth++;
        else if (inner[i] === ")") depth--;
        else if (inner[i] === "," && depth === 0) return inner.slice(0, i).trim();
    }
    return null;
}

// Infer the DTCG $type for a token based on its resolved light value and spec.
function inferDtcgType(token) {
    const spec = token.spec ?? "";
    const lightVal = (token.valueByMode?.light ?? "").trim();
    const rawVal = (token.valueRaw ?? "").trim();
    const tableId = token.sourceTable ?? "";

    // Typography tokens: classify by source table first to avoid numeric false-positives.
    // Font weight values (200, 400, 700…) and voice speech parameters both look like numbers;
    // the table id gives us the semantic intent.
    if (spec === "cem-voice-fonts-typography") {
        if (tableId === "cem-typography-fontography") return "fontFamily";
        if (tableId === "cem-typography-thickness") return "fontWeight";
        if (tableId.includes("size") || tableId.includes("line-height") ||
            tableId.includes("letter-spacing") || tableId.includes("ergonomics")) return "dimension";
        // Voice ink-thickness references the font-weight scale
        if (tableId === "cem-typography-voice-ink-thickness") return "fontWeight";
        if (tableId === "cem-typography-voice-icon-stroke-multiplier" ||
            tableId.includes("voice-speech")) return "number";
        // SSML emphasis levels are string keywords ("reduced", "strong", etc.)
        if (tableId.includes("voice-ssml")) return "string";
        return "string";
    }

    // Duration (ms, s)
    if (/^-?\d+(\.\d+)?(ms|s)$/.test(lightVal) || /^-?\d+(\.\d+)?(ms|s)$/.test(rawVal)) return "duration";

    // Bare 0 in dimension specs (e.g. --cem-bend-sharp: 0 — a dimensionless zero border-radius)
    if ((lightVal === "0" || rawVal === "0") && DIMENSION_SPECS.has(spec)) return "dimension";

    // Dimension (CSS length units)
    if (/^-?\d+(\.\d+)?(px|rem|em|%|vw|vh|ch|vmin|vmax|fr|pt)$/.test(lightVal)) return "dimension";
    if (/^-?\d+(\.\d+)?(px|rem|em|%|vw|vh|ch|vmin|vmax|fr|pt)$/.test(rawVal)) return "dimension";

    // Color (from resolved light value)
    if (
        /^#[0-9a-f]{3,8}$/i.test(lightVal) ||
        /^(rgb|rgba|hsl|hsla|oklch|lch|lab|oklab|hwb|color)\(/i.test(lightVal) ||
        lightVal.startsWith("color-mix(") ||
        lightVal.startsWith("light-dark(") ||
        CSS_SYSTEM_COLORS.has(lightVal.toLowerCase())
    ) return "color";

    // Number (pure unitless numeric — z-index, opacity multipliers)
    if (/^-?\d+(\.\d+)?$/.test(lightVal) || /^-?\d+(\.\d+)?$/.test(rawVal)) return "number";

    // Spec-based fallbacks for unresolved/alias tokens
    if (spec === "cem-colors") return "color";
    if (spec === "cem-timing") return "duration";
    if (DIMENSION_SPECS.has(spec)) return "dimension";
    if (spec === "cem-layering") return "number";

    return "string";
}

// Compute the DTCG $value for a token.
function computeDtcgValue(token) {
    const lightVal = (token.valueByMode?.light ?? "").trim();

    switch (token.valueType) {
        case "alias": {
            const ref = varToDtcgRef(token.valueRaw);
            return ref ?? lightVal;
        }
        case "mode": {
            // Extract the light branch from light-dark() when possible
            const lightBranch = extractLightBranch(lightVal);
            return lightBranch ?? lightVal;
        }
        case "platform-note":
            return (token.valueByMode?.native ?? token.valueRaw ?? "").trim();
        default:
            return lightVal || (token.valueRaw ?? "");
    }
}

function buildDtcgTokenRecord(token) {
    const $type = inferDtcgType(token);
    const $value = computeDtcgValue(token);
    const record = { $type, $value };
    if (token.description) record.$description = token.description;
    record.$extensions = {
        cem: {
            cssName: token.name,
            spec: token.spec,
            sourceTable: token.sourceTable,
            tier: token.tier,
            category: token.category,
            rawValue: token.valueRaw,
            portability: token.valueType,
            modes: token.valueByMode ?? {},
        },
    };
    return record;
}

// Build a nested DTCG token tree.  CSS name parts (split by "-") form the path.
// A node can be both a token ($value present) and a group (nested children).
function buildDtcgTree(tokens) {
    const root = {};
    for (const token of tokens) {
        const path = cssNameToDtcgPath(token.name);
        let node = root;
        for (let i = 0; i < path.length - 1; i++) {
            if (!node[path[i]]) node[path[i]] = {};
            node = node[path[i]];
        }
        const leaf = path[path.length - 1];
        const record = buildDtcgTokenRecord(token);
        if (node[leaf] && typeof node[leaf] === "object") {
            Object.assign(node[leaf], record);
        } else {
            node[leaf] = record;
        }
    }
    return root;
}

// ---------------------------------------------------------------------------
// DTCG validation
// ---------------------------------------------------------------------------

function validateDtcgTree(tree, tokenList, errors) {
    // Duplicate canonical DTCG paths
    const seenPaths = new Map();
    for (const t of tokenList) {
        const p = cssNameToDtcgPath(t.name).join(".");
        if (seenPaths.has(p)) {
            errors.push(`Duplicate DTCG path: ${p} (from ${t.name} and ${seenPaths.get(p)})`);
        } else {
            seenPaths.set(p, t.name);
        }
    }

    // Invalid DTCG shape — every leaf must have $type and $value
    function walkTree(node, nodePath) {
        for (const [key, value] of Object.entries(node)) {
            if (key.startsWith("$")) continue;
            if (typeof value !== "object" || value === null) {
                errors.push(`Invalid DTCG node at ${nodePath}.${key}: not an object`);
                continue;
            }
            if ("$value" in value) {
                if (!value.$type) errors.push(`Token ${nodePath}.${key} missing $type`);
                if (value.$value === undefined) errors.push(`Token ${nodePath}.${key} has undefined $value`);
            } else {
                walkTree(value, `${nodePath}.${key}`);
            }
        }
    }
    walkTree(tree, "");

    // Mode-completeness — every non-deprecated emitted token must have all 5 mode values
    for (const t of tokenList) {
        if (!t.valueByMode || t.tier === "deprecated") continue;
        const missing = MODES.filter((m) => (t.valueByMode[m] ?? "") === "");
        if (missing.length > 0 && missing.length < MODES.length) {
            errors.push(`Mode-completeness violation for ${t.name}: missing ${missing.join(", ")}`);
        }
    }
}

// ---------------------------------------------------------------------------
// Stage 3 — Canonical DTCG-compatible outputs
// ---------------------------------------------------------------------------

function filterByTier(tokens, opts) {
    return tokens.filter((t) => {
        if (t.tier === "required" || t.tier === "recommended") return true;
        if (t.tier === "optional" && opts.withOptional) return true;
        if (t.tier === "adapter" && opts.withAdapter) return true;
        if (t.tier === "deprecated" && opts.withDeprecated) return true;
        return false;
    });
}

async function stage3Emit(resolvedTokens, manifestBySpec, version, opts) {
    const errors = [];
    const warnings = [];

    // --- Tier filtering ---
    const allFiltered = filterByTier(resolvedTokens, opts);
    const filteredNames = new Set(allFiltered.map((t) => t.name));
    const skipped = resolvedTokens.filter((t) => !filteredNames.has(t.name));

    for (const t of skipped) {
        if (t.tier === "optional") warnings.push(`Skipped optional token: ${t.name}`);
        if (t.tier === "adapter") warnings.push(`Skipped adapter token: ${t.name}`);
        // deprecated skips are expected; suppress per-token noise
    }

    // --- Visual / voice split ---
    const voiceTokens = allFiltered.filter((t) => VOICE_TABLES.has(t.sourceTable));
    const visualTokens = allFiltered.filter((t) => !VOICE_TABLES.has(t.sourceTable));

    // Fail-hard: voice tokens must not be in visual output
    for (const t of visualTokens.filter((t2) => VOICE_TABLES.has(t2.sourceTable))) {
        errors.push(`Voice-only token found in visual output: ${t.name}`);
    }

    // Fail-hard: every required/recommended manifest token must be emitted
    const emittedNames = new Set(allFiltered.map((t) => t.name));
    for (const [specName, manifest] of Object.entries(manifestBySpec)) {
        for (const { name, tier } of manifest) {
            if ((tier === "required" || tier === "recommended") && !emittedNames.has(name)) {
                errors.push(`Required/recommended token missing from output: ${name} [${tier}] (spec: ${specName})`);
            }
        }
    }

    // --- Build DTCG trees ---
    const generated = {
        timestamp: new Date().toISOString(),
        packageVersion: version,
        sourceSpecs: SPEC_ORDER.map((s) => s.name),
        sourceBuildCommand: "node packages/cem-theme/scripts/export-tokens.mjs",
        generator: "packages/cem-theme/scripts/export-tokens.mjs",
        options: opts,
    };

    const visualTree = buildDtcgTree(visualTokens);
    const voiceTree = buildDtcgTree(voiceTokens);
    visualTree.$extensions = { cem: { generated } };
    voiceTree.$extensions = { cem: { generated } };

    // Fail-hard: provenance must be present
    if (!visualTree.$extensions?.cem?.generated) {
        errors.push("Missing $extensions.cem.generated provenance in cem.tokens.json");
    }

    // Fail-hard: validate DTCG shape and mode-completeness
    validateDtcgTree(visualTree, visualTokens, errors);
    validateDtcgTree(voiceTree, voiceTokens, errors);

    // Warn-and-report: css-expression tokens
    for (const t of visualTokens.filter((t2) => t2.valueType === "css-expression")) {
        warnings.push(`css-expression (web-only): ${t.name}`);
    }

    // Warn-and-report: platform-note tokens
    for (const t of visualTokens.filter((t2) => t2.valueType === "platform-note")) {
        warnings.push(`platform-note: ${t.name}`);
    }

    return { visualTree, voiceTree, visualTokens, voiceTokens, skipped, errors, warnings, generated };
}

// ---------------------------------------------------------------------------
// Canonical JSON and report emission
// ---------------------------------------------------------------------------

async function emitCanonicalJson(visualTree, voiceTree) {
    await fs.mkdir(DIST_TOKENS, { recursive: true });
    const visualPath = path.join(DIST_TOKENS, "cem.tokens.json");
    const voicePath = path.join(DIST_TOKENS, "cem.voice.tokens.json");
    await fs.writeFile(visualPath, JSON.stringify(visualTree, null, 2), "utf8");
    await fs.writeFile(voicePath, JSON.stringify(voiceTree, null, 2), "utf8");
    return { visualPath, voicePath };
}

async function emitReport(stageResult) {
    const { visualTokens, voiceTokens, skipped, errors, warnings, generated } = stageResult;

    const portabilityStats = {};
    for (const t of visualTokens) {
        portabilityStats[t.valueType] = (portabilityStats[t.valueType] ?? 0) + 1;
    }
    const skippedByTier = {};
    for (const t of skipped) skippedByTier[t.tier] = (skippedByTier[t.tier] ?? 0) + 1;

    const cssExprList = visualTokens.filter((t) => t.valueType === "css-expression");
    const platformNoteList = visualTokens.filter((t) => t.valueType === "platform-note");

    // --- JSON report ---
    const reportJson = {
        $generated: generated,
        summary: {
            visualTokensEmitted: visualTokens.length,
            voiceTokensEmitted: voiceTokens.length,
            skippedTotal: skipped.length,
            skippedByTier,
            errorsCount: errors.length,
            warningsCount: warnings.length,
        },
        portability: portabilityStats,
        skipped: skipped.map((t) => ({ name: t.name, tier: t.tier, spec: t.spec })),
        cssExpressionTokens: cssExprList.map((t) => ({
            name: t.name, spec: t.spec, lightValue: (t.valueByMode?.light ?? "").trim(),
        })),
        platformNoteTokens: platformNoteList.map((t) => ({
            name: t.name, spec: t.spec, nativeValue: (t.valueByMode?.native ?? "").trim(),
        })),
        errors,
        warnings,
    };

    // --- Markdown report ---
    const md = [];
    md.push("# CEM Token Export Report", "");
    md.push(`Generated: ${generated.timestamp}  `);
    md.push(`Package: ${generated.packageVersion}  `);
    md.push(`Specs: ${generated.sourceSpecs.join(", ")}`, "");
    md.push("## Summary", "");
    md.push("| Stat | Count |", "| ---- | ----- |");
    md.push(`| Visual tokens emitted | ${visualTokens.length} |`);
    md.push(`| Voice tokens emitted | ${voiceTokens.length} |`);
    md.push(`| Skipped (optional) | ${skippedByTier.optional ?? 0} |`);
    md.push(`| Skipped (adapter) | ${skippedByTier.adapter ?? 0} |`);
    md.push(`| Skipped (deprecated) | ${skippedByTier.deprecated ?? 0} |`);
    md.push(`| Errors | ${errors.length} |`, "");
    md.push("## Portability", "");
    md.push("| Portability | Count |", "| ----------- | ----- |");
    for (const [k, v] of Object.entries(portabilityStats)) md.push(`| \`${k}\` | ${v} |`);
    md.push("");
    if (cssExprList.length > 0) {
        md.push("## CSS-expression tokens (web-only)", "");
        md.push(
            "These tokens use `color-mix()`, `calc()`, or `env()` — they require a CSS runtime and cannot",
            "be used directly on non-web platforms. Their light-mode computed value is in `$value`.",
            ""
        );
        for (const t of cssExprList) md.push(`- \`${t.name}\``);
        md.push("");
    }
    if (platformNoteList.length > 0) {
        md.push("## Platform-note tokens", "");
        md.push(
            "These tokens use CSS system color keywords (`Canvas`, `ButtonFace`, etc.).",
            "The `native` mode values are Chromium-computed browser-reference values.",
            ""
        );
        for (const t of platformNoteList) md.push(`- \`${t.name}\``);
        md.push("");
    }
    if (skipped.length > 0) {
        md.push("## Skipped tokens", "");
        md.push("Pass `--with-{tier}` to include these in the output.", "");
        for (const t of skipped) md.push(`- \`${t.name}\` (tier: \`${t.tier}\`, spec: ${t.spec})`);
        md.push("");
    }
    if (errors.length > 0) {
        md.push("## Errors", "");
        for (const e of errors) md.push(`- **ERROR:** ${e}`);
        md.push("");
    }
    md.push("---", "");
    md.push("> Generated by `export-tokens.mjs`. Do not edit by hand.", "");

    await fs.mkdir(DIST_TOKENS, { recursive: true });
    const reportJsonPath = path.join(DIST_TOKENS, "cem.tokens.report.json");
    const reportMdPath = path.join(DIST_TOKENS, "cem.tokens.report.md");
    await fs.writeFile(reportJsonPath, JSON.stringify(reportJson, null, 2), "utf8");
    await fs.writeFile(reportMdPath, md.join("\n"), "utf8");
    return { reportJsonPath, reportMdPath };
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

    // Stage 3 — Canonical DTCG-compatible outputs
    console.log("export-tokens: Stage 3 — canonical DTCG JSON emission");
    const s3 = await stage3Emit(resolvedTokens, manifestBySpec, version, opts);

    // Suppress per-token css-expression/deprecated noise — summary in report
    const s3WarnShow = s3.warnings.filter((w) => !w.startsWith("css-expression") && !w.startsWith("Skipped deprecated"));
    for (const w of s3WarnShow) console.warn(`  warn: ${w}`);

    if (s3.errors.length) {
        for (const e of s3.errors) console.error(`  error: ${e}`);
        process.exit(1);
    }

    const { visualPath, voicePath } = await emitCanonicalJson(s3.visualTree, s3.voiceTree);
    const { reportJsonPath, reportMdPath } = await emitReport(s3);

    const cssExprCount = s3.visualTokens.filter((t) => t.valueType === "css-expression").length;
    console.log(`  visual: ${s3.visualTokens.length}  voice: ${s3.voiceTokens.length}  skipped: ${s3.skipped.length}  css-expression: ${cssExprCount}`);
    console.log(`  → ${path.relative(process.cwd(), visualPath)}`);
    console.log(`  → ${path.relative(process.cwd(), voicePath)}`);
    console.log(`  → ${path.relative(process.cwd(), reportMdPath)}`);
    console.log(`  → ${path.relative(process.cwd(), reportJsonPath)}`);
}

main(process.argv).catch((err) => {
    console.error(err);
    process.exit(2);
});
