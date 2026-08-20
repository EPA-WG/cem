#!/usr/bin/env node

import fs from "node:fs/promises";
import path from "node:path";

const PACKAGE_ROOT = path.resolve(new URL("..", import.meta.url).pathname);
const WORKSPACE_ROOT = path.resolve(PACKAGE_ROOT, "../..");
const TOKEN_PATH = path.join(PACKAGE_ROOT, "dist/lib/tokens/cem.tokens.json");
const INVENTORY_PATH = path.join(WORKSPACE_ROOT, "examples/figma/foundations-library.json");
const FIXTURE_PATH = path.join(WORKSPACE_ROOT, "examples/figma/foundations-library-fixture.md");
const LAYERING_SPEC_PATH = path.join(PACKAGE_ROOT, "src/lib/tokens/cem-layering.md");
const TIMING_SPEC_PATH = path.join(PACKAGE_ROOT, "src/lib/tokens/cem-timing.md");
const REPORT_DIRECTORY = path.join(PACKAGE_ROOT, "dist/reports");
const REPORT_JSON_PATH = path.join(REPORT_DIRECTORY, "cem-foundations-report.json");
const REPORT_MARKDOWN_PATH = path.join(REPORT_DIRECTORY, "cem-foundations-report.md");

const EXPECTED_SOURCES = {
    layering: "packages/cem-theme/src/lib/tokens/cem-layering.md#cem-layering-rungs",
    layerAliases: "packages/cem-theme/src/lib/tokens/cem-layering.md#cem-layering-semantic",
    motion: "packages/cem-theme/src/lib/tokens/cem-timing.md#cem-timing-easings",
    canonicalTokens: "packages/cem-theme/dist/lib/tokens/cem.tokens.json",
};
const EXPECTED_MODES = ["Light", "Dark", "Contrast Light", "Contrast Dark", "Native"];
const EXPECTED_LAYERING = [
    { token: "cem.recess.2", name: "Deep Recessed", aliases: [], representation: "effect-style" },
    { token: "cem.recess.1", name: "Recessed", aliases: ["cem.layer.back"], representation: "effect-style" },
    { token: "cem.elevation.0", name: "Base", aliases: ["cem.layer.base"], representation: "no-effect" },
    { token: "cem.elevation.1", name: "Raised", aliases: ["cem.layer.work"], representation: "effect-style" },
    { token: "cem.elevation.2", name: "Floating", aliases: [], representation: "effect-style" },
    { token: "cem.elevation.3", name: "Overlay", aliases: ["cem.layer.overlay"], representation: "effect-style" },
    { token: "cem.elevation.4", name: "Command", aliases: ["cem.layer.command"], representation: "effect-style" },
];
const EXPECTED_MOTION = [
    { token: "cem.easing.smooth", name: "Smooth" },
    { token: "cem.easing.start.smooth", name: "Start Smooth" },
    { token: "cem.easing.end.smooth", name: "End Smooth" },
    { token: "cem.easing.highlighted", name: "Highlighted" },
    { token: "cem.easing.highlighted.start", name: "Highlighted Start" },
    { token: "cem.easing.highlighted.end", name: "Highlighted End" },
    { token: "cem.easing.uniform", name: "Uniform" },
    { token: "cem.easing.classic", name: "Classic" },
];
const CSS_EASING_KEYWORDS = new Map([
    ["ease", [0.25, 0.1, 0.25, 1]],
    ["ease-in", [0.42, 0, 1, 1]],
    ["ease-out", [0, 0, 0.58, 1]],
    ["ease-in-out", [0.42, 0, 0.58, 1]],
    ["linear", [0, 0, 1, 1]],
]);
const failures = [];

const [tokens, inventory, fixture, layeringSpec, timingSpec] = await Promise.all([
    readJson(TOKEN_PATH),
    readJson(INVENTORY_PATH),
    fs.readFile(FIXTURE_PATH, "utf8"),
    fs.readFile(LAYERING_SPEC_PATH, "utf8"),
    fs.readFile(TIMING_SPEC_PATH, "utf8"),
]);

