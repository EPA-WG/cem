/**
 * C1.5 bespoke CEM-ML subset parser — a **narrowed fallback** (slices C2.3 / C2.6).
 *
 * Canonical CEM-ML renders through the `cem_ql` WASM boundary
 * ({@link ../internal/runtime-support/cem-ql-render.js}). As of C2.6 the WASM render plan
 * also drops top-level `<attribute>`/`<slice>` declaration nodes, so declaration-bearing
 * canonical templates render through WASM too. This TypeScript parser is now retained only
 * for: (1) the synchronous declaration scan — parse diagnostics and declared
 * `<attribute>`/`<slice>` extraction needed at `customElements.define` time (before the
 * async WASM module is ready); and (2) rendering the one construct the canonical engine
 * still does not handle — `${}` C1.5 text interpolation — plus WASM-unavailable hosts.
 * Full removal stays blocked on a synchronous declaration-metadata surface from WASM.
 */

import type { SourceMapRef, TemplateSourceAttribute, TemplateSourceNode } from '../projection.js';

export interface CemMlTemplateDiagnostic {
    code: string;
    message: string;
}

export interface CemMlTemplateParseResult {
    source: TemplateSourceNode[];
    diagnostics: CemMlTemplateDiagnostic[];
}

export function parseCemMlTemplateSource(input: string): CemMlTemplateParseResult {
    const parser = new CemMlTemplateParser(input);
    return parser.parse();
}

class CemMlTemplateParser {
    private readonly diagnostics: CemMlTemplateDiagnostic[] = [];
    private offset = 0;

    constructor(private readonly input: string) {}

    parse(): CemMlTemplateParseResult {
        const source: TemplateSourceNode[] = [];
        while (!this.atEnd()) {
            this.skipTrivia();
            if (this.atEnd()) {
                break;
            }
            if (this.peek() !== '{') {
                const text = this.readTextUntilNodeStart();
                if (text.trim().length > 0) {
                    source.push({ kind: 'text', text, sourceMapRef: this.sourceMapRef(this.offset - text.length) });
                }
                continue;
            }
            const node = this.readNode();
            if (node) {
                source.push(node);
            }
        }
        return { source, diagnostics: this.diagnostics };
    }

    private readNode(): TemplateSourceNode | undefined {
        const nodeStart = this.offset;
        this.consume('{');
        this.skipInlineWhitespace();

        const tag = this.readName();
        if (!tag) {
            this.diagnostic('cem-element.cem_ml.node_name_missing', 'CEM-ML node requires a name after `{`');
            this.recoverToNodeEnd();
            return undefined;
        }

        const attributes: TemplateSourceAttribute[] = [];
        while (!this.atEnd()) {
            this.skipInlineWhitespace();
            const char = this.peek();
            if (char === '@') {
                const attribute = this.readAttribute();
                if (attribute) {
                    attributes.push(attribute);
                }
                continue;
            }
            if (char === '|' || char === '}') {
                break;
            }
            this.diagnostic(
                'cem-element.cem_ml.unexpected_token',
                `Unexpected token \`${char ?? 'EOF'}\` in CEM-ML node \`${tag}\``
            );
            this.recoverToNodeEnd();
            return undefined;
        }

        const children: TemplateSourceNode[] = [];
        this.skipInlineWhitespace();
        if (this.peek() === '|') {
            this.consume('|');
            children.push(...this.readContent());
        }

        this.skipInlineWhitespace();
        if (!this.consume('}')) {
            this.diagnostic(
                'cem-element.cem_ml.node_unterminated',
                `CEM-ML node \`${tag}\` starting at offset ${nodeStart} is missing a closing \`}\``
            );
            this.recoverToNodeEnd();
        }

        return {
            kind: 'element',
            namespace: null,
            tag,
            attributes,
            children,
            sourceMapRef: this.sourceMapRef(nodeStart),
        };
    }

    private readAttribute(): TemplateSourceAttribute | undefined {
        this.consume('@');
        const name = this.readName();
        if (!name) {
            this.diagnostic('cem-element.cem_ml.attribute_name_missing', 'CEM-ML attribute requires a name after `@`');
            return undefined;
        }

        this.skipInlineWhitespace();
        if (!this.consume('=')) {
            return { name, value: '' };
        }
        this.skipInlineWhitespace();
        const value = this.readAttributeValue();
        return { name, value };
    }

