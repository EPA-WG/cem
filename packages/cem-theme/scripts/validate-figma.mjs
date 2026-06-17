/**
 * Validate generated Figma token mode files and checked-in native-library evidence.
 *
 * This is intentionally offline. The native CEM UI Kit is refreshed from the
 * generated figma/cem-*.tokens.json files, and CI should not require Figma API
 * credentials to prove the release artifact contract.
 */

import fs from "node:fs/promises";
import path from "node:path";

const PACKAGE_ROOT = path.resolve(new URL("..", import.meta.url).pathname);
const WORKSPACE_ROOT = path.resolve(PACKAGE_ROOT, "../..");
const FIGMA_DIR = path.join(PACKAGE_ROOT, "dist/lib/tokens/figma");
const EXAMPLES_FIGMA = path.join(WORKSPACE_ROOT, "examples/figma");
const EXPECTED_MODES = ["light", "dark", "contrast-light", "contrast-dark", "native"];
const EXPECTED_MODE_LABELS = ["Light", "Dark", "Contrast Light", "Contrast Dark", "Native"];
const EXPECTED_COLLECTION = "CEM Tokens";
const REQUIRED_ALIASES = new Map([
    ["cem.zebra.color.0", "cem.palette.comfort"],
    ["cem.gap.block", "cem.dim.medium"],
    ["cem.layout.stack.gap", "cem.gap.block"],
    ["cem.bend.smooth", "cem.dim.x.small"],
    ["cem.bend.control", "cem.bend"],
]);
const REQUIRED_WEB_SYNTAX = new Map([
    ["cem.palette.comfort", "--cem-palette-comfort"],
    ["cem.dim.medium", "--cem-dim-medium"],
    ["cem.bend.control", "--cem-bend-control"],
]);

async function readJson(filePath) {
    return JSON.parse(await fs.readFile(filePath, "utf8"));
}

async function readText(filePath, errors) {
    try {
        return await fs.readFile(filePath, "utf8");
    } catch (err) {
        errors.push(`cannot read ${path.relative(WORKSPACE_ROOT, filePath)} (${err.message})`);
        return "";
    }
}

function walkTokens(node, prefix = [], out = new Map()) {
    if (!node || typeof node !== "object") return out;
    for (const [key, value] of Object.entries(node)) {
        if (key.startsWith("$")) continue;
        if (!value || typeof value !== "object") continue;
        const next = [...prefix, key];
        if ("$value" in value) {
            out.set(next.join("."), value);
        }
        walkTokens(value, next, out);
    }
    return out;
}

function isTokenReference(value) {
    return typeof value === "string" && /^\{cem(\.[a-z0-9]+)+\}$/.test(value);
}

function expectedFigmaType(dtcgType) {
    switch (dtcgType) {
        case "color":
            return "COLOR";
        case "dimension":
        case "number":
            return "FLOAT";
        case "duration":
        case "fontFamily":
        case "string":
            return "STRING";
        default:
            return null;
    }
}

function valueMatchesType(type, value) {
    if (isTokenReference(value)) return true;
    switch (type) {
        case "color":
            return typeof value === "string" && /^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(value);
        case "dimension":
            return typeof value === "string" && /^-?\d+(\.\d+)?(px|rem)?$/.test(value);
        case "number":
            return typeof value === "number";
        case "duration":
            return typeof value === "string" && /^-?\d+(\.\d+)?(ms|s)$/.test(value);
        case "fontFamily":
        case "string":
            return typeof value === "string" && value.length > 0;
        default:
            return false;
    }
}

function validateModeFile(mode, json, errors) {
    const generated = json.$extensions?.cem?.generated;
    if (generated?.mode !== mode) {
        errors.push(`${mode}: generated mode mismatch (${generated?.mode ?? "missing"})`);
    }
    if (generated?.workflow !== "Tokens Studio pull-only into one CEM collection; write-back disabled") {
        errors.push(`${mode}: missing read-only Figma workflow provenance`);
    }

    const tokens = walkTokens(json);
    for (const [tokenPath, token] of tokens) {
        const cssName = token.$extensions?.cem?.cssName;
        if (!cssName?.startsWith("--cem-")) errors.push(`${mode}: ${tokenPath} missing cssName`);
        if (!token.$type) errors.push(`${mode}: ${tokenPath} missing $type`);
        if (!expectedFigmaType(token.$type)) errors.push(`${mode}: ${tokenPath} has unsupported Figma type ${token.$type}`);
        if (token.$value === undefined || token.$value === null || token.$value === "") {
            errors.push(`${mode}: ${tokenPath} has empty $value`);
        } else if (!valueMatchesType(token.$type, token.$value)) {
            errors.push(`${mode}: ${tokenPath} value ${JSON.stringify(token.$value)} does not match ${token.$type}`);
        }
    }
    return tokens;
}