validateInventoryBoundary();
const layeringReport = validateLayering();
const motionReport = validateMotion();
validateFixture();

if (failures.length > 0) {
    for (const failure of failures) console.error(`error: ${failure}`);
    process.exit(1);
}

const entries = [...inventory.layering, ...inventory.motion];
const report = {
    version: inventory.version,
    sources: inventory.sources,
    figma: inventory.figma,
    summary: {
        total: entries.length,
        effectStyles: inventory.layering.filter((entry) => entry.representation === "effect-style").length,
        noEffectSpecimens: inventory.layering.filter((entry) => entry.representation === "no-effect").length,
        semanticAliases: inventory.layering.reduce((count, entry) => count + entry.semanticAliases.length, 0),
        motionSpecimens: inventory.motion.length,
        planned: entries.filter((entry) => entry.figma.status === "planned").length,
        reviewed: entries.filter((entry) => entry.figma.status === "reviewed").length,
    },
    layering: layeringReport,
    motion: motionReport,
};

await fs.mkdir(REPORT_DIRECTORY, { recursive: true });
await fs.writeFile(REPORT_JSON_PATH, `${JSON.stringify(report, null, 4)}\n`, "utf8");
await fs.writeFile(REPORT_MARKDOWN_PATH, renderMarkdown(report), "utf8");

console.log(
    `Figma Foundations inventory verified (${report.summary.effectStyles} effect styles, ` +
        `${report.summary.noEffectSpecimens} no-effect specimen, ${report.summary.semanticAliases} semantic aliases, ` +
        `${report.summary.motionSpecimens} motion specimens; ${report.summary.reviewed} reviewed, ` +
        `${report.summary.planned} planned).`
);

