#!/usr/bin/env node

import fs from "node:fs/promises";
import path from "node:path";

const PACKAGE_ROOT = path.resolve(new URL("..", import.meta.url).pathname);
const WORKSPACE_ROOT = path.resolve(PACKAGE_ROOT, "../..");
const FIGMA_DIRECTORY = path.join(PACKAGE_ROOT, "dist/lib/tokens/figma");
const EVIDENCE_PATH = path.join(WORKSPACE_ROOT, "examples/figma/native-library-review.json");
const FIXTURE_PATH = path.join(WORKSPACE_ROOT, "examples/figma/native-library-review-fixture.md");
const README_PATH = path.join(WORKSPACE_ROOT, "examples/figma/README.md");
const GENERATED_REPORT_PATH = path.join(FIGMA_DIRECTORY, "cem-figma-report.md");
const REPORT_DIRECTORY = path.join(PACKAGE_ROOT, "dist/reports");
const REPORT_JSON_PATH = path.join(REPORT_DIRECTORY, "cem-figma-native-review.json");
const REPORT_MARKDOWN_PATH = path.join(REPORT_DIRECTORY, "cem-figma-native-review.md");

const EXPECTED_SOURCES = {
    generatedModes: "packages/cem-theme/dist/lib/tokens/figma/cem-*.tokens.json",
    generatedReport: "packages/cem-theme/dist/lib/tokens/figma/cem-figma-report.md",
    workflow: "examples/figma/README.md#native-figma-library-setup",
};
const EXPECTED_FILE = "https://www.figma.com/design/vLZUzjS7xHACjXgYLA9vtD/CEM-UI-Kit";
const EXPECTED_COLLECTION = "CEM Tokens";
const EXPECTED_PAGE = "01 Tokens";
const EXPECTED_MODES = ["Light", "Dark", "Contrast Light", "Contrast Dark", "Native"];
const EXPECTED_MODE_FILES = [
    "cem-light.tokens.json",
    "cem-dark.tokens.json",
    "cem-contrast-light.tokens.json",
    "cem-contrast-dark.tokens.json",
    "cem-native.tokens.json",
];
const EXPECTED_MODE_IDS = ["light", "dark", "contrast-light", "contrast-dark", "native"];
const LIVE_RESULT_FIELDS = [
    "importedModeFiles",
    "reviewedAt",
    "reviewedRevision",
    "variableCount",
    "variableTypes",
    "missingModeValues",
    "nativeAliasModeValues",
];
const failures = [];

const [evidence, fixture, readme, generatedReport, ...modeDocuments] = await Promise.all([
    readJson(EVIDENCE_PATH),
    fs.readFile(FIXTURE_PATH, "utf8"),
    fs.readFile(README_PATH, "utf8"),
    fs.readFile(GENERATED_REPORT_PATH, "utf8"),
    ...EXPECTED_MODE_FILES.map((file) => readJson(path.join(FIGMA_DIRECTORY, file))),
]);

const generated = validateGeneratedModes(modeDocuments);
validateEvidenceBoundary();
validateGeneratedEvidence(generated);
validateHistoricalReview();
validateRefresh(generated);
validateDocumentation(generated);
validateFixture();

if (failures.length > 0) {
    for (const failure of failures) console.error(`error: ${failure}`);
    process.exit(1);
}

const report = {
    version: evidence.version,
    sources: evidence.sources,
    figma: evidence.figma,
    summary: {
        refreshStatus: evidence.refresh.status,
        startingCheckpointReady: evidence.refresh.status === "started" || evidence.refresh.status === "reviewed",
        refreshReviewed: evidence.refresh.status === "reviewed",
        modeCount: EXPECTED_MODES.length,
        variableCountPerMode: generated.variableCountPerMode,
        variableTypes: generated.variableTypes,
        tokenReferenceModeValues: generated.tokenReferenceModeValues,
    },
    lastReviewed: evidence.lastReviewed,
    refresh: evidence.refresh,
};

await fs.mkdir(REPORT_DIRECTORY, { recursive: true });
await fs.writeFile(REPORT_JSON_PATH, `${JSON.stringify(report, null, 4)}\n`, "utf8");
await fs.writeFile(REPORT_MARKDOWN_PATH, renderMarkdown(report), "utf8");

console.log(
    `Figma native-library evidence verified (${report.summary.variableCountPerMode} variables across ` +
        `${report.summary.modeCount} modes; refresh ${report.summary.refreshStatus}).`
);

