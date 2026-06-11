/**
 * Legacy HTML+XSLT → canonical CEM-ML source conversion.
 *
 * Backward-compat bridge: legacy `<custom-element>` templates were authored as declarative
 * HTML + XSLT (`xsl:*` elements, the bare `for-each`/`if`/`choose` spellings, `{…}` AVT, and XPath
 * function calls). Rather than re-introduce a browser `XSLTProcessor` (forbidden by the FF-5 gate),
 * we transpile the parsed template DOM — read into the serializable {@link TemplateSourceNode} tree —
 * into **canonical CEM-ML source text**, then render it through the same cem_ql WASM engine that
 * hand-migrated `type="cem-ml; version=0.0"` templates use. A legacy sample and its migrated CEM-ML
 * twin therefore run on one engine and produce identical output.
 *
 * Scope: Tier 1 (material bare-form constructs) + Tier 2 (inline `xsl:` demos and the XSLT test
 * stories). Tier 3 standalone XSLT stylesheets (push-model `apply-templates`/`call-template`/`sort`,
 * EXSLT `func:function`, `msxsl:script`) are out of scope and emit a diagnostic.
 */

import type { TemplateSourceNode, TemplateSourceAttribute } from '../projection.js';

const XSL_NAMESPACE = 'http://www.w3.org/1999/XSL/Transform';

/** Declaration tags that the runtime treats as non-output and the render plan drops. */
const DECLARATION_TAGS = new Set(['attribute', 'slice', 'data', 'option', 'module-url']);

/** Constructs we deliberately do not transpile (Tier 3 / non-transpilable). */
const UNSUPPORTED_LOCAL_NAMES = new Set([
    'template',
    'apply-templates',
    'call-template',
    'with-param',
    'param',
    'sort',
    'copy',
    'copy-of',
    'element',
    'function',
    'script',
    'stylesheet',
    'output',
]);

export interface LegacyConversionDiagnostic {
    code: string;
    message: string;
}

export interface LegacyConversionResult {
    /** Canonical CEM-ML source text, ready for the cem_ql render boundary. */
    source: string;
    diagnostics: LegacyConversionDiagnostic[];
}

interface ConvertCtx {
    diagnostics: LegacyConversionDiagnostic[];
    /** Innermost `for-each` loop variable, so `.`/`@attr`/`position()` resolve to it. */
    loopVar: string | null;
}

/**
 * Convert a legacy HTML+XSLT template (as a {@link TemplateSourceNode} tree) to canonical CEM-ML
 * source text. The caller routes the result through the cem_ql render boundary.
 */
export function convertLegacyTemplateToCemMl(
    nodes: readonly TemplateSourceNode[],
): LegacyConversionResult {
    const ctx: ConvertCtx = { diagnostics: [], loopVar: null };
    const source = nodes.map((node) => emitNode(node, ctx)).join('');
    return { source, diagnostics: ctx.diagnostics };
}

function localName(tag: string): string {
    const colon = tag.indexOf(':');
    return colon >= 0 ? tag.slice(colon + 1) : tag;
}

function isXsltElement(node: Extract<TemplateSourceNode, { kind: 'element' }>): boolean {
    return node.namespace === XSL_NAMESPACE || node.tag.startsWith('xsl:');
}

function emitNode(node: TemplateSourceNode, ctx: ConvertCtx): string {
    if (node.kind === 'text') {
        return emitText(node.text, ctx);
    }
    if (node.kind === 'comment') {
        // CEM-ML output drops authoring comments (no structural equivalent needed for parity).
        return '';
    }
    return emitElement(node, ctx);
}

