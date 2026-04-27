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
 *   --skip=<id>  Skip named table id (may be repeated, e.g. --skip=cem-breakpoints)
 *
 * Exit codes:
 *   0  No violations (or soft mode with violations)
 *   1  Violations found in hard mode
 *   2  Script error (missing file, parse failure)
 */

import fs from "node:fs/promises";
import path from "node:path";

// ── XHTML table extraction ────────────────────────────────────────────────────

/**
 * Extract the <tbody> rows of the table immediately following <h6 id="tableId">.
 * Returns an array of row arrays (each row = array of plain-text cell strings).
 */
function extractTable(xhtml, tableId) {
    // Match the full <h6 id="...">...</h6> element
    const h6Re = new RegExp(`<h6[^>]*\\bid="${tableId}"[^>]*>[\\s\\S]*?<\\/h6>`, "i");
    const h6Match = xhtml.match(h6Re);
    if (!h6Match) return null;

    const after = xhtml.slice(h6Match.index + h6Match[0].length);

    // The very next element must be <table>
    const tableMatch = after.match(/^\s*<table[\s\S]*?<\/table>/i);
    if (!tableMatch) return null;

    const tbodyMatch = tableMatch[0].match(/<tbody>([\s\S]*?)<\/tbody>/i);
    if (!tbodyMatch) return null;

    const rows = [];
    const trRe = /<tr>([\s\S]*?)<\/tr>/gi;
    let trM;
    while ((trM = trRe.exec(tbodyMatch[1])) !== null) {
        const cells = [];
        const tdRe = /<td>([\s\S]*?)<\/td>/gi;
        let tdM;
        while ((tdM = tdRe.exec(trM[1])) !== null) {
            // strip all tags, collapse whitespace
            cells.push(tdM[1].replace(/<[^>]+>/g, "").replace(/\s+/g, " ").trim());
        }
        if (cells.length > 0) rows.push(cells);
    }
    return rows;
}

/**
 * Extract {name, tier} from a simple per-token table.
 * Token name is in the first cell (wrapped in backtick-code in source, stripped in XHTML).
 * Tier is in the last cell.
 */
function tokensFromTable(rows) {
    return rows
        .map((row) => ({ name: row[0], tier: (row[row.length - 1] || "").toLowerCase().trim() }))
        .filter((t) => t.name.startsWith("--"));
}

/**
 * Derive action tokens from intent × state cross-product.
 * intentRows: rows of cem-action-intent-emotion (col 0 = intent name in <code>)
 * stateRows:  rows of cem-action-state-color   (col 0 = state name, last col = tier)
 */
function actionTokensFromCrossProduct(intentRows, stateRows) {
    const intents = intentRows.map((r) => r[0]).filter(Boolean);
    const tokens = [];
    for (const intent of intents) {
        for (const stateRow of stateRows) {
            const state = stateRow[0];
            const tier = (stateRow[stateRow.length - 1] || "").toLowerCase().trim();
            if (!state) continue;
            tokens.push({ name: `--cem-action-${intent}-${state}-background`, tier });
            tokens.push({ name: `--cem-action-${intent}-${state}-text`, tier });
        }
    }
    return tokens;
}

// ── Manifest derivation ───────────────────────────────────────────────────────

/**
 * Build the full expected token list for cem-colors from the compiled XHTML.
 * Returns { tokens: [{name, tier}], warnings: string[] }
 */
function deriveColorManifest(xhtml) {
    const warnings = [];
    const tokens = [];

    for (const tableId of ["cem-color-hue-variant", "cem-palette-emotion-shift", "cem-zebra-tokens"]) {
        const rows = extractTable(xhtml, tableId);
        if (!rows) {
            warnings.push(`Table not found: #${tableId}`);
            continue;
        }
        const extracted = tokensFromTable(rows);
        if (extracted.length === 0) {
            warnings.push(`No token rows found in table #${tableId}`);
        }
        tokens.push(...extracted);
    }

    // Cross-product: intents × states
    const intentRows = extractTable(xhtml, "cem-action-intent-emotion");
    const stateRows = extractTable(xhtml, "cem-action-state-color");
    if (!intentRows) {
        warnings.push("Table not found: #cem-action-intent-emotion");
    } else if (!stateRows) {
        warnings.push("Table not found: #cem-action-state-color");
    } else {
        tokens.push(...actionTokensFromCrossProduct(intentRows, stateRows));
    }

    return { tokens, warnings };
}