function validateInventoryBoundary() {
    if (!inventory || typeof inventory !== "object" || Array.isArray(inventory)) {
        fail("Figma Foundations inventory must be an object");
        return;
    }
    if (inventory.version !== 1) fail("Figma Foundations inventory version must be 1");
    if (!sameJson(inventory.sources, EXPECTED_SOURCES)) {
        fail("Figma Foundations inventory sources must name the canonical layering, motion, and token owners");
    }
    if (inventory.figma?.file !== "https://www.figma.com/design/vLZUzjS7xHACjXgYLA9vtD/CEM-UI-Kit") {
        fail("Figma Foundations inventory must target the canonical CEM UI Kit file");
    }
    if (inventory.figma?.page !== "02 Foundations") fail("Figma Foundations inventory must target 02 Foundations");
    if (!sameJson(inventory.figma?.modes, EXPECTED_MODES)) {
        fail(`Figma Foundations inventory modes must be ${EXPECTED_MODES.join(", ")}`);
    }
    const expectedGovernance = {
        layering: "derived-effect-styles",
        motion: "derived-motion-specimens",
        nativeVariableImport: "excluded",
        rawValuesAllowed: false,
    };
    if (!sameJson(inventory.figma?.governance, expectedGovernance)) {
        fail("Figma Foundations inventory governance must preserve derived composites and prohibit raw values");
    }
    if (!Array.isArray(inventory.layering)) fail("Figma Foundations layering inventory must be an array");
    if (!Array.isArray(inventory.motion)) fail("Figma Foundations motion inventory must be an array");

    const serialized = JSON.stringify(inventory);
    for (const pattern of [/#(?:[0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})\b/iu, /rgba?\(/iu, /cubic-bezier\(/iu, /-?\d+(?:\.\d+)?(?:px|rem|ms)\b/iu]) {
        if (pattern.test(serialized)) fail(`Figma Foundations inventory contains a raw composite value matching ${pattern}`);
    }
}

function validateLayering() {
    if (!Array.isArray(inventory.layering)) return [];
    validateExactList(
        "layering inventory",
        inventory.layering.map((entry) => entry?.token),
        EXPECTED_LAYERING.map((entry) => entry.token)
    );

    const reportEntries = [];
    const styleNames = new Set();
    for (const [index, expected] of EXPECTED_LAYERING.entries()) {
        const entry = inventory.layering[index];
        if (!entry) continue;
        if (entry.name !== expected.name) fail(`${expected.token}: name must be ${expected.name}`);
        if (entry.representation !== expected.representation) {
            fail(`${expected.token}: representation must be ${expected.representation}`);
        }
        if (!sameJson(entry.semanticAliases, expected.aliases)) {
            fail(`${expected.token}: semantic aliases must be ${expected.aliases.join(", ") || "empty"}`);
        }

        const token = tokenAt(tokens, expected.token);
        if (!token) {
            fail(`${expected.token}: missing from canonical tokens`);
            continue;
        }
        if (!layeringSpec.includes(`--${expected.token.replaceAll(".", "-")}`)) {
            fail(`${expected.token}: missing from canonical layering spec`);
        }

        if (expected.representation === "no-effect") {
            if (entry.style !== null) fail(`${expected.token}: no-effect specimen must not name an Effect Style`);
            if (token.$type !== "string" || token.$value !== "none") {
                fail(`${expected.token}: Base must preserve the canonical string value none`);
            }
        } else {
            const expectedStyle = `CEM/Layering/${expected.name}`;
            if (entry.style !== expectedStyle) fail(`${expected.token}: Effect Style must be ${expectedStyle}`);
            if (styleNames.has(entry.style)) fail(`${expected.token}: duplicate Effect Style ${entry.style}`);
            styleNames.add(entry.style);
            if (token.$type !== "shadow") fail(`${expected.token}: canonical type must be shadow`);
            const parsed = parseCssShadow(token.$extensions?.cem?.rawValue);
            if (!parsed || !sameJson(token.$value, parsed)) {
                fail(`${expected.token}: canonical shadow must be derived exactly from its CSS source value`);
            }
        }

        for (const aliasPath of expected.aliases) {
            const alias = tokenAt(tokens, aliasPath);
            if (!alias) {
                fail(`${aliasPath}: missing canonical semantic alias`);
                continue;
            }
            if (alias.$value !== `{${expected.token}}`) {
                fail(`${aliasPath}: must reference {${expected.token}}`);
            }
            if (alias.$type !== token.$type) {
                fail(`${aliasPath}: type must match ${expected.token} (${token.$type})`);
            }
        }
        validateReviewEvidence(entry, "Layering");
        reportEntries.push({ ...entry, canonicalType: token.$type, derivedValue: token.$value });
    }
    return reportEntries;
}

function validateMotion() {
    if (!Array.isArray(inventory.motion)) return [];
    validateExactList(
        "motion inventory",
        inventory.motion.map((entry) => entry?.token),
        EXPECTED_MOTION.map((entry) => entry.token)
    );

    const reportEntries = [];
    const specimenNames = new Set();
    for (const [index, expected] of EXPECTED_MOTION.entries()) {
        const entry = inventory.motion[index];
        if (!entry) continue;
        if (entry.name !== expected.name) fail(`${expected.token}: name must be ${expected.name}`);
        const expectedSpecimen = `CEM/Motion/${expected.name}`;
        if (entry.specimen !== expectedSpecimen) fail(`${expected.token}: specimen must be ${expectedSpecimen}`);
        if (specimenNames.has(entry.specimen)) fail(`${expected.token}: duplicate motion specimen ${entry.specimen}`);
        specimenNames.add(entry.specimen);

        const token = tokenAt(tokens, expected.token);
        if (!token) {
            fail(`${expected.token}: missing from canonical tokens`);
            continue;
        }
        if (!timingSpec.includes(`--${expected.token.replaceAll(".", "-")}`)) {
            fail(`${expected.token}: missing from canonical timing spec`);
        }
        if (token.$type !== "cubicBezier") fail(`${expected.token}: canonical type must be cubicBezier`);
        const parsed = parseCssEasing(token.$extensions?.cem?.rawValue);
        if (!parsed || !sameJson(token.$value, parsed)) {
            fail(`${expected.token}: canonical cubic-Bézier must be derived exactly from its CSS source value`);
        }
        validateReviewEvidence(entry, "Motion");
        reportEntries.push({ ...entry, canonicalType: token.$type, derivedValue: token.$value });
    }
    return reportEntries;
}

function validateReviewEvidence(entry, section) {
    const evidence = entry.figma;
    if (!evidence || typeof evidence !== "object" || Array.isArray(evidence)) {
        fail(`${entry.token}: Figma evidence must be an object`);
        return;
    }
    if (evidence.status !== "planned" && evidence.status !== "reviewed") {
        fail(`${entry.token}: Figma status must be planned or reviewed`);
    }
    const expectedLocator = `02 Foundations / ${section} / ${entry.name}`;
    if (evidence.locator !== expectedLocator) fail(`${entry.token}: locator must be ${expectedLocator}`);
    if (evidence.status === "planned" && evidence.revision !== null) {
        fail(`${entry.token}: planned Figma evidence must use a null revision`);
    }
    if (
        evidence.status === "reviewed" &&
        (typeof evidence.revision !== "string" ||
            (!/^https:\/\/www\.figma\.com\/(design|file)\//u.test(evidence.revision) &&
                !/^[a-z0-9._-]+$/iu.test(evidence.revision)))
    ) {
        fail(`${entry.token}: reviewed Figma evidence requires a Figma URL or stable revision token`);
    }
}

function validateFixture() {
    const markers = [
        "foundations-library.json",
        "verify:figma-foundations",
        "Derived Effect Styles",
        "Explicit no-effect specimen",
        "Derived motion specimens",
        "Light",
        "Dark",
        "Contrast Light",
        "Contrast Dark",
        "Native",
        "Deliberate rejection cases",
    ];
    for (const marker of markers) {
        if (!fixture.includes(marker)) fail(`foundations-library-fixture.md missing review marker ${marker}`);
    }
}

function tokenAt(tree, tokenPath) {
    return tokenPath.split(".").reduce((node, part) => node?.[part], tree);
}

function parseCssEasing(value) {
    const normalized = (value ?? "").trim().toLowerCase();
    const keyword = CSS_EASING_KEYWORDS.get(normalized);
    if (keyword) return [...keyword];
    const match = normalized.match(/^cubic-bezier\(([^)]+)\)$/u);
    if (!match) return null;
    const coordinates = match[1].split(",").map((part) => Number.parseFloat(part.trim()));
    if (
        coordinates.length !== 4 ||
        coordinates.some((coordinate) => !Number.isFinite(coordinate)) ||
        coordinates[0] < 0 ||
        coordinates[0] > 1 ||
        coordinates[2] < 0 ||
        coordinates[2] > 1
    ) {
        return null;
    }
    return coordinates;
}

