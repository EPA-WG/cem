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

const domChainResults = [
    ['Wrap values', 'a, b, a'],
    ['Parent', 'row'],
    ['Element children', 'a, b'],
    ['All child nodes', '4'],
    ['Ancestors', 'section, row'],
    ['Closest ancestor', 'section'],
    ['Local names', 'fruit'],
    ['Node text', 'ivysaur'],
    ['Unqualified attribute', '2'],
    ['Qualified attribute', 'other'],
    ['First match', '2'],
    ['Last match', 'b'],
    ['Filter', 'a, a'],
    ['First value', 'a'],
    ['Last value', 'b'],
    ['Zero-based selection', 'b'],
    ['Take a prefix', 'a, b'],
    ['Skip a prefix', 'b, c'],
    ['Map values', 'a!, b!'],
    ['Flatten mapped values', 'a, a, b, b'],
    ['Any match', 'true'],
    ['All match', 'false'],
    ['Empty chain', 'true'],
    ['Count values', '3'],
    ['Sort values', 'a, b, b'],
    ['Sort native nodes', 'second, first, third'],
    ['Reverse order', 'c, b, a'],
];
const domChainEdit = (value, expected) => [
    fillThenText('article input', value, 'article output', expected),
    propertyEquals('article output', 'textContent', expected),
    elementIdentity('article input', 'same'), focusedElement('article input'),
    propertyEquals('article input', 'value', value),
    propertyEquals('article input', 'selectionStart', value.length),
    propertyEquals('article input', 'selectionEnd', value.length),
];
const domChainSamples = domChainResults.map(([legend, expected]) => sampleContract(legend, [
    countExactly('article', 1), countExactly('article output', 1),
    propertyEquals('article output', 'textContent', expected),
    ...(legend === 'First match' ? [
        countExactly('article input', 1), propertyEquals('article input', 'value', 'id'),
        elementIdentity('article input', 'remember'),
        ...[['name', 'ivy'], ['missing', ''], ['', ''], ['Name', ''], [' id ', ''],
            ['row', ''], ['name/id', ''], ['<id>', ''], ['名', '']]
            .flatMap(([value, result]) => [...domChainEdit(value, result), ...domChainEdit('id', '2')]),
        ...domChainEdit('name', 'ivy'),
    ] : []),
    ...(legend === 'Sort native nodes' ? [
        countExactly('article select', 1), propertyEquals('article select', 'value', 'ascending'),
        nodeTexts('article option', ['Ascending', 'Descending']),
        propertyEquals('article option:first-child', 'value', 'ascending'),
        propertyEquals('article option:last-child', 'value', 'descending'),
        elementIdentity('article select', 'remember'),
        ...['descending', 'ascending', 'descending', 'ascending', 'descending'].flatMap(direction => [
            focusThenText('article select', 'article label', 'Direction'),
            selectThenText('article select', direction, 'article output',
                direction === 'ascending' ? 'second, first, third' : 'first, third, second'),
            propertyEquals('article output', 'textContent',
                direction === 'ascending' ? 'second, first, third' : 'first, third, second'),
            elementIdentity('article select', 'same'), focusedElement('article select'),
            propertyEquals('article select', 'value', direction),
            propertyEquals('article select', 'selectedIndex', direction === 'ascending' ? 0 : 1),
        ]),
    ] : []),
]));

const functionEditor = (selector, value) => [
    elementIdentity(selector, 'same'), focusedElement(selector),
    propertyEquals(selector, 'value', value),
    propertyEquals(selector, 'selectionStart', value.length),
    propertyEquals(selector, 'selectionEnd', value.length),
];
const functionStringEdit = (value, expected) => [
    fillThenText('input', value, 'output', expected.replace(/\s+/gu, ' ').trim()),
    propertyEquals('output', 'textContent', expected), ...functionEditor('input', value),
];
const functionXmlItems = expected => [
    countExactly('li', expected.length),
    ...expected.map((value, index) => propertyEquals(`li:nth-child(${index + 1})`, 'textContent', value)),
];
const functionXmlEdit = (value, expected) => [
    fillThenText('textarea', value, 'ul', ''), ...functionXmlItems(expected), countExactly('[role="alert"]', 0),
    countExactly('ul', 1), ...functionEditor('textarea', value),
];
const xpathFunctionSamples = [
    ...['1. Named XPath function', '1a. CEM-QL string pair'].map((legend, index) => sampleContract(legend, [
        propertyEquals('output', 'textContent', 'Hello 🍒'), propertyEquals('input', 'value', 'Hello'),
        elementIdentity('input', 'remember'),
        ...['Changed 🍋', '  🍋 & <é>  ', '', index === 0 ? 'First' : 'Second']
            .flatMap(value => functionStringEdit(value, `${value} 🍒`)),
    ])),
    ...['2. Shared XPath predicate', '2a. CEM-QL predicate pair'].map((legend, index) => sampleContract(legend, [
        propertyEquals('output', 'textContent', 'cherry 🍒'), propertyEquals('input', 'value', 'cherry'),
        elementIdentity('input', 'remember'),
        ...['lemon', 'Cherry', ' cherry ', ''].flatMap(value => [
            ...functionStringEdit(value, 'Try cherry'), ...functionStringEdit('cherry', 'cherry 🍒'),
        ]),
        ...(index === 1 ? functionStringEdit('lemon', 'Try cherry') : []),
    ])),
    ...['3. XML nodes and matching', '3a. CEM-QL native node pair'].map((legend, index) => {
        const recovered = index === 0 ? 'Recovered XPath' : 'Recovered CEM-QL';
        const recover = () => functionXmlEdit(`<r><item qty="2">${recovered}</item></r>`, [`${recovered}: stocked`]);
        return sampleContract(legend, [
            ...functionXmlItems(['Cherry: stocked']),
            propertyEquals('textarea', 'value', '<r><item qty="2">Cherry</item></r>'),
            elementIdentity('textarea', 'remember'),
            ...functionXmlEdit('<r><item qty="1.999">Below</item><item qty="2">At</item><item qty=" 2.5 ">Above</item><item qty="-1">Negative</item><item>Missing</item></r>',
                ['Below: low stock', 'At: stocked', 'Above: stocked', 'Negative: low stock', 'Missing: low stock']),
            ...functionXmlEdit('<r xmlns:p="urn:fruit"><!--skip--><?skip it?><p:item qty="9">Qualified</p:item><group><item qty="9">Nested</item></group><item p:qty="9">Qualified qty</item><item qty="2"> A<b>B</b><![CDATA[C]]><!--skip--><?skip it?> D </item></r>',
                ['Qualified qty: low stock', ' ABC D : stocked']),
            ...['<r/>', '<r xmlns="urn:fruit"><item qty="2">Qualified root</item></r>',
                '<other><r><item qty="2">Nested root</item></r></other>'].flatMap(value => [
                ...functionXmlEdit(value, []), ...recover(),
            ]),
            ...['<r>', '<r><item></r>'].flatMap(value => [
                fillThenText('textarea', value, '[role="alert"]', 'XML'),
                countExactly('li', 0), countExactly('ul', 0), ...functionEditor('textarea', value), ...recover(),
            ]),
        ]);
    }),
];

const mapArrayOutputs = expected => [
    countExactly('article output', expected.length), countExactly('article [role="alert"]', 0),
    ...expected.map((value, index) => propertyEquals(`article p:nth-of-type(${index + 1}) output`, 'textContent', value)),
];
const mapArrayEditor = (selector, value) => [
    elementIdentity(selector, 'same'), focusedElement(selector), propertyEquals(selector, 'value', value),
    propertyEquals(selector, 'selectionStart', value.length), propertyEquals(selector, 'selectionEnd', value.length),
];
const mapArrayEdit = (selector, value, expected) => [
    fillThenText(selector, value, 'article output', ''), ...mapArrayOutputs(expected), ...mapArrayEditor(selector, value),
];
const mapArrayInvalid = (value, message) => [
    fillThenText('article textarea', value, 'article [role="alert"]', message),
    countExactly('article output', 0), ...mapArrayEditor('article textarea', value),
];
const mapArrayPick = (position, label) => mapArrayEdit('article input', position,
    ['2', label, 'Empty member (array size 1)']);
const mapArrayRecover = () => mapArrayEdit('article textarea', '<basket><cherry>5</cherry></basket>',
    ['1', 'cherry: 5', 'Empty member (array size 1)']);
const jsonMapRecover = () => mapArrayEdit('article textarea', '{"note":null}', ['1 members; numeric total 0', 'Null value']);
const xpathMapArraySamples = [
    sampleContract('1. An IP-filter map with an optional note', [
        ...mapArrayOutputs(['allow: 192.0.2.0/24', 'Absent entry', '2']),
        propertyEquals('article input', 'value', '192.0.2.0/24'),
        propertyEquals('article select[aria-label="Action"]', 'value', 'allow'),
        propertyEquals('article select[aria-label="Note entry"]', 'value', 'absent'),
        ...['input', 'select[aria-label="Action"]', 'select[aria-label="Note entry"]']
            .map(selector => elementIdentity(`article ${selector}`, 'remember')),
        ...['', '  🍒 & <local>  ', '198.51.100.0/24'].flatMap(value =>
            mapArrayEdit('article input', value, [`allow: ${value}`, 'Absent entry', '2'])),
        focusThenText('article select[aria-label="Action"]', 'article output', 'allow: 198.51.100.0/24'),
        selectThenText('article select[aria-label="Action"]', 'deny', 'article output', 'deny: 198.51.100.0/24'),
        ...mapArrayOutputs(['deny: 198.51.100.0/24', 'Absent entry', '2']),
        elementIdentity('article select[aria-label="Action"]', 'same'), focusedElement('article select[aria-label="Action"]'),
        propertyEquals('article select[aria-label="Action"]', 'value', 'deny'),
        ...[['empty', 'Present, empty sequence', '3'], ['value', 'Local preview', '3'], ['absent', 'Absent entry', '2']]
            .flatMap(([state, expected, count]) => [
                focusThenText('article select[aria-label="Note entry"]', 'article output', 'deny: 198.51.100.0/24'),
                selectThenText('article select[aria-label="Note entry"]', state, 'article p:nth-of-type(2) output', expected),
                ...mapArrayOutputs(['deny: 198.51.100.0/24', expected, count]),
                elementIdentity('article select[aria-label="Note entry"]', 'same'),
                focusedElement('article select[aria-label="Note entry"]'),
                propertyEquals('article select[aria-label="Note entry"]', 'value', state),
                propertyEquals('article select[aria-label="Action"]', 'value', 'deny'),
                propertyEquals('article input', 'value', '198.51.100.0/24'),
            ]),
    ]),
    sampleContract('2. Select a retained fruit by array position', [
        ...mapArrayOutputs(['2', 'apple: 2', 'Empty member (array size 1)']),
        propertyEquals('article input', 'value', '1'),
        propertyEquals('article textarea', 'value', '<basket><apple>2</apple><pear>3</pear></basket>'),
        elementIdentity('article input', 'remember'), elementIdentity('article textarea', 'remember'),
        ...mapArrayPick('2', 'pear: 3'),
        ...['0', '3', '-1', '1.5', '1e0', 'bad', ''].flatMap(position => [
            ...mapArrayPick(position, 'No member at this position'), ...mapArrayPick('1', 'apple: 2'),
        ]),
        ...[[' 02 ', 'pear: 3'], ['+1', 'apple: 2'], ['001', 'apple: 2']].flatMap(([position, label]) => mapArrayPick(position, label)),
        ...mapArrayPick('3', 'No member at this position'),
        ...mapArrayEdit('article textarea', '<basket note="Fresh"><apple>2</apple><pear>3</pear><plum>4</plum></basket>',
            ['3', 'plum: 4', 'Note: Fresh']),
        propertyEquals('article input', 'value', '3'),
        ...mapArrayEdit('article input', '1', ['3', 'apple: 2', 'Note: Fresh']),
        ...mapArrayEdit('article textarea', '<basket xmlns:p="urn:fruit" note=" Fresh &amp; ripe "><!--skip--><?skip it?><p:plum> A<b>B</b><![CDATA[C]]><!--skip--><?skip it?> D </p:plum><pear>3</pear></basket>',
            ['2', 'plum:  ABC D ', 'Note:  Fresh & ripe ']),
        ...mapArrayEdit('article textarea', '<basket note=""/>', ['0', 'No member at this position', 'Note: ']),
        ...mapArrayEdit('article textarea', '<basket/>', ['0', 'No member at this position', 'Empty member (array size 1)']),
        ...mapArrayRecover(),
        ...['<basket>', '<other/>', '<basket xmlns="urn:fruit"><apple>2</apple></basket>'].flatMap(value => [
            ...mapArrayInvalid(value, value === '<basket>' ? 'XML' : 'Use a basket root'), ...mapArrayRecover(),
        ]),
        elementIdentity('article input', 'same'), propertyEquals('article input', 'value', '1'),
    ]),
    sampleContract('3. Query an imported JSON tree', [
        ...mapArrayOutputs(['3 members; numeric total 5', 'Null value']), elementIdentity('article textarea', 'remember'),
        ...[
            ['{"cherry":4,"note":""}', ['2 members; numeric total 4', 'Empty string']],
            ['{"plum":1e2}', ['1 members; numeric total 100', 'Absent member']],
            ['{"cherry":1.25,"pear":2.5,"note":"  Fresh 🍒  "}', ['3 members; numeric total 3.75', '  Fresh 🍒  ']],
            ['{"plum":-2,"pear":1e2,"note":null}', ['3 members; numeric total 98', 'Null value']],
            ['{"nested":{"pear":99},"array":[100],"flag":true,"text":"2","plum":4,"note":""}',
                ['6 members; numeric total 4', 'Empty string']],
            ['{}', ['0 members; numeric total 0', 'Absent member']],
        ].flatMap(([value, expected]) => mapArrayEdit('article textarea', value, expected)),
        ...['{', '[]', 'null', '42', '"fruit"'].flatMap(value => [
            ...mapArrayInvalid(value, value === '{' ? 'JSON' : 'Use a JSON object'), ...jsonMapRecover(),
        ]),
    ]),
];

const userCases = [
    ...['Ada', 'A123456789012345', 'a_9']
        .map(value => [value, 'Valid user name']),
    ...['', '  ', 'Ab', 'A1234567890123456', '7Ada', '_Ada', 'Ada-', 'Äda', 'Ada ', 'Ada\u00a0']
        .map(value => [value, 'Use 3–16 ASCII letters, digits or underscores; start with a letter']),
    ['Grace_2', 'Valid user name'],
];
const ageCases = [
    ...['18', '120', '018']
        .map(value => [value, 'Age in range']),
    ...['0', '17', '121', '999']
        .map(value => [value, 'Use an age from 18 to 120']),
    ...['', '  ', '1000', '18.0', '1e2', '+18', '-18', ' 18 ', '１８']
        .map(value => [value, 'Enter an integer age']),
    ['120', 'Age in range'],
];
const tagsCases = [
    ['A', 'A'],
    ['ABCDEFGHIJKL', 'ABCDEFGHIJKL'],
    ['  Red\t, GOLD ; blue  ', 'Red / GOLD / blue'],
    ...[
        '', '  ', 'ABCDEFGHIJKLM', 'red,,blue', ',red', 'red;', 'red, ;blue', 'red blue', 'blué', 'red1',
        'red\u00a0,blue',
    ]
        .map(value => [value, 'Use 1–12 ASCII letters per tag, separated by commas or semicolons']),
    ['Red; GOLD', 'Red / GOLD'],
];
const addressCases = [
    ...['255.255.255.255', '0.0.0.0', ' 192.0.2.10/24 ', '192.0.2.10/32']
        .map(value => [value, 'Allowed by the local prefix rule']),
    ['192.0.2.10/16', 'Blocked by the local prefix rule'],
    ...['256.0.2.10/24', '192.0.2.256', '192.0.2.10/33', '192.0.2.10/99']
        .map(value => [value, 'Octets must be 0–255 and prefix length 0–32']),
    ...[
        '', ' ', '192.00.2.10/24', '192.0.2.10/024', '192.0.2.10/00', '::1', '192.0.2', '192.0.2.10.1',
        '192.0.2.10/', '192.0.2.10/-1', '192.0.2.10/100', '192.0. 2.10', '192.0.2.10\u00a0',
    ]
        .map(value => [value, 'Enter IPv4 with an optional /prefix; no leading zeros']),
    ['192.0.2.10/24', 'Allowed by the local prefix rule'],
];
const prefixesCases = [
    ...['24', ' 24\t32 24 ', '+24 032', '024']
        .map(value => [value, 'Allowed by the local prefix rule']),
    ...['0', '32']
        .map(value => [value, 'Blocked by the local prefix rule']),
    ...['', ' ', '24 bad', '24 33', '24 -1', '24 1.0', '24 1e1', '24,32', '24\u00a032']
        .map(value => [value, 'Enter allowed prefix lengths from 0 to 32']),
    ['24 32', 'Allowed by the local prefix rule'],
];

const validationOutputs = expected => [
    countExactly('article output', expected.length),
    ...expected.map((value, index) => propertyEquals(`article p:nth-of-type(${index + 1}) output`, 'textContent', value)),
];
const validationEdit = (index, value, expected, outputIndex = index) => {
    const selector = `article label:nth-of-type(${index + 1}) input`;
    return [
        fillThenText(selector, value, `article p:nth-of-type(${outputIndex + 1}) output`, expected[outputIndex]),
        ...validationOutputs(expected), elementIdentity(selector, 'same'), focusedElement(selector),
        propertyEquals(selector, 'value', value), propertyEquals(selector, 'selectionStart', value.length),
        propertyEquals(selector, 'selectionEnd', value.length),
    ];
};
const validationFormChecks = () => {
    const expected = ['Valid user name', 'Age in range', 'Blue / green / RED'];
    const recovered = ['Valid user name', 'Age in range', 'Red / GOLD'];
    const recover = ['Grace_2', '120', 'Red; GOLD'];
    return [userCases, ageCases, tagsCases].flatMap((cases, index) => cases.flatMap(([value, result]) => {
        expected[index] = result;
        const checks = validationEdit(index, value, [...expected]);
        expected[index] = recovered[index];
        return [...checks, ...validationEdit(index, recover[index], [...expected])];
    }));
};
const validationIpEdit = (index, value, expected) => validationEdit(index, value, [expected], 0);
const xpathValidationSamples = [
    sampleContract('1. Form validation preview', [
        ...validationOutputs(['Valid user name', 'Age in range', 'Blue / green / RED']),
        ...['Ada_7', '21', 'Blue, green; RED'].flatMap((value, index) => [
            elementIdentity(`article label:nth-of-type(${index + 1}) input`, 'remember'),
            propertyEquals(`article label:nth-of-type(${index + 1}) input`, 'value', value),
        ]),
        ...validationFormChecks(),
        ...['Grace_2', '120', 'Red; GOLD'].map((value, index) =>
            propertyEquals(`article label:nth-of-type(${index + 1}) input`, 'value', value)),
    ]),
    sampleContract('2. IPv4 prefix-rule preview', [
        ...validationOutputs(['Allowed by the local prefix rule']),
        ...['192.0.2.10/24', '24 32'].flatMap((value, index) => [
            elementIdentity(`article label:nth-of-type(${index + 1}) input`, 'remember'),
            propertyEquals(`article label:nth-of-type(${index + 1}) input`, 'value', value),
        ]),
        ...addressCases.flatMap(([value, expected]) => [
            ...validationIpEdit(0, value, expected),
            propertyEquals('article label:last-of-type input', 'value', '24 32'),
            ...validationIpEdit(0, '192.0.2.10/24', 'Allowed by the local prefix rule'),
        ]),
        ...prefixesCases.flatMap(([value, expected]) => [
            ...validationIpEdit(1, value, expected),
            propertyEquals('article label:first-of-type input', 'value', '192.0.2.10/24'),
            ...validationIpEdit(1, '24 32', 'Allowed by the local prefix rule'),
        ]),
        ...validationIpEdit(1, '0 32', 'Blocked by the local prefix rule'),
        ...validationIpEdit(0, '0.0.0.0/0', 'Allowed by the local prefix rule'),
        ...validationIpEdit(1, '32', 'Blocked by the local prefix rule'),
        ...validationIpEdit(0, '0.0.0.0', 'Allowed by the local prefix rule'),
        ...validationIpEdit(1, 'bad', 'Enter allowed prefix lengths from 0 to 32'),
        ...validationIpEdit(0, '999.0.0.0', 'Octets must be 0–255 and prefix length 0–32'),
        ...validationIpEdit(0, '::1', 'Enter IPv4 with an optional /prefix; no leading zeros'),
        ...validationIpEdit(0, '192.0.2.10/24', 'Enter allowed prefix lengths from 0 to 32'),
        ...validationIpEdit(1, '24 32', 'Allowed by the local prefix rule'),
    ]),
];

const sortNumeric = 'article label:nth-of-type(2) input';
const sortDescending = 'article label:nth-of-type(3) input';
const sortDecimals = 'bad 2 NaN 02 INF +2 1e2 2.0 -0 0 -.5 .5';
const sortDecimalDescending = '2 / 02 / +2 / 2.0 / .5 / -0 / 0 / -.5 / bad / NaN / INF / 1e2';
const sortEditor = (selector, value) => [
    elementIdentity(selector, 'same'), focusedElement(selector), propertyEquals(selector, 'value', value),
    propertyEquals(selector, 'selectionStart', value.length), propertyEquals(selector, 'selectionEnd', value.length),
];
const sortWordEdit = (value, expected) => [
    fillThenText('article input[type=text]', value, 'article output', expected.replace(/\s+/gu, ' ').trim()),
    propertyEquals('article output', 'textContent', expected), ...sortEditor('article input[type=text]', value),
];
const sortWordToggle = (selector, checked, expected) => [
    (checked ? checkThenText : uncheckThenNormalizedText)(selector, 'article output', expected.replace(/\s+/gu, ' ').trim()),
    propertyEquals('article output', 'textContent', expected),
    elementIdentity(selector, 'same'), focusedElement(selector), propertyEquals(selector, 'checked', checked),
];
const sortInitialRows = [
    ['c', 'Cherry', 'A', '2'], ['d', 'Plum', 'A', '2'], ['b', 'Apple', 'A', '10'],
    ['a', 'Pear', 'B', '2'], ['e', 'Kiwi', 'A', 'bad'], ['f', 'Mango', 'B', '∅'],
];
const sortMixedXml = '<r><row id="b" group="B" qty="1">Birch</row><row id="c" group="A" qty="2">Citrus</row><row id="d" group="A" qty="02">Plum</row><row id="e" group="Z" qty="">Empty</row><row id="f" group="A">Missing</row><row id="g" group="A" qty="1e2">Exponent</row><row id="h" group="A" qty="NaN">NaN</row><row id="i" group="A" qty="INF">Infinity</row><row id="a" group="A" qty="-1.5">Apple</row><row id="j" qty="0">Zero</row></r>';
const sortMixedAscending = [
    ['j', 'Zero', '', '0'], ['a', 'Apple', 'A', '-1.5'], ['c', 'Citrus', 'A', '2'], ['d', 'Plum', 'A', '02'],
    ['b', 'Birch', 'B', '1'], ['e', 'Empty', 'Z', ''], ['f', 'Missing', 'A', '∅'], ['g', 'Exponent', 'A', '1e2'],
    ['h', 'NaN', 'A', 'NaN'], ['i', 'Infinity', 'A', 'INF'],
];
const sortMixedDescending = [sortMixedAscending[0], sortMixedAscending[2], sortMixedAscending[3], sortMixedAscending[1], ...sortMixedAscending.slice(4)];
const sortTable = (rows, selected, navigation) => [
    normalizedText('caption', 'Group and quantity'), nodeTexts('thead th[scope="col"]', ['Row', 'Group', 'Qty']),
    countExactly('tbody th[scope="row"]', rows.length), nodeTexts('tbody button', rows.map(row => row[1])),
    tableState('article table', rows.map(row => row.slice(2)), rows.flatMap((row, index) => row[0] === selected ? [index] : [])),
    ...rows.map((row, index) => propertyEquals(`tbody tr:nth-child(${index + 1}) button`, 'value', row[0])),
    countExactly('article output', navigation.length),
    ...navigation.map((value, index) => propertyEquals(`article output:nth-of-type(${index + 1})`, 'textContent', value)),
    countExactly('[role="alert"]', 0),
];
const sortXmlEdit = (value, rows, selected, navigation) => [
    fillThenText('article textarea', value, 'caption', 'Group and quantity'), ...sortTable(rows, selected, navigation),
    ...sortEditor('article textarea', value), elementIdentity('article input[type=checkbox]', 'same'),
    propertyEquals('article input[type=checkbox]', 'checked', true),
];
const sortRecover = () => sortXmlEdit('<r><row id="b" group="A" qty="1">Recovered</row></r>',
    [['b', 'Recovered', 'A', '1']], 'b', ['Recovered', '']);
const xpathSortSamples = [
    sampleContract('1. Text and numeric keys', [
        propertyEquals('article output', 'textContent', '02 / 1 / 10 / 2 / bad'),
        propertyEquals('article input[type=text]', 'value', '10 2 02 bad 1'),
        propertyEquals(sortNumeric, 'checked', false), propertyEquals(sortDescending, 'checked', false),
        ...['article input[type=text]', sortNumeric, sortDescending].map(selector => elementIdentity(selector, 'remember')),
        ...sortWordToggle(sortNumeric, true, '1 / 2 / 02 / 10 / bad'),
        ...sortWordToggle(sortDescending, true, '10 / 2 / 02 / 1 / bad'),
        ...sortWordToggle(sortNumeric, false, 'bad / 2 / 10 / 1 / 02'),
        ...sortWordToggle(sortDescending, false, '02 / 1 / 10 / 2 / bad'),
        ...sortWordEdit(sortDecimals, '+2 / -.5 / -0 / .5 / 0 / 02 / 1e2 / 2 / 2.0 / INF / NaN / bad'),
        ...sortWordToggle(sortDescending, true, 'bad / NaN / INF / 2.0 / 2 / 1e2 / 02 / 0 / .5 / -0 / -.5 / +2'),
        ...sortWordToggle(sortNumeric, true, sortDecimalDescending),
        ...sortWordToggle(sortDescending, false, '-.5 / -0 / 0 / .5 / 2 / 02 / +2 / 2.0 / bad / NaN / INF / 1e2'),
        ...sortWordEdit('b\u00a0a a A 🍒 A', 'b\u00a0a / a / A / 🍒 / A'),
        ...sortWordToggle(sortDescending, true, 'b\u00a0a / a / A / 🍒 / A'),
        ...sortWordToggle(sortNumeric, false, '🍒 / b\u00a0a / a / A / A'),
        ...sortWordToggle(sortDescending, false, 'A / A / a / b\u00a0a / 🍒'),
        ...sortWordEdit('', ''), ...sortWordToggle(sortNumeric, true, ''), ...sortWordToggle(sortDescending, true, ''),
        ...sortWordEdit('   ', ''), ...sortWordEdit(sortDecimals, sortDecimalDescending),
        propertyEquals(sortNumeric, 'checked', true), propertyEquals(sortDescending, 'checked', true),
    ]),
    sampleContract('2. Multiple keys and source selection', [
        ...sortTable(sortInitialRows, '', []), propertyEquals('article input[type=checkbox]', 'checked', false),
        elementIdentity('article textarea', 'remember'), elementIdentity('article input[type=checkbox]', 'remember'),
        clickThenText('article button[value="c"]', 'article output:first-of-type', 'Cherry'),
        ...sortTable(sortInitialRows, 'c', ['Cherry', 'Apple']),
        checkThenText('article input[type=checkbox]', 'article tbody tr:first-child button', 'Apple'),
        ...sortTable([sortInitialRows[2], sortInitialRows[0], sortInitialRows[1], ...sortInitialRows.slice(3)], 'c', ['Cherry', 'Apple']),
        elementIdentity('article input[type=checkbox]', 'same'), focusedElement('article input[type=checkbox]'),
        ...sortXmlEdit(sortMixedXml, sortMixedDescending, 'c', ['Citrus', 'Birch']),
        uncheckThenNormalizedText('article input[type=checkbox]', 'article tbody tr:nth-child(2) button', 'Apple'),
        ...sortTable(sortMixedAscending, 'c', ['Citrus', 'Birch']),
        elementIdentity('article input[type=checkbox]', 'same'), focusedElement('article input[type=checkbox]'),
        propertyEquals('article input[type=checkbox]', 'checked', false),
        checkThenText('article input[type=checkbox]', 'article tbody tr:nth-child(2) button', 'Citrus'),
        ...sortTable(sortMixedDescending, 'c', ['Citrus', 'Birch']),
        elementIdentity('article input[type=checkbox]', 'same'), focusedElement('article input[type=checkbox]'),
        propertyEquals('article input[type=checkbox]', 'checked', true),
        ...[['j', 'Zero', 'Apple'], ['e', 'Empty', 'Plum'], ['f', 'Missing', 'Empty'], ['b', 'Birch', '']].flatMap(([id, label, previous]) => [
            clickThenText(`article button[value="${id}"]`, 'article output:first-of-type', label),
            ...sortTable(sortMixedDescending, id, [label, previous]),
        ]),
        ...sortXmlEdit('<r><row id="z" group="A" qty="1">Zed</row></r>', [['z', 'Zed', 'A', '1']], '', []),
        ...['<r/>', '<r xmlns="urn:other"><row id="b" qty="1">Qualified</row></r>', '<other><row id="b">Other</row></other>']
            .flatMap(value => [...sortXmlEdit(value, [], '', []), ...sortRecover()]),
        ...['<r>', '<r><row></r>'].flatMap(value => [
            fillThenText('article textarea', value, '[role="alert"]', 'XML'), countExactly('table', 0), countExactly('article output', 0),
            ...sortEditor('article textarea', value), ...sortRecover(),
        ]),
        ...sortXmlEdit(sortMixedXml, sortMixedDescending, 'b', ['Birch', '']),
    ]),
];

