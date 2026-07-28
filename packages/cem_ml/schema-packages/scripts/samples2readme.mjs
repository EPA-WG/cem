#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'node:fs';
import { basename, dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReadmePreviews } from './readme-preview.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../..');

if (!process.argv[2]) {
    throw new Error(
        'Usage: node packages/cem_ml/schema-packages/scripts/samples2readme.mjs <package-root>',
    );
}

const packageRoot = resolve(workspaceRoot, process.argv[2]);
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const manifestPath = join(packageRoot, 'package.cem');
const readmePath = join(packageRoot, 'README.md');
const manifest = parseManifest(readFileSync(manifestPath, 'utf8'), packageRoot);
const readme = readFileSync(readmePath, 'utf8');
const packageLabel = readme.match(/^#\s+(.+)$/m)?.[1]?.trim() ?? manifest.packageId;
const cases = manifest.examples.map((example) =>
    previewCaseForExample(example, manifest, packageLabel),
);

await verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    update: true,
    cases,
    packageLabel,
    refreshCommand: `yarn nx run ${projectNameForPackageRoot(packageRoot)}:samples2readme`,
});

writeFileSync(
    readmePath,
    replaceExamplesSection(readme, generatedExamplesSection(manifest, packageLabel)),
    'utf8',
);
console.log(`Updated ${relative(workspaceRoot, readmePath)} from package.cem examples.`);

function parseManifest(source, packageRoot) {
    const packageBlock = findBlocksByName(source, 'package')[0];
    if (!packageBlock) {
        throw new Error('package.cem does not contain a {package} block');
    }
    const packageAttrs = parseBlockHeaderAttributes(packageBlock);
    const examples = findBlocksByName(source, 'example').map((block) => {
        const attrs = parseBlockHeaderAttributes(block);
        for (const key of ['id', 'path', 'content-type', 'schema', 'expected-result']) {
            if (!attrs[key]) {
                throw new Error(`manifest example is missing @${key}: ${block}`);
            }
        }
        return {
            id: attrs.id,
            path: attrs.path,
            contentType: attrs['content-type'],
            schema: attrs.schema,
            expectedResult: attrs['expected-result'],
            expectedDiagnostics: (attrs['expected-diagnostics'] ?? '')
                .split(/\s+/)
                .filter(Boolean),
        };
    });
    if (examples.length === 0) {
        throw new Error('package.cem does not declare any {example} metadata');
    }
    return {
        packageId: packageAttrs.id ?? basename(dirname(packageRoot)),
        examples,
    };
}

function findBlocksByName(source, name) {
    const blocks = [];
    for (let index = 0; index < source.length; index += 1) {
        if (source[index] !== '{') {
            continue;
        }
        let cursor = index + 1;
        while (/\s/.test(source[cursor] ?? '')) {
            cursor += 1;
        }
        const nameStart = cursor;
        while (/[A-Za-z0-9_-]/.test(source[cursor] ?? '')) {
            cursor += 1;
        }
        if (source.slice(nameStart, cursor) !== name) {
            continue;
        }
        const end = findMatchingBrace(source, index);
        blocks.push(source.slice(index, end + 1));
        index = end;
    }
    return blocks;
}

function parseBlockHeaderAttributes(block) {
    let cursor = 1;
    while (/\s/.test(block[cursor] ?? '')) {
        cursor += 1;
    }
    while (/[A-Za-z0-9_-]/.test(block[cursor] ?? '')) {
        cursor += 1;
    }
    let inString = false;
    let escaped = false;
    for (let index = cursor; index < block.length; index += 1) {
        const char = block[index];
        if (inString) {
            if (escaped) {
                escaped = false;
            } else if (char === '\\') {
                escaped = true;
            } else if (char === '"') {
                inString = false;
            }
            continue;
        }
        if (char === '"') {
            inString = true;
            continue;
        }
        if (char === '|' || char === '}') {
            return parseAttributes(block.slice(cursor, index));
        }
    }
    throw new Error(`unterminated CEM block header: ${block.slice(0, 80)}`);
}

