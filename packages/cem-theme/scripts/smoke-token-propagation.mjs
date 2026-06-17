/**
 * End-to-end token propagation smoke for non-Figma platform outputs.
 *
 * The script temporarily changes one canonical markdown token, runs the normal
 * Nx token build, rewrites platform outputs, verifies the changed value reaches
 * CSS, JSON, Swift, and Android outputs, then restores the source and rebuilds.
 */

import { spawnSync } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";

const PACKAGE_ROOT = path.resolve(new URL("..", import.meta.url).pathname);
const WORKSPACE_ROOT = path.resolve(PACKAGE_ROOT, "../..");
const SOURCE_PATH = path.join(PACKAGE_ROOT, "src/lib/tokens/cem-colors.md");
const SMOKE_TOKEN = {
    cssName: "--cem-color-blue-xl",
    dtcgPath: "cem.color.blue.xl",
    swiftName: "cemColorBlueXl",
    androidName: "cem_color_blue_xl",
    smokeValue: "#dce8ff",
};

const MODES = ["light", "dark", "contrast-light", "contrast-dark", "native"];

function rel(filePath) {
    return path.relative(WORKSPACE_ROOT, filePath);
}

async function readText(filePath) {
    return fs.readFile(filePath, "utf8");
}

async function readJson(filePath) {
    return JSON.parse(await readText(filePath));
}