const aggregateOutputs = expected => [
    nodeTexts('article output', expected), countExactly('article [role="alert"]', 0),
];
const aggregateRows = rows => [
    normalizedText('article caption', 'Fruit amounts'),
    nodeTexts('article thead th[scope="col"]', ['Fruit', 'Amount']),
    countExactly('article tbody tr', rows.length),
    nodeTexts('article tbody th[scope="row"]', rows.map(([name]) => name)),
    nodeTexts('article tbody td', rows.map(([, amount]) => amount)),
];
const aggregateEditor = value => [
    elementIdentity('article textarea', 'same'), focusedElement('article textarea'),
    propertyEquals('article textarea', 'value', value),
    propertyEquals('article textarea', 'selectionStart', value.length),
    propertyEquals('article textarea', 'selectionEnd', value.length),
];
const aggregateEdit = (value, expected, rows) => [
    fillThenText('article textarea', value, 'article p:first-of-type output', expected[0]),
    ...aggregateOutputs(expected), ...aggregateEditor(value),
    ...(rows ? aggregateRows(rows) : []),
];
const aggregateInvalid = (value, message, basket = false) => [
    fillThenText('article textarea', value, 'article [role="alert"]', message),
    countExactly('article output', 0), ...aggregateEditor(value),
    ...(basket ? [countExactly('article table', 0)] : []),
];
const xpathAggregateSamples = [
    sampleContract('1. Decimal sequence statistics', [
        ...aggregateOutputs(['0.3', '0.1', '0.2', '0.15']),
        propertyEquals('article textarea', 'value', '0.1 0.2'),
        elementIdentity('article textarea', 'remember'),
        ...aggregateEdit('-2\t1\n4', ['3', '-2', '4', '1']),
        ...aggregateEdit('+001.20 -.20 0', ['1', '-0.2', '1.2', '0.333333333333333333']),
        ...aggregateEdit('0 0 1', ['1', '0', '1', '0.333333333333333333']),
        ...aggregateEdit('0 1 1', ['2', '0', '1', '0.666666666666666667']),
        ...aggregateEdit('0 -1 -1', ['-2', '-1', '0', '-0.666666666666666667']),
        ...aggregateEdit('-0 .5 5.', ['5.5', '0', '5', '1.83333333333333333']),
        ...['1 bad', 'NaN', '1e2', '1\u00a02'].flatMap(value => [
            ...aggregateInvalid(value, 'Enter decimal'),
            ...aggregateEdit('0.1 0.2', ['0.3', '0.1', '0.2', '0.15']),
        ]),
        ...aggregateEdit('', ['0', '∅', '∅', '∅']),
        ...aggregateEdit(' \t\n', ['0', '∅', '∅', '∅']),
    ]),
    sampleContract('2. A basket that accepts new fruits', [
        ...aggregateOutputs(['3.75', '1.25', '2.5', '1.875']),
        ...aggregateRows([['apple', '1.25'], ['cherry', '2.5']]),
        elementIdentity('article textarea', 'remember'),
        ...aggregateEdit('<basket><apple>0.1</apple><cherry>0.2</cherry><pear>0.6</pear></basket>',
            ['0.9', '0.1', '0.6', '0.3'], [['apple', '0.1'], ['cherry', '0.2'], ['pear', '0.6']]),
        ...aggregateEdit('<basket><pear>+001.20</pear><lime>-0</lime></basket>',
            ['1.2', '0', '1.2', '0.6'], [['pear', '1.2'], ['lime', '0']]),
        ...['<basket><pear>bad</pear></basket>', '<basket><pear/></basket>',
            '<basket><pear>-1</pear></basket>', '<basket><pear><qty>1</qty></pear></basket>',
            '<other><pear>1</pear></other>', '<basket>'].flatMap(value => [
            ...aggregateInvalid(value, value === '<basket>' ? 'XML' : 'non-negative decimal', true),
            ...aggregateEdit('<basket><plum>7</plum></basket>', ['7', '7', '7', '7'], [['plum', '7']]),
        ]),
        ...aggregateEdit('<basket/>', ['0', '∅', '∅', '∅'], []),
        ...aggregateEdit('<basket><plum>7</plum></basket>', ['7', '7', '7', '7'], [['plum', '7']]),
    ]),
];

const sequenceStart = 'article label:nth-child(1) input[type="text"]';
const sequenceLength = 'article label:nth-child(2) input[type="text"]';
const sequenceOutputs = expected => [
    countExactly('article output', expected.length), countExactly('article [role="alert"]', 0),
    ...expected.map((value, index) => propertyEquals(`article p:nth-of-type(${index + 2}) output`, 'textContent', value)),
];
const sequenceEditor = (selector, value) => [
    elementIdentity(selector, 'same'), focusedElement(selector), propertyEquals(selector, 'value', value),
    propertyEquals(selector, 'selectionStart', value.length), propertyEquals(selector, 'selectionEnd', value.length),
];
const sequenceEdit = (selector, value, expected) => [
    fillThenText(selector, value, 'article output', ''), ...sequenceOutputs(expected), ...sequenceEditor(selector, value),
];
const sequenceBounds = (start, length, expected) => [
    fillThenText(sequenceStart, start, 'article output', '5'), ...sequenceEditor(sequenceStart, start),
    ...sequenceEdit(sequenceLength, length, expected), propertyEquals(sequenceStart, 'value', start),
    propertyEquals('article input[type="checkbox"]', 'checked', false),
];
const sequenceInitialHeadings = ['@id', 'fruit', '@qty', 'note'];
const sequenceInitialRows = [['2', 'Apple', '∅', '∅'], ['1', 'Cherry', '3', '""']];
const sequenceKindsXml = '<r xmlns:x="urn:x" xmlns:y="urn:x"><row fruit="attr" x:fruit="ns"><fruit/><y:fruit>B</y:fruit></row><row><fruit>C</fruit><fruit>D</fruit><x:fruit/></row></r>';
const sequenceKindsHeadings = ['@fruit', '@fruit [urn:x]', 'fruit', 'fruit [urn:x]'];
const sequenceKindsRows = [['attr', 'ns', '""', 'B'], ['∅', '∅', 'C / D', '""']];
const sequenceTable = (headings, rows) => [
    normalizedText('caption', 'Columns in first-seen order'), nodeTexts('th[scope="col"]', headings),
    countExactly('th', headings.length), countExactly('tbody tr', rows.length),
    ...rows.flatMap((row, i) => [
        countExactly(`tbody tr:nth-child(${i + 1}) td`, row.length),
        ...row.map((value, j) => propertyEquals(`tbody tr:nth-child(${i + 1}) td:nth-child(${j + 1})`, 'textContent', value)),
    ]),
    countExactly('[role="alert"]', 0),
];
const sequenceXmlEdit = (value, headings, rows) => [
    fillThenText('article textarea', value, 'caption', 'Columns in first-seen order'),
    ...sequenceTable(headings, rows), ...sequenceEditor('article textarea', value),
];
const sequenceRecover = () => sequenceXmlEdit('<r><row><fruit>Recovered</fruit></row></r>', ['fruit'], [['Recovered']]);
const xpathSequenceSamples = [
    sampleContract('1. A window into a word sequence', [
        ...sequenceOutputs(['3', 'apple | cherry', 'apple', 'cherry']),
        propertyEquals('article textarea', 'value', 'cherry apple cherry pear'),
        propertyEquals(sequenceStart, 'value', '2'), propertyEquals(sequenceLength, 'value', '2'),
        propertyEquals('article input[type="checkbox"]', 'checked', false),
        ...['article textarea', sequenceStart, sequenceLength, 'article input[type="checkbox"]']
            .map(selector => elementIdentity(selector, 'remember')),
        checkThenText('article input[type="checkbox"]', 'article p:nth-of-type(3) output', 'cherry | apple'),
        ...sequenceOutputs(['3', 'cherry | apple', 'cherry', 'apple']),
        elementIdentity('article input[type="checkbox"]', 'same'), focusedElement('article input[type="checkbox"]'),
        propertyEquals('article input[type="checkbox"]', 'checked', true),
        ...sequenceEdit('article textarea', '🍒 a a b', ['3', 'a | a', 'a', 'a']),
        propertyEquals('article input[type="checkbox"]', 'checked', true),
        uncheckThenNormalizedText('article input[type="checkbox"]', 'article p:nth-of-type(3) output', 'a | a'),
        ...sequenceOutputs(['3', 'a | a', 'a', 'a']),
        elementIdentity('article input[type="checkbox"]', 'same'), focusedElement('article input[type="checkbox"]'),
        propertyEquals('article input[type="checkbox"]', 'checked', false),
        ...sequenceEdit('article textarea', 'a b c d e', ['5', 'b | c', 'b', 'c']),
        ...[
            ['2.5', '1.5', ['5', 'c | d', 'c', 'd']], ['0', '3', ['5', 'a | b', 'a', 'b']],
            ['-1', '3', ['5', 'a', 'a', '']], ['-1.5', '3', ['5', 'a', 'a', '']],
            ['-0.5', '2', ['5', 'a', 'a', '']], ['1', '0', ['5', '', '', '']],
            ['1', '-2', ['5', '', '', '']], ['6', '2', ['5', '', '', '']], ['NaN', '2', ['5', '', '', '']],
            ['1', 'INF', ['5', 'a | b | c | d | e', 'a', 'b | c | d | e']], ['-INF', 'INF', ['5', '', '', '']],
        ].flatMap(([start, length, expected]) => sequenceBounds(start, length, expected)),
        ...sequenceBounds('1', '2', ['5', 'a | b', 'a', 'b']),
        ...[sequenceStart, sequenceLength].flatMap((selector, index) => ['oops', ''].flatMap(value => [
            fillThenText(selector, value, 'article [role="alert"]', 'Enter numeric start and length values.'),
            countExactly('article output', 1), propertyEquals('article output', 'textContent', '5'),
            ...sequenceEditor(selector, value), ...sequenceEdit(selector, index === 0 ? '1' : '2', ['5', 'a | b', 'a', 'b']),
        ])),
        ...sequenceBounds('1', '10', ['5', 'a | b | c | d | e', 'a', 'b | c | d | e']),
        ...[
            ['', ['0', '', '', '']], [' \t\n', ['0', '', '', '']],
            ['é e\u0301 É é', ['3', 'é | e\u0301 | É | é', 'é', 'e\u0301 | É | é']],
            ['a\u00a0b a\u00a0b', ['1', 'a\u00a0b | a\u00a0b', 'a\u00a0b', 'a\u00a0b']],
            ['\tapple\ncherry\tapple\n', ['2', 'apple | cherry | apple', 'apple', 'cherry | apple']],
        ].flatMap(([value, expected]) => sequenceEdit('article textarea', value, expected)),
        propertyEquals('article input[type="checkbox"]', 'checked', false),
    ]),
    sampleContract('2. First-seen XML columns', [
        ...sequenceTable(sequenceInitialHeadings, sequenceInitialRows), elementIdentity('article textarea', 'remember'),
        ...sequenceXmlEdit('<r xmlns:x="urn:x"><row id="1"><fruit>A</fruit><x:fruit>B</x:fruit><fruit>C</fruit></row><row extra="new"><fruit>D</fruit></row></r>',
            ['@id', 'fruit', 'fruit [urn:x]', '@extra'], [['1', 'A / C', 'B', '∅'], ['∅', 'D', '∅', 'new']]),
        ...sequenceXmlEdit(sequenceKindsXml, sequenceKindsHeadings, sequenceKindsRows),
        ...sequenceXmlEdit('<r><row><fruit> A<b>B</b><![CDATA[C]]><!--skip--><?skip it?> D </fruit><fruit/></row><row flag=""><fruit/><fruit/></row><row/></r>',
            ['fruit', '@flag'], [[' ABC D  / ', '∅'], [' / ', '""'], ['∅', '∅']]),
        ...sequenceXmlEdit('<r><row b="2" a="1"><z>Z</z><a>A</a></row><row c="3"><z/></row></r>',
            ['@b', '@a', 'z', 'a', '@c'], [['2', '1', 'Z', 'A', '∅'], ['∅', '∅', '""', '∅', '3']]),
        ...sequenceXmlEdit('<r><row/></r>', [], [[]]), ...sequenceXmlEdit('<r/>', [], []), ...sequenceRecover(),
        ...['<r>', '<r><row></r>'].flatMap(value => [
            fillThenText('article textarea', value, '[role="alert"]', 'XML'), countExactly('table', 0),
            ...sequenceEditor('article textarea', value), ...sequenceRecover(),
        ]),
        ...sequenceXmlEdit(sequenceKindsXml, sequenceKindsHeadings, sequenceKindsRows),
    ]),
];

const nodeTableInitialRows = [['1', 'item urn:b', 'Apple', '2'], ['2', 'item urn:a', 'Zest', '10']];
const nodeTableMixedXml = '<crate xmlns:p="urn:fruit"><p:item id="a" qty="02">Pear</p:item>gap<!--skip--><?skip it?><item id="b"> A<b>B</b><![CDATA[C]]> </item><item id="c" qty="3">Pear</item></crate>';
const nodeTableMixedRows = [['b', 'item', 'ABC', ''], ['a', 'item urn:fruit', 'Pear', '02'], ['c', 'item', 'Pear', '3']];
const nodeTableDescendingRows = [nodeTableMixedRows[1], nodeTableMixedRows[2], nodeTableMixedRows[0]];
const nodeTreeMixedXml = '<r xmlns:p="urn:new" p:mood="calm">Hi<![CDATA[ there]]><!--remark--><?say yes?><p:em>🍒</p:em>!</r>';
const nodeTreeUpdatedXml = nodeTreeMixedXml.replace('Hi<![CDATA[ there]]>', 'Updated<![CDATA[ text]]>');
const nodeTreeUpdatedCodes = ['calm', 'Updated text', 'remark', 'yes', '🍒', '!'];
const nodeTreeRoot = 'article > ul > li > details';
const nodeTable = (rows, selected, navigation) => [
    normalizedText('caption', 'Basket nodes'), nodeTexts('thead th[scope="col"]', ['Select', 'Node', 'Value', 'Qty']),
    countExactly('tbody tr', rows.length), nodeTexts('tbody td', rows.flat()),
    ...rows.flatMap(([id], index) => [
        attributeEquals(`tbody tr:nth-child(${index + 1})`, 'aria-selected', String(id === selected)),
        attributeEquals(`tbody tr:nth-child(${index + 1}) button`, 'aria-pressed', String(id === selected)),
        attributeEquals(`tbody tr:nth-child(${index + 1}) button`, 'aria-label', `Select row ${id}`),
        propertyEquals(`tbody tr:nth-child(${index + 1}) button`, 'value', id),
    ]),
    countExactly('output', navigation.length),
    ...navigation.map((value, index) => propertyEquals(`output:nth-of-type(${index + 1})`, 'textContent', value)),
    countExactly('[role="alert"]', 0),
];
const nodeEditor = value => [
    elementIdentity('textarea', 'same'), focusedElement('textarea'), propertyEquals('textarea', 'value', value),
    propertyEquals('textarea', 'selectionStart', value.length), propertyEquals('textarea', 'selectionEnd', value.length),
];
const nodeTableEdit = (value, rows, selected, navigation) => [
    fillThenText('textarea', value, 'caption', 'Basket nodes'), ...nodeTable(rows, selected, navigation), ...nodeEditor(value),
    elementIdentity('select', 'same'),
];
// Code leaves occur at different depths, so compare their complete document order.
const nodeTreeEdit = (value, codes) => [
    fillThenText('textarea', value, 'article summary', ''), nodeTexts('article code', codes),
    countExactly('[role="alert"]', 0), ...nodeEditor(value),
];
const nodeInvalid = value => [
    fillThenText('textarea', value, '[role="alert"]', 'XML'), countExactly('article :is(table, ul, output)', 0), ...nodeEditor(value),
];
const nodeTableRecover = () => nodeTableEdit('<crate><item id="c" qty="4">Recovered</item></crate>',
    [['c', 'item', 'Recovered', '4']], 'c', ['crate', '']);
const xpathNodeSamples = [
    sampleContract('1. XML table with native navigation', [
        ...nodeTable(nodeTableInitialRows, '', []), propertyEquals('select', 'value', 'ascending'),
        elementIdentity('textarea', 'remember'), elementIdentity('select', 'remember'),
        clickThenText('button[value="1"]', 'output:last-child', 'Zest'),
        ...nodeTable(nodeTableInitialRows, '1', ['basket', 'Zest']),
        focusThenText('select', 'caption', 'Basket nodes'),
        selectThenText('select', 'descending', 'tbody tr:first-child td:nth-child(3)', 'Zest'),
        ...nodeTable([...nodeTableInitialRows].reverse(), '1', ['basket', 'Zest']),
        elementIdentity('select', 'same'), focusedElement('select'), propertyEquals('select', 'value', 'descending'),
        clickThenText('button[value="2"]', 'output:first-of-type', 'basket'),
        ...nodeTable([...nodeTableInitialRows].reverse(), '2', ['basket', '']),
        ...nodeTableEdit(nodeTableMixedXml, nodeTableDescendingRows, '', []),
        propertyEquals('select', 'value', 'descending'),
        clickThenText('button[value="c"]', 'output:first-of-type', 'crate'),
        ...nodeTable(nodeTableDescendingRows, 'c', ['crate', ' ABC ']),
        focusThenText('select', 'caption', 'Basket nodes'),
        selectThenText('select', 'ascending', 'tbody tr:first-child td:nth-child(3)', 'ABC'),
        ...nodeTable(nodeTableMixedRows, 'c', ['crate', ' ABC ']),
        propertyEquals('tbody tr:first-child td:nth-child(3)', 'textContent', ' ABC '),
        elementIdentity('select', 'same'), focusedElement('select'), propertyEquals('select', 'value', 'ascending'),
        clickThenText('button[value="b"]', 'output:last-child', 'Pear'),
        ...nodeTable(nodeTableMixedRows, 'b', ['crate', 'Pear']),
        clickThenText('button[value="c"]', 'output:last-child', 'ABC'),
        ...nodeTable(nodeTableMixedRows, 'c', ['crate', ' ABC ']),
        ...nodeTableEdit('<crate/>', [], '', []), ...nodeTableRecover(),
        ...['<crate>', '<crate><item></crate>'].flatMap(value => [...nodeInvalid(value), ...nodeTableRecover()]),
        ...nodeTableEdit(nodeTableMixedXml, nodeTableMixedRows, 'c', ['crate', ' ABC ']),
    ]),
    sampleContract('2. XML tree with attributes and mixed text', [
        nodeTexts('summary strong', ['note', 'em']), nodeTexts('summary small', ['[urn:notes]', '[urn:marks]']),
        nodeTexts('article code', ['bright', 'Hello & welcome', 'friend', '!']),
        propertyEquals(nodeTreeRoot, 'open', true), propertyEquals(`${nodeTreeRoot} details`, 'open', true),
        elementIdentity('textarea', 'remember'),
        ...[`${nodeTreeRoot} details`, nodeTreeRoot].flatMap(selector => [
            elementIdentity(selector, 'remember'),
            pressThenProperty(`${selector} > summary`, 'Enter', selector, 'open', false),
            elementIdentity(selector, 'same'), focusedElement(`${selector} > summary`),
            pressThenProperty(`${selector} > summary`, 'Space', selector, 'open', true),
            elementIdentity(selector, 'same'), focusedElement(`${selector} > summary`),
        ]),
        ...nodeTreeEdit(nodeTreeMixedXml, ['calm', 'Hi there', 'remark', 'yes', '🍒', '!']),
        nodeTexts('summary strong', ['r', 'em']), nodeTexts('summary small', ['[]', '[urn:new]']),
        nodeTexts('article li:not(:has(details))', [
            '@mood [urn:new] = calm', 'text: Hi there', 'comment: remark', 'processing instruction: yes', 'text: 🍒', 'text: !',
        ]),
        elementIdentity(nodeTreeRoot, 'remember'),
        pressThenProperty(`${nodeTreeRoot} > summary`, 'Enter', nodeTreeRoot, 'open', false),
        ...nodeTreeEdit(nodeTreeUpdatedXml, nodeTreeUpdatedCodes),
        elementIdentity(nodeTreeRoot, 'same'), propertyEquals(nodeTreeRoot, 'open', false),
        pressThenProperty(`${nodeTreeRoot} > summary`, 'Space', nodeTreeRoot, 'open', true),
        focusedElement(`${nodeTreeRoot} > summary`),
        ...nodeTreeEdit('<r/>', []), nodeTexts('summary strong', ['r']), nodeTexts('summary small', ['[]']),
        ...nodeTreeEdit('<r>Recovered</r>', ['Recovered']),
        ...['<broken>', '<r><em></r>'].flatMap(value => [...nodeInvalid(value), ...nodeTreeEdit('<r>Recovered</r>', ['Recovered'])]),
        ...nodeTreeEdit(nodeTreeUpdatedXml, nodeTreeUpdatedCodes),
    ]),
];

const scalarReferrerSample = sampleContract('src by scalar referrer matrix', [
    nodeTexts('thead th', ['referrer / src', 'relative URL', 'module path', 'absolute URL']),
    nodeTexts('tbody th', ['relative URL', 'module path', 'absolute URL']),
    countExactly('thead th[scope="col"]', 4),
    countExactly('tbody th[scope="row"]', 3),
    countExactly('tbody tr', 3),
    {
        kind: 'resolvedUrlTexts', selector: 'tbody td', expected: [
            '/packages/cem-elements/demo/lib-dir/Smiley.svg?case=relative-relative',
            '/packages/cem-elements/demo/lib-dir/Smiley.svg?referrer=relative',
            'https://assets.example.test/logo.svg',
            '/packages/cem-elements/demo/lib-dir/Smiley.svg?case=relative-module',
            '/packages/cem-elements/demo/confused.svg?referrer=module',
            'https://assets.example.test/logo.svg',
            'https://referrer.example.test/lib-dir/Smiley.svg?case=relative-absolute',
            '/packages/cem-elements/demo/wc-square.svg?referrer=absolute',
            'https://assets.example.test/logo.svg',
        ],
    },
    countExactly('cem-module-url', 0),
]);

function locationReader(selector, mode) {
    return { kind: 'locationReader', selector, mode };
}
const locationSamples = [
    sampleContract('1. Window location live update', [
        { kind: 'locationSnapshot' },
        locationReader(':scope', 'live'),
        elementIdentity('article', 'remember'),
        elementIdentity('input', 'remember'),
        propertyEquals('input', 'value', 'hello world'),
        propertyEquals('form', 'method', 'get'),
        attributeEquals('input', 'name', 'query'),
        clickThenText('button:text-is("history.pushState")', 'dl', '#checked'),
        locationReader(':scope', 'live'),
        nodeTexts('li', ['mode = history.pushState', 'tag = one,two']),
        clickThenText('button:text-is("history.replaceState")', 'dl', '#checked'),
        locationReader(':scope', 'live'),
        nodeTexts('li', ['mode = history.replaceState', 'tag = one,two']),
        urlEquals('a', 'href', '#native-link'),
        elementIdentity('article', 'same'),
        elementIdentity('input', 'same'),
    ]),
    sampleContract('2. Window location initial read', [
        locationReader(':scope', 'initial'),
        elementIdentity('article', 'remember'),
        clickThenText('button[aria-label="Change hash after initial read"]', 'dl', 'source window'),
        locationReader(':scope', 'initial'),
        elementIdentity('article', 'same'),
    ]),
    sampleContract('3. External URL from href', [
        locationReader(':scope', 'external'),
        nodeTexts('li', ['a = 1', 'b = 2,3']),
    ]),
];

function storageValue(key, expected) {
    return { kind: 'storageValue', key, expected };
}
function storageOutputs(key, raw, outputs) {
    return [storageValue(key, raw), nodeTexts('output', outputs)];
}
function submitStorageRaw(key, raw, outputs) {
    return [fillThenText('input[name="raw"]', raw, 'form button', 'Write'),
        clickThenText('form button', 'output:last-of-type', outputs.at(-1)),
        ...storageOutputs(key, raw, outputs), propertyEquals('input[name="raw"]', 'value', raw)];
}
function storageEditorState(value) {
    return [storageValue('cemDemoSliceEditor', value), formState({ inputs: [value, value], outputs: [value, value] }),
        elementIdentity('cem-storage-editor:first-of-type input', 'same'),
        elementIdentity('cem-storage-editor:last-of-type input', 'same')];
}
function storageBasketState(cherries, lemons) {
    return [nodeTexts('dd', [String(cherries), String(lemons), String(cherries + lemons)]),
        storageValue('cemDemoBasket', `{"cherries":${cherries},"lemons":${lemons}}`)];
}
function storageDateChecks(key, initial, invalid, recovery) {
    return [
        elementIdentity('input[name="raw"]', 'remember'),
        ...storageOutputs(key, initial, [initial]),
        clickThenText('button:has-text("invalid")', 'output', 'null'),
        ...storageOutputs(key, invalid, ['null']),
        ...[recovery, '', initial].flatMap(raw => submitStorageRaw(key, raw, [raw || 'null'])),
        elementIdentity('input[name="raw"]', 'same'),
        ...(key === 'cemDemoDate' ? [clickThenText('button:text-is("ISO timestamp")', 'output', '2024-04-21'),
            ...storageOutputs(key, '2024-04-21T03:58:42.131Z', ['2024-04-21'])] : []),
    ];
}
const localStorageSamples = [
    sampleContract('0. Read a live text value', [
        ...storageOutputs('cemDemoLiveText', 'stored initial', ['stored initial']),
        ...[['text value', 'text value'], ['another value', 'another value'], ['Empty string', ''],
            ['Clear key', null], ['text value', 'text value']].flatMap(([button, raw]) => [
            clickThenText(`button:text-is("${button}")`, 'output', raw ?? 'null'),
            ...storageOutputs('cemDemoLiveText', raw, [raw ?? 'null']),
        ]),
    ]),
    sampleContract('1. Always override a stored value', [
        ...storageOutputs('cemDemoOverride', 'ABC', ['ABC']),
        ...['Try text value', 'Clear key'].flatMap(button => [
            clickThenText(`button:text-is("${button}")`, 'output', 'ABC'),
            ...storageOutputs('cemDemoOverride', 'ABC', ['ABC']),
        ]),
    ]),
    sampleContract('2. Stored value with a default', [
        ...storageOutputs('cemDemoPersistedDefault', 'DEF', ['DEF']),
        ...[['remember me', 'remember me'], ['Empty string', ''], ['Clear key', null],
            ['remember me', 'remember me']].flatMap(([button, raw]) => [
            clickThenText(`button:text-is("${button}")`, 'output', raw ?? 'null'),
            ...storageOutputs('cemDemoPersistedDefault', raw, [raw ?? 'null']),
        ]),
    ]),
    sampleContract('3a. Date validation', storageDateChecks('cemDemoDate', '2024-04-20', 'ABC', '2024-02-29')),
    sampleContract('3b. Time validation', storageDateChecks('cemDemoTime', '13:30', '25:00', '09:15')),
    sampleContract('3c. Local date and time validation',
        storageDateChecks('cemDemoLocalDateTime', '1977-04-01T14:00:30', 'ABC', '2024-04-20T09:15')),
    sampleContract('3d. Number validation', [
        ...storageOutputs('cemDemoNumber', '1.23456e+5', ['1.23456e+5', '123456']),
        ...[['0001', '0001', '1'], ['0', '0', '0'], ['ABC — invalid', 'ABC', 'null'],
            ['1.23456e+5', '1.23456e+5', '123456']].flatMap(([button, raw, parsed]) => [
            clickThenText(`button:text-is("${button}")`, 'p:nth-of-type(2) output', parsed),
            ...storageOutputs('cemDemoNumber', raw, [raw, parsed]),
        ]),
        ...submitStorageRaw('cemDemoNumber', '-2.5', ['-2.5', '-2.5']),
        ...submitStorageRaw('cemDemoNumber', '', ['', 'null']),
    ]),
    sampleContract('3e. JSON validation', [
        ...storageOutputs('cemDemoJson', '{"a":1,"b":"B"}', ['{"a":1,"b":"B"}', 'object']),
        nodeTexts('ul li', ['a: 1', 'b: B']),
        ...[['JSON string', '"ABC"', 'ABC'], ['JSON number', '12.345', '12.345'], ['JSON false', 'false', 'false'],
            ['ABC — invalid', 'ABC', 'null']].flatMap(([button, raw, parsed]) => [
            clickThenText(`button:text-is("${button}")`, 'p:nth-of-type(2) output', parsed),
            ...storageOutputs('cemDemoJson', raw, [raw, parsed]), countExactly('li', 0),
        ]),
        ...[['0', '0'], ['true', 'true'], ['null', 'null'], ['""', ''], ['{}', 'object'], ['[]', 'array']]
            .flatMap(([raw, parsed]) => [...submitStorageRaw('cemDemoJson', raw, [raw, parsed]), countExactly('li', 0)]),
        ...submitStorageRaw('cemDemoJson', '[1,2,3]', ['[1,2,3]', 'array']),
        nodeTexts('ol li', ['1', '2', '3']), countExactly('ul', 0),
        ...submitStorageRaw('cemDemoJson', '{"fruit":"cherry","count":0}', ['{"fruit":"cherry","count":0}', 'object']),
        nodeTexts('ul li', ['fruit: cherry', 'count: 0']), countExactly('ol', 0),
        clickThenText('button:text-is("Object")', 'ul', 'b : B'),
        ...storageOutputs('cemDemoJson', '{"a":1,"b":"B"}', ['{"a":1,"b":"B"}', 'object']),
        nodeTexts('ul li', ['a: 1', 'b: B']),
    ]),
    sampleContract('4. Simplest initial read', [
        elementIdentity('output', 'remember'),
        normalizedText('cem-storage-cherries', '12 🍒'),
        ...['24', '12', '24'].flatMap(value => [
            clickThenText(`button[aria-label="Store ${value} cherries"]`, 'output', '12'),
            storageValue('cemDemoCherries', value), nodeTexts('output', ['12']), elementIdentity('output', 'same'),
        ]),
    ]),
    sampleContract('5. Live JSON basket', [
        ...storageBasketState(12, 1),
        clickThenText('button[aria-label="Add cherry"]', 'dl', '🛒 14'), ...storageBasketState(13, 1),
        clickThenText('button[aria-label="Add lemon"]', 'dl', '🛒 15'), ...storageBasketState(13, 2),
        clickThenText('button[aria-label="Reset basket"]', 'dl', '🛒 13'), ...storageBasketState(12, 1),
    ]),
    sampleContract('6. Fruit buttons and a storage watcher', [
        nodeTexts('dd', ['1', '12', '0', '0', '13']),
        ...[
            ['Add lemon', 'cemDemoFruitLemons', '2', ['2', '12', '0', '0', '14']],
            ['Add cherry', 'cemDemoFruitCherries', '13', ['2', '13', '0', '0', '15']],
            ['Add apple', 'cemDemoFruitApples', '1', ['2', '13', '1', '0', '16']],
            ['Add banana', 'cemDemoFruitBananas', '1', ['2', '13', '1', '1', '17']],
        ].flatMap(([button, key, value, counts]) => [
            clickThenText(`button[aria-label="${button}"]`, 'dl', `🛒 ${counts.at(-1)}`),
            storageValue(key, value), nodeTexts('dd', counts),
        ]),
    ]),
    sampleContract('7. Write a slice back to storage', [
        elementIdentity('cem-storage-editor:first-of-type input', 'remember'),
        elementIdentity('cem-storage-editor:last-of-type input', 'remember'),
        ...storageEditorState('shared initial'),
        ...[['first', 'from A'], ['last', 'from B'], ['first', ''], ['last', '🍒 B']].flatMap(([position, value]) => [
            fillThenText(`cem-storage-editor:${position}-of-type input`, value, 'cem-storage-editor output', value),
            ...storageEditorState(value), focusedElement(`cem-storage-editor:${position}-of-type input`),
            propertyEquals(`cem-storage-editor:${position}-of-type input`, 'selectionStart', value.length),
        ]),
    ]),
];

