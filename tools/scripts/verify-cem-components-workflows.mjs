#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const componentMvpPath = join(repoRoot, 'docs/component-mvp.md');
const examplesDir = join(repoRoot, 'packages/cem-components/examples');
const workflowFixturesDir = join(repoRoot, 'packages/cem-components/tests/workflows');
const examplesReadmePath = join(examplesDir, 'README.md');

const workflowContracts = [
    {
        name: 'auth-form',
        requiredTags: ['cem-action', 'cem-card', 'cem-checkbox', 'cem-text-field'],
    },
    {
        name: 'profile-editor',
        requiredTags: ['cem-action', 'cem-alert', 'cem-avatar', 'cem-card', 'cem-stack', 'cem-switch', 'cem-text-field', 'cem-textarea'],
    },
    {
        name: 'asset-browser',
        requiredTags: ['cem-app-bar', 'cem-badge', 'cem-card', 'cem-chip', 'cem-icon-button', 'cem-media-preview', 'cem-table', 'cem-tabs'],
    },
    {
        name: 'discussion-thread',
        requiredTags: ['cem-action', 'cem-alert', 'cem-badge', 'cem-card', 'cem-list', 'cem-textarea', 'cem-toast'],
    },
    {
        name: 'settings',
        requiredTags: ['cem-action', 'cem-alert', 'cem-card', 'cem-checkbox', 'cem-dialog', 'cem-progress', 'cem-radio', 'cem-sheet', 'cem-skeleton', 'cem-switch', 'cem-toast'],
    },
];

const failures = [];
const mvpTags = parseMvpTags(readText(componentMvpPath));
const readme = readText(examplesReadmePath);

for (const workflow of workflowContracts) {
    const fixturePath = join(workflowFixturesDir, `${workflow.name}.html`);
    const examplePath = join(examplesDir, `${workflow.name}.html`);

    assertExists(fixturePath, `${workflow.name} workflow fixture`);
    assertExists(examplePath, `${workflow.name} workflow example`);
    assertReadmeLinksExample(readme, workflow.name);

    if (!existsSync(fixturePath) || !existsSync(examplePath)) {
        continue;
    }

    const fixture = readText(fixturePath);
    const example = readText(examplePath);
    assertDeclarativeMarkup(fixture, fixturePath);
    assertDeclarativeMarkup(example, examplePath);

    const fixtureTags = collectCemTags(fixture);
    const exampleTags = collectCemTags(example);
    assertKnownMvpTags(fixtureTags, fixturePath);
    assertKnownMvpTags(exampleTags, examplePath);
    assertRequiredTags(workflow, fixtureTags, fixturePath);
    assertRequiredTags(workflow, exampleTags, examplePath);
    assertSameTagSet(fixtureTags, exampleTags, workflow.name);
}

if (failures.length > 0) {
    for (const failure of failures) {
        console.error(`error: ${failure}`);
    }
    process.exit(1);
}

console.log(`cem-components workflow examples verified (${workflowContracts.length} workflow surfaces).`);

function readText(path) {
    return readFileSync(path, 'utf8');
}

function repoPath(path) {
    return relative(repoRoot, path);
}

function parseMvpTags(markdown) {
    const tags = new Set();
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
        const tag = cells[2]?.replace(/^`|`$/g, '');
        if (tag?.startsWith('cem-')) {
            tags.add(tag);
        }
    }

    return tags;
}

function collectCemTags(markup) {
    return new Set([...markup.matchAll(/<\s*(cem-[a-z0-9-]+)\b/gi)].map((match) => match[1].toLowerCase()));
}

function assertExists(path, label) {
    if (!existsSync(path)) {
        failures.push(`${label} missing at ${repoPath(path)}`);
    }
}

function assertReadmeLinksExample(readmeText, workflowName) {
    if (!readmeText.includes(`./${workflowName}.html`)) {
        failures.push(`examples README must link ${workflowName}.html`);
    }
}

function assertDeclarativeMarkup(markup, path) {
    const label = repoPath(path);
    if (/<script\b/i.test(markup)) {
        failures.push(`${label}: workflow examples/fixtures must not include <script>`);
    }
    if (/\son[a-z]+\s*=/i.test(markup)) {
        failures.push(`${label}: workflow examples/fixtures must not include inline event handlers`);
    }
    if (/<style\b/i.test(markup) || /\sstyle\s*=/i.test(markup)) {
        failures.push(`${label}: workflow examples/fixtures must not include inline styles`);
    }
    if (/<\/?custom-element\b/i.test(markup)) {
        failures.push(`${label}: workflow examples/fixtures must not depend on legacy <custom-element>`);
    }
}

function assertKnownMvpTags(tags, path) {
    for (const tag of tags) {
        if (!mvpTags.has(tag)) {
            failures.push(`${repoPath(path)}: ${tag} is not listed in docs/component-mvp.md`);
        }
    }
}

function assertRequiredTags(workflow, tags, path) {
    for (const tag of workflow.requiredTags) {
        if (!tags.has(tag)) {
            failures.push(`${repoPath(path)}: ${workflow.name} must include ${tag}`);
        }
    }
}

function assertSameTagSet(fixtureTags, exampleTags, workflowName) {
    const missingFromExample = [...fixtureTags].filter((tag) => !exampleTags.has(tag));
    const missingFromFixture = [...exampleTags].filter((tag) => !fixtureTags.has(tag));
    if (missingFromExample.length > 0) {
        failures.push(`${workflowName}: example missing fixture component tags ${missingFromExample.join(', ')}`);
    }
    if (missingFromFixture.length > 0) {
        failures.push(`${workflowName}: fixture missing example component tags ${missingFromFixture.join(', ')}`);
    }
}
