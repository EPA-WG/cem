/**
 * Token export pipeline — orchestrates Phases A through E.
 *
 * Stage 1 (Phase A): Extract token records from compiled XHTML specs into an
 *   in-memory intermediate model and emit cem.tokens.intermediate.json.
 * Stage 2 (Phase B): Resolve CSS custom property values for all supported modes
 *   via headless Chromium and emit cem.tokens.resolved.json.
 * Stage 3 (Phase C): Emit canonical DTCG-compatible JSON (cem.tokens.json,
 *   cem.voice.tokens.json) and reports (cem.tokens.report.{md,json}).
 * Stage 4 (Phase D): Emit Figma/Tokens Studio mode files and the Figma report.
 * Stage 5 (Phase E): Emit TypeScript token metadata.
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

async function stage1Extract(_opts) {
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

function validateIntermediate(tokens, manifestBySpec, _warnings) {
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

function varToCssName(varExpression) {
    const m = (varExpression ?? "").match(/^var\(\s*(--[a-z0-9-]+)\s*\)$/);
    return m?.[1] ?? null;
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
// Stage 4 helpers — Figma value computation
// ---------------------------------------------------------------------------

function rgbStringToHex(rgb) {
    const m3 = rgb.match(/^rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)$/i);
    if (m3) {
        return "#" + [m3[1], m3[2], m3[3]].map((n) => Number(n).toString(16).padStart(2, "0")).join("");
    }
    const m4 = rgb.match(/^rgba\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*([0-9.]+)\s*\)$/i);
    if (m4) {
        const alpha = Math.round(parseFloat(m4[4]) * 255);
        return (
            "#" +
            [m4[1], m4[2], m4[3]].map((n) => Number(n).toString(16).padStart(2, "0")).join("") +
            alpha.toString(16).padStart(2, "0")
        );
    }
    return null;
}

// Extract the dark (second) branch from "light-dark(X, Y)"
function extractDarkBranch(value) {
    const trimmed = (value ?? "").trim();
    if (!trimmed.toLowerCase().startsWith("light-dark(")) return null;
    const inner = trimmed.slice("light-dark(".length, trimmed.length - 1);
    let depth = 0;
    for (let i = 0; i < inner.length; i++) {
        if (inner[i] === "(") depth++;
        else if (inner[i] === ")") depth--;
        else if (inner[i] === "," && depth === 0) return inner.slice(i + 1).trim();
    }
    return null;
}

// Convert a resolved CSS value to a Figma-compatible string.
// Returns null if the value cannot be expressed in Figma format.
function convertToFigmaValue(value, dtcgType) {
    const v = (value ?? "").trim();
    if (!v) return null;

    switch (dtcgType) {
        case "color": {
            if (/^#[0-9a-f]{3,8}$/i.test(v)) return v.toLowerCase();
            if (/^rgba?\(/i.test(v)) return rgbStringToHex(v);
            if (/^hsl\(/i.test(v)) return v;
            return null; // oklch, color-mix, light-dark, system colors
        }
        case "dimension": {
            if (v === "0" || v === "0px") return "0px";
            if (v.endsWith("px")) return v;
            if (v.endsWith("rem")) return `${Math.round(parseFloat(v) * 16 * 100) / 100}px`;
            return null; // %, vw, vh, calc() without prior resolution
        }
        case "duration": {
            if (v.endsWith("ms")) return `${parseFloat(v) / 1000}s`;
            if (/^\d+(\.\d+)?s$/.test(v)) return v;
            return null;
        }
        case "fontFamily":
            // Figma expects a single family name — take the first from a CSS font stack
            return v.split(",")[0].trim().replace(/^["']|["']$/g, "");
        case "fontWeight":
        case "number":
            return /^-?\d+(\.\d+)?$/.test(v) ? Number(v) : null;
        default:
            return v;
    }
}

function figmaTypeForDtcgType(dtcgType) {
    if (dtcgType === "fontWeight") return "number";
    return dtcgType;
}

// Compute the Figma-compatible value for a token in a specific mode.
// figmaResolved: { [mode]: { [cssName]: resolvedValue } } from the browser pass.
function computeFigmaValueForMode(token, mode, dtcgType, figmaResolved) {
    const portability = token.valueType;

    if (portability === "platform-note") return null;

    if (portability === "alias") {
        // DTCG reference is mode-independent; Tokens Studio resolves per mode file
        return varToDtcgRef(token.valueRaw);
    }

    if (portability === "css-expression") {
        const resolved = figmaResolved?.[mode]?.[token.name] ?? "";
        if (!resolved) return null;
        return convertToFigmaValue(resolved, dtcgType);
    }

    if (portability === "mode") {
        if (mode === "native") {
            const resolved = figmaResolved?.native?.[token.name] ?? "";
            if (!resolved) return null;
            return convertToFigmaValue(resolved, dtcgType);
        }
        // Non-native modes: parse light-dark() branches directly
        const modeVal = token.valueByMode?.light ?? "";
        const branch =
            mode === "light" || mode === "contrast-light"
                ? extractLightBranch(modeVal)
                : extractDarkBranch(modeVal);
        return branch ? convertToFigmaValue(branch, dtcgType) : null;
    }

    // literal
    const raw = token.valueByMode?.[mode] ?? token.valueByMode?.light ?? "";
    return convertToFigmaValue(raw, dtcgType);
}

function computeConcreteFigmaValueForMode(token, mode, dtcgType, figmaResolved) {
    const resolved = figmaResolved?.[mode]?.[token.name] ?? token.valueByMode?.[mode] ?? token.valueByMode?.light ?? "";
    return convertToFigmaValue(resolved, dtcgType);
}

// ---------------------------------------------------------------------------
// Stage 4 — browser resolution pass for Figma
// ---------------------------------------------------------------------------

// Resolve css-expression tokens (all modes) and mode tokens (native mode) via
// element computed styles.  Returns { [mode]: { [cssName]: resolvedValue } }.
async function resolveFigmaExpressions(tokens) {
    const cssColorNames = tokens
        .filter((t) => t.valueType === "css-expression" && inferDtcgType(t) === "color")
        .map((t) => t.name);
    const cssDimNames = tokens
        .filter((t) => t.valueType === "css-expression" && inferDtcgType(t) === "dimension")
        .map((t) => t.name);
    const modeColorNames = tokens
        .filter((t) => t.valueType === "mode" && inferDtcgType(t) === "color")
        .map((t) => t.name);
    const modeDimNames = tokens
        .filter((t) => t.valueType === "mode" && inferDtcgType(t) === "dimension")
        .map((t) => t.name);

    const CSS_ABS_PATH = path.join(PACKAGE_ROOT, "dist/lib/css/cem.css");
    const docRoot = path.parse(process.cwd()).root;
    const FIXTURE_URL_PATH = "/__cem-figma-fixture.html";
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
    const context = await browser.newContext({ bypassCSP: true });
    const page = await context.newPage();

    try {
        await page.goto(`http://127.0.0.1:${port}${FIXTURE_URL_PATH}`, { waitUntil: "networkidle" });
        await page.waitForTimeout(500);

        const results = {};
        for (const mode of MODES) {
            const modeClass = `cem-theme-${mode}`;
            // For native mode, also resolve mode tokens (their native value is a system color)
            const colorNames = mode === "native"
                ? [...new Set([...cssColorNames, ...modeColorNames])]
                : cssColorNames;
            const dimNames = mode === "native"
                ? [...new Set([...cssDimNames, ...modeDimNames])]
                : cssDimNames;

            if (colorNames.length === 0 && dimNames.length === 0) {
                results[mode] = {};
                continue;
            }

            // Assign var(--token) to an element's CSS property so the browser fully
            // evaluates light-dark(), color-mix(), calc() etc. in context.
            results[mode] = await page.evaluate(
                ({ colorNames, dimNames, modeClass }) => {
                    document.body.className = modeClass;
                    const result = {};
                    for (const name of colorNames) {
                        const el = document.createElement("div");
                        document.body.appendChild(el);
                        el.style.backgroundColor = `var(${name})`;
                        result[name] = getComputedStyle(el).backgroundColor;
                        el.remove();
                    }
                    for (const name of dimNames) {
                        const el = document.createElement("div");
                        document.body.appendChild(el);
                        el.style.width = `var(${name})`;
                        result[name] = getComputedStyle(el).width;
                        el.remove();
                    }
                    return result;
                },
                { colorNames, dimNames, modeClass }
            );
        }

        return results;
    } finally {
        await browser.close();
        await new Promise((resolve) => server.close(resolve));
    }
}

// ---------------------------------------------------------------------------
// Stage 4 — DTCG mode tree builder and validator
// ---------------------------------------------------------------------------

function buildFigmaModeTree(tokens, tokenFigmaValues, mode, generated) {
    const root = { $extensions: { cem: { generated } } };
    for (const token of tokens) {
        const fv = tokenFigmaValues.get(token.name);
        if (!fv) continue;
        const $value = fv.modeValues[mode];
        if ($value === null || $value === undefined) continue;

        const pathParts = cssNameToDtcgPath(token.name);
        let node = root;
        for (let i = 0; i < pathParts.length - 1; i++) {
            if (!node[pathParts[i]]) node[pathParts[i]] = {};
            node = node[pathParts[i]];
        }
        const leaf = pathParts[pathParts.length - 1];
        const record = { $type: fv.dtcgType, $value };
        if (token.description) record.$description = token.description;
        record.$extensions = { cem: { cssName: token.name, tier: token.tier, portability: token.valueType } };
        if (node[leaf] && typeof node[leaf] === "object") {
            Object.assign(node[leaf], record);
        } else {
            node[leaf] = record;
        }
    }
    return root;
}

// Returns Map<dotPath, { type }> for all leaf tokens in a DTCG tree
function getLeafTokenPaths(tree) {
    const result = new Map();
    function walk(node, prefix) {
        for (const [key, value] of Object.entries(node)) {
            if (key.startsWith("$")) continue;
            if (typeof value !== "object" || value === null) continue;
            const p = prefix ? `${prefix}.${key}` : key;
            if ("$value" in value) result.set(p, { type: value.$type });
            walk(value, p);
        }
    }
    walk(tree, "");
    return result;
}

// ---------------------------------------------------------------------------
// Stage 4 — orchestration
// ---------------------------------------------------------------------------

async function stage4Figma(resolvedTokens, version, opts) {
    const errors = [];
    const warnings = [];

    const filtered = filterByTier(resolvedTokens, opts).filter((t) => !VOICE_TABLES.has(t.sourceTable));

    // Identify tokens needing Playwright resolution
    const needsBrowser = filtered.filter(
        (t) => t.valueType === "css-expression" || t.valueType === "mode"
    );
    const figmaResolved = await resolveFigmaExpressions(needsBrowser);

    // Compute per-mode Figma values for all tokens
    const tokenFigmaValues = new Map();
    for (const t of filtered) {
        const dtcgType = inferDtcgType(t);
        const figmaType = figmaTypeForDtcgType(dtcgType);
        const modeValues = {};
        let allValid = true;
        for (const mode of MODES) {
            const val = computeFigmaValueForMode(t, mode, dtcgType, figmaResolved);
            modeValues[mode] = val;
            if (val === null) allValid = false;
        }
        tokenFigmaValues.set(t.name, {
            modeValues,
            dtcgType: figmaType,
            canonicalType: dtcgType,
            allValid,
            aliasTarget: t.valueType === "alias" ? varToCssName(t.valueRaw) : null,
            aliasResolvedToConcrete: false,
        });
    }

    // Keep aliases where their target is also present in the same Figma collection.
    // If an alias points to a token excluded for Figma, fall back to concrete values
    // so the alias token can still be represented without a broken reference.
    const candidateNames = new Set(
        filtered.filter((t) => tokenFigmaValues.get(t.name).allValid).map((t) => t.name)
    );
    for (const t of filtered.filter((token) => token.valueType === "alias")) {
        const fv = tokenFigmaValues.get(t.name);
        if (!fv?.allValid || !fv.aliasTarget || candidateNames.has(fv.aliasTarget)) continue;

        let allConcrete = true;
        const modeValues = {};
        for (const mode of MODES) {
            const concrete = computeConcreteFigmaValueForMode(t, mode, fv.canonicalType, figmaResolved);
            modeValues[mode] = concrete;
            if (concrete === null) allConcrete = false;
        }
        fv.modeValues = modeValues;
        fv.allValid = allConcrete;
        fv.aliasResolvedToConcrete = allConcrete;
        if (allConcrete) {
            warnings.push(`Figma alias resolved to concrete values because target is excluded: ${t.name} → ${fv.aliasTarget}`);
        }
    }

    const included = filtered.filter((t) => tokenFigmaValues.get(t.name).allValid);
    const excluded = filtered.filter((t) => !tokenFigmaValues.get(t.name).allValid).map((t) => ({
        ...t,
        missingModes: MODES.filter((m) => tokenFigmaValues.get(t.name).modeValues[m] === null),
    }));

    for (const t of excluded) {
        warnings.push(`Excluded from Figma: ${t.name} [${t.valueType}] — missing: ${t.missingModes.join(", ")}`);
    }

    // Validate duplicate slash-normalized names
    const slashSeen = new Set();
    for (const t of included) {
        const slash = cssNameToDtcgPath(t.name).join("/");
        if (slashSeen.has(slash)) errors.push(`Duplicate Figma path: ${slash} (${t.name})`);
        slashSeen.add(slash);
    }

    const generated = {
        timestamp: new Date().toISOString(),
        packageVersion: version,
        sourceSpecs: SPEC_ORDER.map((s) => s.name),
        sourceBuildCommand: "node packages/cem-theme/scripts/export-tokens.mjs",
        generator: "packages/cem-theme/scripts/export-tokens.mjs",
        options: opts,
        workflow: "Tokens Studio pull-only into one CEM collection; write-back disabled",
    };

    const modeFiles = {};
    for (const mode of MODES) {
        modeFiles[mode] = buildFigmaModeTree(included, tokenFigmaValues, mode, { ...generated, mode });
    }

    // Validate cross-mode consistency: all files must have same token paths and $types
    const firstPaths = getLeafTokenPaths(modeFiles[MODES[0]]);
    for (const mode of MODES.slice(1)) {
        const modePaths = getLeafTokenPaths(modeFiles[mode]);
        for (const [p, info] of firstPaths) {
            if (!modePaths.has(p)) {
                errors.push(`Figma mode consistency: ${p} in ${MODES[0]} but missing from ${mode}`);
            } else if (modePaths.get(p).type !== info.type) {
                errors.push(`Figma mode consistency: ${p} type ${info.type} in ${MODES[0]} vs ${modePaths.get(p).type} in ${mode}`);
            }
        }
        for (const p of modePaths.keys()) {
            if (!firstPaths.has(p)) {
                errors.push(`Figma mode consistency: ${p} in ${mode} but missing from ${MODES[0]}`);
            }
        }
    }

    return { modeFiles, included, excluded, errors, warnings, generated, tokenFigmaValues };
}

// ---------------------------------------------------------------------------
// Figma file emission
// ---------------------------------------------------------------------------

async function emitFigmaFiles(result) {
    const { modeFiles, included, excluded, errors, warnings, generated, tokenFigmaValues } = result;
    const figmaDir = path.join(DIST_TOKENS, "figma");
    await fs.mkdir(figmaDir, { recursive: true });

    const modePaths = {};
    for (const mode of MODES) {
        const filePath = path.join(figmaDir, `cem-${mode}.tokens.json`);
        await fs.writeFile(filePath, JSON.stringify(modeFiles[mode], null, 2), "utf8");
        modePaths[mode] = filePath;
    }

    // Markdown report
    const md = [];
    md.push("# CEM Figma Token Report", "");
    md.push(`Generated: ${generated.timestamp}  `);
    md.push(`Package: ${generated.packageVersion}  `, "");
    md.push("## Summary", "");
    md.push("| Stat | Count |", "| ---- | ----- |");
    md.push(`| Tokens in all mode files | ${included.length} |`);
    md.push(`| Excluded (incomplete modes) | ${excluded.length} |`);
    md.push(`| Aliases resolved to concrete values | ${included.filter((t) => tokenFigmaValues.get(t.name)?.aliasResolvedToConcrete).length} |`);
    md.push(`| Warnings | ${warnings.length} |`);
    md.push(`| Errors | ${errors.length} |`, "");
    md.push("## Mode files", "");
    for (const mode of MODES) {
        md.push(`- \`figma/cem-${mode}.tokens.json\` — ${mode} mode`);
    }
    md.push("");

    if (excluded.length > 0) {
        md.push("## Excluded tokens", "");
        md.push(
            "Tokens with no valid Figma value for at least one mode are excluded from all mode files.",
            "Use the canonical `cem.tokens.json` for cross-platform consumption.",
            ""
        );
        for (const t of excluded) {
            md.push(`- \`${t.name}\` [${t.valueType}] — missing: ${t.missingModes.join(", ")}`);
        }
        md.push("");
    }

    const concreteAliases = included.filter((t) => tokenFigmaValues.get(t.name)?.aliasResolvedToConcrete);
    if (concreteAliases.length > 0) {
        md.push("## Aliases resolved to concrete values", "");
        md.push("These aliases point to tokens excluded from the Figma collection, so their per-mode concrete values are emitted instead.", "");
        for (const t of concreteAliases) {
            const fv = tokenFigmaValues.get(t.name);
            md.push(`- \`${t.name}\` → \`${fv.aliasTarget}\``);
        }
        md.push("");
    }

    if (warnings.length > 0) {
        md.push("## Warnings", "");
        for (const w of warnings) md.push(`- ${w}`);
        md.push("");
    }

    md.push("## Tokens Studio setup", "");
    md.push("1. Install the **Tokens Studio** Figma plugin.");
    md.push("2. Create a token project or collection named **CEM**.");
    md.push(
        "3. Configure sync to pull the generated files from `dist/lib/tokens/figma/` as read-only source files."
    );
    md.push("4. Import each mode file as a separate theme/mode: light, dark, contrast-light, contrast-dark, native.");
    md.push("5. Keep push/write-back disabled; markdown token specs remain the source of truth.");
    md.push("6. The token names and types are identical across all mode files.", "");

    if (errors.length > 0) {
        md.push("## Errors", "");
        for (const e of errors) md.push(`- **ERROR:** ${e}`);
        md.push("");
    }

    md.push("---", "");
    md.push("> Generated by `export-tokens.mjs`. Do not edit by hand.", "");

    const reportPath = path.join(figmaDir, "cem-figma-report.md");
    await fs.writeFile(reportPath, md.join("\n"), "utf8");

    return { modePaths, reportPath };
}

// ---------------------------------------------------------------------------
// TypeScript metadata emission
// ---------------------------------------------------------------------------

function tsString(value) {
    return JSON.stringify(value ?? "");
}

function buildTokenMeta(token, bucket) {
    return {
        name: token.name,
        type: inferDtcgType(token),
        tier: token.tier,
        spec: token.spec,
        sourceTable: token.sourceTable,
        category: token.category,
        portability: token.valueType,
        bucket,
        rawValue: token.valueRaw ?? "",
        modes: token.valueByMode ?? {},
    };
}

async function emitTypeScriptMetadata(stage3Result, generated) {
    const outPath = path.join(DIST_TOKENS, "cem.tokens.ts");
    const allMeta = [
        ...stage3Result.visualTokens.map((token) => buildTokenMeta(token, "visual")),
        ...stage3Result.voiceTokens.map((token) => buildTokenMeta(token, "voice")),
    ].sort((a, b) => a.name.localeCompare(b.name));

    const tokenNames = allMeta.map((token) => token.name);
    const tokenTypes = [...new Set(allMeta.map((token) => token.type))].sort();
    const tiers = [...VALID_TIERS].sort();
    const portabilities = [...new Set(allMeta.map((token) => token.portability))].sort();

    const lines = [];
    lines.push("/*");
    lines.push(" * Generated by packages/cem-theme/scripts/export-tokens.mjs.");
    lines.push(" * Do not edit by hand.");
    lines.push(" */");
    lines.push("");
    lines.push(`export const cemTokenGenerated = ${JSON.stringify(generated, null, 4)} as const;`);
    lines.push("");
    lines.push("export type CemTokenName =");
    for (const name of tokenNames) lines.push(`    | ${tsString(name)}`);
    lines.push(";");
    lines.push("");
    lines.push("export type CemTokenType =");
    for (const type of tokenTypes) lines.push(`    | ${tsString(type)}`);
    lines.push(";");
    lines.push("");
    lines.push("export type CemTokenTier =");
    for (const tier of tiers) lines.push(`    | ${tsString(tier)}`);
    lines.push(";");
    lines.push("");
    lines.push("export type CemTokenPortability =");
    for (const portability of portabilities) lines.push(`    | ${tsString(portability)}`);
    lines.push(";");
    lines.push("");
    lines.push("export type CemTokenBucket = \"visual\" | \"voice\";");
    lines.push("");
    lines.push("export interface CemTokenMeta {");
    lines.push("    name: CemTokenName;");
    lines.push("    type: CemTokenType;");
    lines.push("    tier: CemTokenTier;");
    lines.push("    spec: string;");
    lines.push("    sourceTable: string;");
    lines.push("    category: string;");
    lines.push("    portability: CemTokenPortability;");
    lines.push("    bucket: CemTokenBucket;");
    lines.push("    rawValue: string;");
    lines.push("    modes: Readonly<Record<string, string>>;");
    lines.push("}");
    lines.push("");
    lines.push("export const cemTokens = [");
    for (const meta of allMeta) {
        lines.push("    {");
        lines.push(`        name: ${tsString(meta.name)},`);
        lines.push(`        type: ${tsString(meta.type)},`);
        lines.push(`        tier: ${tsString(meta.tier)},`);
        lines.push(`        spec: ${tsString(meta.spec)},`);
        lines.push(`        sourceTable: ${tsString(meta.sourceTable)},`);
        lines.push(`        category: ${tsString(meta.category)},`);
        lines.push(`        portability: ${tsString(meta.portability)},`);
        lines.push(`        bucket: ${tsString(meta.bucket)},`);
        lines.push(`        rawValue: ${tsString(meta.rawValue)},`);
        lines.push(`        modes: ${JSON.stringify(meta.modes)},`);
        lines.push("    },");
    }
    lines.push("] as const satisfies readonly CemTokenMeta[];");
    lines.push("");
    lines.push("export const cemTokenMetaByName = Object.fromEntries(");
    lines.push("    cemTokens.map((token) => [token.name, token]),");
    lines.push(") as unknown as Record<CemTokenName, CemTokenMeta>;");
    lines.push("");

    await fs.mkdir(DIST_TOKENS, { recursive: true });
    await fs.writeFile(outPath, lines.join("\n"), "utf8");
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

    // Stage 4 — Figma/Tokens Studio mode files
    console.log("export-tokens: Stage 4 — Figma mode file emission");
    const s4 = await stage4Figma(resolvedTokens, version, opts);

    const { modePaths, reportPath } = await emitFigmaFiles(s4);

    console.log(`  figma tokens: ${s4.included.length}  excluded: ${s4.excluded.length}`);
    if (s4.warnings.length) console.log(`  warnings: ${s4.warnings.length} (see ${path.relative(process.cwd(), reportPath)})`);
    for (const mode of MODES) console.log(`  → ${path.relative(process.cwd(), modePaths[mode])}`);
    console.log(`  → ${path.relative(process.cwd(), reportPath)}`);

    if (s4.errors.length) {
        for (const e of s4.errors) console.error(`  error: ${e}`);
        process.exit(1);
    }

    // Stage 5 — TypeScript token metadata
    console.log("export-tokens: Stage 5 — TypeScript metadata emission");
    const tsPath = await emitTypeScriptMetadata(s3, s3.generated);
    console.log(`  → ${path.relative(process.cwd(), tsPath)}`);
}

main(process.argv).catch((err) => {
    console.error(err);
    process.exit(2);
});
