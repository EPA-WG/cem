/**
 * Legacy HTML+XSLT → canonical CEM-ML source conversion.
 *
 * Backward-compat bridge: legacy `<custom-element>` templates were authored as declarative
 * HTML + XSLT (`xsl:*` elements, the bare `for-each`/`if`/`choose` spellings, `{…}` AVT, and XPath
 * function calls). Rather than re-introduce a browser XSLT engine (forbidden by the FF-5 gate),
 * we transpile the parsed template DOM — read into the serializable {@link TemplateSourceNode} tree —
 * into **canonical CEM-ML source text**, then render it through the same cem_ql WASM engine that
 * hand-migrated `type="cem-ml; version=0.0"` templates use. A legacy sample and its migrated CEM-ML
 * twin therefore run on one engine and produce identical output.
 *
 * Scope: Tier 1 (material bare-form constructs) + Tier 2 (inline `xsl:` demos and the XSLT test
 * stories). Tier 3 standalone XSLT stylesheets (push-model `apply-templates`/`call-template`/`sort`,
 * EXSLT `func:function`, `msxsl:script`) are out of scope and emit a diagnostic.
 */

import { readTemplateSource, type TemplateSourceNode, type TemplateSourceAttribute } from '../projection.js';

const XSL_NAMESPACE = 'http://www.w3.org/1999/XSL/Transform';
const XHTML_NAMESPACE = 'http://www.w3.org/1999/xhtml';

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

/** A materialized member of an inline `<variable>` node-set literal. */
interface ItemNode {
    text: string;
    attrs: Record<string, string>;
}

interface ConvertCtx {
    diagnostics: LegacyConversionDiagnostic[];
    /** Innermost `for-each` loop variable, so `.`/`@attr`/`position()` resolve to it. */
    loopVar: string | null;
    /** In-scope inline `<variable>` node-set literals, keyed by name (for for-each unrolling). */
    nodeSets: Map<string, ItemNode[]>;
    /** In-scope `<variable name select>` scalar definitions → their rewritten cem-ql expression. */
    scalars: Map<string, string>;
    /** When unrolling, the current member: `.`/`@attr`/`position()` resolve to its literal values. */
    item?: ItemNode & { position: number };
}

/**
 * Convert a legacy HTML+XSLT template (as a {@link TemplateSourceNode} tree) to canonical CEM-ML
 * source text. The caller routes the result through the cem_ql render boundary.
 */
export function convertLegacyTemplateToCemMl(
    nodes: readonly TemplateSourceNode[],
): LegacyConversionResult {
    const ctx: ConvertCtx = {
        diagnostics: [],
        loopVar: null,
        nodeSets: new Map(),
        scalars: new Map(),
    };
    return { source: emitChildren(nodes, ctx), diagnostics: ctx.diagnostics };
}

/**
 * Emit a sibling list, first hoisting any `<variable>` definitions into a child scope. Inline
 * node-set literals feed for-each unrolling; scalar `select` variables feed predicate/expression
 * substitution. The `<variable>` elements themselves produce no output.
 */
function emitChildren(nodes: readonly TemplateSourceNode[], ctx: ConvertCtx): string {
    const scoped = withVariableScope(nodes, ctx);
    return nodes.map((node) => emitNode(node, scoped)).join('');
}

function withVariableScope(nodes: readonly TemplateSourceNode[], ctx: ConvertCtx): ConvertCtx {
    let nodeSets = ctx.nodeSets;
    let scalars = ctx.scalars;
    for (const node of nodes) {
        if (node.kind !== 'element' || localName(node.tag) !== 'variable') {
            continue;
        }
        const name = attrValue(node, 'name');
        if (!name) {
            continue;
        }
        const select = attrValue(node, 'select');
        const members = node.children.filter(
            (child): child is Extract<TemplateSourceNode, { kind: 'element' }> => child.kind === 'element'
        );
        if (select === null && members.length > 0) {
            nodeSets = new Map(nodeSets);
            nodeSets.set(name, members.map(toItemNode));
        } else if (select !== null) {
            scalars = new Map(scalars);
            scalars.set(name, rewriteExpression(select, ctx));
        }
    }
    return nodeSets === ctx.nodeSets && scalars === ctx.scalars ? ctx : { ...ctx, nodeSets, scalars };
}

