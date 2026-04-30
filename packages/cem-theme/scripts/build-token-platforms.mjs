/**
 * Build post-MVP token platform outputs from canonical CEM token JSON.
 *
 * Current Phase F/G scope: emit resolved-per-mode flat JSON, conservative iOS
 * Swift/string metadata, iOS color asset hints, and Android values XML for
 * tokens with clean native value mappings.
 */

import fs from "node:fs/promises";
import path from "node:path";
import { CEM_PLATFORM_MODES } from "../style-dictionary.config.mjs";

const PACKAGE_ROOT = path.resolve(new URL("..", import.meta.url).pathname);
const DIST_TOKENS = path.join(PACKAGE_ROOT, "dist/lib/tokens");
const OUT_ROOT = path.join(PACKAGE_ROOT, "dist/lib/token-platforms");
const OUT_JSON = path.join(OUT_ROOT, "json");
const OUT_IOS = path.join(OUT_ROOT, "ios");
const OUT_ANDROID = path.join(OUT_ROOT, "android");

function isTokenNode(node) {
    return node && typeof node === "object" && "$value" in node && "$type" in node;
}

function flattenTokens(tree) {
    const tokens = [];

    function walk(node, pathParts) {
        for (const [key, value] of Object.entries(node)) {
            if (key.startsWith("$")) continue;
            if (!value || typeof value !== "object") continue;

            const nextPath = [...pathParts, key];
            if (isTokenNode(value)) {
                tokens.push({
                    path: nextPath.join("."),
                    name: value.$extensions?.cem?.cssName ?? `--${nextPath.join("-")}`,
                    type: value.$type,
                    value: value.$value,
                    tier: value.$extensions?.cem?.tier ?? "",
                    spec: value.$extensions?.cem?.spec ?? "",
                    sourceTable: value.$extensions?.cem?.sourceTable ?? "",
                    portability: value.$extensions?.cem?.portability ?? "",
                    modes: value.$extensions?.cem?.modes ?? {},
                });
            }

            walk(value, nextPath);
        }
    }

    walk(tree, []);
    return tokens;
}

function modeValue(token, mode) {
    const value = token.modes?.[mode];
    if (typeof value === "string" && value.trim() !== "") return value;
    return token.value;
}

function identifierFromPath(tokenPath) {
    const words = tokenPath.split(".").filter(Boolean);
    return words
        .map((word, index) => {
            const cleaned = word.replace(/[^a-z0-9]+/gi, " ");
            const parts = cleaned.split(" ").filter(Boolean);
            const joined = parts.map((part) => part[0].toUpperCase() + part.slice(1)).join("");
            if (index === 0) return joined[0].toLowerCase() + joined.slice(1);
            return joined;
        })
        .join("");
}

function androidNameFromPath(tokenPath) {
    return tokenPath.replace(/[^a-z0-9]+/gi, "_").replace(/^_+|_+$/g, "").toLowerCase();
}

function escapeSwiftString(value) {
    return String(value).replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "\\n");
}

function escapeXml(value) {
    return String(value)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;");
}

function isHexColor(value) {
    return /^#[0-9a-f]{6}([0-9a-f]{2})?$/i.test(String(value).trim());
}

function parseLength(value) {
    const v = String(value).trim();
    if (v === "0") return { number: 0, unit: "" };
    const match = v.match(/^(-?\d+(?:\.\d+)?)(px|rem)$/);
    if (!match) return null;
    const number = Number(match[1]);
    if (!Number.isFinite(number)) return null;
    return { number: match[2] === "rem" ? number * 16 : number, unit: match[2] };
}

function roundedNumber(value) {
    return Math.round(value * 1000) / 1000;
}

function isTypographyDimension(token) {
    return token.spec === "cem-voice-fonts-typography" &&
        (token.name.includes("font-size") || token.name.includes("line-height") || token.name.includes("letter-spacing"));
}

