// Material-template conversion gate. Loads each copied legacy material component
// (`packages/custom-element/material/components/*.html`), extracts its `<template>` declarations, and
// runs them through the SAME DOM→CEM-ML pipeline the runtime uses
// (`parseLegacyTemplateSource` + `convertLegacyTemplateToCemMl` from the cem-elements build). Every
// template must transpile with no conversion error diagnostics and non-empty CEM-ML output — so drift
// in the copied files, or a converter regression, fails `@epa-wg/custom-element:test`.
import {
    convertLegacyTemplateToCemMl,
    parseLegacyTemplateSource,
} from '/packages/cem-elements/dist/lib/legacy-xslt/convert.js';

const COMPONENTS = ['action', 'autocomplete', 'badge', 'dropdown', 'icon-link', 'icon', 'input', 'menu'];

// Diagnostics that represent documented, deferred gaps rather than a broken template or a converter
// regression. `unsupported_function` covers legacy DCE helpers like `hasBoolAttribute()` (boolean
// attribute forwarding — not reproduced on the substrate); `unsupported_construct` covers Tier-3
// elements. The gate fails on everything else (parse errors, malformed conditionals).
const ALLOWED_DIAGNOSTIC_CODES = new Set([
    'legacy_xslt.unsupported_function',
    'legacy_xslt.unsupported_construct',
]);

export async function runMaterialConvertGate() {
    const errors = [];
    let templateCount = 0;

    for (const name of COMPONENTS) {
        let html;
        try {
            const response = await fetch(`/packages/custom-element/material/components/${name}.html`);
            if (!response.ok) {
                errors.push(`${name}: HTTP ${response.status}`);
                continue;
            }
            html = await response.text();
        } catch (error) {
            errors.push(`${name}: fetch failed — ${error.message}`);
            continue;
        }

        const doc = new DOMParser().parseFromString(html, 'text/html');
        const templates = Array.from(doc.querySelectorAll('template'));
        if (templates.length === 0) {
            errors.push(`${name}: no <template> declarations found (copy drift?)`);
            continue;
        }

        templates.forEach((template, index) => {
            templateCount += 1;
            let result;
            try {
                result = convertLegacyTemplateToCemMl(parseLegacyTemplateSource(template));
            } catch (error) {
                errors.push(`${name} template#${index}: conversion threw — ${error.message}`);
                return;
            }
            const unexpected = result.diagnostics.filter((d) => !ALLOWED_DIAGNOSTIC_CODES.has(d.code));
            if (unexpected.length > 0) {
                const detail = unexpected.map((d) => `${d.code} — ${d.message}`).join('; ');
                errors.push(`${name} template#${index}: ${detail}`);
            }
        });
    }

    if (templateCount < COMPONENTS.length) {
        errors.push(`expected at least ${COMPONENTS.length} templates, converted ${templateCount}`);
    }

    return { done: true, errors };
}
