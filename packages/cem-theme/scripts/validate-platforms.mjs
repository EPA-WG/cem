/**
 * Validate generated token platform JSON outputs.
 */

import fs from "node:fs/promises";
import path from "node:path";
import { CEM_PLATFORM_MODES } from "../style-dictionary.config.mjs";

const PACKAGE_ROOT = path.resolve(new URL("..", import.meta.url).pathname);
const OUT_JSON = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/json");

async function readJson(filePath) {
    return JSON.parse(await fs.readFile(filePath, "utf8"));
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

    return { errors, tokenCount: firstEntries.size, modeCount: files.size };
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
