import * as wasm from '@epa-wg/cem-ml/wasm';
import runtimeMetadata from '@epa-wg/cem-ml/runtime.json' with { type: 'json' };
import {
    buildBrowserCommandInvocation,
    commandSchema,
    parseCemMlCommand,
} from '@epa-wg/cem-ml-cli/browser';
import type { CommandServiceResultV1 } from '@epa-wg/cem-ml/wasm';
import { highlightCemSource, renderCemMlSource, type CemMlDiagnostic } from './cem-ml-runtime.js';

export interface PlaygroundFormat {
    id: string;
    label: string;
    contentType: string;
}

// These are the native CLI's textual LayerFormat exports. The generated CLI
// grammar below filters them against the runtime's available output formats.
const TEXT_FORMATS: readonly PlaygroundFormat[] = [
    { id: 'html', label: 'HTML', contentType: 'text/html' },
    { id: 'cem', label: 'CEM-ML', contentType: 'application/cem' },
    { id: 'xml', label: 'XML', contentType: 'application/xml' },
    { id: 'xhtml', label: 'XHTML', contentType: 'application/xhtml+xml' },
    { id: 'svg', label: 'SVG', contentType: 'image/svg+xml' },
    { id: 'mathml', label: 'MathML', contentType: 'application/mathml+xml' },
    { id: 'xslt', label: 'XSLT', contentType: 'application/xslt+xml' },
    { id: 'json', label: 'JSON', contentType: 'application/json' },
    { id: 'yaml', label: 'YAML', contentType: 'application/yaml' },
    { id: 'csv', label: 'CSV', contentType: 'text/csv' },
    { id: 'markdown', label: 'Markdown', contentType: 'text/markdown; charset=utf-8; variant=CommonMark' },
    { id: 'css', label: 'CSS', contentType: 'text/css' },
];

export interface PlaygroundRenderResult {
    status: 'rendered' | 'error';
    sourceHtml: string;
    sourceText: string;
    outputText: string;
    outputHtml: string;
    inputFormat: string;
    diagnostics: CemMlDiagnostic[];
}

interface ConversionResult {
    text: string;
    diagnostics: CemMlDiagnostic[];
}

class PlaygroundConversionError extends Error {
    constructor(readonly diagnostics: CemMlDiagnostic[]) {
        super(diagnostics.map(({ message }) => message).join('\n') || 'CEM could not convert this document.');
    }
}

/** Discover the textual formats shared by the built native command grammar. */
export async function getPlaygroundFormats(): Promise<{
    inputs: PlaygroundFormat[];
    outputs: PlaygroundFormat[];
}> {
    const available = commandSchema.commands.find(({ name }) => name === 'convert')
        ?.arguments.find(({ long }) => long === 'to-format')?.possibleValues;
    const xmlDialects = new Set(['xhtml', 'svg', 'mathml', 'xslt']);
    const formats = TEXT_FORMATS.filter(({ id }) => available?.includes(xmlDialects.has(id) ? 'xml' : id))
        .map((format) => ({ ...format }));
    return { inputs: formats, outputs: formats.map((format) => ({ ...format })) };
}

/**
 * A syntax hint for pasted text without a filename or Content-Type header.
 * Ambiguous text can always be selected explicitly; parsing and all AST work
 * stay inside CEM's native import/conversion pipeline.
 */
