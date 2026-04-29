/**
 * Token export pipeline — orchestrates Phases A through E.
 *
 * Stage 1 (Phase A): Extract token records from compiled XHTML specs into an
 *   in-memory intermediate model and emit cem.tokens.intermediate.json.
 *
 * Usage:
 *   node packages/cem-theme/scripts/export-tokens.mjs [--with-optional] [--with-adapter] [--with-deprecated]
 *
 * Depends on: dist/lib/tokens/*.xhtml (produced by build:css)
 */

import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createReadStream } from "node:fs";
import { SPEC_ORDER, deriveManifestForSpec } from "./manifest-utils.mjs";
import { deriveTokensForSpec } from "./derive-tokens.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PACKAGE_ROOT = path.resolve(__dirname, "..");
const DIST_TOKENS = path.join(PACKAGE_ROOT, "dist/lib/tokens");

const VALID_TIERS = new Set(["required", "recommended", "optional", "adapter", "deprecated"]);

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

    const outPath = await emitIntermediate(tokens, warnings, errors, version, opts);
    const rel = path.relative(process.cwd(), outPath);

    console.log(`  extracted ${tokens.length} tokens from ${SPEC_ORDER.length} specs`);
    console.log(`  warnings: ${warnings.length}  errors: ${errors.length}`);
    console.log(`  → ${rel}`);

    if (errors.length) {
        for (const e of errors) console.error(`  error: ${e}`);
        process.exit(1);
    }
}

main(process.argv).catch((err) => {
    console.error(err);
    process.exit(2);
});