function parseCssShadow(value) {
    const normalized = (value ?? "").trim();
    if (!normalized || normalized === "none") return null;
    const shadows = splitTopLevel(normalized).map((part) => parseShadowPart(part));
    if (shadows.some((shadow) => !shadow)) return null;
    return shadows.length === 1 ? shadows[0] : shadows;
}

function parseShadowPart(value) {
    const terms = splitTerms(value);
    const inset = terms.includes("inset");
    const visibleTerms = terms.filter((term) => term !== "inset");
    const colorIndex = visibleTerms.findIndex((term) => parseColor(term));
    if (colorIndex < 0) return null;
    const color = parseColor(visibleTerms[colorIndex]);
    const dimensions = visibleTerms.filter((_, index) => index !== colorIndex).map(parseDimension);
    if (!color || dimensions.length < 2 || dimensions.length > 4 || dimensions.some((dimension) => !dimension)) {
        return null;
    }
    const [offsetX, offsetY, blur = dimension(0), spread = dimension(0)] = dimensions;
    return { color, offsetX, offsetY, blur, spread, ...(inset ? { inset: true } : {}) };
}

function splitTopLevel(value) {
    const parts = [];
    let depth = 0;
    let start = 0;
    for (let index = 0; index < value.length; index++) {
        if (value[index] === "(") depth++;
        else if (value[index] === ")") depth--;
        else if (value[index] === "," && depth === 0) {
            parts.push(value.slice(start, index).trim());
            start = index + 1;
        }
    }
    parts.push(value.slice(start).trim());
    return parts;
}

