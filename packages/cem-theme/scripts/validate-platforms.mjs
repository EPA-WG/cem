/**
 * Validate generated token platform JSON outputs.
 */

import fs from "node:fs/promises";
import path from "node:path";
import {
    CEM_PLATFORM_MODES,
    CEM_STYLE_DICTIONARY_FILTERS,
    CEM_STYLE_DICTIONARY_FILTER_DEFINITIONS,
    CEM_STYLE_DICTIONARY_TRANSFORMS,
    CEM_STYLE_DICTIONARY_TRANSFORM_DEFINITIONS,
    cemTransformValue,
} from "../style-dictionary.config.mjs";
import { CEM_NATIVE_PLATFORM_CONTRACT } from "./native-platform-contract.mjs";

const PACKAGE_ROOT = path.resolve(new URL("..", import.meta.url).pathname);
const OUT_JSON = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/json");
const OUT_IOS = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/ios");
const OUT_ANDROID = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/android");
const WORKSPACE_ROOT = path.resolve(PACKAGE_ROOT, "../..");

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

    validateStyleDictionaryContract(errors);

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

function validateStyleDictionaryContract(errors) {
    for (const name of CEM_STYLE_DICTIONARY_TRANSFORMS) {
        const definition = CEM_STYLE_DICTIONARY_TRANSFORM_DEFINITIONS[name];
        if (!definition) errors.push(`style-dictionary: missing transform definition ${name}`);
        if (definition && typeof definition.transform !== "function") {
            errors.push(`style-dictionary: transform ${name} missing transform function`);
        }
    }

    for (const name of CEM_STYLE_DICTIONARY_FILTERS) {
        const definition = CEM_STYLE_DICTIONARY_FILTER_DEFINITIONS[name];
        if (!definition) errors.push(`style-dictionary: missing filter definition ${name}`);
        if (definition && typeof definition.filter !== "function") {
            errors.push(`style-dictionary: filter ${name} missing filter function`);
        }
    }

    const layoutToken = {
        $type: "dimension",
        $value: "1rem",
        $extensions: { cem: { cssName: "--cem-dim-medium", spec: "cem-dimension", sourceTable: "cem-dim-scale" } },
    };
    const typeToken = {
        $type: "dimension",
        $value: "20px",
        $extensions: {
            cem: {
                cssName: "--cem-typography-size-l",
                spec: "cem-voice-fonts-typography",
                sourceTable: "cem-typography-size",
            },
        },
    };
    const numberToken = {
        $type: "fontWeight",
        $value: "700",
        $extensions: { cem: { cssName: "--cem-thickness-bold", spec: "cem-voice-fonts-typography" } },
    };
    const modeToken = {
        $type: "color",
        $value: "#ffffff",
        $extensions: { cem: { cssName: "--cem-palette-comfort", modes: { dark: "#000000" } } },
    };
    const voiceToken = {
        $type: "string",
        $value: "medium",
        $extensions: { cem: { cssName: "--cem-typography-reading-speech-rate" } },
    };

    const assertions = [
        ["style-dictionary: layout-to-dp rem conversion", cemTransformValue("cem/size/layout-to-dp", layoutToken), "16dp"],
        ["style-dictionary: layout-to-pt rem conversion", cemTransformValue("cem/size/layout-to-pt", layoutToken), "16pt"],
        ["style-dictionary: type-to-sp px conversion", cemTransformValue("cem/size/type-to-sp", typeToken), "20sp"],
        ["style-dictionary: type-to-pt px conversion", cemTransformValue("cem/size/type-to-pt", typeToken), "20pt"],
        ["style-dictionary: mode expansion", cemTransformValue("cem/mode/expand-themes", modeToken, { mode: "dark" }), "#000000"],
    ];

    for (const [label, actual, expected] of assertions) {
        if (actual !== expected) errors.push(`${label}: expected ${expected}, found ${actual}`);
    }

    if (cemTransformValue("cem/number/unitless", numberToken) !== 700) {
        errors.push("style-dictionary: number/unitless did not return a number");
    }

    const webOnlyFilter = CEM_STYLE_DICTIONARY_FILTER_DEFINITIONS["cem/category/web-only-filter"].filter;
    if (webOnlyFilter(voiceToken) !== false) {
        errors.push("style-dictionary: web-only filter did not reject voice/audio token");
    }
}

