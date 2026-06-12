#!/usr/bin/env node
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const ROOT = process.cwd();
const SCAN_ROOTS = ['material/components', 'demo'];
const OUTPUT_DIR = path.join(ROOT, 'dist', 'reports');
const JSON_OUT = path.join(OUTPUT_DIR, 'xslt-compat-inventory.json');
const MD_OUT = path.join(OUTPUT_DIR, 'xslt-compat-inventory.md');

const XSL_NAMESPACE = 'http://www.w3.org/1999/XSL/Transform';
const XHTML_NAMESPACE = 'http://www.w3.org/1999/xhtml';

const XSLT_INSTRUCTION_NAMES = new Set([
    'apply-templates',
    'attribute',
    'call-template',
    'choose',
    'copy',
    'copy-of',
    'element',
    'for-each',
    'if',
    'otherwise',
    'output',
    'param',
    'sort',
    'stylesheet',
    'template',
    'text',
    'value-of',
    'variable',
    'when',
    'with-param',
]);

const BARE_LEGACY_INSTRUCTION_NAMES = new Set([
    'apply-templates',
    'attribute',
    'call-template',
    'choose',
    'for-each',
    'if',
    'otherwise',
    'param',
    'sort',
    'template',
    'text',
    'value-of',
    'variable',
    'when',
    'with-param',
]);

const XPATH_ATTRS = new Set(['select', 'test', 'match', 'use']);

async function main() {
    const files = [];
    for (const scanRoot of SCAN_ROOTS) {
        await collectSourceFiles(path.join(ROOT, scanRoot), files);
    }
    files.sort((a, b) => a.localeCompare(b));

    const fileReports = [];
    for (const file of files) {
        const source = await readFile(file, 'utf8');
        fileReports.push(analyzeFile(file, source));
    }

    const inventory = {
        schemaVersion: 1,
        generatedBy: 'packages/custom-element/scripts/inventory-xslt-compat.mjs',
        scope: {
            roots: SCAN_ROOTS,
            fileExtensions: ['.html', '.xhtml', '.xml', '.xsl', '.xslt'],
        },
        totals: summarize(fileReports),
        files: fileReports,
    };

    await mkdir(OUTPUT_DIR, { recursive: true });
    await writeFile(JSON_OUT, `${JSON.stringify(inventory, null, 2)}\n`);
    await writeFile(MD_OUT, renderMarkdown(inventory));

    const summary = inventory.totals;
    console.log(`Scanned ${summary.files} custom-element material/demo files.`);
    console.log(`XSLT instruction names: ${Array.from(summary.xsltInstructions).join(', ') || '(none)'}`);
    console.log(`XPath functions: ${Array.from(summary.xpathFunctions).join(', ') || '(none)'}`);
    console.log(`EXSLT functions: ${Array.from(summary.exsltFunctions).join(', ') || '(none)'}`);
    console.log(`Wrote ${path.relative(ROOT, JSON_OUT)} and ${path.relative(ROOT, MD_OUT)}.`);

    if (summary.files === 0) {
        console.error('No custom-element material/demo files were scanned.');
        process.exitCode = 1;
    }
}

async function collectSourceFiles(dir, out) {
    let entries;
    try {
        entries = await readdir(dir, { withFileTypes: true });
    } catch (error) {
        if (error.code === 'ENOENT') {
            return;
        }
        throw error;
    }
    for (const entry of entries) {
        const absolute = path.join(dir, entry.name);
        if (entry.isDirectory()) {
            await collectSourceFiles(absolute, out);
            continue;
        }
        if (entry.isFile() && ['.html', '.xhtml', '.xml', '.xsl', '.xslt'].includes(path.extname(entry.name))) {
            out.push(absolute);
        }
    }
}

function analyzeFile(file, source) {
    const withoutComments = source.replace(/<!--[\s\S]*?-->/g, '');
    const relativePath = path.relative(ROOT, file);
    const group = relativePath.startsWith('material/') ? 'material' : 'demo';
    const namespaces = scanNamespaces(withoutComments);
    const tags = scanTags(withoutComments, namespaces);
    const xpathAttrs = scanXPathAttributes(withoutComments);
    const xpathExpressions = xpathAttrs.map((attr) => attr.value);
    const avtExpressions = scanAvtExpressions(withoutComments);
    const allExpressions = [...xpathExpressions, ...avtExpressions];
    const xpathFunctions = uniqueSorted(allExpressions.flatMap(scanFunctions));
    const exsltFunctions = xpathFunctions.filter((name) => name.startsWith('exsl:') || name.startsWith('exslt:'));
    const xpathFeatures = scanXPathFeatures(allExpressions);
    const templateFeatures = scanTemplateFeatures(tags, xpathAttrs);

    return {
        path: relativePath,
        group,
        namespaces,
        xsltInstructions: uniqueSorted(tags.filter((tag) => tag.kind === 'xslt').map((tag) => tag.localName)),
        bareLegacyInstructions: uniqueSorted(tags.filter((tag) => tag.kind === 'bare-legacy').map((tag) => tag.localName)),
        templateFeatures,
        xpathAttributes: xpathAttrs,
        xpathFunctions,
        exsltFunctions,
        xpathFeatures,
    };
}

