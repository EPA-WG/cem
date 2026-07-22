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
        id: 'boolean-or',
        sourceCategory: 'operator/boolean',
        query: 'false || true',
        bindings: {},
        expectedItems: [boolean(true)],
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
        id: 'pipeline-current-item',
        sourceCategory: 'operator/pipeline-current',
        query: '(1, 2, 3).{. + 1}',
        bindings: {},
        expectedItems: [integer(2), integer(3), integer(4)],
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
