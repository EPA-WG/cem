/**
 * Token-record derivation for CEM specs.
 *
 * Mirrors the derive*Manifest() functions in manifest-utils.mjs but returns full
 * normalized token records for use by the export pipeline.
 *
 * Each record: { name, valueRaw, tier, description, category, sourceTable, spec, row }
 * Color palette tokens also carry specModeValues: { light, dark, "native-light", "native-dark" }
 * Action cross-product tokens carry formulaBackground and formulaText.
 */

import { extractTableWithHeaders, tokensFromTableWithValues, extractTable } from "./manifest-utils.mjs";

function addTableTokens(xhtml, tableId, spec, category, tokens, warnings) {
    const result = extractTableWithHeaders(xhtml, tableId);
    if (!result) {
        warnings.push(`Table not found: #${tableId}`);
        return;
    }
    const extracted = tokensFromTableWithValues(result.rows, result.headers).map((t) => ({
        ...t,
        category,
        sourceTable: tableId,
        spec,
    }));
    if (extracted.length === 0) warnings.push(`No token rows found in table #${tableId}`);
    tokens.push(...extracted);
}

function deriveColorTokens(xhtml) {
    const warnings = [];
    const tokens = [];
    const spec = "cem-colors";

    addTableTokens(xhtml, "cem-color-hue-variant", spec, "d0-branded", tokens, warnings);

    // Palette emotion shift: multi-mode table — headers: Token|Role|Light|dark|Native Light|Native Dark|...
    const paletteResult = extractTableWithHeaders(xhtml, "cem-palette-emotion-shift");
    if (!paletteResult) {
        warnings.push("Table not found: #cem-palette-emotion-shift");
    } else {
        const hn = paletteResult.headers.map((h) => h.toLowerCase());
        const lightCol = hn.findIndex((h) => h === "light");
        const darkCol = hn.findIndex((h) => h === "dark");
        const nativeLightCol = hn.findIndex((h) => h === "native light");
        const nativeDarkCol = hn.findIndex((h) => h === "native dark");

        for (const t of tokensFromTableWithValues(paletteResult.rows, paletteResult.headers)) {
            const specModeValues = {};
            if (lightCol >= 0) specModeValues.light = t.row[lightCol] ?? "";
            if (darkCol >= 0) specModeValues.dark = t.row[darkCol] ?? "";
            if (nativeLightCol >= 0) specModeValues["native-light"] = t.row[nativeLightCol] ?? "";
            if (nativeDarkCol >= 0) specModeValues["native-dark"] = t.row[nativeDarkCol] ?? "";
            tokens.push({
                ...t,
                valueRaw: specModeValues.light ?? t.valueRaw,
                category: "d0-palette",
                sourceTable: "cem-palette-emotion-shift",
                spec,
                specModeValues,
            });
        }
    }

    addTableTokens(xhtml, "cem-zebra-tokens", spec, "d0-zebra", tokens, warnings);

    // Action tokens: cross-product of intent × state
    const intentRows = extractTable(xhtml, "cem-action-intent-emotion");
    const stateResult = extractTableWithHeaders(xhtml, "cem-action-state-color");
    if (!intentRows) {
        warnings.push("Table not found: #cem-action-intent-emotion");
    } else if (!stateResult) {
        warnings.push("Table not found: #cem-action-state-color");
    } else {
        const hn = stateResult.headers.map((h) => h.toLowerCase());
        const bgCol = hn.findIndex((h) => h.includes("background"));
        const textCol = hn.findIndex((h) => h.includes("text"));
        const tierCol = hn.findIndex((h) => h === "tier");
        const effectiveBgCol = bgCol >= 0 ? bgCol : 1;
        const effectiveTextCol = textCol >= 0 ? textCol : 2;

        const intents = intentRows.map((r) => r[0]).filter(Boolean);
        for (const intent of intents) {
            for (const stateRow of stateResult.rows) {
                const state = stateRow[0];
                if (!state) continue;
                const tier = (stateRow[tierCol >= 0 ? tierCol : stateRow.length - 1] || "").toLowerCase().trim();
                const formulaBackground = stateRow[effectiveBgCol] ?? "";
                const formulaText = stateRow[effectiveTextCol] ?? "";
                tokens.push({
                    name: `--cem-action-${intent}-${state}-background`,
                    valueRaw: formulaBackground,
                    tier,
                    description: `Action ${intent} ${state} background`,
                    category: "d0-action",
                    sourceTable: "cem-action-state-color",
                    spec,
                    row: stateRow,
                    formulaBackground,
                    formulaText,
                });
                tokens.push({
                    name: `--cem-action-${intent}-${state}-text`,
                    valueRaw: formulaText,
                    tier,
                    description: `Action ${intent} ${state} text`,
                    category: "d0-action",
                    sourceTable: "cem-action-state-color",
                    spec,
                    row: stateRow,
                    formulaBackground,
                    formulaText,
                });
            }
        }
    }

    return { tokens, warnings };
}

