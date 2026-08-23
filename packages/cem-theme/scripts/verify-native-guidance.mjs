/** Verify native guidance coverage and web/SwiftUI/Compose sample parity. */

import fs from "node:fs/promises";
import path from "node:path";
import { CEM_NATIVE_PLATFORM_CONTRACT } from "./native-platform-contract.mjs";

const PACKAGE_ROOT = path.resolve(new URL("..", import.meta.url).pathname);
const WORKSPACE_ROOT = path.resolve(PACKAGE_ROOT, "../..");
const REPORT_ROOT = path.join(PACKAGE_ROOT, "dist/reports");

const paths = {
    catalog: "packages/cem-components/dist/catalog/cem.components.catalog.json",
    guidance: "examples/native/component-guidance.json",
    guidanceReadme: "examples/native/README.md",
    docs: "docs/native-platforms.md",
    flatTokens: "packages/cem-theme/dist/lib/token-platforms/json/cem-tokens-light.json",
    generatedSwift: "packages/cem-theme/dist/lib/token-platforms/ios/Sources/CEMTokens/CEMTokens.swift",
    generatedKotlin: "packages/cem-theme/dist/lib/token-platforms/android/cem-tokens/src/main/kotlin/org/epawg/cem/tokens/CEMTokens.kt",
};

function workspacePath(relativePath) {
    return path.join(WORKSPACE_ROOT, relativePath);
}

async function readText(relativePath) {
    return fs.readFile(workspacePath(relativePath), "utf8");
}

async function readJson(relativePath) {
    return JSON.parse(await readText(relativePath));
}

function assert(condition, message, errors) {
    if (!condition) errors.push(message);
}

function pascalCase(value) {
    return value
        .split(/[^a-z0-9]+/i)
        .filter(Boolean)
        .map((part) => part[0].toUpperCase() + part.slice(1))
        .join("");
}

function sameMembers(actual, expected) {
    return JSON.stringify([...actual].sort()) === JSON.stringify([...expected].sort());
}

