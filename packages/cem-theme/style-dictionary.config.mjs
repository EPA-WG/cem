/**
 * Style Dictionary fan-out configuration for CEM token exports.
 *
 * Style Dictionary is intentionally not required by the MVP token exporter. This
 * module still owns the transform/filter contract so the local platform driver
 * and a future Style Dictionary runner use the same mode, unit, and filtering
 * policy.
 */

export const CEM_PLATFORM_MODES = ["light", "dark", "contrast-light", "contrast-dark", "native"];

export const CEM_STYLE_DICTIONARY_TRANSFORMS = [
    "cem/mode/expand-themes",
    "cem/size/layout-to-pt",
    "cem/size/type-to-pt",
    "cem/size/layout-to-dp",
    "cem/size/type-to-sp",
    "cem/number/unitless",
];

export const CEM_STYLE_DICTIONARY_FILTERS = ["cem/category/web-only-filter"];

const LENGTH_UNITS_TO_PLATFORM_BASE = {
    px: 1,
    rem: 16,
    pt: 1,
};

const VOICE_AUDIO_SUFFIX_RE = /-(speech-rate|speech-pitch|speech-volume|ssml-emphasis|ink-thickness|icon-stroke-multiplier)$/;

function styleDictionaryArgs(tokenOrArgs, options) {
    if (tokenOrArgs?.token) return { token: tokenOrArgs.token, options: tokenOrArgs.options ?? options ?? {} };
    return { token: tokenOrArgs, options: options ?? {} };
}

function roundPlatformNumber(value) {
    return Number.parseFloat((Math.round(value * 1000) / 1000).toFixed(3));
}

function tokenExtensions(token) {
    const extensions = token?.$extensions?.cem ?? token?.extensions?.cem ?? token?.cem ?? {};
    return {
        cssName: token?.name,
        spec: token?.spec,
        sourceTable: token?.sourceTable,
        category: token?.category,
        portability: token?.portability,
        modes: token?.modes,
        ...extensions,
    };
}

export function cemTokenType(token) {
    return token?.$type ?? token?.type ?? "";
}

export function cemTokenValue(token) {
    return token?.$value ?? token?.value ?? "";
}

export function cemTokenName(token) {
    const extensions = tokenExtensions(token);
    if (extensions.cssName) return extensions.cssName;
    if (typeof token?.name === "string" && token.name.startsWith("--cem-")) return token.name;
    if (Array.isArray(token?.path)) return `--${token.path.join("-")}`;
    if (typeof token?.path === "string") return `--${token.path.replace(/\./g, "-")}`;
    return "";
}

export function cemModeValue(token, mode = "light") {
    const modes = tokenExtensions(token).modes ?? token?.modes ?? {};
    const value = modes[mode];
    if (typeof value === "string" && value.trim() !== "") return value;
    return cemTokenValue(token);
}

export function parseCemCssLength(value) {
    const v = String(value ?? "").trim();
    if (v === "0") return { number: 0, unit: "", platformBase: 0 };

    const match = v.match(/^(-?\d+(?:\.\d+)?)(px|rem|pt)$/);
    if (!match) return null;

    const number = Number(match[1]);
    if (!Number.isFinite(number)) return null;

    const unit = match[2];
    return {
        number,
        unit,
        platformBase: number * LENGTH_UNITS_TO_PLATFORM_BASE[unit],
    };
}

export function formatCemPlatformLength(value, unit) {
    const parsed = parseCemCssLength(value);
    if (!parsed) return null;
    return `${roundPlatformNumber(parsed.platformBase)}${unit}`;
}

export function isCemTypographyDimensionToken(token) {
    if (cemTokenType(token) !== "dimension") return false;

    const extensions = tokenExtensions(token);
    const sourceTable = extensions.sourceTable ?? "";
    const name = cemTokenName(token);

    return extensions.spec === "cem-voice-fonts-typography" &&
        (
            sourceTable.includes("size") ||
            sourceTable.includes("line-height") ||
            sourceTable.includes("letter-spacing") ||
            sourceTable.includes("ergonomics") ||
            name.includes("font-size") ||
            name.includes("line-height") ||
            name.includes("letter-spacing")
        );
}

export function isCemLayoutDimensionToken(token) {
    return cemTokenType(token) === "dimension" && !isCemTypographyDimensionToken(token);
}

export function isCemNumericToken(token, options = {}) {
    const type = cemTokenType(token);
    if (type !== "number" && type !== "fontWeight") return false;
    return /^-?\d+(?:\.\d+)?$/.test(String(cemModeValue(token, options.mode)).trim());
}

export function isCemWebOnlyToken(token) {
    const extensions = tokenExtensions(token);
    const name = cemTokenName(token);
    const portability = extensions.portability ?? "";

    return portability === "css-expression" ||
        portability === "platform-note" ||
        VOICE_AUDIO_SUFFIX_RE.test(name);
}