function deriveDimensionTokens(xhtml) {
    const warnings = [];
    const tokens = [];
    const spec = "cem-dimension";

    for (const tableId of [
        "cem-dim-scale",
        "cem-dim-gaps",
        "cem-dim-insets",
        "cem-dim-rhythm-reading",
        "cem-dim-rhythm-data",
        "cem-dim-layout",
    ]) {
        addTableTokens(xhtml, tableId, spec, spec, tokens, warnings);
    }

    return { tokens, warnings };
}

function deriveBreakpointTokens(xhtml) {
    const warnings = [];
    const tokens = [];
    const spec = "cem-breakpoints";

    for (const tableId of ["cem-bp-basis", "cem-bp-height", "cem-bp-active", "cem-bp-cq"]) {
        addTableTokens(xhtml, tableId, spec, spec, tokens, warnings);
    }

    return { tokens, warnings };
}

function deriveCouplingTokens(xhtml) {
    const warnings = [];
    const tokens = [];
    addTableTokens(xhtml, "cem-coupling-minimums", "cem-coupling", "cem-coupling", tokens, warnings);
    return { tokens, warnings };
}

function deriveTypographyTokens(xhtml) {
    const warnings = [];
    const tokens = [];
    const spec = "cem-voice-fonts-typography";

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
        addTableTokens(xhtml, tableId, spec, spec, tokens, warnings);
    }

    return { tokens, warnings };
}

function deriveLayeringTokens(xhtml) {
    const warnings = [];
    const tokens = [];
    const spec = "cem-layering";

    for (const tableId of [
        "cem-layering-rungs",
        "cem-layering-semantic",
        "cem-layering-semantic-optional",
    ]) {
        addTableTokens(xhtml, tableId, spec, spec, tokens, warnings);
    }

    return { tokens, warnings };
}

function deriveStrokeTokens(xhtml) {
    const warnings = [];
    const tokens = [];
    const spec = "cem-stroke";

    for (const tableId of [
        "cem-stroke-basis",
        "cem-stroke-semantic",
        "cem-stroke-zebra-pattern",
        "cem-stroke-rings",
    ]) {
        addTableTokens(xhtml, tableId, spec, spec, tokens, warnings);
    }

    return { tokens, warnings };
}

function deriveShapeTokens(xhtml) {
    const warnings = [];
    const tokens = [];
    const spec = "cem-shape";

    for (const tableId of [
        "cem-shape-basis",
        "cem-shape-semantic",
        "cem-shape-pattern",
        "cem-shape-action-bindings",
    ]) {
        addTableTokens(xhtml, tableId, spec, spec, tokens, warnings);
    }

    return { tokens, warnings };
}

function deriveControlsTokens(xhtml) {
    const warnings = [];
    const tokens = [];
    addTableTokens(xhtml, "cem-controls-geometry", "cem-controls", "cem-controls", tokens, warnings);
    return { tokens, warnings };
}

function deriveTimingTokens(xhtml) {
    const warnings = [];
    const tokens = [];
    const spec = "cem-timing";

    for (const tableId of ["cem-timing-durations", "cem-timing-easings"]) {
        addTableTokens(xhtml, tableId, spec, spec, tokens, warnings);
    }

    return { tokens, warnings };
}

export function deriveTokensForSpec(specName, xhtml) {
    const derive =
        specName === "cem-dimension"              ? deriveDimensionTokens :
        specName === "cem-timing"                 ? deriveTimingTokens :
        specName === "cem-breakpoints"            ? deriveBreakpointTokens :
        specName === "cem-coupling"               ? deriveCouplingTokens :
        specName === "cem-controls"               ? deriveControlsTokens :
        specName === "cem-shape"                  ? deriveShapeTokens :
        specName === "cem-stroke"                 ? deriveStrokeTokens :
        specName === "cem-layering"               ? deriveLayeringTokens :
        specName === "cem-voice-fonts-typography" ? deriveTypographyTokens :
        deriveColorTokens;
    return derive(xhtml);
}