function emitElement(node: Extract<TemplateSourceNode, { kind: 'element' }>, ctx: ConvertCtx): string {
    const name = localName(node.tag);

    // Control-flow constructs (both `xsl:` and the bare legacy spellings).
    switch (name) {
        case 'value-of':
            return emitValueOf(node, ctx);
        case 'text':
            return emitXslText(node);
        case 'if':
            return emitIf(node, ctx);
        case 'choose':
            return emitChoose(node, ctx);
        case 'when':
        case 'otherwise':
            // Handled inside emitChoose; a stray branch outside choose is dropped with a diagnostic.
            ctx.diagnostics.push({
                code: 'legacy_xslt.orphan_branch',
                message: `<${node.tag}> outside <choose> is ignored`,
            });
            return '';
        case 'for-each':
            return emitForEach(node, ctx);
        case 'variable':
            return emitVariable(node, ctx);
        case 'slot':
            return emitSlot(node, ctx);
    }

    if (UNSUPPORTED_LOCAL_NAMES.has(name) && (isXsltElement(node) || name === 'function')) {
        ctx.diagnostics.push({
            code: 'legacy_xslt.unsupported_construct',
            message: `<${node.tag}> (Tier 3 / non-transpilable) is not converted`,
        });
        return '';
    }

    // Declaration passthrough: `<attribute>`/`<slice>` etc. keep their CEM-ML declaration shape; the
    // render plan drops them from output but applies defaults / wiring.
    if (DECLARATION_TAGS.has(name) && !isXsltElement(node)) {
        return emitGenericElement(node, ctx, name);
    }

    // Ordinary HTML/SVG element. Strip a redundant `xhtml:` authoring prefix (used to dodge the HTML
    // parser's table content model) — the canonical output is namespace-clean HTML.
    const tag = node.tag.startsWith('xhtml:') ? name : node.tag;
    return emitGenericElement(node, ctx, tag);
}

function emitGenericElement(
    node: Extract<TemplateSourceNode, { kind: 'element' }>,
    ctx: ConvertCtx,
    tag: string,
): string {
    const attrs = node.attributes.map((attr) => emitAttribute(attr, ctx)).join('');
    const isStyle = localName(tag) === 'style';
    const body = isStyle
        ? emitRichContent(textContent(node))
        : node.children.map((child) => emitNode(child, ctx)).join('');
    if (body === '') {
        return `{${tag}${attrs}}`;
    }
    return `{${tag}${attrs} | ${body}}`;
}

function emitAttribute(attr: TemplateSourceAttribute, ctx: ConvertCtx): string {
    // Drop XML namespace declarations — CEM-ML carries its own `@ns` directives.
    if (attr.name === 'xmlns' || attr.name.startsWith('xmlns:')) {
        return '';
    }
    // Attribute value template: rewrite `{xpath}` spans to `{cem-ql}` interpolation, keep literals.
    return attrAssign(attr.name, interpolate(attr.value, ctx));
}

function emitText(text: string, ctx: ConvertCtx): string {
    return interpolate(text, ctx);
}

/** Rewrite `{xpath}` AVT spans into `{cem-ql}` interpolation; leave surrounding literal text. */
function interpolate(text: string, ctx: ConvertCtx): string {
    return text.replace(/\{([^{}]*)\}/g, (_match, expression: string) => {
        const rewritten = rewriteExpression(expression, ctx);
        return `{${rewritten}}`;
    });
}

function emitValueOf(node: Extract<TemplateSourceNode, { kind: 'element' }>, ctx: ConvertCtx): string {
    const select = attrValue(node, 'select');
    if (select === null) {
        ctx.diagnostics.push({
            code: 'legacy_xslt.value_of_missing_select',
            message: '<value-of> without @select is ignored',
        });
        return '';
    }
    return `{${rewriteExpression(select, ctx)}}`;
}

function emitXslText(node: Extract<TemplateSourceNode, { kind: 'element' }>): string {
    // `xsl:text` emits its literal content verbatim (no interpolation).
    return escapeLiteral(textContent(node));
}

function emitIf(node: Extract<TemplateSourceNode, { kind: 'element' }>, ctx: ConvertCtx): string {
    const test = attrValue(node, 'test');
    if (test === null) {
        ctx.diagnostics.push({
            code: 'legacy_xslt.if_missing_test',
            message: '<if> without @test is ignored',
        });
        return '';
    }
    const body = node.children.map((child) => emitNode(child, ctx)).join('');
    return `{cem:if${exprAttr('test', rewriteExpression(test, ctx))} | ${body}}`;
}

