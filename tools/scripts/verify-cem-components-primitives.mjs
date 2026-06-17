#!/usr/bin/env node

import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import ts from 'typescript';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const mvpPath = join(repoRoot, 'docs/component-mvp.md');
const primitivesPath = join(repoRoot, 'packages/cem-components/src/lib/primitives.ts');

const failures = [];

function fail(message) {
    failures.push(message);
}

function readText(path) {
    return readFileSync(path, 'utf8');
}

function parseMvpComponents(markdown) {
    const components = [];
    let inComponentTable = false;

    for (const line of markdown.split(/\r?\n/)) {
        if (line.startsWith('| Category | Component ID | Element name |')) {
            inComponentTable = true;
            continue;
        }
        if (!inComponentTable) {
            continue;
        }
        if (line.startsWith('| ---')) {
            continue;
        }
        if (!line.startsWith('|')) {
            break;
        }

        const cells = line
            .slice(1, -1)
            .split('|')
            .map((cell) => cell.trim());
        if (cells.length !== 5) {
            fail(`component MVP row must have 5 cells: ${line}`);
            continue;
        }

        const [category, idCell, tagCell, , tokenFamiliesCell] = cells;
        const id = stripCode(idCell);
        const tag = stripCode(tagCell);
        const tokenFamilies = tokenFamiliesCell
            .split(',')
            .map((family) => family.trim())
            .filter(Boolean);

        components.push({ category, id, tag, tokenFamilies });
    }

    return components;
}

function stripCode(value) {
    return value.replace(/^`|`$/g, '');
}

function parsePrimitiveDeclarations(sourceText) {
    const source = ts.createSourceFile(primitivesPath, sourceText, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
    const declaration = source.statements.find(
        (statement) =>
            ts.isVariableStatement(statement) &&
            statement.declarationList.declarations.some(
                (entry) => ts.isIdentifier(entry.name) && entry.name.text === 'CEM_COMPONENT_PRIMITIVES'
            )
    );

    if (!declaration) {
        fail('missing CEM_COMPONENT_PRIMITIVES declaration');
        return [];
    }

    const primitiveDeclaration = declaration.declarationList.declarations.find(
        (entry) => ts.isIdentifier(entry.name) && entry.name.text === 'CEM_COMPONENT_PRIMITIVES'
    );
    const expression = unwrapSatisfies(primitiveDeclaration?.initializer);
    if (!expression || !ts.isArrayLiteralExpression(expression)) {
        fail('CEM_COMPONENT_PRIMITIVES must be an array literal');
        return [];
    }

    return expression.elements.map((element, index) => parsePrimitiveElement(element, index)).filter(Boolean);
}

function unwrapSatisfies(expression) {
    if (!expression) {
        return undefined;
    }
    if (ts.isSatisfiesExpression(expression) || ts.isAsExpression(expression)) {
        return unwrapSatisfies(expression.expression);
    }
    return expression;
}

function parsePrimitiveElement(element, index) {
    if (!ts.isObjectLiteralExpression(element)) {
        fail(`primitive at index ${index} must be an object literal`);
        return undefined;
    }

    const tag = propertyString(element, 'tag');
    const description = propertyString(element, 'description');
    const cemMl = propertyString(element, 'cemMl');

    if (!tag) {
        fail(`primitive at index ${index} is missing tag`);
    }
    if (!description) {
        fail(`${tag ?? `primitive ${index}`}: missing description`);
    }
    if (!cemMl) {
        fail(`${tag ?? `primitive ${index}`}: missing CEM-ML declaration`);
    }

    return { tag, description, cemMl };
}

function propertyString(object, name) {
    const property = object.properties.find(
        (entry) =>
            ts.isPropertyAssignment(entry) &&
            ((ts.isIdentifier(entry.name) && entry.name.text === name) ||
                (ts.isStringLiteral(entry.name) && entry.name.text === name))
    );
    if (!property || !ts.isPropertyAssignment(property)) {
        return '';
    }
    return expressionString(property.initializer);
}

function expressionString(expression) {
    if (ts.isStringLiteral(expression) || ts.isNoSubstitutionTemplateLiteral(expression)) {
        return expression.text;
    }
    if (ts.isBinaryExpression(expression) && expression.operatorToken.kind === ts.SyntaxKind.PlusToken) {
        return `${expressionString(expression.left)}${expressionString(expression.right)}`;
    }
    return '';
}

function duplicateValues(values) {
    const seen = new Set();
    const duplicates = new Set();
    for (const value of values) {
        if (seen.has(value)) {
            duplicates.add(value);
        }
        seen.add(value);
    }
    return [...duplicates];
}

const mvpComponents = parseMvpComponents(readText(mvpPath));
const primitives = parsePrimitiveDeclarations(readText(primitivesPath));

const mvpTags = mvpComponents.map((component) => component.tag);
const primitiveTags = primitives.map((primitive) => primitive.tag);

for (const duplicate of duplicateValues(mvpTags)) {
    fail(`duplicate component MVP tag ${duplicate}`);
}
for (const duplicate of duplicateValues(primitiveTags)) {
    fail(`duplicate primitive tag ${duplicate}`);
}

const missing = mvpTags.filter((tag) => !primitiveTags.includes(tag));
const extra = primitiveTags.filter((tag) => !mvpTags.includes(tag));
if (missing.length > 0) {
    fail(`missing primitive declarations for MVP tags: ${missing.join(', ')}`);
}
if (extra.length > 0) {
    fail(`primitive declarations not listed in component MVP: ${extra.join(', ')}`);
}
if (mvpTags.join('\n') !== primitiveTags.join('\n')) {
    fail('CEM_COMPONENT_PRIMITIVES order must match docs/component-mvp.md component order');
}

for (const component of mvpComponents) {
    if (component.tokenFamilies.length === 0) {
        fail(`${component.tag}: component MVP row must list required token families`);
    }
}

for (const primitive of primitives) {
    if (!primitive.tag || !primitive.cemMl) {
        continue;
    }
    if (!primitive.cemMl.trim().startsWith('{')) {
        fail(`${primitive.tag}: declaration must be CEM-ML source`);
    }
    if (/<\/?custom-element\b/i.test(primitive.cemMl)) {
        fail(`${primitive.tag}: declaration must not depend on legacy <custom-element>`);
    }
    if (/<\/?cem-element\b/i.test(primitive.cemMl)) {
        fail(`${primitive.tag}: primitive source should be CEM-ML, not a declaration wrapper`);
    }
    if (!primitive.cemMl.includes(`cem-${primitive.tag.slice('cem-'.length)}`)) {
        fail(`${primitive.tag}: declaration should include a primitive-scoped class or tag marker`);
    }
}

if (failures.length > 0) {
    for (const failure of failures) {
        console.error(`error: ${failure}`);
    }
    process.exit(1);
}

console.log(`cem-components primitive manifest verified (${primitiveTags.length} primitives).`);
