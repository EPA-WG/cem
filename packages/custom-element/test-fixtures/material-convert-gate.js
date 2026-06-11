// Material-template conversion gate. Loads each copied legacy material component
// (`packages/custom-element/material/components/*.html`), extracts its `<template>` declarations, and
// runs them through the SAME DOM→CEM-ML pipeline the runtime uses
// (`parseLegacyTemplateSource` + `convertLegacyTemplateToCemMl` from the cem-elements build). Every
// template must transpile without unexpected diagnostics, and each manifest-listed primary component
// template must produce non-empty CEM-ML output — so drift in the copied files, or a converter
// regression, fails `@epa-wg/custom-element:test`.
import {
    convertLegacyTemplateToCemMl,
    parseLegacyTemplateSource,
} from '/packages/cem-elements/dist/lib/legacy-xslt/convert.js';
import { LEGACY_XSLT_DIAGNOSTIC_CODES } from '/packages/cem-elements/dist/index.js';

const MANIFEST_ALLOWED_DIAGNOSTIC_CODES = new Set([
    LEGACY_XSLT_DIAGNOSTIC_CODES.unsupportedConstruct,
    LEGACY_XSLT_DIAGNOSTIC_CODES.unsupportedFunction,
]);

export async function runMaterialConvertGate() {
    const errors = [];
    const manifest = await loadManifest(errors);
    if (!manifest) {
        return { done: true, errors };
    }
    let templateCount = 0;

    for (const component of manifest.materialComponents ?? []) {
        const name = component.name;
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
        const templateResults = new Map();
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
            templateResults.set(template, result);
            const unexpected = result.diagnostics.filter((d) => !isAllowedDiagnostic(component, d));
            if (unexpected.length > 0) {
                const detail = unexpected.map((d) => `${d.code} — ${d.message}`).join('; ');
                errors.push(`${name} template#${index}: ${detail}`);
            }
        });

        verifyAllowedDiagnosticBudgets(name, component, templates, templateResults, errors);
        verifyRequiredTemplates(name, component, doc, templateResults, errors);
    }

    if (templateCount < (manifest.materialComponents?.length ?? 0)) {
        errors.push(`expected at least ${manifest.materialComponents.length} templates, converted ${templateCount}`);
    }

    return { done: true, errors };
}

async function loadManifest(errors) {
    try {
        const response = await fetch('/packages/custom-element/test-fixtures/legacy-compat-manifest.json');
        if (!response.ok) {
            errors.push(`legacy compat manifest: HTTP ${response.status}`);
            return null;
        }
        const manifest = await response.json();
        if (manifest.schemaVersion !== 1 || !Array.isArray(manifest.materialComponents)) {
            errors.push('legacy compat manifest: expected schemaVersion=1 and materialComponents[]');
            return null;
        }
        validateManifest(manifest, errors);
        return manifest;
    } catch (error) {
        errors.push(`legacy compat manifest: ${error.message}`);
        return null;
    }
}

function validateManifest(manifest, errors) {
    const names = new Set();
    for (const component of manifest.materialComponents) {
        if (!component.name || names.has(component.name)) {
            errors.push(`legacy compat manifest: duplicate or missing component name \`${component.name ?? ''}\``);
        }
        names.add(component.name);
        for (const allow of component.allowedDiagnostics ?? []) {
            if (!MANIFEST_ALLOWED_DIAGNOSTIC_CODES.has(allow.code)) {
                errors.push(`${component.name}: manifest allowlist uses unsupported diagnostic code \`${allow.code}\``);
            }
        }
    }
}

function isAllowedDiagnostic(component, diagnostic) {
    return (component.allowedDiagnostics ?? []).some((allow) => diagnosticMatches(allow, diagnostic));
}

function diagnosticMatches(allow, diagnostic) {
    if (allow.code && diagnostic.code !== allow.code) {
        return false;
    }
    if (allow.messageIncludes && !diagnostic.message.includes(allow.messageIncludes)) {
        return false;
    }
    return true;
}

function verifyAllowedDiagnosticBudgets(name, component, templates, templateResults, errors) {
    for (const allow of component.allowedDiagnostics ?? []) {
        const count = templates
            .flatMap((template) => templateResults.get(template)?.diagnostics ?? [])
            .filter((diagnostic) => diagnosticMatches(allow, diagnostic)).length;
        if (typeof allow.maxCount === 'number' && count > allow.maxCount) {
            errors.push(
                `${name}: ${allow.code} (${allow.messageIncludes ?? 'any message'}) emitted ${count} times; max ${allow.maxCount}`
            );
        }
    }
}

function verifyRequiredTemplates(name, component, doc, templateResults, errors) {
    for (const required of component.requiredTemplates ?? []) {
        const template = doc.querySelector(required.selector);
        if (!template) {
            errors.push(`${name}: required template selector \`${required.selector}\` did not match`);
            continue;
        }
        const result = templateResults.get(template);
        if (!result) {
            errors.push(`${name}: required template \`${required.selector}\` was not converted`);
            continue;
        }
        const minSourceLength = required.minSourceLength ?? 1;
        if (result.source.trim().length < minSourceLength) {
            errors.push(
                `${name}: required template \`${required.selector}\` produced ${result.source.trim().length} CEM-ML chars; expected >= ${minSourceLength}`
            );
        }
    }
}