function validateGeneratedModes(documents) {
    const tokenMaps = documents.map((document, index) => {
        const actualMode = document?.$extensions?.cem?.generated?.mode;
        if (actualMode !== EXPECTED_MODE_IDS[index]) {
            fail(`${EXPECTED_MODE_FILES[index]}: generated mode must be ${EXPECTED_MODE_IDS[index]}`);
        }
        return walkTokens(document);
    });
    const first = tokenMaps[0] ?? new Map();
    const variableTypes = { COLOR: 0, FLOAT: 0, STRING: 0 };
    for (const [tokenPath, token] of first) {
        const figmaType = toFigmaType(token.$type);
        if (!figmaType) {
            fail(`${EXPECTED_MODE_FILES[0]}: ${tokenPath} has unsupported type ${token.$type ?? "missing"}`);
            continue;
        }
        variableTypes[figmaType] += 1;
    }

    for (const [index, tokens] of tokenMaps.entries()) {
        if (tokens.size !== first.size) {
            fail(`${EXPECTED_MODE_FILES[index]}: expected ${first.size} variables, found ${tokens.size}`);
        }
        for (const [tokenPath, firstToken] of first) {
            const token = tokens.get(tokenPath);
            if (!token) {
                fail(`${EXPECTED_MODE_FILES[index]}: missing ${tokenPath}`);
            } else if (token.$type !== firstToken.$type) {
                fail(`${EXPECTED_MODE_FILES[index]}: ${tokenPath} type differs from the Light mode`);
            }
        }
        for (const tokenPath of tokens.keys()) {
            if (!first.has(tokenPath)) fail(`${EXPECTED_MODE_FILES[index]}: unexpected ${tokenPath}`);
        }
    }

    const tokenReferenceModeValues = tokenMaps.reduce(
        (count, tokens) => count + [...tokens.values()].filter((token) => isTokenReference(token.$value)).length,
        0
    );
    return { variableCountPerMode: first.size, variableTypes, tokenReferenceModeValues };
}

function validateEvidenceBoundary() {
    if (!evidence || typeof evidence !== "object" || Array.isArray(evidence)) {
        fail("native-library-review.json must contain an object");
        return;
    }
    if (evidence.version !== 1) fail("native-library-review.json version must be 1");
    if (!sameJson(evidence.sources, EXPECTED_SOURCES)) {
        fail("native-library-review.json must name the generated modes, report, and workflow owners");
    }
    if (evidence.figma?.file !== EXPECTED_FILE) fail("native review must target the canonical CEM UI Kit file");
    if (evidence.figma?.collection !== EXPECTED_COLLECTION) {
        fail(`native review collection must be ${EXPECTED_COLLECTION}`);
    }
    if (evidence.figma?.page !== EXPECTED_PAGE) fail(`native review page must be ${EXPECTED_PAGE}`);
    if (!sameJson(evidence.figma?.modes, EXPECTED_MODES)) {
        fail(`native review modes must be ${EXPECTED_MODES.join(", ")}`);
    }
}

function validateGeneratedEvidence(generated) {
    if (!isDate(evidence.generated?.verifiedAt)) {
        fail("generated.verifiedAt must be an ISO calendar date");
    }
    if (evidence.generated?.variableCountPerMode !== generated.variableCountPerMode) {
        fail(
            `generated.variableCountPerMode must match the ${generated.variableCountPerMode}-variable mode artifacts`
        );
    }
    if (!sameJson(evidence.generated?.variableTypes, generated.variableTypes)) {
        fail(`generated.variableTypes must match ${JSON.stringify(generated.variableTypes)}`);
    }
    if (!generatedReport.includes(`| Tokens in all mode files | ${generated.variableCountPerMode} |`)) {
        fail("cem-figma-report.md variable count differs from the generated modes");
    }
    if (!generatedReport.includes("| Errors | 0 |")) {
        fail("cem-figma-report.md must report zero errors");
    }
}

function validateHistoricalReview() {
    const review = evidence.lastReviewed;
    if (review?.status !== "reviewed") fail("lastReviewed.status must remain reviewed");
    if (!isDate(review?.reviewedAt)) fail("lastReviewed.reviewedAt must be an ISO calendar date");
    if (!isRevision(review?.revision)) fail("lastReviewed.revision must be a Figma URL or stable revision token");
    if (review?.collection !== EXPECTED_COLLECTION) fail(`lastReviewed.collection must be ${EXPECTED_COLLECTION}`);
    if (!sameJson(review?.modes, EXPECTED_MODES)) fail("lastReviewed.modes must preserve the accepted five modes");
    validateReviewResults(review, "lastReviewed", { requireGeneratedMatch: false });
}

