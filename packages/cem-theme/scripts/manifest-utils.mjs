/**
 * Shared manifest derivation and CSS analysis for CEM token generation.
 */

export const SPEC_ORDER = [
    { name: "cem-colors", category: "Color (D0)" },
    { name: "cem-dimension", category: "Dimension & rhythm (D1)" },
    { name: "cem-timing", category: "Timing & motion (D7)" },
    { name: "cem-breakpoints", category: "Breakpoints (D1x)" },
    { name: "cem-coupling", category: "Coupling safety (D2)" },
    { name: "cem-controls", category: "Controls geometry (D2c)" },
    { name: "cem-shape", category: "Shape & bend (D3)" },
    { name: "cem-stroke", category: "Stroke & separation (D5)" },
    { name: "cem-layering", category: "Layering & elevation (D4)" },
    { name: "cem-voice-fonts-typography", category: "Typography & voice (D6)" },
];

export const COVERAGE_CATEGORIES = [
    { id: "d0-branded", label: "Branded colors (D0)", spec: "cem-colors" },
    { id: "d0-palette", label: "Emotional palette (D0)", spec: "cem-colors" },
    { id: "d0-action", label: "Action tokens (D0)", spec: "cem-colors" },
    { id: "d0-zebra", label: "Zebra tokens (D0)", spec: "cem-colors" },
    { id: "cem-dimension", label: "Dimension & rhythm (D1)", spec: "cem-dimension" },
    { id: "cem-breakpoints", label: "Breakpoints (D1x)", spec: "cem-breakpoints" },
    { id: "cem-coupling", label: "Coupling safety (D2)", spec: "cem-coupling" },
    { id: "cem-controls", label: "Controls geometry (D2c)", spec: "cem-controls" },
    { id: "cem-shape", label: "Shape & bend (D3)", spec: "cem-shape" },
    { id: "cem-stroke", label: "Stroke & separation (D5)", spec: "cem-stroke" },
    { id: "cem-layering", label: "Layering & elevation (D4)", spec: "cem-layering" },
    { id: "cem-voice-fonts-typography", label: "Typography & voice (D6)", spec: "cem-voice-fonts-typography" },
    { id: "cem-timing", label: "Timing & motion (D7)", spec: "cem-timing" },
];