function emitChoose(node: Extract<TemplateSourceNode, { kind: 'element' }>, ctx: ConvertCtx): string {
    const branches: string[] = [];
    for (const child of node.children) {
        if (child.kind !== 'element') {
            continue;
        }
        const name = localName(child.tag);
        if (name === 'when') {
            const test = attrValue(child, 'test');
            if (test === null) {
                ctx.diagnostics.push({
                    code: 'legacy_xslt.when_missing_test',
                    message: '<when> without @test is ignored',
                });
                continue;
            }
            const body = child.children.map((node) => emitNode(node, ctx)).join('');
            branches.push(`{cem:when${exprAttr('test', rewriteExpression(test, ctx))} | ${body}}`);
        } else if (name === 'otherwise') {
            const body = child.children.map((node) => emitNode(node, ctx)).join('');
            branches.push(`{cem:otherwise | ${body}}`);
        }
    }
    return `{cem:choose | ${branches.join('')}}`;
}

function emitForEach(node: Extract<TemplateSourceNode, { kind: 'element' }>, ctx: ConvertCtx): string {
    const select = attrValue(node, 'select');
    if (select === null) {
        ctx.diagnostics.push({
            code: 'legacy_xslt.for_each_missing_select',
            message: '<for-each> without @select is ignored',
        });
        return '';
    }
    const loopVar = 'item';
    const childCtx: ConvertCtx = { diagnostics: ctx.diagnostics, loopVar };
    const body = node.children.map((child) => emitNode(child, childCtx)).join('');
    return `{cem:for-each${exprAttr('select', rewriteExpression(select, ctx))} @as="${loopVar}" | ${body}}`;
}

function emitVariable(node: Extract<TemplateSourceNode, { kind: 'element' }>, ctx: ConvertCtx): string {
    // `<variable name select>` and `<xsl:variable>` with an inline node-set literal are handled at
    // the `for-each` that selects them (the common legacy idiom inlines the sequence). A standalone
    // variable produces no output here; record a diagnostic if it carries an unsupported body.
    if (attrValue(node, 'select') === null && node.children.length > 0) {
        ctx.diagnostics.push({
            code: 'legacy_xslt.inline_variable_deferred',
            message: `<${node.tag}> inline node-set is not yet inlined; reference it via @select`,
        });
    }
    return '';
}

function emitSlot(node: Extract<TemplateSourceNode, { kind: 'element' }>, ctx: ConvertCtx): string {
    const nameAttr = node.attributes.find((attr) => attr.name === 'name');
    const fallback = node.children.map((child) => emitNode(child, ctx)).join('');
    const attrs = nameAttr ? attrAssign('name', nameAttr.value) : '';
    if (fallback === '') {
        return `{slot${attrs}}`;
    }
    return `{slot${attrs} | ${fallback}}`;
}

// ---------------------------------------------------------------------------
// XPath expression → cem-ql expression rewriting
// ---------------------------------------------------------------------------

/** XPath function name → cem-ql `prefix:name`. `concat`/`not`/`position` are special-cased. */
const FUNCTION_MAP: Record<string, string> = {
    contains: 'str:contains',
    'starts-with': 'str:starts_with',
    'ends-with': 'str:ends_with',
    'normalize-space': 'str:normalize_space',
    translate: 'str:translate',
    substring: 'str:substring',
    'substring-before': 'str:substring_before',
    'substring-after': 'str:substring_after',
    'string-length': 'str:length',
    count: 'seq:count',
};

/**
 * Rewrite a (subset of) XPath expression to an equivalent cem-ql expression. Handles the forms the
 * Tier 1/2 legacy samples use: variable refs (`$x`), the context item (`.`), attribute steps
 * (`@a`), slice/datadom paths (`//x`, `/datadom/x`), string/numeric literals, comparison/boolean
 * operators, the `??` coalesce, and the function set in {@link FUNCTION_MAP} plus `not`/`position`/
 * `concat`. Unrecognized fragments are passed through (cem-ql shares much of XPath's surface).
 */
export function rewriteExpression(expression: string, ctx: ConvertCtx): string {
    const tokens = tokenizeXPath(expression);
    const parser = new XPathRewriter(tokens, ctx);
    const out = parser.rewriteAll();
    return out.trim();
}