async function validateIos(errors) {
    const swiftPath = path.join(OUT_IOS, "CEMTokens.swift");
    const packageSwiftPath = path.join(OUT_IOS, "Sources/CEMTokens/CEMTokens.swift");
    const packageManifestPath = path.join(OUT_IOS, "Package.swift");
    const examplePath = path.join(OUT_IOS, "Examples/CEMTokensExampleApp.swift");
    const hintsPath = path.join(OUT_IOS, "CEMTokens.xcassets-hints.json");
    const reportPath = path.join(OUT_IOS, "ios-report.md");
    const contract = CEM_NATIVE_PLATFORM_CONTRACT.ios;

    const swift = await readText(swiftPath, errors);
    if (swift && !swift.includes("public enum CEMTokens")) errors.push("ios: CEMTokens.swift missing CEMTokens enum");
    if (swift && !swift.includes("public enum Light")) errors.push("ios: CEMTokens.swift missing Light mode enum");
    if (swift && !swift.includes("public enum Dark")) errors.push("ios: CEMTokens.swift missing Dark mode enum");
    const packageSwift = await readText(packageSwiftPath, errors);
    if (swift && packageSwift && swift !== packageSwift) {
        errors.push("ios: standalone CEMTokens.swift drifted from Swift Package source");
    }
    const manifest = await readText(packageManifestPath, errors);
    for (const expected of [
        `// swift-tools-version: ${contract.swiftToolsVersion}`,
        `name: "${contract.packageName}"`,
        `.iOS(.v${contract.iosDeploymentTarget.replace(".0", "")})`,
        `swiftLanguageModes: [.v${contract.swiftLanguageMode}]`,
    ]) {
        if (manifest && !manifest.includes(expected)) errors.push(`ios: Package.swift missing ${expected}`);
    }
    const example = await readText(examplePath, errors);
    if (example && !example.includes("import CEMTokens")) errors.push("ios: SwiftUI fixture does not import CEMTokens");
    if (example && !example.includes("CEMTokens.Light.")) errors.push("ios: SwiftUI fixture does not consume generated tokens");

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
    if (report && !report.includes(`Xcode ${contract.xcodeVersion}`)) {
        errors.push("ios: report does not name the supported Xcode contract");
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
    const libraryLightPath = path.join(OUT_ANDROID, "cem-tokens/src/main/res/values/cem-tokens.xml");
    const libraryDarkPath = path.join(OUT_ANDROID, "cem-tokens/src/main/res/values-night/cem-tokens.xml");
    const libraryKotlinPath = path.join(OUT_ANDROID, "cem-tokens/src/main/kotlin/org/epawg/cem/tokens/CEMTokens.kt");
    const settingsPath = path.join(OUT_ANDROID, "settings.gradle.kts");
    const rootBuildPath = path.join(OUT_ANDROID, "build.gradle.kts");
    const libraryBuildPath = path.join(OUT_ANDROID, "cem-tokens/build.gradle.kts");
    const sampleBuildPath = path.join(OUT_ANDROID, "sample/build.gradle.kts");
    const samplePath = path.join(OUT_ANDROID, "sample/src/main/kotlin/org/epawg/cem/example/MainActivity.kt");
    const contract = CEM_NATIVE_PLATFORM_CONTRACT.android;

    const light = await readText(lightPath, errors);
    const dark = await readText(darkPath, errors);
    const lightCount = resourceNames(light, "android values", errors);
    const darkCount = resourceNames(dark, "android values-night", errors);
    if (lightCount === 0) errors.push("android: values/cem-tokens.xml has no resources");
    if (darkCount === 0) errors.push("android: values-night/cem-tokens.xml has no resources");

    const compose = await readText(composePath, errors);
    if (compose && !compose.includes("object CEMTokens")) errors.push("android: compose/CEMTokens.kt missing CEMTokens object");
    const libraryLight = await readText(libraryLightPath, errors);
    const libraryDark = await readText(libraryDarkPath, errors);
    const libraryKotlin = await readText(libraryKotlinPath, errors);
    if (light && libraryLight && light !== libraryLight) errors.push("android: standalone light XML drifted from library resource");
    if (dark && libraryDark && dark !== libraryDark) errors.push("android: standalone night XML drifted from library resource");
    if (compose && libraryKotlin && compose !== libraryKotlin) errors.push("android: standalone Kotlin constants drifted from library source");

    const settings = await readText(settingsPath, errors);
    if (settings && !settings.includes('include(":cem-tokens", ":sample")')) {
        errors.push("android: settings.gradle.kts does not include library and sample modules");
    }
    const rootBuild = await readText(rootBuildPath, errors);
    for (const expected of [
        `com.android.application") version "${contract.androidGradlePluginVersion}`,
        `com.android.library") version "${contract.androidGradlePluginVersion}`,
        `org.jetbrains.kotlin.plugin.compose") version "${contract.kotlinVersion}`,
    ]) {
        if (rootBuild && !rootBuild.includes(expected)) errors.push(`android: root build missing ${expected}`);
    }
    const libraryBuild = await readText(libraryBuildPath, errors);
    if (libraryBuild && !libraryBuild.includes(`compileSdk = ${contract.compileSdk}`)) {
        errors.push("android: library build uses the wrong compileSdk");
    }
    const sampleBuild = await readText(sampleBuildPath, errors);
    for (const expected of [
        `compose-bom:${contract.composeBomVersion}`,
        `activity-compose:${contract.activityComposeVersion}`,
        "implementation(project(\":cem-tokens\"))",
    ]) {
        if (sampleBuild && !sampleBuild.includes(expected)) errors.push(`android: sample build missing ${expected}`);
    }
    const sample = await readText(samplePath, errors);
    if (sample && !sample.includes("org.epawg.cem.tokens.CEMTokens")) {
        errors.push("android: Compose fixture does not import CEMTokens");
    }
    const checkedInSample = await readText(
        path.join(WORKSPACE_ROOT, "examples/android/cem-tokens-example/MainActivity.kt"),
        errors,
    );
    if (sample && checkedInSample && sample !== checkedInSample) {
        errors.push("android: generated Compose fixture drifted from checked-in example");
    }

    const report = await readText(reportPath, errors);
    if (report && !report.includes("| Fail-hard violations | 0 |")) {
        errors.push("android: report does not show zero fail-hard violations");
    }
    if (report && !report.includes(`AGP ${contract.androidGradlePluginVersion}`)) {
        errors.push("android: report does not name the supported AGP contract");
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
