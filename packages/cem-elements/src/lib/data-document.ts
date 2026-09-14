/**
 * Browser-side DOM → cem-ql data-document bridge (BR-PH-1; slice 3 of the cem-theme CSS-generator
 * conversion). The generators run in the browser, so the token document is already a parsed DOM
 * (`http-request.js` feeds `DOMParser` output — BR-PH-3). This shapes that DOM into plain records
 * the CEM-ML template navigates functionally — Record field access + `cem:for-each` +
 * `str:normalize_space` — replacing the legacy XSLT
 * `*[@id='…']/following-sibling::table[1]/tbody/tr` + `normalize-space(td[n])`. No XHTML parser is
 * involved: native DOM queries do the navigation, which is why cem-ql's deliberately-unwired XPath
 * axes are a non-issue.
 *
 * The produced rows are portable records keyed by normalized table headings, plus ordered `cells`,
 * source provenance, and compatible `td1`, `td2`, … aliases. A converted generator feeds them
 * through the substrate `datadom.slices.<name>` surface and reads them with, e.g.,
 * `{cem:for-each @select="$datadom.slices.minimums" @as="row" | {$row.token}: {$row.value};}`.
 */

/** XSLT / `str:normalize_space` parity: trim and collapse internal whitespace runs to single spaces. */
export function normalizeSpace(value: string): string {
    return value
        .split(/\s+/)
        .filter((part) => part.length > 0)
        .join(' ');
}

/** A DOM element projected into a cem-ql-navigable record. */
export interface DomDataNode {
    tag: string;
    attributes: Record<string, string>;
    /** This element's whitespace-normalized text content. */
    text: string;
    children: DomDataNode[];
}

export type MarkdownTableRowValue = string | string[] | number;

/** JSON-compatible row shape shared by browser rendering and SSR data inputs. */
export interface MarkdownTableRow extends Record<string, MarkdownTableRowValue> {
    cells: string[];
    source_table: string;
    source_row: number;
}

export interface MarkdownTableProjectionDiagnostic {
    code:
        | 'cem.markdown.table_header_empty'
        | 'cem.markdown.table_header_duplicate'
        | 'cem.markdown.table_header_reserved';
    message: string;
    sourceTable: string;
    column: number;
    heading: string;
    normalizedHeading: string;
}

export interface MarkdownTableProjection {
    rows: MarkdownTableRow[];
    diagnostics: MarkdownTableProjectionDiagnostic[];
}

/** Walk an already-parsed DOM element into a {@link DomDataNode} tree. */
export function domToRecord(element: Element): DomDataNode {
    const attributes: Record<string, string> = {};
    for (const attribute of Array.from(element.attributes)) {
        attributes[attribute.name] = attribute.value;
    }
    return {
        tag: element.localName,
        attributes,
        text: normalizeSpace(element.textContent ?? ''),
        children: Array.from(element.children).map(domToRecord),
    };
}

/**
 * Deterministically normalize a Markdown table heading for record-field access. The portable
 * spelling is lower snake_case; punctuation and whitespace collapse to one underscore.
 */
export function normalizeTableHeading(value: string): string {
    return normalizeSpace(value)
        .toLocaleLowerCase('en-US')
        .replace(/[^a-z0-9]+/g, '_')
        .replace(/^_+|_+$/g, '');
}

/**
 * Project a `<table>` into the portable Markdown row contract. Named fields are derived from the
 * first header row. Duplicate, empty, or reserved normalized headings are diagnosed and remain
 * available via `cells` and `tdN`; they never overwrite another named field.
 */
export function tableToProjection(table: Element, sourceTable = table.id): MarkdownTableProjection {
    const diagnostics: MarkdownTableProjectionDiagnostic[] = [];
    const headerRow = table.querySelector('thead tr');
    const headings = headerRow
        ? Array.from(headerRow.children)
            .filter((cell) => cell.localName === 'td' || cell.localName === 'th')
            .map((cell) => normalizeSpace(cell.textContent ?? ''))
        : [];
    const fields: Array<string | null> = [];
    const used = new Set<string>();
    for (const [offset, heading] of headings.entries()) {
        const normalizedHeading = normalizeTableHeading(heading);
        if (!normalizedHeading) {
            diagnostics.push({
                code: 'cem.markdown.table_header_empty',
                message: `Markdown table \`${sourceTable}\` column ${offset + 1} has an empty normalized heading`,
                sourceTable,
                column: offset + 1,
                heading,
                normalizedHeading,
            });
            fields.push(null);
            continue;
        }
        if (
            normalizedHeading === 'cells'
            || normalizedHeading === 'source_table'
            || normalizedHeading === 'source_row'
            || /^td[1-9][0-9]*$/.test(normalizedHeading)
        ) {
            diagnostics.push({
                code: 'cem.markdown.table_header_reserved',
                message: `Markdown table \`${sourceTable}\` column ${offset + 1} uses reserved normalized heading \`${normalizedHeading}\``,
                sourceTable,
                column: offset + 1,
                heading,
                normalizedHeading,
            });
            fields.push(null);
            continue;
        }
        if (used.has(normalizedHeading)) {
            diagnostics.push({
                code: 'cem.markdown.table_header_duplicate',
                message: `Markdown table \`${sourceTable}\` column ${offset + 1} duplicates normalized heading \`${normalizedHeading}\``,
                sourceTable,
                column: offset + 1,
                heading,
                normalizedHeading,
            });
            fields.push(null);
            continue;
        }
        used.add(normalizedHeading);
        fields.push(normalizedHeading);
    }

    const body = table.querySelector('tbody') ?? table;
    const rows: MarkdownTableRow[] = [];
    for (const tr of Array.from(body.children)) {
        if (tr.localName !== 'tr') {
            continue;
        }
        const cells = Array.from(tr.children)
            .filter((cell) => cell.localName === 'td' || cell.localName === 'th')
            .map((cell) => normalizeSpace(cell.textContent ?? ''));
        const row: MarkdownTableRow = {
            cells,
            source_table: sourceTable,
            source_row: rows.length + 1,
        };
        for (const [offset, value] of cells.entries()) {
            row[`td${offset + 1}`] = value;
            const field = fields[offset];
            if (field) {
                row[field] = value;
            }
        }
        rows.push(row);
    }
    return { rows, diagnostics };
}

/** Compatibility row-only projection; prefer {@link tableToProjection} when diagnostics matter. */
export function tableToRows(table: Element): MarkdownTableRow[] {
    return tableToProjection(table).rows;
}

/**
 * Find the first `<table>` following the element with `id` (native DOM, replacing the legacy
 * `*[@id='…']/following-sibling::table[1]`), or `null` when the anchor or a following table is
 * absent.
 */
export function followingTable(root: ParentNode, id: string): Element | null {
    const anchor = root.querySelector(`#${CSS.escape(id)}`);
    let sibling = anchor?.nextElementSibling ?? null;
    while (sibling && sibling.localName !== 'table') {
        sibling = sibling.nextElementSibling;
    }
    return sibling;
}

/** Rows of the token table anchored by the element with `id`; `[]` when not found. */
export function tokenTableProjection(root: ParentNode, id: string): MarkdownTableProjection {
    const table = followingTable(root, id);
    return table ? tableToProjection(table, id) : { rows: [], diagnostics: [] };
}

/** Rows of the token table anchored by the element with `id`; `[]` when not found. */
export function tokenTableRows(root: ParentNode, id: string): MarkdownTableRow[] {
    return tokenTableProjection(root, id).rows;
}
