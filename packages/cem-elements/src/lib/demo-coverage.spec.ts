import { readFileSync, readdirSync } from 'node:fs';
import ts from 'typescript';
import { describe, expect, it } from 'vitest';
import {
    assertAuthoredInventory, assertInventory, demoCoverage, normalizeLegend, type DemoCoverage,
} from '../../.storybook/demo-coverage.js';

const packageRoot = new URL('../../', import.meta.url);

function htmlPages(directory: URL, prefix = 'demo/'): string[] {
    return readdirSync(directory, { withFileTypes: true }).flatMap(entry =>
        entry.isDirectory() ? htmlPages(new URL(`${entry.name}/`, directory), `${prefix}${entry.name}/`)
            : entry.name.endsWith('.html') ? [`${prefix}${entry.name}`] : []);
}

function assertStoryContract(source: string, entry: DemoCoverage): void {
    const file = ts.createSourceFile(entry.storyFile, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
    const variables = new Map<string, ts.Expression>();
    const exports = new Set<string>();
    let meta: ts.Expression | undefined;
    for (const statement of file.statements) {
        if (ts.isExportAssignment(statement)) meta = statement.expression;
        if (!ts.isVariableStatement(statement)) continue;
        for (const declaration of statement.declarationList.declarations) {
            if (!ts.isIdentifier(declaration.name) || !declaration.initializer) continue;
            variables.set(declaration.name.text, declaration.initializer);
            if (statement.modifiers?.some(modifier => modifier.kind === ts.SyntaxKind.ExportKeyword)) {
                exports.add(declaration.name.text);
            }
        }
    }
    function resolve(expression: ts.Expression | undefined): ts.Expression | undefined {
        const seen = new Set<ts.Expression>();
        while (expression && !seen.has(expression)) {
            seen.add(expression);
            if (ts.isIdentifier(expression)) expression = variables.get(expression.text);
            else if (ts.isSatisfiesExpression(expression) || ts.isAsExpression(expression) || ts.isParenthesizedExpression(expression)) {
                expression = expression.expression;
            } else return expression;
        }
        return undefined;
    }
    function property(expression: ts.Expression | undefined, name: string): ts.Expression | undefined {
        const object = resolve(expression);
        if (!object || !ts.isObjectLiteralExpression(object)) return undefined;
        const field = object.properties.find(value => value.name
            && (ts.isIdentifier(value.name) || ts.isStringLiteral(value.name)) && value.name.text === name);
        if (field && ts.isPropertyAssignment(field)) return field.initializer;
        if (field && ts.isShorthandPropertyAssignment(field)) return field.name;
        return undefined;
    }
    const title = property(meta, 'title');
    if (!title || !ts.isStringLiteral(title) || title.text !== entry.title || property(meta, 'id')) {
        throw new Error(`${entry.page}: missing or changed story title/ID`);
    }
    if (!exports.has(entry.exportName)) throw new Error(`${entry.page}: missing story export ${entry.exportName}`);
    const story = variables.get(entry.exportName);
    if (property(story, 'name')) throw new Error(`${entry.page}: inventory owner must use its export name`);
    for (const object of [meta, story]) {
        const tags = resolve(property(object, 'tags'));
        if (tags && ts.isArrayLiteralExpression(tags) && tags.elements.some(element =>
            ts.isStringLiteral(element) && ['!test', '!dev'].includes(element.text))) {
            throw new Error(`${entry.page}: inventory story is excluded`);
        }
    }
    const render = resolve(property(story, 'render'));
    // Function declarations are also allowed as render references.
    const renderName = property(story, 'render');
    const namedRender = renderName && ts.isIdentifier(renderName) && file.statements.some(statement =>
        ts.isFunctionDeclaration(statement) && statement.name?.text === renderName.text);
    if (!namedRender && (!render || !(ts.isArrowFunction(render) || ts.isFunctionExpression(render)))) {
        throw new Error(`${entry.page}: missing render function`);
    }
    const play = resolve(property(story, 'play'));
    if (!play || !(ts.isArrowFunction(play) || ts.isFunctionExpression(play))
        || !play.modifiers?.some(modifier => modifier.kind === ts.SyntaxKind.AsyncKeyword)) {
        throw new Error(`${entry.page}: missing asynchronous play function`);
    }
}

describe('authored demo coverage inventory', () => {
    it('accounts for every HTML document, including nested support libraries', () => {
        const actual = ['index.html', ...htmlPages(new URL('demo/', packageRoot))].sort();
        assertInventory(actual, demoCoverage.map(entry => entry.page).sort(), 'demo pages');
        assertAuthoredInventory(new Map(demoCoverage.map(entry => [entry.page, entry.legends])));
    });

    it.each(demoCoverage)('$page names an existing asynchronous source-story contract', entry => {
        assertStoryContract(readFileSync(new URL(entry.storyFile, packageRoot), 'utf8'), entry);
    });

    const entry: DemoCoverage = {
        page: 'demo/example.html', source: 'demo/example.html', storyFile: 'src/lib/example.stories.ts',
        title: 'Example', exportName: 'Sample', legends: ['One', 'Two'],
    };
    const pages = new Map([[entry.page, entry.legends]]);
    const story = "export default {title: 'Example', tags: ['test']}; export const Sample = {render: () => '', play: async () => {}};";

    it('normalizes layout whitespace without changing case or punctuation', () => {
        expect(normalizeLegend('  One\n  & two.  ')).toBe('One & two.');
    });

    it.each([
        ['missing page', new Map([...pages, ['demo/new.html', []]]), [entry], /missing contracts.*new.html/],
        ['removed page', new Map(), [entry], /stale contracts.*example.html/],
        ['missing legend', new Map([[entry.page, ['One', 'Two', 'Three']]]), [entry], /missing contracts.*Three/],
        ['removed legend', new Map([[entry.page, ['One']]]), [entry], /stale contracts.*Two/],
        ['duplicate page', pages, [entry, entry], /duplicate contract/],
        ['duplicate legend', new Map([[entry.page, ['One', 'One']]]), [entry], /duplicate actual/],
        ['blank legend', new Map([[entry.page, ['']]]), [entry], /blank/],
        ['reordered legends', new Map([[entry.page, ['Two', 'One']]]), [entry], /order differs/],
        ['wrong source', pages, [{ ...entry, source: 'demo/copied.html' }], /must load the authored page/],
        ['duplicate owner', new Map([...pages, ['demo/other.html', entry.legends]]),
            [entry, { ...entry, page: 'demo/other.html', source: 'demo/other.html' }], /duplicate actual/],
    ] as const)('rejects %s', (_label, actual, contracts, diagnostic) => {
        expect(() => assertAuthoredInventory(actual, contracts)).toThrow(diagnostic);
    });

    it.each([
        ['removed export', story.replace('export const Sample', 'const Sample'), /missing story export/],
        ['renamed title', story.replace("title: 'Example'", "title: 'Renamed'"), /changed story title/],
        ['missing render', story.replace("render: () => '',", ''), /missing render/],
        ['missing play', story.replace('play: async () => {}', ''), /missing asynchronous play/],
        ['synchronous play', story.replace('async ', ''), /missing asynchronous play/],
        ['excluded test', story.replace("['test']", "['!test']"), /excluded/],
        ['renamed story', story.replace('render:', "name: 'Renamed', render:"), /must use its export name/],
        ['changed ID', story.replace("title: 'Example'", "id: 'renamed', title: 'Example'"), /changed story title/],
    ])('rejects a %s', (_label, source, diagnostic) => {
        expect(() => assertStoryContract(source as string, entry)).toThrow(diagnostic);
    });
});
