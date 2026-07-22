import type { Meta, StoryObj } from '@storybook/web-components-vite';

import {
    cemQlNode,
    cemQlStream,
    evaluateCemQlQuery,
    type CemQlQueryBindings,
    type CemQlQueryItem,
} from './internal/runtime-support/cem-ql-query.js';

const meta: Meta = {
    title: 'CEM Elements/CEM-QL Rust-First Parity',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

interface ParityRow {
    id: string;
    sourceCategory: string;
    query: string;
    bindings: CemQlQueryBindings;
    expectedItems: CemQlQueryItem[];
    expectedDiagnosticCodes: string[];
    expectedErrorKind?: string;
}

const rows: ParityRow[] = [
    {
        id: 'comparison-eq',
        sourceCategory: 'operator/comparison',
        query: '1 == 1',
        bindings: {},
        expectedItems: [boolean(true)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'comparison-ne',
        sourceCategory: 'operator/comparison',
        query: '1 != 2',
        bindings: {},
        expectedItems: [boolean(true)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'comparison-lt',
        sourceCategory: 'operator/comparison',
        query: '1 < 2',
        bindings: {},
        expectedItems: [boolean(true)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'comparison-le',
        sourceCategory: 'operator/comparison',
        query: '2 <= 2',
        bindings: {},
        expectedItems: [boolean(true)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'comparison-gt',
        sourceCategory: 'operator/comparison',
        query: '3 > 2',
        bindings: {},
        expectedItems: [boolean(true)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'comparison-ge',
        sourceCategory: 'operator/comparison',
        query: '3 >= 3',
        bindings: {},
        expectedItems: [boolean(true)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'arithmetic-add',
        sourceCategory: 'operator/arithmetic',
        query: '1 + 2',
        bindings: {},
        expectedItems: [integer(3)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'arithmetic-subtract',
        sourceCategory: 'operator/arithmetic',
        query: '5 - 2',
        bindings: {},
        expectedItems: [integer(3)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'arithmetic-multiply',
        sourceCategory: 'operator/arithmetic',
        query: '2 * 3',
        bindings: {},
        expectedItems: [integer(6)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'arithmetic-divide',
        sourceCategory: 'operator/arithmetic',
        query: '5 / 2',
        bindings: {},
        expectedItems: [integer(2)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'arithmetic-remainder',
        sourceCategory: 'operator/arithmetic',
        query: '5 % 2',
        bindings: {},
        expectedItems: [integer(1)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'arithmetic-unary-minus',
        sourceCategory: 'operator/arithmetic',
        query: '-(5)',
        bindings: {},
        expectedItems: [integer(-5)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'arithmetic-precedence',
        sourceCategory: 'operator/arithmetic',
        query: '1 + 2 * 3',
        bindings: {},
        expectedItems: [integer(7)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'json-bindings',
        sourceCategory: 'runtime/bindings',
        query: 'left + right',
        bindings: { left: 2, right: 3 },
        expectedItems: [integer(5)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'boolean-and',
        sourceCategory: 'operator/boolean',
        query: 'true && false',
        bindings: {},
        expectedItems: [boolean(false)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'boolean-or',
        sourceCategory: 'operator/boolean',
        query: 'false || true',
        bindings: {},
        expectedItems: [boolean(true)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'boolean-not',
        sourceCategory: 'operator/boolean',
        query: '!false',
        bindings: {},
        expectedItems: [boolean(true)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'coalesce-null',
        sourceCategory: 'operator/coalesce',
        query: 'null ?? "fallback"',
        bindings: {},
        expectedItems: [string('fallback')],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'stream-union',
        sourceCategory: 'operator/set',
        query: 'left | right',
        bindings: {
            left: cemQlStream([1, 2]),
            right: cemQlStream([2, 3]),
        },
        expectedItems: [integer(1), integer(2), integer(3)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'stream-intersect',
        sourceCategory: 'operator/set',
        query: 'left & right',
        bindings: {
            left: cemQlStream([1, 2, 3]),
            right: cemQlStream([2, 3, 4]),
        },
        expectedItems: [integer(2), integer(3)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'stream-difference',
        sourceCategory: 'operator/set',
        query: 'left - right',
        bindings: {
            left: cemQlStream([1, 2, 3]),
            right: cemQlStream([2, 4]),
        },
        expectedItems: [integer(1), integer(3)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'stream-symmetric-difference',
        sourceCategory: 'operator/set',
        query: 'left ^ right',
        bindings: {
            left: cemQlStream([1, 2, 3]),
            right: cemQlStream([2, 4]),
        },
        expectedItems: [integer(1), integer(3), integer(4)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'dot-pipeline-lambda',
        sourceCategory: 'operator/pipeline-current',
        query: '(1, 2, 3).{. + 1}',
        bindings: {},
        expectedItems: [integer(2), integer(3), integer(4)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'leading-dot-current-item',
        sourceCategory: 'operator/pipeline-current',
        query: '(41).{.}',
        bindings: {},
        expectedItems: [integer(41)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'json-array-record-navigation',
        sourceCategory: 'data/boundary-array',
        query: 'rows.name',
        bindings: {
            rows: [{ name: 'Ada' }, { name: 'Lin' }],
        },
        expectedItems: [string('Ada'), string('Lin')],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'type-test-is',
        sourceCategory: 'operator/type',
        query: 'first is node',
        bindings: {
            first: cemQlNode('node-1'),
        },
        expectedItems: [boolean(true)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'cast-as',
        sourceCategory: 'operator/type',
        query: '"42" as integer',
        bindings: {},
        expectedItems: [integer(42)],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'treat-as',
        sourceCategory: 'operator/type',
        query: 'treat_as(first, node)',
        bindings: {
            first: cemQlNode('node-1'),
        },
        expectedItems: [node('node-1')],
        expectedDiagnosticCodes: [],
    },
    {
        id: 'same-node',
        sourceCategory: 'operator/node-identity',
        query: 'same_node(first, second)',
        bindings: {
            first: cemQlNode('node-1'),
            second: cemQlNode('node-1'),
        },
        expectedItems: [boolean(true)],
        expectedDiagnosticCodes: [],
    },
    ...sequenceFunctionRows(),
    ...stringFunctionRows(),
    ...numberFunctionRows(),
    ...datetimeFunctionRows(),
    ...hostFunctionRows(),
    ...contentTypeFunctionRows(),
    ...unsupportedTierBFunctionRows(),
    {
        id: 'legacy-boolean-diagnostic',
        sourceCategory: 'diagnostic/legacy-syntax',
        query: 'true and false',
        bindings: {},
        expectedItems: [],
        expectedDiagnosticCodes: ['cem.ql.compile_failed'],
        expectedErrorKind: 'compile',
    },
];

function sequenceFunctionRows(): ParityRow[] {
    return [
        {
            id: 'function-sequence-map',
            sourceCategory: 'function/sequence',
            query: 'seq:map((1, 2), fn(x) => x + 1)',
            bindings: {},
            expectedItems: [integer(2), integer(3)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-where',
            sourceCategory: 'function/sequence',
            query: 'seq:where((1, 2, 3), fn(x) => x > 1)',
            bindings: {},
            expectedItems: [integer(2), integer(3)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-flat-map',
            sourceCategory: 'function/sequence',
            query: 'seq:flat_map((1, 2), fn(x) => (x, x + 10))',
            bindings: {},
            expectedItems: [integer(1), integer(11), integer(2), integer(12)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-take',
            sourceCategory: 'function/sequence',
            query: 'seq:take((1, 2, 3), 2)',
            bindings: {},
            expectedItems: [integer(1), integer(2)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-drop',
            sourceCategory: 'function/sequence',
            query: 'seq:drop((1, 2, 3), 1)',
            bindings: {},
            expectedItems: [integer(2), integer(3)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-first',
            sourceCategory: 'function/sequence',
            query: 'seq:first((1, 2, 3))',
            bindings: {},
            expectedItems: [integer(1)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-last',
            sourceCategory: 'function/sequence',
            query: 'seq:last((1, 2, 3))',
            bindings: {},
            expectedItems: [integer(3)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-nth',
            sourceCategory: 'function/sequence',
            query: 'seq:nth((1, 2, 3), 2)',
            bindings: {},
            expectedItems: [integer(2)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-peek',
            sourceCategory: 'function/sequence',
            query: 'seq:peek((1, 2), fn(x) => x + 1)',
            bindings: {},
            expectedItems: [integer(1), integer(2)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-union',
            sourceCategory: 'function/sequence',
            query: 'seq:union((1, 2), (2, 3))',
            bindings: {},
            expectedItems: [integer(1), integer(2), integer(3)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-intersect',
            sourceCategory: 'function/sequence',
            query: 'seq:intersect((1, 2, 3), (2, 4))',
            bindings: {},
            expectedItems: [integer(2)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-difference',
            sourceCategory: 'function/sequence',
            query: 'seq:difference((1, 2, 3), (2, 4))',
            bindings: {},
            expectedItems: [integer(1), integer(3)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-symmetric-difference',
            sourceCategory: 'function/sequence',
            query: 'seq:symmetric_difference((1, 2, 3), (2, 4))',
            bindings: {},
            expectedItems: [integer(1), integer(3), integer(4)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-count',
            sourceCategory: 'function/sequence',
            query: 'seq:count(("a", "b", "c"))',
            bindings: {},
            expectedItems: [integer(3)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-any',
            sourceCategory: 'function/sequence',
            query: 'any((1, 2, 3), fn(x) => x == 2)',
            bindings: {},
            expectedItems: [boolean(true)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-sequence-all',
            sourceCategory: 'function/sequence',
            query: 'all((1, 2, 3), fn(x) => x < 10)',
            bindings: {},
            expectedItems: [boolean(true)],
            expectedDiagnosticCodes: [],
        },
    ];
}

function stringFunctionRows(): ParityRow[] {
    return [
        {
            id: 'function-string-length',
            sourceCategory: 'function/string',
            query: 'str:length("CEM")',
            bindings: {},
            expectedItems: [integer(3)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-string-codepoints',
            sourceCategory: 'function/string',
            query: 'str:codepoints("AZ")',
            bindings: {},
            expectedItems: [integer(65), integer(90)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-string-lower',
            sourceCategory: 'function/string',
            query: 'str:lower("CEM")',
            bindings: {},
            expectedItems: [string('cem')],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-string-upper',
            sourceCategory: 'function/string',
            query: 'str:upper("cem")',
            bindings: {},
            expectedItems: [string('CEM')],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-string-slice',
            sourceCategory: 'function/string',
            query: 'str:slice("abcdef", 2, 3)',
            bindings: {},
            expectedItems: [string('cde')],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-string-concat',
            sourceCategory: 'function/string',
            query: 'str:concat(("a", "b", "c"), "-")',
            bindings: {},
            expectedItems: [string('a-b-c')],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-string-contains',
            sourceCategory: 'function/string',
            query: 'str:contains("semantic", "man")',
            bindings: {},
            expectedItems: [boolean(true)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-string-starts-with',
            sourceCategory: 'function/string',
            query: 'str:starts_with("semantic", "sem")',
            bindings: {},
            expectedItems: [boolean(true)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-string-ends-with',
            sourceCategory: 'function/string',
            query: 'str:ends_with("semantic", "tic")',
            bindings: {},
            expectedItems: [boolean(true)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-string-normalize-space',
            sourceCategory: 'function/string',
            query: 'str:normalize_space("  --cem-gap    0.5rem  ")',
            bindings: {},
            expectedItems: [string('--cem-gap 0.5rem')],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-string-replace',
            sourceCategory: 'function/string',
            query: 'str:replace("tone-[state]", "[state]", "active")',
            bindings: {},
            expectedItems: [string('tone-active')],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-string-translate',
            sourceCategory: 'function/string',
            query: 'str:translate("Cem-ML", "ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz")',
            bindings: {},
            expectedItems: [string('cem-ml')],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-string-substring',
            sourceCategory: 'function/string',
            query: 'str:substring("semantic", 3, 4)',
            bindings: {},
            expectedItems: [string('mant')],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-string-substring-before',
            sourceCategory: 'function/string',
            query: 'str:substring_before("fa-github", "-")',
            bindings: {},
            expectedItems: [string('fa')],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-string-substring-after',
            sourceCategory: 'function/string',
            query: 'str:substring_after("fa-github", "-")',
            bindings: {},
            expectedItems: [string('github')],
            expectedDiagnosticCodes: [],
        },
    ];
}

function numberFunctionRows(): ParityRow[] {
    return [
        {
            id: 'function-number-double',
            sourceCategory: 'function/number',
            query: 'num:double("1.5")',
            bindings: {},
            expectedItems: [double(1.5)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-number-decimal',
            sourceCategory: 'function/number',
            query: 'num:decimal("1.5")',
            bindings: {},
            expectedItems: [decimal('1.5')],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-number-integer',
            sourceCategory: 'function/number',
            query: 'num:integer("12.9")',
            bindings: {},
            expectedItems: [integer(12)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-number-string',
            sourceCategory: 'function/number',
            query: 'num:string(12)',
            bindings: {},
            expectedItems: [string('12')],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-number-abs',
            sourceCategory: 'function/number',
            query: 'num:abs(-3)',
            bindings: {},
            expectedItems: [integer(3)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-number-floor',
            sourceCategory: 'function/number',
            query: 'num:floor(3.6)',
            bindings: {},
            expectedItems: [integer(3)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-number-ceil',
            sourceCategory: 'function/number',
            query: 'num:ceil(3.2)',
            bindings: {},
            expectedItems: [integer(4)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-number-round',
            sourceCategory: 'function/number',
            query: 'num:round(3.6)',
            bindings: {},
            expectedItems: [integer(4)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-number-format',
            sourceCategory: 'function/number',
            query: 'num:format(12, "value={}")',
            bindings: {},
            expectedItems: [string('value=12')],
            expectedDiagnosticCodes: [],
        },
    ];
}

function datetimeFunctionRows(): ParityRow[] {
    return [
        {
            id: 'function-datetime-to-utc',
            sourceCategory: 'function/datetime',
            query: 'dt:to_utc("2026-05-23T01:02:03")',
            bindings: {},
            expectedItems: [string('2026-05-23T01:02:03Z')],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-datetime-components',
            sourceCategory: 'function/datetime',
            query: 'dt:components("2026-05-23T01:02:03Z")',
            bindings: {},
            expectedItems: [datetimeComponents()],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-datetime-format',
            sourceCategory: 'function/datetime',
            query: 'dt:format("2026-05-23T01:02:03Z", "ignored")',
            bindings: {},
            expectedItems: [string('2026-05-23T01:02:03Z')],
            expectedDiagnosticCodes: [],
        },
    ];
}

function hostFunctionRows(): ParityRow[] {
    return [
        {
            id: 'function-dom-children',
            sourceCategory: 'function/host',
            query: 'dom:children(cemml:parse("{main | {p | Hi}}"))',
            bindings: {},
            expectedItems: [],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-dom-descendants',
            sourceCategory: 'function/host',
            query: 'dom:descendants(cemml:parse("{main | {p | Hi}}"))',
            bindings: {},
            expectedItems: [],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-dom-parent',
            sourceCategory: 'function/host',
            query: 'dom:parent(cemml:parse("{main | {p | Hi}}"))',
            bindings: {},
            expectedItems: [],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-dom-attribute',
            sourceCategory: 'function/host',
            query: 'dom:attribute(cemml:parse("{input @id=email}"), "id")',
            bindings: {},
            expectedItems: [],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-dom-resolve-ref',
            sourceCategory: 'function/host',
            query:
                'dom:resolve_ref(' +
                'cemml:parse("{main | {h1 @id=title | Title} {section @aria-labelledby=title | Body}}")' +
                ')',
            bindings: {},
            expectedItems: [node('h1#title')],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-dom-tainted',
            sourceCategory: 'function/host',
            query: 'dom:tainted(cemml:parse("{main | Hi}"))',
            bindings: {},
            expectedItems: [boolean(false)],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-report-emit',
            sourceCategory: 'function/host',
            query: 'report:emit("cem.ql.story", "hello", "info")',
            bindings: {},
            expectedItems: [],
            expectedDiagnosticCodes: evalDiagnosticCodes('cem.ql.story'),
        },
        {
            id: 'function-report-severity-floor',
            sourceCategory: 'function/host',
            query: 'report:severity_floor("warning")',
            bindings: {},
            expectedItems: [],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-state-read',
            sourceCategory: 'function/host',
            query: 'state:read("active")',
            bindings: {},
            expectedItems: [],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-state-keys',
            sourceCategory: 'function/host',
            query: 'state:keys()',
            bindings: {},
            expectedItems: [],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-template-lookup',
            sourceCategory: 'function/host',
            query: 'tpl:lookup("button")',
            bindings: {},
            expectedItems: [],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-template-names',
            sourceCategory: 'function/host',
            query: 'tpl:names()',
            bindings: {},
            expectedItems: [],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-cemml-parse',
            sourceCategory: 'function/host',
            query: 'cemml:parse("{p | Hi}")',
            bindings: {},
            expectedItems: [node('{p | Hi}\n')],
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-cemml-format',
            sourceCategory: 'function/host',
            query: 'cemml:format("{p | Hi}\\n")',
            bindings: {},
            expectedItems: [string('{p | Hi}\n')],
            expectedDiagnosticCodes: [],
        },
    ];
}

function contentTypeFunctionRows(): ParityRow[] {
    return [
        {
            id: 'function-content-type-read',
            sourceCategory: 'function/content-type',
            query: 'ct:read("file:///tmp/example.json", ct:json())',
            bindings: {},
            expectedItems: [node('application/json:example.json')],
            expectedDiagnosticCodes: [],
        },
        ...contentTypeConstants().map(([name, mediaType]) => ({
            id: `function-content-type-${name}`,
            sourceCategory: 'function/content-type',
            query: `ct:${name}()`,
            bindings: {},
            expectedItems: [string(mediaType)],
            expectedDiagnosticCodes: [],
        })),
        {
            id: 'function-content-type-floor',
            sourceCategory: 'function/content-type',
            query: 'ct:floor()',
            bindings: {},
            expectedItems: contentTypeFloor(),
            expectedDiagnosticCodes: [],
        },
        {
            id: 'function-content-type-default-accepts',
            sourceCategory: 'function/content-type',
            query: 'ct:default_accepts()',
            bindings: {},
            expectedItems: contentTypeFloor(),
            expectedDiagnosticCodes: [],
        },
    ];
}

function unsupportedTierBFunctionRows(): ParityRow[] {
    const sequenceHelpers = [
        'unique',
        'distinct_by',
        'flatten',
        'zip',
        'enumerate',
        'chunked',
        'windowed',
        'sliding',
        'group_by',
        'count_by',
        'partition',
        'take_while',
        'drop_while',
        'sorted',
        'reversed',
        'reduce',
        'fold',
        'scan',
        'none',
        'min',
        'max',
        'sum',
        'avg',
    ];
    const stringHelpers = ['nfc', 'nfd', 'matches', 'split'];

    return [
        ...sequenceHelpers.map((name) =>
            unsupportedFunctionRow(`tier-b-sequence-${name}`, `seq:${name}((1, 2, 3))`)
        ),
        ...stringHelpers.map((name) =>
            unsupportedFunctionRow(`tier-b-string-${name}`, `str:${name}("Cem")`)
        ),
    ];
}

function unsupportedFunctionRow(id: string, query: string): ParityRow {
    return {
        id: `function-${id}`,
        sourceCategory: 'function/pending-tier-b',
        query,
        bindings: {},
        expectedItems: [],
        expectedDiagnosticCodes: evalDiagnosticCodes('cem.ql.unknown_function'),
        expectedErrorKind: 'eval',
    };
}

export const RustFirstEvaluationTable: Story = {
    render: () => renderParityTable(rows),
    play: async ({ canvasElement }) => {
        for (const row of rows) {
            const result = await evaluateCemQlQuery(row.query, row.bindings);
            const renderedRow = requiredRow(canvasElement, row.id);
            setCellText(renderedRow, 'items', pretty(result.items));
            setCellText(renderedRow, 'diagnostics', pretty(result.diagnostics));

            assertDeepEqual(result.items, row.expectedItems, `${row.id} items`);
            assertDeepEqual(
                result.diagnostics.map((diagnostic) => diagnostic.code),
                row.expectedDiagnosticCodes,
                `${row.id} diagnostic codes`
            );
            assertEqual(
                result.error == null ? undefined : result.error.kind,
                row.expectedErrorKind,
                `${row.id} error`
            );
        }
    },
};

function renderParityTable(parityRows: readonly ParityRow[]): HTMLElement {
    const root = document.createElement('section');
    root.className = 'cem-ql-parity';
    root.append(styleElement());

    const table = document.createElement('table');
    const thead = document.createElement('thead');
    const header = document.createElement('tr');
    for (const label of ['Source category', 'Query source', 'Input bindings', 'Output items', 'Diagnostics']) {
        const th = document.createElement('th');
        th.textContent = label;
        header.append(th);
    }
    thead.append(header);
    table.append(thead);

    const tbody = document.createElement('tbody');
    for (const row of parityRows) {
        const tr = document.createElement('tr');
        tr.dataset.rowId = row.id;
        appendTextCell(tr, row.sourceCategory);
        appendPreCell(tr, row.query);
        appendPreCell(tr, pretty(row.bindings));
        appendPreCell(tr, pretty(row.expectedItems), 'items');
        appendPreCell(
            tr,
            row.expectedDiagnosticCodes.length === 0 ? '[]' : pretty(row.expectedDiagnosticCodes),
            'diagnostics'
        );
        tbody.append(tr);
    }
    table.append(tbody);
    root.append(table);
    return root;
}

function styleElement(): HTMLStyleElement {
    const style = document.createElement('style');
    style.textContent = `
        .cem-ql-parity {
            color: #17202a;
            font: 13px/1.45 ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
            padding: 16px;
        }
        .cem-ql-parity table {
            border-collapse: collapse;
            inline-size: 100%;
            table-layout: fixed;
        }
        .cem-ql-parity th,
        .cem-ql-parity td {
            border-block-end: 1px solid #d0d7de;
            padding: 8px;
            text-align: start;
            vertical-align: top;
        }
        .cem-ql-parity th {
            background: #f6f8fa;
            color: #24292f;
            font-weight: 700;
        }
        .cem-ql-parity td:first-child {
            inline-size: 15%;
            font-weight: 600;
        }
        .cem-ql-parity pre {
            margin: 0;
            overflow-x: auto;
            white-space: pre-wrap;
            word-break: break-word;
            font: 12px/1.45 ui-monospace, SFMono-Regular, Consolas, "Liberation Mono", monospace;
        }
    `;
    return style;
}

function appendTextCell(row: HTMLTableRowElement, value: string): void {
    const cell = document.createElement('td');
    cell.textContent = value;
    row.append(cell);
}

function appendPreCell(row: HTMLTableRowElement, value: string, role?: string): void {
    const cell = document.createElement('td');
    if (role) {
        cell.dataset.cell = role;
    }
    const pre = document.createElement('pre');
    pre.textContent = value;
    cell.append(pre);
    row.append(cell);
}

function requiredRow(root: HTMLElement, id: string): HTMLTableRowElement {
    const row = root.querySelector<HTMLTableRowElement>(`tr[data-row-id="${id}"]`);
    assert(row, `expected rendered row ${id}`);
    return row;
}

function setCellText(row: HTMLTableRowElement, role: string, value: string): void {
    const cell = row.querySelector<HTMLTableCellElement>(`td[data-cell="${role}"] pre`);
    assert(cell, `expected ${role} cell for row ${row.dataset.rowId ?? 'unknown'}`);
    cell.textContent = value;
}

function integer(value: number): CemQlQueryItem {
    return { kind: 'atomic', type: 'integer', value };
}

function boolean(value: boolean): CemQlQueryItem {
    return { kind: 'atomic', type: 'boolean', value };
}

function string(value: string): CemQlQueryItem {
    return { kind: 'atomic', type: 'string', value };
}

function decimal(value: string): CemQlQueryItem {
    return { kind: 'atomic', type: 'decimal', value };
}

function double(value: number): CemQlQueryItem {
    return { kind: 'atomic', type: 'double', value };
}

function node(id: string): CemQlQueryItem {
    return { kind: 'node', id };
}

function record(fields: Record<string, CemQlQueryItem[]>): CemQlQueryItem {
    return { kind: 'record', fields };
}

function datetimeComponents(): CemQlQueryItem {
    return record({
        day: [integer(23)],
        hour: [integer(1)],
        minute: [integer(2)],
        month: [integer(5)],
        second: [integer(3)],
        tz: [string('Z')],
        year: [integer(2026)],
    });
}

function contentTypeConstants(): [string, string][] {
    return [
        ['html', 'text/html'],
        ['xml', 'application/xml'],
        ['svg', 'image/svg+xml'],
        ['mathml', 'application/mathml+xml'],
        ['css', 'text/css'],
        ['scss', 'text/x-scss'],
        ['json', 'application/json'],
        ['yaml', 'application/yaml'],
        ['csv', 'text/csv'],
        ['js', 'application/javascript'],
        ['ts', 'application/typescript'],
        ['cemml', 'application/cem+xml'],
    ];
}

function contentTypeFloor(): CemQlQueryItem[] {
    return contentTypeConstants().map(([, mediaType]) => string(mediaType));
}

function evalDiagnosticCodes(code: string): string[] {
    return [code, code];
}

function pretty(value: unknown): string {
    return JSON.stringify(value, null, 2);
}

function assert(value: unknown, message: string): asserts value {
    if (!value) {
        throw new Error(message);
    }
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
    if (actual !== expected) {
        throw new Error(`${label}: expected ${String(expected)}, got ${String(actual)}`);
    }
}

function assertDeepEqual(actual: unknown, expected: unknown, label: string): void {
    const actualJson = JSON.stringify(actual);
    const expectedJson = JSON.stringify(expected);
    if (actualJson !== expectedJson) {
        throw new Error(`${label}: expected ${expectedJson}, got ${actualJson}`);
    }
}