// ── CSS analysis ─────────────────────────────────────────────────────────────

/**
 * Parse CSS text (without PostCSS) and return:
 * - defined: Set of custom property names declared at the top level of any rule
 * - violations: array of issue strings
 */
function analyzeCSS(cssText) {
    const defined = new Set();
    const violations = [];

    // Strip CSS comments for most checks (keep original for AVT check in comments)
    const noComments = cssText.replace(/\/\*[\s\S]*?\*\//g, "");

    // Extract all custom property declarations: --name: value;
    const declRe = /(--cem-[a-z][a-z0-9-]*)\s*:/g;
    let m;
    while ((m = declRe.exec(noComments)) !== null) {
        defined.add(m[1]);
    }

    // AVT remnants in actual values (not in comments): {expression} inside a property value
    // These appear when template substitution failed. Pattern: --foo: ...{...}...;
    const avtRe = /(--cem-[a-z][a-z0-9-]*)\s*:[^;]*\{[^}]*\}/g;
    while ((m = avtRe.exec(noComments)) !== null) {
        violations.push(`AVT remnant in value of ${m[1]}: ${m[0].slice(0, 120)}`);
    }

    // Placeholder stub rules: .someClass{} or similar entirely empty rules
    const emptyRuleRe = /[.#][a-zA-Z][^{]*\{\s*\}/g;
    while ((m = emptyRuleRe.exec(noComments)) !== null) {
        violations.push(`Empty placeholder rule: ${m[0].trim().slice(0, 80)}`);
    }

    // Balanced braces
    let depth = 0;
    for (const ch of noComments) {
        if (ch === "{") depth++;
        else if (ch === "}") depth--;
        if (depth < 0) {
            violations.push("Unbalanced braces: unexpected } in CSS");
            depth = 0;
        }
    }
    if (depth !== 0) {
        violations.push(`Unbalanced braces: ${depth} unclosed { in CSS`);
    }

    // Use PostCSS for a proper parse check if available
    return { defined, violations };
}

async function tryPostcssValidation(cssText, violations) {
    let postcss;
    try {
        ({ default: postcss } = await import("postcss"));
    } catch {
        return; // PostCSS not available; skip
    }
    try {
        postcss().process(cssText, { from: undefined });
    } catch (err) {
        violations.push(`PostCSS parse error: ${err.message}`);
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

function usage() {
    console.error(
        "Usage: node packages/cem-theme/scripts/validate-manifest.mjs [--hard] [--skip=<id>] <spec.xhtml> <output.css>"
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

    // Derive manifest
    const { tokens: manifest, warnings: manifestWarnings } = deriveColorManifest(xhtml);
    if (manifestWarnings.length) {
        for (const w of manifestWarnings) console.warn(`[manifest] ${w}`);
    }

    // Analyse CSS
    const { defined, violations: cssViolations } = analyzeCSS(cssText);
    await tryPostcssValidation(cssText, cssViolations);

    const allViolations = [...cssViolations];

    // Coverage: expected vs. actual
    const missing = [];
    const extras = [];

    for (const { name, tier } of manifest) {
        if (skip.has(name)) continue;
        if (!defined.has(name)) {
            missing.push({ name, tier });
        }
    }

    const manifestNames = new Set(manifest.map((t) => t.name));
    for (const name of defined) {
        if (!manifestNames.has(name)) {
            extras.push(name);
        }
    }

    // Report
    const hasViolations = allViolations.length > 0 || missing.length > 0 || extras.length > 0;

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

    if (allViolations.length) {
        console.log(`\n  CSS violations (${allViolations.length}):`);
        for (const v of allViolations) {
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