function toItemNode(element: Extract<TemplateSourceNode, { kind: 'element' }>): ItemNode {
    const attrs: Record<string, string> = {};
    for (const attribute of element.attributes) {
        attrs[attribute.name] = attribute.value;
    }
    return { text: textContent(element), attrs };
}

/**
 * Read a legacy HTML+XSLT `<template>` into the serializable source tree with HTML and XSLT
 * recognized as distinct namespaces. The browser HTML parser garbles non-HTML constructs (`xsl:*`,
 * `for-each`) — especially inside table content models — so the raw markup is re-parsed as XML under
 * both the XHTML default namespace and the XSL namespace. Falls back to the HTML-parsed content when
 * the XML is not well-formed. Pair with {@link convertLegacyTemplateToCemMl}; the runtime and the
 * material-template gate share this one pipeline.
 */
export function parseLegacyTemplateSource(template: HTMLTemplateElement): TemplateSourceNode[] {
    const raw = template.innerHTML.trim().length > 0 ? template.innerHTML : template.textContent ?? '';
    const xml =
        `<cem-legacy-root xmlns="${XHTML_NAMESPACE}" xmlns:xsl="${XSL_NAMESPACE}" ` +
        `xmlns:xhtml="${XHTML_NAMESPACE}">${raw}</cem-legacy-root>`;
    try {
        const parsed = new DOMParser().parseFromString(xml, 'application/xml');
        if (parsed.querySelector('parsererror') === null && parsed.documentElement) {
            return readTemplateSource(parsed.documentElement);
        }
    } catch {
        // fall through to the HTML-parsed content
    }
    return readTemplateSource(template.content);
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
            return emitVariable();
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
    const body = isStyle ? emitRichContent(textContent(node)) : emitChildren(node.children, ctx);
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
        // While unrolling a node-set member, `.`/`@attr`/`position()` become literal text/values.
        if (ctx.item) {
            const literal = resolveItemLiteral(expression.trim(), ctx.item);
            if (literal !== null) {
                return literal;
            }
        }
        return `{${rewriteExpression(expression, ctx, true)}}`;
    });
}

/** Resolve `.` / `@attr` / `position()` against the current unrolled member, else null. */
function resolveItemLiteral(expression: string, item: ItemNode & { position: number }): string | null {
    if (expression === '.') {
        return item.text;
    }
    if (expression === 'position()') {
        return String(item.position);
    }
    const attr = expression.match(/^@([\w-]+)$/);
    if (attr) {
        return item.attrs[attr[1]] ?? '';
    }
    return null;
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
    return `{${rewriteExpression(select, ctx, true)}}`;
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
    const body = emitChildren(node.children, ctx);
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
            const body = emitChildren(child.children, ctx);
            branches.push(`{cem:when${exprAttr('test', rewriteExpression(test, ctx))} | ${body}}`);
        } else if (name === 'otherwise') {
            const body = emitChildren(child.children, ctx);
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
    // Inline node-set idiom: `for-each select="exsl:node-set($var)/*[pred?]"` over a literal
    // `<variable>` — unroll at conversion time into static CEM-ML (no runtime loop), substituting
    // `.`/`@attr`/`position()` with each member's literal values. A predicate becomes a per-item
    // `cem:if`. This is the legacy parity path for the for-each demos.
    const nodeSetRef = matchNodeSetSelect(select, ctx);
    if (nodeSetRef) {
        const members = ctx.nodeSets.get(nodeSetRef.name) ?? [];
        const predicateTest = nodeSetRef.predicate ? rewritePredicate(nodeSetRef.predicate, ctx) : null;
        return members
            .map((member, index) => {
                const itemCtx: ConvertCtx = {
                    ...ctx,
                    loopVar: null,
                    item: { ...member, position: index + 1 },
                };
                const body = emitChildren(node.children, itemCtx);
                return predicateTest ? `{cem:if${exprAttr('test', predicateTest)} | ${body}}` : body;
            })
            .join('');
    }

    const loopVar = 'item';
    const childCtx: ConvertCtx = { ...ctx, loopVar, item: undefined };
    const body = emitChildren(node.children, childCtx);
    return `{cem:for-each${exprAttr('select', rewriteExpression(select, ctx))} @as="${loopVar}" | ${body}}`;
}