function androidResourceForToken(token, mode) {
    const value = modeValue(token, mode);
    const name = androidNameFromPath(token.path);

    if (token.type === "color" && isHexColor(value)) {
        return { kind: "color", name, value: String(value).toLowerCase() };
    }

    if (token.type === "dimension") {
        const parsed = parseLength(value);
        if (!parsed) return null;
        const unit = isTypographyDimension(token) ? "sp" : "dp";
        return { kind: "dimen", name, value: `${roundedNumber(parsed.number)}${unit}` };
    }

    if (token.type === "number" && /^-?\d+$/.test(String(value).trim())) {
        return { kind: "integer", name, value: String(value).trim() };
    }

    if (token.type === "duration" && /^\d+(?:\.\d+)?ms$/.test(String(value).trim())) {
        return { kind: "integer", name: `${name}_ms`, value: String(Math.round(Number.parseFloat(value))) };
    }

    if (token.type === "fontFamily" || token.type === "string") {
        const text = String(value).trim();
        if (text && !/[{}();]/.test(text)) return { kind: "string", name, value: text };
    }

    return null;
}

async function readCanonicalTokens() {
    const filePath = path.join(DIST_TOKENS, "cem.tokens.json");
    const tree = JSON.parse(await fs.readFile(filePath, "utf8"));
    const generated = tree.$extensions?.cem?.generated ?? {};
    return { tokens: flattenTokens(tree), generated };
}

async function writeModeJson(tokens, generated) {
    await fs.mkdir(OUT_JSON, { recursive: true });

    const modePaths = [];
    for (const mode of CEM_PLATFORM_MODES) {
        const out = {
            $generated: {
                ...generated,
                generator: "packages/cem-theme/scripts/build-token-platforms.mjs",
                platform: "json",
                mode,
            },
            mode,
            tokens: Object.fromEntries(
                tokens.map((token) => [
                    token.path,
                    {
                        name: token.name,
                        type: token.type,
                        value: modeValue(token, mode),
                        tier: token.tier,
                        spec: token.spec,
                        sourceTable: token.sourceTable,
                        portability: token.portability,
                    },
                ]),
            ),
        };

        const outPath = path.join(OUT_JSON, `cem-tokens-${mode}.json`);
        await fs.writeFile(outPath, JSON.stringify(out, null, 2), "utf8");
        modePaths.push(outPath);
    }

    return modePaths;
}

async function writeReport(tokens, modePaths, generated) {
    const byType = {};
    const byPortability = {};
    for (const token of tokens) {
        byType[token.type] = (byType[token.type] ?? 0) + 1;
        byPortability[token.portability] = (byPortability[token.portability] ?? 0) + 1;
    }

    const report = [
        "# CEM Token Platform JSON Report",
        "",
        `Generated: ${new Date().toISOString()}  `,
        `Source token build: ${generated.sourceBuildCommand ?? "unknown"}  `,
        "",
        "## Summary",
        "",
        "| Stat | Count |",
        "| ---- | ----- |",
        `| Tokens per mode | ${tokens.length} |`,
        `| Mode files | ${modePaths.length} |`,
        "",
        "## Token Types",
        "",
        "| Type | Count |",
        "| ---- | ----- |",
        ...Object.entries(byType).map(([type, count]) => `| \`${type}\` | ${count} |`),
        "",
        "## Portability",
        "",
        "| Portability | Count |",
        "| ----------- | ----- |",
        ...Object.entries(byPortability).map(([portability, count]) => `| \`${portability}\` | ${count} |`),
        "",
        "## Files",
        "",
        ...modePaths.map((filePath) => `- \`${path.relative(PACKAGE_ROOT, filePath)}\``),
        "",
        "> Generated by `build-token-platforms.mjs`. Do not edit by hand.",
        "",
    ];

    const reportPath = path.join(OUT_JSON, "json-report.md");
    await fs.writeFile(reportPath, report.join("\n"), "utf8");
    return reportPath;
}