function validateModeConsistency(files, errors) {
    const firstMode = EXPECTED_MODES[0];
    const firstEntries = files.get(firstMode) ?? new Map();
    for (const mode of EXPECTED_MODES.slice(1)) {
        const entries = files.get(mode) ?? new Map();
        for (const [tokenPath, token] of firstEntries) {
            const other = entries.get(tokenPath);
            if (!other) {
                errors.push(`${mode}: missing token ${tokenPath}`);
                continue;
            }
            if (other.$type !== token.$type) {
                errors.push(`${mode}: ${tokenPath} type mismatch (${other.$type} vs ${token.$type})`);
            }
        }
        for (const tokenPath of entries.keys()) {
            if (!firstEntries.has(tokenPath)) errors.push(`${mode}: extra token ${tokenPath}`);
        }
    }
}

function validateAliases(tokens, errors) {
    for (const [tokenPath, targetPath] of REQUIRED_ALIASES) {
        const token = tokens.get(tokenPath);
        if (!token) {
            errors.push(`alias check: missing ${tokenPath}`);
            continue;
        }
        const expected = `{${targetPath}}`;
        if (token.$value !== expected) {
            errors.push(`alias check: ${tokenPath} expected ${expected}, found ${JSON.stringify(token.$value)}`);
        }
    }
}

function validateWebSyntax(tokens, errors) {
    for (const [tokenPath, cssName] of REQUIRED_WEB_SYNTAX) {
        const token = tokens.get(tokenPath);
        if (!token) {
            errors.push(`code syntax check: missing ${tokenPath}`);
            continue;
        }
        if (token.$extensions?.cem?.cssName !== cssName) {
            errors.push(`code syntax check: ${tokenPath} expected ${cssName}, found ${token.$extensions?.cem?.cssName ?? "missing"}`);
        }
    }
}

async function validateEvidence(errors) {
    const readmePath = path.join(EXAMPLES_FIGMA, "README.md");
    const samplePath = path.join(EXAMPLES_FIGMA, "sample-token-application.md");
    const readme = await readText(readmePath, errors);
    const sample = await readText(samplePath, errors);

    for (const label of [EXPECTED_COLLECTION, ...EXPECTED_MODE_LABELS]) {
        if (!readme.includes(label)) errors.push(`examples/figma/README.md missing native evidence for ${label}`);
    }
    if (!readme.includes("Variable count: 230")) {
        errors.push("examples/figma/README.md missing expected native variable count evidence");
    }
    if (!readme.includes("Missing mode values: 0")) {
        errors.push("examples/figma/README.md missing zero missing-mode-values evidence");
    }
    if (!readme.includes("Variable aliases present:")) {
        errors.push("examples/figma/README.md missing alias evidence");
    }

    for (const required of ["cem/palette/comfort", "cem/palette/comfort/text", "cem/bend/control", "cem/inset/control"]) {
        if (!sample.includes(required)) errors.push(`sample-token-application.md missing ${required}`);
    }
}

async function validateFigma() {
    const errors = [];
    const files = new Map();

    for (const mode of EXPECTED_MODES) {
        const filePath = path.join(FIGMA_DIR, `cem-${mode}.tokens.json`);
        try {
            const json = await readJson(filePath);
            files.set(mode, validateModeFile(mode, json, errors));
        } catch (err) {
            errors.push(`${mode}: cannot read ${path.relative(WORKSPACE_ROOT, filePath)} (${err.message})`);
        }
    }

    validateModeConsistency(files, errors);
    const lightTokens = files.get("light") ?? new Map();
    validateAliases(lightTokens, errors);
    validateWebSyntax(lightTokens, errors);

    const report = await readText(path.join(FIGMA_DIR, "cem-figma-report.md"), errors);
    if (report && !report.includes("| Errors | 0 |")) {
        errors.push("cem-figma-report.md does not show zero errors");
    }

    await validateEvidence(errors);

    return {
        errors,
        tokenCount: lightTokens.size,
        modeCount: files.size,
    };
}

async function main() {
    const { errors, tokenCount, modeCount } = await validateFigma();
    if (errors.length > 0) {
        for (const error of errors) console.error(`error: ${error}`);
        process.exit(1);
    }
    console.log(`validate-figma: ${tokenCount} tokens consistent across ${modeCount} Figma mode files`);
}

main().catch((err) => {
    console.error(err);
    process.exit(2);
});