type XToken =
    | { kind: 'string'; value: string }
    | { kind: 'number'; value: string }
    | { kind: 'name'; value: string }
    | { kind: 'var'; value: string }
    | { kind: 'punct'; value: string };

function tokenizeXPath(input: string): XToken[] {
    const tokens: XToken[] = [];
    let i = 0;
    const isNameStart = (c: string) => /[A-Za-z_]/.test(c);
    const isNameChar = (c: string) => /[A-Za-z0-9_.\-:]/.test(c);
    while (i < input.length) {
        const c = input[i];
        if (/\s/.test(c)) {
            i += 1;
            continue;
        }
        if (c === '"' || c === "'") {
            let j = i + 1;
            let value = '';
            while (j < input.length && input[j] !== c) {
                value += input[j];
                j += 1;
            }
            tokens.push({ kind: 'string', value });
            i = j + 1;
            continue;
        }
        if (/[0-9]/.test(c)) {
            let j = i;
            while (j < input.length && /[0-9.]/.test(input[j])) {
                j += 1;
            }
            tokens.push({ kind: 'number', value: input.slice(i, j) });
            i = j;
            continue;
        }
        if (c === '$') {
            let j = i + 1;
            while (j < input.length && isNameChar(input[j])) {
                j += 1;
            }
            tokens.push({ kind: 'var', value: input.slice(i + 1, j) });
            i = j;
            continue;
        }
        if (c === '@') {
            let j = i + 1;
            while (j < input.length && isNameChar(input[j])) {
                j += 1;
            }
            tokens.push({ kind: 'punct', value: '@' });
            tokens.push({ kind: 'name', value: input.slice(i + 1, j) });
            i = j;
            continue;
        }
        if (c === '/' && input[i + 1] === '/') {
            tokens.push({ kind: 'punct', value: '//' });
            i += 2;
            continue;
        }
        if (c === '?' && input[i + 1] === '?') {
            tokens.push({ kind: 'punct', value: '??' });
            i += 2;
            continue;
        }
        if (c === '!' && input[i + 1] === '=') {
            tokens.push({ kind: 'punct', value: '!=' });
            i += 2;
            continue;
        }
        if (isNameStart(c)) {
            let j = i;
            while (j < input.length && isNameChar(input[j])) {
                j += 1;
            }
            tokens.push({ kind: 'name', value: input.slice(i, j) });
            i = j;
            continue;
        }
        tokens.push({ kind: 'punct', value: c });
        i += 1;
    }
    return tokens;
}

class XPathRewriter {
    private pos = 0;

    constructor(private readonly tokens: XToken[], private readonly ctx: ConvertCtx) {}

    rewriteAll(): string {
        let out = '';
        while (this.pos < this.tokens.length) {
            out += this.rewriteToken();
        }
        return out;
    }

    private peek(): XToken | undefined {
        return this.tokens[this.pos];
    }

    private rewriteToken(): string {
        const token = this.tokens[this.pos];
        this.pos += 1;
        switch (token.kind) {
            case 'string':
                return `"${token.value.replace(/"/g, '\\"')}" `;
            case 'number':
                return `${token.value} `;
            case 'var':
                return `$${token.value} `;
            case 'punct':
                return this.rewritePunct(token.value);
            case 'name':
                return this.rewriteName(token.value);
        }
    }

    private rewritePunct(value: string): string {
        if (value === '//') {
            // `//name` → datadom slice path; consume the following name.
            const next = this.peek();
            if (next && next.kind === 'name') {
                this.pos += 1;
                return `$datadom.slices.${next.value} `;
            }
            return '';
        }
        if (value === '@') {
            const next = this.peek();
            if (next && next.kind === 'name') {
                this.pos += 1;
                const base = this.ctx.loopVar ? `$${this.ctx.loopVar}` : '$datadom.attributes';
                return `${base}.${next.value} `;
            }
            return '';
        }
        if (value === '.') {
            return this.ctx.loopVar ? `$${this.ctx.loopVar} ` : '. ';
        }
        if (value === '??' || value === '=' || value === '!=' || value === '(' || value === ')' ||
            value === ',' || value === '<' || value === '>' || value === '+' || value === '-' ||
            value === '*') {
            return `${value} `;
        }
        return `${value} `;
    }