export function detectPlaygroundFormat(source: string): string {
    const text = source.trimStart();
    if (/<svg\b[^>]*xmlns=["']http:\/\/www\.w3\.org\/2000\/svg["']/.test(text)) return 'svg';
    if (/<math\b[^>]*xmlns=["']http:\/\/www\.w3\.org\/1998\/Math\/MathML["']/.test(text)) return 'mathml';
    if (/xmlns:\w+=["']http:\/\/www\.w3\.org\/1999\/XSL\/Transform["']/.test(text)) return 'xslt';
    if (/<html\b[^>]*xmlns=["']http:\/\/www\.w3\.org\/1999\/xhtml["']/.test(text)) return 'xhtml';
    if (/^<\?xml\b/i.test(text)) return 'xml';
    if (/^<(?:!doctype\b|!--|[a-z])/i.test(text)) return 'html';
    if (/^(?:@(?:doc|ns|default)\b|\{\s*[\w:-]+(?:\s|[|@}]))/.test(text)) return 'cem';
    if (/^(?:\[|\{\s*(?:"|})|"|true\b|false\b|null\b|-?\d)/.test(text)) return 'json';
    if (/^(?:@(?:media|supports|layer|import)\b|[^{}\n]+\{[^{}]*[\w-]+\s*:)/.test(text)) return 'css';
    if (/^(?:---\s*\n|[\w .-]+:\s|\s*-\s+[\w .-]+:)/.test(text)) return 'yaml';
    const lines = text.trimEnd().split(/\r?\n/);
    if (lines.length > 1 && lines.slice(0, 2).every((line) => line.includes(','))) return 'csv';
    return 'markdown';
}

/** Import source into CEM, format it, and export the requested representation. */
export async function renderPlaygroundSource(
    source: string,
    inputFormat = 'auto',
    outputFormat = 'html',
): Promise<PlaygroundRenderResult> {
    const selected = inputFormat === 'auto' ? detectPlaygroundFormat(source) : inputFormat;
    const input = requireFormat(selected);
    const output = requireFormat(outputFormat);
    let sourceHtml = escapeHtml(source);
    let sourceText = source;
    const diagnostics: CemMlDiagnostic[] = [];

    try {
        const formatted = await presentSource(source, input);
        sourceHtml = formatted.html;
        sourceText = formatted.text;
        diagnostics.push(...formatted.diagnostics);

        if (input.id === output.id && output.id !== 'html') {
            return {
                status: 'rendered', sourceHtml, sourceText, inputFormat: selected,
                outputText: sourceText, outputHtml: sourceHtml, diagnostics,
            };
        }

        const converted = input.id === 'cem' && output.id === 'html'
            ? await renderCemHtml(source)
            : await convert(source, input, output, false);
        diagnostics.push(...converted.diagnostics);
        if (output.id === 'html') {
            return {
                status: 'rendered', sourceHtml, sourceText, inputFormat: selected,
                outputText: converted.text, outputHtml: converted.text,
                diagnostics: uniqueDiagnostics(diagnostics),
            };
        }
        const presentation = await presentSource(converted.text, output);
        diagnostics.push(...presentation.diagnostics);
        return {
            status: 'rendered', sourceHtml, sourceText, inputFormat: selected,
            outputText: presentation.text, outputHtml: presentation.html,
            diagnostics: uniqueDiagnostics(diagnostics),
        };
    } catch (error) {
        diagnostics.push(...(error instanceof PlaygroundConversionError ? error.diagnostics : [{
            code: 'cem.playground.render_failed', severity: 'error',
            message: error instanceof Error ? error.message : String(error),
        }]));
        return {
            status: 'error', sourceHtml, sourceText, outputText: '', outputHtml: '',
            inputFormat: selected, diagnostics: uniqueDiagnostics(diagnostics),
        };
    }
}

async function renderCemHtml(source: string): Promise<ConversionResult> {
    const rendered = await renderCemMlSource(source, 'memory:playground.cem');
    if (rendered.status !== 'rendered') throw new PlaygroundConversionError(rendered.diagnostics);
    return { text: rendered.html, diagnostics: rendered.diagnostics };
}

async function presentSource(source: string, format: PlaygroundFormat): Promise<{
    html: string;
    text: string;
    diagnostics: CemMlDiagnostic[];
}> {
    // CSS's browser export requires plain CSS; CEM's writer exports native
    // text. Their shared Rust source highlighter supplies semantic HTML after
    // the native formatter, without parsing either language in JavaScript.
    if (format.id === 'cem' || format.id === 'css') {
        const formatted = await convert(source, format, format, false);
        const colored = await highlightCemSource(formatted.text, format.contentType);
        return {
            html: colored.html, text: formatted.text,
            diagnostics: [...formatted.diagnostics, ...colored.diagnostics],
        };
    }
    const formatted = await convert(source, format, format, true);
    const presentation = document.createElement('template');
    presentation.innerHTML = formatted.text;
    const pre = presentation.content.querySelector('pre.cem-output');
    return {
        html: pre ? pre.innerHTML : formatted.text,
        text: (pre ?? presentation.content).textContent ?? '',
        diagnostics: formatted.diagnostics,
    };
}

async function convert(
    source: string,
    input: PlaygroundFormat,
    output: PlaygroundFormat,
    color: boolean,
): Promise<ConversionResult> {
    const sourceUri = `memory:playground.${input.id}`;
    const argv = [
        'convert', sourceUri, '--content-type', input.contentType,
        '--to-content-type', output.contentType, '--cemt-formatter-profile', 'tabular',
        ...(color ? ['--cemt-color-profile', 'html'] : []),
    ];
    const invocation = await buildBrowserCommandInvocation(
        parseCemMlCommand(argv, { runtime: 'wasm-browser-worker' }),
        async ({ uri }) => {
            if (uri !== sourceUri) throw new Error(`This editor has no resource at ${uri}.`);
            return [{ uri, bytes: new TextEncoder().encode(source) }];
        },
        { projectId: 'cem-demo-playground', cwd: '/', resolverPolicyStamp: 'playground-memory-only-v1' },
    );
    const staged = new Map<string, { uri: string; bytes: Uint8Array }>();
    let text = '';
    try {
        // A small editor uses the same WASM command boundary directly. It does
        // not need worker-only module resolution or a second WASM instance.
        const result = JSON.parse(await wasm.executeCommandServiceV1(
            JSON.stringify(invocation.request),
            JSON.stringify({
                runtime: 'wasm-browser-worker', targetIdentity: 'wasm32-unknown-unknown:web-main-thread',
                abiIdentity: runtimeMetadata.abi.identity, debugControlActive: false,
            }),
            async () => JSON.stringify({
                project: invocation.request.project,
                resourceVersions: invocation.request.resourceVersions,
            }),
            async () => { throw new Error('The editor only reads its in-memory source.'); },
            async (json, bytes) => {
                const request = JSON.parse(json) as { uri: string };
                if (request.uri !== 'cem-stdio://stdout') throw new Error('Unexpected output destination.');
                const token = crypto.randomUUID();
                staged.set(token, { uri: request.uri, bytes: new Uint8Array(bytes) });
                return JSON.stringify({ token });
            },
            async (token) => {
                const prepared = staged.get(token);
                if (!prepared) throw new Error('The output transaction is no longer available.');
                text = new TextDecoder().decode(prepared.bytes);
                return JSON.stringify({ uri: prepared.uri });
            },
            async (token) => { staged.delete(token); },
        )) as CommandServiceResultV1 & { error?: { message: string } };
        if (result.error) throw new Error(result.error.message);
        if (result.status !== 'succeeded') throw new PlaygroundConversionError(result.diagnostics.items);
        return { text: exportText(text, output), diagnostics: result.diagnostics.items };
    } finally {
        wasm.disposeCommandArtifactsV1(invocation.request.requestId);
        staged.clear();
    }
}

function exportText(text: string, output: PlaygroundFormat): string {
    // The DOM-to-HTML/XML converters publish an explicit content artifact.
    // Unwrap only that named export boundary; never project or reconstruct an
    // AST from JSON. Native JSON output remains the requested JSON document.
    if ((output.id === 'html' || output.id === 'xml') && text.startsWith('{')) {
        const artifact: unknown = JSON.parse(text);
        if (artifact && typeof artifact === 'object'
            && 'kind' in artifact && artifact.kind === output.id
            && 'content' in artifact && typeof artifact.content === 'string') {
            return artifact.content;
        }
    }
    return text;
}

function requireFormat(id: string): PlaygroundFormat {
    const format = TEXT_FORMATS.find((candidate) => candidate.id === id);
    if (!format) throw new Error(`Unsupported format: ${id}`);
    return format;
}

function uniqueDiagnostics(diagnostics: CemMlDiagnostic[]): CemMlDiagnostic[] {
    const seen = new Set<string>();
    return diagnostics.filter((diagnostic) => {
        const key = JSON.stringify(diagnostic);
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
    });
}

function escapeHtml(text: string): string {
    return text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
}
