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
 * The produced rows are a sequence of records keyed `td1`, `td2`, … (1-based, matching the legacy
 * `td[n]`). A converted generator feeds them through the substrate `datadom.slices.<name>` surface
 * and reads them with, e.g.,
 * `{cem:for-each @select="$datadom.slices.minimums" @as="row" | {$row.td1}: {$row.td2};}`.
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
 * Project a `<table>`'s body rows into records keyed `td1`, `td2`, … (1-based, matching the legacy
 * `normalize-space(xhtml:td[n])`); each cell's text is whitespace-normalized. Cells count in
 * column order whether `<td>` or `<th>`, so the indices stay stable.
 */
export function tableToRows(table: Element): Array<Record<string, string>> {
    const body = table.querySelector('tbody') ?? table;
    const rows: Array<Record<string, string>> = [];
    for (const tr of Array.from(body.children)) {
        if (tr.localName !== 'tr') {
            continue;
        }
        const row: Record<string, string> = {};
        let column = 0;
        for (const cell of Array.from(tr.children)) {
            if (cell.localName === 'td' || cell.localName === 'th') {
                column += 1;
                row[`td${column}`] = normalizeSpace(cell.textContent ?? '');
            }
        }
        rows.push(row);
    }
    return rows;
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
export function tokenTableRows(root: ParentNode, id: string): Array<Record<string, string>> {
    const table = followingTable(root, id);
    return table ? tableToRows(table) : [];
}
