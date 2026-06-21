/**
 * Smoke-test Figma propagation for one canonical token change.
 *
 * The live CEM UI Kit is governed by the manual/API sync policy, so this gate
 * stays offline: it proves the generated Figma mode files carry a source-token
 * change through the same variable paths used by the checked-in fixture.
 */

import fs from "node:fs/promises";
import path from "node:path";

const PACKAGE_ROOT = path.resolve(new URL("..", import.meta.url).pathname);
const WORKSPACE_ROOT = path.resolve(PACKAGE_ROOT, "../..");
const FIGMA_DIR = path.join(PACKAGE_ROOT, "dist/lib/tokens/figma");
const SAMPLE_FIXTURE = path.join(WORKSPACE_ROOT, "examples/figma/sample-token-application.md");

const MODES = ["light", "dark", "contrast-light", "contrast-dark", "native"];
const SOURCE_TOKEN = {
    path: "cem.color.cyan.xl",
    slashPath: "cem/color/cyan/xl",
    cssName: "--cem-color-cyan-xl",
    smokeValue: "#e8ffff",
};
const FIXTURE_BINDINGS = ["cem.palette.comfort", "cem.palette.comfort.text"];
const PROPAGATION = new Map([
    ["light", ["cem.color.cyan.xl", "cem.palette.comfort"]],
    ["dark", ["cem.color.cyan.xl", "cem.palette.comfort.text"]],
    ["contrast-light", ["cem.color.cyan.xl", "cem.palette.comfort"]],
    ["contrast-dark", ["cem.color.cyan.xl", "cem.palette.comfort.text"]],
    ["native", ["cem.color.cyan.xl"]],
]);

async function readJson(filePath) {
    return JSON.parse(await fs.readFile(filePath, "utf8"));
}

function cloneJson(value) {
    return JSON.parse(JSON.stringify(value));
}

function tokenAt(tree, tokenPath) {
    return tokenPath.split(".").reduce((node, part) => node?.[part], tree);
}

function setTokenValue(tree, tokenPath, value) {
    const token = tokenAt(tree, tokenPath);
    if (!token || typeof token !== "object" || !("$value" in token)) {
        throw new Error(`${tokenPath} is not a token leaf`);
    }
    token.$value = value;
}

function assert(condition, message, errors) {
    if (!condition) errors.push(message);
}

async function loadModeFiles(errors) {
    const files = new Map();
    for (const mode of MODES) {
        const filePath = path.join(FIGMA_DIR, `cem-${mode}.tokens.json`);
        try {
            files.set(mode, await readJson(filePath));
        } catch (err) {
            errors.push(`cannot read ${path.relative(WORKSPACE_ROOT, filePath)} (${err.message})`);
        }
    }
    return files;
}

async function validateFixtureBindings(errors) {
    let fixture;
    try {
        fixture = await fs.readFile(SAMPLE_FIXTURE, "utf8");
    } catch (err) {
        errors.push(`cannot read ${path.relative(WORKSPACE_ROOT, SAMPLE_FIXTURE)} (${err.message})`);
        return;
    }

    for (const binding of FIXTURE_BINDINGS) {
        const slashPath = binding.replaceAll(".", "/");
        assert(fixture.includes(slashPath), `sample fixture missing binding ${slashPath}`, errors);
    }
}

function validateSourceToken(files, errors) {
    for (const [mode, json] of files) {
        const token = tokenAt(json, SOURCE_TOKEN.path);
        assert(token, `${mode}: missing source token ${SOURCE_TOKEN.path}`, errors);
        assert(token?.$type === "color", `${mode}: ${SOURCE_TOKEN.path} is not a color token`, errors);
        assert(
            token?.$extensions?.cem?.cssName === SOURCE_TOKEN.cssName,
            `${mode}: ${SOURCE_TOKEN.path} cssName drifted`,
            errors
        );
    }
}

function validateFixturePropagation(files, errors) {
    const refreshed = new Map();
    for (const [mode, json] of files) {
        const next = cloneJson(json);
        for (const tokenPath of PROPAGATION.get(mode) ?? []) {
            setTokenValue(next, tokenPath, SOURCE_TOKEN.smokeValue);
        }
        refreshed.set(mode, next);
    }

    let changedFixtureBindings = 0;
    for (const mode of MODES) {
        const before = files.get(mode);
        const after = refreshed.get(mode);
        const expectedChanged = new Set(PROPAGATION.get(mode) ?? []);

        for (const tokenPath of expectedChanged) {
            const beforeToken = tokenAt(before, tokenPath);
            const afterToken = tokenAt(after, tokenPath);
            assert(beforeToken?.$value !== SOURCE_TOKEN.smokeValue, `${mode}: ${tokenPath} already has smoke value`, errors);
            assert(afterToken?.$value === SOURCE_TOKEN.smokeValue, `${mode}: ${tokenPath} did not receive smoke value`, errors);
            assert(afterToken?.$type === beforeToken?.$type, `${mode}: ${tokenPath} type changed during refresh`, errors);
            assert(
                afterToken?.$extensions?.cem?.cssName === beforeToken?.$extensions?.cem?.cssName,
                `${mode}: ${tokenPath} cssName changed during refresh`,
                errors
            );
        }

        for (const binding of FIXTURE_BINDINGS) {
            const beforeValue = tokenAt(before, binding)?.$value;
            const afterValue = tokenAt(after, binding)?.$value;
            if (expectedChanged.has(binding)) {
                changedFixtureBindings += 1;
                assert(beforeValue !== afterValue, `${mode}: fixture binding ${binding} did not change`, errors);
            } else {
                assert(beforeValue === afterValue, `${mode}: fixture binding ${binding} changed unexpectedly`, errors);
            }
        }
    }

    assert(changedFixtureBindings === 4, `expected 4 fixture binding value changes, found ${changedFixtureBindings}`, errors);
}

async function main() {
    const errors = [];
    const files = await loadModeFiles(errors);

    await validateFixtureBindings(errors);
    validateSourceToken(files, errors);
    validateFixturePropagation(files, errors);

    if (errors.length > 0) {
        for (const error of errors) console.error(`error: ${error}`);
        process.exit(1);
    }

    console.log(
        `smoke-figma-propagation: ${SOURCE_TOKEN.slashPath} -> ${SOURCE_TOKEN.smokeValue} reaches fixture bindings in generated Figma modes`
    );
}

main().catch((err) => {
    console.error(err);
    process.exit(2);
});