function scanNamespaces(source) {
    const namespaces = [];
    const seen = new Set();
    const attrPattern = /\bxmlns(?::([A-Za-z_][\w.-]*))?\s*=\s*(["'])([\s\S]*?)\2/g;
    let match;
    while ((match = attrPattern.exec(source))) {
        const prefix = match[1] ?? '';
        const uri = decodeEntities(match[3]);
        const key = `${prefix}\u0000${uri}`;
        if (!seen.has(key)) {
            seen.add(key);
            namespaces.push({ prefix, uri });
        }
    }
    namespaces.sort((a, b) => `${a.prefix}:${a.uri}`.localeCompare(`${b.prefix}:${b.uri}`));
    return namespaces;
}

function scanTags(source, namespaces) {
    const xslPrefixes = new Set(
        namespaces.filter((namespace) => namespace.uri === XSL_NAMESPACE && namespace.prefix).map((namespace) => namespace.prefix)
    );
    xslPrefixes.add('xsl');

    const tags = [];
    const tagPattern = /<\s*\/?\s*([A-Za-z_][\w:.-]*)\b/g;
    let match;
    while ((match = tagPattern.exec(source))) {
        const rawName = match[1];
        const [prefix, localName] = splitName(rawName);
        let kind = 'output';
        if ((prefix && xslPrefixes.has(prefix)) || (!prefix && rawName === 'stylesheet' && hasDefaultNamespace(namespaces, XSL_NAMESPACE))) {
            kind = 'xslt';
        } else if (!prefix && BARE_LEGACY_INSTRUCTION_NAMES.has(localName)) {
            kind = 'bare-legacy';
        } else if (prefix === 'xhtml' || (!prefix && hasDefaultNamespace(namespaces, XHTML_NAMESPACE))) {
            kind = 'html';
        }
        tags.push({ rawName, prefix, localName, kind });
    }
    return tags;
}

function hasDefaultNamespace(namespaces, uri) {
    return namespaces.some((namespace) => namespace.prefix === '' && namespace.uri === uri);
}

function scanXPathAttributes(source) {
    const attrs = [];
    const attrPattern = /\b(select|test|match|use)\s*=\s*(["'])([\s\S]*?)\2/g;
    let match;
    while ((match = attrPattern.exec(source))) {
        const name = match[1];
        const value = decodeEntities(match[3]);
        if (XPATH_ATTRS.has(name)) {
            attrs.push({ name, value });
        }
    }
    return attrs;
}

function scanAvtExpressions(source) {
    const expressions = [];
    const attrPattern = /\b[A-Za-z_:][\w:.-]*\s*=\s*(["'])([\s\S]*?)\1/g;
    let match;
    while ((match = attrPattern.exec(source))) {
        const value = decodeEntities(match[2]);
        for (const expression of extractBracedExpressions(value)) {
            expressions.push(expression);
        }
    }
    const textWithoutTags = source.replace(/<[^>]*>/g, ' ');
    for (const expression of extractBracedExpressions(decodeEntities(textWithoutTags))) {
        expressions.push(expression);
    }
    return expressions;
}

function extractBracedExpressions(value) {
    const expressions = [];
    const pattern = /\{([^{}]+)\}/g;
    let match;
    while ((match = pattern.exec(value))) {
        const expression = match[1].trim();
        if (isLikelyXPathExpression(expression)) {
            expressions.push(expression);
        }
    }
    return expressions;
}

function scanFunctions(expression) {
    const functions = [];
    const pattern = /([A-Za-z_][\w.-]*(?::[A-Za-z_][\w.-]*)?)\s*\(/g;
    let match;
    while ((match = pattern.exec(expression))) {
        const name = match[1];
        if (!['and', 'or', 'div', 'mod'].includes(name)) {
            functions.push(name);
        }
    }
    return functions;
}

function isLikelyXPathExpression(expression) {
    if (!expression || expression.includes(';') || expression.includes('{') || expression.includes('}')) {
        return false;
    }
    if (/^(?:#[\da-fA-F]{3,8}|[A-Za-z-]+\s*:|--[A-Za-z-]+|[.\d]+(?:px|rem|em|s|ms|%)?)\b/.test(expression)) {
        return false;
    }
    return (
        expression.startsWith('$') ||
        expression.startsWith('/') ||
        expression.startsWith('.') ||
        expression.startsWith('@') ||
        expression.includes('//') ||
        expression.includes('exsl:') ||
        expression.includes('exslt:') ||
        /\b(?:not|contains|starts-with|substring|substring-before|substring-after|string-length|normalize-space|translate|count|position|name|local-name|current|sum|text|node)\s*\(/.test(
            expression
        )
    );
}

function scanXPathFeatures(expressions) {
    const features = new Set();
    for (const expression of expressions) {
        if (expression.includes('..')) features.add('parent-axis-or-parent-step');
        if (expression.includes('//')) features.add('descendant-or-self-abbrev');
        if (/@\*|@[A-Za-z_]/.test(expression)) features.add('attribute-axis');
        if (/\|/.test(expression)) features.add('union');
        if (/\[[^\]]+\]/.test(expression)) features.add('predicate');
        if (/\btext\s*\(\s*\)/.test(expression)) features.add('text-node-test');
        if (/\bnode\s*\(\s*\)/.test(expression)) features.add('node-test');
        if (/\*\s*\[|\/\*/.test(expression) || expression.includes('*|')) features.add('wildcard');
        if (/\$[A-Za-z_][\w.-]*/.test(expression)) features.add('variables');
        if (/\b(current|name|local-name|position|count|normalize-space|string-length|contains|not|substring|translate)\s*\(/.test(expression)) {
            features.add('xpath-functions');
        }
        if (/\bexsl:?t?:node-set\s*\(/.test(expression)) features.add('exsl-node-set');
        if (/^\s*\.\s*$|[/(]\s*\.\s*[)\]]?/.test(expression)) features.add('current-item-dot');
    }
    return Array.from(features).sort();
}

function scanTemplateFeatures(tags, attrs) {
    const instructions = new Set(tags.filter((tag) => tag.kind === 'xslt' || tag.kind === 'bare-legacy').map((tag) => tag.localName));
    const features = [];
    for (const name of [
        'stylesheet',
        'template',
        'apply-templates',
        'call-template',
        'with-param',
        'param',
        'sort',
        'for-each',
        'if',
        'choose',
        'variable',
    ]) {
        if (instructions.has(name)) {
            features.push(name);
        }
    }
    if (attrs.some((attr) => attr.name === 'mode')) features.push('mode');
    if (attrs.some((attr) => attr.name === 'priority')) features.push('priority');
    if (attrs.some((attr) => attr.name === 'match')) features.push('match-patterns');
    return uniqueSorted(features);
}

function summarize(fileReports) {
    return {
        files: fileReports.length,
        groups: {
            material: fileReports.filter((file) => file.group === 'material').length,
            demo: fileReports.filter((file) => file.group === 'demo').length,
        },
        xsltInstructions: uniqueSorted(fileReports.flatMap((file) => file.xsltInstructions)),
        bareLegacyInstructions: uniqueSorted(fileReports.flatMap((file) => file.bareLegacyInstructions)),
        templateFeatures: uniqueSorted(fileReports.flatMap((file) => file.templateFeatures)),
        xpathFunctions: uniqueSorted(fileReports.flatMap((file) => file.xpathFunctions)),
        exsltFunctions: uniqueSorted(fileReports.flatMap((file) => file.exsltFunctions)),
        xpathFeatures: uniqueSorted(fileReports.flatMap((file) => file.xpathFeatures)),
    };
}

function renderMarkdown(inventory) {
    const lines = [];
    lines.push('# Custom-Element XSLT Compatibility Inventory');
    lines.push('');
    lines.push('Generated by `packages/custom-element/scripts/inventory-xslt-compat.mjs`.');
    lines.push('');
    lines.push('## Summary');
    lines.push('');
    lines.push(`- Files scanned: ${inventory.totals.files}`);
    lines.push(`- Material files: ${inventory.totals.groups.material}`);
    lines.push(`- Demo/reference files: ${inventory.totals.groups.demo}`);
    lines.push(`- XSLT instructions: ${formatList(inventory.totals.xsltInstructions)}`);
    lines.push(`- Bare legacy instructions: ${formatList(inventory.totals.bareLegacyInstructions)}`);
    lines.push(`- Template features: ${formatList(inventory.totals.templateFeatures)}`);
    lines.push(`- XPath functions: ${formatList(inventory.totals.xpathFunctions)}`);
    lines.push(`- EXSLT functions: ${formatList(inventory.totals.exsltFunctions)}`);
    lines.push(`- XPath feature markers: ${formatList(inventory.totals.xpathFeatures)}`);
    lines.push('');
    lines.push('## File Matrix');
    lines.push('');
    lines.push('| File | Group | Template features | XPath functions | XPath markers |');
    lines.push('| --- | --- | --- | --- | --- |');
    for (const file of inventory.files) {
        lines.push(
            `| \`${file.path}\` | ${file.group} | ${formatList(file.templateFeatures)} | ${formatList(file.xpathFunctions)} | ${formatList(file.xpathFeatures)} |`
        );
    }
    lines.push('');
    return `${lines.join('\n')}\n`;
}

function formatList(values) {
    if (!values || values.length === 0) {
        return '-';
    }
    return values.map((value) => `\`${value}\``).join(', ');
}

function splitName(rawName) {
    const index = rawName.indexOf(':');
    if (index === -1) {
        return ['', rawName];
    }
    return [rawName.slice(0, index), rawName.slice(index + 1)];
}

function uniqueSorted(values) {
    return Array.from(new Set(values)).sort((a, b) => a.localeCompare(b));
}

function decodeEntities(value) {
    return value
        .replace(/&lt;/g, '<')
        .replace(/&gt;/g, '>')
        .replace(/&quot;/g, '"')
        .replace(/&apos;/g, "'")
        .replace(/&amp;/g, '&');
}

main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
});
