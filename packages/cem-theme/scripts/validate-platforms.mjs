/**
 * Validate generated token platform JSON outputs.
 */

import fs from "node:fs/promises";
import path from "node:path";
import { CEM_PLATFORM_MODES } from "../style-dictionary.config.mjs";

const PACKAGE_ROOT = path.resolve(new URL("..", import.meta.url).pathname);
const OUT_JSON = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/json");
const OUT_IOS = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/ios");
const OUT_ANDROID = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/android");

async function readJson(filePath) {
    return JSON.parse(await fs.readFile(filePath, "utf8"));
}

async function readText(filePath, errors) {
    try {
        return await fs.readFile(filePath, "utf8");
    } catch (err) {
        errors.push(`cannot read ${path.relative(PACKAGE_ROOT, filePath)} (${err.message})`);
        return "";
    }
}

function validateModeFile(mode, json, errors) {
    if (json.mode !== mode) errors.push(`${mode}: expected mode "${mode}", found "${json.mode}"`);
    if (!json.$generated?.generator) errors.push(`${mode}: missing generated provenance`);
    if (!json.tokens || typeof json.tokens !== "object") errors.push(`${mode}: missing tokens object`);

    for (const [tokenPath, token] of Object.entries(json.tokens ?? {})) {
        if (!token.name?.startsWith("--cem-")) errors.push(`${mode}: ${tokenPath} missing CSS token name`);
        if (!token.type) errors.push(`${mode}: ${tokenPath} missing type`);
        if (token.value === undefined || token.value === "") errors.push(`${mode}: ${tokenPath} has empty value`);
    }
}

async function validatePlatforms() {
    const errors = [];
    const files = new Map();

    for (const mode of CEM_PLATFORM_MODES) {
        const filePath = path.join(OUT_JSON, `cem-tokens-${mode}.json`);
        try {
            files.set(mode, await readJson(filePath));
        } catch (err) {
            errors.push(`${mode}: cannot read ${path.relative(PACKAGE_ROOT, filePath)} (${err.message})`);
        }
    }

    for (const [mode, json] of files) validateModeFile(mode, json, errors);

    const first = files.get(CEM_PLATFORM_MODES[0]);
    const firstEntries = new Map(
        Object.entries(first?.tokens ?? {}).map(([tokenPath, token]) => [tokenPath, token.type]),
    );

    for (const mode of CEM_PLATFORM_MODES.slice(1)) {
        const entries = new Map(Object.entries(files.get(mode)?.tokens ?? {}).map(([tokenPath, token]) => [tokenPath, token.type]));
        for (const [tokenPath, type] of firstEntries) {
            if (!entries.has(tokenPath)) errors.push(`${mode}: missing token ${tokenPath}`);
            else if (entries.get(tokenPath) !== type) {
                errors.push(`${mode}: token ${tokenPath} type mismatch (${entries.get(tokenPath)} vs ${type})`);
            }
        }
        for (const tokenPath of entries.keys()) {
            if (!firstEntries.has(tokenPath)) errors.push(`${mode}: extra token ${tokenPath}`);
        }
    }

    await validateIos(errors);
    await validateAndroid(errors);

    return { errors, tokenCount: firstEntries.size, modeCount: files.size };
}

async function validateIos(errors) {
    const swiftPath = path.join(OUT_IOS, "CEMTokens.swift");
    const hintsPath = path.join(OUT_IOS, "CEMTokens.xcassets-hints.json");
    const reportPath = path.join(OUT_IOS, "ios-report.md");

    const swift = await readText(swiftPath, errors);
    if (swift && !swift.includes("public enum CEMTokens")) errors.push("ios: CEMTokens.swift missing CEMTokens enum");
    if (swift && !swift.includes("public enum Light")) errors.push("ios: CEMTokens.swift missing Light mode enum");
    if (swift && !swift.includes("public enum Dark")) errors.push("ios: CEMTokens.swift missing Dark mode enum");

    try {
        const hints = await readJson(hintsPath);
        if (!hints.$generated?.generator) errors.push("ios: CEMTokens.xcassets-hints.json missing provenance");
        if (!hints.colors || typeof hints.colors !== "object") errors.push("ios: CEMTokens.xcassets-hints.json missing colors");
    } catch (err) {
        errors.push(`ios: cannot parse CEMTokens.xcassets-hints.json (${err.message})`);
    }

    const report = await readText(reportPath, errors);
    if (report && !report.includes("| Fail-hard violations | 0 |")) {
        errors.push("ios: report does not show zero fail-hard violations");
    }
}

function resourceNames(xml, label, errors) {
    if (!xml.includes("<resources>") || !xml.includes("</resources>")) {
        errors.push(`${label}: missing <resources> root`);
    }

    const names = [];
    const nameRe = /\bname="([^"]+)"/g;
    let match;
    while ((match = nameRe.exec(xml)) !== null) names.push(match[1]);

    const seen = new Set();
    for (const name of names) {
        if (!/^[a-z][a-z0-9_]*$/.test(name)) errors.push(`${label}: invalid resource name ${name}`);
        if (seen.has(name)) errors.push(`${label}: duplicate resource name ${name}`);
        seen.add(name);
    }

    return names.length;
}

async function validateAndroid(errors) {
    const lightPath = path.join(OUT_ANDROID, "values/cem-tokens.xml");
    const darkPath = path.join(OUT_ANDROID, "values-night/cem-tokens.xml");
    const composePath = path.join(OUT_ANDROID, "compose/CEMTokens.kt");
    const reportPath = path.join(OUT_ANDROID, "android-report.md");

    const light = await readText(lightPath, errors);
    const dark = await readText(darkPath, errors);
    const lightCount = resourceNames(light, "android values", errors);
    const darkCount = resourceNames(dark, "android values-night", errors);
    if (lightCount === 0) errors.push("android: values/cem-tokens.xml has no resources");
    if (darkCount === 0) errors.push("android: values-night/cem-tokens.xml has no resources");

    const compose = await readText(composePath, errors);
    if (compose && !compose.includes("object CEMTokens")) errors.push("android: compose/CEMTokens.kt missing CEMTokens object");

    const report = await readText(reportPath, errors);
    if (report && !report.includes("| Fail-hard violations | 0 |")) {
        errors.push("android: report does not show zero fail-hard violations");
    }
}

async function main() {
    const { errors, tokenCount, modeCount } = await validatePlatforms();
    if (errors.length > 0) {
        for (const error of errors) console.error(`error: ${error}`);
        process.exit(1);
    }

    console.log(`validate-platforms: ${tokenCount} tokens consistent across ${modeCount} JSON mode files`);
}

main().catch((err) => {
    console.error(err);
    process.exit(2);
});