function decodeHtmlEntities(str) {
    return str
        .replace(/&quot;/g, '"')
        .replace(/&apos;/g, "'")
        .replace(/&lt;/g, "<")
        .replace(/&gt;/g, ">")
        .replace(/&amp;/g, "&")
        .replace(/&#(\d+);/g, (_, n) => String.fromCharCode(Number(n)))
        .replace(/&#x([0-9a-f]+);/gi, (_, h) => String.fromCharCode(parseInt(h, 16)));
}

function extractCellText(html) {
    return decodeHtmlEntities(html.replace(/<[^>]+>/g, "").replace(/\s+/g, " ").trim());
}

/**
 * Extract the <tbody> rows of the table immediately following <h6 id="tableId">.
 * Returns an array of row arrays (each row = array of plain-text cell strings).
 */
export function extractTable(xhtml, tableId) {
    const result = extractTableWithHeaders(xhtml, tableId);
    return result ? result.rows : null;
}

/**
 * Extract headers and tbody rows of the table following <h6 id="tableId">.
 * Returns { headers: string[], rows: string[][] } or null if not found.
 */
export function extractTableWithHeaders(xhtml, tableId) {
    const h6Re = new RegExp(`<h6[^>]*\\bid="${tableId}"[^>]*>[\\s\\S]*?<\\/h6>`, "i");
    const h6Match = xhtml.match(h6Re);
    if (!h6Match) return null;

    const after = xhtml.slice(h6Match.index + h6Match[0].length);
    const tableMatch = after.match(/^\s*<table[\s\S]*?<\/table>/i);
    if (!tableMatch) return null;

    const tableHtml = tableMatch[0];

    const headers = [];
    const theadMatch = tableHtml.match(/<thead>([\s\S]*?)<\/thead>/i);
    if (theadMatch) {
        const thRe = /<th>([\s\S]*?)<\/th>/gi;
        let thM;
        while ((thM = thRe.exec(theadMatch[1])) !== null) {
            headers.push(extractCellText(thM[1]));
        }
    }

    const rows = [];
    const tbodyMatch = tableHtml.match(/<tbody>([\s\S]*?)<\/tbody>/i);
    if (tbodyMatch) {
        const trRe = /<tr>([\s\S]*?)<\/tr>/gi;
        let trM;
        while ((trM = trRe.exec(tbodyMatch[1])) !== null) {
            const cells = [];
            const tdRe = /<td>([\s\S]*?)<\/td>/gi;
            let tdM;
            while ((tdM = tdRe.exec(trM[1])) !== null) {
                cells.push(extractCellText(tdM[1]));
            }
            if (cells.length > 0) rows.push(cells);
        }
    }

    return { headers, rows };
}

/**
 * Extract {name, tier} from a simple per-token table.
 */
export function tokensFromTable(rows) {
    return rows
        .map((row) => ({ name: row[0], tier: (row[row.length - 1] || "").toLowerCase().trim() }))
        .filter((t) => t.name.startsWith("--"));
}

/**
 * Extract {name, valueRaw, description, tier, row} from table rows + headers.
 * Uses header names to locate value and description columns automatically.
 */
export function tokensFromTableWithValues(rows, headers = []) {
    const hn = headers.map((h) => h.toLowerCase());

    const VALUE_HEADERS = ["default-formula", "value"];
    let valueCol = hn.findIndex((h) => VALUE_HEADERS.some((c) => h === c));
    if (valueCol < 0) valueCol = 1;

    const DESC_HEADERS = ["description", "notes", "label", "intended use", "role", "usage"];
    const descCol = hn.findIndex((h) => DESC_HEADERS.some((c) => h.includes(c)));

    const tierCol = hn.findIndex((h) => h === "tier");

    return rows
        .filter((row) => row[0]?.startsWith("--"))
        .map((row) => {
            const effectiveTierCol = tierCol >= 0 ? tierCol : row.length - 1;
            return {
                name: row[0],
                valueRaw: row[valueCol] ?? "",
                description: descCol >= 0 ? (row[descCol] ?? "") : "",
                tier: (row[effectiveTierCol] || "").toLowerCase().trim(),
                row,
            };
        });
}

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

function addTableCategory(xhtml, tableId, categoryId, tokens, warnings, filter = () => true) {
    const rows = extractTable(xhtml, tableId);
    if (!rows) {
        warnings.push(`Table not found: #${tableId}`);
        return;
    }
    const extracted = tokensFromTable(rows).filter(filter).map((token) => ({ ...token, categoryId }));
    if (extracted.length === 0) warnings.push(`No token rows found in table #${tableId}`);
    tokens.push(...extracted);
}

function deriveColorManifest(xhtml) {
    const warnings = [];
    const tokens = [];

    addTableCategory(xhtml, "cem-color-hue-variant", "d0-branded", tokens, warnings);
    addTableCategory(xhtml, "cem-palette-emotion-shift", "d0-palette", tokens, warnings);
    addTableCategory(xhtml, "cem-zebra-tokens", "d0-zebra", tokens, warnings);

    const intentRows = extractTable(xhtml, "cem-action-intent-emotion");
    const stateRows = extractTable(xhtml, "cem-action-state-color");
    if (!intentRows) {
        warnings.push("Table not found: #cem-action-intent-emotion");
    } else if (!stateRows) {
        warnings.push("Table not found: #cem-action-state-color");
    } else {
        tokens.push(...actionTokensFromCrossProduct(intentRows, stateRows).map((token) => ({
            ...token,
            categoryId: "d0-action",
        })));
    }

    return { tokens, warnings };
}

function deriveDimensionManifest(xhtml) {
    const warnings = [];
    const tokens = [];
    const categoryId = "cem-dimension";

    for (const tableId of [
        "cem-dim-scale",
        "cem-dim-gaps",
        "cem-dim-insets",
        "cem-dim-rhythm-reading",
        "cem-dim-rhythm-data",
    ]) {
        addTableCategory(xhtml, tableId, categoryId, tokens, warnings);
    }

    addTableCategory(xhtml, "cem-dim-layout", categoryId, tokens, warnings, (token) => token.tier !== "deprecated");
    return { tokens, warnings };
}

function deriveBreakpointManifest(xhtml) {
    const warnings = [];
    const tokens = [];
    const categoryId = "cem-breakpoints";

    for (const tableId of ["cem-bp-basis", "cem-bp-height", "cem-bp-active", "cem-bp-cq"]) {
        addTableCategory(xhtml, tableId, categoryId, tokens, warnings);
    }

    return { tokens, warnings };
}

function deriveCouplingManifest(xhtml) {
    const warnings = [];
    const tokens = [];
    addTableCategory(xhtml, "cem-coupling-minimums", "cem-coupling", tokens, warnings);
    return { tokens, warnings };
}

function deriveTypographyManifest(xhtml) {
    const warnings = [];
    const tokens = [];
    const categoryId = "cem-voice-fonts-typography";

    for (const tableId of [
        "cem-typography-fontography",
        "cem-typography-thickness",
        "cem-typography-size",
        "cem-typography-line-height",
        "cem-typography-letter-spacing",
        "cem-typography-feature",
        "cem-typography-reading-ergonomics",
        "cem-typography-voice-ink-thickness",
        "cem-typography-voice-icon-stroke-multiplier",
        "cem-typography-voice-speech-volume",
        "cem-typography-voice-speech-rate",
        "cem-typography-voice-speech-pitch",
        "cem-typography-voice-ssml-emphasis",
        "cem-typography-roles",
    ]) {
        addTableCategory(xhtml, tableId, categoryId, tokens, warnings);
    }

    return { tokens, warnings };
}

function deriveLayeringManifest(xhtml) {
    const warnings = [];
    const tokens = [];
    const categoryId = "cem-layering";

    for (const tableId of [
        "cem-layering-rungs",
        "cem-layering-semantic",
        "cem-layering-semantic-optional",
    ]) {
        addTableCategory(xhtml, tableId, categoryId, tokens, warnings);
    }

    return { tokens, warnings };
}

function deriveStrokeManifest(xhtml) {
    const warnings = [];
    const tokens = [];
    const categoryId = "cem-stroke";

    for (const tableId of [
        "cem-stroke-basis",
        "cem-stroke-semantic",
        "cem-stroke-zebra-pattern",
        "cem-stroke-rings",
    ]) {
        addTableCategory(xhtml, tableId, categoryId, tokens, warnings);
    }

    return { tokens, warnings };
}

function deriveShapeManifest(xhtml) {
    const warnings = [];
    const tokens = [];
    const categoryId = "cem-shape";

    for (const tableId of ["cem-shape-basis", "cem-shape-semantic", "cem-shape-pattern", "cem-shape-action-bindings"]) {
        addTableCategory(xhtml, tableId, categoryId, tokens, warnings);
    }

    return { tokens, warnings };
}

function deriveControlsManifest(xhtml) {
    const warnings = [];
    const tokens = [];
    addTableCategory(xhtml, "cem-controls-geometry", "cem-controls", tokens, warnings);
    return { tokens, warnings };
}

function deriveTimingManifest(xhtml) {
    const warnings = [];
    const tokens = [];
    const categoryId = "cem-timing";

    for (const tableId of ["cem-timing-durations", "cem-timing-easings"]) {
        addTableCategory(xhtml, tableId, categoryId, tokens, warnings);
    }

    return { tokens, warnings };
}

export function deriveManifestForSpec(specName, xhtml) {
    const deriveManifest =
        specName === "cem-dimension"   ? deriveDimensionManifest :
        specName === "cem-timing"      ? deriveTimingManifest :
        specName === "cem-breakpoints" ? deriveBreakpointManifest :
        specName === "cem-coupling"    ? deriveCouplingManifest :
        specName === "cem-controls"    ? deriveControlsManifest :
        specName === "cem-shape"       ? deriveShapeManifest :
        specName === "cem-stroke"      ? deriveStrokeManifest :
        specName === "cem-layering"    ? deriveLayeringManifest :
        specName === "cem-voice-fonts-typography" ? deriveTypographyManifest :
        deriveColorManifest;
    return deriveManifest(xhtml);
}

/**
 * Parse CSS text and return:
 * - defined: Set of custom property names declared in any rule
 * - violations: array of issue strings
 */
export function analyzeCSS(cssText) {
    const defined = new Set();
    const violations = [];
    const noComments = cssText.replace(/\/\*[\s\S]*?\*\//g, "");

    const declRe = /(--cem-[a-z][a-z0-9-]*)\s*:/g;
    let m;
    while ((m = declRe.exec(noComments)) !== null) {
        defined.add(m[1]);
    }

    const avtRe = /(--cem-[a-z][a-z0-9-]*)\s*:[^;]*\{[^}]*\}/g;
    while ((m = avtRe.exec(noComments)) !== null) {
        violations.push(`AVT remnant in value of ${m[1]}: ${m[0].slice(0, 120)}`);
    }

    const emptyRuleRe = /[.#][a-zA-Z][^{]*\{\s*\}/g;
    while ((m = emptyRuleRe.exec(noComments)) !== null) {
        violations.push(`Empty placeholder rule: ${m[0].trim().slice(0, 80)}`);
    }

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

    return { defined, violations };
}

export async function tryPostcssValidation(cssText, violations) {
    let postcss;
    try {
        ({ default: postcss } = await import("postcss"));
    } catch {
        return;
    }
    try {
        postcss().process(cssText, { from: undefined });
    } catch (err) {
        violations.push(`PostCSS parse error: ${err.message}`);
    }
}

export function compareManifestToCss(manifest, defined, skip = new Set()) {
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

    return { missing, extras };
}