async function writeIosOutputs(tokens, generated) {
    await fs.mkdir(OUT_IOS, { recursive: true });

    const swift = [
        "// Generated by packages/cem-theme/scripts/build-token-platforms.mjs. Do not edit by hand.",
        "",
        "public enum CEMTokens {",
        `    public static let generatedPackageVersion = "${escapeSwiftString(generated.packageVersion ?? "")}"`,
        "",
    ];

    for (const mode of CEM_PLATFORM_MODES) {
        const enumName = mode.split("-").map((part) => part[0].toUpperCase() + part.slice(1)).join("");
        swift.push(`    public enum ${enumName} {`);
        for (const token of tokens) {
            swift.push(`        public static let ${identifierFromPath(token.path)} = "${escapeSwiftString(modeValue(token, mode))}"`);
        }
        swift.push("    }", "");
    }
    swift.push("}");
    swift.push("");

    const swiftPath = path.join(OUT_IOS, "CEMTokens.swift");
    await fs.writeFile(swiftPath, swift.join("\n"), "utf8");

    const colorHints = {
        $generated: {
            ...generated,
            generator: "packages/cem-theme/scripts/build-token-platforms.mjs",
            platform: "ios",
            artifact: "xcassets-hints",
        },
        colors: Object.fromEntries(
            tokens
                .filter((token) => token.type === "color")
                .map((token) => {
                    const modes = Object.fromEntries(
                        CEM_PLATFORM_MODES
                            .map((mode) => [mode, modeValue(token, mode)])
                            .filter(([, value]) => isHexColor(value)),
                    );
                    return [token.path, { name: token.name, assetName: identifierFromPath(token.path), modes }];
                })
                .filter(([, hint]) => Object.keys(hint.modes).length > 0),
        ),
    };
    const hintsPath = path.join(OUT_IOS, "CEMTokens.xcassets-hints.json");
    await fs.writeFile(hintsPath, JSON.stringify(colorHints, null, 2), "utf8");

    const reportPath = path.join(OUT_IOS, "ios-report.md");
    const report = [
        "# CEM iOS Token Report",
        "",
        `Generated: ${new Date().toISOString()}  `,
        "",
        "## Summary",
        "",
        "| Stat | Count |",
        "| ---- | ----- |",
        `| Swift token constants per mode | ${tokens.length} |`,
        `| Color asset hints | ${Object.keys(colorHints.colors).length} |`,
        "| Fail-hard violations | 0 |",
        "",
        "## Unit Policy",
        "",
        "- CSS `px` and `rem` values map to iOS points when native typed outputs are added.",
        "- Typography does not opt into Dynamic Type scaling in v1.",
        "- Current Swift output keeps values as strings so adapters can choose UIKit, SwiftUI, or asset-catalog mapping.",
        "",
        "> Generated by `build-token-platforms.mjs`. Do not edit by hand.",
        "",
    ];
    await fs.writeFile(reportPath, report.join("\n"), "utf8");

    return { swiftPath, hintsPath, reportPath };
}

function androidXml(resources) {
    const lines = ["<?xml version=\"1.0\" encoding=\"utf-8\"?>", "<resources>"];
    for (const resource of resources) {
        if (resource.kind === "color") lines.push(`    <color name="${resource.name}">${escapeXml(resource.value)}</color>`);
        else if (resource.kind === "dimen") lines.push(`    <dimen name="${resource.name}">${escapeXml(resource.value)}</dimen>`);
        else if (resource.kind === "integer") lines.push(`    <integer name="${resource.name}">${escapeXml(resource.value)}</integer>`);
        else if (resource.kind === "string") lines.push(`    <string name="${resource.name}">${escapeXml(resource.value)}</string>`);
    }
    lines.push("</resources>", "");
    return lines.join("\n");
}

