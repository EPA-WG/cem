/**
 * Style Dictionary fan-out configuration for CEM token exports.
 *
 * Style Dictionary is intentionally not required by the MVP token exporter. The
 * config records the transform/filter names and output policy that the
 * post-MVP platform driver uses as the native outputs come online.
 */

export const CEM_PLATFORM_MODES = ["light", "dark", "contrast-light", "contrast-dark", "native"];

export const CEM_STYLE_DICTIONARY_TRANSFORMS = [
    "cem/size/layout-to-pt",
    "cem/size/type-to-pt",
    "cem/size/layout-to-dp",
    "cem/size/type-to-sp",
    "cem/number/unitless",
    "cem/category/web-only-filter",
    "cem/mode/expand-themes",
];

export default {
    source: ["dist/lib/tokens/cem.tokens.json"],
    platforms: {
        json: {
            buildPath: "dist/lib/token-platforms/json/",
            transforms: ["cem/mode/expand-themes"],
            files: CEM_PLATFORM_MODES.map((mode) => ({
                destination: `cem-tokens-${mode}.json`,
                format: "json/nested",
                options: { mode },
            })),
        },
    },
};