    private readAttributeValue(): string {
        const quote = this.peek();
        if (quote === '"' || quote === "'") {
            this.offset += 1;
            const start = this.offset;
            while (!this.atEnd() && this.peek() !== quote) {
                this.offset += 1;
            }
            const value = this.input.slice(start, this.offset);
            this.consume(quote);
            return value;
        }

        if (this.peek() === '{') {
            return this.readBalancedBraces();
        }

        const start = this.offset;
        while (!this.atEnd() && !/[\s|}]/.test(this.peek() ?? '')) {
            this.offset += 1;
        }
        return this.input.slice(start, this.offset);
    }

    private readBalancedBraces(): string {
        const start = this.offset;
        let depth = 0;
        while (!this.atEnd()) {
            const char = this.peek();
            this.offset += 1;
            if (char === '{') {
                depth += 1;
            } else if (char === '}') {
                depth -= 1;
                if (depth === 0) {
                    return this.input.slice(start, this.offset);
                }
            }
        }
        this.diagnostic('cem-element.cem_ml.attribute_value_unterminated', 'CEM-ML braced attribute value is unterminated');
        return this.input.slice(start);
    }

    private readContent(): TemplateSourceNode[] {
        const children: TemplateSourceNode[] = [];
        let textStart = this.offset;

        const flushText = (end = this.offset): void => {
            if (end <= textStart) {
                return;
            }
            const text = this.input.slice(textStart, end);
            if (text.length > 0) {
                children.push({ kind: 'text', text, sourceMapRef: this.sourceMapRef(textStart) });
            }
        };

        while (!this.atEnd()) {
            if (this.startsWith('${')) {
                this.skipInterpolationSpan();
                continue;
            }

            if (this.peek() === '}') {
                flushText();
                return children;
            }

            if (this.startsWith('//')) {
                flushText();
                const commentStart = this.offset;
                children.push({ kind: 'comment', text: this.readLineComment(), sourceMapRef: this.sourceMapRef(commentStart) });
                textStart = this.offset;
                continue;
            }

            if (this.startsWith('/*')) {
                flushText();
                const commentStart = this.offset;
                children.push({ kind: 'comment', text: this.readBlockComment(), sourceMapRef: this.sourceMapRef(commentStart) });
                textStart = this.offset;
                continue;
            }

            if (this.isNodeStart()) {
                flushText();
                const node = this.readNode();
                if (node) {
                    children.push(node);
                }
                textStart = this.offset;
                continue;
            }

            this.offset += 1;
        }

        flushText();
        return children;
    }

    private readLineComment(): string {
        this.consume('/');
        this.consume('/');
        const start = this.offset;
        while (!this.atEnd() && this.peek() !== '\n') {
            this.offset += 1;
        }
        return this.input.slice(start, this.offset).trim();
    }

    private readBlockComment(): string {
        this.consume('/');
        this.consume('*');
        const start = this.offset;
        while (!this.atEnd() && !this.startsWith('*/')) {
            this.offset += 1;
        }
        const text = this.input.slice(start, this.offset).trim();
        if (!this.atEnd()) {
            this.consume('*');
            this.consume('/');
        }
        return text;
    }

    private readTextUntilNodeStart(): string {
        const start = this.offset;
        while (!this.atEnd() && !this.isNodeStart()) {
            if (this.startsWith('${')) {
                this.skipInterpolationSpan();
                continue;
            }
            this.offset += 1;
        }
        return this.input.slice(start, this.offset);
    }

    private readName(): string {
        const start = this.offset;
        while (!this.atEnd() && /[A-Za-z0-9_.:$-]/.test(this.peek() ?? '')) {
            this.offset += 1;
        }
        return this.input.slice(start, this.offset);
    }

    private skipTrivia(): void {
        while (!this.atEnd()) {
            this.skipInlineWhitespace();
            if (this.peek() === '\n' || this.peek() === '\r' || this.peek() === '\t') {
                this.offset += 1;
                continue;
            }
            break;
        }
    }

    private skipInlineWhitespace(): void {
        while (!this.atEnd() && /[ \n\r\t]/.test(this.peek() ?? '')) {
            this.offset += 1;
        }
    }

    private recoverToNodeEnd(): void {
        while (!this.atEnd() && this.peek() !== '}') {
            this.offset += 1;
        }
        this.consume('}');
    }

    private canStartNode(offset: number): boolean {
        return /[A-Za-z_$@]/.test(this.input[offset] ?? '');
    }

    private isNodeStart(): boolean {
        return this.peek() === '{' && this.input[this.offset - 1] !== '$' && this.canStartNode(this.offset + 1);
    }

    private skipInterpolationSpan(): void {
        this.offset += 2;
        while (!this.atEnd() && this.peek() !== '}') {
            this.offset += 1;
        }
        this.consume('}');
    }

    private startsWith(value: string): boolean {
        return this.input.startsWith(value, this.offset);
    }

    private consume(char: string): boolean {
        if (this.peek() !== char) {
            return false;
        }
        this.offset += char.length;
        return true;
    }

    private peek(): string | undefined {
        return this.input[this.offset];
    }

    private atEnd(): boolean {
        return this.offset >= this.input.length;
    }

    private diagnostic(code: string, message: string): void {
        this.diagnostics.push({ code, message });
    }

    private sourceMapRef(offset: number): SourceMapRef {
        return {
            fidelity: 'author-byte-exact',
            frame: `cem:${offset}`,
        };
    }
}