function validateRefresh(generated) {
    const refresh = evidence.refresh;
    if (!refresh || typeof refresh !== "object" || Array.isArray(refresh)) {
        fail("refresh must be an object");
        return;
    }
    if (!["pending", "started", "reviewed"].includes(refresh.status)) {
        fail("refresh.status must be pending, started, or reviewed");
        return;
    }

    if (refresh.status === "pending") {
        for (const field of ["startedAt", "startingRevision", "confirmedCollection", "confirmedModes", ...LIVE_RESULT_FIELDS]) {
            if (refresh[field] !== null) fail(`refresh.${field} must remain null while the refresh is pending`);
        }
        if (!sameJson(refresh.evidenceLocators, [])) {
            fail("refresh.evidenceLocators must remain empty while the refresh is pending");
        }
        return;
    }

    if (!isDate(refresh.startedAt)) fail("refresh.startedAt must be an ISO calendar date once review starts");
    if (!isRevision(refresh.startingRevision)) {
        fail("refresh.startingRevision must be a Figma URL or stable revision token once review starts");
    }
    if (refresh.confirmedCollection !== EXPECTED_COLLECTION) {
        fail(`refresh.confirmedCollection must be ${EXPECTED_COLLECTION} once review starts`);
    }
    if (!sameJson(refresh.confirmedModes, EXPECTED_MODES)) {
        fail("refresh.confirmedModes must preserve the accepted five modes once review starts");
    }

    if (refresh.status === "started") {
        for (const field of LIVE_RESULT_FIELDS) {
            if (refresh[field] !== null) fail(`refresh.${field} must remain null until the refresh is reviewed`);
        }
        if (!sameJson(refresh.evidenceLocators, [])) {
            fail("refresh.evidenceLocators must remain empty until the refresh is reviewed");
        }
        return;
    }

    if (!sameJson(refresh.importedModeFiles, EXPECTED_MODE_FILES)) {
        fail("refresh.importedModeFiles must list all five generated mode files in mode order");
    }
    if (!isDate(refresh.reviewedAt)) fail("refresh.reviewedAt must be an ISO calendar date");
    if (!isRevision(refresh.reviewedRevision)) {
        fail("refresh.reviewedRevision must be a Figma URL or stable revision token");
    }
    validateReviewResults(refresh, "refresh", { requireGeneratedMatch: true, generated });
}

function validateReviewResults(review, label, { requireGeneratedMatch, generated }) {
    if (!Number.isInteger(review?.variableCount) || review.variableCount <= 0) {
        fail(`${label}.variableCount must be a positive integer`);
    }
    const types = review?.variableTypes;
    if (
        !types ||
        typeof types !== "object" ||
        Array.isArray(types) ||
        !["COLOR", "FLOAT", "STRING"].every((type) => Number.isInteger(types[type]) && types[type] >= 0) ||
        Object.keys(types).length !== 3
    ) {
        fail(`${label}.variableTypes must contain non-negative COLOR, FLOAT, and STRING counts`);
    } else if (Object.values(types).reduce((sum, count) => sum + count, 0) !== review.variableCount) {
        fail(`${label}.variableTypes must add up to ${label}.variableCount`);
    }
    if (review?.missingModeValues !== 0) fail(`${label}.missingModeValues must be zero`);
    if (!Number.isInteger(review?.nativeAliasModeValues) || review.nativeAliasModeValues <= 0) {
        fail(`${label}.nativeAliasModeValues must be a positive integer`);
    }
    if (!Array.isArray(review?.evidenceLocators) || review.evidenceLocators.length === 0) {
        fail(`${label}.evidenceLocators must contain at least one review location`);
    } else if (review.evidenceLocators.some((locator) => typeof locator !== "string" || locator.length === 0)) {
        fail(`${label}.evidenceLocators must contain non-empty strings`);
    }

    if (requireGeneratedMatch) {
        if (review.variableCount !== generated.variableCountPerMode) {
            fail(`${label}.variableCount must match the generated ${generated.variableCountPerMode} variables`);
        }
        if (!sameJson(review.variableTypes, generated.variableTypes)) {
            fail(`${label}.variableTypes must match generated ${JSON.stringify(generated.variableTypes)}`);
        }
        if (review.nativeAliasModeValues > generated.tokenReferenceModeValues) {
            fail(
                `${label}.nativeAliasModeValues cannot exceed ${generated.tokenReferenceModeValues} generated ` +
                    "token-reference mode values"
            );
        }
    }
}