async function main() {
    const errors = [];
    const [catalog, guidance, docs, guidanceReadme, flatTokens, generatedSwift, generatedKotlin] = await Promise.all([
        readJson(paths.catalog),
        readJson(paths.guidance),
        readText(paths.docs),
        readText(paths.guidanceReadme),
        readJson(paths.flatTokens),
        readText(paths.generatedSwift),
        readText(paths.generatedKotlin),
    ]);

    assert(guidance.version === 1, "native guidance version must be 1", errors);
    assert(
        catalog.components.length === guidance.expectedPublicPrimitiveCount,
        `native guidance expected ${guidance.expectedPublicPrimitiveCount} components, catalog has ${catalog.components.length}`,
        errors,
    );
    assert(catalog.$generated?.componentCount === catalog.components.length, "component catalog count drifted", errors);

    const catalogFamilies = new Set(catalog.components.flatMap((component) => component.tokenFamilies));
    assert(
        sameMembers(Object.keys(guidance.tokenFamilyKinds), catalogFamilies),
        "native token-family mappings do not exactly cover the public component catalog",
        errors,
    );
    const catalogStates = new Set(
        catalog.components.flatMap((component) => component.categoryStates.map((state) => state.name)),
    );
    assert(
        sameMembers(Object.keys(guidance.stateMappings), catalogStates),
        "native state mappings do not exactly cover the public component state catalog",
        errors,
    );
    const catalogCategories = new Set(catalog.components.map((component) => component.category));
    assert(
        sameMembers(Object.keys(guidance.accessibilityByCategory), catalogCategories),
        "native accessibility guidance does not exactly cover every component category",
        errors,
    );

    const mappings = catalog.components.map((component) => {
        for (const state of component.categoryStates) {
            assert(state.status === "covered", `${component.tag} state ${state.name} is not covered`, errors);
            assert(Boolean(guidance.stateMappings[state.name]), `${component.tag} state ${state.name} has no native mapping`, errors);
        }
        for (const family of component.tokenFamilies) {
            assert(Boolean(guidance.tokenFamilyKinds[family]), `${component.tag} token family ${family} has no native mapping`, errors);
        }
        assert(
            Boolean(guidance.accessibilityByCategory[component.category]),
            `${component.tag} category ${component.category} has no accessibility guidance`,
            errors,
        );
        const suffix = pascalCase(component.id);
        return {
            id: component.id,
            web: component.tag,
            swiftUI: `${guidance.naming.swiftUIPrefix}${suffix}`,
            compose: `${guidance.naming.composePrefix}${suffix}`,
            category: component.category,
            states: component.categoryStates.map((state) => state.name),
            tokenFamilies: component.tokenFamilies,
            accessibility: guidance.accessibilityByCategory[component.category],
        };
    });

    const parity = guidance.sampleParity;
    const [web, swiftUI, compose] = await Promise.all([
        readText(parity.web),
        readText(parity.swiftUI),
        readText(parity.compose),
    ]);
    for (const semantic of parity.semantics) {
        for (const [platform, source] of Object.entries({ web, swiftUI, compose })) {
            assert(source.includes(semantic), `${platform} parity fixture is missing ${semantic}`, errors);
        }
    }
    for (const token of parity.tokens) {
        assert(web.includes(`var(${token.css})`), `web parity fixture is missing ${token.css}`, errors);
        assert(swiftUI.includes(`CEMTokens.Light.${token.native}`), `SwiftUI parity fixture is missing ${token.native}`, errors);
        assert(compose.includes(`CEMTokens.${token.native}`), `Compose parity fixture is missing ${token.native}`, errors);
        const flatToken = Object.values(flatTokens.tokens).find((candidate) => candidate.name === token.css);
        assert(Boolean(flatToken), `canonical flat token output is missing ${token.css}`, errors);
        assert(
            generatedSwift.includes(`public static let ${token.native} =`),
            `generated Swift package is missing ${token.native}`,
            errors,
        );
        assert(
            generatedKotlin.includes(`const val ${token.native} =`),
            `generated Kotlin library is missing ${token.native}`,
            errors,
        );
    }
    assert(web.includes("<button"), "web parity fixture must preserve native button semantics", errors);
    assert(swiftUI.includes("Button(\"Primary action\")"), "SwiftUI parity fixture must use Button", errors);
    assert(compose.includes("Button("), "Compose parity fixture must use Button", errors);
    assert(
        swiftUI.includes("cemCouplingZoneMin") && compose.includes("cemCouplingZoneMin"),
        "native samples must use the CEM minimum target token",
        errors,
    );
    assert(
        guidanceReadme.includes("Live Figma canvas review remains in Phase 10"),
        "native fixture README must preserve the Phase 10 Figma review boundary",
        errors,
    );

    for (const expected of [
        `Xcode ${CEM_NATIVE_PLATFORM_CONTRACT.ios.xcodeVersion}`,
        `Swift ${CEM_NATIVE_PLATFORM_CONTRACT.ios.swiftToolsVersion}`,
        `AGP ${CEM_NATIVE_PLATFORM_CONTRACT.android.androidGradlePluginVersion}`,
        `Gradle ${CEM_NATIVE_PLATFORM_CONTRACT.android.gradleVersion}`,
        `Kotlin/Compose ${CEM_NATIVE_PLATFORM_CONTRACT.android.kotlinVersion}`,
        "49 public CEM primitives",
    ]) {
        assert(docs.includes(expected), `native platform guidance is missing ${expected}`, errors);
    }

    if (errors.length > 0) {
        for (const error of errors) console.error(`error: ${error}`);
        throw new Error(`native guidance validation failed with ${errors.length} error(s)`);
    }

    const report = {
        version: 1,
        contract: CEM_NATIVE_PLATFORM_CONTRACT,
        summary: {
            publicPrimitives: mappings.length,
            componentCategories: catalogCategories.size,
            mappedStates: catalogStates.size,
            mappedTokenFamilies: catalogFamilies.size,
            parityTokens: parity.tokens.length,
            failHardViolations: 0,
        },
        sources: paths,
        components: mappings,
        sampleParity: parity,
    };
    const markdown = [
        "# CEM Native Component Guidance Report",
        "",
        "| Stat | Count |",
        "| ---- | ----- |",
        `| Public primitives | ${report.summary.publicPrimitives} |`,
        `| Component categories | ${report.summary.componentCategories} |`,
        `| Mapped states | ${report.summary.mappedStates} |`,
        `| Mapped token families | ${report.summary.mappedTokenFamilies} |`,
        `| Cross-platform parity tokens | ${report.summary.parityTokens} |`,
        "| Fail-hard violations | 0 |",
        "",
        "| Web primitive | SwiftUI guidance name | Compose guidance name | Category |",
        "| ------------- | --------------------- | --------------------- | -------- |",
        ...mappings.map(
            (mapping) => `| \`${mapping.web}\` | \`${mapping.swiftUI}\` | \`${mapping.compose}\` | ${mapping.category} |`,
        ),
        "",
        "> Generated by `verify-native-guidance.mjs`. Do not edit by hand.",
        "",
    ].join("\n");

    await fs.mkdir(REPORT_ROOT, { recursive: true });
    await fs.writeFile(path.join(REPORT_ROOT, "native-component-guidance.json"), `${JSON.stringify(report, null, 2)}\n`, "utf8");
    await fs.writeFile(path.join(REPORT_ROOT, "native-component-guidance.md"), markdown, "utf8");
    console.log(
        `verify-native-guidance: ${mappings.length} primitives, ${catalogStates.size} states, ` +
            `${catalogFamilies.size} token families, ${parity.tokens.length} parity tokens, zero violations`,
    );
}

main().catch((error) => {
    console.error(error);
    process.exit(1);
});