function findMatchingBrace(source, start) {
    let depth = 0;
    let inString = false;
    let escaped = false;
    for (let index = start; index < source.length; index += 1) {
        const char = source[index];
        if (inString) {
            if (escaped) {
                escaped = false;
            } else if (char === '\\') {
                escaped = true;
            } else if (char === '"') {
                inString = false;
            }
            continue;
        }
        if (char === '"') {
            inString = true;
            continue;
        }
        if (char === '{') {
            depth += 1;
            continue;
        }
        if (char === '}') {
            depth -= 1;
            if (depth === 0) {
                return index;
            }
        }
    }
    throw new Error(`unterminated CEM block at byte ${start}`);
}

function parseAttributes(block) {
    const attrs = {};
    const pattern = /@([A-Za-z0-9_-]+)\s*=\s*(?:"((?:\\.|[^"\\])*)"|([^\s|}]+))/g;
    for (const match of block.matchAll(pattern)) {
        attrs[match[1]] = match[2] === undefined ? match[3] : unescapeCemString(match[2]);
    }
    return attrs;
}

function unescapeCemString(value) {
    return value.replace(/\\(["\\nrt])/g, (_, escaped) => {
        switch (escaped) {
            case 'n':
                return '\n';
            case 'r':
                return '\r';
            case 't':
                return '\t';
            default:
                return escaped;
        }
    });
}

function previewCaseForExample(example, manifest, packageLabel) {
    const plan = previewPlanForExample(example, manifest);
    return {
        id: `${example.id}-preview`,
        preview: `${previewFileBase(example)}.svg`,
        html: `${previewFileBase(example)}.html`,
        title: `${packageLabel} ${example.id} example preview`,
        description: `Preview of ${example.path} from package.cem example metadata.`,
        terminalTitle: `${plan.label} ${basename(example.path)}`,
        renderer: plan.renderer,
        expectedStatus: plan.expectedStatus,
        args: plan.args,
        sourcePath: plan.sourcePath,
        fallbackSourcePath: plan.fallbackSourcePath,
        width: plan.width,
        minHeight: plan.minHeight,
    };
}

function previewPlanForExample(example, manifest) {
    const inputPath = relative(workspaceRoot, join(packageRoot, example.path));
    const inputSpec = inputSpecForExample(example, inputPath);
    const essence = contentTypeEssence(example.contentType);
    const sourceOnly = {
        label: 'source',
        renderer: 'text',
        sourcePath: inputPath,
        width: 920,
        minHeight: 190,
    };
    const expectedStatus = 'any';

    if (isCemFamilyExample(example, essence)) {
        return {
            label: 'tabular',
            renderer: 'ansi',
            expectedStatus,
            fallbackSourcePath: inputPath,
            width: 980,
            minHeight: 190,
            args: convertPreviewArgs(inputSpec, {
                toContentType: 'application/cem',
                toSchema: 'https://cem.dev/ns/cem-ml/1',
            }),
        };
    }

    if (essence === 'text/csv') {
        return {
            label: 'tabular',
            renderer: 'html',
            expectedStatus,
            fallbackSourcePath: inputPath,
            width: 680,
            minHeight: 170,
            args: convertPreviewArgs(inputSpec, {
                toContentType: 'text/csv',
                toSchema: example.schema,
                colorProfile: 'html',
                outputColorType: null,
                extra: [
                    '--cemt-formatter-option',
                    'csv.maxFieldWidth=24',
                    '--cemt-formatter-option',
                    'csv.stringTrim=middle',
                ],
            }),
        };
    }

    if (isCemQlModuleExample(example, essence)) {
        return {
            label: 'tabular',
            renderer: 'ansi',
            expectedStatus,
            fallbackSourcePath: inputPath,
            width: 980,
            minHeight: 190,
            args: convertPreviewArgs(inputSpec, {
                toContentType: 'application/vnd.cem.query+cem-ql',
                toSchema: 'https://cem.dev/ns/query/cem-ql/1',
            }),
        };
    }

    if (isJsonTextExample(example, manifest, essence)) {
        return {
            label: 'json',
            renderer: 'ansi',
            expectedStatus,
            fallbackSourcePath: inputPath,
            width: 780,
            minHeight: 190,
            args: convertPreviewArgs(inputSpec, {
                toContentType: 'application/json',
                toSchema: 'https://cem.dev/ns/data/json/1',
            }),
        };
    }

    if (isJsonSchemaTextExample(example, manifest, essence)) {
        if (example.expectedResult !== 'pass') {
            return {
                label: 'json-schema validate',
                renderer: 'json',
                expectedStatus: 'success',
                width: 1040,
                minHeight: 520,
                args: validatePreviewArgs(inputPath, {
                    format: 'json',
                    failLevel: 'parse',
                    contentType: example.contentType,
                    schema: example.schema,
                }),
            };
        }
        return {
            label: 'json-schema',
            renderer: 'html',
            expectedStatus,
            fallbackSourcePath: inputPath,
            width: 820,
            minHeight: 190,
            args: convertPreviewArgs(inputSpec, {
                toContentType: 'application/schema+json',
                toSchema: 'https://cem.dev/ns/data/json-schema/1',
                colorProfile: 'html',
                outputColorType: null,
            }),
        };
    }

    if (isYamlTextExample(example, manifest, essence)) {
        return {
            label: 'yaml',
            renderer: 'ansi',
            expectedStatus,
            fallbackSourcePath: inputPath,
            width: 780,
            minHeight: 190,
            args: convertPreviewArgs(inputSpec, {
                toContentType: 'application/yaml',
                toSchema: 'https://cem.dev/ns/data/yaml/1',
            }),
        };
    }

    if (isMarkdownTextExample(example, manifest, essence)) {
        if (example.expectedResult !== 'pass') {
            return {
                label: 'markdown validate',
                renderer: 'json',
                expectedStatus: 'success',
                width: 1040,
                minHeight: 520,
                args: validatePreviewArgs(inputPath, {
                    format: 'json',
                    failLevel: 'parse',
                    contentType: example.contentType,
                    schema: example.schema,
                }),
            };
        }
        return {
            label: 'markdown',
            renderer: 'html',
            expectedStatus,
            width: 920,
            minHeight: 190,
            args: convertPreviewArgs(inputSpec, {
                toContentType: 'text/markdown',
                toSchema: 'https://cem.dev/ns/data/markdown/1',
                colorProfile: 'html',
                outputColorType: null,
            }),
        };
    }

    if (isHtmlExample(essence)) {
        return {
            label: 'html',
            renderer: 'ansi',
            expectedStatus,
            fallbackSourcePath: inputPath,
            width: 980,
            minHeight: 190,
            args: convertPreviewArgs(inputSpec, {
                fromFormat: 'html',
                toContentType: 'text/html',
                toSchema: 'https://cem.dev/ns/data/html/1',
            }),
        };
    }

    if (isXmlFamilyExample(essence)) {
        return {
            label: 'xml',
            renderer: 'ansi',
            expectedStatus,
            fallbackSourcePath: inputPath,
            width: 980,
            minHeight: 190,
            args: convertPreviewArgs(inputSpec, {
                fromFormat: 'xml',
                toContentType: example.contentType,
                toSchema: example.schema,
            }),
        };
    }

    return sourceOnly;
}

function inputSpecForExample(example, inputPath) {
    return `uri=${inputPath},contentType=${example.contentType},schema=${example.schema}`;
}

function convertPreviewArgs(
    inputSpec,
    {
        fromFormat = null,
        toContentType,
        toSchema,
        colorProfile = 'terminal',
        outputColorType = 'ansi-256',
        extra = [],
    },
) {
    const args = ['convert', '--input-spec', inputSpec];
    if (fromFormat) {
        args.push('--from-format', fromFormat);
    }
    args.push(
        '--to-content-type',
        toContentType,
        '--to-schema',
        toSchema,
        '--cemt-formatter-profile',
        'tabular',
        '--cemt-color-profile',
        colorProfile,
    );
    if (outputColorType) {
        args.push('--output-color-type', outputColorType);
    }
    args.push(...extra);
    return args;
}

function validatePreviewArgs(
    inputPath,
    { format = 'json', failLevel = 'validate', contentType = null, schema = null } = {},
) {
    const args = ['validate', '--format', format, '--fail-level', failLevel];
    if (contentType) {
        args.push('--content-type', contentType);
    }
    if (schema) {
        args.push('--schema', schema);
    }
    args.push(inputPath);
    return args;
}

function contentTypeEssence(contentType) {
    return contentType.split(';', 1)[0].trim().toLowerCase();
}

function isCemFamilyExample(example, essence) {
    return (
        essence === 'application/cem' ||
        essence.endsWith('+cem') ||
        example.path.endsWith('.cem') ||
        example.path.endsWith('.cemt')
    );
}

function isCemQlModuleExample(example, essence) {
    return (
        (essence === 'application/vnd.cem.query+cem-ql' || essence === 'text/cem-ql') &&
        !example.schema.endsWith('#expression')
    );
}

function isJsonTextExample(_example, manifest, essence) {
    return (
        manifest.packageId === 'json' &&
        (essence === 'application/json' || essence === 'text/json')
    );
}

function isJsonSchemaTextExample(_example, manifest, essence) {
    return (
        manifest.packageId === 'json-schema' &&
        (essence === 'application/schema+json' ||
            essence === 'application/json' ||
            essence === 'text/json')
    );
}

function isYamlTextExample(_example, manifest, essence) {
    return (
        manifest.packageId === 'yaml' &&
        (essence === 'application/yaml' ||
            essence === 'application/x-yaml' ||
            essence === 'text/yaml' ||
            essence === 'text/x-yaml')
    );
}

function isMarkdownTextExample(_example, manifest, essence) {
    return manifest.packageId === 'markdown' && essence === 'text/markdown';
}

function isHtmlExample(essence) {
    return essence === 'text/html';
}

function isXmlFamilyExample(essence) {
    return (
        essence === 'application/xml' ||
        essence === 'text/xml' ||
        essence === 'application/xhtml+xml' ||
        essence === 'image/svg+xml' ||
        essence === 'application/mathml+xml' ||
        essence === 'application/mathml-content+xml' ||
        essence === 'application/xslt+xml' ||
        essence === 'text/xsl' ||
        essence === 'application/relax-ng+xml'
    );
}

function generatedExamplesSection(manifest, packageLabel) {
    const hasValidationPreviews = manifest.examples.some(
        (example) => previewPlanForExample(example, manifest).args?.[0] === 'validate',
    );
    const hasSourceSnapshotFallback = manifest.examples.some((example) => {
        const plan = previewPlanForExample(example, manifest);
        return Boolean(plan.sourcePath || plan.fallbackSourcePath);
    });
    const lines = [
        '## Examples',
        '',
        'This section is generated from `package.cem` `{example}` metadata by the',
        ...(hasValidationPreviews
            ? [
                  '`samples2readme` Nx target. Each SVG previews the rendered example',
                  'content or validation diagnostics for expected-fail examples. The target writes a',
                  'preformatted HTML preview to',
              ]
            : [
                  '`samples2readme` Nx target. Each SVG previews the example content, not',
                  'the validation report. The target writes a preformatted HTML preview to',
              ]),
        '`dist/cem_ml/schema-packages/<package>/v1/examples/<example-file>.html`,',
        'then renders the `<pre>` spans through headless Chromium into',
        '`examples/previews/<example-file>.svg`.',
        ...(hasSourceSnapshotFallback
            ? [
                  'Source snapshots are used only where the current CLI cannot yet render',
                  'the package formatter/colorizer path for that content identity.',
              ]
            : []),
        '',
    ];
    for (const example of manifest.examples) {
        const plan = previewPlanForExample(example, manifest);
        const preview = `examples/previews/${previewFileBase(example)}.svg`;
        const htmlPreview = `dist/cem_ml/schema-packages/${manifest.packageId}/v1/examples/${previewFileBase(example)}.html`;
        lines.push('<details>', `<summary>${escapeHtml(example.id)}</summary>`, '');
        lines.push(`- Source: [\`${example.path}\`](${relativeMarkdownLink(example.path)})`);
        lines.push(`- Content type: \`${example.contentType}\``);
        lines.push(`- Schema: \`${example.schema}\``);
        lines.push(`- Expected result: \`${example.expectedResult}\``);
        if (example.expectedDiagnostics.length > 0) {
            lines.push(
                `- Expected diagnostics: ${example.expectedDiagnostics
                    .map((code) => `\`${code}\``)
                .join(', ')}`,
            );
        }
        lines.push(`- Preview renderer: \`${previewRendererLabel(plan)}\``);
        lines.push(`- Preview HTML: \`${htmlPreview}\``);
        if (plan.args) {
            lines.push('', '```bash');
            lines.push(...shellCommandLines(['dist/target/cem_ml_cli/debug/cem-ml', ...plan.args]));
            lines.push('```');
        }
        lines.push('', '</details>', '');
        lines.push(`![Preview of ${packageLabel} ${example.id} example](${preview})`, '');
    }
    return `${lines.join('\n').trimEnd()}\n`;
}