    private rewriteName(value: string): string {
        // Function call?
        if (this.peek()?.kind === 'punct' && this.peek()?.value === '(') {
            return this.rewriteCall(value);
        }
        // Boolean operators / keywords pass through.
        if (value === 'and' || value === 'or' || value === 'div' || value === 'mod') {
            return `${value} `;
        }
        if (value === 'true' || value === 'false') {
            return `${value} `;
        }
        // Bare path step (e.g. a datadom field / slice referenced by name).
        return `$datadom.slices.${value} `;
    }

    private rewriteCall(name: string): string {
        // Consume '(' … ')' capturing argument fragments split on top-level commas.
        this.pos += 1; // skip '('
        const args: string[] = [];
        let depth = 0;
        let current = '';
        while (this.pos < this.tokens.length) {
            const token = this.tokens[this.pos];
            if (token.kind === 'punct' && token.value === '(') {
                depth += 1;
                current += this.rewriteToken();
                continue;
            }
            if (token.kind === 'punct' && token.value === ')') {
                if (depth === 0) {
                    this.pos += 1;
                    break;
                }
                depth -= 1;
                current += this.rewriteToken();
                continue;
            }
            if (token.kind === 'punct' && token.value === ',' && depth === 0) {
                args.push(current.trim());
                current = '';
                this.pos += 1;
                continue;
            }
            current += this.rewriteToken();
        }
        if (current.trim() !== '') {
            args.push(current.trim());
        }
        return this.emitCall(name, args);
    }

    private emitCall(name: string, args: string[]): string {
        if (name === 'position') {
            return '$position ';
        }
        if (name === 'not') {
            return `not (${args.join(', ')}) `;
        }
        if (name === 'concat') {
            // XPath concat(a, b, …) → cem-ql str:concat((a, b, …)) (sequence join, empty separator).
            return `str:concat((${args.join(', ')})) `;
        }
        const mapped = FUNCTION_MAP[name];
        if (mapped) {
            return `${mapped}(${args.join(', ')}) `;
        }
        this.ctx.diagnostics.push({
            code: 'legacy_xslt.unsupported_function',
            message: `XPath function ${name}() has no cem-ql mapping; passed through`,
        });
        return `${name}(${args.join(', ')}) `;
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

function attrValue(node: Extract<TemplateSourceNode, { kind: 'element' }>, name: string): string | null {
    const attr = node.attributes.find((attr) => attr.name === name);
    return attr ? attr.value : null;
}

function textContent(node: Extract<TemplateSourceNode, { kind: 'element' }>): string {
    return node.children
        .map((child) => (child.kind === 'text' ? child.text : child.kind === 'element' ? textContent(child) : ''))
        .join('');
}

/** A CEM-ML control-flow attribute (`@test`/`@select`) carrying a cem-ql expression. */
function exprAttr(name: string, expr: string): string {
    return attrAssign(name, expr);
}

/**
 * Emit ` @name="value"` choosing the quote style by content. The rewriter emits double-quoted
 * cem-ql string literals, so a value containing `"` is single-quote wrapped (parity with the
 * hand-migrated generators' `@test='row.td2 = "0px"'`). Values with both quote kinds escape the
 * single quote (best effort — not exercised by the bridged samples).
 */
function attrAssign(name: string, value: string): string {
    if (!value.includes('"')) {
        return ` @${name}="${value}"`;
    }
    if (!value.includes("'")) {
        return ` @${name}='${value}'`;
    }
    return ` @${name}='${value.replace(/'/g, '&apos;')}'`;
}

/** Escape literal text for CEM-ML output: structural braces/pipe need rich-content wrapping. */
function escapeLiteral(text: string): string {
    if (/[{}|`]/.test(text)) {
        return emitRichContent(text);
    }
    return text;
}

/** Wrap literal content (e.g. CSS rule blocks with `{ }`) in CEM-ML triple-backtick rich content. */
function emitRichContent(text: string): string {
    return '```' + text + '```';
}
