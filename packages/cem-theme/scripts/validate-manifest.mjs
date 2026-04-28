/**
 * Manifest-vs-CSS validator for CEM token specs.
 *
 * Reads source tables from a compiled XHTML spec, derives the expected token set,
 * then checks the generated CSS for coverage, extras, placeholders, and parse errors.
 *
 * Usage:
 *   node packages/cem-theme/scripts/validate-manifest.mjs [options] <spec.xhtml> <output.css>
 *
 * Options:
 *   --hard    Exit with code 1 on any violation (default: report only)
 *   --skip=<id>  Skip named token (may be repeated, e.g. --skip=--cem-bp-example)
 *
 * Exit codes:
 *   0  No violations (or soft mode with violations)
 *   1  Violations found in hard mode
 *   2  Script error (missing file, parse failure)
 */

import fs from "node:fs/promises";
import path from "node:path";
import {
    analyzeCSS,
    compareManifestToCss,
    deriveManifestForSpec,
    tryPostcssValidation,
} from "./manifest-utils.mjs";

function usage() {
    console.error(
        "Usage: node packages/cem-theme/scripts/validate-manifest.mjs [--hard] [--skip=<token>] <spec.xhtml> <output.css>"
    );
}

async function main(argv) {
    const args = argv.slice(2);
    let hard = false;
    const skip = new Set();
    const positional = [];

    for (const arg of args) {
        if (arg === "--hard") { hard = true; }
        else if (arg.startsWith("--skip=")) { skip.add(arg.slice(7)); }
        else { positional.push(arg); }
    }

    if (positional.length < 2) {
        usage();
        process.exit(2);
    }

    const [xhtmlPath, cssPath] = positional.map((p) => path.resolve(p));

    let xhtml, cssText;
    try {
        xhtml = await fs.readFile(xhtmlPath, "utf8");
    } catch (err) {
        console.error(`Cannot read XHTML: ${xhtmlPath}\n${err.message}`);
        process.exit(2);
    }
    try {
        cssText = await fs.readFile(cssPath, "utf8");
    } catch (err) {
        console.error(`Cannot read CSS: ${cssPath}\n${err.message}`);
        process.exit(2);
    }

    const specName = path.basename(xhtmlPath, ".xhtml");
    const { tokens: manifest, warnings: manifestWarnings } = deriveManifestForSpec(specName, xhtml);
    if (manifestWarnings.length) {
        for (const w of manifestWarnings) console.warn(`[manifest] ${w}`);
    }

    const { defined, violations: cssViolations } = analyzeCSS(cssText);
    await tryPostcssValidation(cssText, cssViolations);

    const { missing, extras } = compareManifestToCss(manifest, defined, skip);
    const hasViolations = cssViolations.length > 0 || missing.length > 0 || extras.length > 0;

    if (!hasViolations) {
        console.log(`✓  ${manifest.length} manifest tokens all present, no extras, CSS valid`);
        console.log(`   ${path.relative(process.cwd(), cssPath)}`);
        return;
    }

    console.log(`\nManifest validation: ${path.relative(process.cwd(), cssPath)}`);
    console.log(`  Manifest tokens : ${manifest.length}`);
    console.log(`  CSS definitions : ${defined.size}`);

    if (missing.length) {
        console.log(`\n  Missing tokens (${missing.length}):`);
        for (const { name, tier } of missing) {
            console.log(`    [${tier}]  ${name}`);
        }
    }

    if (extras.length) {
        console.log(`\n  Extra tokens not in manifest (${extras.length}):`);
        for (const name of extras) {
            console.log(`    ${name}`);
        }
    }

    if (cssViolations.length) {
        console.log(`\n  CSS violations (${cssViolations.length}):`);
        for (const v of cssViolations) {
            console.log(`    ${v}`);
        }
    }

    if (hard) {
        process.exit(1);
    }
}

main(process.argv).catch((err) => {
    console.error(err);
    process.exit(2);
});
