export const fixtureDirectory = '/cem-ml-all-operations';

export const fixtureFiles = Object.freeze({
    'input.cem': `@doc cem-ml 1
@ns cem = "https://cem.dev/ns/core/1"
@ns html = "http://www.w3.org/1999/xhtml"
@default html

{main @cem:screen="fixture" | {h1 | Fixture}}
`,
    'data.xml': '<catalog><item id="one"/><item id="two"/></catalog>\n',
    'template.xpath': '/catalog/item\n',
    'graph.cem': `{run |
  {import @id=data @src="data.xml" @content-type="application/xml" |
    {transform @id=items @src="template.xpath" @template-content-type="application/vnd.cem.xpath" @template-schema="https://cem.dev/ns/query/xpath/1" |
      {export @id=result @out="graph-output.json" @content-type="application/vnd.cem.xpath-result+json" @schema="https://cem.dev/ns/query/xpath/1"}
    }
  }
}
`,
});

export const commandCases = Object.freeze([
    {
        name: 'parse',
        operation: 'parse',
        argv: ['parse', 'input.cem', '--format', 'ast-json', '--preserve-source-offsets'],
    },
    {
        name: 'validate',
        operation: 'validate',
        argv: ['validate', 'input.cem', '--format', 'json', '--report-json', 'validate-report.json'],
    },
    {
        name: 'check',
        operation: 'check',
        argv: ['check', 'input.cem', '--format', 'json', '--zero-hard-violations'],
    },
    {
        name: 'inspect',
        operation: 'inspect',
        argv: ['inspect', 'input.cem', '--show', 'summary', '--format', 'json'],
    },
    {
        name: 'convert',
        operation: 'convert',
        argv: ['convert', 'input.cem', '--to-format', 'dom-json', '--preserve-source-offsets'],
    },
    {
        name: 'query',
        operation: 'query',
        argv: [
            'query',
            'data.xml',
            '--query',
            '//item',
            '--query-content-type',
            'application/vnd.cem.xpath',
            '--output',
            'json',
        ],
    },
    {
        name: 'transform-direct',
        operation: 'transform',
        sourceKind: 'direct',
        argv: [
            'transform',
            'data.xml',
            '--data-content-type',
            'application/xml',
            '--template',
            'template.xpath',
            '--template-content-type',
            'application/vnd.cem.xpath',
            '--to-content-type',
            'application/vnd.cem.xpath-result+json',
            '--to-schema',
            'https://cem.dev/ns/query/xpath/1',
        ],
    },
    {
        name: 'transform-graph',
        operation: 'transform',
        sourceKind: 'graph',
        argv: ['transform', '--config', 'graph.cem'],
    },
    {
        name: 'trace',
        operation: 'trace',
        argv: ['trace', 'input.cem', '--format', 'json'],
    },
    {
        name: 'version-capabilities',
        operation: 'version-capabilities',
        argv: ['version'],
    },
]);

export const portableOperationKinds = Object.freeze([
    'parse',
    'validate',
    'check',
    'inspect',
    'convert',
    'query',
    'transform',
    'trace',
    'version-capabilities',
]);