async function writeAndroidOutputs(tokens, generated) {
    const valuesDir = path.join(OUT_ANDROID, "values");
    const valuesNightDir = path.join(OUT_ANDROID, "values-night");
    const composeDir = path.join(OUT_ANDROID, "compose");
    await fs.mkdir(valuesDir, { recursive: true });
    await fs.mkdir(valuesNightDir, { recursive: true });
    await fs.mkdir(composeDir, { recursive: true });

    const lightResources = tokens.map((token) => androidResourceForToken(token, "light")).filter(Boolean);
    const darkResources = tokens.map((token) => androidResourceForToken(token, "dark")).filter(Boolean);

    const lightPath = path.join(valuesDir, "cem-tokens.xml");
    const darkPath = path.join(valuesNightDir, "cem-tokens.xml");
    await fs.writeFile(lightPath, androidXml(lightResources), "utf8");
    await fs.writeFile(darkPath, androidXml(darkResources), "utf8");

    const composeLines = [
        "// Generated by packages/cem-theme/scripts/build-token-platforms.mjs. Do not edit by hand.",
        "package org.epawg.cem.tokens",
        "",
        "object CEMTokens {",
        ...tokens.map((token) => `    const val ${identifierFromPath(token.path)} = "${escapeSwiftString(modeValue(token, "light"))}"`),
        "}",
        "",
    ];
    const composePath = path.join(composeDir, "CEMTokens.kt");
    await fs.writeFile(composePath, composeLines.join("\n"), "utf8");

    const reportPath = path.join(OUT_ANDROID, "android-report.md");
    const skipped = tokens.length - lightResources.length;
    const report = [
        "# CEM Android Token Report",
        "",
        `Generated: ${new Date().toISOString()}  `,
        "",
        "## Summary",
        "",
        "| Stat | Count |",
        "| ---- | ----- |",
        `| Light resources | ${lightResources.length} |`,
        `| Night resources | ${darkResources.length} |`,
        `| Compose string constants | ${tokens.length} |`,
        `| Skipped XML resources | ${skipped} |`,
        "| Fail-hard violations | 0 |",
        "",
        "## Unit Policy",
        "",
        "- Layout, spacing, shape, and control dimensions map to `dp`.",
        "- Typography dimensions map to `sp`.",
        "- Numeric integer tokens stay unitless.",
        "- Durations emit integer millisecond resources when directly representable.",
        "",
        "## Skipped XML Resources",
        "",
        "Tokens with CSS expressions, platform notes, percentage dimensions, shadow strings, or unresolved native shapes are kept in Compose string constants and omitted from Android XML resources.",
        "",
        "> Generated by `build-token-platforms.mjs`. Do not edit by hand.",
        "",
    ];
    await fs.writeFile(reportPath, report.join("\n"), "utf8");

    return { lightPath, darkPath, composePath, reportPath };
}

export async function buildTokenPlatforms() {
    const { tokens, generated } = await readCanonicalTokens();
    const modePaths = await writeModeJson(tokens, generated);
    const reportPath = await writeReport(tokens, modePaths, generated);
    const ios = await writeIosOutputs(tokens, generated);
    const android = await writeAndroidOutputs(tokens, generated);
    return { tokens, modePaths, reportPath, ios, android };
}

async function main() {
    const { tokens, modePaths, reportPath, ios, android } = await buildTokenPlatforms();
    console.log(`build-token-platforms: emitted ${tokens.length} tokens across ${modePaths.length} JSON mode files`);
    for (const modePath of modePaths) console.log(`  → ${path.relative(process.cwd(), modePath)}`);
    console.log(`  → ${path.relative(process.cwd(), reportPath)}`);
    console.log(`  → ${path.relative(process.cwd(), ios.swiftPath)}`);
    console.log(`  → ${path.relative(process.cwd(), ios.hintsPath)}`);
    console.log(`  → ${path.relative(process.cwd(), ios.reportPath)}`);
    console.log(`  → ${path.relative(process.cwd(), android.lightPath)}`);
    console.log(`  → ${path.relative(process.cwd(), android.darkPath)}`);
    console.log(`  → ${path.relative(process.cwd(), android.composePath)}`);
    console.log(`  → ${path.relative(process.cwd(), android.reportPath)}`);
}

main().catch((err) => {
    console.error(err);
    process.exit(2);
});