function validateDocumentation(generated) {
    const markers = [
        "[`native-library-review.json`](./native-library-review.json)",
        "verify:figma-native-review",
        `Refresh status: \`${evidence.refresh.status}\``,
        `Last manual validation run: ${evidence.lastReviewed.reviewedAt}`,
        `Variable count: ${evidence.lastReviewed.variableCount}`,
        `Missing mode values: ${evidence.lastReviewed.missingModeValues}`,
        `Variable aliases present: ${evidence.lastReviewed.nativeAliasModeValues} mode values`,
        `Generated variable count: ${generated.variableCountPerMode} per mode`,
    ];
    for (const marker of markers) {
        if (!readme.includes(marker)) fail(`examples/figma/README.md missing native-review marker ${marker}`);
    }
}

function validateFixture() {
    const markers = [
        "native-library-review.json",
        "verify:figma-native-review",
        "startingRevision",
        "CEM Tokens",
        ...EXPECTED_MODES,
        ...EXPECTED_MODE_FILES,
        "Deliberate rejection cases",
    ];
    for (const marker of markers) {
        if (!fixture.includes(marker)) fail(`native-library-review-fixture.md missing review marker ${marker}`);
    }
}

function walkTokens(node, prefix = [], result = new Map()) {
    if (!node || typeof node !== "object") return result;
    for (const [key, value] of Object.entries(node)) {
        if (key.startsWith("$") || !value || typeof value !== "object") continue;
        const tokenPath = [...prefix, key];
        if ("$value" in value) result.set(tokenPath.join("."), value);
        walkTokens(value, tokenPath, result);
    }
    return result;
}

function toFigmaType(type) {
    if (type === "color") return "COLOR";
    if (["dimension", "duration", "number"].includes(type)) return "FLOAT";
    if (["fontFamily", "string"].includes(type)) return "STRING";
    return null;
}

function isTokenReference(value) {
    return typeof value === "string" && /^\{cem(?:\.[a-z0-9]+)+\}$/u.test(value);
}

function isDate(value) {
    return typeof value === "string" && /^\d{4}-\d{2}-\d{2}$/u.test(value);
}

function isRevision(value) {
    return (
        typeof value === "string" &&
        (/^https:\/\/www\.figma\.com\/(design|file)\//u.test(value) || /^[a-z0-9._-]+$/iu.test(value))
    );
}

function sameJson(left, right) {
    return JSON.stringify(left) === JSON.stringify(right);
}

function fail(message) {
    failures.push(message);
}

function renderMarkdown(currentReport) {
    const { summary, lastReviewed, refresh } = currentReport;
    const nextAction =
        refresh.status === "pending"
            ? "Open the canonical CEM UI Kit, record the starting revision, and confirm the collection and five modes."
            : refresh.status === "started"
              ? "Import all five generated mode files, review the live results, and record the reviewed revision."
              : "The live native-library refresh evidence is complete.";
    return `# CEM Figma Native Library Review Report

## Summary

| Check | Result |
| --- | --- |
| Refresh status | ${summary.refreshStatus} |
| Starting checkpoint ready | ${summary.startingCheckpointReady ? "yes" : "no"} |
| Refresh reviewed | ${summary.refreshReviewed ? "yes" : "no"} |
| Generated modes | ${summary.modeCount} |
| Generated variables per mode | ${summary.variableCountPerMode} |
| Generated variable types | ${Object.entries(summary.variableTypes)
        .map(([type, count]) => `${type}: ${count}`)
        .join(", ")} |
| Generated token-reference mode values | ${summary.tokenReferenceModeValues} |
| Last live review | ${lastReviewed.reviewedAt}; ${lastReviewed.variableCount} variables |

## Next action

${nextAction}

Repository generation does not update the external Figma file. Promote the
checked-in refresh record only from a live review, following
\`examples/figma/native-library-review-fixture.md\`.
`;
}

async function readJson(filePath) {
    return JSON.parse(await fs.readFile(filePath, "utf8"));
}