function splitPartChecks(...values) {
    return [
        countExactly('ol > li', values.length),
        ...values.map((value, index) =>
            propertyEquals(`ol > li:nth-of-type(${index + 1})`, 'textContent', `“${value}”`)),
    ];
}

const counterControl = ':is(input, textarea)';
const counterSelection = value => value ? [0, Math.min(2, value.length), 'backward'] : [0, 0];
const domMergeSamples = [
    sampleContract('1. Textarea word count', [
        elementIdentity(counterControl, 'remember'),
        counterState('Hello world!', ['2']),
        fillThenText(counterControl, 'one two three', 'article strong', '2'),
        counterState('one two three', ['2'], { focused: true }),
        pressThenProperty(counterControl, 'Tab', counterControl, 'value', 'one two three'),
        counterState('one two three', ['3'], { focused: false }),
        ...[
            [' one\tone\n🍒  🍋 ', '4'], ['\t\n\u00a0\u2003', '0'],
            ['🍒e\u0301', '1'], [' \t\n', '0'], ['', '0'],
        ].flatMap(([value, words]) => [
            editControl(value, 'change'),
            counterState(value, [words], { focused: true, selection: counterSelection(value) }),
        ]),
    ]),
    sampleContract('2. Input word and character count', [
        elementIdentity(counterControl, 'remember'),
        counterState('Type to update', ['14', '3'], { output: 'Type to update' }),
        editControl('two words'),
        counterState('two words', ['9', '2'], { output: 'two words', focused: true, selection: counterSelection('two words') }),
        replaceControlText(4, 4, 'short '),
        counterState('two short words', ['15', '3'], { output: 'two short words', focused: true, selection: [10, 10] }),
        replaceControlText(4, 9, '🍒'),
        counterState('two 🍒 words', ['11', '3'], { output: 'two 🍒 words', focused: true, selection: [6, 6] }),
        ...[
            ['🍒 🍒 🍋', '3'], ['🍒e\u0301', '1'], ['one\u00a0one\u2003🍋', '3'],
            ['\u00a0\u2003', '0'], ['   ', '0'], ['', '0'],
        ].flatMap(([value, words]) => [
            editControl(value),
            counterState(value, [String([...value].length), words],
                { output: value, focused: true, selection: counterSelection(value) }),
        ]),
    ]),
    sampleContract('3. XPath word and character count', [
        elementIdentity(counterControl, 'remember'),
        counterState('🍒 🍒 🍋', ['5', '3']),
        replaceControlText(2, 2, ' red'),
        counterState('🍒 red 🍒 🍋', ['9', '4'], { focused: true, selection: [6, 6] }),
        ...[
            [' one\tone\n🍒  🍋 ', '4'], ['\t\n\u00a0\u2003', '1'],
            ['one\u00a0one\u2003🍋', '1'], ['🍒e\u0301', '1'], [' \t\n', '0'], ['', '0'],
        ].flatMap(([value, words]) => [
            editControl(value),
            counterState(value, [String([...value].length), words],
                { focused: true, selection: counterSelection(value) }),
        ]),
    ]),
];

const shortenRows = [
    ['str:shorten("short", 8)', 'short'],
    ['str:shorten("abcdefghij", 7)', 'abc…hij'],
    ['str:shorten("abcdefghij", 8)', 'abc…ghij'],
    ['str:shorten("abcdefghij", 8, "...")', 'ab...hij'],
    ['str:shorten("abcdefghij", 6, "")', 'abchij'],
    ['str:shorten("αβ😀δεζη", 5, "💠")', 'αβ💠ζη'],
    ['str:shorten( "https://example.test/lib/semantic-card.cem" , 32)', 'https://example…emantic-card.cem'],
];
const stringCases = [
    {
        legend: 'URL ID with a string chain',
        fields: [
            ['Pokémon URL', 'article label:nth-of-type(1) input', 'https://pokeapi.co/api/v2/pokemon/1/'],
        ],
        initial: '1',
        edits: [
            [0, 'https://pokeapi.co/api/v2/pokemon/10/', '10'],
            [0, '/short/', 'No ID'],
            [0, '', 'No ID'],
            [0, 'https://pokeapi.co/api/v2/pokemon/', ''],
            [0, 'https://pokeapi.co/api/v2/pokemon', 'No ID'],
            [0, 'a/b/c/d/e/f/🍒', '🍒'],
            [0, 'a/b/c/d/e/f/  ivy  /more', '  ivy  '],
            [0, 'https://pokeapi.co/api/v2/pokemon/25/', '25'],
        ],
    },
    {
        legend: 'str:split',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '🍒,🍋,,🍌'],
            ['Separator', 'article label:nth-of-type(2) input', ','],
        ],
        initial: '4',
        initialParts: ['🍒', '🍋', '', '🍌'],
        edits: [
            [0, 'a::b::::', '1', ['a::b::::']],
            [1, '::', '4', ['a', 'b', '', '']],
            [0, 'aaa', '1', ['aaa']],
            [1, 'aa', '2', ['', 'a']],
            [0, 'a.b.*c', '1', ['a.b.*c']],
            [1, '.', '3', ['a', 'b', '*c']],
            [0, '🍒🍋🍒', '1', ['🍒🍋🍒']],
            [1, '🍒', '3', ['', '🍋', '']],
            [0, 'a🍒e\u0301', '2', ['a', 'e\u0301']],
            [1, '', '6', ['', 'a', '🍒', 'e', '\u0301', '']],
            [0, '', '2', ['', '']],
            [1, ',', '1', ['']],
            [0, '🍒,🍋,,🍌,', '5', ['🍒', '🍋', '', '🍌', '']],
        ],
    },
    {
        legend: 'str:trim',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '  🍒  🍋  '],
        ],
        initial: '🍒  🍋',
        edits: [
            [0, ' \u00a0🍌  🍒 \uFEFF', '🍌  🍒'],
            [0, '\u0085🍒\u0085', '\u0085🍒\u0085'],
            [0, '\u200b🍒\u200b', '\u200b🍒\u200b'],
            [0, '\t \u00a0\u2003\uFEFF', ''],
            [0, '', ''],
            [0, '  🍒  🍋  ', '🍒  🍋'],
        ],
    },
    {
        legend: 'str:trim_start',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '  🍒  🍋  '],
        ],
        initial: '🍒  🍋  ',
        edits: [
            [0, ' \u00a0🍌  🍒 \uFEFF', '🍌  🍒 \uFEFF'],
            [0, '\u0085🍒\u0085', '\u0085🍒\u0085'],
            [0, '\u200b🍒\u200b', '\u200b🍒\u200b'],
            [0, '\t \u00a0\u2003\uFEFF', ''],
            [0, '', ''],
            [0, '  🍒  🍋  ', '🍒  🍋  '],
        ],
    },
    {
        legend: 'str:trim_end',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '  🍒  🍋  '],
        ],
        initial: '  🍒  🍋',
        edits: [
            [0, ' \u00a0🍌  🍒 \uFEFF', ' \u00a0🍌  🍒'],
            [0, '\u0085🍒\u0085', '\u0085🍒\u0085'],
            [0, '\u200b🍒\u200b', '\u200b🍒\u200b'],
            [0, '\t \u00a0\u2003\uFEFF', ''],
            [0, '', ''],
            [0, '  🍒  🍋  ', '  🍒  🍋'],
        ],
    },
    {
        legend: 'str:char_at',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '🍒🍋🍌'],
            ['Index', 'article label:nth-of-type(2) input', '1'],
        ],
        initial: '🍋',
        edits: [
            [1, '99', ''],
            [1, '-1', ''],
            [1, '-3', ''],
            [1, '-4', ''],
            [1, '0', '🍒'],
            [1, '2', '🍌'],
            [1, '3', ''],
            [1, '', '🍒'],
            [1, '1.5', '🍒'],
            [1, '1e2', '🍒'],
            [0, 'a🍒e\u0301', 'a'],
            [1, '1', '🍒'],
            [1, '2', 'e'],
            [1, '3', '\u0301'],
            [1, '-1', ''],
            [0, '', ''],
            [1, '0', ''],
            [0, 'a🍒b', 'a'],
            [1, '1', '🍒'],
        ],
    },
    {
        legend: 'str:at',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '🍒🍋🍌'],
            ['Index', 'article label:nth-of-type(2) input', '-1'],
        ],
        initial: '🍌',
        edits: [
            [1, '99', '∅'],
            [1, '-1', '🍌'],
            [1, '-3', '🍒'],
            [1, '-4', '∅'],
            [1, '0', '🍒'],
            [1, '2', '🍌'],
            [1, '3', '∅'],
            [1, '', '🍒'],
            [1, '1.5', '🍒'],
            [1, '1e2', '🍒'],
            [0, 'a🍒e\u0301', 'a'],
            [1, '1', '🍒'],
            [1, '2', 'e'],
            [1, '3', '\u0301'],
            [1, '-1', '\u0301'],
            [0, '', '∅'],
            [1, '0', '∅'],
            [0, 'a🍒b', 'a'],
            [1, '1', '🍒'],
        ],
    },
    {
        legend: 'str:index_of',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '🍒🍋🍒'],
            ['Find', 'article label:nth-of-type(2) input', '🍒'],
            ['Position', 'article label:nth-of-type(3) input', '0'],
        ],
        initial: '0',
        edits: [
            [2, '1', '2'],
            [1, '🍋', '1'],
            [1, '🥦', '-1'],
            [1, '', '1'],
            [2, '99', '3'],
            [0, 'aaaa', '4'],
            [1, 'aa', '-1'],
            [2, '1', '1'],
            [2, '-9', '0'],
            [2, '', '0'],
            [2, '1.5', '0'],
            [2, '1e2', '0'],
            [0, 'a🍒e\u0301🍒', '-1'],
            [1, '🍒', '1'],
            [2, '2', '4'],
            [1, 'e\u0301', '2'],
            [1, 'é', '-1'],
            [0, '', '-1'],
            [1, '', '0'],
            [2, '99', '0'],
            [0, 'a🍒a', '3'],
            [1, 'a', '-1'],
            [2, '0', '0'],
        ],
    },
    {
        legend: 'str:last_index_of',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '🍒🍋🍒'],
            ['Find', 'article label:nth-of-type(2) input', '🍒'],
            ['Position', 'article label:nth-of-type(3) input', '3'],
        ],
        initial: '2',
        edits: [
            [2, '1', '0'],
            [1, '🍋', '1'],
            [1, '🥦', '-1'],
            [1, '', '1'],
            [2, '99', '3'],
            [0, 'aaaa', '4'],
            [1, 'aa', '2'],
            [2, '1', '1'],
            [2, '-9', '0'],
            [2, '', '2'],
            [2, '1.5', '2'],
            [2, '1e2', '2'],
            [0, 'a🍒e\u0301🍒', '-1'],
            [1, '🍒', '4'],
            [2, '2', '1'],
            [1, 'e\u0301', '2'],
            [1, 'é', '-1'],
            [0, '', '-1'],
            [1, '', '0'],
            [2, '99', '0'],
            [0, 'a🍒a', '3'],
            [1, 'a', '2'],
            [2, '0', '0'],
        ],
    },
    {
        legend: 'XPath normalize-space',
        fields: [
            ['Text', 'article label:nth-of-type(1) textarea', '  🍒  🍋  '],
        ],
        initial: '🍒 🍋',
        edits: [
            [0, ' a\ta\n🍒  ', 'a a 🍒'],
            [0, ' a\u00a0\u2003b ', 'a\u00a0\u2003b'],
            [0, ' \t\n', ''],
            [0, '', ''],
            [0, ' \uFEFFa\u0085b\u200bc ', '\uFEFFa\u0085b\u200bc'],
            [0, '  a a  🍒 \n', 'a a 🍒'],
        ],
    },
    {
        legend: 'XPath tokenize and string-join',
        fields: [
            ['Text', 'article label:nth-of-type(1) textarea', '🍒 🍒 🍋'],
            ['Separator', 'article label:nth-of-type(2) input', '/'],
        ],
        initial: '🍒/🍒/🍋',
        edits: [
            [0, 'a\ta\nb', 'a/a/b'],
            [1, '🍒', 'a🍒a🍒b'],
            [1, '', 'aab'],
            [1, ' <&> ', 'a <&> a <&> b'],
            [0, 'a\u00a0b', 'a\u00a0b'],
            [0, ' a\u00a0\u2003b a ', 'a\u00a0\u2003b <&> a'],
            [0, ' \t\n', ''],
            [0, '', ''],
            [1, '|', ''],
            [0, ' 🍒 🍒 🍋 ', '🍒|🍒|🍋'],
        ],
    },
];

const shortenMatrixSample = sampleContract('str:shorten query/result matrix', [
    countExactly('table', 1), nodeTexts('thead th', ['Query', 'Result']),
    countExactly('tbody tr', 7), countExactly('tbody td', 14), countExactly('tbody code', 14),
    ...shortenRows.flatMap((row, index) => row.map((value, column) =>
        normalizedText(`tbody tr:nth-child(${index + 1}) td:nth-child(${column + 1})`, value))),
]);
const stringMethodSamples = stringCases.map(entry => {
    const values = entry.fields.map(([, , value]) => value);
    return sampleContract(entry.legend, [
        countExactly('article', 1), countExactly('article output', 1),
        propertyEquals('article output', 'textContent', entry.initial),
        ...entry.fields.flatMap(([, selector, value]) => [
            elementIdentity(selector, 'remember'), propertyEquals(selector, 'value', value),
        ]),
        ...(entry.initialParts ? splitPartChecks(...entry.initialParts) : []),
        ...entry.edits.flatMap(([index, value, output, parts]) => {
            const [label, selector] = entry.fields[index];
            values[index] = value;
            return [
                fillThenText(selector, value, 'article output', output.replace(/\s+/gu, ' ').trim()),
                propertyEquals(selector, 'defaultValue', value),
                propertyEquals('article output', 'textContent', output),
                focusedElement(selector),
                ...entry.fields.flatMap(([, field], fieldIndex) => [
                    elementIdentity(field, 'same'), propertyEquals(field, 'value', values[fieldIndex]),
                ]),
                ...(['Index', 'Position'].includes(label) ? [] : [
                    propertyEquals(selector, 'selectionStart', value.length), propertyEquals(selector, 'selectionEnd', value.length),
                ]),
                ...(parts ? splitPartChecks(...parts) : []),
            ];
        }),
    ]);
});
const stringSamples = [shortenMatrixSample, ...stringMethodSamples];

const dataTablePage = await readFile(join(repoRoot, 'packages/cem-elements/demo/data-table.html'), 'utf8');
const tableSources = Object.fromEntries(['xml', 'csv', 'yaml', 'json'].map(format => [format,
    dataTablePage.split(`<cem-data-table format="${format}">`)[1].split('</cem-data-table>')[0]
        .replaceAll('&lt;', '<').replaceAll('&gt;', '>').replaceAll('&amp;', '&').trim(),
]));
const xmlTableCells = [['10', 'caterpie', '∅', '∅'], ['2', 'ivysaur', '🌱', '∅'], ['3', 'venusaur', '""', null]];
const csvTableCells = [['10', '🍒', 'sweet, red'], ['2', '🍋', 'say "zest"'], ['3', '🍌', '""']];
const yamlTableCells = [['10', '🍒', null, '∅'], ['2', '🍋', '∅', 'true'], ['3', '🍌', '∅', 'false']];
const jsonTableCells = [['10', '🍒', '""'], ['2', '🍋', '∅'], ['3', '🍌', 'null']];
const dataTableSamples = [
    sampleContract('1. XML table: attributes and text', [
        countExactly('table', 2),
        tableState('table[aria-label$="/name"]', [['🌿'], ['🌸']]),
        ...auditedTableChecks('xml', '@id', 'table[aria-label$="/row"]', ['@id', '#text', '@mood', 'evolutions'], xmlTableCells),
    ]),
    sampleContract('2. CSV table: quoted fields', [
        countExactly('table', 1),
        ...auditedTableChecks('csv', 'qty', 'table', ['qty', 'fruit', 'note'], csvTableCells),
    ]),
    sampleContract('3. YAML table: nested collections', [
        countExactly('table', 2),
        tableState('table[aria-label="tags"]', [['red'], ['sweet']]),
        ...auditedTableChecks('yaml', 'qty', 'table[aria-label="document"]', ['qty', 'fruit', 'tags', 'fresh'], yamlTableCells),
    ]),
    sampleContract('4. JSON table: empty and missing', [
        countExactly('table', 1),
        ...auditedTableChecks('json', 'qty', 'table', ['qty', 'fruit', 'note'], jsonTableCells),
        ...tableParseRecovery(),
    ]),
    sampleContract('5. Presentation aspects: tree and IP-filter form', aspectTableChecks()),
    sampleContract('6. XSLT table: native sorting and selection', [
        countExactly('table', 1),
        ...auditedTableChecks('json', 'qty', 'table', ['qty', 'fruit', 'note'], jsonTableCells),
        ...tableParseRecovery(),
    ]),
    sampleContract('7. XSLT aspects: tree and IP-filter form', aspectTableChecks()),
    sampleContract('./data-table-view.cemt', [
        attributeEquals(':scope', 'src', './data-table-view.cemt'),
        attributeEquals(':scope', 'type', 'text/cem-ml'),
        propertyEquals('[slot=text] code', 'textContent',
            await readFile(join(repoRoot, 'packages/cem-elements/demo/data-table-view.cemt'), 'utf8')),
        propertyEquals('[slot=demo]', 'textContent', ''),
        countExactly('[slot=demo] > *', 0),
        attributeEquals(':scope', 'data-state', 'ready'),
    ]),
    sampleContract('./data-table-view.xslt', [
        attributeEquals(':scope', 'src', './data-table-view.xslt'),
        attributeEquals(':scope', 'type', 'application/xslt+xml'),
        propertyEquals('[slot=text] code', 'textContent',
            await readFile(join(repoRoot, 'packages/cem-elements/demo/data-table-view.xslt'), 'utf8')),
        propertyEquals('[slot=demo]', 'textContent', ''),
        countExactly('[slot=demo] > *', 0),
        attributeEquals(':scope', 'data-state', 'ready'),
    ]),
    sampleContract('./data-table-aspects.xslt', [
        attributeEquals(':scope', 'src', './data-table-aspects.xslt'),
        attributeEquals(':scope', 'type', 'application/xslt+xml'),
        propertyEquals('[slot=text] code', 'textContent',
            await readFile(join(repoRoot, 'packages/cem-elements/demo/data-table-aspects.xslt'), 'utf8')),
        propertyEquals('[slot=demo]', 'textContent', ''),
        countExactly('[slot=demo] > *', 0),
        attributeEquals(':scope', 'data-state', 'ready'),
    ]),
];

function auditedTableChecks(format, column, table, headings, cells) {
    return [
        tableState(table, cells),
        ...['✓', ...headings].map((heading, index) => normalizedText(`${table} > thead th:nth-child(${index + 1})`, heading)),
        clickThenText(`${table} > tbody > tr:nth-child(2) > th > button`, `${table} > tbody`, '✓'),
        tableState(table, cells, [1]),
        selectThenText('select[aria-label="Sort column"]', column, table, '10'),
        tableState(table, cells, [1]),
        selectThenText('select[aria-label="Compare"]', 'number', table, '2'),
        tableState(table, [cells[1], cells[2], cells[0]], [0]),
        selectThenText('select[aria-label="Direction"]', 'descending', table, '10'),
        tableState(table, [cells[0], cells[2], cells[1]], [2]),
        fillBlurThenText('textarea', tableSources[format].replace('10', '11'), table, '11'),
        tableState(table, [['11', ...cells[0].slice(1)], cells[2], cells[1]]),
        clickThenText('button[aria-label="Reset source"]', table, '10'),
        tableState(table, [cells[0], cells[2], cells[1]], [2]),
        propertyEquals('textarea', 'value', tableSources[format]),
        selectThenText('select[aria-label="Sort column"]', '', table, '10'),
        tableState(table, cells, [1]),
        clickThenText('article > details > summary', 'article', 'Source'),
        propertyEquals('article > details', 'open', false),
        pressThenProperty('article > details > summary', 'Enter', 'article > details', 'open', true),
        pressThenProperty('article > details > summary', 'Space', 'article > details', 'open', false),
        pressThenProperty('article > details > summary', 'Enter', 'article > details', 'open', true),
    ];
}

function tableParseRecovery() {
    return [
        fillBlurThenText('textarea', '[oops]', '[role="alert"]', '⚠'),
        countExactly('table', 0),
        clickThenText('button[aria-label="Reset source"]', 'tbody', '🍒'),
        tableState('table', jsonTableCells, [1]),
        countExactly('[role="alert"]', 0),
        propertyEquals('textarea', 'value', tableSources.json),
    ];
}

function aspectTableChecks() {
    const visits = [['192.0.2.1', '10'], ['192.0.2.2', '2']];
    const original = dataTablePage.split('<cem-aspect-view format="json">')[1].split('</cem-aspect-view>')[0].trim();
    return [
        tableState('table[aria-label="visits"]', visits),
        countExactly('table[aria-label="notes"]', 0),
        text('article > details > ul > li:nth-child(2) > details', '🍒 ready'),
        text('article > details > ul > li:nth-child(2) > details', '🍋 review'),
        propertyEquals('input[aria-label="Presentation aspects"]', 'checked', true),
        propertyEquals('input[aria-label="Address / CIDR"]', 'value', '192.0.2.0/24'),
        propertyEquals('form output', 'textContent', 'allow: 192.0.2.0/24'),
        fillThenText('input[aria-label="Address / CIDR"]', '198.51.100.0/24', 'form output', '198.51.100.0/24'),
        selectThenText('select[aria-label="Action"]', 'deny', 'form output', 'deny'),
        propertyEquals('form output', 'textContent', 'deny: 198.51.100.0/24'),
        clickThenText('input[aria-label="Presentation aspects"]', 'article', 'ip-filter'),
        tableState('table[aria-label="notes"]', [['🍒 ready'], ['🍋 review']]),
        countExactly('form[aria-label="IP filter"]', 0),
        tableState('table[aria-label="visits"]', visits),
        propertyEquals('input[aria-label="Presentation aspects"]', 'checked', false),
        clickThenText('input[aria-label="Presentation aspects"]', 'form output', 'deny'),
        propertyEquals('form output', 'textContent', 'deny: 198.51.100.0/24'),
        propertyEquals('input[aria-label="Address / CIDR"]', 'value', '198.51.100.0/24'),
        fillThenText('input[aria-label="Address / CIDR"]', '', 'form output', 'deny'),
        propertyEquals('form output', 'textContent', 'deny: '),
        propertyEquals('input[aria-label="Address / CIDR"]', 'value', ''),
        propertyEquals('textarea', 'value', original),
        countExactly('table[aria-label="notes"]', 0),
        tableState('table[aria-label="visits"]', visits),
    ];
}

const formSamples = [
    sampleContract('1. Simple validation', [
        propertyEquals('input[name="username"]', 'value', ''),
        ...simpleFormChecks('', false),
        nodeTexts('button', ['→']),
        submitForm('button', 'blocked'),
        ...['form', 'input[name="username"]'].map(selector => elementIdentity(selector, 'remember')),
        fillThenText('input[name="username"]', 'abcdefghij', 'form > p:first-of-type output', 'abcdefghij'),
        submitForm('button', 'cancelled'),
        countExactly('input[name="password"]', 0),
        fillThenText('input[name="username"]', 'abcdefghijk', 'form > p:first-of-type output', 'abcdefghijk'),
        ...simpleFormChecks('abcdefghijk', false),
        countExactly('input[name="password"]', 0),
        submitForm('button', 'cancelled'),
        countExactly('input[name="password"]', 1),
        attributeEquals('button', 'aria-label', 'Sign in'),
        elementIdentity('input[name="password"]', 'remember'),
        ...[['abc', false], ['abcd', true], ['', false], ['secret', true]].flatMap(([value, valid]) => [
            fillThenText('input[name="password"]', value, 'form > p:nth-of-type(2) output', String(valid)),
            ...simpleFormChecks('abcdefghijk', valid),
            ...['form', 'input[name="username"]', 'input[name="password"]'].map(selector => elementIdentity(selector, 'same')),
        ]),
        formState({ formData: [['username', 'abcdefghijk'], ['password', 'secret']] }),
        submitForm('button', 'allowed'),
    ]),
    sampleContract('2. Form lifecycle', [
        nodeTexts('fieldset > p', ['Select a confirmation method.']),
        countExactly('input[name="password"]', 0),
        ...['email', 'sms', 'password'].map(method => propertyEquals(`input[value="${method}"]`, 'checked', false)),
        ...['form', 'input[name="username"]', ...['email', 'sms', 'password'].map(method => `input[value="${method}"]`)]
            .map(selector => elementIdentity(selector, 'remember')),
        fillThenText('input[name="username"]', 'short', 'form > p:nth-of-type(2) output', 'short'),
        formState({ outputs: ['Use at least 10 characters', 'short', '', 'false', 'Complete the username and confirmation method'] }),
        fillThenText('input[name="username"]', 'abcdefghij', 'form > p:nth-of-type(2) output', 'abcdefghij'),
        ...lifecycleChecks('', false),
        clickThenText('input[value="email"]', 'form > p:nth-of-type(3) output', 'email'),
        ...lifecycleChecks('email', true),
        pressThenProperty('input[value="sms"]', 'Space', 'input[value="sms"]', 'checked', true),
        ...lifecycleChecks('sms', true),
        clickThenText('input[value="password"]', 'form > p:nth-of-type(3) output', 'password'),
        countExactly('input[name="password"]', 1),
        propertyEquals('input[name="password"]', 'value', ''),
        propertyEquals('input[name="password"]', 'required', true),
        ...lifecycleChecks('password', false),
        ...[['abc', false], ['abcd', true], ['', false], ['secret', true]].flatMap(([value, valid]) => [
            fillThenText('input[name="password"]', value, 'form > p:nth-of-type(4) output', String(valid)),
            propertyEquals('input[name="password"]', 'value', value),
            ...lifecycleChecks('password', valid),
        ]),
        pressThenProperty('input[value="email"]', 'Space', 'input[value="email"]', 'checked', true),
        ...lifecycleChecks('email', true),
        clickThenText('input[value="password"]', 'form > p:nth-of-type(3) output', 'password'),
        ...lifecycleChecks('password', true),
        propertyEquals('input[name="password"]', 'value', 'secret'),
        ...['form', 'input[name="username"]', ...['email', 'sms', 'password'].map(method => `input[value="${method}"]`)]
            .map(selector => elementIdentity(selector, 'same')),
    ]),
    sampleContract('3. Native control validity message', [
        propertyEquals('input[name="email"]', 'value', 'person@example.test'),
        nodeTexts('output', ['true', '']),
        elementIdentity('input[name="email"]', 'remember'),
        ...['', 'not-an-email', 'reader@example.test', '', 'person@example.test'].flatMap(value => [
            fillThenText('input[name="email"]', value, 'form > p:first-of-type output', String(value.includes('@'))),
            nativeValidity('input[name="email"]', { valid: value.includes('@'), valueMissing: value === '', typeMismatch: value === 'not-an-email' },
                'form > p:nth-of-type(2) output'),
            propertyEquals('input[name="email"]', 'value', value),
            elementIdentity('input[name="email"]', 'same'),
        ]),
    ]),
    sampleContract('4. Form custom validity message', [
        propertyEquals('input[name="email"]', 'value', ''),
        formState({ outputs: ['', '0', 'false', 'Use more than 3 characters'] }),
        elementIdentity('input[name="email"]', 'remember'),
        ...['abc', 'abcd', '🍒ab', '🍒abc', '', 'reader'].flatMap(value => {
            const length = [...value].length;
            return [
                fillThenText('input[name="email"]', value, 'form > p:nth-of-type(2) output', String(length)),
                formState({ outputs: [value, String(length), String(length > 3), length > 3 ? '' : 'Use more than 3 characters'] }),
                propertyEquals('input[name="email"]', 'value', value),
                elementIdentity('input[name="email"]', 'same'),
            ];
        }),
    ]),
    sampleContract('5. DCE as a form input', [
        countExactly('cem-form-fruit-choice', 2),
        ...['first', 'last'].flatMap(position => [
            elementIdentity(`cem-form-fruit-choice:${position}-of-type`, 'remember'),
            nodeTexts(`cem-form-fruit-choice:${position}-of-type button`, ['Choose fruit', '🍏', '🍌']),
            ...['Choose fruit', 'Apple', 'Banana'].flatMap((label, index) => [
                attributeEquals(`cem-form-fruit-choice:${position}-of-type button[data-option-index="${index}"]`, 'aria-label', label),
                attributeEquals(`cem-form-fruit-choice:${position}-of-type button[data-option-index="${index}"]`, 'title', label),
            ]),
        ]),
        ...fruitFormChecks('', ''),
        submitForm('button[type="submit"]', 'blocked'),
        focusedElement('cem-form-fruit-choice:first-of-type button[data-option-index="0"]'),
        ...fruitFormChecks('', ''),
        ...[
            ['first', 1, '🍏', ''], ['last', 2, '🍏', '🍌'], ['last', 1, '🍏', '🍏'],
            ['first', 0, '', '🍏'], ['first', 2, '🍌', '🍏'], ['last', 2, '🍌', '🍌'],
        ].flatMap(([position, index, first, second]) => [
            clickThenText(`cem-form-fruit-choice:${position}-of-type button[data-option-index="${index}"]`,
                'form > p:first-of-type output', position === 'first' ? first : second),
            ...fruitFormChecks(first, second),
            ...(!first || !second ? [
                submitForm('button[type="submit"]', 'blocked'),
                focusedElement(`cem-form-fruit-choice:${!first ? 'first' : 'last'}-of-type button[data-option-index="0"]`),
                ...fruitFormChecks(first, second),
            ] : []),
        ]),
        ...['Space', 'ArrowUp', 'Space'].map(key => pressThenProperty(
            'cem-form-fruit-choice:last-of-type button[data-option-index="2"]', key,
            'cem-form-fruit-choice:last-of-type', 'isConnected', true)),
        ...fruitFormChecks('🍌', '🍏'),
        submitForm('button[type="submit"]', 'cancelled'),
        clickThenText('cem-form-fruit-choice:last-of-type button[data-option-index="2"]', 'form > p:nth-of-type(2) output', 'true'),
        ...fruitFormChecks('🍌', '🍌'),
        submitForm('button[type="submit"]', 'allowed'),
    ]),
];

