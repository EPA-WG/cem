// Build-time inspection only. XML, CEM-ML and retained CSS adoption remain native.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import postcss from 'postcss';
import init, { compileTemplate, disposeTemplate } from '../../packages/cem_ql/dist/wasm/cem_ql.js';

await init({ module_or_path: readFileSync(new URL('../../packages/cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
const cli = fileURLToPath(new URL('../../packages/cem-ml-cli-npm/dist/bin.js', import.meta.url));

export function readCanonicalStyles(path, tag) {
    if (!/^cem-[a-z0-9-]+$/.test(tag)) throw new Error(`Invalid canonical component tag ${tag}`);
    // Declarations are XML fragments rooted at cem-element, not XHTML documents
    // rooted at html. The native XML lifecycle owns entity/CDATA handling.
    let queried;
    try {
        queried = JSON.parse(execFileSync(process.execPath, [cli, 'query', path,
            '--content-type', 'application/xml', '--query-content-type', 'application/vnd.cem.xpath',
            '--query', `//*[namespace-uri()="http://www.w3.org/1999/xhtml" and local-name()="template"][@id="${tag}"][@type="text/cem-ml"] ! string(.)`,
            '--output', 'json'], { encoding: 'utf8', maxBuffer: 4 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'] }));
    } catch (error) {
        throw new Error(`${path}: native XML template extraction failed: ${error.stderr ?? error.message}`, { cause: error });
    }
    const items = queried.result?.sequence?.items;
    if (items?.length !== 1 || typeof items[0].value?.lexicalValue !== 'string') {
        throw new Error(`${path}: expected exactly one ${tag} text/cem-ml template`);
    }
    const compiled = JSON.parse(compileTemplate(items[0].value.lexicalValue, '[]'));
    try {
        const errors = (compiled.diagnostics ?? []).filter(item => item.severity === 'error');
        if (compiled.error || errors.length) throw new Error(`${path}: native template compilation failed: ${JSON.stringify(compiled.error ?? errors)}`);
        if (!compiled.stylesheets?.length) throw new Error(`${path}: canonical component must own embedded static CSS`);
        return compiled.stylesheets.map(sheet => sheet.css).join('\n');
    } finally {
        if (compiled.artifactId !== undefined) disposeTemplate(compiled.artifactId);
    }
}

// Resolve matching paths for audit comparison only; this does not rewrite the
// retained stylesheet or attempt to reproduce browser cascade/specificity.
function nestedSelectors(selectorList, parents) {
    const children = postcss.list.comma(selectorList);
    if (!parents.length) return children;
    const parent = parents.length === 1 ? parents[0] : `:is(${parents.join(', ')})`;
    return children.map(child => {
        const positions = [];
        let quote = null, brackets = 0;
        for (let i = 0; i < child.length; i++) {
            const char = child[i];
            if (char === '\\') { i++; continue; }
            if (quote) { if (char === quote) quote = null; continue; }
            if (char === '"' || char === "'") { quote = char; continue; }
            if (char === '[') brackets++;
            else if (char === ']') brackets--;
            else if (char === '&' && brackets === 0) positions.push(i);
            else if (char === '/' && child[i + 1] === '*') {
                const end = child.indexOf('*/', i + 2);
                i = end < 0 ? child.length : end + 1;
            }
        }
        if (!positions.length) return `${parent} ${child}`;
        const replacement = parents.length === 1 && !(positions.length === 1 && positions[0] === 0)
            ? `:is(${parent})` : parent;
        for (const position of positions.reverse()) child = child.slice(0, position) + replacement + child.slice(position + 1);
        return child;
    });
}

export function inspectCssRules(pathLabel, cssText, allowedKeyframes = ['cem-progress-spinner-cycle']) {
    const failures = [];
    const fail = message => failures.push(`${pathLabel}: ${message}`);
    let root;
    try { root = postcss.parse(cssText, { from: pathLabel }); }
    catch (error) { return { rules: [], failures: [`${pathLabel}: invalid CSS: ${error.message}`] }; }
    const rules = [];
    function visit(container, parents = [], media = []) {
        const declarations = new Map();
        for (const node of container.nodes ?? []) {
            if (node.type !== 'decl') continue;
            if (!parents.length) fail('declaration outside a component selector');
            if (declarations.has(node.prop)) fail(`duplicate ${node.prop} declaration in \`${parents.join(', ')}\``);
            declarations.set(node.prop, node.value);
        }
        if (declarations.size) for (const selector of parents) rules.push({
            selector: selector.replace(/\s+/g, ' ').trim(), declarations,
            media: media.length ? media.join(' and ') : null,
        });
        for (const node of container.nodes ?? []) {
            if (node.type === 'comment' || node.type === 'decl') continue;
            if (node.type === 'rule') {
                const selectors = nestedSelectors(node.selector, parents);
                if (selectors.some(selector => !selector.trim())) fail('empty component selector');
                visit(node, selectors, media);
            }
            else if (node.type === 'atrule' && node.name === 'media' &&
                ['(forced-colors: active)', '(prefers-reduced-motion: reduce)'].includes(node.params)) {
                visit(node, parents, [...media, node.params]);
            } else if (node.type === 'atrule' && node.name === 'keyframes' && allowedKeyframes.includes(node.params)) {
                // Animation geometry is checked separately by its owning contract.
                continue;
            } else fail(`unsupported component stylesheet ${node.type === 'atrule' ? `at-rule @${node.name} ${node.params}` : node.type}`);
        }
    }
    visit(root);
    return { rules, failures };
}

const CSS_COLOR_LITERAL = /#[0-9a-f]{3,8}\b|\b(?:rgb|rgba|hsl|hsla|hwb|lab|lch|oklab|oklch|color-mix)\s*\(/i;
const CSS_SPACING_PROPERTY =
    /\b(?:margin|padding|gap|inset|top|right|bottom|left|width|height|min-width|max-width|min-height|max-height|border-radius|border-width|outline-width|font-size|line-height)\s*:[^;{}]*/gi;
const CSS_SPACING_LITERAL = /\b\d*\.?\d+(?:px|rem|em|vh|vw|vmin|vmax|ch|ex|%)\b|calc\s*\(/i;
export function inspectTokenUsage(pathLabel, cssText) {
    const failures = [];
    const fail = message => failures.push(message);
    if (CSS_COLOR_LITERAL.test(cssText)) {
        fail(`${pathLabel}: color literals are forbidden; use CEM theme color tokens`);
    }

    for (const match of cssText.matchAll(CSS_SPACING_PROPERTY)) {
        const declaration = match[0];
        if (CSS_SPACING_LITERAL.test(declaration) && !declaration.includes('var(--cem-')) {
            fail(`${pathLabel}: spacing literal \`${declaration.trim()}\` must resolve through a CEM theme token`);
        }
    }
    return failures;
}

function checkPendingKeyframes(label, css, fail) {
    let root;
    try { root = postcss.parse(css); } catch { return; }
    const animations = root.nodes.filter(node => node.type === 'atrule' && node.name === 'keyframes');
    const frames = animations[0]?.nodes ?? [];
    if (animations.length !== 1 || frames.length !== 2 ||
        frames[0].selector !== 'from' || frames[1].selector !== 'to' ||
        frames.some(frame => frame.nodes?.length !== 1 || frame.nodes[0].prop !== 'background-position-x') ||
        frames[0].nodes[0].value !== '0px' || frames[1].nodes[0].value !== 'var(--cem-pending-tile-size)') {
        fail(`${label}: pending keyframes must shift exactly one theme tile per cycle`);
    }
}


export function inspectActionStyles(label, tag, css) {
    const failures = [];
    const fail = message => failures.push(message);
    failures.push(...inspectTokenUsage(label, css));
    const { rules, failures: syntaxFailures } = inspectCssRules(label, css, ['cem-pending-shift']);
    failures.push(...syntaxFailures);
    const normal = new Map(rules.filter(rule => !rule.media).map(rule => [rule.selector, rule.declarations]));
    checkPendingKeyframes(label, css, fail);
    if (/--_cem-action-|--cem-action-internal-/.test(css)) {
        fail(`${label}: action must consume theme endpoints without private state aliases`);
    }
    const control = tag === 'cem-icon-button' ? ':is(button, a)' : 'button';
    for (const intent of ['primary', 'explicit', 'contextual', 'alternate', 'destructive']) {
        const host = intent === 'primary' ? ':scope' : `:scope:where([variant="${intent}"])`;
        for (const [state, suffix] of [
            ['default', ''], ['hover', ':where(:enabled:hover)'], ['active', ':where(:enabled:active)'],
            ['disabled', ":where(:disabled:not([aria-busy='true']))"], ['pending', ":where([aria-busy='true'])"],
        ]) {
            const adaptedSuffix = tag === 'cem-icon-button'
                ? suffix.replace(':enabled:hover', ':enabled:hover, [href]:hover').replace(':enabled:active', ':enabled:active, [href]:active').replace(":where(:disabled:not([aria-busy='true']))", ":where(:disabled, [aria-disabled='true']):where(:not([aria-busy='true']))")
                : suffix;
            const declarations = normal.get(`${host} > ${control}[part~='control']${adaptedSuffix}`);
            if (['hover', 'active', 'disabled'].includes(state) && declarations?.size !== 2) {
                fail(`${label}: ${intent} ${state} changes only the color pair`);
            }
            for (const [property, channel] of [['background-color', 'background'], ['color', 'text']]) {
                const expected = state === 'pending'
                    ? `var(--cem-action-${intent}-pending-${channel}, var(--cem-action-${intent}-active-${channel}))`
                    : `var(--cem-action-${intent}-${state}-${channel})`;
                if (declarations?.get(property) !== expected) {
                    fail(`${label}: ${intent} ${state} must consume its ${channel} theme endpoint directly`);
                }
            }
        }
    }
    return failures;
}