function shellCommandLines(args) {
    const rendered = args.map(shellArg);
    const lines = [];
    let current = rendered[0];
    for (const arg of rendered.slice(1)) {
        if (current.length + 1 + arg.length > 88) {
            lines.push(`${current} \\`);
            current = `  ${arg}`;
        } else {
            current = `${current} ${arg}`;
        }
    }
    lines.push(current);
    return lines;
}

function previewRendererLabel(plan) {
    if (plan.sourcePath) {
        return 'source snapshot HTML + html2svg';
    }
    if (plan.args?.[0] === 'validate') {
        return 'CLI validate, JSON report, preview HTML + html2svg';
    }
    return 'CLI convert, tabular formatter, preview HTML + html2svg';
}

function shellArg(value) {
    if (/^[A-Za-z0-9_./:@%+=,-]+$/.test(value)) {
        return value;
    }
    return `'${value.replaceAll("'", "'\\''")}'`;
}

function relativeMarkdownLink(value) {
    if (value.startsWith('./') || value.startsWith('../')) {
        return value;
    }
    return `./${value}`;
}

function escapeHtml(value) {
    return value
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
        .replaceAll('"', '&quot;');
}

function replaceExamplesSection(readme, replacement) {
    const heading = /^## .*Examples.*$/gm;
    const match = heading.exec(readme);
    if (!match) {
        return `${readme.trimEnd()}\n\n${replacement}`;
    }
    const start = match.index;
    const nextHeading = /^## /gm;
    nextHeading.lastIndex = start + match[0].length;
    const next = nextHeading.exec(readme);
    const end = next ? next.index : readme.length;
    return `${readme.slice(0, start).trimEnd()}\n\n${replacement}${readme
        .slice(end)
        .replace(/^\n+/, '\n')}`;
}

function safeFileStem(value) {
    return value.replace(/[^A-Za-z0-9._-]+/g, '-').replace(/^-+|-+$/g, '');
}

function previewFileBase(example) {
    return safeFileStem(basename(example.path));
}

function projectNameForPackageRoot(root) {
    const packageId = basename(dirname(root)).replaceAll('-', '_');
    return `cem_ml_schema_package_${packageId}_v1`;
}