function simpleFormChecks(username, valid) {
    return [nodeTexts('form > p output', [username, String(valid), valid ? '' : 'Enter a long username and password'])];
}

function fruitFormChecks(first, second) {
    const valid = first !== '' && first === second;
    return [
        nodeTexts('form > p output', [first, second, String(valid), valid ? '' : 'Choose the same fruit']),
        formState({ formData: [['firstFruit', first], ['secondFruit', second]] }),
        ...[[first, 'first'], [second, 'last']].flatMap(([value, position]) => [
            elementIdentity(`cem-form-fruit-choice:${position}-of-type`, 'same'),
            nodeTexts(`cem-form-fruit-choice:${position}-of-type output`, [value]),
            ...['', '🍏', '🍌'].map((option, index) => attributeEquals(
                `cem-form-fruit-choice:${position}-of-type button[data-option-index="${index}"]`, 'aria-pressed', String(option === value))),
        ]),
    ];
}

function lifecycleChecks(method, valid) {
    return [
        formState({ outputs: ['', 'abcdefghij', method, String(valid), valid ? '' : 'Complete the username and confirmation method'] }),
        countExactly('input[name="password"]', method === 'password' ? 1 : 0),
        nodeTexts('fieldset > p', method === 'sms' ? ['Message and data rates may apply.'] : method ? [] : ['Select a confirmation method.']),
        ...['email', 'sms', 'password'].map(value => propertyEquals(`input[value="${value}"]`, 'checked', value === method)),
    ];
}

const forEachSamples = [
    sampleContract('1. Simple for-each', [
        nodeTexts('ul > li', ['🍏', '🍌', '🍒']),
        countExactly('ul', 1),
    ]),
    sampleContract('2. for-each with position()', [
        nodeTexts('article > div > div', ['1. Red', '2. Green', '3. Blue']),
        ...['rgb(193, 18, 31)', 'rgb(21, 128, 61)', 'rgb(29, 78, 216)'].flatMap((color, index) => [
            computedStyle(`article > div > div:nth-child(${index + 1})`, 'backgroundColor', color),
            computedStyle(`article > div > div:nth-child(${index + 1})`, 'color', 'rgb(255, 255, 255)'),
        ]),
    ]),
    sampleContract('3. Conditional for-each', [
        propertyEquals('input[type=checkbox]', 'checked', false),
        countExactly('input', 1),
        elementIdentity('input[type=checkbox]', 'remember'),
        ...conditionalLoopChecks(false),
        ...[true, false, true, false].flatMap(shown => [
            shown ? clickThenText('input[type=checkbox]', 'article', 'BEFORE')
                : pressThenProperty('input[type=checkbox]', 'Space', 'input[type=checkbox]', 'checked', false),
            propertyEquals('input[type=checkbox]', 'checked', shown),
            elementIdentity('input[type=checkbox]', 'same'),
            ...conditionalLoopChecks(shown),
        ]),
    ]),
    sampleContract('4. Nested for-each table', [
        countExactly('table', 1),
        nodeTexts('thead th', ['Col 1', 'Col 2', 'Col 3']),
        ...loopTableChecks([['A1', 'A2', 'A3'], ['B1', 'B2', 'B3'], ['C1', 'C2', 'C3']]),
        countExactly('article > :is(tr, td)', 0),
    ]),
    sampleContract('5. for-each with attributes', [
        nodeTexts('article > div', ['#1 Alice (admin)', '#2 Bob (editor)', '#3 Charlie (viewer)']),
        nodeTexts('article > div > strong', ['#1', '#2', '#3']),
        nodeTexts('article > div > em', ['(admin)', '(editor)', '(viewer)']),
    ]),
    sampleContract('6. Dynamic table with toggle', [
        propertyEquals('input[type=checkbox]', 'checked', false),
        countExactly('table', 1),
        nodeTexts('thead th', ['#', 'Product', 'Price']),
        ...loopTableChecks([]),
        ...['input[type=checkbox]', 'table', 'thead'].map(selector => elementIdentity(selector, 'remember')),
        ...[true, false, true, false].flatMap(shown => [
            shown ? clickThenText('input[type=checkbox]', 'tbody', 'Widget')
                : pressThenProperty('input[type=checkbox]', 'Space', 'input[type=checkbox]', 'checked', false),
            propertyEquals('input[type=checkbox]', 'checked', shown),
            ...loopTableChecks(shown ? [['1', 'Widget', '$10'], ['2', 'Gadget', '$25'], ['3', 'Gizmo', '$15']] : []),
            nodeTexts('thead th', ['#', 'Product', 'Price']),
            countExactly('table', 1),
            ...['input[type=checkbox]', 'table', 'thead'].map(selector => elementIdentity(selector, 'same')),
        ]),
    ]),
    sampleContract('7. for-each over payload data', [
        countExactly('cem-loop-payload .payload-feed', 1),
        nodeTexts('.payload-feed li', ['1. payload-alpha: Payload Alpha', '2. payload-beta: Payload Beta']),
    ]),
    sampleContract('8. for-each over location data', [
        nodeTexts('.location-feed li', ['topic = feeds', 'item = payload,resource']),
    ]),
    sampleContract('9. for-each over HTTP JSON/XML data', [
        nodeTexts('output', ['loaded', 'loaded']),
        nodeTexts('.http-json-feed li', ['alpha: ready', 'beta: loaded']),
        nodeTexts('.http-xml-feed li', ['gamma: xml-ready', 'delta: xml-loaded']),
    ]),
    sampleContract('http-data.json', await externalPreviewChecks('http-data.json', 'json')),
    sampleContract('http-data.xml', await externalPreviewChecks('http-data.xml', 'xml')),
];

function conditionalLoopChecks(shown) {
    return [
        nodeTexts('article > div span', shown ? ['1:First', '2:Second', '3:Third'] : []),
        nodeTexts('article > div', [shown ? 'BEFORE 1:First 2:Second 3:Third AFTER' : 'BEFORE AFTER']),
    ];
}

function loopTableChecks(rows) {
    return [
        countExactly('table > tbody > tr', rows.length),
        ...rows.map((cells, index) => nodeTexts(`table > tbody > tr:nth-child(${index + 1}) > td`, cells)),
    ];
}

const externalTemplateSamples = [
    sampleContract('1. reference the template in page DOM', [
        countExactly('dce-internal', 2),
        normalizedText('dce-internal:first-of-type', '👋 World!'),
        normalizedText('dce-internal:last-of-type', 'Hello World!'),
    ]),
    sampleContract('2. without TAG, inline instantiation', [
        countExactly('cem-element[src="#template2"][data-cem-anonymous-declaration]', 2),
        countExactly('[data-cem-anonymous-instance]', 2),
        normalizedText('cem-element:first-of-type > [data-cem-anonymous-instance]', '🏗️ construction'),
        normalizedText('cem-element:last-of-type > [data-cem-anonymous-instance]', '🏗️ construction'),
    ]),
    sampleContract('3. external SVG file', externalSvgChecks('dce-external')),
    sampleContract('3a. Anonymous external SVG', externalSvgChecks(
        'cem-element[src="confused.svg"] > [data-cem-anonymous-instance]')),
    sampleContract('3b. Missing source fallback', [
        normalizedText('dce-external-missing', 'fallback for missing image'),
        countExactly('dce-external-missing > i', 1),
        countExactly('svg', 0),
    ]),
    sampleContract('4. external CEM-ML template file', externalIslandChecks('dce-external-4', [
        'Payload comment: explicit inert envelope follows', 'DCE with complete external CEMT island',
        'wrapped-payload', 'slot="heading"', 'slot=""', 'data-fruit="🍌"', 'aria-label="Fruit choice"',
        'Every element, attribute, dataset entry, and text node is data.',
    ])),
    sampleContract('4a. Live HTML payload capture', externalIslandChecks('dce-external-4-inline', [
        'A second external-CEMT data island', 'DCE with live payload capture',
        'name="data-smile"', 'name="data-basket"', 'data-kind="live-payload"', '👼', '🍒',
    ])),
    sampleContract('4b. CEM-ML source payload', externalIslandChecks('dce-external-4-cem-ml', [
        'content-type="text/cem-ml"', 'schema="https://cem.dev/ns/cem-ml/1"',
        '{payload:item @name=fruit @slot=default | Banana from CEM-ML payload source}',
    ])),
    sampleContract('5. external HTML template', externalHtmlChecks('dce-external-5')),
    sampleContract('5a. Anonymous external HTML', externalHtmlChecks(
        '#dce-external-5-inline > [data-cem-anonymous-instance]')),
    sampleContract('6. HTML, SVG by ID within external file', [
        normalizedText('dce-html-wave', '👋'),
        countExactly('dce-html-wave b', 1),
        countExactly('dce-html-wave :is(svg, math, #ok, i, script)', 0),
    ]),
    sampleContract('6a. SVG fragment by ID', [
        countExactly('dce-html-logo svg', 1),
        propertyEquals('dce-html-logo svg', 'namespaceURI', 'http://www.w3.org/2000/svg'),
        countExactly('dce-html-logo :is(math, #wave, #ok, i, script)', 0),
    ]),
    sampleContract('6b. MathML fragment by ID', [
        countExactly('dce-html-formula math', 1),
        propertyEquals('dce-html-formula math', 'namespaceURI', 'http://www.w3.org/1998/Math/MathML'),
        countExactly('dce-html-formula :is(svg, #wave, #ok, i, script)', 0),
    ]),
    sampleContract('7a. external CEM-ML data-island tree template', externalPayloadTreeChecks(
        'CEM-ML data island tree', 'cem-elements', 'alpha', 'a1', 'Leaf text from cem-elements data island')),
    sampleContract('7b. External XSLT XML payload tree', externalPayloadTreeChecks(
        'XSLT XML payload tree', 'cem-elements-xslt', 'beta', 'b1', 'Leaf text from cem-elements XSLT data island')),
    sampleContract('7c. Missing fragment fallback', [
        normalizedText('dce-missing-none', 'element with id=none is missing in template'),
        countExactly('dce-missing-none > i', 1),
        countExactly(':is(svg, math, #wave, #ok)', 0),
    ]),
    sampleContract('7d. Anonymous external XSLT', [
        attributeEquals('cem-element', 'data-cem-anonymous-declaration', ''),
        ...externalPayloadTreeChecks('XSLT XML payload tree', 'anonymous-xslt', 'fruit', 'cherry', '🍒 from anonymous XSLT'),
    ]),
    sampleContract('7e. Embedded XSLT fragment', [
        normalizedText('article h2', 'Embedded XSLT fruit tree'),
        normalizedText('summary', 'basket'),
        countExactly('details', 1),
        countExactly('li', 2),
        normalizedText('li:first-of-type', '🍒'),
        normalizedText('li:last-of-type', '🍋'),
        countExactly(':is(svg, math, #wave, #ok, script)', 0),
        ...externalDisclosureChecks('basket'),
    ]),
    sampleContract('8. external file with embedding of another external DCE', [
        normalizedText('dce-embed-1 h4', 'embed-1.html'),
        normalizedText('dce-embed-1 [data-cem-anonymous-instance]', '🖖'),
    ]),
    sampleContract('9. external file with invoking of relative template as hash by enclosed custom-element', [
        text('dce-embed-relative-hash', '👌 from embed-relative-hash invoking'),
        normalizedText('dce-embed-lib-component', '👋 from embed-lib-component'),
        urlEquals('a', 'href', '/packages/cem-elements/demo/lib-dir/embed-lib.html#embed-lib-component'),
        urlEquals('img', 'src', '/packages/cem-elements/demo/lib-dir/Smiley.svg'),
        imageLoaded('img'),
    ]),
    sampleContract('10. external file with invoking of template in another relative path file by enclosed custom-element', [
        text('dce-embed-relative-file', '👍 from embed-relative-file invoking'),
        urlEquals('a', 'href', '/packages/cem-elements/demo/embed-1.html'),
        normalizedText('dce-embed-lib-file h4', 'embed-1.html'),
        normalizedText('dce-embed-lib-file [data-cem-anonymous-instance]', '🖖'),
    ]),
    sampleContract('embed-1.html external file', await externalPreviewChecks('embed-1.html')),
    sampleContract('embed-lib.html with multiple templates', await externalPreviewChecks('embed-lib.html')),
];

function externalSvgChecks(selector) {
    return [
        countExactly(`${selector} svg`, 1),
        propertyEquals(`${selector} svg`, 'namespaceURI', 'http://www.w3.org/2000/svg'),
        svgUseReferences(`${selector} svg`, ['#h', '#j']),
        countExactly(`${selector} i`, 0),
    ];
}

function externalIslandChecks(selector, evidence) {
    return [
        normalizedText(`${selector} h2`, 'External CEMT data-island transformation'),
        ...[
            'template[data-cem-island="instance"]', 'cem-island:context-root', 'cem-hydration:data',
            'cem-attributes:attributes', 'cem-dataset:dataset', 'cem-payload:payload', 'cem-slices:slices',
            'cem-resources:resources', 'cem-form:form-state', 'cem-validation:validation-state', 'cem-events:event-state',
            ...evidence,
        ].map(value => text(selector, value)),
        countAtLeast(`${selector} details`, 21),
        ...externalDisclosureChecks('template[data-cem-island="instance"]'),
    ];
}

function externalHtmlChecks(selector) {
    return [
        normalizedText(`${selector} #wave`, '👋'),
        normalizedText(`${selector} #ok`, '👌'),
        countExactly(`${selector} svg`, 1),
        propertyEquals(`${selector} svg`, 'namespaceURI', 'http://www.w3.org/2000/svg'),
        countExactly(`${selector} math`, 1),
        propertyEquals(`${selector} math`, 'namespaceURI', 'http://www.w3.org/1998/Math/MathML'),
        countExactly(`${selector} :is(script, i)`, 0),
    ];
}

async function externalPreviewChecks(file, type = 'html') {
    return [
        attributeEquals(':scope', 'src', `./${file}`),
        attributeEquals(':scope', 'type', type),
        attributeEquals(':scope', 'demo', 'false'),
        attributeEquals(':scope', 'data-state', 'ready'),
        propertyEquals('[slot=text] code', 'textContent',
            await readFile(join(repoRoot, 'packages/cem-elements/demo', file), 'utf8')),
        countAtLeast(`[slot=text] code ${type === 'json' ? 'i' : 'b'}`, 1),
        propertyEquals('[slot=demo]', 'textContent', ''),
        countExactly('[slot=demo] > *', 0),
    ];
}

function externalDisclosureChecks(label) {
    const disclosure = 'article > details';
    return [
        propertyEquals(disclosure, 'open', true),
        elementIdentity(disclosure, 'remember'),
        clickThenText(`${disclosure} > summary`, `${disclosure} > summary`, label),
        elementIdentity(disclosure, 'same'),
        propertyEquals(disclosure, 'open', false),
        clickThenText(`${disclosure} > summary`, `${disclosure} > summary`, label),
        elementIdentity(disclosure, 'same'),
        propertyEquals(disclosure, 'open', true),
    ];
}

function externalPayloadTreeChecks(heading, root, name, code, payload) {
    return [
        normalizedText('article h2', heading),
        countExactly('details', 4),
        ...[
            ['catalog', `data-root="${root}"`], ['section', 'data-level="1"', `name="${name}"`],
            ['item', 'data-level="2"', `code="${code}"`], ['leaf', 'data-level="3"'],
        ].flatMap(([tag, ...attributes], depth) => {
            const summary = `article${' > details'.repeat(depth + 1)} > summary`;
            return [
                normalizedText(`${summary} > b`, tag),
                countExactly(`${summary} > code`, attributes.length),
                // Read each attribute's actual text without inserting spaces at binding boundaries.
                ...attributes.map((attribute, index) => propertyEquals(
                    `${summary} > code:nth-of-type(${index + 1})`, 'textContent', attribute)),
            ];
        }),
        normalizedText('article p', payload),
        ...externalDisclosureChecks('catalog'),
    ];
}

const mappedImageFragmentSample = sampleContract('4d. Mapped image and same-library fragment', [
    countExactly('article img', 2),
    text('article', '👌 from embed-relative-hash'),
    text('article', '👋 from embed-lib-component'),
    attributeContains('img[alt="Mapped Smiley"]', 'src', '/demo/lib-dir/Smiley.svg'),
    attributeContains('img[alt="Library Smiley"]', 'src', '/demo/lib-dir/Smiley.svg'),
    attributeContains('article > a', 'href', '/demo/lib-dir/embed-lib.html#embed-relative-hash'),
    attributeContains('article cem-element a', 'href', '/demo/lib-dir/embed-lib.html#embed-lib-component'),
]);

function moduleImageChecks(path, selector = 'image-link') {
    return [
        urlEquals(`${selector} img`, 'src', path),
        urlEquals(`${selector} a`, 'href', path),
        imageLoaded(`${selector} img`),
        shortenedHrefText(`${selector} a`, 32),
        propertyEquals(`${selector} details`, 'open', false),
        pressThenProperty(`${selector} summary`, 'Enter', `${selector} details`, 'open', true),
        text(`${selector} details`, path),
        pressThenProperty(`${selector} summary`, 'Space', `${selector} details`, 'open', false),
    ];
}
const moduleDemoPath = '/packages/cem-elements/demo/';
const moduleLibraryPath = `${moduleDemoPath}lib-dir/embed-lib.html`;
const moduleUrlSamples = [
    sampleContract('this page import maps', [text('pre', '"lib-root"'), text('pre', '"embed-lib"'),
        text('pre', '"demo-module-referrer"'), text('pre', '"scopes"')]),
    sampleContract('1. module path by symbolic name', moduleImageChecks(`${moduleDemoPath}wc-square.svg`)),
    sampleContract('2. src forms: relative URL', moduleImageChecks(`${moduleDemoPath}lib-dir/Smiley.svg?src=relative`)),
    sampleContract('3. src forms: absolute URL', [
        attributeContains('image-link img', 'src', 'data:image/svg+xml,'),
        attributeContains('image-link a', 'href', 'data:image/svg+xml,'), imageLoaded('image-link img'),
        shortenedHrefText('image-link a', 32),
        pressThenProperty('image-link summary', 'Enter', 'image-link details', 'open', true),
        pressThenProperty('image-link summary', 'Space', 'image-link details', 'open', false),
    ]),
    sampleContract('4. Relative declaration source', [
        { kind: 'resolvedUrlTexts', selector: 'output', expected: [`${moduleDemoPath}embed-1.html`] },
        urlEquals('a', 'href', `${moduleDemoPath}embed-1.html`),
        normalizedText('cem-module-relative-declaration', 'embed-1.html 🖖'),
    ]),
    sampleContract('4a. Mapped declaration source', [
        { kind: 'resolvedUrlTexts', selector: 'output', expected: [moduleLibraryPath] },
        urlEquals('a', 'href', moduleLibraryPath),
        normalizedText('cem-module-mapped-declaration', '👋 from embed-lib-component'),
    ]),
    sampleContract('4b. Missing import-map entry', [
        normalizedText('output', 'not published'), text('article', 'cem-element.module_url_resolve_failed'),
    ]),
    sampleContract('4c. Mapped fragment with a relative dependency', [
        { kind: 'resolvedUrlTexts', selector: 'output', expected: [`${moduleLibraryPath}#embed-relative-file`] },
        urlEquals('article > p a', 'href', `${moduleLibraryPath}#embed-relative-file`),
        urlEquals('cem-module-mapped-fragment a', 'href', `${moduleDemoPath}embed-1.html`),
        normalizedText('cem-module-mapped-fragment', '👍 from embed-relative-file invoking ../embed-1.html : embed-1.html 🖖'),
    ]),
    sampleContract(mappedImageFragmentSample.legend, [
        ...mappedImageFragmentSample.checks,
        urlEquals('img[alt="Mapped Smiley"]', 'src', `${moduleDemoPath}lib-dir/Smiley.svg`),
        urlEquals('img[alt="Library Smiley"]', 'src', `${moduleDemoPath}lib-dir/Smiley.svg`),
        imageLoaded('img[alt="Mapped Smiley"]'), imageLoaded('img[alt="Library Smiley"]'),
        urlEquals('article > a', 'href', `${moduleLibraryPath}#embed-relative-hash`),
        urlEquals('article cem-element a', 'href', `${moduleLibraryPath}#embed-lib-component`),
    ]),
    sampleContract('5. component-local map: naked', moduleImageChecks(`${moduleDemoPath}lib-dir/Smiley.svg?owner=component`)),
    sampleContract('6. component-local map: wrapper override', [
        ...moduleImageChecks(`${moduleDemoPath}confused.svg?owner=wrapper`),
        urlEquals('cem-local-map-override-wrapper > img', 'src', `${moduleDemoPath}lib-dir/Smiley.svg`),
        imageLoaded('cem-local-map-override-wrapper > img'),
    ]),
    sampleContract('7. component-local map: node referrer', [
        ...moduleImageChecks(`${moduleDemoPath}wc-square.svg?owner=component`, 'image-link.node-referrer-image'),
        nodeTexts('thead th', ['relative URL src', 'module path src', 'absolute URL src']),
        urlEquals('table td:first-of-type a', 'href', `${moduleDemoPath}lib-dir/Smiley.svg?referrer=node`),
        urlEquals('table td:nth-of-type(2) a', 'href', `${moduleDemoPath}wc-square.svg?owner=component`),
        urlEquals('table td:last-of-type a', 'href', 'https://assets.example.test/logo.svg'),
        text('cem-local-map-referrer', 'Child owns the inner-only module mapping'),
    ]),
    sampleContract('image-link', moduleImageChecks(`${moduleDemoPath}confused.svg`)),
];
const scopedCssSamples = [
    sampleContract('1. Private declaration CSS and ordinary outer cascade', [
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
    ]),
    sampleContract('2. Component in a named scope shares default declaration styles with peers in the same scope', [
        computedStyle('cem-css-shared-bare .sample-shared-bare', 'color', 'rgb(0, 128, 0)'),
        computedStyle('cem-css-shared-peer .sample-shared-bare', 'color', 'rgb(0, 128, 0)'),
        attributeEquals('cem-css-shared-bare', 'scope', 'css-samples'),
        attributeEquals('cem-css-shared-peer', 'scope', 'css-samples'),
        countExactly('cem-element[tag="cem-css-shared-bare"] > style[data-cem-declaration-style="shared"]', 1),
        styleTextContains(
            'cem-element[tag="cem-css-shared-bare"] > style[data-cem-declaration-style="shared"]',
            '[scope="css-samples"]:has(> template[data-cem-island="instance"])',
        ),
    ]),
    sampleContract('3. Style can be scoped explicitly', [
        computedStyle('cem-css-shared-explicit .shared-bg', 'backgroundColor', 'rgb(219, 234, 254)'),
        computedStyle('cem-css-explicit-peer .shared-bg', 'backgroundColor', 'rgb(219, 234, 254)'),
        countExactly('cem-element[tag="cem-css-shared-explicit"] > style[data-cem-declaration-style="shared"]', 1),
    ]),
    sampleContract('4. Mixed private and shared styles', [
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
    ]),
    sampleContract('5. Invalid and mismatched scopes fail closed', [
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
    ]),
    sampleContract('6. Payload style belongs to one instance', [
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
    ]),
    sampleContract('7. Declaration styles must be static', [
        countExactly('cem-element[tag="cem-css-dynamic"] > style[data-cem-declaration-style]', 0),
        countExactly('cem-css-dynamic style', 0),
        computedStyleNot('cem-css-dynamic .must-not-apply', 'color', 'rgb(255, 0, 0)'),
        computedStyleNot('cem-css-dynamic .must-not-apply', 'backgroundColor', 'rgb(255, 0, 0)'),
    ]),
    sampleContract('8. Fragment template CSS uses the effective produced tag', [
        computedStyle('cem-css-fragment .sample-fragment', 'backgroundColor', 'rgb(254, 243, 199)'),
        countExactly('cem-element[tag="cem-css-fragment"] > style[data-cem-declaration-style="private"]', 1),
        styleTextContains(
            'cem-element[tag="cem-css-fragment"] > style[data-cem-declaration-style="private"]',
            '@scope (\n    cem-css-fragment',
        ),
    ]),
    sampleContract('9. Anonymous declaration CSS uses its generated tag', [
        text('.sample-anonymous', 'anonymous'),
        computedStyle('.sample-anonymous', 'color', 'rgb(238, 130, 238)'),
        countExactly(
            'cem-element[uid-seed="demo/css/anonymous"] > style[data-cem-declaration-style="private"]',
            1,
        ),
        attributeContains('cem-element[uid-seed="demo/css/anonymous"]', 'tag', 'cem-'),
    ]),
    sampleContract('10. uid-seed stabilizes keyframe names', [
        attributeEquals('cem-element[tag="cem-css-keyframes"]', 'uid-seed', 'demo/css/keyframes'),
        keyframeIdentity(
            'cem-element[tag="cem-css-keyframes"] > style[data-cem-declaration-style="private"]',
            'cem-css-keyframes',
            '[part~="indicator"]',
            'seeded-pulse',
            'udemoz2fcssz2fkeyframes',
        ),
    ]),
    sampleContract('11. Descendant selectors stay inside the component', [
        computedStyle(
            'label',
            'color',
            'rgb(0, 128, 0)',
        ),
        computedStyle(
            '[slot="demo"] b',
            'color',
            'rgb(0, 0, 139)',
        ),
        computedStyleNot('[slot=demo] b', 'textShadow', 'none'),
        elementIdentity('input', 'remember'),
        pressThenProperty('input', 'Space', 'input', 'checked', false),
        computedStyle('[slot=demo] b', 'color', 'rgb(0, 0, 255)'),
        computedStyle('[slot=demo] b', 'textShadow', 'none'),
        computedStyle('label', 'color', 'rgb(0, 128, 0)'),
        pressThenProperty('input', 'Space', 'input', 'checked', true),
        computedStyle('[slot=demo] b', 'color', 'rgb(0, 0, 139)'),
        computedStyleNot('[slot=demo] b', 'textShadow', 'none'),
        elementIdentity('input', 'same'),
    ]),
    sampleContract('12. CSS from an external template fragment', [
        text('cem-css-external-fragment', 'projected external template'),
        computedStyle(
            'cem-css-external-fragment .external-scoped-item',
            'backgroundColor',
            'rgb(254, 243, 199)',
        ),
        computedStyle('.external-scoped-item', 'borderTopColor', 'rgb(180, 83, 9)'),
        countExactly('cem-element[tag="cem-css-external-fragment"] > style[data-cem-declaration-style="private"]', 1),
        countExactly('cem-css-external-fragment style', 0),
    ]),
];

const scopedCssNavigationChecks = [
    urlEquals('nav a', 'href', '/packages/cem-elements/index.html'),
    urlEquals('main > section a[href$="external-template.html"]', 'href', '/packages/cem-elements/demo/external-template.html'),
    urlEquals('main > section a[href$="hex-grid.html"]', 'href', '/packages/cem-elements/demo/hex-grid.html'),
    urlEquals('cem-css-external-fragment a', 'href', '/packages/cem-elements/demo/external-template-templates.html'),
];