/**
 * Recognize a for-each `@select` that iterates an inline node-set literal: `exsl:node-set($X)/*`,
 * `exsl:node-set($X)/*[PRED]`, or a bare `$X`, where `X` is an in-scope inline `<variable>`. Returns
 * the variable name and any predicate, or null when the select is a runtime sequence (kept as a real
 * `cem:for-each`).
 */
function matchNodeSetSelect(select: string, ctx: ConvertCtx): { name: string; predicate: string | null } | null {
    const trimmed = select.trim();
    const nodeSet = trimmed.match(/^(?:exsl:)?node-set\(\s*\$([\w-]+)\s*\)\s*\/\s*\*\s*(?:\[(.+)\])?$/);
    if (nodeSet && ctx.nodeSets.has(nodeSet[1])) {
        return { name: nodeSet[1], predicate: nodeSet[2] ?? null };
    }
    const bare = trimmed.match(/^\$([\w-]+)$/);
    if (bare && ctx.nodeSets.has(bare[1])) {
        return { name: bare[1], predicate: null };
    }
    return null;
}

/** A for-each predicate `[$cond]`: a scalar `<variable>` ref resolves to its select; else rewritten. */
function rewritePredicate(predicate: string, ctx: ConvertCtx): string {
    const scalarRef = predicate.trim().match(/^\$([\w-]+)$/);
    if (scalarRef && ctx.scalars.has(scalarRef[1])) {
        return ctx.scalars.get(scalarRef[1]) as string;
    }
    return rewriteExpression(predicate, ctx);
}

function emitVariable(): string {
    // `<variable>` definitions are hoisted into scope by `withVariableScope` (inline node-set
    // literals feed for-each unrolling; scalar `select` variables feed predicate substitution) and
    // produce no output of their own.
    return '';
}

function emitSlot(node: Extract<TemplateSourceNode, { kind: 'element' }>, ctx: ConvertCtx): string {
    const nameAttr = node.attributes.find((attr) => attr.name === 'name');
    const fallback = emitChildren(node.children, ctx);
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
 * `concat`.
 *
 * Variable references are emitted **bare** (no `$`): cem-ql's `@test`/`@select` expression grammar
 * binds bare names but rejects `$x` inside compound expressions (functions, operators, `not`). When
 * `interpolation` is set (an element/attribute `{…}` span) and the whole expression is a single
 * simple path, a leading `$` is added — the form element-content interpolation requires (`{$icon}`).
 */
export function rewriteExpression(expression: string, ctx: ConvertCtx, interpolation = false): string {
    const tokens = tokenizeXPath(expression);
    const bare = new XPathRewriter(tokens, ctx).rewriteAll().trim();
    if (interpolation && isSimplePath(bare)) {
        return `$${bare}`;
    }
    return bare;
}

/** A bare dotted identifier path (`icon`, `item.hex`, `datadom.slices.x`) — safe to `$`-prefix. */
function isSimplePath(expression: string): boolean {
    return /^[A-Za-z_][\w.]*$/.test(expression);
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
                // Bare variable reference (no `$`) — see rewriteExpression.
                return `${token.value} `;
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
                return `datadom.slices.${next.value} `;
            }
            return '';
        }
        if (value === '@') {
            const next = this.peek();
            if (next && next.kind === 'name') {
                this.pos += 1;
                const base = this.ctx.loopVar ? this.ctx.loopVar : 'datadom.attributes';
                return `${base}.${next.value} `;
            }
            return '';
        }
        if (value === '.') {
            return this.ctx.loopVar ? `${this.ctx.loopVar} ` : '. ';
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
        // Bare name → a flat binding reference (legacy DCE `{name}` resolves the host attribute /
        // dataset / slice of that name; the runtime seeds those as flat bindings). Slices are
        // referenced explicitly with `//name`.
        return `${value} `;
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
            return 'position ';
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
