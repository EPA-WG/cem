#!/usr/bin/env node

import { createReadStream } from 'node:fs';
import { readFile, readdir, stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { dirname, extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const timeout = 45_000;

const htmlDemoElementModule = `
class HtmlDemoElement extends HTMLElement {
    connectedCallback() {
        if (this.__cemDemoMounted) return;
        this.__cemDemoMounted = true;
        const template = Array.from(this.children).find((child) => child.localName === 'template');
        if (!template) return;
        const demo = document.createElement('div');
        demo.setAttribute('slot', 'demo');
        demo.append(template.content.cloneNode(true));
        this.append(demo);
    }
}
customElements.define('cem-demo-element', HtmlDemoElement);
`;

const dataSliceSamples = [
    sampleContract('A1. inline slice initialization, change on event', [
        formState({ inputs: ['0'], outputs: [] }),
        normalizedText('article.demo-card', '+ − 0'),
        clickThenText('button:first-of-type', 'article.demo-card', '1'),
        formState({ inputs: ['1'], outputs: [] }),
        normalizedText('article.demo-card', '+ − 1'),
        clickThenText('button:nth-of-type(2)', 'article.demo-card', '0'),
        formState({ inputs: ['0'], outputs: [] }),
        normalizedText('article.demo-card', '+ − 0'),
        fillBlurThenText('input', '5', 'article.demo-card', '5'),
        formState({ inputs: ['5'], outputs: [] }),
        normalizedText('article.demo-card', '+ − 5'),
    ]),
    sampleContract('A2. slice initialization, change on event', [
        formState({ inputs: ['0'], outputs: ['0'] }),
        clickThenText('button:first-of-type', 'output', '1'),
        formState({ inputs: ['1'], outputs: ['1'] }),
        dispatchThenText('button:first-of-type', 'tap', 'output', '2'),
        formState({ inputs: ['2'], outputs: ['2'] }),
        clickThenText('button:nth-of-type(2)', 'output', '1'),
        formState({ inputs: ['1'], outputs: ['1'] }),
    ]),
    sampleContract('B. slice event data.', [
        formState({ outputs: ['', '', ''] }),
        propertyEquals('textarea', 'value', ''),
        mouseThenText('textarea', { x: 42, y: 17 }, 'p:nth-of-type(2) output', 'mousemove'),
        normalizedText('p:nth-of-type(3) output', '17'),
        text('p:first-of-type output', 'x:'),
        attributeContains('textarea', 'style', '42px 17px'),
        computedStyleNot('textarea', 'boxShadow', 'none'),
        computedStyleNot('textarea', 'boxShadow', ''),
        dispatchThenText('textarea', 'click', 'p:nth-of-type(2) output', 'click'),
        normalizedText('p:nth-of-type(2) output', 'click'),
    ]),
    sampleContract('1. slice change on event. 1:1 slice⮂value', [
        formState({ inputs: [''], outputs: [''] }),
        fillThenText('input', 'pending', 'output', ''),
        formState({ inputs: ['pending'], outputs: [''] }),
        fillBlurThenText('input', 'basic', 'output', 'basic'),
        formState({ inputs: ['basic'], outputs: ['basic'] }),
        fillBlurThenText('input', '', 'output', ''),
        formState({ inputs: [''], outputs: [''] }),
    ]),
    sampleContract('2. initial slice value, slice change on event. slice⮂value', [
        formState({ inputs: ['B'], outputs: ['B'] }),
        fillThenText('input', 'pending', 'output', 'B'),
        formState({ inputs: ['pending'], outputs: ['B'] }),
        fillBlurThenText('input', 'changed', 'output', 'changed'),
        formState({ inputs: ['changed'], outputs: ['changed'] }),
        fillBlurThenText('input', '', 'output', ''),
        formState({ inputs: [''], outputs: [''] }),
    ]),
    sampleContract('3. on input event. slice⮂value', [
        formState({ inputs: ['B'], outputs: ['B'] }),
        fillThenText('input', 'input event', 'output', 'input event'),
        formState({ inputs: ['input event'], outputs: ['input event'] }),
        fillThenText('input', '', 'output', ''),
        formState({ inputs: [''], outputs: [''] }),
    ]),
    sampleContract('4. initial slice value from attribute', [
        formState({ inputs: ['😁', '🤗'], outputs: ['😁', '', '🤗', ''] }),
        fillDispatchThenText('cem-slice-attribute-initial:first-of-type input', 'qqq', 'keyup', 'cem-slice-attribute-initial:first-of-type p:nth-of-type(2) output', 'qqq'),
        formState({ inputs: ['qqq', '🤗'], outputs: ['😁', 'qqq', '🤗', ''] }),
        fillDispatchThenText('cem-slice-attribute-initial:last-of-type input', 'second', 'keyup', 'cem-slice-attribute-initial:last-of-type p:nth-of-type(2) output', 'second'),
        formState({ inputs: ['qqq', 'second'], outputs: ['😁', 'qqq', '🤗', 'second'] }),
        fillDispatchThenText('cem-slice-attribute-initial:first-of-type input', '', 'keyup', 'cem-slice-attribute-initial:first-of-type p:nth-of-type(2) output', ''),
        formState({ inputs: ['', 'second'], outputs: ['😁', '', '🤗', 'second'] }),
        attributeEquals('cem-slice-attribute-initial:first-of-type', 'a', '😁'),
        attributeEquals('cem-slice-attribute-initial:last-of-type', 'a', '🤗'),
    ]),
    sampleContract('5. slice value computed from event', [
        formState({ inputs: ['B'], outputs: ['xB'] }),
        fillBlurThenText('input', 'C', 'output', 'xC'),
        formState({ inputs: ['C'], outputs: ['xC'] }),
        fillBlurThenText('input', '', 'output', 'x'),
        formState({ inputs: [''], outputs: ['x'] }),
    ]),
    sampleContract('6. button ignored till change on click.', [
        formState({ inputs: ['anonymous'], outputs: ['anonymous'] }),
        clickThenText('button', 'output', 'broccoli'),
        formState({ inputs: ['broccoli'], outputs: ['broccoli'] }),
    ]),
    sampleContract('7. initial slice value from SLICE element', [
        formState({ outputs: ['0'] }),
        clickThenText('button', 'output', '1'),
        formState({ outputs: ['1'] }),
        dispatchThenText('button', 'tap', 'output', '2'),
        formState({ outputs: ['2'] }),
    ]),
    sampleContract('8. multiple slices by SLICE element', [
        formState({ outputs: ['0', '0'] }),
        focusThenText('button', 'p:nth-of-type(2) output', '1'),
        formState({ outputs: ['0', '1'] }),
        clickThenText('button', 'p:first-of-type output', '1'),
        formState({ outputs: ['1', '1'] }),
        dispatchThenText('button', 'tap', 'p:first-of-type output', '2'),
        formState({ outputs: ['2', '1'] }),
        blurThenText('button', 'p:nth-of-type(2) output', '0'),
        formState({ outputs: ['2', '0'] }),
    ]),
    sampleContract('9. slice in attribute', [
        formState({ inputs: [':)', '😃'], outputs: [':)', '😃'] }),
        fillBlurThenText('cem-slice-emotion-attribute:first-of-type input', 'supplied change', 'cem-slice-emotion-attribute:first-of-type output', 'supplied change'),
        formState({ inputs: ['supplied change', '😃'], outputs: ['supplied change', '😃'] }),
        attributeEquals('cem-slice-emotion-attribute:first-of-type', 'emotion', 'supplied change'),
        fillBlurThenText('cem-slice-emotion-attribute:last-of-type input', 'joyful', 'cem-slice-emotion-attribute:last-of-type output', 'joyful'),
        formState({ inputs: ['supplied change', 'joyful'], outputs: ['supplied change', 'joyful'] }),
        fillBlurThenText('cem-slice-emotion-attribute:first-of-type input', '', 'cem-slice-emotion-attribute:first-of-type output', ''),
        formState({ inputs: ['', 'joyful'], outputs: ['', 'joyful'] }),
        attributeEquals('cem-slice-emotion-attribute:first-of-type', 'emotion', ''),
        attributeEquals('cem-slice-emotion-attribute:last-of-type', 'emotion', 'joyful'),
    ]),
    sampleContract('10. multiple slices by same field', [
        formState({ inputs: [''], outputs: ['', ''] }),
        fillThenText('input', 'mirrored', 'p:nth-of-type(2) output', 'mirrored'),
        formState({ inputs: ['mirrored'], outputs: ['mirrored', 'mirrored'] }),
        fillThenText('input', '', 'p:nth-of-type(2) output', ''),
        formState({ inputs: [''], outputs: ['', ''] }),
    ]),
    sampleContract('11. slices and attribute', [
        formState({ inputs: ['😃'], outputs: ['😃', ''] }),
        fillBlurThenText('input', 'grinning', 'p:first-of-type output', 'grinning'),
        formState({ inputs: ['grinning'], outputs: ['grinning', 'grinning'] }),
        attributeEquals('cem-slice-attribute-fanout', 'emotion', 'grinning'),
        fillBlurThenText('input', '', 'p:first-of-type output', ''),
        formState({ inputs: [''], outputs: ['', ''] }),
        attributeEquals('cem-slice-attribute-fanout', 'emotion', ''),
    ]),
    sampleContract('12. checkbox use', [
        formState({ checked: [true, false, false], outputs: ['V0', '', ''] }),
        uncheckThenNormalizedText('label:first-of-type input', 'p:first-of-type output', ''),
        formState({ checked: [false, false, false], outputs: ['', '', ''] }),
        checkThenText('label:first-of-type input', 'p:first-of-type output', 'V0'),
        formState({ checked: [true, false, false], outputs: ['V0', '', ''] }),
        checkThenText('label:nth-of-type(2) input', 'p:nth-of-type(3) output', 'V1'),
        formState({ checked: [true, true, false], outputs: ['V0', 'V1', ''] }),
        checkThenText('label:nth-of-type(3) input', 'p:nth-of-type(4) output', 'V1'),
        formState({ checked: [true, true, true], outputs: ['V0', 'V1', 'V1'] }),
        uncheckThenNormalizedText('label:nth-of-type(2) input', 'p:nth-of-type(3) output', ''),
        formState({ checked: [true, false, true], outputs: ['V0', '', 'V1'] }),
        uncheckThenNormalizedText('label:nth-of-type(3) input', 'p:nth-of-type(4) output', ''),
        formState({ checked: [true, false, false], outputs: ['V0', '', ''] }),
    ]),
    sampleContract('13. Radio group', [
        formState({ checked: [false, true], outputs: ['V1'] }),
        checkThenText('label:first-of-type input', 'output', 'V0'),
        formState({ checked: [true, false], outputs: ['V0'] }),
        checkThenText('label:last-of-type input', 'output', 'V1'),
        formState({ checked: [false, true], outputs: ['V1'] }),
    ]),
];

const domChainSamples = [
    sampleContract("Wrap values", [normalizedText('output', "a, b, a")]),
    sampleContract("Parent", [normalizedText('output', "row")]),
    sampleContract("Element children", [normalizedText('output', "a, b")]),
    sampleContract("All child nodes", [normalizedText('output', "4")]),
    sampleContract("Ancestors", [normalizedText('output', "section, row")]),
    sampleContract("Closest ancestor", [normalizedText('output', "section")]),
    sampleContract("Local names", [normalizedText('output', "fruit")]),
    sampleContract("Node text", [normalizedText('output', "ivysaur")]),
    sampleContract("Unqualified attribute", [normalizedText('output', "2")]),
    sampleContract("Qualified attribute", [normalizedText('output', "other")]),
    sampleContract("First match", [normalizedText('output', "2")]),
    sampleContract("Last match", [normalizedText('output', "b")]),
    sampleContract("Filter", [normalizedText('output', "a, a")]),
    sampleContract("First value", [normalizedText('output', "a")]),
    sampleContract("Last value", [normalizedText('output', "b")]),
    sampleContract("Zero-based selection", [normalizedText('output', "b")]),
    sampleContract("Take a prefix", [normalizedText('output', "a, b")]),
    sampleContract("Skip a prefix", [normalizedText('output', "b, c")]),
    sampleContract("Map values", [normalizedText('output', "a!, b!")]),
    sampleContract("Flatten mapped values", [normalizedText('output', "a, a, b, b")]),
    sampleContract("Any match", [normalizedText('output', "true")]),
    sampleContract("All match", [normalizedText('output', "false")]),
    sampleContract("Empty chain", [normalizedText('output', "true")]),
    sampleContract("Count values", [normalizedText('output', "3")]),
    sampleContract("Sort values", [normalizedText('output', "a, b, b")]),
    sampleContract("Sort native nodes", [normalizedText('output', "second, first, third")]),
    sampleContract("Reverse order", [normalizedText('output', "c, b, a")]),
];

const xpathMapArraySamples = [
    sampleContract('1. An IP-filter map with an optional note', [
        normalizedText('article p:first-of-type output', 'allow: 192.0.2.0/24'),
        normalizedText('article p:nth-of-type(2) output', 'Absent entry'),
        selectThenText('article select[aria-label="Note entry"]', 'empty', 'article p:nth-of-type(2) output', 'Present, empty sequence'),
        normalizedText('article p:nth-of-type(3) output', '3'),
        selectThenText('article select[aria-label="Note entry"]', 'value', 'article p:nth-of-type(2) output', 'Local preview'),
        fillThenText('article input', '198.51.100.0/24', 'article p:first-of-type output', 'allow: 198.51.100.0/24'),
        selectThenText('article select[aria-label="Action"]', 'deny', 'article p:first-of-type output', 'deny: 198.51.100.0/24'),
    ]),
    sampleContract('2. Select a retained fruit by array position', [
        normalizedText('article p:nth-of-type(2) output', 'apple: 2'),
        fillThenText('article input', '2', 'article p:nth-of-type(2) output', 'pear: 3'),
        fillThenText('article input', '0', 'article p:nth-of-type(2) output', 'No member at this position'),
        fillThenText('article textarea', '<basket note="Fresh"><plum>4</plum></basket>', 'article p:nth-of-type(3) output', 'Note: Fresh'),
        fillThenText('article input', '1', 'article p:nth-of-type(2) output', 'plum: 4'),
        fillThenText('article textarea', '<other/>', 'article [role="alert"]', 'Use a basket root'),
        fillThenText('article textarea', '<basket/>', 'article p:first-of-type output', '0'),
        normalizedText('article p:nth-of-type(3) output', 'Empty member (array size 1)'),
    ]),
];

xpathMapArraySamples.push(sampleContract('3. Query an imported JSON tree', [
    normalizedText('article p:first-of-type output', '3 members; numeric total 5'),
    normalizedText('article p:nth-of-type(2) output', 'Null value'),
    fillThenText('article textarea', '{"cherry":4,"note":""}', 'article p:nth-of-type(2) output', 'Empty string'),
    fillThenText('article textarea', '{"plum":1e2}', 'article p:first-of-type output', '1 members; numeric total 100'),
    normalizedText('article p:nth-of-type(2) output', 'Absent member'),
    fillThenText('article textarea', '[]', 'article [role="alert"]', 'Use a JSON object'),
    fillThenText('article textarea', '{"note":null}', 'article p:nth-of-type(2) output', 'Null value'),
]));

const xpathValidationSamples = [
    sampleContract('1. Form validation preview', [
        normalizedText('article p:nth-of-type(1) output', 'Valid user name'),
        normalizedText('article p:nth-of-type(2) output', 'Age in range'),
        normalizedText('article p:nth-of-type(3) output', 'Blue / green / RED'),
        fillThenText('article label:nth-of-type(1) input', '7Ada', 'article p:nth-of-type(1) output', 'start with a letter'),
        fillThenText('article label:nth-of-type(2) input', '17', 'article p:nth-of-type(2) output', '18 to 120'),
        fillThenText('article label:nth-of-type(2) input', '1e2', 'article p:nth-of-type(2) output', 'Enter an integer age'),
        fillThenText('article label:nth-of-type(3) input', 'red,,blue', 'article p:nth-of-type(3) output', 'per tag'),
        fillThenText('article label:nth-of-type(1) input', 'Grace_2', 'article p:nth-of-type(1) output', 'Valid user name'),
        fillThenText('article label:nth-of-type(2) input', '120', 'article p:nth-of-type(2) output', 'Age in range'),
        fillThenText('article label:nth-of-type(3) input', 'Red; GOLD', 'article p:nth-of-type(3) output', 'Red / GOLD'),
    ]),
    sampleContract('2. IPv4 prefix-rule preview', [
        normalizedText('article output', 'Allowed by the local prefix rule'),
        fillThenText('article label:first-of-type input', '192.0.2.10/16', 'article output', 'Blocked by the local prefix rule'),
        fillThenText('article label:first-of-type input', '256.0.2.10/24', 'article output', 'Octets must be 0–255'),
        fillThenText('article label:first-of-type input', '192.0.2.10/33', 'article output', 'prefix length 0–32'),
        fillThenText('article label:first-of-type input', '192.00.2.10/24', 'article output', 'no leading zeros'),
        fillThenText('article label:first-of-type input', '::1', 'article output', 'Enter IPv4'),
        fillThenText('article label:first-of-type input', '255.255.255.255', 'article output', 'Allowed by the local prefix rule'),
        fillThenText('article label:last-of-type input', 'bad', 'article output', 'Enter allowed prefix lengths'),
        fillThenText('article label:last-of-type input', '24', 'article output', 'Blocked by the local prefix rule'),
        fillThenText('article label:first-of-type input', '192.0.2.10/24', 'article output', 'Allowed by the local prefix rule'),
    ]),
];

const xpathSortSamples = [
    sampleContract('1. Text and numeric keys', [
        normalizedText('article output', '02 / 1 / 10 / 2 / bad'),
        checkThenText('article label:nth-of-type(2) input', 'article output', '1 / 2 / 02 / 10 / bad'),
        checkThenText('article label:nth-of-type(3) input', 'article output', '10 / 2 / 02 / 1 / bad'),
        fillThenText('article input[type=text]', '3 nope 03 bad -1', 'article output', '3 / 03 / -1 / nope / bad'),
        uncheckThenNormalizedText('article label:nth-of-type(2) input', 'article output', 'nope / bad / 3 / 03 / -1'),
    ]),
    sampleContract('2. Multiple keys and source selection', [
        normalizedText('article tbody tr:first-child button', 'Cherry'),
        normalizedText('article tbody tr:last-child button', 'Mango'),
        clickThenText('article button[value="c"]', 'article output:first-of-type', 'Cherry'),
        normalizedText('article output:last-of-type', 'Apple'),
        checkThenText('article input[type=checkbox]', 'article tbody tr:first-child button', 'Apple'),
        normalizedText('article button[aria-pressed=true]', 'Cherry'),
        normalizedText('article output:last-of-type', 'Apple'),
        fillThenText('article textarea', '<r>', 'article [role="alert"]', 'XML'),
        fillThenText('article textarea', '<r><row id="c" group="A" qty="1">One</row></r>', 'article tbody button', 'One'),
    ]),
];

const xpathAggregateSamples = [
    sampleContract('1. Decimal sequence statistics', [
        normalizedText('article p:first-of-type output', '0.3'),
        normalizedText('article p:last-of-type output', '0.15'),
        fillThenText('article textarea', '-2 1 4', 'article p:first-of-type output', '3'),
        normalizedText('article p:nth-of-type(2) output', '-2'),
        fillThenText('article textarea', 'bad', 'article [role="alert"]', 'Enter decimal'),
        fillThenText('article textarea', '', 'article p:first-of-type output', '0'),
        normalizedText('article p:last-of-type output', '∅'),
    ]),
    sampleContract('2. A basket that accepts new fruits', [
        normalizedText('article p:first-of-type output', '3.75'),
        fillThenText('article textarea', '<basket><apple>0.1</apple><cherry>0.2</cherry><pear>0.6</pear></basket>', 'article p:first-of-type output', '0.9'),
        normalizedText('article tbody tr:last-child th', 'pear'),
        normalizedText('article p:last-of-type output', '0.3'),
        fillThenText('article textarea', '<basket><pear>bad</pear></basket>', 'article [role="alert"]', 'non-negative decimal'),
        fillThenText('article textarea', '<basket/>', 'article p:first-of-type output', '0'),
        normalizedText('article p:last-of-type output', '∅'),
        countExactly('article tbody tr', 0),
    ]),
];

const xpathSequenceSamples = [
    sampleContract('1. A window into a word sequence', [
        normalizedText('article p:nth-of-type(2) output', '3'),
        normalizedText('article p:nth-of-type(3) output', 'apple | cherry'),
        checkThenText('article input[type="checkbox"]', 'article p:nth-of-type(3) output', 'cherry | apple'),
        fillThenText('article textarea', 'a b c d', 'article p:nth-of-type(3) output', 'c | b'),
        normalizedText('article p:nth-of-type(4) output', 'c'),
        normalizedText('article p:nth-of-type(5) output', 'b'),
        fillThenText('article textarea', '', 'article p:nth-of-type(2) output', '0'),
    ]),
    sampleContract('2. First-seen XML columns', [
        ...['@id', 'fruit', '@qty', 'note'].map((value, index) =>
            normalizedText(`article th:nth-child(${index + 1})`, value)),
        ...['2', 'Apple', '∅', '∅'].map((value, index) =>
            normalizedText(`article tbody tr:first-child td:nth-child(${index + 1})`, value)),
        normalizedText('article tbody tr:nth-child(2) td:last-child', '""'),
        fillThenText('article textarea', '<r><row><fruit>A</fruit></row><row new="yes"><fruit>B</fruit></row></r>', 'article th:nth-child(2)', '@new'),
        normalizedText('article th:first-child', 'fruit'),
        normalizedText('article tbody tr:nth-child(2) td:last-child', 'yes'),
        fillThenText('article textarea', '<r/>', 'article thead', ''),
        countExactly('article th', 0),
        countExactly('article td', 0),
    ]),
];

const xpathNodeSamples = [
    sampleContract('1. XML table with native navigation', [
        normalizedText('tbody tr:first-child td:nth-child(3)', 'Apple'),
        normalizedText('tbody tr:first-child small', 'urn:b'),
        clickThenText('button[aria-label="Select row 1"]', 'output:last-child', 'Zest'),
        fillThenText('textarea', '<basket><item id="1" qty="4">Cherry</item></basket>', 'tbody td:nth-child(3)', 'Cherry'),
    ]),
    sampleContract('2. XML tree with attributes and mixed text', [
        normalizedText('article code', 'bright'),
        normalizedText('article li:nth-child(2) > code', 'Hello & welcome'),
        fillThenText('textarea', '<r>Recovered</r>', 'article code', 'Recovered'),
    ]),
];

const localStorageSamples = [
    sampleContract('0. Read a live text value', [
        normalizedText('output', 'stored initial'),
        clickThenText('button:first-of-type', 'output', 'text value'),
        clickThenText('button:nth-of-type(2)', 'output', 'another value'),
        clickThenText('button:nth-of-type(3)', 'p', 'liveText slice:'),
        normalizedText('output', ''),
        clickThenText('button:nth-of-type(4)', 'output', 'null'),
        clickThenText('button:first-of-type', 'output', 'text value'),
    ]),
    sampleContract('1. Always override a stored value', [
        normalizedText('output', 'ABC'),
        clickThenText('button:first-of-type', 'output', 'ABC'),
        clickThenText('button:nth-of-type(2)', 'output', 'ABC'),
    ]),
    sampleContract('2. Stored value with a default', [
        normalizedText('output', 'DEF'),
        clickThenText('button:first-of-type', 'output', 'remember me'),
        clickThenText('button:nth-of-type(3)', 'output', 'null'),
        clickThenText('button:first-of-type', 'output', 'remember me'),
    ]),
    sampleContract('3a. Date validation', [
        normalizedText('output', '2024-04-20'),
        clickThenText('button:nth-of-type(3)', 'output', 'null'),
        clickThenText('form button', 'output', '2024-02-29'),
        clickThenText('button:nth-of-type(2)', 'output', '2024-04-21'),
    ]),
    sampleContract('3b. Time validation', [
        normalizedText('output', '13:30'),
        clickThenText('button:nth-of-type(2)', 'output', 'null'),
        clickThenText('form button', 'output', '09:15'),
    ]),
    sampleContract('3c. Local date and time validation', [
        normalizedText('output', '1977-04-01T14:00:30'),
        clickThenText('button:nth-of-type(2)', 'output', 'null'),
        clickThenText('form button', 'output', '2024-04-20T09:15'),
    ]),
    sampleContract('3d. Number validation', [
        normalizedText('p:nth-of-type(2) output', '123456'),
        clickThenText('button:nth-of-type(2)', 'p:nth-of-type(2) output', '1'),
        normalizedText('p:first-of-type output', '0001'),
        clickThenText('button:nth-of-type(3)', 'p:nth-of-type(2) output', '0'),
        normalizedText('p:nth-of-type(2) output', '0'),
        clickThenText('button:nth-of-type(4)', 'p:nth-of-type(2) output', 'null'),
        normalizedText('p:first-of-type output', 'ABC'),
        clickThenText('form button', 'p:nth-of-type(2) output', '24'),
    ]),
    sampleContract('3e. JSON validation', [
        text('ul', 'b : B'),
        clickThenText('button:nth-of-type(2)', 'p:nth-of-type(2) output', 'ABC'),
        clickThenText('button:nth-of-type(3)', 'p:nth-of-type(2) output', '12.345'),
        clickThenText('button:nth-of-type(4)', 'p:nth-of-type(2) output', 'false'),
        clickThenText('button:nth-of-type(5)', 'p:nth-of-type(2) output', 'null'),
        normalizedText('p:first-of-type output', 'ABC'),
        clickThenText('form button', 'p:nth-of-type(2) output', 'array'),
        normalizedText('ol', '1 2 3'),
        clickThenText('button:first-of-type', 'ul', 'b : B'),
    ]),
    sampleContract('4. Simplest initial read', [
        normalizedText('cem-storage-cherries', '12 🍒'),
        clickThenText('button:first-of-type', 'output', '12'),
    ]),
    sampleContract('5. Live JSON basket', [
        text('dl', '🛒 13'),
        clickThenText('button:first-of-type', 'dl', '🛒 14'),
        clickThenText('button:nth-of-type(2)', 'dl', '🛒 15'),
        clickThenText('button:nth-of-type(3)', 'dl', '🛒 13'),
    ]),
    sampleContract('6. Fruit buttons and a storage watcher', [
        text('dl', '🛒 13'),
        clickThenText('button[aria-label="Add lemon"]', 'dl', '🍋 2'),
        clickThenText('button[aria-label="Add cherry"]', 'dl', '🍒 13'),
        clickThenText('button[aria-label="Add apple"]', 'dl', '🍏 1'),
        clickThenText('button[aria-label="Add banana"]', 'dl', '🍌 1'),
        text('dl', '🛒 17'),
    ]),
    sampleContract('7. Write a slice back to storage', [
        text('cem-storage-editor:first-of-type output', 'shared initial'),
        fillThenText(
            'cem-storage-editor:first-of-type input',
            'shared edit',
            'cem-storage-editor:last-of-type output',
            'shared edit',
        ),
    ]),
];

function splitPartChecks(...values) {
    return [
        countExactly('ol > li', values.length),
        ...values.map((value, index) =>
            propertyEquals(`ol > li:nth-of-type(${index + 1})`, 'textContent', `“${value}”`)),
    ];
}

const xpathWordSample = sampleContract('3. XPath word and character count', [
    normalizedText('p:first-of-type strong', '5'),
    normalizedText('p:nth-of-type(2) strong', '3'),
    fillThenText('textarea', ' one\tone\n🍒  🍋 ', 'p:nth-of-type(2) strong', '4'),
    fillThenText('textarea', '\t\n\u00a0\u2003', 'p:nth-of-type(2) strong', '1'),
    fillThenText('textarea', '🍒e\u0301', 'p:first-of-type strong', '3'),
    fillThenText('textarea', ' \t\n', 'p:nth-of-type(2) strong', '0'),
    fillThenText('textarea', '', 'p:first-of-type strong', '0'),
]);

const stringMethodSamples = [
    sampleContract('URL ID with a string chain', [
        normalizedText('output', '1'),
        fillThenText('input', 'https://pokeapi.co/api/v2/pokemon/10/', 'output', '10'),
        fillThenText('input', '/short/', 'output', 'No ID'),
    ]),
    sampleContract('str:split', [
        normalizedText('output', '4'),
        ...splitPartChecks('🍒', '🍋', '', '🍌'),
        fillThenText('label:nth-of-type(2) input', '', 'output', '8'),
        fillThenText('label:first-of-type input', '🍒🍋', 'output', '4'),
        ...splitPartChecks('', '🍒', '🍋', ''),
        fillThenText('label:first-of-type input', '', 'output', '2'),
        fillThenText('label:nth-of-type(2) input', ',', 'output', '1'),
        ...splitPartChecks(''),
        fillThenText('label:first-of-type input', 'a::b::::', 'ol', 'a::b::::'),
        ...splitPartChecks('a::b::::'),
        fillThenText('label:nth-of-type(2) input', '::', 'output', '4'),
        ...splitPartChecks('a', 'b', '', ''),
    ]),
    ...[
        ['trim', '🍒  🍋', '🍌'],
        ['trim_start', '🍒  🍋  ', '🍌  '],
        ['trim_end', '  🍒  🍋', '  🍌'],
    ].map(([method, initial, edited]) => sampleContract(`str:${method}`, [
        propertyEquals('output', 'textContent', initial),
        fillThenText('input', '  🍌  ', 'output', '🍌'),
        propertyEquals('output', 'textContent', edited),
        fillThenText('input', '', 'output', ''),
        propertyEquals('output', 'textContent', ''),
    ])),
    ...['char_at', 'at'].map((method) => sampleContract(`str:${method}`, [
        normalizedText('output', method === 'at' ? '🍌' : '🍋'),
        fillThenText('input[type="number"]', '99', 'output', method === 'at' ? '∅' : ''),
        normalizedText('output', method === 'at' ? '∅' : ''),
        fillThenText('input[type="number"]', '-1', 'output', method === 'at' ? '🍌' : ''),
        normalizedText('output', method === 'at' ? '🍌' : ''),
        fillThenText('input[type="number"]', '0', 'output', '🍒'),
    ])),
    ...['index_of', 'last_index_of'].map((method) => sampleContract(`str:${method}`, [
        normalizedText('output', method === 'index_of' ? '0' : '2'),
        fillThenText('input[type="number"]', '1', 'output', method === 'index_of' ? '2' : '0'),
        fillThenText('label:nth-of-type(2) input', '🍋', 'output', '1'),
        fillThenText('label:nth-of-type(2) input', '🥦', 'output', '-1'),
        fillThenText('label:nth-of-type(2) input', '', 'output', '1'),
        normalizedText('output', '1'),
        fillThenText('input[type="number"]', '99', 'output', '3'),
        fillThenText('label:first-of-type input', 'aaaa', 'output', '4'),
        fillThenText('label:nth-of-type(2) input', 'aa', 'output', method === 'index_of' ? '-1' : '2'),
        fillThenText('input[type="number"]', '1', 'output', '1'),
        normalizedText('output', '1'),
    ])),
    sampleContract('XPath normalize-space', [
        propertyEquals('output', 'textContent', '🍒 🍋'),
        fillThenText('textarea', ' a\ta\n🍒  ', 'output', 'a a 🍒'),
        fillThenText('textarea', ' a\u00a0\u2003b ', 'output', 'a b'),
        propertyEquals('output', 'textContent', 'a\u00a0\u2003b'),
        fillThenText('textarea', ' \t\n', 'output', ''),
    ]),
    sampleContract('XPath tokenize and string-join', [
        propertyEquals('output', 'textContent', '🍒/🍒/🍋'),
        fillThenText('textarea', 'a\ta\nb', 'output', 'a/a/b'),
        fillThenText('input', '🍒', 'output', 'a🍒a🍒b'),
        fillThenText('textarea', 'a\u00a0b', 'output', 'a b'),
        propertyEquals('output', 'textContent', 'a\u00a0b'),
        fillThenText('textarea', '', 'output', ''),
    ]),
];

const wordCountEdgeChecks = [
    fillBlurThenText(
        'cem-demo-element[legend="1. Textarea word count"] textarea', ' one\tone\n🍒  🍋 ',
        'cem-demo-element[legend="1. Textarea word count"] form > p strong', '4',
    ),
    fillBlurThenText(
        'cem-demo-element[legend="1. Textarea word count"] textarea', '\t\n\u00a0\u2003',
        'cem-demo-element[legend="1. Textarea word count"] form > p strong', '0',
    ),
    fillThenText(
        'cem-demo-element[legend="2. Input word and character count"] input', '🍒 🍒 🍋',
        'cem-demo-element[legend="2. Input word and character count"] form > p:nth-of-type(2) strong', '3',
    ),
    normalizedText('cem-demo-element[legend="2. Input word and character count"] form > p:first-of-type strong', '5'),
    fillThenText(
        'cem-demo-element[legend="2. Input word and character count"] input', '',
        'cem-demo-element[legend="2. Input word and character count"] form > p:nth-of-type(2) strong', '0',
    ),
];

const dataTableSamples = [
    sampleContract('1. XML table: attributes and text', [
        text('table[aria-label$="/row"]', 'ivysaur'),
        text('table[aria-label$="/row"]', '@mood'),
        ...tableInteractions('@id', 'table[aria-label$="/row"]'),
    ]),
    sampleContract('2. CSV table: quoted fields', [
        text('tbody', 'sweet, red'), text('tbody', 'say "zest"'),
        ...tableInteractions('qty', 'table'),
    ]),
    sampleContract('3. YAML table: nested collections', [
        countExactly('table', 2), text('table[aria-label="document"]', 'fresh'),
        ...tableInteractions('qty', 'table[aria-label="document"]'),
    ]),
    sampleContract('4. JSON table: empty and missing', [
        text('tbody', '∅'), text('tbody', '""'), text('tbody', 'null'),
        ...tableInteractions('qty', 'table'),
        fillBlurThenText('textarea', '[oops]', '[role="alert"]', '⚠'),
        countExactly('table', 0),
        clickThenText('button[aria-label="Reset source"]', 'tbody', '🍒'),
        countExactly('[role="alert"]', 0),
    ]),
    sampleContract('5. Presentation aspects: tree and IP-filter form', [
        countExactly('table[aria-label="visits"]', 1),
        countExactly('table[aria-label="notes"]', 0),
        text('form[aria-label="IP filter"] output', 'allow'),
        text('form[aria-label="IP filter"] output', '192.0.2.0/24'),
        fillBlurThenText('input[aria-label="Address / CIDR"]', '198.51.100.0/24', 'form output', '198.51.100.0/24'),
        selectThenText('select[aria-label="Action"]', 'deny', 'form output', 'deny'),
        clickThenText('input[aria-label="Presentation aspects"]', 'article', 'ip-filter'),
        countExactly('table[aria-label="notes"]', 1),
        countExactly('form[aria-label="IP filter"]', 0),
        clickThenText('input[aria-label="Presentation aspects"]', 'form output', '198.51.100.0/24'),
        text('form output', 'deny'),
        countExactly('table[aria-label="notes"]', 0),
        countExactly('table[aria-label="visits"]', 1),
    ]),
    sampleContract('6. XSLT table: native sorting and selection', [
        text('tbody', '∅'), text('tbody', '""'), text('tbody', 'null'),
        ...tableInteractions('qty', 'table'),
        fillBlurThenText('textarea', '[oops]', '[role="alert"]', '⚠'),
        countExactly('table', 0),
        clickThenText('button[aria-label="Reset source"]', 'tbody', '🍒'),
        countExactly('[role="alert"]', 0),
    ]),
    sampleContract('7. XSLT aspects: tree and IP-filter form', [
        countExactly('table[aria-label="visits"]', 1),
        countExactly('table[aria-label="notes"]', 0),
        text('form[aria-label="IP filter"] output', 'allow'),
        text('form[aria-label="IP filter"] output', '192.0.2.0/24'),
        fillBlurThenText('input[aria-label="Address / CIDR"]', '198.51.100.0/24', 'form output', '198.51.100.0/24'),
        selectThenText('select[aria-label="Action"]', 'deny', 'form output', 'deny'),
        clickThenText('input[aria-label="Presentation aspects"]', 'article', 'ip-filter'),
        countExactly('table[aria-label="notes"]', 1),
        countExactly('form[aria-label="IP filter"]', 0),
        clickThenText('input[aria-label="Presentation aspects"]', 'form output', '198.51.100.0/24'),
        text('form output', 'deny'),
        countExactly('table[aria-label="notes"]', 0),
        countExactly('table[aria-label="visits"]', 1),
    ]),
    sampleContract('./data-table-view.cemt', [
        attributeEquals(':scope', 'src', './data-table-view.cemt'),
        attributeEquals(':scope', 'type', 'text/cem-ml'),
    ]),
    sampleContract('./data-table-view.xslt', [
        attributeEquals(':scope', 'src', './data-table-view.xslt'),
        attributeEquals(':scope', 'type', 'application/xslt+xml'),
    ]),
    sampleContract('./data-table-aspects.xslt', [
        attributeEquals(':scope', 'src', './data-table-aspects.xslt'),
        attributeEquals(':scope', 'type', 'application/xslt+xml'),
    ]),
];

function tableInteractions(column, table) {
    return [
        clickThenText(`${table} > tbody > tr:nth-child(2) > th > button`, `${table} > tbody`, '✓'),
        selectThenText('select[aria-label="Sort column"]', column, `${table} > tbody > tr:first-child`, '10'),
        selectThenText('select[aria-label="Compare"]', 'number', `${table} > tbody > tr:first-child > td:nth-child(2)`, '2'),
        attributeEquals(`${table} > tbody > tr:first-child > th > button`, 'aria-pressed', 'true'),
        selectThenText('select[aria-label="Direction"]', 'descending', `${table} > tbody > tr:first-child > td:nth-child(2)`, '10'),
        attributeEquals(`${table} > tbody > tr:last-child > th > button`, 'aria-pressed', 'true'),
        clickThenText('article > details > summary', 'article', 'Source'),
        propertyEquals('article > details', 'open', false),
        pressThenProperty('article > details > summary', 'Enter', 'article > details', 'open', true),
        pressThenProperty('article > details > summary', 'Space', 'article > details', 'open', false),
        pressThenProperty('article > details > summary', 'Enter', 'article > details', 'open', true),
    ];
}

const xsltVariantSamples = [
    sampleContract('7d. Anonymous external XSLT', [
        text('article h2', 'XSLT XML payload tree'),
        text('article', '🍒 from anonymous XSLT'),
        countExactly('details', 4),
        clickThenText('article > details > summary', 'article', 'catalog'),
        propertyEquals('article > details', 'open', false),
        clickThenText('article > details > summary', 'article', 'catalog'),
        propertyEquals('article > details', 'open', true),
    ]),
    sampleContract('7e. Embedded XSLT fragment', [
        text('article h2', 'Embedded XSLT fruit tree'),
        normalizedText('summary', 'basket'),
        countExactly('details', 1),
        countExactly('li', 2),
        normalizedText('li:first-of-type', '🍒'),
        normalizedText('li:last-of-type', '🍋'),
        clickThenText('summary', 'article', 'basket'),
        propertyEquals('details', 'open', false),
        clickThenText('summary', 'article', 'basket'),
        propertyEquals('details', 'open', true),
    ]),
];

const mappedImageFragmentSample = sampleContract('4d. Mapped image and same-library fragment', [
    countExactly('article img', 2),
    text('article', '👌 from embed-relative-hash'),
    text('article', '👋 from embed-lib-component'),
    attributeContains('img[alt="Mapped Smiley"]', 'src', '/demo/lib-dir/Smiley.svg'),
    attributeContains('img[alt="Library Smiley"]', 'src', '/demo/lib-dir/Smiley.svg'),
    attributeContains('article > a', 'href', '/demo/lib-dir/embed-lib.html#embed-relative-hash'),
    attributeContains('article cem-element a', 'href', '/demo/lib-dir/embed-lib.html#embed-lib-component'),
]);

const hexRowSample = sampleContract('9. Horizontal row with a current page', [
    countExactly('nav[aria-label="Demo page links"] a', 3),
    countExactly('nav a[aria-current="page"]', 1),
    countExactly('nav a[aria-current="false"]', 2),
    attributeContains('nav a[aria-current="page"]', 'href', '/demo/hex-grid.html'),
    text('nav a[aria-current="page"] .hex-label', '✓'),
    computedStyle('nav ul', 'flexWrap', 'nowrap'),
    computedStyle('nav ul', 'overflowX', 'auto'),
    computedStyle('nav a[aria-current="page"] .hex-label', 'color', 'rgb(254, 240, 138)'),
    computedStyle('nav a[aria-current="page"] .hex-label', 'transform', 'matrix(1, 0, 0, 1, 0, 0)'),
]);

const cellPokemonNames = ['bulbasaur', 'ivysaur', 'venusaur', 'charmander', 'charmeleon', 'charizard', 'squirtle', 'wartortle', 'blastoise', 'caterpie'];
const cellStockRows = [['Cherry', 'Out of stock (0)'], ['Lemon', '5'], ['Apple', '12'], ['Pear', '3'], ['Plum', '8']];
function cellStockChecks(rows) {
    return rows.flatMap(([name, stock], index) => [
        normalizedText(`table > tbody > tr:nth-child(${index + 1}) > td:nth-last-of-type(2) .value`, name),
        normalizedText(`table > tbody > tr:nth-child(${index + 1}) > td:last-of-type :is(strong, .value)`, stock),
    ]);
}

const cellOverrideSamples = [
    sampleContract('1. Name cells become Pokémon pictures', [
        countExactly('img', 10),
        ...cellPokemonNames.map(name => imageLoaded(`img[alt="${name}"]`)),
        attributeEquals('img[alt="ivysaur"]', 'src', 'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/2.svg'),
        text('table > tbody > tr:first-child', 'bulbasaur'),
        clickThenText('table > tbody > tr:first-child button', 'tr[aria-selected=true]', 'bulbasaur'),
        selectThenText('select[aria-label="Sort column"]', 'name',
            'table > tbody > tr:first-child', 'blastoise'),
        attributeEquals('table > tbody > tr:first-child img', 'alt', 'blastoise'),
        countExactly('img', 10),
        selectThenText('select[aria-label="Direction"]', 'descending',
            'table > tbody > tr:first-child', 'wartortle'),
        ...[...cellPokemonNames].sort().reverse().flatMap((name, index) => [
            normalizedText(`table > tbody > tr:nth-child(${index + 1}) .pokemon-name`, name),
            normalizedText(`table > tbody > tr:nth-child(${index + 1}) > td:last-child`,
                `https://pokeapi.co/api/v2/pokemon/${cellPokemonNames.indexOf(name) + 1}/`),
        ]),
        countExactly('tr[aria-selected=true]', 1),
        text('tr[aria-selected=true]', 'bulbasaur'),
        attributeEquals('tr[aria-selected=true] button', 'aria-pressed', 'true'),
        countExactly('img', 10),
        countExactly('textarea', 0),
        countExactly('button[aria-label="Reset source"]', 0),
    ]),
    sampleContract('pokemon-cells.json', [
        attributeContains(':scope', 'src', '/pokemon-cells.json'),
        attributeEquals(':scope', 'type', 'json'),
        attributeEquals(':scope', 'demo', 'false'),
        propertyEquals('[slot=text] code', 'textContent',
            await readFile(join(repoRoot, 'packages/cem-elements/demo/pokemon-cells.json'), 'utf8')),
        propertyEquals('[slot=demo]', 'textContent', ''),
        countExactly('[slot=demo] > *', 0),
        countExactly('table', 0),
    ]),
    sampleContract('2. Zero-stock cells get a warning', [
        countExactly('strong', 1),
        normalizedText('table strong', 'Out of stock (0)'),
        countExactly('table > tbody > tr', 5),
        normalizedText('table > thead th:nth-last-child(2)', 'name'),
        normalizedText('table > thead th:last-child', 'stock'),
        ...cellStockChecks(cellStockRows),
        clickThenText('table > tbody > tr:first-child button', 'tr[aria-selected=true]', 'Cherry'),
        selectThenText('select[aria-label="Sort column"]', 'name',
            'table > tbody > tr:first-child', 'Apple'),
        text('tr[aria-selected=true]', 'Cherry'),
        selectThenText('select[aria-label="Direction"]', 'descending',
            'table > tbody > tr:first-child', 'Plum'),
        ...cellStockChecks([...cellStockRows].sort(([left], [right]) => right.localeCompare(left))),
        countExactly('tr[aria-selected=true]', 1),
        text('tr[aria-selected=true]', 'Cherry'),
        attributeEquals('tr[aria-selected=true] button', 'aria-pressed', 'true'),
        countExactly('strong', 1),
        countExactly('textarea', 0),
        countExactly('button[aria-label="Reset source"]', 0),
    ]),
    sampleContract('stock-cells.xml', [
        attributeContains(':scope', 'src', '/stock-cells.xml'),
        attributeEquals(':scope', 'type', 'xml'),
        attributeEquals(':scope', 'demo', 'false'),
        propertyEquals('[slot=text] code', 'textContent',
            await readFile(join(repoRoot, 'packages/cem-elements/demo/stock-cells.xml'), 'utf8')),
        propertyEquals('[slot=demo]', 'textContent', ''),
        countExactly('[slot=demo] > *', 0),
        countExactly('table', 0),
    ]),
    sampleContract('stock-cell.cemt', [
        attributeContains(':scope', 'src', '/stock-cell.cemt'),
        attributeEquals(':scope', 'type', 'cem-ml'),
        attributeEquals(':scope', 'demo', 'false'),
        propertyEquals('[slot=text] code', 'textContent',
            await readFile(join(repoRoot, 'packages/cem-elements/demo/stock-cell.cemt'), 'utf8')),
        propertyEquals('[slot=demo]', 'textContent', ''),
        countExactly('[slot=demo] > *', 0),
        countExactly('table', 0),
    ]),
    sampleContract('3. Native values pass into another component', [
        text('cem-native-value-card article', 'Next count: 3'),
        text('cem-native-value-card article', 'Date: 2024-02-29'),
        propertyEquals('cem-native-value-card .value-label > name', 'textContent', 'ivysaur'),
        normalizedText('cem-native-value-card .value-label > name > em', 'saur'),
        normalizedText('cem-native-value-card article > section > p', 'Text-only label: ivysaur'),
        countExactly('cem-native-value-card article > section name', 0),
    ]),
];

const tableInspectorSamples = [
    sampleContract('1. Columns from every row', [
        text('table caption', 'document/row'),
        normalizedText('table > tbody > tr:first-child > td:nth-child(2)', '""'),
        normalizedText('table > tbody > tr:first-child > td:last-child', '∅'),
        normalizedText('table > tbody > tr:last-child > td:nth-child(2)', '∅'),
        normalizedText('table > tbody > tr:last-child > td:last-child', 'ripe'),
        countExactly('th[scope="col"]', 4),
    ]),
    sampleContract('2. Text-only rows stay visible', [
        normalizedText('table > tbody > tr:first-child > td:last-child', 'ivysaur'),
        normalizedText('table > tbody > tr:last-child > td:last-child', 'venusaur'),
        pressThenProperty('button[aria-label="Sort #text descending in document/name"]', 'Enter',
            'th[aria-sort]', 'ariaSort', 'descending'),
        normalizedText('table > tbody > tr:first-child > td:last-child', 'venusaur'),
        normalizedText('table > tbody > tr:last-child > td:last-child', 'ivysaur'),
    ]),
    sampleContract('3. Nested tables keep their own state', [
        countExactly('table', 3),
        checkThenText('table[aria-label="document/row"] > tbody > tr:first-child table > tbody > tr:first-child input',
            'table[aria-label="document/row"] > tbody > tr:first-child table caption output', '1'),
        clickThenText('table[aria-label="document/row"] > tbody > tr:first-child button[aria-label="Sort #text descending in tags/tag"]',
            'table[aria-label="document/row"] > tbody > tr:first-child table > tbody > tr:first-child > td:last-child', 'sweet'),
        normalizedText('table[aria-label="document/row"] > tbody > tr:last-child table > tbody > tr:first-child > td:last-child', 'yellow'),
        normalizedText('table[aria-label="document/row"] > tbody > tr:last-child table caption output', '0'),
        propertyEquals('table[aria-label="document/row"] > tbody > tr:first-child table > tbody > tr:last-child input', 'checked', true),
        countExactly('table[aria-label="document/row"] > tbody > tr[aria-selected="true"]', 0),
    ]),
    sampleContract('4. Multiple selections survive sorting', [
        pressThenProperty('table > tbody > tr:nth-child(2) input', 'Space',
            'table > tbody > tr:nth-child(2) input', 'checked', true),
        normalizedText('caption output', '1'),
        pressThenProperty('table > tbody > tr:nth-child(3) input', 'Space',
            'table > tbody > tr:nth-child(3) input', 'checked', true),
        normalizedText('caption output', '2'),
        clickThenText('button[aria-label="Sort @qty ascending in document/row"]',
            'table > tbody > tr:first-child > td:last-child', 'Apple 🍏'),
        selectThenText('select[aria-label="Compare document/row"]', 'number',
            'table > tbody > tr:first-child > td:last-child', 'Cherry 🍒'),
        normalizedText('table > tbody > tr:nth-child(2) > td:last-child', 'Apple 🍏'),
        attributeEquals('th[aria-sort]', 'aria-sort', 'ascending'),
        propertyEquals('table > tbody > tr:nth-child(2) input', 'checked', true),
        propertyEquals('table > tbody > tr:nth-child(3) input', 'checked', true),
        clickThenText('button[aria-label="Sort @qty descending in document/row"]',
            'table > tbody > tr:first-child > td:last-child', 'Lemon 🍋'),
        normalizedText('table > tbody > tr:nth-child(2) > td:last-child', 'Cherry 🍒'),
        normalizedText('table > tbody > tr:nth-child(3) > td:last-child', 'Apple 🍏'),
        normalizedText('table > tbody > tr:last-child > td:last-child', 'Banana 🍌'),
        normalizedText('caption output', '2'),
        countExactly('input:checked', 2),
        clickThenText('button[aria-label="Restore source order in document/row"]',
            'table > tbody > tr:first-child > td:last-child', 'Cherry 🍒'),
        countExactly('th[aria-sort]', 0),
        countExactly('input:checked', 2),
        clickThenText('button[aria-label="Reset source"]', 'caption output', '0'),
        countExactly('input:checked', 0),
        fillBlurThenText('textarea', '<broken>', '[role="alert"]', 'XML'),
        countExactly('table', 0),
        clickThenText('button[aria-label="Reset source"]', 'table > tbody > tr:first-child > td:last-child', 'Cherry 🍒'),
        countExactly('input:checked', 0),
    ]),
];

const dataTreeSamples = [
    sampleContract('1. XML branches: independent selection', [
        text('pre[aria-label="CEM-ML document"]', '{ast'),
        computedStyle('pre[aria-label="CEM-ML document"] code', 'whiteSpace', 'pre-wrap'),
        text('pre[aria-label="CEM-ML document"]', '<raw>🍋'),
        normalizedText('output[aria-label="Selected branches"]', '0'),
        pressThenProperty('input[aria-label="Select branch 1.1: fruit"]', 'Space',
            'input[aria-label="Select branch 1.1: fruit"]', 'checked', true),
        normalizedText('output[aria-label="Selected branches"]', '1'),
        pressThenProperty('input[aria-label="Select branch 1.2: fruit"]', 'Space',
            'input[aria-label="Select branch 1.2: fruit"]', 'checked', true),
        normalizedText('output[aria-label="Selected branches"]', '2'),
        pressThenProperty('li:has(> label > input[aria-label="Select branch 1.1: fruit"]) > details > summary', 'Enter',
            'li:has(> label > input[aria-label="Select branch 1.1: fruit"]) > details', 'open', false),
        normalizedText('output[aria-label="Selected branches"]', '2'),
        propertyEquals('input[aria-label="Select branch 1.1: fruit"]', 'checked', true),
        pressThenProperty('input[aria-label="Select branch 1.2: fruit"]', 'Space',
            'input[aria-label="Select branch 1.2: fruit"]', 'checked', false),
        normalizedText('output[aria-label="Selected branches"]', '1'),
        propertyEquals('li:has(> label > input[aria-label="Select branch 1.1: fruit"]) > details', 'open', false),
        pressThenProperty('li:has(> label > input[aria-label="Select branch 1.1: fruit"]) > details > summary', 'Space',
            'li:has(> label > input[aria-label="Select branch 1.1: fruit"]) > details', 'open', true),
        clickThenText('article > button', 'output[aria-label="Selected branches"]', '0'),
        countExactly('input:checked', 0),
        countExactly('script, raw', 0),
    ]),
    sampleContract('2. JSON through the same CEM tree', [
        text('pre[aria-label="CEM-ML document"]', '🍒'),
        text('pre[aria-label="CEM-ML document"]', 'sweet'),
        normalizedText('output[aria-label="Selected branches"]', '0'),
        countExactly('input:checked', 0),
    ]),
    sampleContract('3. Malformed source and repair', [
        text('[role="alert"]', 'could not be imported'),
        countExactly('pre[aria-label="CEM-ML document"]', 0),
        fillBlurThenText('textarea', '<orchard><fruit>🍒</fruit></orchard>', 'pre[aria-label="CEM-ML document"]', '{ast'),
        countExactly('[role="alert"]', 0),
        clickThenText('article > button', '[role="alert"]', 'could not be imported'),
        countExactly('input[type="checkbox"]', 0),
        fillBlurThenText('textarea', '<orchard><fruit>🍋</fruit></orchard>', 'pre[aria-label="CEM-ML document"]', '🍋'),
        normalizedText('output[aria-label="Selected branches"]', '0'),
    ]),
    sampleContract('4. Load and release a local document', [
        text('pre[aria-label="CEM-ML document"]', 'xml-stylesheet'),
        propertyEquals('select[aria-label="Local source"]', 'value', './tree-source.xml'),
        attributeContains('article a', 'href', '/demo/tree-source.xml'),
        attributeEquals('article a', 'download', ''),
        selectThenText('select[aria-label="Local source"]', './tree-source.json',
            'pre[aria-label="CEM-ML document"]', 'tree-source.json'),
        normalizedText('output[aria-label="Selected branches"]', '0'),
        selectThenText('select[aria-label="Local source"]', '', 'output[aria-label="Request state"]', 'idle'),
        countExactly('pre[aria-label="CEM-ML document"]', 0),
        selectThenText('select[aria-label="Local source"]', './tree-source.xml',
            'pre[aria-label="CEM-ML document"]', 'xml-stylesheet'),
        countExactly('input:checked', 0),
    ]),
];

const fixtureSpecs = [
    {
        path: '/packages/cem-elements/demo/cell-overrides.html',
        checks: cellOverrideSamples.flatMap((sample) => sample.checks.map((check) =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/table-inspector.html',
        checks: tableInspectorSamples.flatMap((sample) => sample.checks.map((check) =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/data-tree.html',
        checks: dataTreeSamples.flatMap((sample) => sample.checks.map((check) =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/data-table.html',
        checks: dataTableSamples.flatMap((sample) => sample.checks.map((check) =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/index.html',
        checks: [
            text('dce-link a', 'link'),
            text('dce-1-slot', '\u{1f955}'),
            attributeEquals(
                'cem-demo-element[legend^="2a."] dce-2-slots:first-of-type input',
                'placeholder',
                '\u{1f407}\u{2764}\u{fe0f}\u{1f955}',
            ),
            attributeEquals(
                'cem-demo-element[legend^="2a."] dce-2-slots:last-of-type input',
                'placeholder',
                '\u{1f407}\u{2764}\u{fe0f}\u{1f407}',
            ),
            normalizedText('cem-demo-element[legend^="2b."] dce-3-slot:first-of-type', '1 \u{1f603} 2 \u{1f603}'),
            normalizedText('cem-demo-element[legend^="2c."] dce-4-slot', '1 \u{1f955} 2 \u{1f955}'),
            text('pokemon-tile h3', 'bulbasaur'),
            text('pokemon-tile', 'Smile as:'),
            attributeEquals(
                'pokemon-tile img[alt="bulbasaur image"]',
                'src',
                'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/1.svg',
            ),
            attributeEquals(
                'pokemon-tile button img[alt="ivysaur"]',
                'src',
                'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/2.svg',
            ),
            countExactly('pokemon-tile[title="bulbasaur"] button', 2),
            countExactly('pokemon-tile[title="ninetales"] button', 1),
            countExactly('pokemon-tile button img:not([alt]), pokemon-tile button img[alt=""]', 0),
            text('pokemon-tile button', 'ivysaur'),
            text('pokemon-tile button', 'venusaur'),
            text('pokemon-tile button', 'vulpix'),
            countAtLeast(
                'cem-demo-element[legend^="1."] code.cem-source-code.language-html > b',
                2,
            ),
            countAtLeast(
                'cem-demo-element[legend^="1."] code.cem-source-code.language-html > var',
                1,
            ),
            countAtLeast(
                'cem-demo-element[legend^="1."] code.cem-source-code.language-html > i',
                1,
            ),
            countExactly(
                'cem-demo-element[legend^="1."] code.cem-source-code [class], cem-demo-element[legend^="1."] code.cem-source-code [data-role]',
                0,
            ),
        ],
    },
    {
        path: '/packages/cem-elements/demo/attributes.html',
        checks: [
            text('#defaults-1 article.demo-card h2', 'attributes definition'),
            text('#defaults-2 article.demo-card', 'p1: 123'),
            text('#defaults-2 article.demo-card', 'p2: always_p2'),
            attributeEquals('#defaults-1', 'p1', 'default_P1'),
            attributeEquals('#defaults-1', 'p2', 'always_p2'),
            attributeEquals('#defaults-1', 'p3', 'def_P3'),
            clickThenText('button[aria-label="set p2"]', '#defaults-2', 'p2: always_p2'),
            attributeEquals('#defaults-2', 'p2', 'always_p2'),
            fillThenText('#p3-input', '', '#defaults-2 article.demo-card p:last-of-type', 'p3: def_P3'),
            clickThenText('button[aria-label="set p3"]', '#defaults-2 article.demo-card p:last-of-type', 'p3:'),
            normalizedText('#defaults-2 article.demo-card p:last-of-type', 'p3:'),
            attributeEquals('#defaults-2', 'p3', ''),
            clickThenText('button[aria-label="remove p3"]', '#defaults-2', 'p3: def_P3'),
            attributeEquals('#defaults-2', 'p3', 'def_P3'),
            attributeEquals('#title-from-slice', 'title', '😃'),
            fillThenText('#title-from-slice input', 'Typed title', '#title-from-slice', 'title attribute: Typed title'),
            attributeEquals('#title-from-slice', 'title', 'Typed title'),
            attributeEquals('#value-default', 'v', 'def'),
            attributeEquals('#value-default', 'is-changed', 'false'),
            fillThenText('#value-default input', 'From input', '#value-default', 'v: From input'),
            attributeEquals('#value-default', 'v', 'From input'),
            attributeEquals('#value-default', 'is-changed', 'true'),
            text('#precedence-default article.demo-card', 'datadom.attributes.v: def'),
            typeThenText(
                '#precedence-default input',
                'qqq',
                '#precedence-default article.demo-card',
                'datadom.attributes.v: qqq',
            ),
            text('#precedence-default article.demo-card', 'effective value: qqq'),
            text('#precedence-default article.demo-card', 'has-input: true'),
            attributeEquals('#precedence-default', 'v', 'qqq'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/data-slices.html',
        checks: dataSliceSamples.flatMap((sample) => sample.checks.map((check) =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/xpath-maps-arrays.html',
        checks: xpathMapArraySamples.flatMap((sample) => sample.checks.map((check) =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/xpath-validation.html',
        checks: xpathValidationSamples.flatMap((sample) => sample.checks.map((check) =>
            scopeCheck(check, 'cem-demo-element[legend="' + sample.legend + '"]'))),
    },
    {
        path: '/packages/cem-elements/demo/xpath-sort.html',
        checks: xpathSortSamples.flatMap((sample) => sample.checks.map((check) =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/xpath-aggregates.html',
        checks: xpathAggregateSamples.flatMap((sample) => sample.checks.map((check) =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/xpath-sequences.html',
        checks: xpathSequenceSamples.flatMap((sample) => sample.checks.map((check) =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/xpath-nodes.html',
        checks: xpathNodeSamples.flatMap((sample) => sample.checks.map((check) =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/xpath-functions.html',
        checks: [
            sampleContract('1. Named XPath function', [
                normalizedText('output', 'Hello 🍒'),
                fillThenText('input', 'Changed', 'output', 'Changed 🍒'),
                fillThenText('input', '', 'output', '🍒'),
            ]),
            sampleContract('1a. CEM-QL string pair', [
                normalizedText('output', 'Hello 🍒'),
                fillThenText('input', 'Changed', 'output', 'Changed 🍒'),
                fillThenText('input', '', 'output', '🍒'),
            ]),
            sampleContract('2. Shared XPath predicate', [
                normalizedText('output', 'cherry 🍒'),
                fillThenText('input', 'lemon', 'output', 'Try cherry'),
                fillThenText('input', 'cherry', 'output', 'cherry 🍒'),
            ]),
            sampleContract('2a. CEM-QL predicate pair', [
                normalizedText('output', 'cherry 🍒'),
                fillThenText('input', 'lemon', 'output', 'Try cherry'),
                fillThenText('input', 'cherry', 'output', 'cherry 🍒'),
            ]),
            sampleContract('3. XML nodes and matching', [
                normalizedText('li', 'Cherry : stocked'),
                fillThenText('textarea', '<r><item qty="1">Lemon</item></r>', 'li', 'Lemon : low stock'),
                fillThenText('textarea', '<r><item qty="3">Grape</item></r>', 'li', 'Grape : stocked'),
            ]),
            sampleContract('3a. CEM-QL native node pair', [
                normalizedText('li', 'Cherry : stocked'),
                fillThenText('textarea', '<r><item qty="1">Lemon</item></r>', 'li', 'Lemon : low stock'),
                fillThenText('textarea', '<r><item qty=" 2.5 ">Grape</item></r>', 'li', 'Grape : stocked'),
            ]),
        ].flatMap((sample) => sample.checks.map((check) =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/dom-merge.html',
        checks: [
            text('cem-demo-element[legend="1. Textarea word count"] h2', 'Textarea word count'),
            fillBlurThenText(
                'cem-demo-element[legend="1. Textarea word count"] textarea',
                'one two three',
                'cem-demo-element[legend="1. Textarea word count"] form > p strong',
                '3',
            ),
            fillThenText(
                'cem-demo-element[legend="2. Input word and character count"] input',
                'two words',
                'cem-demo-element[legend="2. Input word and character count"] output',
                'two words',
            ),
            text(
                'cem-demo-element[legend="2. Input word and character count"] form > p:first-of-type strong',
                '9',
            ),
            text(
                'cem-demo-element[legend="2. Input word and character count"] form > p:nth-of-type(2) strong',
                '2',
            ),
            ...wordCountEdgeChecks,
            ...xpathWordSample.checks.map((check) => scopeCheck(check, `cem-demo-element[legend="${xpathWordSample.legend}"]`)),
        ],
    },
    {
        path: '/packages/cem-elements/demo/external-template.html',
        allowedPageErrors: ['Failed to load resource: the server responded with a status of 404 (Not Found)'],
        checks: [
            text('dce-internal', '👋'),
            text('dce-internal', 'World!'),
            countExactly('cem-demo-element[legend="2. without TAG, inline instantiation"] cem-element[src="#template2"]', 2),
            text('cem-demo-element[legend="2. without TAG, inline instantiation"] cem-element[src="#template2"]', 'construction'),
            countAtLeast('dce-external svg', 1),
            svgUseReferences('dce-external svg', ['#h', '#j']),
            countAtLeast('cem-element[src="confused.svg"] svg', 1),
            svgUseReferences('cem-element[src="confused.svg"] svg', ['#h', '#j']),
            text('dce-external-missing', 'fallback for missing image'),
            text('dce-external-4', 'External CEMT data-island transformation'),
            text('dce-external-4', 'template[data-cem-island="instance"]'),
            text('dce-external-4', 'cem-island:context-root'),
            text('dce-external-4', 'cem-hydration:data'),
            text('dce-external-4', 'cem-attributes:attributes'),
            text('dce-external-4', 'cem-dataset:dataset'),
            text('dce-external-4', 'cem-payload:payload'),
            text('dce-external-4', 'cem-slices:slices'),
            text('dce-external-4', 'cem-resources:resources'),
            text('dce-external-4', 'cem-form:form-state'),
            text('dce-external-4', 'cem-validation:validation-state'),
            text('dce-external-4', 'cem-events:event-state'),
            text('dce-external-4', 'Payload comment: explicit inert envelope follows'),
            text('dce-external-4', 'DCE with complete external CEMT island'),
            text('dce-external-4', 'wrapped-payload'),
            text('dce-external-4', 'slot="heading"'),
            text('dce-external-4', 'slot=""'),
            text('dce-external-4', 'data-fruit="🍌"'),
            text('dce-external-4', 'aria-label="Fruit choice"'),
            text('dce-external-4', 'Every element, attribute, dataset entry, and text node is data.'),
            countAtLeast('dce-external-4 details', 20),
            text('dce-external-4-inline', 'A second external-CEMT data island'),
            text('dce-external-4-inline', 'DCE with live payload capture'),
            text('dce-external-4-inline', 'name="data-smile"'),
            text('dce-external-4-inline', 'name="data-basket"'),
            text('dce-external-4-inline', 'data-kind="live-payload"'),
            text('dce-external-4-cem-ml', 'content-type="text/cem-ml"'),
            text('dce-external-4-cem-ml', 'Banana from CEM-ML payload source'),
            text('dce-external-5', '👋'),
            text('dce-external-5', '👌'),
            countAtLeast('dce-external-5 svg', 1),
            countAtLeast('dce-external-5 math', 1),
            attributeEquals('#dce-external-5-inline', 'data-cem-anonymous-declaration', ''),
            text('#dce-external-5-inline > [data-cem-anonymous-instance]', '👋'),
            text('#dce-external-5-inline > [data-cem-anonymous-instance]', '👌'),
            countAtLeast('#dce-external-5-inline > [data-cem-anonymous-instance] svg', 1),
            countAtLeast('#dce-external-5-inline > [data-cem-anonymous-instance] math', 1),
            text('dce-html-wave', '👋'),
            countAtLeast('dce-html-logo svg', 1),
            countAtLeast('dce-html-formula math', 1),
            text('dce-cemt-tree', 'CEM-ML data island tree'),
            text('dce-cemt-tree article.demo-card > details > summary > b', 'catalog'),
            text('dce-cemt-tree', 'data-root='),
            text('dce-cemt-tree', 'data-level='),
            text('dce-cemt-tree', 'cem-elements'),
            text('dce-cemt-tree', 'code='),
            text('dce-cemt-tree', 'a1'),
            text('dce-cemt-tree', 'Leaf text from cem-elements data island'),
            countAtLeast('dce-cemt-tree details', 4),
            text('dce-xslt-tree', 'XSLT XML payload tree'),
            text('dce-xslt-tree article.demo-card > details > summary > b', 'catalog'),
            text('dce-xslt-tree', 'data-root='),
            text('dce-xslt-tree', 'data-level='),
            text('dce-xslt-tree', 'cem-elements-xslt'),
            text('dce-xslt-tree', 'code='),
            text('dce-xslt-tree', 'b1'),
            text('dce-xslt-tree', 'Leaf text from cem-elements XSLT data island'),
            countAtLeast('dce-xslt-tree details', 4),
            text('dce-missing-none', 'element with id=none is missing in template'),
            ...xsltVariantSamples.flatMap((sample) => sample.checks.map((check) =>
                scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
            text('dce-embed-1', '🖖'),
            text('dce-embed-relative-hash', 'from embed-lib-component'),
            text('dce-embed-relative-file', '🖖'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/for-each.html',
        checks: [
            countExactly('cem-demo-element[legend]', 11),
            countExactly('cem-demo-element[legend="1. Simple for-each"] li', 3),
            text('cem-demo-element[legend="1. Simple for-each"] li', '🍏'),
            text('cem-demo-element[legend="2. for-each with position()"]', '1 . Red'),
            text('cem-demo-element[legend="2. for-each with position()"]', '3 . Blue'),
            clickThenText('cem-demo-element[legend="3. Conditional for-each"] input[type="checkbox"]', 'cem-demo-element[legend="3. Conditional for-each"] span', '1 : First'),
            countExactly('cem-demo-element[legend="4. Nested for-each table"] tbody tr', 3),
            text('cem-demo-element[legend="4. Nested for-each table"] tbody', 'B2'),
            text('cem-demo-element[legend="5. for-each with attributes"] article', '# 1 Alice ( admin )'),
            text('cem-demo-element[legend="5. for-each with attributes"] article', '# 3 Charlie ( viewer )'),
            clickThenText('cem-demo-element[legend="6. Dynamic table with toggle"] input[type="checkbox"]', 'cem-demo-element[legend="6. Dynamic table with toggle"] tbody', 'Widget'),
            text('cem-loop-payload .payload-feed li', 'payload-alpha : Payload Alpha'),
            text('cem-loop-payload .payload-feed li', 'payload-beta : Payload Beta'),
            text('cem-demo-element[legend="8. for-each over location data"] .location-feed li', 'topic = feeds'),
            text('cem-demo-element[legend="8. for-each over location data"] .location-feed li', 'item = payload,resource'),
            text('cem-loop-http output[data-role="json-state"]', 'loaded'),
            text('cem-loop-http .http-json-feed li', 'alpha : ready'),
            text('cem-loop-http .http-json-feed li', 'beta : loaded'),
            text('cem-loop-http output[data-role="xml-state"]', 'loaded'),
            text('cem-loop-http .http-xml-feed li', 'gamma : xml-ready'),
            text('cem-loop-http .http-xml-feed li', 'delta : xml-loaded'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/form.html',
        checks: [
            countExactly('cem-demo-element[legend]', 5),
            text('main > section', 'datadom.formData.<slice>'),
            fillThenText(
                'cem-demo-element[legend="1. Simple validation"] input[name="username"]',
                'long-username',
                'cem-demo-element[legend="1. Simple validation"] form',
                'Username: long-username',
            ),
            clickThenText(
                'cem-demo-element[legend="1. Simple validation"] button',
                'cem-demo-element[legend="1. Simple validation"] form', '🔑',
            ),
            fillThenText(
                'cem-demo-element[legend="1. Simple validation"] input[name="password"]',
                'secret',
                'cem-demo-element[legend="1. Simple validation"] form > p:nth-of-type(2) output',
                'true',
            ),
            fillThenText(
                'cem-demo-element[legend="2. Form lifecycle"] input[name="username"]',
                'long-username',
                'cem-demo-element[legend="2. Form lifecycle"] form > p:nth-of-type(2) output',
                'long-username',
            ),
            clickThenText(
                'cem-demo-element[legend="2. Form lifecycle"] input[value="sms"]',
                'cem-demo-element[legend="2. Form lifecycle"] fieldset',
                'Message and data rates may apply.',
            ),
            clickThenText(
                'cem-demo-element[legend="2. Form lifecycle"] input[value="password"]',
                'cem-demo-element[legend="2. Form lifecycle"] form > p:nth-of-type(3) output',
                'password',
            ),
            fillThenText(
                'cem-demo-element[legend="2. Form lifecycle"] input[name="password"]',
                'secret',
                'cem-demo-element[legend="2. Form lifecycle"] form > p:nth-of-type(4) output',
                'true',
            ),
            fillThenText(
                'cem-demo-element[legend="3. Native control validity message"] input[name="email"]',
                '',
                'cem-demo-element[legend="3. Native control validity message"] form > p:nth-of-type(2) output',
                'Please fill out this field.',
            ),
            fillThenText(
                'cem-demo-element[legend="4. Form custom validity message"] input[name="email"]',
                'abc',
                'cem-demo-element[legend="4. Form custom validity message"] form > p:nth-of-type(4) output',
                'Use more than 3 characters',
            ),
            text(
                'cem-demo-element[legend="4. Form custom validity message"] form > p:nth-of-type(2) output',
                '3',
            ),
            clickThenText(
                'cem-demo-element[legend="5. DCE as a form input"] cem-form-fruit-choice:first-of-type button[data-option-index="1"]',
                'cem-demo-element[legend="5. DCE as a form input"] form > p:first-of-type output:first-of-type',
                '🍏',
            ),
            clickThenText(
                'cem-demo-element[legend="5. DCE as a form input"] cem-form-fruit-choice:last-of-type button[data-option-index="2"]',
                'cem-demo-element[legend="5. DCE as a form input"] form > p:nth-of-type(3) output',
                'Choose the same fruit',
            ),
            clickThenText(
                'cem-demo-element[legend="5. DCE as a form input"] cem-form-fruit-choice:last-of-type button[data-option-index="1"]',
                'cem-demo-element[legend="5. DCE as a form input"] form > p:nth-of-type(2) output',
                'true',
            ),
        ],
    },
    {
        path: '/packages/cem-elements/demo/hex-grid.html',
        checks: [
            countAtLeast(
                'cem-demo-element[legend="1. Responsive framework link honeycomb"] code.cem-source-code.language-html > b',
                2,
            ),
            countAtLeast(
                'cem-demo-element[legend="1. Responsive framework link honeycomb"] code.cem-source-code.language-html > var',
                1,
            ),
            countAtLeast(
                'cem-demo-element[legend="1. Responsive framework link honeycomb"] code.cem-source-code.language-html > i',
                1,
            ),
            countExactly(
                'cem-demo-element[legend="1. Responsive framework link honeycomb"] code.cem-source-code [class], cem-demo-element[legend="1. Responsive framework link honeycomb"] code.cem-source-code [data-role]',
                0,
            ),
            computedStyle(
                'cem-demo-element[legend="1. Responsive framework link honeycomb"] code.cem-source-code.language-html > b',
                'color',
                'rgb(0, 47, 101)',
            ),
            computedStyle(
                'cem-demo-element[legend="1. Responsive framework link honeycomb"] code.cem-source-code.language-html > var',
                'color',
                'rgb(80, 36, 0)',
            ),
            computedStyle(
                'cem-demo-element[legend="1. Responsive framework link honeycomb"] code.cem-source-code.language-html > i',
                'color',
                'rgb(106, 27, 154)',
            ),
            countExactly('cem-demo-element[legend="1. Responsive framework link honeycomb"] cem-hex-grid .hex-link', 14),
            countExactly('cem-demo-element[legend="1. Responsive framework link honeycomb"] cem-hex-grid .hex-logo', 14),
            text('cem-demo-element[legend="1. Responsive framework link honeycomb"] cem-hex-grid .hex:last-child .hex-label', 'Next.js'),
            attributeContains('cem-demo-element[legend="1. Responsive framework link honeycomb"] cem-hex-grid .hex:first-child .hex-link', 'href', '/demo/module-url.html'),
            attributeEquals('cem-demo-element[legend="1. Responsive framework link honeycomb"] cem-hex-grid .hex:nth-child(2) .hex-link', 'href', 'https://react.dev/'),
            attributeEquals(
                'cem-demo-element[legend="1. Responsive framework link honeycomb"] cem-hex-grid .hex:nth-child(2) .hex-logo',
                'src',
                'https://upload.wikimedia.org/wikipedia/commons/a/a7/React-icon.svg',
            ),
            attributeContains('cem-demo-element[legend="1. Responsive framework link honeycomb"] cem-hex-grid .hex:first-child .hex-logo', 'src', '/demo/framework-logos/wc-square.svg'),
            computedStyle('cem-demo-element[legend="1. Responsive framework link honeycomb"] cem-hex-grid .hex-grid', 'display', 'flex'),
            computedStyleNot('cem-demo-element[legend="1. Responsive framework link honeycomb"] cem-hex-grid .hex-link', 'clipPath', 'none'),
            computedStyle('cem-demo-element[legend="1. Responsive framework link honeycomb"] cem-hex-image-link', 'boxSizing', 'border-box'),
            countExactly('cem-demo-element[legend="2. Compact percentage links"] .hex-link', 6),
            attributeEquals(
                'cem-demo-element[legend="2. Compact percentage links"] cem-hex-grid',
                'size',
                '65%',
            ),
            countExactly('cem-demo-element[legend="3. Fixed-length links"] .hex-link', 6),
            attributeEquals(
                'cem-demo-element[legend="3. Fixed-length links"] cem-hex-grid',
                'size',
                '35rem',
            ),
            countExactly('cem-demo-element[legend="4. Alternating backgrounds"] .hex-link', 3),
            attributeEquals(
                'cem-demo-element[legend="4. Alternating backgrounds"] cem-hex-grid',
                'alternate',
                'true',
            ),
            attributeContains(
                'cem-demo-element[legend="4. Alternating backgrounds"] .hex:nth-child(2)',
                'class',
                'hex-true',
            ),
            text('cem-demo-element[legend="5. Wrapping long label"] .hex-label', 'Declarative Custom Element Framework With A Long Name'),
            computedStyle('cem-demo-element[legend="6. Missing-image fallback"] .hex-logo', 'opacity', '0'),
            computedStyle('cem-demo-element[legend="6. Missing-image fallback"] .image-fallback', 'visibility', 'visible'),
            countExactly('cem-demo-element[legend="7. Wrapper DCE theme"] .hex-link', 3),
            text(
                'cem-demo-element[legend="7. Wrapper DCE theme"] code.cem-source-code.language-html > b',
                'style',
            ),
            text(
                'cem-demo-element[legend="7. Wrapper DCE theme"] code.cem-source-code.language-html > var',
                '--cem-hex-background-start',
            ),
            text(
                'cem-demo-element[legend="7. Wrapper DCE theme"] code.cem-source-code.language-html > u',
                '1rem',
            ),
            computedStyle('cem-demo-element[legend="7. Wrapper DCE theme"] .hex-label', 'color', 'rgb(30, 27, 75)'),
            computedStyle('cem-demo-element[legend="7. Wrapper DCE theme"] .theme-frame', 'backgroundColor', 'rgb(238, 242, 255)'),
            countExactly('cem-demo-element[legend="8. Image-button presentation"] .hex-link', 1),
            computedStyleNot('cem-demo-element[legend="8. Image-button presentation"] .hex-link', 'filter', 'none'),
            computedStyle('cem-demo-element[legend="8. Image-button presentation"] .hex-label', 'transform', 'matrix(1, 0, 0, 1, 0, 0)'),
            ...hexRowSample.checks.map((check) => scopeCheck(
                check, `cem-demo-element[legend="${hexRowSample.legend}"]`,
            )),
        ],
    },
    {
        path: '/packages/cem-elements/demo/http-request.html',
        checks: [
            text(
                'cem-demo-element[legend="0. URL from text to http-request"] article',
                'Request state: idle',
            ),
            clickThenText(
                'cem-demo-element[legend="0. URL from text to http-request"] article > button',
                'cem-demo-element[legend="0. URL from text to http-request"] article', 'Request state: loaded',
            ),
            text(
                'cem-demo-element[legend="0. URL from text to http-request"] li',
                'beta : loaded',
            ),
            clickThenText(
                'cem-demo-element[legend="0. URL from text to http-request"] .url-presets button:nth-of-type(2)',
                'cem-demo-element[legend="0. URL from text to http-request"] article',
                'Selected URL: ./http-data-compact.json',
            ),
            clickThenText(
                'cem-demo-element[legend="0. URL from text to http-request"] article > button',
                'cem-demo-element[legend="0. URL from text to http-request"] li',
                'solo : compact',
            ),
            countExactly(
                'cem-demo-element[legend="1. Simplest http-request"] .result-buttons button',
                6,
            ),
            attributeEquals(
                'cem-demo-element[legend="1. Simplest http-request"] .result-buttons button:first-of-type',
                'aria-label',
                'bulbasaur',
            ),
            attributeEquals(
                'cem-demo-element[legend="1. Simplest http-request"] .result-buttons button:first-of-type img',
                'src',
                'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/1.svg',
            ),
            attributeEquals(
                'cem-demo-element[legend="1. Simplest http-request"] .result-buttons button:last-of-type img',
                'src',
                'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/6.svg',
            ),
            text(
                'cem-demo-element[legend="2. http-request response and headers"] dl dd:nth-of-type(6)',
                'ported-from-legacy',
            ),
            text(
                'cem-demo-element[legend="2. http-request response and headers"] dl dd:nth-of-type(7)',
                '200',
            ),
            text(
                'cem-demo-element[legend="2. http-request response and headers"] dl dd:nth-of-type(8)',
                'application/json',
            ),
            attributeContains(
                'cem-demo-element[legend="2. http-request response and headers"] dl a',
                'href',
                '/packages/cem-elements/demo/http-data.json',
            ),
            shortenedHrefText(
                'cem-demo-element[legend="2. http-request response and headers"] dl a',
                32,
            ),
        ],
    },
    {
        path: '/packages/cem-elements/demo/local-storage.html',
        checks: localStorageSamples.flatMap((sample) => sample.checks.map(
            (check) => scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`),
        )),
    },
    {
        path: '/packages/cem-elements/demo/location-element.html',
        checks: [
            text('cem-demo-element[legend="2. Window location initial read"] dl', 'source window'),
            text('cem-demo-element[legend="3. External URL from href"] dl', 'hostname my.example'),
            text('cem-demo-element[legend="3. External URL from href"] dl', 'pathname /docs'),
            text('cem-demo-element[legend="3. External URL from href"] dl', 'hash #details'),
            text('cem-demo-element[legend="3. External URL from href"] ul', 'a = 1'),
            text('cem-demo-element[legend="3. External URL from href"] ul', 'b = 2,3'),
            clickThenText(
                'cem-demo-element[legend="1. Window location live update"] button:has-text("history.pushState")',
                'cem-demo-element[legend="1. Window location live update"] dl',
                '#checked',
            ),
            text('cem-demo-element[legend="1. Window location live update"] ul', 'mode = history.pushState'),
            text('cem-demo-element[legend="1. Window location live update"] ul', 'tag = one,two'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/module-url.html',
        checks: [
            attributeContains(
                'cem-demo-element[legend="1. module path by symbolic name"] image-link',
                'src',
                '/packages/cem-elements/demo/wc-square.svg',
            ),
            attributeContains(
                'cem-demo-element[legend="1. module path by symbolic name"] image-link img',
                'src',
                '/packages/cem-elements/demo/wc-square.svg',
            ),
            attributeContains(
                'cem-demo-element[legend="1. module path by symbolic name"] image-link a',
                'href',
                '/packages/cem-elements/demo/wc-square.svg',
            ),
            shortenedHrefText('cem-demo-element[legend="1. module path by symbolic name"] image-link a', 32),
            attributeContains(
                'cem-demo-element[legend="2. src forms: relative URL"] image-link',
                'src',
                'Smiley.svg?src=relative',
            ),
            attributeContains(
                'cem-demo-element[legend="2. src forms: relative URL"] image-link img',
                'src',
                'Smiley.svg?src=relative',
            ),
            shortenedHrefText('cem-demo-element[legend="2. src forms: relative URL"] image-link a', 32),
            attributeContains(
                'cem-demo-element[legend="3. src forms: absolute URL"] image-link',
                'src',
                'data:image/svg+xml,',
            ),
            attributeContains(
                'cem-demo-element[legend="3. src forms: absolute URL"] image-link img',
                'src',
                'data:image/svg+xml,',
            ),
            shortenedHrefText('cem-demo-element[legend="3. src forms: absolute URL"] image-link a', 32),
            text('cem-module-relative-declaration', '🖖'),
            text('cem-module-mapped-declaration', '👋 from embed-lib-component'),
            normalizedText(
                'cem-demo-element[legend="4b. Missing import-map entry"] output',
                'not published',
            ),
            text('cem-module-mapped-fragment', '👍 from embed-relative-file'),
            text('cem-module-mapped-fragment', '🖖'),
            ...mappedImageFragmentSample.checks.map((check) => scopeCheck(
                check, `cem-demo-element[legend="${mappedImageFragmentSample.legend}"]`,
            )),
            attributeContains('cem-local-map-naked-image img.component-owned-image', 'src', 'Smiley.svg?owner=component'),
            shortenedHrefText('cem-local-map-naked-image image-link a', 32),
            attributeContains('cem-local-map-override-wrapper img.component-owned-image', 'src', 'confused.svg?owner=wrapper'),
            shortenedHrefText('cem-local-map-override-wrapper image-link a', 32),
            attributeContains('cem-local-map-referrer-demo img.node-referrer-image', 'src', 'wc-square.svg?owner=component'),
            shortenedHrefText('cem-local-map-referrer-demo image-link.node-referrer-image a', 32),
            text('cem-local-map-referrer-demo table.node-referrer-matrix td:first-of-type', 'Smiley.svg?referrer=node'),
            text('cem-local-map-referrer-demo table.node-referrer-matrix td:nth-of-type(2)', 'wc-square.svg?owner=component'),
            text('cem-local-map-referrer-demo table.node-referrer-matrix td:last-of-type', 'https://assets.example.test/logo.svg'),
            shortenedHrefText('cem-demo-element[legend="image-link"] image-link a', 32),
            countExactly('cem-module-url', 0),
        ],
    },
    {
        path: '/packages/cem-elements/demo/module-url-referrer.html',
        checks: [
            text('tbody tr:first-of-type td:first-of-type', 'Smiley.svg?case=relative-relative'),
            text('tbody tr:first-of-type td:nth-of-type(2)', 'Smiley.svg?referrer=relative'),
            text('tbody tr:nth-of-type(2) td:nth-of-type(2)', 'confused.svg?referrer=module'),
            text('tbody tr:last-of-type td:first-of-type', 'https://referrer.example.test/lib-dir/Smiley.svg?case=relative-absolute'),
            text('tbody tr:last-of-type td:nth-of-type(2)', 'wc-square.svg?referrer=absolute'),
            countExactly('cem-module-url', 0),
        ],
    },
    {
        path: '/packages/cem-elements/demo/functions/dom.html',
        checks: domChainSamples.flatMap(sample => sample.checks.map(check =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/functions/str.html',
        checks: [
            ...stringMethodSamples.flatMap((sample) => sample.checks.map(
                (check) => scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`),
            )),
            normalizedText('cem-str-shorten-matrix tbody tr:first-of-type td:nth-of-type(2)', 'short'),
            normalizedText('cem-str-shorten-matrix tbody tr:nth-of-type(2) td:nth-of-type(2)', 'abc…hij'),
            normalizedText('cem-str-shorten-matrix tbody tr:nth-of-type(3) td:nth-of-type(2)', 'abc…ghij'),
            normalizedText('cem-str-shorten-matrix tbody tr:nth-of-type(4) td:nth-of-type(2)', 'ab...hij'),
            normalizedText('cem-str-shorten-matrix tbody tr:nth-of-type(5) td:nth-of-type(2)', 'abchij'),
            normalizedText('cem-str-shorten-matrix tbody tr:nth-of-type(6) td:nth-of-type(2)', 'αβ💠ζη'),
            normalizedText('cem-str-shorten-matrix tbody tr:last-of-type td:nth-of-type(2)', 'https://example…emantic-card.cem'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/npm-versions-demo.html',
        checks: [
            countExactly('cem-demo-element[legend="1. Default to the latest version"] select option', 4),
            normalizedText(
                'cem-demo-element[legend="1. Default to the latest version"] select option:first-of-type',
                '0.1.0',
            ),
            propertyEquals(
                'cem-demo-element[legend="2. Preselect a version and show dates"] select',
                'value',
                '0.0.22',
            ),
            text(
                'cem-demo-element[legend="2. Preselect a version and show dates"] option[value="0.0.22"]',
                '2024-04-20',
            ),
            selectThenText(
                'cem-demo-element[legend="3. Propagate the selected value"] select',
                '0.0.25',
                'cem-demo-element[legend="3. Propagate the selected value"] output',
                '0.0.25',
            ),
            attributeEquals('cem-npm-version-propagated', 'value', '0.0.25'),
            text('cem-demo-element[legend="4. Override the label slot"] label', 'Select a release:'),
            clickThenText(
                'cem-demo-element[legend="5. Synchronize the selected version with the URL"] button[aria-label="Set URL to 0.0.22"]',
                'cem-demo-element[legend="5. Synchronize the selected version with the URL"] article',
                'Current hash: #version=0.0.22',
            ),
            propertyEquals(
                'cem-demo-element[legend="5. Synchronize the selected version with the URL"] select',
                'value',
                '0.0.22',
            ),
            selectThenText(
                'cem-demo-element[legend="5. Synchronize the selected version with the URL"] select',
                '0.1.0',
                'cem-demo-element[legend="5. Synchronize the selected version with the URL"] article',
                'selected-version slice: 0.1.0',
            ),
            text(
                'cem-demo-element[legend="5. Synchronize the selected version with the URL"] article',
                'Current hash: #version=0.1.0',
            ),
        ],
    },
    {
        path: '/packages/cem-elements/demo/scoped-css.html',
        checks: [
            // Private declaration CSS: one managed style, native tag scope,
            // no style cloned into either instance, the classless outside button
            // keeps browser/global styling, and ordinary outer cascade remains open.
            countExactly('cem-css-private button', 2),
            text('cem-css-private button', 'First DCE dashed border'),
            text('cem-css-private button', 'Second DCE dashed border'),
            text('button', 'Browser default border'),
            computedStyleByText('button', 'First DCE dashed border', 'borderTopStyle', 'dashed'),
            computedStyleByText('button', 'First DCE dashed border', 'borderTopColor', 'rgb(0, 128, 0)'),
            computedStyleByText('button', 'First DCE dashed border', 'color', 'rgb(148, 0, 211)'),
            computedStyleByText('button', 'Second DCE dashed border', 'borderTopStyle', 'dashed'),
            computedStyleByText('button', 'Second DCE dashed border', 'borderTopColor', 'rgb(0, 128, 0)'),
            computedStyleByText('button', 'Second DCE dashed border', 'color', 'rgb(148, 0, 211)'),
            computedStyleNotByText('button', 'Browser default border', 'borderTopStyle', 'dashed'),
            computedStyleNotByText('button', 'Browser default border', 'borderTopColor', 'rgb(0, 128, 0)'),
            countExactly('cem-element[tag="cem-css-private"] > style[data-cem-declaration-style="private"]', 1),
            countExactly('cem-css-private style', 0),
            styleTextContains(
                'cem-element[tag="cem-css-private"] > style[data-cem-declaration-style="private"]',
                '@scope (\n    cem-css-private',
            ),
            countExactly('cem-css-private[data-cem-render-scope*="cem-scope-"]', 2),
            countExactly(
                'cem-css-private[data-cem-scope], cem-css-private[data-cem-instance-scope], cem-css-private[scope]',
                0,
            ),

            // Bare-only and explicit-only shared declarations both target the public
            // group boundary and apply to separately declared group peers.
            computedStyle('cem-css-shared-bare .sample-shared-bare', 'color', 'rgb(0, 128, 0)'),
            computedStyle('cem-css-shared-peer .sample-shared-bare', 'color', 'rgb(0, 128, 0)'),
            attributeEquals('cem-css-shared-bare', 'scope', 'css-samples'),
            attributeEquals('cem-css-shared-peer', 'scope', 'css-samples'),
            countExactly('cem-element[tag="cem-css-shared-bare"] > style[data-cem-declaration-style="shared"]', 1),
            styleTextContains(
                'cem-element[tag="cem-css-shared-bare"] > style[data-cem-declaration-style="shared"]',
                '[scope="css-samples"]:has(> template[data-cem-island="instance"])',
            ),
            computedStyle('cem-css-shared-explicit .shared-bg', 'backgroundColor', 'rgb(219, 234, 254)'),
            computedStyle('cem-css-explicit-peer .shared-bg', 'backgroundColor', 'rgb(219, 234, 254)'),
            countExactly('cem-element[tag="cem-css-shared-explicit"] > style[data-cem-declaration-style="shared"]', 1),

            // Matching explicit scope separates the mixed declaration into independent
            // private and shared styles; no combined tag-or-group selector is emitted.
            computedStyle('cem-css-mixed .sample-mixed', 'borderTopColor', 'rgb(0, 0, 255)'),
            computedStyleNot('cem-css-mixed-peer .sample-mixed-shared', 'borderTopColor', 'rgb(0, 0, 255)'),
            computedStyle('cem-css-mixed .sample-mixed-shared', 'color', 'rgb(255, 0, 0)'),
            computedStyle('cem-css-mixed-peer .sample-mixed-shared', 'color', 'rgb(255, 0, 0)'),
            countExactly('cem-element[tag="cem-css-mixed"] > style[data-cem-declaration-style]', 2),
            countExactly('cem-element[tag="cem-css-mixed"] > style[data-cem-declaration-style="private"]', 1),
            countExactly('cem-element[tag="cem-css-mixed"] > style[data-cem-declaration-style="shared"]', 1),
            styleTextContains(
                'cem-element[tag="cem-css-mixed"] > style[data-cem-declaration-style="private"]',
                '@scope (\n    cem-css-mixed',
            ),
            styleTextContains(
                'cem-element[tag="cem-css-mixed"] > style[data-cem-declaration-style="shared"]',
                '[scope="css-samples"]:has(> template[data-cem-island="instance"])',
            ),
            styleTextNotContains(
                'cem-element[tag="cem-css-mixed"] > style[data-cem-declaration-style]',
                'cem-css-mixed, [scope=',
            ),

            // Invalid explicit scopes fail closed. A mismatch does not suppress a valid
            // bare shared shorthand; an invalid declaration scope falls back to private.
            countExactly('cem-element[tag="cem-css-unscoped-explicit"] > style[data-cem-declaration-style]', 0),
            countExactly('cem-element[tag="cem-css-mismatch"] > style[data-cem-declaration-style]', 0),
            computedStyleNot('cem-css-unscoped-explicit .must-not-apply', 'color', 'rgb(255, 0, 0)'),
            computedStyleNot('cem-css-mismatch .must-not-apply', 'color', 'rgb(255, 0, 0)'),
            countExactly('cem-element[tag="cem-css-mismatch-bare"] > style[data-cem-declaration-style="shared"]', 1),
            computedStyle('cem-css-mismatch-bare .sample-valid-bare', 'color', 'rgb(0, 128, 0)'),
            countExactly(
                'cem-element[tag="cem-css-invalid-declaration"] > style[data-cem-declaration-style="private"]',
                1,
            ),
            attributeAbsent('cem-css-invalid-declaration', 'scope'),
            computedStyle('cem-css-invalid-declaration .sample-invalid-declaration', 'color', 'rgb(0, 0, 255)'),

            // An inert instance payload style becomes a managed direct child under an
            // implicit parent-rooted scope; it overrides only that instance.
            computedStyle('cem-css-instance:first-of-type button', 'borderTopColor', 'rgb(0, 0, 255)'),
            computedStyle('cem-css-instance:last-of-type button', 'borderTopColor', 'rgb(255, 0, 0)'),
            countExactly('cem-element[tag="cem-css-instance"] > style[data-cem-declaration-style="private"]', 1),
            countExactly('cem-css-instance:last-of-type style[data-cem-render-node-id^="payload-"]', 1),
            countExactly('cem-css-instance:first-of-type style[data-cem-render-node-id^="payload-"]', 0),
            styleTextContains(
                'cem-css-instance:last-of-type style[data-cem-render-node-id^="payload-"]',
                '@scope to (',
            ),
            styleTextNotContains(
                'cem-css-instance:last-of-type style[data-cem-render-node-id^="payload-"]',
                'data-cem-render-scope',
            ),
            attributeAbsent('cem-css-instance:last-of-type', 'data-cem-instance-scope'),

            // Dynamic declaration styles are rejected, while fragment and anonymous
            // declarations scope static CSS to their effective produced tags.
            countExactly('cem-element[tag="cem-css-dynamic"] > style[data-cem-declaration-style]', 0),
            countExactly('cem-css-dynamic style', 0),
            computedStyleNot('cem-css-dynamic .must-not-apply', 'color', 'rgb(255, 0, 0)'),
            computedStyle('cem-css-fragment .sample-fragment', 'backgroundColor', 'rgb(254, 243, 199)'),
            countExactly('cem-element[tag="cem-css-fragment"] > style[data-cem-declaration-style="private"]', 1),
            styleTextContains(
                'cem-element[tag="cem-css-fragment"] > style[data-cem-declaration-style="private"]',
                '@scope (\n    cem-css-fragment',
            ),
            text('.sample-anonymous', 'anonymous'),
            computedStyle('.sample-anonymous', 'color', 'rgb(238, 130, 238)'),
            countExactly(
                'cem-element[uid-seed="demo/css/anonymous"] > style[data-cem-declaration-style="private"]',
                1,
            ),
            attributeContains('cem-element[uid-seed="demo/css/anonymous"]', 'tag', 'cem-'),

            // uid-seed is absent from ordinary scope samples. The focused keyframe
            // sample proves its internal identity purpose by matching the host render
            // identity to both the rewritten declaration and computed animation name.
            countExactly('cem-element[uid-seed]', 2),
            attributeEquals('cem-element[tag="cem-css-keyframes"]', 'uid-seed', 'demo/css/keyframes'),
            keyframeIdentity(
                'cem-element[tag="cem-css-keyframes"] > style[data-cem-declaration-style="private"]',
                'cem-css-keyframes',
                '[part~="indicator"]',
                'seeded-pulse',
                'udemoz2fcssz2fkeyframes',
            ),
            computedStyle(
                'cem-demo-element[legend="11. Descendant selectors stay inside the component"] label',
                'color',
                'rgb(0, 128, 0)',
            ),
            computedStyle(
                'cem-demo-element[legend="11. Descendant selectors stay inside the component"] [slot="demo"] b',
                'color',
                'rgb(0, 0, 139)',
            ),
            text('cem-css-external-fragment', 'projected external template'),
            computedStyle(
                'cem-css-external-fragment .external-scoped-item',
                'backgroundColor',
                'rgb(254, 243, 199)',
            ),
        ],
    },
    {
        path: '/packages/cem-elements/demo/set-url.html',
        checks: [
            clickThenText(
                'cem-demo-element[legend="1. Set the page hash"] button[value="#hash-two"]',
                'cem-demo-element[legend="1. Set the page hash"] article',
                'Current hash: #hash-two',
            ),
            clickThenText(
                'cem-demo-element[legend="2. Select the URL write method"] button:has-text("history.pushState")',
                'cem-demo-element[legend="2. Select the URL write method"] article',
                'Selected method: history.pushState',
            ),
            clickThenText(
                'cem-demo-element[legend="3. Conditionally inject a URL writer"] button',
                'cem-demo-element[legend="3. Conditionally inject a URL writer"] article',
                'Current hash: #conditional-writer',
            ),
            fillThenText(
                'cem-demo-element[legend="4. Set URL from form controls"] input[type="text"]',
                '#form-verified',
                'cem-demo-element[legend="4. Set URL from form controls"] article',
                'Pending: history.pushState = #form-verified',
            ),
            clickThenText(
                'cem-demo-element[legend="4. Set URL from form controls"] form > button',
                'cem-demo-element[legend="4. Set URL from form controls"] article',
                'Current hash: #form-verified',
            ),
        ],
    },
];

const sourceDocumentSpecs = [
    { path: '/packages/cem-elements/demo/cell-overrides.html', samples: cellOverrideSamples },
    {
        path: '/packages/cem-elements/demo/table-inspector.html',
        samples: tableInspectorSamples,
    },
    { path: '/packages/cem-elements/demo/data-tree.html', samples: dataTreeSamples },
    { path: '/packages/cem-elements/demo/data-table.html', samples: dataTableSamples },
    {
        path: '/packages/cem-elements/index.html',
        samples: [
            sampleContract('1. simple payload', [text('dce-link a', 'link')]),
            sampleContract('2. payload with slot definition and slot value', [text('dce-1-slot', '🐇❤️'), text('dce-1-slot', '🥕')]),
            sampleContract('2a. payload with slot definition and slot value', [
                countExactly('dce-2-slots', 2),
                attributeEquals('dce-2-slots:first-of-type input', 'placeholder', '🐇❤️🥕'),
                attributeEquals('dce-2-slots:last-of-type input', 'placeholder', '🐇❤️🐇'),
            ]),
            sampleContract('2b. named default slot', [
                normalizedText('dce-3-slot:nth-of-type(1)', '1 😃 2 😃'),
                normalizedText('dce-3-slot:nth-of-type(2)', '1 🥕 2 🥕'),
                normalizedText('dce-3-slot:nth-of-type(3)', '1 ✌️ 2 ✌️'),
            ]),
            sampleContract('2c. named default slot', [normalizedText('dce-4-slot', '1 🥕 2 🥕')]),
            sampleContract('2d. default slot', [
                normalizedText('greet-element:first-of-type', 'Hello World!'),
                normalizedText('greet-element:last-of-type', '👋 World!'),
            ]),
            sampleContract('3. 💪 DCE template', [
                countExactly('pokemon-tile[title="bulbasaur"] button', 2),
                countExactly('pokemon-tile[title="ninetales"] button', 1),
                countExactly('pokemon-tile button img:not([alt]), pokemon-tile button img[alt=""]', 0),
                text('pokemon-tile[title="bulbasaur"] h3', 'bulbasaur'),
                text('pokemon-tile[title="bulbasaur"]', 'Smile as: 👼'),
                attributeEquals(
                    'pokemon-tile img[alt="bulbasaur image"]',
                    'src',
                    'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/1.svg',
                ),
                attributeEquals(
                    'pokemon-tile button img[alt="ivysaur"]',
                    'src',
                    'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/2.svg',
                ),
                text('pokemon-tile button', 'venusaur'),
                text('pokemon-tile button', 'vulpix'),
            ]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/attributes.html',
        samples: [
            sampleContract('1. attributes definition', [
                text('#defaults-1 article.demo-card', 'p1: default_P1'),
                text('#defaults-1 article.demo-card', 'p2: always_p2'),
                text('#defaults-1 article.demo-card', 'p3: def_P3'),
                attributeEquals('#defaults-1', 'p1', 'default_P1'),
                attributeEquals('#defaults-1', 'p2', 'always_p2'),
                attributeEquals('#defaults-1', 'p3', 'def_P3'),
            ]),
            sampleContract('1a. External attribute changes', [
                text('#defaults-2 article.demo-card', 'p1: 123'),
                clickThenText('button[aria-label="set p1"]', '#defaults-2', 'p1: changed p1'),
                clickThenText('button[aria-label="set p3"]', '#defaults-2', 'p3: changed p3'),
                clickThenText('button[aria-label="remove p3"]', '#defaults-2', 'p3: def_P3'),
                clickThenText('button[aria-label="set p2"]', '#defaults-2 article.demo-card', 'p2: always_p2'),
                attributeEquals('#defaults-2', 'p2', 'always_p2'),
                fillThenText('#p3-input', '', '#defaults-2 article.demo-card p:last-of-type', 'p3: def_P3'),
                clickThenText('button[aria-label="set p3"]', '#defaults-2 article.demo-card p:last-of-type', 'p3:'),
                normalizedText('#defaults-2 article.demo-card p:last-of-type', 'p3:'),
                attributeEquals('#defaults-2', 'p3', ''),
                clickThenText('button[aria-label="remove p3"]', '#defaults-2', 'p3: def_P3'),
                attributeEquals('#defaults-2', 'p3', 'def_P3'),
            ]),
            sampleContract('1b. Container attribute values', [
                text('#defaults-3 article.demo-card', 'p3: qwe'),
            ]),
            sampleContract('2. attribute from slice', [
                text('#title-from-slice article.demo-card', 'title attribute: 😃'),
                attributeEquals('#title-from-slice', 'title', '😃'),
                fillThenText('#title-from-slice input', 'Typed title', '#title-from-slice article.demo-card', 'title attribute: Typed title'),
                attributeEquals('#title-from-slice', 'title', 'Typed title'),
            ]),
            sampleContract('3. V attribute matches input value', [
                text('#value-default article.demo-card', 'v: def'),
                attributeEquals('#value-default', 'is-changed', 'false'),
                fillThenText('#value-default input', 'From input', '#value-default article.demo-card', 'v: From input'),
                attributeEquals('#value-default', 'v', 'From input'),
                attributeEquals('#value-default', 'is-changed', 'true'),
            ]),
            sampleContract('3a. Container value before input', [
                text('#value-container', 'v: V1'),
                fillThenText('#value-container input', '', '#value-container', 'is-changed: true'),
                attributeEquals('#value-container', 'v', ''),
            ]),
            sampleContract('4. attribute defaults, from container, and from slice', [
                text('#precedence-default article.demo-card', 'datadom.attributes.v: def'),
                text('#precedence-default article.demo-card', 'effective value: def'),
                attributeEquals('#precedence-default', 'v', 'def'),
                typeThenText(
                    '#precedence-default input',
                    'qqq',
                    '#precedence-default article.demo-card',
                    'datadom.attributes.v: qqq',
                ),
                text('#precedence-default article.demo-card', 'effective value: qqq'),
                text('#precedence-default article.demo-card', 'has-input: true'),
                attributeEquals('#precedence-default', 'v', 'qqq'),
            ]),
            sampleContract('4a. External changes versus user input', [
                text('#precedence-container', 'effective value: From Container'),
                clickThenText('button', '#precedence-container', 'effective value: External update'),
                fillThenText('#precedence-container input', 'Own value', '#precedence-container', 'effective value: Own value'),
                clickThenText('button', '#precedence-container', 'effective value: Own value'),
                attributeEquals('#precedence-container', 'v', 'Own value'),
            ]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/data-slices.html',
        samples: dataSliceSamples,
    },
    {
        path: '/packages/cem-elements/demo/xpath-nodes.html',
        samples: xpathNodeSamples,
    },
    {
        path: '/packages/cem-elements/demo/xpath-sequences.html',
        samples: xpathSequenceSamples,
    },
    {
        path: '/packages/cem-elements/demo/xpath-maps-arrays.html',
        samples: xpathMapArraySamples,
    },
    {
        path: '/packages/cem-elements/demo/xpath-validation.html',
        samples: xpathValidationSamples,
    },
    {
        path: '/packages/cem-elements/demo/xpath-sort.html',
        samples: xpathSortSamples,
    },
    {
        path: '/packages/cem-elements/demo/xpath-aggregates.html',
        samples: xpathAggregateSamples,
    },
    {
        path: '/packages/cem-elements/demo/xpath-functions.html',
        samples: [
            sampleContract('1. Named XPath function', [
                normalizedText('output', 'Hello 🍒'),
                fillThenText('input', 'Changed', 'output', 'Changed 🍒'),
                fillThenText('input', '', 'output', '🍒'),
            ]),
            sampleContract('1a. CEM-QL string pair', [
                normalizedText('output', 'Hello 🍒'),
                fillThenText('input', 'Changed', 'output', 'Changed 🍒'),
                fillThenText('input', '', 'output', '🍒'),
            ]),
            sampleContract('2. Shared XPath predicate', [
                normalizedText('output', 'cherry 🍒'),
                fillThenText('input', 'lemon', 'output', 'Try cherry'),
                fillThenText('input', 'cherry', 'output', 'cherry 🍒'),
            ]),
            sampleContract('2a. CEM-QL predicate pair', [
                normalizedText('output', 'cherry 🍒'),
                fillThenText('input', 'lemon', 'output', 'Try cherry'),
                fillThenText('input', 'cherry', 'output', 'cherry 🍒'),
            ]),
            sampleContract('3. XML nodes and matching', [
                normalizedText('li', 'Cherry : stocked'),
                fillThenText('textarea', '<r><item qty="1">Lemon</item></r>', 'li', 'Lemon : low stock'),
                fillThenText('textarea', '<r><item qty="3">Grape</item></r>', 'li', 'Grape : stocked'),
            ]),
            sampleContract('3a. CEM-QL native node pair', [
                normalizedText('li', 'Cherry : stocked'),
                fillThenText('textarea', '<r><item qty="1">Lemon</item></r>', 'li', 'Lemon : low stock'),
                fillThenText('textarea', '<r><item qty=" 2.5 ">Grape</item></r>', 'li', 'Grape : stocked'),
            ]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/dom-merge.html',
        checks: wordCountEdgeChecks,
        samples: [
            sampleContract('1. Textarea word count', [
                text('h2', 'Textarea word count'),
                fillBlurThenText('textarea', 'one two three', 'form > p strong', '3'),
            ]),
            sampleContract('2. Input word and character count', [
                text('h2', 'Input word and character count'),
                fillThenText('input', 'two words', 'output', 'two words'),
                text('form > p:first-of-type strong', '9'),
                text('form > p:nth-of-type(2) strong', '2'),
            ]),
            xpathWordSample,
        ],
    },
    { path: '/packages/cem-elements/demo/embed-1.html', checks: [text('h4', 'embed-1.html'), text(':scope', '🖖')] },
    {
        path: '/packages/cem-elements/demo/embed-lib.html#embed-lib-component',
        checks: [text(':scope', '👋 from embed-lib-component')],
    },
    {
        path: '/packages/cem-elements/demo/external-template-document.html',
        checks: [text('h2', 'External document'), text('p', 'External document fallback')],
    },
    {
        path: '/packages/cem-elements/demo/external-template-templates.html#external-card-template',
        attributes: { title: 'Source-loaded card' },
        content: 'Projected source content',
        checks: [text('h2', 'Source-loaded card'), text('p', 'Projected source content')],
    },
    {
        path: '/packages/cem-elements/demo/external-template.html',
        allowedPageErrors: ['Failed to load resource: the server responded with a status of 404 (Not Found)'],
        samples: [
            sampleContract('1. reference the template in page DOM', [text('dce-internal:first-of-type', '👋 World!'), text('dce-internal:last-of-type', 'Hello World!')]),
            sampleContract('2. without TAG, inline instantiation', [
                countExactly('cem-element[src="#template2"]', 2),
                text('cem-element[src="#template2"]', 'construction'),
            ]),
            sampleContract('3. external SVG file', [countAtLeast('dce-external svg', 1), svgUseReferences('dce-external svg', ['#h', '#j'])]),
            sampleContract('3a. Anonymous external SVG', [countAtLeast('cem-element[src="confused.svg"] svg', 1), svgUseReferences('cem-element[src="confused.svg"] svg', ['#h', '#j'])]),
            sampleContract('3b. Missing source fallback', [text('dce-external-missing', 'fallback for missing image')]),
            sampleContract('4. external CEM-ML template file', [
                text('dce-external-4', 'External CEMT data-island transformation'),
                text('dce-external-4', 'template[data-cem-island="instance"]'),
                text('dce-external-4', 'cem-island:context-root'),
                text('dce-external-4', 'cem-hydration:data'),
                text('dce-external-4', 'cem-attributes:attributes'),
                text('dce-external-4', 'cem-dataset:dataset'),
                text('dce-external-4', 'cem-payload:payload'),
                text('dce-external-4', 'cem-slices:slices'),
                text('dce-external-4', 'cem-resources:resources'),
                text('dce-external-4', 'cem-form:form-state'),
                text('dce-external-4', 'cem-validation:validation-state'),
                text('dce-external-4', 'cem-events:event-state'),
                text('dce-external-4', 'Payload comment: explicit inert envelope follows'),
                text('dce-external-4', 'DCE with complete external CEMT island'),
                text('dce-external-4', 'wrapped-payload'),
                text('dce-external-4', 'slot="heading"'),
                text('dce-external-4', 'slot=""'),
                text('dce-external-4', 'data-fruit="🍌"'),
                text('dce-external-4', 'aria-label="Fruit choice"'),
                text('dce-external-4', 'Every element, attribute, dataset entry, and text node is data.'),
                countAtLeast('dce-external-4 details', 20),
            ]),
            sampleContract('4a. Live HTML payload capture', [
                text('dce-external-4-inline', 'A second external-CEMT data island'),
                text('dce-external-4-inline', 'DCE with live payload capture'),
                text('dce-external-4-inline', 'name="data-smile"'),
                text('dce-external-4-inline', 'name="data-basket"'),
                text('dce-external-4-inline', 'data-kind="live-payload"'),
            ]),
            sampleContract('4b. CEM-ML source payload', [
                text('dce-external-4-cem-ml', 'content-type="text/cem-ml"'),
                text('dce-external-4-cem-ml', 'Banana from CEM-ML payload source'),
            ]),
            sampleContract('5. external HTML template', [
                text('dce-external-5', '👋'),
                text('dce-external-5', '👌'),
                countAtLeast('dce-external-5 svg', 1),
                countAtLeast('dce-external-5 math', 1),
            ]),
            sampleContract('5a. Anonymous external HTML', [
                attributeEquals('#dce-external-5-inline', 'data-cem-anonymous-declaration', ''),
                text('#dce-external-5-inline > [data-cem-anonymous-instance]', '👋'),
                text('#dce-external-5-inline > [data-cem-anonymous-instance]', '👌'),
                countAtLeast('#dce-external-5-inline > [data-cem-anonymous-instance] svg', 1),
                countAtLeast('#dce-external-5-inline > [data-cem-anonymous-instance] math', 1),
            ]),
            sampleContract('6. HTML, SVG by ID within external file', [text('dce-html-wave', '👋')]),
            sampleContract('6a. SVG fragment by ID', [countAtLeast('dce-html-logo svg', 1)]),
            sampleContract('6b. MathML fragment by ID', [countAtLeast('dce-html-formula math', 1)]),
            sampleContract('7a. external CEM-ML data-island tree template', [
                text('dce-cemt-tree', 'CEM-ML data island tree'),
                text('dce-cemt-tree article.demo-card > details > summary > b', 'catalog'),
                text('dce-cemt-tree', 'Leaf text from cem-elements data island'),
                countAtLeast('dce-cemt-tree details', 4),
            ]),
            sampleContract('7b. External XSLT XML payload tree', [
                text('dce-xslt-tree', 'XSLT XML payload tree'),
                text('dce-xslt-tree article.demo-card > details > summary > b', 'catalog'),
                text('dce-xslt-tree', 'Leaf text from cem-elements XSLT data island'),
                countAtLeast('dce-xslt-tree details', 4),
            ]),
            sampleContract('7c. Missing fragment fallback', [
                text('dce-missing-none', 'element with id=none is missing in template'),
            ]),
            ...xsltVariantSamples,
            sampleContract('8. external file with embedding of another external DCE', [text('dce-embed-1', '🖖')]),
            sampleContract('9. external file with invoking of relative template as hash by enclosed custom-element', [
                text('dce-embed-relative-hash', 'from embed-lib-component'),
                attributeContains(
                    'dce-embed-relative-hash img',
                    'src',
                    '/packages/cem-elements/demo/lib-dir/Smiley.svg',
                ),
            ]),
            sampleContract('10. external file with invoking of template in another relative path file by enclosed custom-element', [text('dce-embed-relative-file', '🖖')]),
            sampleContract('embed-1.html external file', [attributeEquals(':scope', 'src', './embed-1.html'), attributeEquals(':scope', 'type', 'html'), attributeEquals(':scope', 'demo', 'false')]),
            sampleContract('embed-lib.html with multiple templates', [attributeEquals(':scope', 'src', './embed-lib.html'), attributeEquals(':scope', 'type', 'html'), attributeEquals(':scope', 'demo', 'false')]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/for-each.html',
        samples: [
            sampleContract('1. Simple for-each', [
                countExactly('li', 3),
                text('li', '🍏'),
            ]),
            sampleContract('2. for-each with position()', [
                text('article', '1 . Red'),
                text('article', '3 . Blue'),
            ]),
            sampleContract('3. Conditional for-each', [
                text('article', 'BEFORE'),
                text('article', 'AFTER'),
                clickThenText('input[type="checkbox"]', 'span', '1 : First'),
            ]),
            sampleContract('4. Nested for-each table', [
                countExactly('tbody tr', 3),
                text('tbody', 'B2'),
            ]),
            sampleContract('5. for-each with attributes', [
                text('article', '# 1 Alice ( admin )'),
                text('article', '# 3 Charlie ( viewer )'),
            ]),
            sampleContract('6. Dynamic table with toggle', [
                clickThenText('input[type="checkbox"]', 'tbody', 'Widget'),
            ]),
            sampleContract('7. for-each over payload data', [
                text('cem-loop-payload .payload-feed', 'payload-alpha : Payload Alpha'),
                text('cem-loop-payload .payload-feed', 'payload-beta : Payload Beta'),
            ]),
            sampleContract('8. for-each over location data', [
                text('.location-feed', 'topic = feeds'),
                text('.location-feed', 'item = payload,resource'),
            ]),
            sampleContract('9. for-each over HTTP JSON/XML data', [
                text('cem-loop-http output[data-role="json-state"]', 'loaded'),
                text('cem-loop-http .http-json-feed', 'beta : loaded'),
                text('cem-loop-http output[data-role="xml-state"]', 'loaded'),
                text('cem-loop-http .http-xml-feed', 'delta : xml-loaded'),
            ]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/form.html',
        samples: [
            sampleContract('1. Simple validation', [
                fillThenText('input[name="username"]', 'long-username', 'form', 'Username: long-username'),
                countExactly('input[name="password"]', 0),
                clickThenText('button', 'form', '🔑'),
                fillThenText('input[name="password"]', 'secret', 'form > p:nth-of-type(2) output', 'true'),
            ]),
            sampleContract('2. Form lifecycle', [
                fillThenText('input[name="username"]', 'long-username', 'form > p:nth-of-type(2) output', 'long-username'),
                clickThenText('input[value="sms"]', 'fieldset', 'Message and data rates may apply.'),
                clickThenText('input[value="password"]', 'form > p:nth-of-type(3) output', 'password'),
                fillThenText('input[name="password"]', 'secret', 'form > p:nth-of-type(4) output', 'true'),
            ]),
            sampleContract('3. Native control validity message', [
                fillThenText('input[name="email"]', '', 'form > p:nth-of-type(2) output', 'Please fill out this field.'),
            ]),
            sampleContract('4. Form custom validity message', [
                fillThenText('input[name="email"]', 'abc', 'form > p:nth-of-type(4) output', 'Use more than 3 characters'),
                text('form > p:nth-of-type(2) output', '3'),
            ]),
            sampleContract('5. DCE as a form input', [
                clickThenText(
                    'cem-form-fruit-choice:first-of-type button[data-option-index="1"]',
                    'form > p:first-of-type output:first-of-type',
                    '🍏',
                ),
                clickThenText(
                    'cem-form-fruit-choice:last-of-type button[data-option-index="2"]',
                    'form > p:nth-of-type(3) output',
                    'Choose the same fruit',
                ),
                clickThenText(
                    'cem-form-fruit-choice:last-of-type button[data-option-index="1"]',
                    'form > p:nth-of-type(2) output',
                    'true',
                ),
            ]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/hex-grid.html',
        samples: [
            sampleContract('1. Responsive framework link honeycomb', [
                countExactly('cem-hex-grid .hex-link', 14),
                countExactly('cem-hex-grid .hex-logo', 14),
                text('cem-hex-grid .hex:last-child .hex-label', 'Next.js'),
                attributeContains('.hex:first-child .hex-link', 'href', '/demo/module-url.html'),
                attributeEquals('.hex:nth-child(2) .hex-link', 'href', 'https://react.dev/'),
                attributeEquals(
                    '.hex:nth-child(2) .hex-logo',
                    'src',
                    'https://upload.wikimedia.org/wikipedia/commons/a/a7/React-icon.svg',
                ),
                attributeContains('.hex:first-child .hex-logo', 'src', '/demo/framework-logos/wc-square.svg'),
                computedStyle('.hex-grid', 'display', 'flex'),
                computedStyleNot('.hex-link', 'clipPath', 'none'),
            ]),
            sampleContract('2. Compact percentage links', [
                countExactly('.hex-link', 6),
                attributeEquals(
                    'cem-hex-grid',
                    'size',
                    '65%',
                ),
            ]),
            sampleContract('3. Fixed-length links', [
                countExactly('.hex-link', 6),
                attributeEquals(
                    'cem-hex-grid',
                    'size',
                    '35rem',
                ),
            ]),
            sampleContract('4. Alternating backgrounds', [
                countExactly('.hex-link', 3),
                attributeEquals('cem-hex-grid', 'alternate', 'true'),
                attributeContains('.hex:nth-child(2)', 'class', 'hex-true'),
            ]),
            sampleContract('5. Wrapping long label', [
                countExactly('.hex-link', 1),
                text('.hex-label', 'Declarative Custom Element Framework With A Long Name'),
            ]),
            sampleContract('6. Missing-image fallback', [
                countExactly('.hex-link', 1),
                computedStyle('.hex-logo', 'opacity', '0'),
                computedStyle('.image-fallback', 'visibility', 'visible'),
            ]),
            sampleContract('7. Wrapper DCE theme', [
                countExactly('.hex-link', 3),
                computedStyle('.hex-label', 'color', 'rgb(30, 27, 75)'),
                computedStyle('.theme-frame', 'backgroundColor', 'rgb(238, 242, 255)'),
            ]),
            sampleContract('8. Image-button presentation', [
                countExactly('.hex-link', 1),
                computedStyleNot('.hex-link', 'filter', 'none'),
                computedStyle('.hex-label', 'transform', 'matrix(1, 0, 0, 1, 0, 0)'),
            ]),
            hexRowSample,
        ],
    },
    { path: '/packages/cem-elements/demo/html-template.html', checks: [text('#wave', '👋'), text('#ok', '👌'), countExactly('#dwc-logo', 1), countExactly('#sophomores-dream', 1)] },
    {
        path: '/packages/cem-elements/demo/http-request.html',
        samples: [
            sampleContract('0. URL from text to http-request', [
                text('article', 'Request state: idle'),
                clickThenText('article > button', 'article', 'Request state: loaded'),
                text('li', 'beta : loaded'),
                clickThenText(
                    '.url-presets button:nth-of-type(2)',
                    'article',
                    'Selected URL: ./http-data-compact.json',
                ),
                clickThenText('article > button', 'li', 'solo : compact'),
                clickThenText('button[aria-label="Invalid JSON response"]', 'article', 'Selected URL: ./http-data-invalid.json'),
                text('article', 'Request state: loaded'),
                clickThenText('article > button', 'article', 'Request state: failed'),
                countExactly('li', 0),
                clickThenText('button:has-text("All records")', 'article', 'Selected URL: ./http-data.json'),
                clickThenText('article > button', 'li', 'beta : loaded'),
                clickThenText('button[aria-label="Empty URL"]', 'article', 'Selected URL:'),
                attributeAbsent('input[type="text"]', 'value'),
                clickThenText('article > button', 'article', 'Request state: idle'),
                countExactly('li', 0),
            ]),
            sampleContract('1. Simplest http-request', [
                countExactly('.result-buttons button', 6),
                countExactly('.result-buttons span', 0),
                attributeEquals('.result-buttons button:first-of-type', 'aria-label', 'bulbasaur'),
                attributeEquals(
                    '.result-buttons button:first-of-type img',
                    'src',
                    'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/1.svg',
                ),
                attributeEquals(
                    '.result-buttons button:last-of-type img',
                    'src',
                    'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/6.svg',
                ),
            ]),
            sampleContract('2. http-request response and headers', [
                text('dl dd:nth-of-type(2)', 'GET'),
                text('dl dd:nth-of-type(5)', 'application/json'),
                text('dl dd:nth-of-type(6)', 'ported-from-legacy'),
                text('dl dd:nth-of-type(7)', '200'),
                text('dl dd:nth-of-type(8)', 'application/json'),
                attributeContains('dl a', 'href', '/packages/cem-elements/demo/http-data.json'),
                shortenedHrefText('dl a', 32),
            ]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/lib-dir/embed-lib.html#embed-lib-component',
        checks: [text(':scope', '👋 from embed-lib-component')],
    },
    {
        path: '/packages/cem-elements/demo/local-storage.html',
        samples: localStorageSamples,
    },
    {
        path: '/packages/cem-elements/demo/location-element.html',
        samples: [
            sampleContract('1. Window location live update', [
                clickThenText('button:has-text("history.pushState")', 'dl', '#checked'),
                text('ul', 'mode = history.pushState'),
                text('ul', 'tag = one,two'),
            ]),
            sampleContract('2. Window location initial read', [text('dl', 'source window')]),
            sampleContract('3. External URL from href', [
                text('dl', 'hostname my.example'),
                text('dl', 'pathname /docs'),
                text('dl', 'hash #details'),
                text('ul', 'a = 1'),
                text('ul', 'b = 2,3'),
            ]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/module-url.html',
        samples: [
            sampleContract('this page import maps', [text(':scope', '"lib-root"'), text(':scope', '"embed-lib"')]),
            sampleContract('1. module path by symbolic name', [
                attributeContains('image-link', 'src', '/packages/cem-elements/demo/wc-square.svg'),
                attributeContains('image-link img', 'src', '/packages/cem-elements/demo/wc-square.svg'),
                attributeContains('image-link a', 'href', '/packages/cem-elements/demo/wc-square.svg'),
                shortenedHrefText('image-link a', 32),
            ]),
            sampleContract('2. src forms: relative URL', [
                attributeContains('image-link', 'src', 'Smiley.svg?src=relative'),
                attributeContains('image-link img', 'src', 'Smiley.svg?src=relative'),
                shortenedHrefText('image-link a', 32),
            ]),
            sampleContract('3. src forms: absolute URL', [
                attributeContains('image-link', 'src', 'data:image/svg+xml,'),
                attributeContains('image-link img', 'src', 'data:image/svg+xml,'),
                shortenedHrefText('image-link a', 32),
            ]),
            sampleContract('4. Relative declaration source', [
                text('output', '/packages/cem-elements/demo/embed-1.html'),
            ]),
            sampleContract('4a. Mapped declaration source', [
                text('output', '/packages/cem-elements/demo/lib-dir/embed-lib.html'),
            ]),
            sampleContract('4b. Missing import-map entry', [
                normalizedText('output', 'not published'),
                text('article', 'cem-element.module_url_resolve_failed'),
            ]),
            sampleContract('4c. Mapped fragment with a relative dependency', [
                text('output', '/packages/cem-elements/demo/lib-dir/embed-lib.html#embed-relative-file'),
            ]),
            mappedImageFragmentSample,
            sampleContract('5. component-local map: naked', [
                attributeContains('cem-local-map-naked-image img.component-owned-image', 'src', 'Smiley.svg?owner=component'),
                shortenedHrefText('cem-local-map-naked-image image-link a', 32),
            ]),
            sampleContract('6. component-local map: wrapper override', [
                attributeContains('cem-local-map-override-wrapper img.component-owned-image', 'src', 'confused.svg?owner=wrapper'),
                shortenedHrefText('cem-local-map-override-wrapper image-link a', 32),
            ]),
            sampleContract('7. component-local map: node referrer', [
                attributeContains('cem-local-map-referrer-demo img.node-referrer-image', 'src', 'wc-square.svg?owner=component'),
                shortenedHrefText('cem-local-map-referrer-demo image-link.node-referrer-image a', 32),
                text('cem-local-map-referrer-demo table.node-referrer-matrix td:first-of-type', 'Smiley.svg?referrer=node'),
                text('cem-local-map-referrer-demo table.node-referrer-matrix td:nth-of-type(2)', 'wc-square.svg?owner=component'),
                text('cem-local-map-referrer-demo table.node-referrer-matrix td:last-of-type', 'https://assets.example.test/logo.svg'),
                countExactly('cem-module-url', 0),
            ]),
            sampleContract('image-link', [shortenedHrefText('image-link a', 32)]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/module-url-referrer.html',
        samples: [
            sampleContract('src by scalar referrer matrix', [
                text('tbody tr:first-of-type td:first-of-type', 'Smiley.svg?case=relative-relative'),
                text('tbody tr:first-of-type td:nth-of-type(2)', 'Smiley.svg?referrer=relative'),
                text('tbody tr:nth-of-type(2) td:nth-of-type(2)', 'confused.svg?referrer=module'),
                text('tbody tr:last-of-type td:first-of-type', 'https://referrer.example.test/lib-dir/Smiley.svg?case=relative-absolute'),
                text('tbody tr:last-of-type td:nth-of-type(2)', 'wc-square.svg?referrer=absolute'),
                countExactly('cem-module-url', 0),
            ]),
        ],
    },
    { path: '/packages/cem-elements/demo/functions/dom.html', samples: domChainSamples },
    {
        path: '/packages/cem-elements/demo/functions/str.html',
        samples: [
            sampleContract('str:shorten query/result matrix', [
                normalizedText('cem-str-shorten-matrix tbody tr:first-of-type td:nth-of-type(2)', 'short'),
                normalizedText('cem-str-shorten-matrix tbody tr:nth-of-type(2) td:nth-of-type(2)', 'abc…hij'),
                normalizedText('cem-str-shorten-matrix tbody tr:nth-of-type(3) td:nth-of-type(2)', 'abc…ghij'),
                normalizedText('cem-str-shorten-matrix tbody tr:nth-of-type(4) td:nth-of-type(2)', 'ab...hij'),
                normalizedText('cem-str-shorten-matrix tbody tr:nth-of-type(5) td:nth-of-type(2)', 'abchij'),
                normalizedText('cem-str-shorten-matrix tbody tr:nth-of-type(6) td:nth-of-type(2)', 'αβ💠ζη'),
                normalizedText('cem-str-shorten-matrix tbody tr:last-of-type td:nth-of-type(2)', 'https://example…emantic-card.cem'),
            ]),
            ...stringMethodSamples,
        ],
    },
    {
        path: '/packages/cem-elements/demo/npm-versions-demo.html',
        samples: [
            sampleContract('1. Default to the latest version', [
                countExactly('select option', 4),
                normalizedText('select option:first-of-type', '0.1.0'),
            ]),
            sampleContract('2. Preselect a version and show dates', [
                propertyEquals('select', 'value', '0.0.22'),
                text('option[value="0.0.22"]', '2024-04-20'),
            ]),
            sampleContract('3. Propagate the selected value', [
                selectThenText('select', '0.0.25', 'output', '0.0.25'),
                attributeEquals('cem-npm-version-propagated', 'value', '0.0.25'),
            ]),
            sampleContract('4. Override the label slot', [
                text('label', 'Select a release:'),
                selectThenText('select', '0.0.21', 'output', '0.0.21'),
            ]),
            sampleContract('5. Synchronize the selected version with the URL', [
                clickThenText('button[aria-label="Set URL to 0.0.22"]', 'article', 'Current hash: #version=0.0.22'),
                propertyEquals('select', 'value', '0.0.22'),
                selectThenText('select', '0.1.0', 'article', 'selected-version slice: 0.1.0'),
                text('article', 'Current hash: #version=0.1.0'),
                clickThenText('button[aria-label="Set URL to 0.0.25"]', 'article', 'Current hash: #version=0.0.25'),
                propertyEquals('select', 'value', '0.0.25'),
                selectThenText('select', '0.1.0', 'article', 'Current hash: #version=0.1.0'),
            ]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/scoped-css.html',
        samples: [
            sampleContract('1. Private declaration CSS and ordinary outer cascade', [countExactly('cem-css-private button', 2), computedStyleByText('cem-css-private button', 'First DCE dashed border', 'borderTopStyle', 'dashed'), computedStyleByText('cem-css-private button', 'First DCE dashed border', 'color', 'rgb(148, 0, 211)'), computedStyleNotByText('button', 'Browser default border', 'borderTopStyle', 'dashed')]),
            sampleContract('2. Component in a named scope shares default declaration styles with peers in the same scope', [computedStyle('cem-css-shared-bare .sample-shared-bare', 'color', 'rgb(0, 128, 0)'), computedStyle('cem-css-shared-peer .sample-shared-bare', 'color', 'rgb(0, 128, 0)')]),
            sampleContract('3. Style can be scoped explicitly', [computedStyle('cem-css-shared-explicit .shared-bg', 'backgroundColor', 'rgb(219, 234, 254)'), computedStyle('cem-css-explicit-peer .shared-bg', 'backgroundColor', 'rgb(219, 234, 254)')]),
            sampleContract('4. Mixed private and shared styles', [computedStyle('cem-css-mixed .sample-mixed', 'borderTopColor', 'rgb(0, 0, 255)'), computedStyle('cem-css-mixed-peer .sample-mixed-shared', 'color', 'rgb(255, 0, 0)')]),
            sampleContract('5. Invalid and mismatched scopes fail closed', [computedStyleNot('cem-css-unscoped-explicit .must-not-apply', 'color', 'rgb(255, 0, 0)'), computedStyle('cem-css-mismatch-bare .sample-valid-bare', 'color', 'rgb(0, 128, 0)'), computedStyle('cem-css-invalid-declaration .sample-invalid-declaration', 'color', 'rgb(0, 0, 255)')]),
            sampleContract('6. Payload style belongs to one instance', [computedStyle('cem-css-instance:first-of-type button', 'borderTopColor', 'rgb(0, 0, 255)'), computedStyle('cem-css-instance:last-of-type button', 'borderTopColor', 'rgb(255, 0, 0)')]),
            sampleContract('7. Declaration styles must be static', [countExactly('cem-css-dynamic style', 0), computedStyleNot('cem-css-dynamic .must-not-apply', 'color', 'rgb(255, 0, 0)')]),
            sampleContract('8. Fragment template CSS uses the effective produced tag', [computedStyle('cem-css-fragment .sample-fragment', 'backgroundColor', 'rgb(254, 243, 199)')]),
            sampleContract('9. Anonymous declaration CSS uses its generated tag', [text('.sample-anonymous', 'anonymous violet'), computedStyle('.sample-anonymous', 'color', 'rgb(238, 130, 238)')]),
            sampleContract('10. uid-seed stabilizes keyframe names', [keyframeIdentity('cem-element[tag="cem-css-keyframes"] > style[data-cem-declaration-style="private"]', 'cem-css-keyframes', '[part~="indicator"]', 'seeded-pulse', 'udemoz2fcssz2fkeyframes')]),
            sampleContract('11. Descendant selectors stay inside the component', [
                computedStyle('label', 'color', 'rgb(0, 128, 0)'),
                computedStyle('[slot=demo] b', 'color', 'rgb(0, 0, 139)'),
            ]),
            sampleContract('12. CSS from an external template fragment', [
                text('cem-css-external-fragment', 'projected external template'),
                computedStyle('.external-scoped-item', 'backgroundColor', 'rgb(254, 243, 199)'),
            ]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/set-url.html',
        samples: [
            sampleContract('1. Set the page hash', [
                clickThenText('button[value="#hash-two"]', 'article', 'Current hash: #hash-two'),
            ]),
            sampleContract('2. Select the URL write method', [
                countExactly('button', 6),
                clickThenText('button:has-text("history.pushState")', 'article', 'Selected method: history.pushState'),
            ]),
            sampleContract('3. Conditionally inject a URL writer', [
                clickThenText('button', 'article', 'Current hash: #conditional-writer'),
            ]),
            sampleContract('4. Set URL from form controls', [
                fillThenText('input[type="text"]', '#form-verified', 'article', 'Pending: history.pushState = #form-verified'),
                clickThenText('form > button', 'article', 'Current hash: #form-verified'),
                fillThenText('input[type="text"]', '#next-draft', 'article', 'Pending: history.pushState = #next-draft'),
                text('article', 'Current hash: #form-verified'),
                clickThenText('form > button', 'article', 'Current hash: #next-draft'),
            ]),
        ],
    },
];

// External source cards are part of both the standalone and source-loaded inventories.
for (const [directory, page, files] of [
    ['cem-elements', 'http-request.html', ['http-data.json', 'http-data-compact.json', 'http-data-invalid.json', 'http-pokemon.json']],
    ['cem-elements', 'for-each.html', ['http-data.json', 'http-data.xml']],
    ['cem-elements', 'data-tree.html', ['tree-source.xml', 'tree-source.json']],
    ['cem-elements', 'npm-versions-demo.html', ['npm-versions.json']],
    ['custom-element', 'http-request.html', ['http-data.json', 'http-data-compact.json', 'http-data-invalid.json', 'http-pokemon.json']],
    ['custom-element', 'npm-versions-demo.html', ['npm-versions.json']],
]) {
    const path = `/packages/${directory}/demo/${page}`;
    const previews = files.map(file => sampleContract(file, [
        attributeEquals(':scope', 'src', `./${file}`),
        attributeEquals(':scope', 'type', file.endsWith('.xml') ? 'xml' : 'json'),
        attributeEquals(':scope', 'demo', 'false'),
        countExactly('[slot="demo"] > *', 0),
    ]));
    const checks = previews.flatMap(sample => sample.checks.map(check =>
        scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`)));
    const fixture = fixtureSpecs.find(fixture => fixture.path === path);
    if (fixture) fixture.checks.push(...checks);
    else fixtureSpecs.push({ path, checks });
    const source = sourceDocumentSpecs.find(source => source.path === path);
    if (source) source.samples.push(...previews);
    else sourceDocumentSpecs.push({ path, samples: previews });
}

const sourceHarnessHtml = `<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <script type="importmap">
        {
            "imports": {
                "@epa-wg/cem-ml/wasm": "/packages/cem-ml-npm/dist/wasm/browser/cem_ml.js",
                "@epa-wg/cem-elements/": "/packages/cem-elements/",
                "@epa-wg/cem-elements/demo/lib-dir/Smiley.svg": "/packages/cem-elements/demo/lib-dir/Smiley.svg",
                "@epa-wg/material": "/packages/custom-element/material/",
                "demo-src-image": "/packages/cem-elements/demo/lib-dir/Smiley.svg?src=module",
                "demo-referrer-image": "/packages/cem-elements/demo/lib-dir/Smiley.svg?referrer=default",
                "demo-module-referrer": "/packages/cem-elements/demo/module-referrer/component.js",
                "embed-lib": "/packages/cem-elements/demo/lib-dir/embed-lib.html",
                "lib-root/": "/packages/cem-elements/demo/lib-dir/"
            },
            "scopes": {
                "/packages/cem-elements/demo/relative-referrer/": {
                    "demo-referrer-image": "/packages/cem-elements/demo/lib-dir/Smiley.svg?referrer=relative"
                },
                "/packages/cem-elements/demo/module-referrer/": {
                    "demo-referrer-image": "/packages/cem-elements/demo/confused.svg?referrer=module"
                },
                "https://referrer.example.test/absolute/": {
                    "demo-referrer-image": "/packages/cem-elements/demo/wc-square.svg?referrer=absolute"
                }
            }
        }
    </script>
    <script>
        localStorage.setItem('cemDemoLiveText', 'stored initial');
        localStorage.setItem('cemDemoPersistedDefault', 'DEF');
        localStorage.setItem('cemDemoDate', '2024-04-20');
        localStorage.setItem('cemDemoTime', '13:30');
        localStorage.setItem('cemDemoLocalDateTime', '1977-04-01T14:00:30');
        localStorage.setItem('cemDemoNumber', '1.23456e+5');
        localStorage.setItem('cemDemoJson', '{"a":1,"b":"B"}');
        localStorage.setItem('cemDemoCherries', '12');
        localStorage.setItem('cemDemoBasket', '{"cherries":12,"lemons":1}');
        localStorage.setItem('cemDemoFruitLemons', '1');
        localStorage.setItem('cemDemoFruitCherries', '12');
        localStorage.setItem('cemDemoFruitApples', '0');
        localStorage.setItem('cemDemoFruitBananas', '0');
        localStorage.setItem('cemDemoSliceEditor', 'shared initial');
        localStorage.removeItem('cemDemoOverride');
        document.addEventListener('click', (event) => {
            const target = event.target instanceof Element ? event.target : null;
            const selectButton = target?.closest('[data-dispatch-select]');
            selectButton?.dispatchEvent(new CustomEvent('cem-select', { bubbles: true, detail: { id: 'demo' } }));
        });
    </script>
    <script type="module">
        import '/packages/cem-demo-element/dist/index.js';
        import { installCemElementRuntime } from '/packages/cem-elements/dist/index.js';
        window.__cemFixtureRuntime = installCemElementRuntime(window);
    </script>
</head>
<body></body>
</html>`;

const server = createServer(async (request, response) => {
    try {
        const requestUrl = new URL(request.url ?? '/', 'http://127.0.0.1');
        const pathname = decodeURIComponent(
            requestUrl.pathname === '/' ? '/packages/cem-elements/index.html' : requestUrl.pathname,
        );
        if (pathname === '/__cem-source-harness.html') {
            response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
            response.end(sourceHarnessHtml);
            return;
        }
        // The source harness is the page URL for ordinary relative HTML assets.
        // Provide the comparison image used by the module-URL wrapper sample at
        // that page-relative address; CEM module resolution remains covered by
        // the distinct /packages/cem-elements/demo/... URLs.
        const fixturePathname = pathname === '/lib-dir/Smiley.svg'
            ? '/packages/cem-elements/demo/lib-dir/Smiley.svg'
            : pathname;
        const filePath = normalize(join(repoRoot, fixturePathname));
        if (filePath !== repoRoot && !filePath.startsWith(repoRoot + sep)) {
            response.writeHead(403);
            response.end('Forbidden');
            return;
        }
        const fileStat = await stat(filePath);
        if (!fileStat.isFile()) {
            response.writeHead(404);
            response.end('Not found');
            return;
        }
        response.writeHead(200, { 'content-type': contentType(filePath) });
        createReadStream(filePath).pipe(response);
    } catch {
        response.writeHead(404);
        response.end('Not found');
    }
});

await new Promise((resolveListen) => server.listen(0, '127.0.0.1', resolveListen));

const address = server.address();
const port = typeof address === 'object' && address ? address.port : 0;
const browser = await chromium.launch({ headless: true });

try {
    await verifySourceDocumentInventory();
    for (const fixture of fixtureSpecs) {
        const pageErrors = [];
        const context = await browser.newContext();
        const page = await context.newPage();
        page.on('pageerror', (error) => pageErrors.push(error.message));
        page.on('console', (message) => {
            if (message.type() === 'error') {
                const location = message.location().url;
                pageErrors.push(location ? `${message.text()} (${location})` : message.text());
            }
        });
        await installOfflineRoutes(page);
        await installTextHelpers(page);

        try {
            await page.goto(`http://127.0.0.1:${port}${fixture.path}`, { waitUntil: 'networkidle' });
            await page.waitForTimeout(250);
            for (const check of fixture.checks) {
                await runCheck(page, check);
            }
            await verifySymbolicControls(page, fixture.path);
            if (fixture.path === '/packages/cem-elements/demo/hex-grid.html') {
                await verifyHexRowNavigation(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/local-storage.html') {
                await verifyLocalStorageLifecycle(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/location-element.html') {
                await verifyLocationLifecycle(page);
            }
        } catch (error) {
            const unexpectedErrors = unexpectedPageErrors(fixture, pageErrors);
            const diagnostics =
                unexpectedErrors.length > 0
                    ? `\nBrowser errors:\n${unexpectedErrors.map((item) => `- ${item}`).join('\n')}`
                    : '';
            const snapshot = await collectDebugSnapshot(page, error?.check);
            throw new Error(
                `${fixture.path} failed while running ${describeCheck(error?.check)}:\n${error.message}${diagnostics}${snapshot}`,
                { cause: error },
            );
        } finally {
            await context.close();
        }

        const unexpectedErrors = unexpectedPageErrors(fixture, pageErrors);
        if (unexpectedErrors.length > 0) {
            throw new Error(
                `${fixture.path} emitted browser errors:\n${unexpectedErrors.map((error) => `- ${error}`).join('\n')}`,
            );
        }
    }
    for (const [index, fixture] of sourceDocumentSpecs.entries()) {
        const pageErrors = [];
        const page = await browser.newPage();
        page.on('pageerror', (error) => pageErrors.push(error.message));
        page.on('console', (message) => {
            if (message.type() === 'error') {
                const location = message.location().url;
                pageErrors.push(location ? `${message.text()} (${location})` : message.text());
            }
        });
        await installOfflineRoutes(page);
        await installTextHelpers(page);

        const tag = `cem-demo-source-${index + 1}`;
        try {
            await page.goto(`http://127.0.0.1:${port}/__cem-source-harness.html`, { waitUntil: 'networkidle' });
            await mountSourceDocument(page, fixture, tag);
            await verifySampleContractInventory(page, tag, fixture);
            if (fixture.samples) {
                for (const [sampleIndex, sample] of fixture.samples.entries()) {
                    const rootSelector = await markSampleRoot(page, tag, sample.legend, sampleIndex);
                    for (const check of sample.checks) {
                        await runCheck(page, scopeCheck(check, rootSelector));
                    }
                }
            }
            for (const check of fixture.checks ?? []) {
                await runCheck(page, scopeCheck(check, tag));
            }
            await verifySymbolicControls(page, fixture.path);
            if (fixture.path === '/packages/cem-elements/demo/hex-grid.html') {
                await verifyHexRowNavigation(page);
            }
        } catch (error) {
            const unexpectedErrors = unexpectedPageErrors(fixture, pageErrors);
            const diagnostics =
                unexpectedErrors.length > 0
                    ? `\nBrowser errors:\n${unexpectedErrors.map((item) => `- ${item}`).join('\n')}`
                    : '';
            const snapshot = await collectDebugSnapshot(page, error?.check);
            throw new Error(
                `${fixture.path} source contract failed while running ${describeCheck(error?.check)}:\n${error.message}${diagnostics}${snapshot}`,
                { cause: error },
            );
        } finally {
            await page.close();
        }

        const unexpectedErrors = unexpectedPageErrors(fixture, pageErrors);
        if (unexpectedErrors.length > 0) {
            throw new Error(
                `${fixture.path} source contract emitted browser errors:\n${unexpectedErrors.map((error) => `- ${error}`).join('\n')}`,
            );
        }
    }
} finally {
    await browser.close();
    await new Promise((resolveClose) => server.close(resolveClose));
}

console.log(
    `cem-elements demo fixtures verified (${fixtureSpecs.length} standalone pages, ${sourceDocumentSpecs.length} source-loaded documents).`,
);

async function verifyLocalStorageLifecycle(page) {
    const persisted = 'cem-demo-element[legend="2. Stored value with a default"]';
    const initial = 'cem-demo-element[legend="4. Simplest initial read"]';
    const live = 'cem-demo-element[legend="0. Read a live text value"]';
    await poll(page, () => localStorage.getItem('cemDemoOverride') === 'ABC'
        && localStorage.getItem('cemDemoCherries') === '24');
    await waitForNormalizedText(page, `${initial} output`, '12');
    await page.reload({ waitUntil: 'networkidle' });
    await waitForNormalizedText(page, `${persisted} output`, 'remember me');
    await waitForNormalizedText(page, `${initial} output`, '24');

    await page.locator(persisted).getByRole('button', { name: 'Empty string', exact: true }).click();
    await poll(page, () => localStorage.getItem('cemDemoPersistedDefault') === '');
    await page.reload({ waitUntil: 'networkidle' });
    await waitForNormalizedText(page, `${persisted} output`, '');
    await poll(page, () => localStorage.getItem('cemDemoPersistedDefault') === '');

    await page.locator(persisted).getByRole('button', { name: 'Clear key', exact: true }).click();
    await waitForNormalizedText(page, `${persisted} output`, 'null');
    await page.reload({ waitUntil: 'networkidle' });
    await waitForNormalizedText(page, `${persisted} output`, 'DEF');

    const otherTab = await page.context().newPage();
    const errors = [];
    otherTab.on('pageerror', (error) => errors.push(error.message));
    try {
        await installOfflineRoutes(otherTab);
        await installTextHelpers(otherTab);
        await otherTab.goto(page.url(), { waitUntil: 'networkidle' });
        await waitForNormalizedText(otherTab, `${live} output`, 'text value');
        await otherTab.locator(live).getByRole('button', { name: 'another value', exact: true }).click();
        await waitForNormalizedText(otherTab, `${live} output`, 'another value');
        await waitForNormalizedText(page, `${live} output`, 'another value');
        await otherTab.locator(live).getByRole('button', { name: 'Clear key', exact: true }).click();
        await waitForNormalizedText(otherTab, `${live} output`, 'null');
        await waitForNormalizedText(page, `${live} output`, 'null');
        if (errors.length > 0) throw new Error(`storage tab errors: ${errors.join(', ')}`);
    } finally {
        await otherTab.close();
    }
}

async function verifySymbolicControls(page, path) {
    const controlsByDemo = {
        'attributes.html': [
            ['set p1', '→p1'], ['set p2', '→p2'], ['set p3', '→p3'],
            ['remove p3', '−p3'], ['Set container value', '→v'],
        ],
        'data-slices.html': [
            ['Increase', '+'], ['Decrease', '−'], ['Set nickname to broccoli', '🥦'],
        ],
        'form.html': [
            ['Next', '→'], ['Sign in', '🔑'], ['Submit matching fruit', '✓'],
            ['Apple', '🍏'], ['Banana', '🍌'], ['Choose fruit', 'Choose fruit'],
        ],
        'http-request.html': [['GET', '↓ GET'], ['Empty URL', '∅ URL'], ['Invalid JSON response', '⚠ JSON']],
        'set-url.html': [['Set', '→']],
        'location-element.html': [
            ['Navigate with GET (reloads)', '→ GET ↻'], ['Change hash after initial read', '→#'],
        ],
        'npm-versions-demo.html': [
            ['Set URL to 0.0.22', '→0.0.22'], ['Set URL to 0.0.25', '→0.0.25'], ['Clear URL version', '∅#'],
        ],
        'local-storage.html': [
            ['Store 24 cherries', '24🍒'], ['Store 12 cherries', '12🍒'],
            ['Add cherry', '+🍒'], ['Add lemon', '+🍋'], ['Reset basket', '↺🛒'],
            ['Add apple', '+🍏'], ['Add banana', '+🍌'],
        ],
    };
    for (const [name, symbol] of controlsByDemo[path.split('/').at(-1)] ?? []) {
        const controls = page.getByRole('button', { name, exact: true });
        await controls.first().waitFor({ state: 'visible', timeout });
        const actual = await controls.evaluateAll((buttons) => buttons.map((button) => ({
            text: button.textContent.replace(/\s+/gu, ' ').trim(),
            title: button.title,
        })));
        if (actual.some((button) => button.text !== symbol || button.title !== name)) {
            throw new Error(`${path}: ${name} must display ${symbol} and retain its tooltip; got ${JSON.stringify(actual)}`);
        }
    }
    if (path.endsWith('/local-storage.html')) {
        const basket = page.locator('cem-demo-element[legend="5. Live JSON basket"]');
        const cherry = basket.getByRole('button', { name: 'Add cherry', exact: true });
        await cherry.focus();
        await cherry.press('Enter');
        await waitForText(page, 'cem-demo-element[legend="5. Live JSON basket"] dl', '🛒 14');
        await cherry.press('Space');
        await waitForText(page, 'cem-demo-element[legend="5. Live JSON basket"] dl', '🛒 15');
        await basket.getByRole('button', { name: 'Reset basket', exact: true }).click();
        await waitForText(page, 'cem-demo-element[legend="5. Live JSON basket"] dl', '🛒 13');
        const terms = await basket.locator('dt').evaluateAll((elements) => elements.map((element) => ({
            symbol: element.textContent.trim(), name: element.getAttribute('aria-label'), title: element.title,
        })));
        const expected = [
            { symbol: '🍒', name: 'Cherries', title: 'Cherries' },
            { symbol: '🍋', name: 'Lemons', title: 'Lemons' },
            { symbol: '🛒', name: 'Total', title: 'Total' },
        ];
        if (JSON.stringify(terms) !== JSON.stringify(expected)) throw new Error('fruit counters lost their accessible labels');
    }
}

async function verifyLocationLifecycle(page) {
    const live = 'cem-demo-element[legend="1. Window location live update"]';
    const initial = 'cem-demo-element[legend="2. Window location initial read"]';
    const captured = await page.locator(`${initial} dl`).textContent();
    await page.locator(initial).getByRole('button', { name: 'Change hash after initial read' }).click();
    await waitForText(page, `${live} dl`, '#after-initial-read');
    await page.goBack();
    await waitForText(page, `${live} dl`, '#checked');
    await page.goForward();
    await waitForText(page, `${live} dl`, '#after-initial-read');
    if (await page.locator(`${initial} dl`).textContent() !== captured) {
        throw new Error('initial-only location reader changed during history navigation');
    }
    await page.locator(live).getByRole('button', { name: 'Navigate with GET (reloads)' }).click();
    await waitForText(page, `${live} ul`, 'query = hello world');
    await waitForText(page, `${initial} dl`, '?query=hello+world');
}

async function verifySourceDocumentInventory() {
    const discovered = await demoHtmlPaths(resolve(repoRoot, 'packages/cem-elements/demo'));
    await verifyDemoFormatterImportMaps(discovered);
    const declared = Array.from(
        new Set(
            sourceDocumentSpecs
                .map(({ path }) => path.split('#', 1)[0])
                .filter((path) => path.startsWith('/packages/cem-elements/demo/')),
        ),
    ).sort();
    if (JSON.stringify(discovered) !== JSON.stringify(declared)) {
        const missing = discovered.filter((path) => !declared.includes(path));
        const stale = declared.filter((path) => !discovered.includes(path));
        throw new Error(
            `source-loaded demo HTML inventory mismatch; missing contracts: ${missing.join(', ') || 'none'}; stale contracts: ${stale.join(', ') || 'none'}`,
        );
    }
}

async function verifyDemoFormatterImportMaps(paths) {
    const expectedWasmPath = '/packages/cem-ml-npm/dist/wasm/browser/cem_ml.js';
    const missing = [];
    const invalid = [];
    for (const documentPath of paths) {
        const source = await readFile(resolve(repoRoot, documentPath.slice(1)), 'utf8');
        if (!source.includes('cem-demo-element/dist/index.js')) continue;

        const mapping = source.match(/"@epa-wg\/cem-ml\/wasm"\s*:\s*"([^"]+)"/u);
        if (!mapping) {
            missing.push(documentPath);
            continue;
        }
        const resolvedPath = new URL(mapping[1], `https://fixture.test${documentPath}`).pathname;
        const mappingIndex = source.indexOf(mapping[0]);
        const firstModuleIndex = source.indexOf('<script type="module"');
        if (
            resolvedPath !== expectedWasmPath
            || firstModuleIndex < 0
            || mappingIndex > firstModuleIndex
        ) {
            invalid.push(`${documentPath} -> ${mapping[1]}`);
        }
    }
    if (missing.length > 0 || invalid.length > 0) {
        throw new Error(
            `standalone demo formatter import-map mismatch; missing: ${missing.join(', ') || 'none'}; invalid or late: ${invalid.join(', ') || 'none'}`,
        );
    }
}

async function demoHtmlPaths(directory, relativeDirectory = '') {
    const paths = [];
    for (const entry of await readdir(directory, { withFileTypes: true })) {
        const relativePath = relativeDirectory ? `${relativeDirectory}/${entry.name}` : entry.name;
        if (entry.isDirectory()) {
            paths.push(...(await demoHtmlPaths(join(directory, entry.name), relativePath)));
        } else if (entry.isFile() && entry.name.endsWith('.html')) {
            paths.push(`/packages/cem-elements/demo/${relativePath}`);
        }
    }
    return paths.sort();
}

async function mountSourceDocument(page, fixture, tag) {
    await page.evaluate(
        async ({ path, producedTag, attributes, content }) => {
            await customElements.whenDefined('cem-element');
            const declaration = document.createElement('cem-element');
            declaration.hidden = true;
            declaration.setAttribute('tag', producedTag);
            declaration.setAttribute('src', path);
            const instance = document.createElement(producedTag);
            for (const [name, value] of Object.entries(attributes ?? {})) {
                instance.setAttribute(name, value);
            }
            if (content) instance.textContent = content;
            document.body.append(declaration, instance);
        },
        { path: fixture.path, producedTag: tag, attributes: fixture.attributes, content: fixture.content },
    );
}

async function verifySampleContractInventory(page, tag, fixture) {
    const expected = (fixture.samples ?? []).map((sample) => sample.legend.replace(/\s+/gu, ' ').trim());
    await poll(
        page,
        ({ hostSelector, expectedCount }) => {
            const host = document.querySelector(hostSelector);
            if (!host) return false;
            const renderComplete = Array.from(host.childNodes).some(
                (node) => node.nodeType === Node.COMMENT_NODE && node.nodeValue === 'cem-render-end',
            );
            return renderComplete && host.querySelectorAll('cem-demo-element[legend]').length >= expectedCount;
        },
        { hostSelector: tag, expectedCount: expected.length },
    );
    const actual = await page.evaluate(
        (hostSelector) =>
            Array.from(document.querySelectorAll(`${hostSelector} cem-demo-element[legend]`)).map((element) =>
                (element.getAttribute('legend') ?? '').replace(/\s+/gu, ' ').trim(),
            ),
        tag,
    );
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        const missing = actual.filter((legend) => !expected.includes(legend));
        const stale = expected.filter((legend) => !actual.includes(legend));
        throw new Error(
            `${fixture.path} sample inventory mismatch; missing legend contracts: ${missing.join(', ') || 'none'}; stale legend contracts: ${stale.join(', ') || 'none'}`,
        );
    }
}

async function markSampleRoot(page, tag, legend, index) {
    const marker = String(index + 1);
    await poll(
        page,
        ({ hostSelector, expectedLegend, markerValue }) => {
            const host = document.querySelector(hostSelector);
            const normalizeText = (value) => value.replace(/\s+/gu, ' ').trim();
            const sample = host
                ? Array.from(host.querySelectorAll('cem-demo-element')).find(
                      (element) => normalizeText(element.getAttribute('legend') ?? '') === normalizeText(expectedLegend),
                  )
                : null;
            if (!sample) return false;
            sample.setAttribute('data-cem-fixture-sample', markerValue);
            return true;
        },
        { hostSelector: tag, expectedLegend: legend, markerValue: marker },
    );
    return `${tag} cem-demo-element[data-cem-fixture-sample="${marker}"]`;
}

function scopeCheck(check, rootSelector) {
    const scoped = { ...check };
    for (const key of ['selector', 'actionSelector', 'resultSelector', 'styleSelector', 'hostSelector']) {
        if (typeof scoped[key] === 'string') {
            scoped[key] = scoped[key] === ':scope' ? rootSelector : `${rootSelector} ${scoped[key]}`;
        }
    }
    return scoped;
}

function unexpectedPageErrors(fixture, pageErrors) {
    const allowed = fixture.allowedPageErrors ?? [];
    return pageErrors.filter((error) => !allowed.some((allowedError) => error.includes(allowedError)));
}

async function verifyHexRowNavigation(page) {
    const selector = `cem-demo-element[legend="${hexRowSample.legend}"] nav`;
    await page.waitForFunction((selector) => {
        const images = Array.from(document.querySelectorAll(`${selector} img`));
        return images.length === 3 && images.every((image) => image.complete && image.naturalWidth > 0);
    }, selector, { timeout });
    const nav = page.locator(selector);
    const links = nav.locator('a');
    const geometry = await nav.locator('li').evaluateAll((cells) => cells.map((cell) => {
        const rect = cell.getBoundingClientRect();
        return { top: Math.round(rect.top), width: rect.width, margin: getComputedStyle(cell).marginBottom };
    }));
    if (new Set(geometry.map((cell) => cell.top)).size !== 1 || geometry.some((cell) => cell.margin !== '0px')) {
        throw new Error(`hex row must not stagger or overlap: ${JSON.stringify(geometry)}`);
    }
    await links.nth(0).focus();
    await page.keyboard.press('Tab');
    if (!await links.nth(1).evaluate((link) => link === document.activeElement)) {
        throw new Error('Tab must reach the current-page link');
    }
    await page.keyboard.press('Tab');
    if (!await links.nth(2).evaluate((link) => link === document.activeElement)) {
        throw new Error('Tab must reach the next link');
    }
    await page.keyboard.press('Shift+Tab');
    if (!await links.nth(1).evaluate((link) => link === document.activeElement)) {
        throw new Error('Shift+Tab must return to the current-page link');
    }
    if (await nav.locator('a[aria-current="page"]').count() !== 1 ||
        await links.nth(1).getAttribute('aria-current') !== 'page') {
        throw new Error('keyboard focus must not move the current-page marker');
    }
    await page.keyboard.press('Tab');
    await Promise.all([
        page.waitForURL('**/demo/scoped-css.html', { timeout }),
        page.keyboard.press('Enter'),
    ]);
}

async function installOfflineRoutes(page) {
    await page.route('https://unpkg.com/cem-demo-element@*/cem-demo-element.js', (route) =>
        route.fulfill({ contentType: 'text/javascript; charset=utf-8', body: htmlDemoElementModule }),
    );
    await page.route(/^https:\/\/unpkg\.com\/pokeapi-sprites@.*\.svg$/, (route) =>
        route.fulfill({
            contentType: 'image/svg+xml; charset=utf-8',
            body: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><title>fixture sprite</title></svg>',
        }),
    );
}

async function installTextHelpers(page) {
    await page.addInitScript(() => {
        globalThis.__cemFixtureVisibleText = (root) => {
            const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
                acceptNode(node) {
                    const parent = node.parentElement;
                    if (!parent || parent.closest('template,script,style,[hidden]')) {
                        return NodeFilter.FILTER_REJECT;
                    }
                    return NodeFilter.FILTER_ACCEPT;
                },
            });
            const parts = [];
            for (let node = walker.nextNode(); node; node = walker.nextNode()) {
                parts.push(node.textContent ?? '');
            }
            return parts.join(' ');
        };
        globalThis.__cemFixtureNormalizeText = (value) => value.replace(/\s+/gu, ' ').trim();
    });
}

async function runCheck(page, check) {
    try {
        switch (check.kind) {
            case 'text':
                await waitForText(page, check.selector, check.expected);
                return;
            case 'normalizedText':
                await waitForNormalizedText(page, check.selector, check.expected);
                return;
            case 'countAtLeast':
                await waitForCount(page, check.selector, check.min);
                return;
            case 'countExactly':
                await waitForExactCount(page, check.selector, check.count);
                return;
            case 'attributeContains':
                await waitForAttribute(page, check.selector, check.name, check.expected);
                return;
            case 'attributeEquals':
                await waitForExactAttribute(page, check.selector, check.name, check.expected);
                return;
            case 'propertyEquals':
                await waitForExactProperty(page, check.selector, check.name, check.expected);
                return;
            case 'formState':
                await poll(page, ({ selector, expected }) => {
                    const root = document.querySelector(selector);
                    if (!root) return false;
                    const inputs = Array.from(root.querySelectorAll('input'));
                    const actual = {
                        inputs: inputs.map(input => input.value),
                        checked: inputs.map(input => input.checked),
                        outputs: Array.from(root.querySelectorAll('output'), output =>
                            globalThis.__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(output))),
                    };
                    return Object.entries(expected).every(([key, values]) =>
                        JSON.stringify(actual[key]) === JSON.stringify(values));
                }, check);
                return;
            case 'imageLoaded':
                await page.waitForFunction((selector) => {
                    const image = document.querySelector(selector);
                    return image instanceof HTMLImageElement && image.complete && image.naturalWidth > 0;
                }, check.selector, { timeout });
                return;
            case 'pressThenProperty':
                await page.locator(check.actionSelector).press(check.key);
                await waitForExactProperty(page, check.selector, check.name, check.expected);
                return;
            case 'removeElement':
                await page.waitForSelector(check.selector, { timeout });
                await page.locator(check.selector).evaluate((element) => element.remove());
                return;
            case 'shortenedHrefText':
                await waitForShortenedHrefText(page, check.selector, check.maxLength, check.ellipsis);
                return;
            case 'attributeAbsent':
                await waitForAbsentAttribute(page, check.selector, check.name);
                return;
            case 'svgUseReferences':
                await verifySvgUseReferences(page, check.selector, check.expected);
                return;
            case 'styleTextContains':
                await waitForStyleText(page, check.selector, check.expected, true);
                return;
            case 'styleTextNotContains':
                await waitForStyleText(page, check.selector, check.unexpected, false);
                return;
            case 'computedStyle':
                await waitForComputedStyle(page, check.selector, check.property, check.expected);
                return;
            case 'computedStyleNot':
                await waitForComputedStyleNot(page, check.selector, check.property, check.unexpected);
                return;
            case 'computedStyleByText':
                await waitForComputedStyleByText(page, check.selector, check.text, check.property, check.expected, true);
                return;
            case 'computedStyleNotByText':
                await waitForComputedStyleByText(page, check.selector, check.text, check.property, check.unexpected, false);
                return;
            case 'keyframeIdentity':
                await waitForKeyframeIdentity(page, check);
                return;
            case 'clickThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.click(check.actionSelector);
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'fillThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.fill(check.actionSelector, check.value);
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'selectThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.selectOption(check.actionSelector, check.value);
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'typeThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.locator(check.actionSelector).pressSequentially(check.value);
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'fillBlurThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.fill(check.actionSelector, check.value);
                await page.locator(check.actionSelector).blur();
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'fillDispatchThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.fill(check.actionSelector, check.value);
                await page.dispatchEvent(check.actionSelector, check.eventName);
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'dispatchThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.dispatchEvent(check.actionSelector, check.eventName);
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'mouseThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.locator(check.actionSelector).hover({ position: check.eventInit });
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'focusThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.locator(check.actionSelector).focus();
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'blurThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.locator(check.actionSelector).blur();
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'checkThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.locator(check.actionSelector).check();
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'uncheckThenNormalizedText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.locator(check.actionSelector).uncheck();
                await waitForNormalizedText(page, check.resultSelector, check.expected);
                return;
            default:
                throw new Error(`unknown check kind ${check.kind}`);
        }
    } catch (error) {
        error.check = check;
        throw error;
    }
}

async function collectDebugSnapshot(page, check) {
    try {
        const snapshot = await page.evaluate((failedCheck) => {
            const customElementTags = [
                'dce-link',
                'dce-1-slot',
                'dce-2-slots',
                'dce-3-slot',
                'dce-4-slot',
                'pokemon-tile',
                'cem-attr-card',
                'cem-attr-defaults',
                'cem-attr-slice',
                'cem-slice-field',
                'cem-loop-list',
                'cem-css-private',
                'cem-css-shared-bare',
                'cem-css-shared-peer',
                'cem-css-shared-explicit',
                'cem-css-explicit-peer',
                'cem-css-mixed',
                'cem-css-mixed-peer',
                'cem-css-unscoped-explicit',
                'cem-css-mismatch',
                'cem-css-mismatch-bare',
                'cem-css-invalid-declaration',
                'cem-css-instance',
                'cem-css-dynamic',
                'cem-css-fragment',
                'cem-css-keyframes',
                'cem-form-fruit-choice',
            ];
            const failedSelector =
                failedCheck?.selector ??
                failedCheck?.targetSelector ??
                failedCheck?.hostSelector ??
                failedCheck?.resultSelector ??
                failedCheck?.actionSelector ??
                undefined;
            const failedTexts = failedSelector
                ? Array.from(document.querySelectorAll(failedSelector)).map((element) =>
                      globalThis.__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(element)),
                  )
                : [];
            const failedElement = failedSelector ? document.querySelector(failedSelector) : null;
            return {
                bodyText: globalThis
                    .__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(document.body))
                    .slice(0, 1000),
                failedSelector,
                failedExpected: failedCheck?.expected,
                failedComputedValue:
                    failedCheck?.kind === 'computedStyle' && failedElement
                        ? getComputedStyle(failedElement)[failedCheck.property]
                        : null,
                styles: Array.from(document.querySelectorAll('style')).map(
                    (style) => style.textContent?.trim().slice(0, 1000) ?? '',
                ),
                failedCheckNow:
                    failedCheck?.kind === 'text' && typeof failedCheck.expected === 'string'
                        ? failedTexts.some((value) => value.includes(failedCheck.expected))
                        : null,
                failedSelectorMatches: failedSelector
                    ? Array.from(document.querySelectorAll(failedSelector)).map((element) => ({
                          tag: element.localName,
                          text: globalThis
                              .__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(element))
                              .slice(0, 500),
                          html: element.outerHTML.slice(0, 1000),
                      }))
                    : [],
                samples: Array.from(document.querySelectorAll('cem-demo-element')).map((sample) => ({
                    legend: sample.getAttribute('legend'),
                    marker: sample.getAttribute('data-cem-fixture-sample'),
                    state: sample.getAttribute('data-state'),
                    helperMounted: sample.__cemDemoMounted ?? null,
                    hasTemplate: !!sample.querySelector(':scope > template'),
                    hasDemo: !!sample.querySelector(':scope > [slot="demo"]'),
                    warnings: sample.querySelectorAll('[slot="demo"] strong').length,
                })),
                declarations: Array.from(document.querySelectorAll('cem-element[tag]')).map((declaration) => {
                    const tag = declaration.getAttribute('tag');
                    return {
                        tag,
                        src: declaration.getAttribute('src'),
                        defined: !!customElements.get(tag),
                        styles: declaration.querySelectorAll('style[data-cem-declaration-style]').length,
                        diagnostics: window.__cemFixtureRuntime?.diagnosticsFor(declaration) ?? null,
                        instances: Array.from(document.querySelectorAll(CSS.escape(tag))).map((instance) => ({
                            artifact: instance.getAttribute('data-cem-template-artifact-id'),
                            diagnostics: window.__cemFixtureRuntime?.diagnosticsFor(instance) ?? null,
                            text: globalThis.__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(instance)).slice(0, 200),
                        })),
                    };
                }),
                elements: customElementTags
                    .flatMap((tag) => Array.from(document.querySelectorAll(tag)))
                    .map((element) => ({
                        tag: element.localName,
                        text: globalThis
                            .__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(element))
                            .slice(0, 500),
                        html: element.outerHTML.slice(0, 1000),
                    })),
            };
        }, check);
        return `\nDebug snapshot:\n${JSON.stringify(snapshot, null, 2)}`;
    } catch (error) {
        return `\nDebug snapshot unavailable: ${error instanceof Error ? error.message : String(error)}`;
    }
}

function text(selector, expected) {
    return { kind: 'text', selector, expected };
}

function sampleContract(legend, checks) {
    return { legend, checks };
}

function normalizedText(selector, expected) {
    return { kind: 'normalizedText', selector, expected };
}

function countAtLeast(selector, min) {
    return { kind: 'countAtLeast', selector, min };
}

function countExactly(selector, count) {
    return { kind: 'countExactly', selector, count };
}

function attributeContains(selector, name, expected) {
    return { kind: 'attributeContains', selector, name, expected };
}

function attributeEquals(selector, name, expected) {
    return { kind: 'attributeEquals', selector, name, expected };
}

function propertyEquals(selector, name, expected) {
    return { kind: 'propertyEquals', selector, name, expected };
}

function formState(expected) {
    return { kind: 'formState', selector: ':scope', expected };
}

function imageLoaded(selector) {
    return { kind: 'imageLoaded', selector };
}

function pressThenProperty(actionSelector, key, selector, name, expected) {
    return { kind: 'pressThenProperty', actionSelector, key, selector, name, expected };
}

function removeElement(selector) {
    return { kind: 'removeElement', selector };
}

function shortenedHrefText(selector, maxLength, ellipsis = '…') {
    return { kind: 'shortenedHrefText', selector, maxLength, ellipsis };
}

function attributeAbsent(selector, name) {
    return { kind: 'attributeAbsent', selector, name };
}

function svgUseReferences(selector, expected) {
    return { kind: 'svgUseReferences', selector, expected };
}

function styleTextContains(selector, expected) {
    return { kind: 'styleTextContains', selector, expected };
}

function styleTextNotContains(selector, unexpected) {
    return { kind: 'styleTextNotContains', selector, unexpected };
}

function computedStyle(selector, property, expected) {
    return { kind: 'computedStyle', selector, property, expected };
}

function computedStyleNot(selector, property, unexpected) {
    return { kind: 'computedStyleNot', selector, property, unexpected };
}

function computedStyleByText(selector, text, property, expected) {
    return { kind: 'computedStyleByText', selector, text, property, expected };
}

function computedStyleNotByText(selector, text, property, unexpected) {
    return { kind: 'computedStyleNotByText', selector, text, property, unexpected };
}

function keyframeIdentity(styleSelector, hostSelector, targetSelector, authoredName, encodedSeed) {
    return { kind: 'keyframeIdentity', styleSelector, hostSelector, targetSelector, authoredName, encodedSeed };
}

function clickThenText(actionSelector, resultSelector, expected) {
    return { kind: 'clickThenText', actionSelector, resultSelector, expected };
}

function fillThenText(actionSelector, value, resultSelector, expected) {
    return { kind: 'fillThenText', actionSelector, value, resultSelector, expected };
}

function selectThenText(actionSelector, value, resultSelector, expected) {
    return { kind: 'selectThenText', actionSelector, value, resultSelector, expected };
}

function typeThenText(actionSelector, value, resultSelector, expected) {
    return { kind: 'typeThenText', actionSelector, value, resultSelector, expected };
}

function fillBlurThenText(actionSelector, value, resultSelector, expected) {
    return { kind: 'fillBlurThenText', actionSelector, value, resultSelector, expected };
}

function fillDispatchThenText(actionSelector, value, eventName, resultSelector, expected) {
    return { kind: 'fillDispatchThenText', actionSelector, value, eventName, resultSelector, expected };
}

function dispatchThenText(actionSelector, eventName, resultSelector, expected) {
    return { kind: 'dispatchThenText', actionSelector, eventName, resultSelector, expected };
}

function mouseThenText(actionSelector, eventInit, resultSelector, expected) {
    return { kind: 'mouseThenText', actionSelector, eventInit, resultSelector, expected };
}

function focusThenText(actionSelector, resultSelector, expected) {
    return { kind: 'focusThenText', actionSelector, resultSelector, expected };
}

function blurThenText(actionSelector, resultSelector, expected) {
    return { kind: 'blurThenText', actionSelector, resultSelector, expected };
}

function checkThenText(actionSelector, resultSelector, expected) {
    return { kind: 'checkThenText', actionSelector, resultSelector, expected };
}

function uncheckThenNormalizedText(actionSelector, resultSelector, expected) {
    return { kind: 'uncheckThenNormalizedText', actionSelector, resultSelector, expected };
}

async function waitForText(page, selector, expected) {
    await poll(
        page,
        ({ selector: checkSelector, expected: checkExpected }) => {
            const elements = Array.from(document.querySelectorAll(checkSelector));
            return elements.some((element) =>
                globalThis
                    .__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(element))
                    .includes(checkExpected),
            );
        },
        { selector, expected },
    );
}

async function waitForNormalizedText(page, selector, expected) {
    await poll(
        page,
        ({ selector: checkSelector, expected: checkExpected }) => {
            const element = document.querySelector(checkSelector);
            return element
                ? globalThis.__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(element)) === checkExpected
                : false;
        },
        { selector, expected },
    );
}

async function waitForCount(page, selector, min) {
    await poll(
        page,
        ({ selector: checkSelector, min: checkMin }) => document.querySelectorAll(checkSelector).length >= checkMin,
        { selector, min },
    );
}

async function waitForExactCount(page, selector, count) {
    await poll(
        page,
        ({ selector: checkSelector, count: checkCount }) =>
            document.querySelectorAll(checkSelector).length === checkCount,
        { selector, count },
    );
}

async function waitForAttribute(page, selector, name, expected) {
    await poll(
        page,
        ({ selector: checkSelector, name: attributeName, expected: checkExpected }) => {
            const element = document.querySelector(checkSelector);
            return element ? (element.getAttribute(attributeName) ?? '').includes(checkExpected) : false;
        },
        { selector, name, expected },
    );
}

async function waitForExactAttribute(page, selector, name, expected) {
    await poll(
        page,
        ({ selector: checkSelector, name: attributeName, expected: checkExpected }) =>
            document.querySelector(checkSelector)?.getAttribute(attributeName) === checkExpected,
        { selector, name, expected },
    );
}

async function waitForExactProperty(page, selector, name, expected) {
    await poll(
        page,
        ({ selector: checkSelector, name: propertyName, expected: checkExpected }) =>
            document.querySelector(checkSelector)?.[propertyName] === checkExpected,
        { selector, name, expected },
    );
}

async function waitForShortenedHrefText(page, selector, maxLength, ellipsis) {
    await poll(
        page,
        ({ selector: checkSelector, maxLength: checkMaxLength, ellipsis: checkEllipsis }) => {
            const link = document.querySelector(checkSelector);
            const href = link?.getAttribute('href') ?? '';
            const codepoints = Array.from(href);
            if (!link || codepoints.length <= checkMaxLength) return false;
            const ellipsisLength = Array.from(checkEllipsis).length;
            if (ellipsisLength > checkMaxLength) return false;
            const retainedLength = checkMaxLength - ellipsisLength;
            const prefixLength = Math.floor(retainedLength / 2);
            const suffixLength = retainedLength - prefixLength;
            const expected = `${codepoints.slice(0, prefixLength).join('')}${checkEllipsis}${codepoints.slice(-suffixLength).join('')}`;
            const visibleText = globalThis.__cemFixtureNormalizeText(
                globalThis.__cemFixtureVisibleText(link),
            );
            return visibleText === expected && Array.from(visibleText).length === checkMaxLength;
        },
        { selector, maxLength, ellipsis },
    );
}

async function waitForAbsentAttribute(page, selector, name) {
    await poll(
        page,
        ({ selector: checkSelector, name: attributeName }) => {
            const element = document.querySelector(checkSelector);
            return element ? !element.hasAttribute(attributeName) : false;
        },
        { selector, name },
    );
}

async function verifySvgUseReferences(page, selector, expected) {
    await page.waitForSelector(selector, { timeout });
    const actual = await page.evaluate(
        ({ selector: checkSelector, xlinkNamespace }) => {
            const svg = document.querySelector(checkSelector);
            return svg
                ? Array.from(svg.querySelectorAll('use')).map((element) => {
                      const bounds = element.getBBox();
                      return {
                          href: element.getAttributeNS(xlinkNamespace, 'href'),
                          width: bounds.width,
                          height: bounds.height,
                      };
                  })
                : [];
        },
        { selector, xlinkNamespace: 'http://www.w3.org/1999/xlink' },
    );
    const renderedReferences = actual.map(({ href }) => href);
    if (
        JSON.stringify(renderedReferences) !== JSON.stringify(expected) ||
        actual.some(({ width, height }) => width <= 0 || height <= 0)
    ) {
        throw new Error(
            `expected rendered SVG use references ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`,
        );
    }
}

async function waitForStyleText(page, selector, text, contains) {
    await poll(
        page,
        ({ selector: checkSelector, text: checkText, contains: shouldContain }) => {
            const styles = Array.from(document.querySelectorAll(checkSelector));
            if (styles.length === 0) return false;
            return shouldContain
                ? styles.some((style) => (style.textContent ?? '').includes(checkText))
                : styles.every((style) => !(style.textContent ?? '').includes(checkText));
        },
        { selector, text, contains },
    );
}

async function waitForComputedStyle(page, selector, property, expected) {
    await poll(
        page,
        ({ selector: checkSelector, property: styleProperty, expected: checkExpected }) => {
            const element = document.querySelector(checkSelector);
            return element ? getComputedStyle(element)[styleProperty] === checkExpected : false;
        },
        { selector, property, expected },
    );
}

async function waitForComputedStyleNot(page, selector, property, unexpected) {
    await poll(
        page,
        ({ selector: checkSelector, property: styleProperty, unexpected: checkUnexpected }) => {
            const element = document.querySelector(checkSelector);
            return element ? getComputedStyle(element)[styleProperty] !== checkUnexpected : false;
        },
        { selector, property, unexpected },
    );
}

async function waitForComputedStyleByText(page, selector, text, property, value, equals) {
    await poll(
        page,
        ({ selector: checkSelector, text: checkText, property: styleProperty, value: checkValue, equals: shouldEqual }) => {
            const element = Array.from(document.querySelectorAll(checkSelector)).find(
                (candidate) =>
                    globalThis.__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(candidate)) === checkText,
            );
            return element ? (getComputedStyle(element)[styleProperty] === checkValue) === shouldEqual : false;
        },
        { selector, text, property, value, equals },
    );
}

async function waitForKeyframeIdentity(page, check) {
    await poll(
        page,
        ({ styleSelector, hostSelector, targetSelector, authoredName, encodedSeed }) => {
            const style = document.querySelector(styleSelector);
            const host = document.querySelector(hostSelector);
            const target = document.querySelector(`${hostSelector} ${targetSelector}`);
            const renderScope = host?.getAttribute('data-cem-render-scope') ?? '';
            if (!style || !target || !renderScope.includes(encodedSeed)) return false;
            const rewrittenName = `${authoredName}-${renderScope}-s1`;
            const css = style.textContent ?? '';
            return (
                css.includes(`@keyframes ${rewrittenName}`) &&
                css.includes(`animation: ${rewrittenName} `) &&
                getComputedStyle(target).animationName === rewrittenName
            );
        },
        check,
    );
}

async function poll(page, predicate, arg) {
    const startedAt = Date.now();
    let lastError;
    while (Date.now() - startedAt <= timeout) {
        try {
            if (await page.evaluate(predicate, arg)) {
                return;
            }
        } catch (error) {
            lastError = error;
        }
        await page.waitForTimeout(500);
    }
    try {
        if (await page.evaluate(predicate, arg)) {
            return;
        }
    } catch (error) {
        lastError = error;
    }
    await page.waitForTimeout(500);
    try {
        if (await page.evaluate(predicate, arg)) {
            return;
        }
    } catch (error) {
        lastError = error;
    }
    if (lastError) {
        throw lastError;
    }
    throw new Error(`poll timed out after ${timeout}ms`);
}

function describeCheck(check) {
    if (!check) return 'unknown check';
    switch (check.kind) {
        case 'text':
        case 'normalizedText':
            return `${check.kind}(${check.selector}, ${JSON.stringify(check.expected)})`;
        case 'countAtLeast':
            return `countAtLeast(${check.selector}, ${check.min})`;
        case 'countExactly':
            return `countExactly(${check.selector}, ${check.count})`;
        case 'attributeContains':
            return `attributeContains(${check.selector}, ${check.name}, ${JSON.stringify(check.expected)})`;
        case 'attributeEquals':
            return `attributeEquals(${check.selector}, ${check.name}, ${JSON.stringify(check.expected)})`;
        case 'propertyEquals':
            return `propertyEquals(${check.selector}, ${check.name}, ${JSON.stringify(check.expected)})`;
        case 'formState':
            return `formState(${check.selector}, ${JSON.stringify(check.expected)})`;
        case 'imageLoaded':
            return `imageLoaded(${check.selector})`;
        case 'pressThenProperty':
            return `pressThenProperty(${check.actionSelector}, ${check.key}, ${check.selector}, ${check.name}, ${JSON.stringify(check.expected)})`;
        case 'removeElement':
            return `removeElement(${check.selector})`;
        case 'shortenedHrefText':
            return `shortenedHrefText(${check.selector}, ${check.maxLength}, ${JSON.stringify(check.ellipsis)})`;
        case 'attributeAbsent':
            return `attributeAbsent(${check.selector}, ${check.name})`;
        case 'svgUseReferences':
            return `svgUseReferences(${check.selector}, ${JSON.stringify(check.expected)})`;
        case 'styleTextContains':
            return `styleTextContains(${check.selector}, ${JSON.stringify(check.expected)})`;
        case 'styleTextNotContains':
            return `styleTextNotContains(${check.selector}, ${JSON.stringify(check.unexpected)})`;
        case 'computedStyle':
            return `computedStyle(${check.selector}, ${check.property}, ${JSON.stringify(check.expected)})`;
        case 'computedStyleNot':
            return `computedStyleNot(${check.selector}, ${check.property}, ${JSON.stringify(check.unexpected)})`;
        case 'computedStyleByText':
            return `computedStyleByText(${check.selector}, ${JSON.stringify(check.text)}, ${check.property}, ${JSON.stringify(check.expected)})`;
        case 'computedStyleNotByText':
            return `computedStyleNotByText(${check.selector}, ${JSON.stringify(check.text)}, ${check.property}, ${JSON.stringify(check.unexpected)})`;
        case 'keyframeIdentity':
            return `keyframeIdentity(${check.hostSelector}, ${check.authoredName}, ${check.encodedSeed})`;
        case 'clickThenText':
            return `clickThenText(${check.actionSelector}, ${check.resultSelector}, ${JSON.stringify(check.expected)})`;
        case 'fillThenText':
            return `fillThenText(${check.actionSelector}, ${check.resultSelector}, ${JSON.stringify(check.expected)})`;
        case 'selectThenText':
            return `selectThenText(${check.actionSelector}, ${JSON.stringify(check.value)}, ${check.resultSelector}, ${JSON.stringify(check.expected)})`;
        case 'typeThenText':
            return `typeThenText(${check.actionSelector}, ${check.resultSelector}, ${JSON.stringify(check.expected)})`;
        default:
            return check.kind;
    }
}

function contentType(filePath) {
    switch (extname(filePath)) {
        case '.html':
            return 'text/html; charset=utf-8';
        case '.cemt':
            return 'text/cem-ml; charset=utf-8';
        case '.xhtml':
            return 'application/xhtml+xml; charset=utf-8';
        case '.xsl':
            return 'application/xslt+xml; charset=utf-8';
        case '.js':
        case '.mjs':
            return 'text/javascript; charset=utf-8';
        case '.json':
            return 'application/json; charset=utf-8';
        case '.xml':
            return 'application/xml; charset=utf-8';
        case '.wasm':
            return 'application/wasm';
        case '.css':
            return 'text/css; charset=utf-8';
        case '.svg':
            return 'image/svg+xml; charset=utf-8';
        case '.map':
            return 'application/json; charset=utf-8';
        default:
            return 'application/octet-stream';
    }
}