const npmReleases = [
    ['0.1.0', '2026-08-01'], ['0.0.25', '2024-05-18'],
    ['0.0.22', '2024-04-20'], ['0.0.21', '2024-03-09'],
];
const npmPickerChecks = (tag, selected, dates = false, label = '@epa-wg/cem-elements version:') => [
    countExactly('select', 1),
    nodeTexts('select option', npmReleases.map(([version, date]) => dates ? `${version} - ${date}` : version)),
    ...npmReleases.map(([version], index) => attributeEquals(`option:nth-of-type(${index + 1})`, 'value', version)),
    propertyEquals('select', 'value', selected),
    attributeEquals(tag, 'value', ''),
    normalizedText('label span', label),
];
const npmVersionSamples = [
    sampleContract('1. Default to the latest version', [
        ...npmPickerChecks('cem-npm-version-default', '0.1.0'),
        elementIdentity('select', 'remember'),
        pressThenProperty('select', 'ArrowDown', 'select', 'value', '0.0.25'),
        attributeEquals('cem-npm-version-default', 'value', '0.0.25'),
        pressThenProperty('select', 'Home', 'select', 'value', '0.1.0'),
        attributeEquals('cem-npm-version-default', 'value', '0.1.0'),
        elementIdentity('select', 'same'),
    ]),
    sampleContract('2. Preselect a version and show dates', [
        ...npmPickerChecks('cem-npm-version-preselected', '0.0.22', true),
        elementIdentity('select', 'remember'),
        pressThenProperty('select', 'End', 'select', 'value', '0.0.21'),
        attributeEquals('cem-npm-version-preselected', 'value', '0.0.21'),
        pressThenProperty('select', 'ArrowUp', 'select', 'value', '0.0.22'),
        attributeEquals('cem-npm-version-preselected', 'value', '0.0.22'),
        attributeEquals('cem-npm-version-preselected', 'initialversion', '0.0.22'),
        elementIdentity('select', 'same'),
    ]),
    sampleContract('3. Propagate the selected value', [
        ...npmPickerChecks('cem-npm-version-propagated', '0.1.0'),
        normalizedText('output', ''),
        elementIdentity('select', 'remember'),
        ...['0.0.25', '0.0.21', '0.0.25'].flatMap(value => [
            selectThenText('select', value, 'output', value),
            propertyEquals('select', 'value', value),
            normalizedText('output', value),
            attributeEquals('cem-npm-version-propagated', 'value', value),
            elementIdentity('select', 'same'),
        ]),
    ]),
    sampleContract('4. Override the label slot', [
        ...npmPickerChecks('cem-npm-version-label', '0.1.0', false, 'Select a release:'),
        countExactly('label code', 0),
        normalizedText('i[slot="label"]', 'Select a release:'),
        normalizedText('output', ''),
        elementIdentity('select', 'remember'),
        ...['0.0.21', '0.1.0', '0.0.21'].flatMap(value => [
            selectThenText('select', value, 'output', value),
            propertyEquals('select', 'value', value),
            normalizedText('output', value),
            attributeEquals('cem-npm-version-label', 'value', value),
            elementIdentity('select', 'same'),
        ]),
    ]),
    sampleContract('5. Synchronize the selected version with the URL', [
        ...npmPickerChecks('cem-npm-version-url', '0.1.0', true),
        formState({ outputs: ['', ''] }),
        clickThenText('button[aria-label="Set URL to 0.0.22"]', 'article', 'Current hash: #version=0.0.22'),
        propertyEquals('select', 'value', '0.0.22'),
        formState({ outputs: ['#version=0.0.22', ''] }),
        selectThenText('select', '0.1.0', 'article', 'selected-version slice: 0.1.0'),
        formState({ outputs: ['#version=0.1.0', '0.1.0'] }),
        attributeEquals('cem-npm-version-url', 'value', '0.1.0'),
        clickThenText('button[aria-label="Set URL to 0.0.25"]', 'article', 'Current hash: #version=0.0.25'),
        propertyEquals('select', 'value', '0.0.25'),
        formState({ outputs: ['#version=0.0.25', '0.1.0'] }),
        selectThenText('select', '0.1.0', 'article', 'Current hash: #version=0.1.0'),
        propertyEquals('select', 'value', '0.1.0'),
        formState({ outputs: ['#version=0.1.0', '0.1.0'] }),
    ]),
];
const npmNavigationChecks = [
    urlEquals('nav a', 'href', '/packages/cem-elements/index.html'),
    ...['http-request', 'location-element', 'set-url'].map(name =>
        urlEquals(`main > section a[href$="/${name}.html"]`, 'href', `/packages/cem-elements/demo/${name}.html`)),
];

const moduleUrlNavigationChecks = [
    urlEquals('nav a', 'href', '/packages/cem-elements/index.html'),
    ...['set-url.html', 'external-template.html', 'module-url-referrer.html', 'functions/str.html'].map(path =>
        urlEquals(`main > section a[href$="${path}"]`, 'href', `${moduleDemoPath}${path}`)),
    countExactly('cem-module-url', 0),
];

const httpFullRows = ['alpha : ready', 'beta : loaded'];
const httpCompactRows = ['solo : compact'];
function httpUrlState(selected, requested, status, rows) {
    return [
        formState({ inputs: [selected], outputs: [selected, requested, status] }),
        propertyEquals('button[aria-label="GET"]', 'value', selected),
        elementIdentity('input', 'same'),
        elementIdentity('button[aria-label="GET"]', 'same'),
        countExactly('li', rows.length),
        ...rows.map((row, index) => normalizedText(`li:nth-of-type(${index + 1})`, row)),
        ...(status === 'failed' ? [text('article > p:last-of-type',
            'This URL did not provide an accepted JSON response. Choose All records or Compact records and press GET to recover.')] : []),
    ];
}
function editHttpUrl(value) {
    return [
        fillThenText('input', value, 'article > p:nth-of-type(2) output', value),
        focusedElement('input'),
        propertyEquals('input', 'selectionStart', value.length),
    ];
}
const httpSamples = [
    sampleContract('0. URL from text to http-request', [
        elementIdentity('input', 'remember'),
        elementIdentity('button[aria-label="GET"]', 'remember'),
        ...httpUrlState('./http-data.json', '', 'idle', []),
        ...editHttpUrl('./http-data-compact.json'),
        ...httpUrlState('./http-data-compact.json', '', 'idle', []),
        clickThenText('button:has-text("All records")', 'article', 'Selected URL: ./http-data.json'),
        ...httpUrlState('./http-data.json', '', 'idle', []),
        clickThenText('article > button', 'article', 'Request state: loaded'),
        ...httpUrlState('./http-data.json', './http-data.json', 'loaded', httpFullRows),
        clickThenText('button:has-text("Compact records")', 'article', 'Selected URL: ./http-data-compact.json'),
        ...httpUrlState('./http-data-compact.json', './http-data.json', 'loaded', httpFullRows),
        clickThenText('article > button', 'li', 'solo : compact'),
        ...httpUrlState('./http-data-compact.json', './http-data-compact.json', 'loaded', httpCompactRows),
        clickThenText('button[aria-label="Invalid JSON response"]', 'article', 'Selected URL: ./http-data-invalid.json'),
        ...httpUrlState('./http-data-invalid.json', './http-data-compact.json', 'loaded', httpCompactRows),
        clickThenText('article > button', 'article', 'Request state: failed'),
        ...httpUrlState('./http-data-invalid.json', './http-data-invalid.json', 'failed', []),
        clickThenText('button:has-text("All records")', 'article', 'Selected URL: ./http-data.json'),
        ...httpUrlState('./http-data.json', './http-data-invalid.json', 'failed', []),
        clickThenText('article > button', 'li', 'beta : loaded'),
        ...httpUrlState('./http-data.json', './http-data.json', 'loaded', httpFullRows),
        clickThenText('button[aria-label="Empty URL"]', 'article', 'Selected URL:'),
        ...httpUrlState('', './http-data.json', 'loaded', httpFullRows),
        clickThenText('article > button', 'article', 'Request state: idle'),
        ...httpUrlState('', '', 'idle', []),
        ...editHttpUrl('./http-data-compact.json'),
        ...httpUrlState('./http-data-compact.json', '', 'idle', []),
        pressThenProperty('button[aria-label="GET"]', 'Enter', 'article > p:nth-of-type(4) output', 'textContent', 'loaded'),
        ...httpUrlState('./http-data-compact.json', './http-data-compact.json', 'loaded', httpCompactRows),
        ...editHttpUrl('./http-data.json'),
        ...httpUrlState('./http-data.json', './http-data-compact.json', 'loaded', httpCompactRows),
        clickThenText('article > button', 'li', 'beta : loaded'),
        ...httpUrlState('./http-data.json', './http-data.json', 'loaded', httpFullRows),
    ]),
    sampleContract('1. Simplest http-request', [
        countExactly('.result-buttons button', 6),
        countExactly('.result-buttons img', 6),
        countExactly('.result-buttons span', 0),
        normalizedText('article output', 'loaded'),
        ...['bulbasaur', 'ivysaur', 'venusaur', 'charmander', 'charmeleon', 'charizard'].flatMap((name, index) => {
            const button = `.result-buttons button:nth-of-type(${index + 1})`;
            return [
                attributeEquals(button, 'aria-label', name),
                attributeEquals(button, 'title', name),
                attributeEquals(button, 'type', 'button'),
                normalizedText(button, ''),
                attributeEquals(`${button} img`, 'alt', name),
                attributeEquals(`${button} img`, 'src',
                    `https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/${index + 1}.svg`),
                imageLoaded(`${button} img`),
            ];
        }),
    ]),
    sampleContract('2. http-request response and headers', [
        ...['State', 'Method', 'Authored URL', 'Resolved URL', 'Accept header', 'X-Demo header', 'Status', 'Content type']
            .map((term, index) => normalizedText(`dl dt:nth-of-type(${index + 1})`, term)),
        normalizedText('dl dd:nth-of-type(1)', 'loaded'),
        normalizedText('dl dd:nth-of-type(2)', 'GET'),
        normalizedText('dl dd:nth-of-type(3)', './http-data.json'),
        normalizedText('dl dd:nth-of-type(5)', 'application/json'),
        normalizedText('dl dd:nth-of-type(6)', 'ported-from-legacy'),
        normalizedText('dl dd:nth-of-type(7)', '200'),
        text('dl dd:nth-of-type(8)', 'application/json'),
        attributeContains('dl a', 'href', '/packages/cem-elements/demo/http-data.json'),
        shortenedHrefText('dl a', 32),
    ]),
];

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

const inspectorOuter = 'table[aria-label="document/row"]';
const inspectorNested = index => `${inspectorOuter} > tbody > tr:nth-child(${index}) table`;
const inspectorDisclosure = index => `${inspectorOuter} > tbody > tr:nth-child(${index}) details:has(> .table-scroll)`;
const inspectorFruit = [['2', 'Cherry 🍒'], ['10', 'Lemon 🍋'], ['02', 'Apple 🍏'], ['∅', 'Banana 🍌']];
const tableInspectorSamples = [
    sampleContract('1. Columns from every row', [
        inspectorTable('table', [['""', '🍒', '∅'], ['∅', '🍋', 'ripe']], [],
            { headings: ['✓', '@early', '#text', '@later'], sort: null, mode: 'text' }),
        elementIdentity('table > tbody > tr:first-child input', 'remember'),
        pressThenProperty('table > tbody > tr:first-child input', 'Space',
            'table > tbody > tr:first-child input', 'checked', true),
        inspectorTable('table', [['""', '🍒', '∅'], ['∅', '🍋', 'ripe']], [0]),
        pressThenProperty('table > tbody > tr:first-child input', 'Space',
            'table > tbody > tr:first-child input', 'checked', false),
        inspectorTable('table', [['""', '🍒', '∅'], ['∅', '🍋', 'ripe']]),
        elementIdentity('table > tbody > tr:first-child input', 'same'),
    ]),
    sampleContract('2. Text-only rows stay visible', [
        inspectorTable('table', [['ivysaur'], ['venusaur']], [], { headings: ['✓', '#text'], sort: null }),
        pressThenProperty('button[aria-label="Sort #text descending in document/name"]', 'Enter',
            'th[aria-sort]', 'ariaSort', 'descending'),
        inspectorTable('table', [['venusaur'], ['ivysaur']], [], { sort: 'descending' }),
        clickThenText('button[aria-label="Sort #text ascending in document/name"]',
            'table > tbody > tr:first-child > td', 'ivysaur'),
        inspectorTable('table', [['ivysaur'], ['venusaur']], [], { sort: 'ascending' }),
        clickThenText('button[aria-label="Restore source order in document/name"]',
            'table > tbody > tr:first-child > td', 'ivysaur'),
        inspectorTable('table', [['ivysaur'], ['venusaur']], [], { sort: null }),
    ]),
    sampleContract('3. Nested tables keep their own state', [
        countExactly('table', 3),
        inspectorTable(inspectorOuter, [['""', null], ['""', null]], [], { headings: ['✓', '#text', 'tags'], sort: null }),
        inspectorTable(inspectorNested(1), [['red'], ['sweet']], [], { headings: ['✓', '#text'], sort: null }),
        inspectorTable(inspectorNested(2), [['yellow'], ['tart']], [], { headings: ['✓', '#text'], sort: null }),
        checkThenText(`${inspectorNested(1)} > tbody > tr:first-child input`,
            `${inspectorNested(1)} > caption > output`, '1'),
        clickThenText(`${inspectorNested(1)} button[aria-label="Sort #text descending in tags/tag"]`,
            `${inspectorNested(1)} > tbody > tr:first-child > td`, 'sweet'),
        inspectorTable(inspectorNested(1), [['sweet'], ['red']], [1], { sort: 'descending' }),
        elementIdentity(inspectorDisclosure(1), 'remember'),
        pressThenProperty(`${inspectorDisclosure(1)} > summary`, 'Enter', inspectorDisclosure(1), 'open', false),
        pressThenProperty(`${inspectorOuter} > tbody > tr:first-child > th input`, 'Space',
            `${inspectorOuter} > tbody > tr:first-child > th input`, 'checked', true),
        inspectorTable(inspectorOuter, [['""', null], ['""', null]], [0], { sort: null }),
        inspectorTable(inspectorNested(1), [['sweet'], ['red']], [1], { sort: 'descending' }),
        propertyEquals(inspectorDisclosure(1), 'open', false),
        elementIdentity(inspectorDisclosure(1), 'same'),
        pressThenProperty(`${inspectorDisclosure(1)} > summary`, 'Space', inspectorDisclosure(1), 'open', true),
        focusedElement(`${inspectorDisclosure(1)} > summary`),
        clickThenText(`${inspectorNested(2)} button[aria-label="Sort #text ascending in tags/tag"]`,
            `${inspectorNested(2)} > tbody > tr:first-child > td`, 'tart'),
        inspectorTable(inspectorNested(2), [['tart'], ['yellow']], [], { sort: 'ascending' }),
        inspectorTable(inspectorNested(1), [['sweet'], ['red']], [1], { sort: 'descending' }),
        inspectorTable(inspectorOuter, [['""', null], ['""', null]], [0], { sort: null }),
    ]),
    sampleContract('4. Multiple selections survive sorting', [
        inspectorTable('table', inspectorFruit, [], { headings: ['✓', '@qty', '#text'], sort: null, mode: 'text' }),
        elementIdentity('textarea', 'remember'),
        pressThenProperty('table > tbody > tr:nth-child(2) input', 'Space',
            'table > tbody > tr:nth-child(2) input', 'checked', true),
        inspectorTable('table', inspectorFruit, [1]),
        pressThenProperty('table > tbody > tr:nth-child(3) input', 'Space',
            'table > tbody > tr:nth-child(3) input', 'checked', true),
        inspectorTable('table', inspectorFruit, [1, 2]),
        clickThenText('button[aria-label="Sort @qty ascending in document/row"]',
            'table > tbody > tr:first-child > td:last-child', 'Apple 🍏'),
        inspectorTable('table', [inspectorFruit[2], inspectorFruit[1], inspectorFruit[0], inspectorFruit[3]], [0, 1], { sort: 'ascending', mode: 'text' }),
        selectThenText('select[aria-label="Compare document/row"]', 'number',
            'table > tbody > tr:first-child > td:last-child', 'Cherry 🍒'),
        inspectorTable('table', [inspectorFruit[0], inspectorFruit[2], inspectorFruit[1], inspectorFruit[3]], [1, 2], { sort: 'ascending', mode: 'number' }),
        clickThenText('button[aria-label="Sort @qty descending in document/row"]',
            'table > tbody > tr:first-child > td:last-child', 'Lemon 🍋'),
        inspectorTable('table', [inspectorFruit[1], inspectorFruit[0], inspectorFruit[2], inspectorFruit[3]], [0, 2], { sort: 'descending', mode: 'number' }),
        pressThenProperty('table > tbody > tr:first-child input', 'Space',
            'table > tbody > tr:first-child input', 'checked', false),
        inspectorTable('table', [inspectorFruit[1], inspectorFruit[0], inspectorFruit[2], inspectorFruit[3]], [2]),
        clickThenText('button[aria-label="Restore source order in document/row"]',
            'table > tbody > tr:first-child > td:last-child', 'Cherry 🍒'),
        inspectorTable('table', inspectorFruit, [2], { sort: null }),
        clickThenText('button[aria-label="Reset source"]', 'caption output', '0'),
        inspectorTable('table', inspectorFruit),
        fillBlurThenText('textarea', '<broken>', '[role="alert"]', 'XML'),
        countExactly('table', 0),
        elementIdentity('textarea', 'same'),
        clickThenText('button[aria-label="Reset source"]', 'table > tbody > tr:first-child > td:last-child', 'Cherry 🍒'),
        inspectorTable('table', inspectorFruit),
        countExactly('[role="alert"]', 0),
        checkThenText('table > tbody > tr:first-child input', 'caption output', '1'),
        fillBlurThenText('textarea', '<fruit><row qty="3">Pear</row><row qty="1">Peach</row></fruit>',
            'table > tbody > tr:first-child > td:last-child', 'Pear'),
        inspectorTable('table', [['3', 'Pear'], ['1', 'Peach']]),
        elementIdentity('textarea', 'same'),
        clickThenText('button[aria-label="Reset source"]', 'table > tbody > tr:first-child > td:last-child', 'Cherry 🍒'),
        inspectorTable('table', inspectorFruit),
    ]),
];

const dataTreePage = await readFile(join(repoRoot, 'packages/cem-elements/demo/data-tree.html'), 'utf8');
const treeSource = format => dataTreePage.split(`<cem-data-tree format="${format}">`)[1]
    .split('</cem-data-tree>')[0].replaceAll('&lt;', '<').replaceAll('&gt;', '>').replaceAll('&amp;', '&').trim();
const treeBranch = label => `li:has(> label > input[aria-label="Select branch ${label}"])`;
const dataTreeSamples = [
    sampleContract('1. XML branches: independent selection', [
        text('pre[aria-label="CEM-ML document"]', '{ast'),
        computedStyle('pre[aria-label="CEM-ML document"] code', 'whiteSpace', 'pre-wrap'),
        text('pre[aria-label="CEM-ML document"]', '@kind=cdata'), text('pre[aria-label="CEM-ML document"]', '@target=keep'),
        treeSelection([], ['1: orchard', '1.1: fruit', '1.2: fruit']),
        countExactly(`${treeBranch('1: orchard')} > details > ul > li`, 0),
        countExactly(`${treeBranch('1.1: fruit')} > details > ul > li`, 1),
        countExactly(`${treeBranch('1.2: fruit')} > details > ul > li`, 0),
        text(`${treeBranch('1.1: fruit')} > details > summary`, 'urn:fruit'),
        normalizedText(`${treeBranch('1.1: fruit')} > details > ul`, '@ color : ""'),
        propertyEquals(`${treeBranch('1.1: fruit')} > details > ul .value`, 'textContent', '""'),
        ...['text : pre', 'cdata : <raw>🍋', 'processing-instruction · keep : inert', 'text : post'].map((value, index) =>
            normalizedText(`${treeBranch('1.2: fruit')} > details > ol > li:nth-child(${index + 1})`, value)),
        pressThenProperty('input[aria-label="Select branch 1.1: fruit"]', 'Space',
            'input[aria-label="Select branch 1.1: fruit"]', 'checked', true),
        treeSelection(['1.1: fruit']),
        pressThenProperty('input[aria-label="Select branch 1.2: fruit"]', 'Space',
            'input[aria-label="Select branch 1.2: fruit"]', 'checked', true),
        treeSelection(['1.1: fruit', '1.2: fruit']),
        pressThenProperty(`${treeBranch('1.1: fruit')} > details > summary`, 'Enter',
            `${treeBranch('1.1: fruit')} > details`, 'open', false),
        treeSelection(['1.1: fruit', '1.2: fruit']),
        pressThenProperty('input[aria-label="Select branch 1.2: fruit"]', 'Space',
            'input[aria-label="Select branch 1.2: fruit"]', 'checked', false),
        treeSelection(['1.1: fruit']),
        propertyEquals(`${treeBranch('1.1: fruit')} > details`, 'open', false),
        pressThenProperty(`${treeBranch('1.1: fruit')} > details > summary`, 'Space',
            `${treeBranch('1.1: fruit')} > details`, 'open', true),
        clickThenText('article > button', 'output[aria-label="Selected branches"]', '0'),
        treeSelection([]),
        propertyEquals('textarea', 'value', treeSource('xml')),
        clickThenText('input[aria-label="Select branch 1.1: fruit"]', 'output[aria-label="Selected branches"]', '1'),
        fillBlurThenText('textarea', '<r><script>neverRun()</script><fruit>🍏</fruit></r>', 'pre[aria-label="CEM-ML document"]', 'neverRun()'),
        treeSelection([], ['1: r', '1.1: script', '1.2: fruit']),
        countExactly(':is(script, raw)', 0),
        clickThenText('article > button', 'pre[aria-label="CEM-ML document"]', '<raw>🍋'),
        treeSelection([], ['1: orchard', '1.1: fruit', '1.2: fruit']),
        propertyEquals('textarea', 'value', treeSource('xml')),
    ]),
    sampleContract('2. JSON through the same CEM tree', [
        text('pre[aria-label="CEM-ML document"]', '{ast'),
        treeSelection([], ['1: object', '1.1: property', '1.1.1: string', '1.2: property',
            '1.2.1: string', '1.3: property', '1.3.1: array', '1.3.1.1: string', '1.3.1.2: string']),
        text(treeBranch('1.1: property'), 'fruit'),
        text(treeBranch('1.1.1: string'), '🍒'),
        text(treeBranch('1.2: property'), 'note'),
        text(treeBranch('1.2.1: string'), '""'),
        text(treeBranch('1.3.1.1: string'), 'red'),
        text(treeBranch('1.3.1.2: string'), 'sweet'),
        clickThenText('input[aria-label="Select branch 1.3: property"]', 'output[aria-label="Selected branches"]', '1'),
        treeSelection(['1.3: property']),
        clickThenText('input[aria-label="Select branch 1.3.1.2: string"]', 'output[aria-label="Selected branches"]', '2'),
        treeSelection(['1.3: property', '1.3.1.2: string']),
        elementIdentity(`${treeBranch('1.3: property')} > details`, 'remember'),
        elementIdentity(`${treeBranch('1.3: property')} > details > summary`, 'remember'),
        elementIdentity(`${treeBranch('1.3.1.2: string')} > label > input`, 'remember'),
        pressThenProperty(`${treeBranch('1.3: property')} > details > summary`, 'Enter',
            `${treeBranch('1.3: property')} > details`, 'open', false),
        treeSelection(['1.3: property', '1.3.1.2: string']),
        clickThenText('input[aria-label="Select branch 1.3: property"]', 'output[aria-label="Selected branches"]', '1'),
        treeSelection(['1.3.1.2: string']),
        elementIdentity(`${treeBranch('1.3: property')} > details`, 'check'),
        elementIdentity(`${treeBranch('1.3: property')} > details > summary`, 'check'),
        elementIdentity(`${treeBranch('1.3.1.2: string')} > label > input`, 'check'),
        propertyEquals(`${treeBranch('1.3: property')} > details`, 'open', false),
        pressThenProperty(`${treeBranch('1.3: property')} > details > summary`, 'Space',
            `${treeBranch('1.3: property')} > details`, 'open', true),
        // A change event with the same source bytes must clear selection too.
        dispatchThenText('textarea', 'change', 'output[aria-label="Selected branches"]', '0'),
        treeSelection([]),
        clickThenText('input[aria-label="Select branch 1.2.1: string"]', 'output[aria-label="Selected branches"]', '1'),
        treeSelection(['1.2.1: string']),
        clickThenText('article > button', 'output[aria-label="Selected branches"]', '0'),
        treeSelection([]),
        propertyEquals('textarea', 'value', treeSource('json')),
    ]),
    sampleContract('3. Malformed source and repair', [
        text('[role="alert"]', 'could not be imported'),
        countExactly('article :is(pre, input, output)', 0),
        fillBlurThenText('textarea', '<orchard><fruit>🍒</fruit></orchard>', 'pre[aria-label="CEM-ML document"]', '{ast'),
        countExactly('[role="alert"]', 0),
        treeSelection([], ['1: orchard', '1.1: fruit']),
        clickThenText('input[aria-label="Select branch 1.1: fruit"]', 'output', '1'),
        treeSelection(['1.1: fruit']),
        clickThenText('article > button', '[role="alert"]', 'could not be imported'),
        countExactly('article :is(pre, input, output)', 0),
        propertyEquals('textarea', 'value', '<orchard><fruit>🍒</orchard>'),
        fillBlurThenText('textarea', '<orchard><fruit>🍋</fruit></orchard>', 'pre[aria-label="CEM-ML document"]', '🍋'),
        treeSelection([], ['1: orchard', '1.1: fruit']),
        countExactly('[role="alert"]', 0),
    ]),
    sampleContract('4. Load and release a local document', [
        text('pre[aria-label="CEM-ML document"]', 'xml-stylesheet'), text('pre[aria-label="CEM-ML document"]', '🍒'), text('pre[aria-label="CEM-ML document"]', '🍋'),
        propertyEquals('select[aria-label="Local source"]', 'value', './tree-source.xml'),
        propertyEquals('output[aria-label="Request state"]', 'textContent', 'loaded'),
        attributeContains('article a', 'href', '/packages/cem-elements/demo/tree-source.xml'),
        attributeEquals('article a', 'download', ''),
        treeSelection([]),
        clickThenText('article > section:nth-of-type(2) > ol > li > label > input', 'output[aria-label="Selected branches"]', '1'),
        countExactly('input:checked', 1),
        selectThenText('select[aria-label="Local source"]', './tree-source.json', 'pre[aria-label="CEM-ML document"]', 'tree-source.json'),
        propertyEquals('output[aria-label="Request state"]', 'textContent', 'loaded'),
        propertyEquals('select[aria-label="Local source"]', 'value', './tree-source.json'),
        attributeContains('article a', 'href', '/packages/cem-elements/demo/tree-source.json'),
        attributeEquals('article a', 'download', ''),
        text('pre[aria-label="CEM-ML document"]', '🍋'),
        treeSelection([]),
        clickThenText('input[aria-label="Select branch 1: object"]', 'output[aria-label="Selected branches"]', '1'),
        treeSelection(['1: object']),
        selectThenText('select[aria-label="Local source"]', '', 'output[aria-label="Request state"]', 'idle'),
        propertyEquals('select[aria-label="Local source"]', 'value', ''),
        countExactly(':is(article pre, article input, article a, output[aria-label="Selected branches"])', 0),
        selectThenText('select[aria-label="Local source"]', './tree-source.xml', 'pre[aria-label="CEM-ML document"]', 'xml-stylesheet'),
        treeSelection([]),
        propertyEquals('output[aria-label="Request state"]', 'textContent', 'loaded'),
        attributeContains('article a', 'href', '/packages/cem-elements/demo/tree-source.xml'),
        countExactly('script', 0),
    ]),
    sampleContract('tree-source.xml', [
        attributeEquals(':scope', 'src', './tree-source.xml'),
        attributeEquals(':scope', 'type', 'xml'),
        attributeEquals(':scope', 'demo', 'false'),
        attributeEquals(':scope', 'data-state', 'ready'),
        propertyEquals('[slot=text] code', 'textContent',
            await readFile(join(repoRoot, 'packages/cem-elements/demo/tree-source.xml'), 'utf8')),
        propertyEquals('[slot=demo]', 'textContent', ''),
        countExactly('[slot=demo] > *', 0),
    ]),
    sampleContract('tree-source.json', [
        attributeEquals(':scope', 'src', './tree-source.json'),
        attributeEquals(':scope', 'type', 'json'),
        attributeEquals(':scope', 'demo', 'false'),
        attributeEquals(':scope', 'data-state', 'ready'),
        propertyEquals('[slot=text] code', 'textContent',
            await readFile(join(repoRoot, 'packages/cem-elements/demo/tree-source.json'), 'utf8')),
        propertyEquals('[slot=demo]', 'textContent', ''),
        countExactly('[slot=demo] > *', 0),
    ]),
];

const setUrlMethods = ['location.href', 'location.hash', 'location.assign', 'location.replace',
    'history.pushState', 'history.replaceState'];