function runCommand(command, args, options = {}) {
    const result = spawnSync(command, args, {
        cwd: WORKSPACE_ROOT,
        env: { ...process.env, NX_DAEMON: "false" },
        stdio: "inherit",
        ...options,
    });
    if (result.status !== 0) {
        throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status ?? "unknown"}`);
    }
}

function runPlatformBuild(label) {
    console.log(`smoke-token-propagation: ${label}`);
    runCommand("yarn", ["nx", "run", "@epa-wg/cem-theme:build:tokens"]);
    // Run platform emission directly after build:tokens so Nx cache cannot
    // leave restored-source platform artifacts at the temporary smoke value.
    runCommand(process.execPath, ["scripts/build-token-platforms.mjs"], { cwd: PACKAGE_ROOT });
    runCommand(process.execPath, ["scripts/validate-platforms.mjs"], { cwd: PACKAGE_ROOT });
}

function escapeRegex(value) {
    return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function sourcePattern(currentValue) {
    const token = escapeRegex(SMOKE_TOKEN.cssName);
    const value = escapeRegex(currentValue);
    return new RegExp("(\\| `" + token + "`\\s+\\| blue\\s+\\| xl\\s+\\| )" + value + "(\\s+\\|)");
}

function currentTokenValue(source) {
    const token = escapeRegex(SMOKE_TOKEN.cssName);
    const match = source.match(new RegExp("\\| `" + token + "`\\s+\\| blue\\s+\\| xl\\s+\\| (#[0-9a-fA-F]{6})\\s+\\|"));
    if (!match) throw new Error(`cannot find ${SMOKE_TOKEN.cssName} source row in ${rel(SOURCE_PATH)}`);
    return match[1].toLowerCase();
}

function assert(condition, message, errors) {
    if (!condition) errors.push(message);
}

function assertContains(text, needle, filePath, errors) {
    assert(text.includes(needle), `${rel(filePath)} missing ${needle}`, errors);
}

function tokenAt(tree, tokenPath) {
    return tokenPath.split(".").reduce((node, part) => node?.[part], tree);
}

async function mutateSource(originalSource, originalValue) {
    if (originalValue === SMOKE_TOKEN.smokeValue) {
        throw new Error(`${SMOKE_TOKEN.cssName} already has smoke value ${SMOKE_TOKEN.smokeValue}`);
    }
    const next = originalSource.replace(sourcePattern(originalValue), `$1${SMOKE_TOKEN.smokeValue}$2`);
    if (next === originalSource) {
        throw new Error(`failed to replace ${SMOKE_TOKEN.cssName} ${originalValue} in ${rel(SOURCE_PATH)}`);
    }
    await fs.writeFile(SOURCE_PATH, next, "utf8");
}

async function validatePropagation(expectedValue, previousValue) {
    const errors = [];
    const cssPath = path.join(PACKAGE_ROOT, "dist/lib/css/cem-colors.css");
    const combinedCssPath = path.join(PACKAGE_ROOT, "dist/lib/css/cem-combined.css");
    const xhtmlPath = path.join(PACKAGE_ROOT, "dist/lib/tokens/cem-colors.xhtml");
    const canonicalPath = path.join(PACKAGE_ROOT, "dist/lib/tokens/cem.tokens.json");
    const resolvedPath = path.join(PACKAGE_ROOT, "dist/lib/tokens/cem.tokens.resolved.json");
    const tsPath = path.join(PACKAGE_ROOT, "dist/lib/tokens/cem.tokens.ts");
    const swiftPath = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/ios/CEMTokens.swift");
    const iosHintsPath = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/ios/CEMTokens.xcassets-hints.json");
    const androidLightPath = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/android/values/cem-tokens.xml");
    const androidDarkPath = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/android/values-night/cem-tokens.xml");
    const composePath = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/android/compose/CEMTokens.kt");
    const androidReportPath = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/android/android-report.md");
    const iosReportPath = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/ios/ios-report.md");

    const source = await readText(SOURCE_PATH);
    assertContains(source, `${SMOKE_TOKEN.cssName}\`                 | blue   | xl      | ${expectedValue}`, SOURCE_PATH, errors);

    const css = await readText(cssPath);
    const combinedCss = await readText(combinedCssPath);
    const xhtml = await readText(xhtmlPath);
    assertContains(css, `${SMOKE_TOKEN.cssName}: ${expectedValue};`, cssPath, errors);
    assertContains(combinedCss, `${SMOKE_TOKEN.cssName}: ${expectedValue};`, combinedCssPath, errors);
    assertContains(xhtml, `<td>${expectedValue}</td>`, xhtmlPath, errors);

    const canonical = await readJson(canonicalPath);
    const canonicalToken = tokenAt(canonical, SMOKE_TOKEN.dtcgPath);
    assert(canonicalToken?.$value === expectedValue, `${rel(canonicalPath)} ${SMOKE_TOKEN.dtcgPath} did not update`, errors);
    for (const mode of MODES) {
        assert(
            canonicalToken?.$extensions?.cem?.modes?.[mode] === expectedValue,
            `${rel(canonicalPath)} ${SMOKE_TOKEN.dtcgPath} ${mode} mode did not update`,
            errors,
        );
    }

    const resolved = await readJson(resolvedPath);
    const resolvedToken = resolved.tokens?.find((token) => token.name === SMOKE_TOKEN.cssName);
    assert(resolvedToken?.valueRaw === expectedValue, `${rel(resolvedPath)} ${SMOKE_TOKEN.cssName} raw value did not update`, errors);
    for (const mode of MODES) {
        assert(
            resolvedToken?.valueByMode?.[mode] === expectedValue,
            `${rel(resolvedPath)} ${SMOKE_TOKEN.cssName} ${mode} mode did not update`,
            errors,
        );
    }

    const ts = await readText(tsPath);
    assertContains(ts, `name: "${SMOKE_TOKEN.cssName}"`, tsPath, errors);
    assertContains(ts, `rawValue: "${expectedValue}"`, tsPath, errors);

    for (const mode of MODES) {
        const modePath = path.join(PACKAGE_ROOT, `dist/lib/token-platforms/json/cem-tokens-${mode}.json`);
        const modeJson = await readJson(modePath);
        assert(
            modeJson.tokens?.[SMOKE_TOKEN.dtcgPath]?.value === expectedValue,
            `${rel(modePath)} ${SMOKE_TOKEN.dtcgPath} did not update`,
            errors,
        );
    }

    const swift = await readText(swiftPath);
    const swiftMatches = swift.match(new RegExp(`public static let ${SMOKE_TOKEN.swiftName} = "${expectedValue}"`, "g")) ?? [];
    assert(swiftMatches.length === MODES.length, `${rel(swiftPath)} expected ${MODES.length} updated Swift constants, found ${swiftMatches.length}`, errors);

    const iosHints = await readJson(iosHintsPath);
    for (const mode of MODES) {
        assert(
            iosHints.colors?.[SMOKE_TOKEN.dtcgPath]?.modes?.[mode] === expectedValue,
            `${rel(iosHintsPath)} ${SMOKE_TOKEN.dtcgPath} ${mode} mode did not update`,
            errors,
        );
    }

    const androidLight = await readText(androidLightPath);
    const androidDark = await readText(androidDarkPath);
    const compose = await readText(composePath);
    assertContains(androidLight, `<color name="${SMOKE_TOKEN.androidName}">${expectedValue}</color>`, androidLightPath, errors);
    assertContains(androidDark, `<color name="${SMOKE_TOKEN.androidName}">${expectedValue}</color>`, androidDarkPath, errors);
    assertContains(compose, `const val ${SMOKE_TOKEN.swiftName} = "${expectedValue}"`, composePath, errors);

    const androidReport = await readText(androidReportPath);
    const iosReport = await readText(iosReportPath);
    assertContains(androidReport, "| Fail-hard violations | 0 |", androidReportPath, errors);
    assertContains(iosReport, "| Fail-hard violations | 0 |", iosReportPath, errors);

    if (previousValue) {
        for (const [filePath, text] of [
            [cssPath, css],
            [combinedCssPath, combinedCss],
            [xhtmlPath, xhtml],
            [canonicalPath, JSON.stringify(canonical)],
            [resolvedPath, JSON.stringify(resolved)],
            [tsPath, ts],
            [swiftPath, swift],
            [iosHintsPath, JSON.stringify(iosHints)],
            [androidLightPath, androidLight],
            [androidDarkPath, androidDark],
            [composePath, compose],
        ]) {
            assert(!text.includes(previousValue), `${rel(filePath)} still contains previous value ${previousValue}`, errors);
        }
    }

    if (errors.length > 0) {
        for (const error of errors) console.error(`error: ${error}`);
        throw new Error(`propagation validation failed for ${expectedValue}`);
    }
}

async function main() {
    const originalSource = await readText(SOURCE_PATH);
    const originalValue = currentTokenValue(originalSource);
    let smokeFailed = null;

    try {
        await mutateSource(originalSource, originalValue);
        runPlatformBuild(`building with ${SMOKE_TOKEN.cssName}=${SMOKE_TOKEN.smokeValue}`);
        await validatePropagation(SMOKE_TOKEN.smokeValue, originalValue);
    } catch (err) {
        smokeFailed = err;
    } finally {
        await fs.writeFile(SOURCE_PATH, originalSource, "utf8");
        runPlatformBuild(`restoring ${SMOKE_TOKEN.cssName}=${originalValue}`);
    }

    await validatePropagation(originalValue, SMOKE_TOKEN.smokeValue);

    if (smokeFailed) throw smokeFailed;

    console.log(
        `smoke-token-propagation: ${SMOKE_TOKEN.cssName} ${originalValue} -> ${SMOKE_TOKEN.smokeValue} reached CSS, JSON, Swift, and Android outputs`
    );
}

main().catch((err) => {
    console.error(err);
    process.exit(1);
});