function splitTerms(value) {
    const terms = [];
    let depth = 0;
    let start = 0;
    for (let index = 0; index <= value.length; index++) {
        const character = value[index];
        if (character === "(") depth++;
        else if (character === ")") depth--;
        if ((index === value.length || /\s/u.test(character)) && depth === 0) {
            const term = value.slice(start, index).trim();
            if (term) terms.push(term);
            start = index + 1;
        }
    }
    return terms;
}

function parseDimension(value) {
    const normalized = (value ?? "").trim().toLowerCase();
    if (normalized === "0") return dimension(0);
    const match = normalized.match(/^(-?(?:\d+\.?\d*|\.\d+))(px|rem)$/u);
    return match ? { value: Number.parseFloat(match[1]), unit: match[2] } : null;
}

function dimension(value) {
    return { value, unit: "px" };
}

function parseColor(value) {
    const match = (value ?? "").trim().match(/^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)(?:\s*,\s*([0-9.]+))?\s*\)$/iu);
    if (!match) return null;
    const [red, green, blue] = [match[1], match[2], match[3]].map(Number);
    const alpha = match[4] === undefined ? 1 : Number.parseFloat(match[4]);
    if (
        [red, green, blue, alpha].some((component) => !Number.isFinite(component)) ||
        [red, green, blue].some((component) => component < 0 || component > 255) ||
        alpha < 0 ||
        alpha > 1
    ) {
        return null;
    }
    const hex = `#${[red, green, blue].map((component) => component.toString(16).padStart(2, "0")).join("")}`;
    return { colorSpace: "srgb", components: [red / 255, green / 255, blue / 255], alpha, hex: hex.toUpperCase() };
}

function validateExactList(label, actual, expected) {
    if (!sameJson(actual, expected)) fail(`${label} must be exactly ${expected.join(", ")}`);
}

function sameJson(left, right) {
    return JSON.stringify(left) === JSON.stringify(right);
}

function fail(message) {
    failures.push(message);
}

async function readJson(filePath) {
    return JSON.parse(await fs.readFile(filePath, "utf8"));
}

function renderMarkdown(value) {
    const lines = [
        "# CEM Figma Foundations Report",
        "",
        "Generated from canonical CEM DTCG tokens and the checked-in `02 Foundations` inventory.",
        "",
        "## Summary",
        "",
        "| Evidence | Count |",
        "| --- | ---: |",
        `| Derived Effect Styles | ${value.summary.effectStyles} |`,
        `| Explicit no-effect specimens | ${value.summary.noEffectSpecimens} |`,
        `| Semantic layer aliases | ${value.summary.semanticAliases} |`,
        `| Derived motion specimens | ${value.summary.motionSpecimens} |`,
        `| Reviewed entries | ${value.summary.reviewed} |`,
        `| Planned entries | ${value.summary.planned} |`,
        "",
        "## Layering",
        "",
        "| Token | Representation | Style | Aliases | Status | Derived value |",
        "| --- | --- | --- | --- | --- | --- |",
        ...value.layering.map(
            (entry) =>
                `| \`${entry.token}\` | ${entry.representation} | ${entry.style ? `\`${entry.style}\`` : "—"} | ` +
                `${entry.semanticAliases.map((alias) => `\`${alias}\``).join(", ") || "—"} | ${entry.figma.status} | ` +
                `\`${JSON.stringify(entry.derivedValue)}\` |`
        ),
        "",
        "## Motion",
        "",
        "| Token | Specimen | Status | Derived cubic-Bézier |",
        "| --- | --- | --- | --- |",
        ...value.motion.map(
            (entry) =>
                `| \`${entry.token}\` | \`${entry.specimen}\` | ${entry.figma.status} | ` +
                `\`${JSON.stringify(entry.derivedValue)}\` |`
        ),
        "",
        "The generated values are review evidence only. The inventory remains raw-value-free, and live Figma review is required before any entry is promoted to reviewed.",
        "",
        "---",
        "",
        "> Generated by `verify-figma-foundations.mjs`. Do not edit by hand.",
        "",
    ];
    return lines.join("\n");
}