export function cemTransformValue(name, token, options = {}) {
    const mode = options.mode ?? "light";
    const value = cemModeValue(token, mode);

    switch (name) {
        case "cem/mode/expand-themes":
            return value;
        case "cem/size/layout-to-pt":
            return isCemLayoutDimensionToken(token) ? formatCemPlatformLength(value, "pt") ?? value : value;
        case "cem/size/type-to-pt":
            return isCemTypographyDimensionToken(token) ? formatCemPlatformLength(value, "pt") ?? value : value;
        case "cem/size/layout-to-dp":
            return isCemLayoutDimensionToken(token) ? formatCemPlatformLength(value, "dp") ?? value : value;
        case "cem/size/type-to-sp":
            return isCemTypographyDimensionToken(token) ? formatCemPlatformLength(value, "sp") ?? value : value;
        case "cem/number/unitless":
            return isCemNumericToken(token, options) ? Number(value) : value;
        default:
            throw new Error(`Unknown CEM Style Dictionary transform: ${name}`);
    }
}

export const CEM_STYLE_DICTIONARY_TRANSFORM_DEFINITIONS = Object.fromEntries(
    CEM_STYLE_DICTIONARY_TRANSFORMS.map((name) => [
        name,
        {
            name,
            type: "value",
            filter(tokenOrArgs, options) {
                const { token, options: resolvedOptions } = styleDictionaryArgs(tokenOrArgs, options);
                if (name.includes("/layout-to-")) return isCemLayoutDimensionToken(token);
                if (name.includes("/type-to-")) return isCemTypographyDimensionToken(token);
                if (name === "cem/number/unitless") return isCemNumericToken(token, resolvedOptions);
                return true;
            },
            transform(tokenOrArgs, options) {
                const { token, options: resolvedOptions } = styleDictionaryArgs(tokenOrArgs, options);
                return cemTransformValue(name, token, resolvedOptions);
            },
        },
    ]),
);

export const CEM_STYLE_DICTIONARY_FILTER_DEFINITIONS = {
    "cem/category/web-only-filter": {
        name: "cem/category/web-only-filter",
        filter(tokenOrArgs) {
            const { token } = styleDictionaryArgs(tokenOrArgs);
            return !isCemWebOnlyToken(token);
        },
    },
};

export function registerCemStyleDictionaryTransforms(styleDictionary) {
    const target = styleDictionary?.default ?? styleDictionary;
    if (!target?.registerTransform || !target?.registerFilter) {
        throw new Error("Style Dictionary instance must expose registerTransform() and registerFilter().");
    }

    for (const transform of Object.values(CEM_STYLE_DICTIONARY_TRANSFORM_DEFINITIONS)) {
        target.registerTransform(transform);
    }

    for (const filter of Object.values(CEM_STYLE_DICTIONARY_FILTER_DEFINITIONS)) {
        target.registerFilter(filter);
    }
}

const jsonModeFiles = CEM_PLATFORM_MODES.map((mode) => ({
    destination: `cem-tokens-${mode}.json`,
    format: "json/nested",
    filter: "cem/category/web-only-filter",
    options: { mode },
}));

export default {
    source: ["dist/lib/tokens/cem.tokens.json"],
    hooks: {
        transforms: CEM_STYLE_DICTIONARY_TRANSFORM_DEFINITIONS,
        filters: CEM_STYLE_DICTIONARY_FILTER_DEFINITIONS,
    },
    platforms: {
        json: {
            buildPath: "dist/lib/token-platforms/json/",
            transforms: ["cem/mode/expand-themes"],
            files: jsonModeFiles,
        },
        ios: {
            buildPath: "dist/lib/token-platforms/ios/",
            transforms: ["cem/mode/expand-themes", "cem/size/layout-to-pt", "cem/size/type-to-pt", "cem/number/unitless"],
            files: [
                {
                    destination: "CEMTokens.swift",
                    format: "ios-swift/enum.swift",
                    filter: "cem/category/web-only-filter",
                    options: { mode: "light" },
                },
            ],
        },
        android: {
            buildPath: "dist/lib/token-platforms/android/",
            transforms: ["cem/mode/expand-themes", "cem/size/layout-to-dp", "cem/size/type-to-sp", "cem/number/unitless"],
            files: [
                {
                    destination: "values/cem-tokens.xml",
                    format: "android/resources",
                    filter: "cem/category/web-only-filter",
                    options: { mode: "light" },
                },
                {
                    destination: "values-night/cem-tokens.xml",
                    format: "android/resources",
                    filter: "cem/category/web-only-filter",
                    options: { mode: "dark" },
                },
            ],
        },
    },
};