const setUrlSamples = [
    sampleContract('1. Set the page hash', [
        nodeTexts('button', ['#hash-one', '#hash-two']), nodeTexts('output', ['', '']),
    ]),
    sampleContract('2. Select the URL write method', [
        nodeTexts('button', setUrlMethods), nodeTexts('output', ['', '']),
    ]),
    sampleContract('3. Conditionally inject a URL writer', [
        countExactly('button', 1), nodeTexts('output', ['']),
    ]),
    sampleContract('4. Set URL from form controls', [
        countExactly('input[type="radio"]', 6), countExactly('input:checked', 1),
        propertyEquals('input:checked', 'value', 'history.pushState'),
        propertyEquals('input[type="text"]', 'value', '#form-driven'),
        nodeTexts('output', ['history.pushState', '#form-driven', '']),
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
        checks: xpathFunctionSamples.flatMap(sample => sample.checks.map(check =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/dom-merge.html',
        checks: domMergeSamples.flatMap(sample => sample.checks.map(check =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/external-template.html',
        allowedPageErrors: ['Failed to load resource: the server responded with a status of 404 (Not Found)'],
        checks: externalTemplateSamples.flatMap(sample => sample.checks.map(check =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/for-each.html',
        checks: forEachSamples.flatMap(sample => sample.checks.map(check =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/form.html',
        checks: [
            countExactly('cem-demo-element[legend]', 5),
            text('main > section', 'datadom.formData.<slice>'),
            ...formSamples.flatMap(sample => sample.checks.map(check =>
                scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
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
        checks: httpSamples.flatMap(sample => sample.checks.map(check =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/local-storage.html',
        checks: localStorageSamples.flatMap((sample) => sample.checks.map(
            (check) => scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`),
        )),
    },
    {
        path: '/packages/cem-elements/demo/location-element.html',
        checks: locationSamples.flatMap(sample => sample.checks.map(check =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/module-url.html',
        checks: [...moduleUrlSamples.flatMap(sample => sample.checks.map(check =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))), ...moduleUrlNavigationChecks],
    },
    {
        path: '/packages/cem-elements/demo/module-url-referrer.html',
        checks: scalarReferrerSample.checks.map(check => scopeCheck(check,
            `cem-demo-element[legend="${scalarReferrerSample.legend}"]`)),
    },
    {
        path: '/packages/cem-elements/demo/functions/dom.html',
        checks: domChainSamples.flatMap(sample => sample.checks.map(check =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/functions/str.html',
        checks: stringSamples.flatMap(sample => sample.checks.map(check =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
    {
        path: '/packages/cem-elements/demo/npm-versions-demo.html',
        checks: [...npmVersionSamples.flatMap(sample => sample.checks.map(check =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))), ...npmNavigationChecks],
    },
    {
        path: '/packages/cem-elements/demo/scoped-css.html',
        checks: [...scopedCssSamples.flatMap(sample => sample.checks.map(check =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))), ...scopedCssNavigationChecks],
    },
    {
        path: '/packages/cem-elements/demo/set-url.html',
        checks: setUrlSamples.flatMap(sample => sample.checks.map(check =>
            scopeCheck(check, `cem-demo-element[legend="${sample.legend}"]`))),
    },
];

const sourceDocumentSpecs = [
    { path: '/packages/cem-elements/demo/cell-overrides.html', samples: cellOverrideSamples },
    {
        path: '/packages/cem-elements/demo/table-inspector.html',
        samples: tableInspectorSamples,
        declarationAttributes: { 'link-base': 'source' },
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
        declarationAttributes: { 'link-base': 'source' },
    },
    {
        path: '/packages/cem-elements/demo/xpath-sequences.html',
        samples: xpathSequenceSamples,
        declarationAttributes: { 'link-base': 'source' },
    },
    {
        path: '/packages/cem-elements/demo/xpath-maps-arrays.html',
        samples: xpathMapArraySamples,
        declarationAttributes: { 'link-base': 'source' },
    },
    {
        path: '/packages/cem-elements/demo/xpath-validation.html',
        samples: xpathValidationSamples,
        declarationAttributes: { 'link-base': 'source' },
    },
    {
        path: '/packages/cem-elements/demo/xpath-sort.html',
        samples: xpathSortSamples,
        declarationAttributes: { 'link-base': 'source' },
    },
    {
        path: '/packages/cem-elements/demo/xpath-aggregates.html',
        samples: xpathAggregateSamples,
        declarationAttributes: { 'link-base': 'source' },
    },
    {
        path: '/packages/cem-elements/demo/xpath-functions.html',
        samples: xpathFunctionSamples,
        declarationAttributes: { 'link-base': 'source' },
    },
    {
        path: '/packages/cem-elements/demo/dom-merge.html',
        samples: domMergeSamples,
    },
    { path: '/packages/cem-elements/demo/embed-1.html', checks: supportEmbeddedDocumentChecks() },
    {
        path: '/packages/cem-elements/demo/embed-lib.html#embed-lib-component',
        checks: [normalizedText(':scope', '👋 from embed-lib-component'), countExactly(':is(h1,h4,a,img,article,script)', 0)],
    },
    {
        path: '/packages/cem-elements/demo/external-template-document.html',
        checks: [countExactly('article.external-document-template', 1),
            propertyEquals('h2', 'textContent', 'External document'), propertyEquals('p', 'textContent', 'External document fallback')],
    },
    {
        path: '/packages/cem-elements/demo/external-template-templates.html#external-card-template',
        attributes: { title: 'Source-loaded card' },
        content: 'Projected source content',
        declarationAttributes: { 'link-base': 'source' },
        checks: [countExactly('article', 1), propertyEquals('h2', 'textContent', 'Source-loaded card'),
            propertyEquals('p', 'textContent', 'Projected source content'),
            urlEquals('a', 'href', '/packages/cem-elements/demo/external-template-templates.html#external-card-template'),
            countExactly(':is(#external-subtree-template, .external-scoped-card, script)', 0)],
    },
    {
        path: '/packages/cem-elements/demo/external-template.html',
        allowedPageErrors: ['Failed to load resource: the server responded with a status of 404 (Not Found)'],
        samples: externalTemplateSamples,
    },
    {
        path: '/packages/cem-elements/demo/for-each.html',
        samples: forEachSamples,
    },
    {
        path: '/packages/cem-elements/demo/form.html',
        samples: formSamples,
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
    { path: '/packages/cem-elements/demo/html-template.html', checks: [
        propertyEquals('#wave', 'textContent', '👋'), propertyEquals('#ok', 'textContent', '👌'),
        countExactly('b', 2), ...supportSvgChecks(), ...supportMathChecks(), countExactly('script', 0)] },
    {
        path: '/packages/cem-elements/demo/http-request.html',
        samples: httpSamples,
    },
    {
        path: '/packages/cem-elements/demo/lib-dir/embed-lib.html#embed-lib-component',
        checks: [normalizedText(':scope', '👋 from embed-lib-component'), countExactly(':is(h1,h4,a,img,article,script)', 0)],
    },
    {
        path: '/packages/cem-elements/demo/local-storage.html',
        samples: localStorageSamples,
    },
    {
        path: '/packages/cem-elements/demo/location-element.html',
        samples: locationSamples,
    },
    {
        path: '/packages/cem-elements/demo/module-url.html',
        samples: moduleUrlSamples,
        checks: moduleUrlNavigationChecks,
        declarationAttributes: { 'link-base': 'source' },
    },
    {
        path: '/packages/cem-elements/demo/module-url-referrer.html',
        samples: [scalarReferrerSample],
        declarationAttributes: { 'link-base': 'source' },
    },
    {
        path: '/packages/cem-elements/demo/functions/dom.html',
        samples: domChainSamples,
        declarationAttributes: { 'link-base': 'source' },
    },
    {
        path: '/packages/cem-elements/demo/functions/str.html',
        samples: stringSamples,
        declarationAttributes: { 'link-base': 'source' },
    },
    {
        path: '/packages/cem-elements/demo/npm-versions-demo.html',
        samples: npmVersionSamples,
        checks: npmNavigationChecks,
        declarationAttributes: { 'link-base': 'source' },
    },
    {
        path: '/packages/cem-elements/demo/scoped-css.html',
        samples: scopedCssSamples,
        checks: scopedCssNavigationChecks,
        declarationAttributes: { 'link-base': 'source' },
    },
    {
        path: '/packages/cem-elements/demo/set-url.html',
        samples: setUrlSamples,
        declarationAttributes: { 'link-base': 'source' },
    },
];

// External source cards are part of both the standalone and source-loaded inventories.
for (const [directory, page, files] of [
    ['cem-elements', 'http-request.html', ['http-data.json', 'http-data-compact.json', 'http-data-invalid.json', 'http-pokemon.json']],
    ['cem-elements', 'npm-versions-demo.html', ['npm-versions.json']],
    ['custom-element', 'http-request.html', ['http-data.json', 'http-data-compact.json', 'http-data-invalid.json', 'http-pokemon.json']],
    ['custom-element', 'npm-versions-demo.html', ['npm-versions.json']],
]) {
    const path = `/packages/${directory}/demo/${page}`;
    const previews = await Promise.all(files.map(async file => sampleContract(file, [
        attributeEquals(':scope', 'src', `./${file}`),
        attributeEquals(':scope', 'type', file.endsWith('.xml') ? 'xml' : 'json'),
        attributeEquals(':scope', 'demo', 'false'),
        countExactly('[slot="demo"] > *', 0),
        ...(directory === 'cem-elements' && page === 'http-request.html' ? [
            attributeEquals(':scope', 'data-state', 'ready'),
            propertyEquals('[slot=text] code', 'textContent',
                await readFile(join(repoRoot, 'packages/cem-elements/demo', file), 'utf8')),
            file === 'http-data-invalid.json' ? attributeEquals('[slot=text] code', 'data-language', 'js')
                : countAtLeast('[slot=text] code i', 1),
            propertyEquals('[slot=demo]', 'textContent', ''),
        ] : []),
    ])));
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
        const filePath = normalize(join(repoRoot, pathname));
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
        const resolutionRequests = observeModuleUrlRequests(page, fixture);
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
            if (fixture.path === '/packages/cem-elements/demo/module-url.html') {
                await verifyModuleUrlPresentation(page, resolutionRequests, `http://127.0.0.1:${port}${fixture.path}`);
            }
            if (fixture.path === '/packages/cem-elements/demo/npm-versions-demo.html') {
                await verifyNpmVersionsPresentation(page);
                await verifyDemoLayout(page, 6);
                await verifyNpmVersionsLifecycle(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/scoped-css.html') {
                await verifyScopedCssPresentation(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/set-url.html') {
                await verifySetUrlLifecycle(page);
                await verifySetUrlPresentation(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/table-inspector.html') {
                await verifyTableInspectorPresentation(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/xpath-aggregates.html') {
                await verifyXPathAggregatesPresentation(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/xpath-functions.html') {
                await verifyXPathFunctionsPresentation(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/xpath-maps-arrays.html') {
                await verifyXPathMapsArraysPresentation(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/xpath-nodes.html') {
                await verifyXPathNodesPresentation(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/xpath-sequences.html') {
                await verifyXPathSequencesPresentation(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/functions/str.html') {
                await verifyStringFunctionsPresentation(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/functions/dom.html') {
                await verifyDomChainsPresentation(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/xpath-validation.html') {
                await verifyXPathValidationPresentation(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/xpath-sort.html') {
                await verifyXPathSortPresentation(page);
            }
            await verifySymbolicControls(page, fixture.path);
            if (fixture.path === '/packages/cem-elements/demo/module-url-referrer.html') {
                await verifyScalarReferrerPresentation(page, resolutionRequests,
                    `http://127.0.0.1:${port}${fixture.path}`);
            }
            if (fixture.path === '/packages/cem-elements/demo/dom-merge.html') {
                await verifyDemoLayout(page, 3);
            }
            if (fixture.path === '/packages/cem-elements/demo/for-each.html') {
                await verifyDemoLayout(page, 11);
            }
            if (fixture.path === '/packages/cem-elements/demo/form.html') {
                await verifyDemoLayout(page, 5);
            }
            if (fixture.path === '/packages/cem-elements/demo/http-request.html') {
                await verifyDemoLayout(page, 7);
            }
            if (fixture.path === '/packages/cem-elements/demo/hex-grid.html') {
                await verifyHexSamples(page);
                await verifyDemoLayout(page, 9);
                await page.setViewportSize({ width: 1280, height: 900 });
                await verifyHexRowNavigation(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/local-storage.html') {
                await verifyDemoLayout(page, 12);
                await verifyLocalStorageLifecycle(page);
            }
            if (fixture.path === '/packages/cem-elements/demo/location-element.html') {
                await verifyDemoLayout(page, 3);
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
        const resolutionRequests = observeModuleUrlRequests(page, fixture);
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
            if (isSupportingHtmlSource(fixture.path)) await verifySupportingHtmlSource(page, fixture, tag);
            if (fixture.path === '/packages/cem-elements/demo/module-url.html') {
                await verifyModuleUrlPresentation(page, resolutionRequests, `http://127.0.0.1:${port}/__cem-source-harness.html`);
                await verifySourceDocumentDiagnostics(page, tag, {
                    '4b. Missing import-map entry': ['cem-element.module_url_resolve_failed'],
                });
            }
            if (fixture.path === '/packages/cem-elements/demo/npm-versions-demo.html') {
                await verifyNpmVersionsPresentation(page);
                await verifySourceDocumentDiagnostics(page, tag);
                await verifyNpmVersionsLifecycle(page, async () => {
                    await mountSourceDocument(page, fixture, tag);
                    await verifySampleContractInventory(page, tag, fixture);
                });
                await verifySourceDocumentDiagnostics(page, tag);
            }
            if (fixture.path === '/packages/cem-elements/demo/scoped-css.html') {
                await verifyScopedCssPresentation(page);
                await verifyScopedCssDiagnostics(page, tag);
            }
            if (fixture.path === '/packages/cem-elements/demo/set-url.html') {
                await verifySetUrlLifecycle(page);
                await verifySetUrlPresentation(page);
                await verifySourceDocumentDiagnostics(page, tag);
            }
            if (fixture.path === '/packages/cem-elements/demo/table-inspector.html') {
                await verifyTableInspectorPresentation(page);
                await verifySourceDocumentDiagnostics(page, tag);
            }
            if (fixture.path === '/packages/cem-elements/demo/xpath-aggregates.html') {
                await verifyXPathAggregatesPresentation(page);
                await verifySourceDocumentDiagnostics(page, tag);
            }
            if (fixture.path === '/packages/cem-elements/demo/xpath-functions.html') {
                await verifyXPathFunctionsPresentation(page);
                await verifySourceDocumentDiagnostics(page, tag);
            }
            if (fixture.path === '/packages/cem-elements/demo/xpath-maps-arrays.html') {
                await verifyXPathMapsArraysPresentation(page);
                await verifySourceDocumentDiagnostics(page, tag);
            }
            if (fixture.path === '/packages/cem-elements/demo/xpath-nodes.html') {
                await verifyXPathNodesPresentation(page);
                await verifySourceDocumentDiagnostics(page, tag);
            }
            if (fixture.path === '/packages/cem-elements/demo/xpath-sequences.html') {
                await verifyXPathSequencesPresentation(page);
                await verifySourceDocumentDiagnostics(page, tag);
            }
            if (fixture.path === '/packages/cem-elements/demo/functions/str.html') {
                await verifyStringFunctionsPresentation(page);
                await verifySourceDocumentDiagnostics(page, tag);
            }
            if (fixture.path === '/packages/cem-elements/demo/functions/dom.html') {
                await verifyDomChainsPresentation(page);
                await verifySourceDocumentDiagnostics(page, tag);
            }
            if (fixture.path === '/packages/cem-elements/demo/xpath-validation.html') {
                await verifyXPathValidationPresentation(page);
                await verifySourceDocumentDiagnostics(page, tag);
            }
            if (fixture.path === '/packages/cem-elements/demo/xpath-sort.html') {
                await verifyXPathSortPresentation(page);
                await verifySourceDocumentDiagnostics(page, tag);
            }
            await verifySymbolicControls(page, fixture.path);
            if (fixture.path === '/packages/cem-elements/demo/module-url-referrer.html') {
                await verifyScalarReferrerPresentation(page, resolutionRequests,
                    `http://127.0.0.1:${port}/__cem-source-harness.html`);
            }
            if (fixture.path === '/packages/cem-elements/demo/location-element.html') {
                await verifyLocationLifecycle(page, async () => {
                    await mountSourceDocument(page, fixture, tag);
                    await verifySampleContractInventory(page, tag, fixture);
                });
            }
            if (fixture.path === '/packages/cem-elements/demo/hex-grid.html') {
                await verifyHexSamples(page);
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
        const editors = 'cem-demo-element[legend="7. Write a slice back to storage"]';
        for (const [writer, value] of [[otherTab, 'other tab'], [page, 'original tab'], [otherTab, '']]) {
            await writer.locator(`${editors} cem-storage-editor:first-of-type input`).fill(value);
            for (const reader of [page, otherTab]) {
                await runCheck(reader, scopeCheck(formState({ inputs: [value, value], outputs: [value, value] }), editors));
                await runCheck(reader, storageValue('cemDemoSliceEditor', value));
            }
        }
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

async function verifyLocationLifecycle(page, remountSource) {
    const live = 'cem-demo-element[legend="1. Window location live update"]';
    const initial = 'cem-demo-element[legend="2. Window location initial read"]';
    const external = 'cem-demo-element[legend="3. External URL from href"]';
    const readers = async () => {
        for (const [selector, mode] of [[live, 'live'], [initial, 'initial'], [external, 'external']]) {
            await runCheck(page, locationReader(selector, mode));
        }
    };
    await page.waitForURL(url => url.hash === '#after-initial-read');
    await readers();
    const beforeLink = page.url();
    const linked = new URL('#native-link', beforeLink).href;
    await page.locator(live).getByRole('link', { name: 'Native hash link', exact: true }).click();
    await page.waitForURL(linked);
    await readers();
    await page.goBack();
    await page.waitForURL(beforeLink);
    await readers();
    await page.goForward();
    await page.waitForURL(linked);
    await readers();

    const input = page.locator(`${live} input`);
    for (const [value, keyboard] of [['pear & cherry', false], ['', true]]) {
        const beforeDraft = page.url();
        await input.fill(value);
        await readers();
        if (page.url() !== beforeDraft) throw new Error('Typing a GET query navigated before submission');
        const expected = new URL(await page.locator(`${live} form`).evaluate(form => form.action));
        expected.search = new URLSearchParams({ query: value }).toString();
        await page.evaluate(() => { globalThis.__cemFixtureNavigationDocument = true; });
        await Promise.all([
            page.waitForURL(expected.href),
            keyboard ? input.press('Enter')
                : page.locator(live).getByRole('button', { name: 'Navigate with GET (reloads)', exact: true }).click(),
        ]);
        await page.waitForLoadState('networkidle');
        if (await page.evaluate(() => globalThis.__cemFixtureNavigationDocument === true)) {
            throw new Error('Native GET did not replace the document');
        }
        if (remountSource) await remountSource();
        await runCheck(page, { kind: 'locationSnapshot' });
        await readers();
        await runCheck(page, nodeTexts(`${live} li`, [`query = ${value}`.trim()]));
        // Reload restores the authored form default, while both readers capture the submitted URL.
        await runCheck(page, propertyEquals(`${live} input`, 'value', 'hello world'));
    }
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
        async ({ path, producedTag, attributes, declarationAttributes, content }) => {
            await customElements.whenDefined('cem-element');
            const declaration = document.createElement('cem-element');
            declaration.hidden = true;
            declaration.setAttribute('tag', producedTag);
            declaration.setAttribute('src', path);
            for (const [name, value] of Object.entries(declarationAttributes ?? {})) {
                declaration.setAttribute(name, value);
            }
            const instance = document.createElement(producedTag);
            for (const [name, value] of Object.entries(attributes ?? {})) {
                instance.setAttribute(name, value);
            }
            if (content) instance.textContent = content;
            document.body.append(declaration, instance);
        },
        { path: fixture.path, producedTag: tag, attributes: fixture.attributes,
            declarationAttributes: fixture.declarationAttributes, content: fixture.content },
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

async function verifyHexSamples(page) {
    const samples = [
        ['1. Responsive framework link honeycomb', ['DCE', 'React', 'AngularJS', 'Semantic UI', 'Open WC', 'Flutter',
            'Refine', 'Bootstrap', 'Vue.js', 'Lit', 'Redux', 'Svelte', 'SolidJS', 'Next.js']],
        ['2. Compact percentage links', ['DCE', 'React', 'Lit', 'AngularJS', 'Open WC', 'Flutter']],
        ['3. Fixed-length links', ['DCE', 'React', 'Lit', 'AngularJS', 'Open WC', 'Flutter']],
        ['4. Alternating backgrounds', ['DCE', 'React', 'Lit']],
        ['5. Wrapping long label', ['Declarative Custom Element Framework With A Long Name']],
        ['6. Missing-image fallback', ['Unavailable logo']],
        ['7. Wrapper DCE theme', ['DCE', 'React', 'Lit']],
        ['8. Image-button presentation', ['DCE']],
        [hexRowSample.legend, ['Loops', 'Hex grid', 'CSS']],
    ];
    for (const [index, [legend, labels]] of samples.entries()) {
        const selector = `cem-demo-element[legend="${legend}"]`;
        try {
            await poll(page, ({ selector, labels, index }) => {
                const sample = document.querySelector(selector);
                const links = Array.from(sample.querySelectorAll('.hex-link'));
                const images = Array.from(sample.querySelectorAll('.hex-logo'));
                return links.length === labels.length && images.length === labels.length
                    && links.every((link, i) => link.getAttribute('aria-label') === labels[i] && link.title === labels[i]
                        && link.textContent.replace(/\s+/gu, ' ').trim() === (index === 8 && i === 1 ? `✓ ${labels[i]}` : labels[i]))
                    && images.every((image, i) => image.alt === labels[i] && (image.src.startsWith('https://upload.wikimedia.org/')
                        || (image.complete && (index === 5 ? image.naturalWidth === 0 : image.naturalWidth > 0)
                            && image.classList.contains(index === 5 ? 'hex-logo-error' : 'hex-logo-load')
                            && getComputedStyle(image.previousElementSibling).visibility === (index === 5 ? 'visible' : 'hidden'))));
            }, { selector, labels, index });
            const sample = page.locator(selector);
            const links = sample.locator('.hex-link');
            const first = links.first();
            await page.mouse.move(0, 0);
            await page.evaluate(() => document.activeElement?.blur());
            await waitForHexLabel(page, `${selector} .hex-link`, index === 7);
            const resting = await first.evaluate(link => ({
                background: getComputedStyle(link).backgroundImage,
                filter: getComputedStyle(link).filter,
            }));
            const backgrounds = await links.evaluateAll(links => links.map(link => getComputedStyle(link).backgroundImage));
            if ((index === 0 && new Set(backgrounds).size !== 1) || (index === 3 && new Set(backgrounds).size !== 3)) {
                throw new Error(`unexpected background cycle: ${JSON.stringify(backgrounds)}`);
            }
            // Real pointer input exercises :hover independently of keyboard focus.
            await first.hover();
            await waitForHexLabel(page, `${selector} .hex-link`, true);
            await poll(page, ({ selector, index, resting }) => {
                const link = document.querySelector(`${selector} .hex-link`);
                const style = getComputedStyle(link);
                return link.matches(':hover') && style.filter !== resting.filter
                    && (index !== 6 || (style.backgroundImage.includes('rgb(0, 128, 0)')
                        && style.backgroundImage.includes('rgb(255, 255, 0)')))
                    && (index !== 7 || (style.filter.includes('drop-shadow') && style.filter.includes('saturate(1.15)')));
            }, { selector, index, resting });
            if (index === 6 && await links.nth(1).evaluate(link => getComputedStyle(link).backgroundImage) !== backgrounds[1]) {
                throw new Error('wrapper hover changed its sibling background');
            }
            await page.mouse.move(0, 0);
            await waitForHexLabel(page, `${selector} .hex-link`, index === 7);
            await first.focus();
            await page.keyboard.press('Tab');
            await page.keyboard.press('Shift+Tab');
            await waitForHexLabel(page, `${selector} .hex-link`, true);
            await poll(page, selector => {
                const link = document.querySelector(`${selector} .hex-link`);
                return document.activeElement === link && getComputedStyle(link, '::after').opacity === '1';
            }, selector);
            if (index === 4 && !await first.locator('.hex-label').evaluate(label => {
                const range = document.createRange();
                range.selectNodeContents(label);
                return range.getClientRects().length > 1;
            })) throw new Error('long label does not wrap');
            await first.evaluate(link => link.blur());
            await waitForHexLabel(page, `${selector} .hex-link`, index === 7);
            await poll(page, ({ selector, resting }) => {
                const style = getComputedStyle(document.querySelector(`${selector} .hex-link`));
                return style.backgroundImage === resting.background && style.filter === resting.filter;
            }, { selector, resting });
        } catch (error) {
            throw new Error(`${legend}: ${error.message}`, { cause: error });
        }
    }
}

async function waitForHexLabel(page, selector, raised) {
    await poll(page, ({ selector, raised }) => {
        const link = document.querySelector(selector);
        const label = link.querySelector('.hex-label');
        const box = link.getBoundingClientRect();
        const text = label.getBoundingClientRect();
        return raised ? label.scrollWidth <= label.clientWidth + 1 && text.left >= box.left
            && text.right <= box.right && text.top >= box.top && text.bottom <= box.bottom - box.height * 0.1
            : text.top >= box.bottom;
    }, { selector, raised });
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

function observeModuleUrlRequests(page, fixture) {
    if (!['/packages/cem-elements/demo/module-url-referrer.html', '/packages/cem-elements/demo/module-url.html'].includes(fixture.path)) return null;
    const requests = [];
    page.on('request', request => requests.push(request.url()));
    return requests;
}

async function verifyNpmVersionsPresentation(page) {
    const selector = 'cem-demo-element[legend]';
    await poll(page, () => {
        const selects = Array.from(document.querySelectorAll('cem-demo-element select'));
        return selects.length === 5 && selects.every(select => select.labels.length === 1
            && select.labels[0] === select.closest('label') && select.name === 'version')
            && JSON.stringify(selects.slice(0, 4).map(select => select.value))
                === JSON.stringify(['0.1.0', '0.0.22', '0.0.25', '0.0.21']);
    });
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, ({ selector, width }) => {
            const cards = Array.from(document.querySelectorAll(selector));
            return cards.length === 6 && document.documentElement.scrollWidth <= width
                && cards.every(card => {
                    const box = card.getBoundingClientRect();
                    return box.left >= 0 && box.right <= width
                        && Array.from(card.querySelectorAll('[slot="demo"], article, label, select, button'))
                            .every(node => {
                                const rect = node.getBoundingClientRect();
                                return node.scrollWidth <= node.clientWidth + 1
                                    && rect.left >= box.left && rect.right <= box.right;
                            });
                });
        }, { selector, width });
        const sourceReachable = await page.locator(`${selector} [slot="text"] pre`).evaluateAll(sources =>
            sources.length === 6 && sources.every(pre => {
                const max = pre.scrollWidth - pre.clientWidth;
                pre.scrollLeft = max;
                const reached = Math.abs(pre.scrollLeft - max) <= 1;
                pre.scrollLeft = 0;
                return reached;
            }));
        if (!sourceReachable) throw new Error('NPM source text cannot be scrolled to its end');
    }
}

async function verifyNpmVersionsLifecycle(page, remount) {
    const sample = `cem-demo-element[legend="${npmVersionSamples[4].legend}"]`;
    const expectUrl = async (hash, version, selection) => {
        try {
            await poll(page, ({ sample, hash, version, selection }) => {
                const root = document.querySelector(sample);
                return location.hash === hash && root?.querySelector('select')?.value === version
                    && (root.querySelector('cem-npm-version-url')?.getAttribute('currentversion') ?? '') === (hash ? version : '')
                    && (root.querySelector('select')?.getAttribute('value') ?? '') === (hash ? version : '')
                    && root.querySelector('cem-npm-version-url')?.getAttribute('value') === selection
                    && JSON.stringify(Array.from(root.querySelectorAll('output'), output => output.textContent.trim()))
                        === JSON.stringify([hash, selection]);
            }, { sample, hash, version, selection });
        } catch (error) {
            const actual = await page.locator(sample).evaluate(root => ({
                hash: location.hash,
                version: root.querySelector('select')?.value,
                reflected: root.querySelector('cem-npm-version-url')?.getAttribute('value'),
                currentversion: root.querySelector('cem-npm-version-url')?.getAttribute('currentversion'),
                defaults: Array.from(root.querySelectorAll('option[selected]'), option => option.value),
                outputs: Array.from(root.querySelectorAll('output'), output => output.textContent.trim()),
            }));
            throw new Error(`Expected NPM URL state ${JSON.stringify({ hash, version, selection })}; got ${JSON.stringify(actual)}`, { cause: error });
        }
    };
    // Return to the preceding URL without waiting for the child's intermediate
    // selection render; explicit select[value] must survive a superseded plan.
    await page.locator(`${sample} button[aria-label="Set URL to 0.0.25"]`).press('Enter');
    await expectUrl('#version=0.0.25', '0.0.25', '0.1.0');
    await page.goBack();
    await expectUrl('#version=0.1.0', '0.1.0', '0.1.0');
    await page.goForward();
    await expectUrl('#version=0.0.25', '0.0.25', '0.1.0');
    await page.locator(`${sample} button[aria-label="Clear URL version"]`).press('Space');
    await expectUrl('', '0.1.0', '0.1.0');
    await page.goBack();
    await expectUrl('#version=0.0.25', '0.0.25', '0.1.0');
    // A fresh page reads a deep link before any selection event has occurred.
    await page.reload({ waitUntil: 'networkidle' });
    if (remount) await remount();
    await expectUrl('#version=0.0.25', '0.0.25', '');
    await page.locator(`${sample} select`).selectOption('0.0.21');
    await expectUrl('#version=0.0.21', '0.0.21', '0.0.21');
    await page.locator(`${sample} button[aria-label="Clear URL version"]`).click();
    await expectUrl('', '0.1.0', '0.0.21');
}

async function verifyModuleUrlPresentation(page, requests, originalUrl) {
    const samples = 'cem-demo-element[legend]';
    const summaries = page.locator(`${samples} expando-link summary`);
    // Inspect complete URLs as well as their shortened summaries.
    for (let index = 0; index < await summaries.count(); index++) await summaries.nth(index).press('Enter');
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, ({ samples, width }) => {
            const cards = Array.from(document.querySelectorAll(samples));
            const images = cards.flatMap(card => Array.from(card.querySelectorAll('[slot="demo"] img')));
            return cards.length === 13 && images.length === 10
                && images.every(image => image.complete && image.naturalWidth > 0 && image.alt.trim())
                && document.documentElement.scrollWidth <= width
                && cards.every(card => {
                    const box = card.getBoundingClientRect();
                    return box.left >= 0 && box.right <= width
                        && Array.from(card.querySelectorAll('[slot="demo"], [slot="demo"] pre, [slot="demo"] table, [slot="demo"] th, [slot="demo"] td, [slot="demo"] figure, [slot="demo"] details'))
                            .every(node => {
                                const rect = node.getBoundingClientRect();
                                return node.scrollWidth <= node.clientWidth + 1
                                    && rect.left >= box.left && rect.right <= box.right;
                            });
                });
        }, { samples, width });
        const sourcesReachable = await page.locator(`${samples} [slot="text"] pre`).evaluateAll(sources =>
            sources.length === 13 && sources.every(pre => {
                const max = pre.scrollWidth - pre.clientWidth;
                pre.scrollLeft = max;
                const reachable = Math.abs(pre.scrollLeft - max) <= 1;
                pre.scrollLeft = 0;
                return reachable;
            }));
        if (!sourcesReachable) throw new Error('Module-URL source text cannot be scrolled to its end');
    }
    for (let index = 0; index < await summaries.count(); index++) await summaries.nth(index).press('Space');
    if (new URL(originalUrl).pathname !== '/__cem-source-harness.html') await verifyDemoLayout(page, 12);
    if (page.url() !== originalUrl) throw new Error('Module URL resolution or disclosure toggling navigated the page');
    const unexpected = requests.filter(request => {
        const url = new URL(request);
        return url.hostname.endsWith('.example.test') || url.pathname === '/lib-dir/Smiley.svg'
            || url.pathname.includes('/missing-demo-lib/');
    });
    if (unexpected.length) throw new Error(`Unexpected module URL requests: ${unexpected.join(', ')}`);
}

async function verifySourceDocumentDiagnostics(page, tag, expectedByLegend = {}) {
    await poll(page, ({ tag, expectedByLegend }) => {
        const runtime = window.__cemFixtureRuntime;
        const host = document.querySelector(tag);
        const declaration = document.querySelector(`cem-element[tag="${tag}"]`);
        if (!host || !declaration || !runtime || runtime.diagnosticsFor(host).length
            || runtime.diagnosticsFor(declaration).length) return false;
        return Array.from(host.querySelectorAll('cem-demo-element[legend]')).every(sample => {
            const codes = new Set();
            for (const declaration of sample.querySelectorAll('cem-element[tag]')) {
                if (runtime.diagnosticsFor(declaration).length) return false;
                for (const instance of sample.querySelectorAll(declaration.getAttribute('tag'))) {
                    for (const diagnostic of runtime.diagnosticsFor(instance)) codes.add(diagnostic.code);
                }
            }
            const expected = expectedByLegend[sample.getAttribute('legend')] ?? [];
            return JSON.stringify([...codes]) === JSON.stringify(expected);
        });
    }, { tag, expectedByLegend });
}

async function verifyScalarReferrerPresentation(page, requests, originalUrl) {
    const selector = `cem-demo-element[legend="${scalarReferrerSample.legend}"]`;
    const expectedControls = [
        ['Relative', './relative-referrer/component.js'],
        ['Module', 'demo-module-referrer'],
        ['Absolute', 'https://referrer.example.test/absolute/component.js'],
    ].flatMap(([suffix, referrer]) => [
        ['relative', `../lib-dir/Smiley.svg?case=relative-${suffix.toLowerCase()}`],
        ['module', 'demo-referrer-image'],
        ['absolute', 'https://assets.example.test/logo.svg'],
    ].map(([prefix, src]) => `@slice=${prefix}By${suffix} @src="${src}" @referrer="${referrer}"`));
    await poll(page, ({ selector, expectedControls }) => {
        const sample = document.querySelector(selector);
        const source = sample?.querySelector('[slot="text"] pre')?.textContent ?? '';
        const controls = [...source.matchAll(/\{cem-module-url\s+([^}]+)\}/gu)]
            .map(match => match[1].replace(/\s+/gu, ' ').trim());
        return source.replace(/^\r?\n/u, '').startsWith('<cem-element>')
            && JSON.stringify(controls) === JSON.stringify(expectedControls)
            && expectedControls.every(control => source.includes(`{$datadom.slices.${control.split(' ')[0].slice(7)}}`))
            && !(sample?.querySelector('[slot="status"]')?.textContent ?? '').trim();
    }, { selector, expectedControls });
    await runCheck(page, urlEquals('main section a[href$="module-url.html"]', 'href',
        '/packages/cem-elements/demo/module-url.html'));
    await runCheck(page, urlEquals('nav a', 'href', '/packages/cem-elements/index.html'));
    // A dedicated wide matrix has one card. Verify the output itself, since the
    // shared card's overflow:hidden can hide a table without widening the page.
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, ({ selector, width }) => {
            const sample = document.querySelector(selector);
            if (!sample || document.documentElement.scrollWidth > width) return false;
            const card = sample.getBoundingClientRect();
            const output = sample.querySelector('[slot="demo"]');
            const table = output?.querySelector('table');
            if (!output || !table) return false;
            const boxes = [sample, output, table, ...table.querySelectorAll('th, td')];
            return card.left >= 0 && card.right <= width
                && boxes.every(element => {
                    const rect = element.getBoundingClientRect();
                    return element.scrollWidth <= element.clientWidth + 1
                        && rect.left >= card.left && rect.right <= card.right;
                });
        }, { selector, width });
        // Long source lines remain reachable in the source panel's own scroller.
        const source = page.locator(`${selector} [slot="text"] pre`);
        const reachable = await source.evaluate(pre => {
            const max = pre.scrollWidth - pre.clientWidth;
            pre.scrollLeft = max;
            const reached = Math.abs(pre.scrollLeft - max) <= 1;
            pre.scrollLeft = 0;
            return reached;
        });
        if (!reachable) throw new Error('Matrix source text cannot be scrolled to its end');
    }
    if (page.url() !== originalUrl) throw new Error('Scalar URL resolution navigated the page');
    const fetched = requests.filter(request => {
        const url = new URL(request);
        return url.hostname.endsWith('.example.test') || url.pathname.endsWith('.svg')
            || url.pathname.endsWith('/component.js');
    });
    if (fetched.length > 0) throw new Error(`Scalar URL resolution fetched a target or referrer: ${fetched.join(', ')}`);
}

async function verifyDemoLayout(page, expectedCount) {
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, ({ width, expectedCount }) => {
            const cards = Array.from(document.querySelectorAll('main > cem-demo-element'),
                card => card.getBoundingClientRect());
            return cards.length === expectedCount
                && document.documentElement.scrollWidth <= width
                && cards.every(card => card.left >= 0 && card.right <= width)
                && (width < 1000 || (Math.abs(cards[0].top - cards[1].top) < 1 && cards[0].left !== cards[1].left));
        }, { width, expectedCount });
    }
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
            case 'locationSnapshot':
                await page.evaluate(() => { globalThis.__cemFixtureInitialLocationHref = location.href; });
                return;
            case 'locationReader':
                await poll(page, ({ selector, mode }) => {
                    const root = document.querySelector(selector);
                    if (!root) return false;
                    const url = new URL(mode === 'external' ? 'https://my.example/docs?a=1&b=2&b=3#details'
                        : mode === 'initial' ? globalThis.__cemFixtureInitialLocationHref : location.href);
                    const expected = mode === 'initial'
                        ? [['source', 'window'], ['origin', url.origin], ['pathname', url.pathname], ['search', url.search], ['hash', url.hash]]
                        : mode === 'external' ? [['href', url.href], ['hostname', url.hostname], ['pathname', url.pathname], ['hash', url.hash]]
                            : [['href', url.href], ['pathname', url.pathname], ['hash', url.hash]];
                    const normalize = value => (value ?? '').replace(/\s+/gu, ' ').trim();
                    const entries = Array.from(root.querySelectorAll('dt'), dt => [normalize(dt.textContent), normalize(dt.nextElementSibling?.textContent)]);
                    const rows = Array.from(root.querySelectorAll('li'), li => normalize(li.textContent));
                    const params = mode === 'initial' ? [] : [...new Set(url.searchParams.keys())]
                        .map(name => normalize(`${name} = ${url.searchParams.getAll(name).join(',')}`));
                    return JSON.stringify(entries) === JSON.stringify(expected) && JSON.stringify(rows) === JSON.stringify(params);
                }, check);
                return;
            case 'storageValue':
                await poll(page, ({ key, expected }) => localStorage.getItem(key) === expected, check);
                return;
            case 'text':
                await waitForText(page, check.selector, check.expected);
                return;
            case 'resolvedUrlTexts':
                await poll(page, ({ selector, expected }) => {
                    const actual = Array.from(document.querySelectorAll(selector), element =>
                        (element.textContent ?? '').replace(/\s+/gu, ' ').trim());
                    return JSON.stringify(actual) === JSON.stringify(expected.map(value => new URL(value, location.href).href));
                }, check);
                return;
            case 'nodeTexts':
                await poll(page, ({ selector, expected }) => {
                    const actual = Array.from(document.querySelectorAll(selector), element =>
                        (element.textContent ?? '').replace(/\s+/gu, ' ').trim());
                    return JSON.stringify(actual) === JSON.stringify(expected);
                }, check);
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
            case 'focusedElement':
                await poll(page, ({ selector }) => {
                    const element = document.querySelector(selector);
                    return element?.isConnected && document.activeElement === element;
                }, check);
                return;
            case 'elementIdentity':
                if (check.action === 'remember') {
                    await waitForExactCount(page, check.selector, 1);
                    await page.evaluate(({ selector }) => {
                        globalThis.__cemFixtureElements ??= new Map();
                        globalThis.__cemFixtureElements.set(selector, document.querySelector(selector));
                    }, check);
                } else {
                    await poll(page, ({ selector }) => {
                        const remembered = globalThis.__cemFixtureElements?.get(selector);
                        return remembered?.isConnected && document.querySelector(selector) === remembered;
                    }, check);
                }
                return;
            case 'treeSelection':
                await poll(page, ({ selector, selected, branches }) => {
                    const root = document.querySelector(selector);
                    if (!root) return false;
                    const inputs = Array.from(root.querySelectorAll('input[type=checkbox]'));
                    const label = input => input.getAttribute('aria-label')?.replace(/^Select branch /u, '');
                    return (!branches || JSON.stringify(inputs.map(label)) === JSON.stringify(branches))
                        && JSON.stringify(inputs.filter(input => input.checked).map(label)) === JSON.stringify(selected)
                        && root.querySelector('output[aria-label="Selected branches"]')?.textContent === String(selected.length)
                        && root.querySelectorAll('strong').length === selected.length
                        && inputs.every(input => {
                            const li = input.closest('li');
                            return li?.classList.contains('selected') === input.checked
                                && (li?.querySelector(':scope > label > strong')?.textContent ?? '') === (input.checked ? 'Selected' : '');
                        });
                }, check);
                return;
            case 'editControl':
                await page.waitForSelector(check.selector, { timeout });
                await page.evaluate(({ selector, value, eventName, selection, replacement }) => {
                    const control = document.querySelector(selector);
                    const revision = control.closest('article')?.getAttribute('data-cem-data-revision');
                    if (!revision) throw new Error('Counter render revision is missing');
                    globalThis.__cemFixtureCounterRevisions ??= new Map();
                    globalThis.__cemFixtureCounterRevisions.set(selector, revision);
                    control.focus();
                    if (replacement) {
                        const [start, end, text] = replacement;
                        control.setSelectionRange(start, end);
                        control.setRangeText(text, start, end, 'end');
                    } else {
                        control.value = value;
                        control.setSelectionRange(...selection);
                    }
                    control.dispatchEvent(new Event(eventName, { bubbles: true }));
                }, check);
                return;
            case 'counterState':
                await poll(page, ({ selector, value, counts, output, focused, selection }) => {
                    const control = document.querySelector(selector);
                    const article = control?.closest('article');
                    if (!article || !control.isConnected || globalThis.__cemFixtureElements?.get(selector) !== control) return false;
                    const before = globalThis.__cemFixtureCounterRevisions?.get(selector);
                    // Even equal counts must be checked after the edit's projection.
                    if (before !== undefined && article.getAttribute('data-cem-data-revision') === before) return false;
                    return control.value === value
                        && JSON.stringify(Array.from(article.querySelectorAll('strong'), node => node.textContent.trim())) === JSON.stringify(counts)
                        && (output === undefined || article.querySelector('output')?.textContent === output)
                        && (focused === undefined || (document.activeElement === control) === focused)
                        && (!selection || (control.selectionStart === selection[0] && control.selectionEnd === selection[1]
                            && (!selection[2] || control.selectionDirection === selection[2])));
                }, check);
                return;
            case 'inspectorTable':
                await poll(page, ({ selector, cells, selected, headings, sort, mode }) => {
                    const table = document.querySelector(selector);
                    if (!table) return false;
                    const ownRows = Array.from(table.querySelectorAll(':scope > tbody > tr'));
                    const text = node => (node.textContent ?? '').trim();
                    const headers = Array.from(table.querySelectorAll(':scope > thead > tr > th'));
                    const active = headers.filter(header => header.hasAttribute('aria-sort'));
                    return ownRows.length === cells.length && ownRows.every((row, index) => {
                        const actual = Array.from(row.cells).slice(1).map(text);
                        return actual.length === cells[index].length
                            && cells[index].every((value, column) => value === null || actual[column] === value)
                            && row.querySelector(':scope > th input')?.checked === selected.includes(index)
                            && row.getAttribute('aria-selected') === String(selected.includes(index));
                    }) && table.querySelector(':scope > caption > output')?.textContent.trim() === String(selected.length)
                        && (!headings || JSON.stringify(headers.map(header => text(header).replace(/[↑↓]/gu, '').trim())) === JSON.stringify(headings))
                        && (sort === undefined || (sort === null ? active.length === 0
                            : active.length === 1 && active[0].getAttribute('aria-sort') === sort))
                        && (mode === undefined || table.querySelector(':scope > caption select')?.value === mode);
                }, check);
                return;
            case 'tableState':
                await poll(page, ({ selector, cells, selected }) => {
                    const table = document.querySelector(selector);
                    if (!table) return false;
                    const rows = Array.from(table.querySelectorAll(':scope > tbody > tr'));
                    return rows.length === cells.length && rows.every((row, index) => {
                        const actual = Array.from(row.querySelectorAll(':scope > td'), cell =>
                            (cell.textContent ?? '').replace(/\s+/gu, ' ').trim());
                        const pressed = String(selected.includes(index));
                        return actual.length === cells[index].length
                            && cells[index].every((value, column) => value === null || actual[column] === value)
                            && row.getAttribute('aria-selected') === pressed
                            && row.querySelector(':scope > th > button')?.getAttribute('aria-pressed') === pressed;
                    });
                }, check);
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
                        ...(expected.formData ? { formData: Array.from(new FormData(root.querySelector('form')).entries()) } : {}),
                    };
                    return Object.entries(expected).every(([key, values]) =>
                        JSON.stringify(actual[key]) === JSON.stringify(values));
                }, check);
                return;
            case 'nativeValidity':
                await poll(page, ({ selector, expected, resultSelector }) => {
                    const input = document.querySelector(selector);
                    return input && Object.entries(expected).every(([key, value]) => input.validity[key] === value)
                        && document.querySelector(resultSelector)?.textContent?.trim() === input.validationMessage;
                }, check);
                return;
            case 'submitForm': {
                const root = page.locator(check.selector);
                await root.evaluate(element => {
                    const capture = { events: [], listener: null };
                    capture.listener = event => {
                        capture.events.push(event.defaultPrevented);
                        event.preventDefault(); // Keep valid demo GET submissions on this test page.
                    };
                    element.__cemFixtureSubmit = capture;
                    element.addEventListener('submit', capture.listener);
                });
                try {
                    await page.locator(check.actionSelector).click();
                    const actual = await root.evaluate(element => element.__cemFixtureSubmit.events);
                    const expected = check.expected === 'blocked' ? [] : [check.expected === 'cancelled'];
                    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
                        throw new Error(`Expected ${check.expected} submission, received cancellation flags ${JSON.stringify(actual)}`);
                    }
                } finally {
                    await root.evaluate(element => {
                        element.removeEventListener('submit', element.__cemFixtureSubmit.listener);
                        delete element.__cemFixtureSubmit;
                    });
                }
                return;
            }
            case 'urlEquals':
                await poll(page, ({ selector, name, expected }) =>
                    document.querySelector(selector)?.[name] === new URL(expected, location.href).href, check);
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

function nodeTexts(selector, expected) {
    return { kind: 'nodeTexts', selector, expected };
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

function focusedElement(selector) {
    return { kind: 'focusedElement', selector };
}

function elementIdentity(selector, action) {
    return { kind: 'elementIdentity', selector, action };
}

function counterState(value, counts, options = {}) {
    return { kind: 'counterState', selector: counterControl, value, counts, ...options };
}

function editControl(value, eventName = 'input') {
    return { kind: 'editControl', selector: counterControl, value, eventName, selection: counterSelection(value) };
}

function replaceControlText(start, end, text) {
    return { kind: 'editControl', selector: counterControl, eventName: 'input', replacement: [start, end, text] };
}

function treeSelection(selected, branches) {
    return { kind: 'treeSelection', selector: ':scope', selected, branches };
}

// null skips a nested cell whose own table has a separate contract.
function inspectorTable(selector, cells, selected = [], options = {}) {
    return { kind: 'inspectorTable', selector, cells, selected, ...options };
}

function tableState(selector, cells, selected = []) {
    return { kind: 'tableState', selector, cells, selected };
}

function formState(expected) {
    return { kind: 'formState', selector: ':scope', expected };
}

function nativeValidity(selector, expected, resultSelector) {
    return { kind: 'nativeValidity', selector, expected, resultSelector };
}

function submitForm(actionSelector, expected) {
    return { kind: 'submitForm', selector: ':scope', actionSelector, expected };
}

function urlEquals(selector, name, expected) {
    return { kind: 'urlEquals', selector, name, expected };
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
        case 'locationSnapshot':
            return 'capture the initial window URL';
        case 'locationReader':
            return `locationReader(${check.selector}, ${check.mode})`;
        case 'storageValue':
            return `storageValue(${check.key}, ${JSON.stringify(check.expected)})`;
        case 'text':
        case 'normalizedText':
        case 'resolvedUrlTexts':
        case 'nodeTexts':
            return `${check.kind}(${check.selector}, ${JSON.stringify(check.expected)})`;
        case 'countAtLeast':
            return `countAtLeast(${check.selector}, ${check.min})`;
        case 'countExactly':
            return `countExactly(${check.selector}, ${check.count})`;
        case 'attributeContains':
            return `attributeContains(${check.selector}, ${check.name}, ${JSON.stringify(check.expected)})`;
        case 'attributeEquals':
            return `attributeEquals(${check.selector}, ${check.name}, ${JSON.stringify(check.expected)})`;
        case 'urlEquals':
            return `urlEquals(${check.selector}, ${check.name}, ${JSON.stringify(check.expected)})`;
        case 'propertyEquals':
            return `propertyEquals(${check.selector}, ${check.name}, ${JSON.stringify(check.expected)})`;
        case 'focusedElement':
            return `focusedElement(${check.selector})`;
        case 'elementIdentity':
            return `elementIdentity(${check.selector}, ${check.action})`;
        case 'editControl':
            return `editControl(${check.selector}, ${JSON.stringify(check.value ?? check.replacement)}, ${check.eventName})`;
        case 'counterState':
            return `counterState(${check.selector}, ${JSON.stringify(check.value)}, counts=${JSON.stringify(check.counts)}, selection=${JSON.stringify(check.selection)})`;
        case 'treeSelection':
            return `treeSelection(${check.selector}, selected=${JSON.stringify(check.selected)}, branches=${JSON.stringify(check.branches)})`;
        case 'inspectorTable':
        case 'tableState':
            return `${check.kind}(${check.selector}, ${JSON.stringify(check.cells)}, selected=${JSON.stringify(check.selected)})`;
        case 'formState':
            return `formState(${check.selector}, ${JSON.stringify(check.expected)})`;
        case 'nativeValidity':
            return `nativeValidity(${check.selector}, ${JSON.stringify(check.expected)})`;
        case 'submitForm':
            return `submitForm(${check.actionSelector}, ${check.expected})`;
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

async function verifyScopedCssPresentation(page) {
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, width => {
            const cards = Array.from(document.querySelectorAll('cem-demo-element[legend]'));
            return cards.length === 12 && document.documentElement.scrollWidth <= width
                && cards.every(card => {
                    const box = card.getBoundingClientRect();
                    const output = card.querySelector('[slot="demo"]');
                    return box.left >= 0 && box.right <= width && output
                        && output.scrollWidth <= output.clientWidth + 1;
                });
        }, width);
        const reachable = await page.locator('cem-demo-element [slot="text"] pre').evaluateAll(sources =>
            sources.length === 12 && sources.every(pre => {
                const end = pre.scrollWidth - pre.clientWidth;
                pre.scrollLeft = end;
                const reached = Math.abs(pre.scrollLeft - end) <= 1;
                pre.scrollLeft = 0;
                return reached;
            }));
        if (!reachable) throw new Error('Scoped CSS source cannot be scrolled to its end');
    }
    if (new URL(page.url()).pathname !== '/__cem-source-harness.html') await verifyDemoLayout(page, 12);
}

async function verifyScopedCssDiagnostics(page, tag) {
    await page.evaluate(async tag => {
        const runtime = window.__cemFixtureRuntime;
        const host = document.querySelector(tag);
        const loader = document.querySelector(`cem-element[tag="${tag}"]`);
        await runtime.whenRenderSettled(host);
        const declarations = Array.from(host.querySelectorAll('cem-element'));
        for (const declaration of declarations) await runtime.whenDeclarationSettled(declaration);
        const check = (element, expected) => {
            const actual = runtime.diagnosticsFor(element).map(d => d.code);
            if (JSON.stringify(actual) !== JSON.stringify(expected))
                throw new Error(`${element.localName}[${element.getAttribute('tag') ?? ''}]: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
        };
        check(host, []);
        check(loader, []);
        for (const declaration of declarations) {
            const producedTag = declaration.getAttribute('tag');
            const expected = producedTag === 'cem-css-invalid-declaration' ? ['cem-element.stylesheet_scope_invalid']
                : ['cem-css-unscoped-explicit', 'cem-css-mismatch', 'cem-css-mismatch-bare'].includes(producedTag)
                    ? ['cem-element.stylesheet_scope_mismatch']
                    : producedTag === 'cem-css-dynamic' ? Array(2).fill('cem.ql.template.stylesheet_dynamic_unsupported') : [];
            check(declaration, expected);
            for (const instance of host.querySelectorAll(producedTag)) {
                await runtime.whenRenderSettled(instance);
                check(instance, producedTag === 'cem-css-dynamic' ? expected : []);
            }
        }
    }, tag);
}

async function verifySetUrlLifecycle(page) {
    const sample = index => `cem-demo-element[legend="${setUrlSamples[index].legend}"]`;
    const hashButton = hash => page.locator(`${sample(0)} button[value="${hash}"]`);
    const methodButton = method => page.locator(`${sample(1)} button[value="${method}"]`);
    const form = page.locator(sample(3));
    const expectHash = async hash => {
        await poll(page, hash => location.hash === hash
            && Array.from(document.querySelectorAll('cem-demo-element[legend]')).every(card =>
                Array.from(card.querySelectorAll('output')).at(-1)?.textContent === hash), hash);
    };
    const historyLength = () => page.evaluate(() => history.length);
    const command = async (button, method, hash) => {
        const before = await historyLength();
        await button.click();
        await expectHash(hash);
        const expected = before + (['location.replace', 'history.replaceState'].includes(method) ? 0 : 1);
        if (await historyLength() !== expected) throw new Error(`${method} changed the history length incorrectly`);
        await button.click();
        // An unchanged output cannot acknowledge a no-op command. Observe a
        // quiet interval, then verify the fresh trigger after external navigation.
        await page.waitForTimeout(250);
        await expectHash(hash);
        if (await historyLength() !== expected) throw new Error(`${method} duplicated an equal-URL history entry`);
    };
    if (new URL(page.url()).hash !== '') throw new Error('Set-URL writers navigated before an event');
    for (const hash of ['#hash-one', '#hash-two']) {
        await command(hashButton(hash), 'location.hash', hash);
        await runCheck(page, nodeTexts(`${sample(0)} output`, [hash, hash]));
    }
    for (const method of setUrlMethods) {
        await command(methodButton(method), method, `#${method}`);
        await runCheck(page, nodeTexts(`${sample(1)} output`, [method, `#${method}`]));
    }
    await page.locator(`${sample(2)} button`).press('Enter');
    await expectHash('#conditional-writer');
    await runCheck(page, elementIdentity(`${sample(3)} input[type="text"]`, 'remember'));
    for (const method of setUrlMethods) {
        const previousHash = new URL(page.url()).hash;
        const previousLength = await historyLength();
        await form.locator(`input[value="${method}"]`).check();
        await form.locator('input[type="text"]').fill(`#form-${method}`);
        await runCheck(page, nodeTexts(`${sample(3)} output`, [method, `#form-${method}`, previousHash]));
        if (new URL(page.url()).hash !== previousHash || await historyLength() !== previousLength)
            throw new Error(`${method} applied a pending form draft`);
        await runCheck(page, countExactly(`${sample(3)} input:checked`, 1));
        await runCheck(page, propertyEquals(`${sample(3)} input:checked`, 'value', method));
        await command(form.locator('button'), method, `#form-${method}`);
        await runCheck(page, elementIdentity(`${sample(3)} input[type="text"]`, 'same'));
        await runCheck(page, propertyEquals(`${sample(3)} input[type="text"]`, 'value', `#form-${method}`));
    }
    await page.evaluate(() => history.replaceState({}, '', '#external-change'));
    await expectHash('#external-change');
    await form.locator('button').press('Space');
    await expectHash('#form-history.replaceState');
    // Replacing the second hash entry must make Back skip that replaced URL.
    await hashButton('#hash-one').click();
    await expectHash('#hash-one');
    await hashButton('#hash-two').click();
    await expectHash('#hash-two');
    await methodButton('history.replaceState').click();
    await expectHash('#history.replaceState');
    await page.goBack();
    await expectHash('#hash-one');
    await page.goForward();
    await expectHash('#history.replaceState');
    await runCheck(page, nodeTexts(`${sample(0)} output`, ['#hash-two', '#history.replaceState']));
    await runCheck(page, nodeTexts(`${sample(3)} output`, ['history.replaceState', '#form-history.replaceState', '#history.replaceState']));
}

async function verifySetUrlPresentation(page) {
    for (const [selector, target] of [
        ['nav a', '/packages/cem-elements/index.html'],
        ['main > section a[href$="location-element.html"]', '/packages/cem-elements/demo/location-element.html'],
        ['main > section a[href$="data-slices.html"]', '/packages/cem-elements/demo/data-slices.html'],
    ]) await runCheck(page, urlEquals(selector, 'href', target));
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, width => {
            const cards = Array.from(document.querySelectorAll('cem-demo-element[legend]'));
            return cards.length === 4 && document.documentElement.scrollWidth <= width
                && cards.every(card => {
                    const box = card.getBoundingClientRect();
                    return box.left >= 0 && box.right <= width
                        && Array.from(card.querySelectorAll('[slot="demo"], article, form, fieldset, button, input')).every(node => {
                            const rect = node.getBoundingClientRect();
                            return rect.left >= box.left && rect.right <= box.right
                                && (node.localName === 'input' || node.scrollWidth <= node.clientWidth + 1);
                        });
                });
        }, width);
        const reachable = await page.locator('cem-demo-element [slot="text"] pre').evaluateAll(sources =>
            sources.length === 4 && sources.every(pre => {
                const end = pre.scrollWidth - pre.clientWidth;
                pre.scrollLeft = end;
                const reached = Math.abs(pre.scrollLeft - end) <= 1;
                pre.scrollLeft = 0;
                return reached;
            }));
        if (!reachable) throw new Error('Set-URL source cannot be scrolled to its end');
    }
    if (new URL(page.url()).pathname !== '/__cem-source-harness.html') await verifyDemoLayout(page, 4);
}

async function verifyTableInspectorPresentation(page) {
    await runCheck(page, countExactly('nav a, main > section a', 5));
    await runCheck(page, urlEquals('nav a', 'href', '/packages/cem-elements/index.html'));
    for (const target of ['data-table-view.cemt', 'data-table.html', 'data-tree.html', 'xpath-sort.html']) {
        await runCheck(page, urlEquals(`main > section a[href$="${target}"]`, 'href', `/packages/cem-elements/demo/${target}`));
    }
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, width => {
            const cards = Array.from(document.querySelectorAll('cem-demo-element[legend]'));
            return cards.length === 4 && document.documentElement.scrollWidth <= width
                && cards.every(card => {
                    const box = card.getBoundingClientRect();
                    return box.left >= 0 && box.right <= width
                        && Array.from(card.querySelectorAll('[slot="demo"], article, textarea, .table-scroll')).every(node => {
                            const rect = node.getBoundingClientRect();
                            return rect.left >= box.left && rect.right <= box.right
                                && (node.classList.contains('table-scroll') || node.scrollWidth <= node.clientWidth + 1);
                        });
                });
        }, width);
        const reachable = await page.locator('cem-demo-element [slot="text"] pre, cem-table-inspector .table-scroll').evaluateAll(regions =>
            regions.length === 10 && regions.every(region => {
                const end = region.scrollWidth - region.clientWidth;
                region.scrollLeft = end;
                const reached = Math.abs(region.scrollLeft - end) <= 1;
                region.scrollLeft = 0;
                return reached;
            }));
        if (!reachable) throw new Error('Table-inspector source/table content cannot be scrolled to its end');
    }
    if (new URL(page.url()).pathname !== '/__cem-source-harness.html') await verifyDemoLayout(page, 4);
}

async function verifyXPathAggregatesPresentation(page) {
    await runCheck(page, nodeTexts('cem-demo-element[legend="1. Decimal sequence statistics"] output', ['0', '∅', '∅', '∅']));
    await runCheck(page, countExactly('nav a, main > section a', 6));
    await runCheck(page, urlEquals('nav a', 'href', '/packages/cem-elements/index.html'));
    for (const target of ['xpath-aggregates.cemt', 'xpath-maps-arrays.html', 'xpath-sequences.html', 'xpath-nodes.html', 'data-table.html']) {
        await runCheck(page, urlEquals(`main > section a[href$="${target}"]`, 'href', `/packages/cem-elements/demo/${target}`));
    }
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, width => {
            const cards = Array.from(document.querySelectorAll('cem-demo-element[legend]'));
            return cards.length === 2 && document.documentElement.scrollWidth <= width
                && cards.every(card => {
                    const box = card.getBoundingClientRect();
                    return box.left >= 0 && box.right <= width
                        && Array.from(card.querySelectorAll('[slot="demo"], article, textarea, table')).every(node => {
                            const rect = node.getBoundingClientRect();
                            return rect.left >= box.left && rect.right <= box.right
                                && node.scrollWidth <= node.clientWidth + 1;
                        });
                });
        }, width);
        const reachable = await page.locator('cem-demo-element [slot="text"] pre').evaluateAll(sources =>
            sources.length === 2 && sources.every(pre => {
                const end = pre.scrollWidth - pre.clientWidth;
                pre.scrollLeft = end;
                const reached = Math.abs(pre.scrollLeft - end) <= 1;
                pre.scrollLeft = 0;
                return reached;
            }));
        if (!reachable) throw new Error('XPath aggregate source cannot be scrolled to its end');
    }
    if (new URL(page.url()).pathname !== '/__cem-source-harness.html') await verifyDemoLayout(page, 2);
}

async function verifyXPathFunctionsPresentation(page) {
    await runCheck(page, nodeTexts('cem-demo-element output', ['First 🍒', 'Second 🍒', 'cherry 🍒', 'Try cherry']));
    await runCheck(page, nodeTexts('cem-demo-element li', ['Recovered XPath: stocked', 'Recovered CEM-QL: stocked']));
    const targets = ['../index.html', './xpath-functions.cemt', '#cem-ql-use-cases', '#cem-ql-label',
        '#cem-ql-predicate', '#cem-ql-nodes', '#cem-ql-import', './xpath-nodes.html', './dom-merge.html',
        './functions/str.html', './functions/str.html', './cell-overrides.html', './functions/dom.html'];
    await poll(page, targets => {
        const links = Array.from(document.querySelectorAll('nav a, main > section a'));
        const source = new URL('/packages/cem-elements/demo/xpath-functions.html', location.href);
        return links.length === targets.length && links.every((link, index) => {
            const target = targets[index];
            return link.href === new URL(target, target.startsWith('#') ? location.href : source).href
                && (!target.startsWith('#') || (link.getAttribute('href') === target && document.querySelector(target)));
        });
    }, targets);
    const originalUrl = page.url();
    for (const fragment of targets.filter(target => target.startsWith('#'))) {
        await page.locator(`a[href="${fragment}"]`).click();
        await poll(page, fragment => location.hash === fragment, fragment);
        await runCheck(page, nodeTexts('cem-demo-element output', ['First 🍒', 'Second 🍒', 'cherry 🍒', 'Try cherry']));
    }
    await page.evaluate(url => history.replaceState(null, '', url), originalUrl);
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, width => {
            const cards = Array.from(document.querySelectorAll('cem-demo-element[legend]'));
            return cards.length === 6 && document.documentElement.scrollWidth <= width
                && cards.every(card => {
                    const box = card.getBoundingClientRect();
                    return box.left >= 0 && box.right <= width
                        && Array.from(card.querySelectorAll('[slot="demo"], article, input, textarea')).every(node => {
                            const rect = node.getBoundingClientRect();
                            return rect.left >= box.left && rect.right <= box.right
                                && node.scrollWidth <= node.clientWidth + 1;
                        });
                });
        }, width);
        const reachable = await page.locator('cem-demo-element [slot="text"] pre').evaluateAll(sources =>
            sources.length === 6 && sources.every(pre => {
                const end = pre.scrollWidth - pre.clientWidth;
                pre.scrollLeft = end;
                const reached = Math.abs(pre.scrollLeft - end) <= 1;
                pre.scrollLeft = 0;
                return reached;
            }));
        if (!reachable) throw new Error('XPath function source cannot be scrolled to its end');
    }
    if (new URL(page.url()).pathname !== '/__cem-source-harness.html') await verifyDemoLayout(page, 6);
}

async function verifyXPathMapsArraysPresentation(page) {
    await runCheck(page, nodeTexts('cem-demo-element output', [
        'deny: 198.51.100.0/24', 'Absent entry', '2', '1', 'cherry: 5', 'Empty member (array size 1)',
        '1 members; numeric total 0', 'Null value',
    ]));
    await runCheck(page, countExactly('nav a, main > section a', 6));
    await runCheck(page, urlEquals('nav a', 'href', '/packages/cem-elements/index.html'));
    for (const target of ['xpath-maps-arrays.cemt', 'xpath-validation.html', 'xpath-sequences.html', 'xpath-aggregates.html', 'data-table.html']) {
        await runCheck(page, urlEquals(`main > section a[href$="${target}"]`, 'href', `/packages/cem-elements/demo/${target}`));
    }
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, width => {
            const cards = Array.from(document.querySelectorAll('cem-demo-element[legend]'));
            return cards.length === 3 && document.documentElement.scrollWidth <= width
                && cards.every(card => {
                    const box = card.getBoundingClientRect();
                    return box.left >= 0 && box.right <= width
                        && Array.from(card.querySelectorAll('[slot="demo"], article, input, textarea, select')).every(node => {
                            const rect = node.getBoundingClientRect();
                            return rect.left >= box.left && rect.right <= box.right
                                && node.scrollWidth <= node.clientWidth + 1;
                        });
                });
        }, width);
        const reachable = await page.locator('cem-demo-element [slot="text"] pre').evaluateAll(sources =>
            sources.length === 3 && sources.every(pre => {
                const end = pre.scrollWidth - pre.clientWidth;
                pre.scrollLeft = end;
                const reached = Math.abs(pre.scrollLeft - end) <= 1;
                pre.scrollLeft = 0;
                return reached;
            }));
        if (!reachable) throw new Error('XPath map/array source cannot be scrolled to its end');
    }
    if (new URL(page.url()).pathname !== '/__cem-source-harness.html') await verifyDemoLayout(page, 3);
}

async function verifyXPathNodesPresentation(page) {
    const table = 'cem-demo-element[legend="1. XML table with native navigation"]';
    for (const check of nodeTable(nodeTableMixedRows, 'c', ['crate', ' ABC '])) {
        await runCheck(page, scopeCheck(check, table));
    }
    await runCheck(page, nodeTexts('cem-demo-element[legend="2. XML tree with attributes and mixed text"] article code', nodeTreeUpdatedCodes));
    await runCheck(page, countExactly('nav a, main > section a', 7));
    await runCheck(page, urlEquals('nav a', 'href', '/packages/cem-elements/index.html'));
    for (const target of ['xpath-nodes.cemt', 'xpath-sort.html', 'xpath-aggregates.html', 'xpath-sequences.html', 'xpath-functions.html', 'data-table.html']) {
        await runCheck(page, urlEquals(`main > section a[href$="${target}"]`, 'href', `/packages/cem-elements/demo/${target}`));
    }
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, width => {
            const cards = Array.from(document.querySelectorAll('cem-demo-element[legend]'));
            return cards.length === 2 && document.documentElement.scrollWidth <= width
                && cards.every(card => {
                    const box = card.getBoundingClientRect();
                    return box.left >= 0 && box.right <= width
                        && Array.from(card.querySelectorAll('[slot="demo"], article, textarea, select, table, th, td, details, summary')).every(node => {
                            const rect = node.getBoundingClientRect();
                            return rect.left >= box.left && rect.right <= box.right
                                && node.scrollWidth <= node.clientWidth + 1;
                        });
                });
        }, width);
        const reachable = await page.locator('cem-demo-element [slot="text"] pre').evaluateAll(sources =>
            sources.length === 2 && sources.every(pre => {
                const end = pre.scrollWidth - pre.clientWidth;
                pre.scrollLeft = end;
                const reached = Math.abs(pre.scrollLeft - end) <= 1;
                pre.scrollLeft = 0;
                return reached;
            }));
        if (!reachable) throw new Error('XPath node source cannot be scrolled to its end');
    }
    if (new URL(page.url()).pathname !== '/__cem-source-harness.html') await verifyDemoLayout(page, 2);
}

async function verifyXPathSequencesPresentation(page) {
    for (const check of sequenceOutputs(['2', 'apple | cherry | apple', 'apple', 'cherry | apple'])) {
        await runCheck(page, scopeCheck(check, 'cem-demo-element[legend="1. A window into a word sequence"]'));
    }
    await runCheck(page, countExactly('nav a, main > section a', 8));
    await runCheck(page, urlEquals('nav a', 'href', '/packages/cem-elements/index.html'));
    for (const target of ['xpath-sequences.cemt', 'xpath-sort.html', 'xpath-maps-arrays.html', 'xpath-aggregates.html', 'xpath-nodes.html', 'functions/str.html', 'data-table.html']) {
        await runCheck(page, urlEquals(`main > section a[href$="${target}"]`, 'href', `/packages/cem-elements/demo/${target}`));
    }
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, width => {
            const cards = Array.from(document.querySelectorAll('cem-demo-element[legend]'));
            return cards.length === 2 && document.documentElement.scrollWidth <= width
                && cards.every(card => {
                    const box = card.getBoundingClientRect();
                    return box.left >= 0 && box.right <= width
                        && Array.from(card.querySelectorAll('[slot="demo"], article, textarea, input, table, th, td')).every(node => {
                            const rect = node.getBoundingClientRect();
                            return rect.left >= box.left && rect.right <= box.right
                                && node.scrollWidth <= node.clientWidth + 1;
                        });
                });
        }, width);
        const reachable = await page.locator('cem-demo-element [slot="text"] pre').evaluateAll(sources =>
            sources.length === 2 && sources.every(pre => {
                const end = pre.scrollWidth - pre.clientWidth;
                pre.scrollLeft = end;
                const reached = Math.abs(pre.scrollLeft - end) <= 1;
                pre.scrollLeft = 0;
                return reached;
            }));
        if (!reachable) throw new Error('XPath sequence source cannot be scrolled to its end');
    }
    if (new URL(page.url()).pathname !== '/__cem-source-harness.html') await verifyDemoLayout(page, 2);
}

async function verifyXPathSortPresentation(page) {
    await runCheck(page, propertyEquals('cem-demo-element[legend="1. Text and numeric keys"] output', 'textContent', sortDecimalDescending));
    await runCheck(page, countExactly('nav a, main > section a', 6));
    await runCheck(page, urlEquals('nav a', 'href', '/packages/cem-elements/index.html'));
    for (const target of ['xpath-sort.cemt', 'xpath-nodes.html', 'xpath-sequences.html', 'data-table.html', 'xpath-validation.html']) {
        await runCheck(page, urlEquals(`main > section a[href$="${target}"]`, 'href', `/packages/cem-elements/demo/${target}`));
    }
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, width => {
            const cards = Array.from(document.querySelectorAll('cem-demo-element[legend]'));
            return cards.length === 2 && document.documentElement.scrollWidth <= width
                && cards.every(card => {
                    const box = card.getBoundingClientRect();
                    return box.left >= 0 && box.right <= width
                        && Array.from(card.querySelectorAll('[slot="demo"], article, textarea, input, table, th, td')).every(node => {
                            const rect = node.getBoundingClientRect();
                            return rect.left >= box.left && rect.right <= box.right
                                && node.scrollWidth <= node.clientWidth + 1;
                        });
                });
        }, width);
        const reachable = await page.locator('cem-demo-element [slot="text"] pre').evaluateAll(sources =>
            sources.length === 2 && sources.every(pre => {
                const end = pre.scrollWidth - pre.clientWidth;
                pre.scrollLeft = end;
                const reached = Math.abs(pre.scrollLeft - end) <= 1;
                pre.scrollLeft = 0;
                return reached;
            }));
        if (!reachable) throw new Error('XPath sorting source cannot be scrolled to its end');
    }
    if (new URL(page.url()).pathname !== '/__cem-source-harness.html') await verifyDemoLayout(page, 2);
}

async function verifyXPathValidationPresentation(page) {
    const form = 'cem-demo-element[legend="1. Form validation preview"]';
    const ip = 'cem-demo-element[legend="2. IPv4 prefix-rule preview"]';
    for (const check of validationOutputs(['Valid user name', 'Age in range', 'Red / GOLD'])) {
        await runCheck(page, scopeCheck(check, form));
    }
    await runCheck(page, countExactly('nav a, main > section a', 6));
    await runCheck(page, urlEquals('nav a', 'href', '/packages/cem-elements/index.html'));
    for (const target of ['xpath-validation.cemt', 'dom-merge.html', 'xpath-maps-arrays.html', 'xpath-sort.html']) {
        await runCheck(page, urlEquals(`main > section a[href$="${target}"]`, 'href', `/packages/cem-elements/demo/${target}`));
    }
    await runCheck(page, urlEquals('main > section a[href$="README.md"]', 'href', '/packages/cem_ml/schema-packages/xpath/v1/README.md'));
    // Exercise wrapping with the longest validation messages visible together.
    for (const [index, value, expected] of [
        [0, '7Ada', 'Use 3–16 ASCII letters, digits or underscores; start with a letter'],
        [1, '1e2', 'Enter an integer age'],
        [2, 'red,,blue', 'Use 1–12 ASCII letters per tag, separated by commas or semicolons'],
    ]) {
        await runCheck(page, scopeCheck(fillThenText(`article label:nth-of-type(${index + 1}) input`, value,
            `article p:nth-of-type(${index + 1}) output`, expected), form));
    }
    await runCheck(page, scopeCheck(fillThenText('article label:first-of-type input', '::1', 'article output',
        'Enter IPv4 with an optional /prefix; no leading zeros'), ip));
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, width => {
            const cards = Array.from(document.querySelectorAll('cem-demo-element[legend]'));
            return cards.length === 2 && document.documentElement.scrollWidth <= width
                && cards.every(card => {
                    const box = card.getBoundingClientRect();
                    return box.left >= 0 && box.right <= width
                        && Array.from(card.querySelectorAll('[slot="demo"], article, label, input, p')).every(node => {
                            const rect = node.getBoundingClientRect();
                            return rect.left >= box.left && rect.right <= box.right
                                && node.scrollWidth <= node.clientWidth + 1;
                        });
                });
        }, width);
        const reachable = await page.locator('cem-demo-element [slot="text"] pre').evaluateAll(sources =>
            sources.length === 2 && sources.every(pre => {
                const end = pre.scrollWidth - pre.clientWidth;
                pre.scrollLeft = end;
                const reached = Math.abs(pre.scrollLeft - end) <= 1;
                pre.scrollLeft = 0;
                return reached;
            }));
        if (!reachable) throw new Error('XPath validation source cannot be scrolled to its end');
    }
    if (new URL(page.url()).pathname !== '/__cem-source-harness.html') await verifyDemoLayout(page, 2);
}

async function verifyDomChainsPresentation(page) {
    for (const [legend, expected] of domChainResults) {
        const final = legend === 'First match' ? 'ivy'
            : legend === 'Sort native nodes' ? 'first, third, second' : expected;
        await runCheck(page, propertyEquals(`cem-demo-element[legend="${legend}"] article output`, 'textContent', final));
    }
    await runCheck(page, propertyEquals('cem-demo-element[legend="First match"] input', 'value', 'name'));
    const source = await readFile(join(repoRoot, 'packages/cem-elements/demo/functions/dom.html'), 'utf8');
    await poll(page, source => {
        const inert = document.createElement('template');
        inert.innerHTML = source;
        const expected = Array.from(inert.content.querySelectorAll('cem-demo-element'), card => ({
            legend: card.getAttribute('legend'), source: card.querySelector('template').innerHTML,
        }));
        const actual = Array.from(document.querySelectorAll('cem-demo-element[legend]'), card => ({
            legend: card.getAttribute('legend'), source: card.querySelector('[slot="text"] code')?.textContent,
        }));
        return JSON.stringify(actual) === JSON.stringify(expected);
    }, source);
    await runCheck(page, countExactly('nav a, main > section a', 5));
    for (const [selector, path] of [
        ['nav a', '/packages/cem-elements/index.html'],
        ['main > section a[href$="cem-ql-chains.md"]', '/docs/cem-ql-chains.md'],
        ['main > section a[href$="cell-overrides.html"]', '/packages/cem-elements/demo/cell-overrides.html'],
        ['main > section a[href$="xpath-functions.html#cem-ql-nodes"]', '/packages/cem-elements/demo/xpath-functions.html#cem-ql-nodes'],
        ['main > section a[href$="str.html"]', '/packages/cem-elements/demo/functions/str.html'],
    ]) {
        await runCheck(page, urlEquals(selector, 'href', path));
    }
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, width => {
            const cards = Array.from(document.querySelectorAll('cem-demo-element[legend]'));
            return cards.length === 27 && document.documentElement.scrollWidth <= width
                && cards.every(card => {
                    const box = card.getBoundingClientRect();
                    return box.left >= 0 && box.right <= width
                        && Array.from(card.querySelectorAll('[slot="demo"], article, input, select')).every(node => {
                            const rect = node.getBoundingClientRect();
                            return rect.left >= box.left && rect.right <= box.right
                                && node.scrollWidth <= node.clientWidth + 1;
                        });
                });
        }, width);
        const reachable = await page.locator('cem-demo-element [slot="text"] pre').evaluateAll(sources =>
            sources.length === 27 && sources.every(pre => {
                const end = pre.scrollWidth - pre.clientWidth;
                pre.scrollLeft = end;
                const reached = Math.abs(pre.scrollLeft - end) <= 1;
                pre.scrollLeft = 0;
                return reached;
            }));
        if (!reachable) throw new Error('DOM chain source cannot be scrolled to its end');
    }
    if (new URL(page.url()).pathname !== '/__cem-source-harness.html') await verifyDemoLayout(page, 27);
}

async function verifyStringFunctionsPresentation(page) {
    for (const entry of stringCases) {
        const selector = `cem-demo-element[legend="${entry.legend}"]`;
        await runCheck(page, propertyEquals(`${selector} output`, 'textContent', entry.edits.at(-1)[2]));
        const values = entry.fields.map(([, , value]) => value);
        for (const [index, value] of entry.edits) values[index] = value;
        for (const [index, [, field]] of entry.fields.entries()) {
            await runCheck(page, propertyEquals(`${selector} ${field}`, 'value', values[index]));
        }
    }
    const source = await readFile(join(repoRoot, 'packages/cem-elements/demo/functions/str.html'), 'utf8');
    await poll(page, source => {
        const inert = document.createElement('template');
        inert.innerHTML = source;
        const expected = Array.from(inert.content.querySelectorAll('cem-demo-element'), card => ({
            legend: card.getAttribute('legend'), source: card.querySelector('template').innerHTML,
        }));
        const actual = Array.from(document.querySelectorAll('cem-demo-element[legend]'), card => ({
            legend: card.getAttribute('legend'), source: card.querySelector('[slot="text"] code')?.textContent,
        }));
        return JSON.stringify(actual) === JSON.stringify(expected);
    }, source);
    await runCheck(page, countExactly('nav a, main > section a', 8));
    await runCheck(page, urlEquals('nav a', 'href', '/packages/cem-elements/index.html'));
    await runCheck(page, urlEquals('main > section a[href$="/dom.html"]', 'href', '/packages/cem-elements/demo/functions/dom.html'));
    for (const target of ['xpath-text.cemt', 'xpath-functions.html', 'xpath-sequences.html', 'dom-merge.html', 'data-slices.html', 'module-url.html']) {
        await runCheck(page, urlEquals(`main > section a[href$="${target}"]`, 'href', `/packages/cem-elements/demo/${target}`));
    }
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, width => {
            const cards = Array.from(document.querySelectorAll('cem-demo-element[legend]'));
            return cards.length === 12 && document.documentElement.scrollWidth <= width
                && cards.every(card => {
                    const box = card.getBoundingClientRect();
                    if (box.left < 0 || box.right > width) return false;
                    return Array.from(card.querySelectorAll('[slot="demo"], article, table, th, td, p, ol, li, [slot="demo"] pre, input, textarea')).every(node => {
                        const rect = node.getBoundingClientRect();
                        return rect.left >= box.left && rect.right <= box.right
                            && (node.matches('input,textarea') || node.scrollWidth <= node.clientWidth + 1);
                    });
                });
        }, width);
        const reachable = await page.locator('cem-demo-element [slot="text"] pre').evaluateAll(sources =>
            sources.length === 12 && sources.every(pre => {
                const end = pre.scrollWidth - pre.clientWidth;
                pre.scrollLeft = end;
                const reached = Math.abs(pre.scrollLeft - end) <= 1;
                pre.scrollLeft = 0;
                return reached;
            }));
        if (!reachable) throw new Error('String-function source cannot be scrolled to its end');
    }
    if (new URL(page.url()).pathname !== '/__cem-source-harness.html') await verifyDemoLayout(page, 12);
}

function isSupportingHtmlSource(path) {
    return ['embed-1.html', 'embed-lib.html', 'lib-dir/embed-lib.html',
        'external-template-document.html', 'external-template-templates.html', 'html-template.html']
        .some(file => path.split('#')[0] === `/packages/cem-elements/demo/${file}`);
}

function supportEmbeddedDocumentChecks() {
    return [countExactly('h4', 1), propertyEquals('h4', 'textContent', 'embed-1.html'),
        countExactly('[data-cem-anonymous-instance]', 1), normalizedText('[data-cem-anonymous-instance]', '🖖'),
        countExactly(':is(h1,a,img,article,script)', 0)];
}

function supportSvgChecks() {
    return [countExactly('svg', 1), propertyEquals('svg', 'namespaceURI', 'http://www.w3.org/2000/svg'),
        attributeEquals('svg', 'viewBox', '0 0 216 209.18'), countExactly('svg polygon', 1), countExactly('svg path', 21)];
}

function supportMathChecks() {
    return [countExactly('math', 1), propertyEquals('math', 'namespaceURI', 'http://www.w3.org/1998/Math/MathML'),
        attributeEquals('math', 'display', 'block'), countExactly('math msubsup', 1), countExactly('math munderover', 1),
        countExactly('math msup', 3), nodeTexts('math mi', ['x', 'x', 'x', 'n', 'n', 'n', 'n'])];
}

async function verifySupportingHtmlSource(page, fixture, tag) {
    const base = '/packages/cem-elements/demo/';
    const file = fixture.path.slice(base.length).split('#')[0];
    const checks = async (selector, items) => {
        for (const check of items) await runCheck(page, scopeCheck(check, selector));
    };
    let variant = 0;
    const source = async (path, items) => {
        const child = `${tag}-fragment-${++variant}`;
        const spec = { path: base + path, declarationAttributes: { 'link-base': 'source' } };
        await mountSourceDocument(page, spec, child);
        await verifySampleContractInventory(page, child, spec);
        await checks(child, items);
        return child;
    };
    const instance = async (parent, payload) => {
        const index = await page.evaluate(({ parent, payload }) => {
            const existing = document.querySelector(parent);
            const element = document.createElement(existing.localName);
            if (payload !== undefined) {
                const strong = document.createElement('strong');
                strong.textContent = payload;
                element.append(strong);
            }
            existing.parentNode.append(element);
            return document.querySelectorAll(existing.localName).length;
        }, { parent, payload });
        return `${parent}:nth-of-type(${index})`;
    };
    if (file.endsWith('embed-lib.html')) {
        const library = 'lib-dir/embed-lib.html';
        const hash = await source(`${file}#embed-relative-hash`, [
            countExactly('a', 1), urlEquals('a', 'href', `${base}${library}#embed-lib-component`),
            normalizedText('dce-embed-lib-component', '👋 from embed-lib-component'),
            countExactly('img', 1), urlEquals('img', 'src', `${base}lib-dir/Smiley.svg`), imageLoaded('img'),
            countExactly(':is(h1,h4,article,script)', 0),
            attributeEquals('img', 'alt', 'Library Smiley'),
        ]);
        await source(`${file}#embed-relative-file`, [countExactly('a', 1), urlEquals('a', 'href', `${base}embed-1.html`),
            ...supportEmbeddedDocumentChecks().map(check => scopeCheck(check, 'dce-embed-lib-file'))]);
        await checks(hash, [normalizedText('dce-embed-lib-component', '👋 from embed-lib-component')]);
        await checks(tag, [normalizedText(':scope', '👋 from embed-lib-component')]);
    }
    if (file === 'external-template-document.html') {
        const projected = await instance(tag, 'Projected <fruit> & 🍒');
        await checks(projected, [propertyEquals('h2', 'textContent', 'External document'),
            propertyEquals('p', 'textContent', 'Projected <fruit> & 🍒'), countExactly('p > strong', 1), countExactly(':is(script,slot)', 0)]);
        await checks(`${tag}:first-of-type`, [propertyEquals('p', 'textContent', 'External document fallback')]);
    }
    if (file === 'external-template-templates.html') {
        const original = `${tag}:first-of-type`;
        const fallback = await instance(tag);
        await checks(fallback, [propertyEquals('h2', 'textContent', 'External template'), propertyEquals('p', 'textContent', 'External fallback')]);
        const projected = await instance(tag, 'Projected <fruit> & 🍒');
        await checks(projected, [propertyEquals('p > strong', 'textContent', 'Projected <fruit> & 🍒')]);
        for (const selector of ['article', 'h2', 'p', 'a']) await checks(original, [elementIdentity(selector, 'remember')]);
        for (const value of ['Edited title', '', '<Fruit & 🍒>', null]) {
            await page.evaluate(({ tag, value }) => {
                const host = document.querySelector(tag);
                if (value === null) host.removeAttribute('title');
                else host.setAttribute('title', value);
            }, { tag, value });
            await checks(original, [propertyEquals('h2', 'textContent', value ?? 'External template'),
                propertyEquals('p', 'textContent', 'Projected source content'),
                urlEquals('a', 'href', `${base}external-template-templates.html#external-card-template`),
                ...['article', 'h2', 'p', 'a'].map(selector => elementIdentity(selector, 'same'))]);
            await checks(fallback, [propertyEquals('h2', 'textContent', 'External template')]);
            await checks(projected, [propertyEquals('p > strong', 'textContent', 'Projected <fruit> & 🍒')]);
        }
        const subtree = await source(`${file}#external-subtree-template`, [countExactly('article', 1),
            propertyEquals('#external-subtree-template h2', 'textContent', 'External subtree'),
            propertyEquals('p', 'textContent', 'External subtree fallback'), countExactly(':is(a, .external-scoped-card)', 0)]);
        const subtreePayload = await instance(subtree, 'Subtree payload 🍋');
        await checks(subtreePayload, [propertyEquals('p > strong', 'textContent', 'Subtree payload 🍋')]);
        await source(`${file}#scoped-css-external-template`, [countExactly('section.external-scoped-card', 1),
            normalizedText('p', 'External scoped fallback ./external-template-templates.html#scoped-css-external-template'),
            computedStyle('p', 'backgroundColor', 'rgb(254, 243, 199)'), computedStyle('p', 'borderTopColor', 'rgb(180, 83, 9)'),
            urlEquals('a', 'href', `${base}external-template-templates.html`), countExactly(':is(article,h2,script)', 0)]);
    }
    if (file === 'html-template.html') {
        for (const [id, items] of [
            ['wave', [propertyEquals('b', 'textContent', '👋')]],
            ['ok', [propertyEquals('b', 'textContent', '👌')]],
            ['dwc-logo', supportSvgChecks()], ['sophomores-dream', supportMathChecks()],
        ]) await source(`${file}#${id}`, [countExactly(':is(b,svg,math)', 1), countExactly('script', 0), ...items]);
        await poll(page, () => Array.from(document.querySelectorAll('svg,math')).every(root =>
            Array.from(root.querySelectorAll('*')).every(node => node.namespaceURI === root.namespaceURI)));
    }
    await poll(page, async () => {
        const runtime = window.__cemFixtureRuntime;
        for (const declaration of document.querySelectorAll('cem-element[tag]')) {
            await runtime.whenDeclarationSettled(declaration);
            if (runtime.diagnosticsFor(declaration).length) return false;
            for (const host of document.querySelectorAll(declaration.getAttribute('tag'))) {
                await runtime.whenRenderSettled(host);
                if (runtime.diagnosticsFor(host).length) return false;
            }
        }
        return true;
    });
    for (const width of [1280, 390]) {
        await page.setViewportSize({ width, height: 900 });
        await poll(page, width => document.documentElement.scrollWidth <= width
            && Array.from(document.querySelectorAll('article,h1,h2,h4,p,a,b,svg,math,img')).every(node => {
                const box = node.getBoundingClientRect();
                return box.left >= 0 && box.right <= width && node.scrollWidth <= node.clientWidth + 1;
            }), width);
    }
}
