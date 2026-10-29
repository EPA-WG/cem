import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises';
import { dirname, resolve, relative, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { gzipSync, brotliCompressSync, constants } from 'node:zlib';
import ts from 'typescript';

const root = resolve(import.meta.dirname, '../..');
const sourceRoot = 'packages/cem-elements/src/';
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const sorted = values => [...new Set(values)].sort();
const attrs = text => Object.fromEntries([...text.matchAll(/([\w:-]+)\s*=\s*(["'])(.*?)\2/gs)].map(m => [m[1], m[3]]));
const selectors = new Set(['mode', 'format', 'content-type', 'highlight', 'type', 'href', 'src', 'image', 'query', 'language', 'view', 'align', 'size', 'direction', 'orientation', 'fit', 'variant', 'layout', 'selection', 'multiple', 'expanded', 'open', 'readonly', 'disabled', 'checked', 'indeterminate']);

// Source metadata scanner, not a CEM parser, runtime interpreter or liveness proof.
export function componentEvidence(entry, source, registry, tags = new Set()) {
    const header = attrs(source.match(/<cem-element\b([^>]*)>/s)?.[1] ?? '');
    assert.equal(header.tag, entry.tag, `catalog/declaration tag drift: ${entry.tag}`);
    const templates = [...source.matchAll(/<template\b([^>]*)>/gs)].map(m => attrs(m[1])).filter(a => a.id === entry.tag);
    assert.equal(templates.length, 1, `ambiguous/missing template for ${entry.tag}`);
    assert.equal(templates[0].type, 'text/cem-ml', `unsupported declaration language: ${entry.tag}`);
    const capability = templates[0].capability ?? header.capability ?? null;
    assert(!capability || registry.has(capability), `unknown capability ${capability}: ${entry.tag}`);
    const template = source.replace(/```[\s\S]*?```/g, '');
    const attributes = sorted([...template.matchAll(/\{attribute\s+@name=(?:"([^"]+)"|'([^']+)'|([^\s}|]+))/g)].map(m => m[1] ?? m[2] ?? m[3]));
    const referencedAttributes = sorted([...template.matchAll(/\bdatadom\.attributes\.([\w-]+)/g)].map(m => m[1]));
    const nativeCalls = sorted([...template.matchAll(/\b([\w.-]+:[\w.-]+)\s*\(/g)].map(m => m[1]));
    const nativeDataImport = nativeCalls.includes('data:read');
    const references = sorted([...template.matchAll(/\{(cem-[\w-]+)\b/g)].map(m => m[1]).filter(tag => tags.has(tag) && tag !== entry.tag));
    return {
        tag: entry.tag, source: entry.implementation.source, sha256: sha(source),
        capability, capabilityLocation: templates[0].capability !== undefined ? 'template' : capability ? 'declaration' : 'none',
        behaviorSource: registry.get(capability)?.source ?? null, attributes, referencedAttributes, modeSelectors: sorted([...attributes,...referencedAttributes].filter(a => selectors.has(a))),
        nativeCalls, componentReferences: references, dynamicPayloads: /\{slot\b/.test(template),
        nativeDataImport, requiredCore: ['cem-ml/cem-ql-render', 'native-css-and-selectors', 'native-values-and-provenance', 'admission-ownership-recovery'],
        conditionalCandidates: entry.tag === 'cem-source-editor' ? ['language-specific-highlighting']
            : entry.tag === 'cem-media-preview' ? ['selected-preview-operation-or-template-language']
            : nativeDataImport ? ['csv-import-after-native-handoff'] : [],
        boundary: nativeDataImport ? 'JSON/CSV data:read currently runs inside native evaluation; keep required parsers in core until typed handoff/resumption is proven.'
            : 'Component/source references are conservative metadata; dynamic payloads, providers, function libraries and optional template languages need native requirement receipts.',
    };
}

function syntax(source, file) { return ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true); }
function runtimeImport(node) {
    if (ts.isImportDeclaration(node)) {
        const clause = node.importClause;
        return !clause || !clause.isTypeOnly && (!!clause.name || !clause.namedBindings || !ts.isNamedImports(clause.namedBindings) || clause.namedBindings.elements.some(e => !e.isTypeOnly));
    }
    return !node.isTypeOnly && (!node.exportClause || !ts.isNamedExports(node.exportClause) || node.exportClause.elements.some(e => !e.isTypeOnly));
}
export function importEvidence(source, file) {
    const ast = syntax(source, file), edges = []; let unresolvedDynamicImports = 0;
    function visit(node) {
        if ((ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) && node.moduleSpecifier && runtimeImport(node)) {
            edges.push({ kind: 'static', specifier: node.moduleSpecifier.text });
        } else if (ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword) {
            const value = node.arguments[0];
            if (value && (ts.isStringLiteral(value) || ts.isNoSubstitutionTemplateLiteral(value))) edges.push({ kind: 'dynamic', specifier: value.text });
            else unresolvedDynamicImports++;
        } else if (ts.isNewExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === 'URL' && node.arguments?.[0] && ts.isStringLiteral(node.arguments[0]) && /\.(?:js|ts|wasm)$/.test(node.arguments[0].text)) {
            edges.push({ kind: 'asset', specifier: node.arguments[0].text });
        }
        ts.forEachChild(node, visit);
    }
    visit(ast); return { edges, unresolvedDynamicImports };
}
export function behaviorAttributeReads(source, file) {
    const names = [];
    function visit(node) {
        if (ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression)
            && ['getAttribute', 'hasAttribute', 'getAttributeNS', 'hasAttributeNS'].includes(node.expression.name.text)) {
            const index = node.expression.name.text.endsWith('NS') ? 1 : 0;
            const value = node.arguments[index];
            if (value && ts.isStringLiteral(value)) names.push(value.text);
        }
        ts.forEachChild(node, visit);
    }
    visit(syntax(source, file)); return sorted(names);
}
function objectValue(node) {
    if (ts.isObjectLiteralExpression(node)) return node;
    if (ts.isAsExpression(node) || ts.isSatisfiesExpression(node) || ts.isParenthesizedExpression(node)) return objectValue(node.expression);
    if (ts.isCallExpression(node) && node.arguments[0]) return objectValue(node.arguments[0]);
    throw Error('unsupported capability registry shape');
}
export function capabilityRegistry(source, file) {
    const ast = syntax(source, file), imports = new Map(); let declaration;
    function visit(node) {
        if (ts.isImportDeclaration(node) && node.importClause?.namedBindings && ts.isNamedImports(node.importClause.namedBindings)) {
            for (const entry of node.importClause.namedBindings.elements) imports.set(entry.name.text, node.moduleSpecifier.text);
        }
        if (ts.isVariableDeclaration(node) && node.name.getText(ast) === 'CEM_DECLARATIVE_CAPABILITIES') declaration = node;
        ts.forEachChild(node, visit);
    }
    visit(ast); assert(declaration, 'missing capability registry');
    return new Map(objectValue(declaration.initializer).properties.map(property => {
        const behavior = objectValue(property.initializer).properties.find(p => p.name?.getText(ast) === 'behavior');
        const symbol = behavior?.initializer?.getText(ast);
        assert(imports.has(symbol), `unknown behavior ${symbol}`);
        return [property.name.text, { symbol, source: imports.get(symbol) }];
    }));
}
export function classifyNativeEntrypoint(name) {
    if (/admission|retire|inventory|checkpoint|memory|owner|sealcommitted/i.test(name)) return 'core-control';
    if (/highlight|debug|commandservice|commandrun|inspect|format|coloriz|transformconfig/i.test(name)) return 'candidate-tooling';
    if (/xslt|xpath|legacy|scss|markdown|yaml|csv/i.test(name)) return 'candidate-language';
    return 'core-render-or-data';
}
export function wasmSections(input) {
    const bytes = new Uint8Array(input); assert.deepEqual([...bytes.slice(0,8)], [0,97,115,109,1,0,0,0], 'WASM header');
    let offset = 8; const sections = [], names = ['custom','type','import','function','table','memory','global','export','start','element','code','data','data-count','tag'];
    function leb() {
        let value = 0, shift = 0;
        for (let i = 0; i < 5; i++) { assert(offset < bytes.length, 'truncated LEB'); const b = bytes[offset++]; value += (b & 127) * 2 ** shift; if (!(b & 128)) return value; shift += 7; }
        throw Error('invalid LEB');
    }
    while (offset < bytes.length) {
        const start = offset, id = bytes[offset++], payloadBytes = leb(), framingBytes = offset - start;
        assert(offset + payloadBytes <= bytes.length, 'truncated section');
        sections.push({ id, name: names[id] ?? `section-${id}`, payloadBytes, framingBytes, totalBytes: payloadBytes + framingBytes }); offset += payloadBytes;
    }
    return sections;
}
export function summarizeTraffic(records) {
    const unique = new Map();
    for (const record of records) {
        const previous = unique.get(record.path);
        assert(!previous || previous.sha256 === record.sha256, `response drift: ${record.path}`);
        unique.set(record.path, record);
    }
    return { requests: records.length, decodedBodyBytes: records.reduce((n,r) => n + r.bodyBytes,0),
        uniqueDecodedBodyBytes: [...unique.values()].reduce((n,r) => n + r.bodyBytes,0),
        wireBodyBytes: records.every(r => Number.isSafeInteger(r.wireBodyBytes)) ? records.reduce((n,r) => n + r.wireBodyBytes,0) : null,
        uniqueWasm: [...unique.values()].filter(r => new URL(r.path, 'http://fixture.test').pathname.endsWith('.wasm')).map(r => ({ path:r.path, bytes:r.bodyBytes, sha256:r.sha256 })),
        wasmRequests: records.filter(r => new URL(r.path, 'http://fixture.test').pathname.endsWith('.wasm')).length };
}
async function walk(directory, accept) {
    const result = [];
    for (const entry of await readdir(resolve(root, directory), { withFileTypes: true })) {
        const path = `${directory}/${entry.name}`;
        if (entry.isDirectory()) result.push(...await walk(path, accept)); else if (accept(path)) result.push(path);
    }
    return result.sort();
}
const text = path => readFile(resolve(root, path), 'utf8');
function stringsFromVariable(source, file, name) {
    let values;
    function visit(node) {
        if (ts.isVariableDeclaration(node) && node.name.getText() === name) {
            let initializer = node.initializer;
            while (ts.isAsExpression(initializer) || ts.isSatisfiesExpression(initializer)) initializer = initializer.expression;
            assert(ts.isArrayLiteralExpression(initializer), `expected ${name} array`);
            values = initializer.elements.map(e => { assert(ts.isStringLiteral(e)); return e.text; });
        }
        ts.forEachChild(node, visit);
    }
    visit(syntax(source, file)); assert(values, `missing ${name}`); return values;
}
function closure(graph, start) {
    const paths = new Map([[start, [start]]]), queue = [start];
    for (const current of queue) for (const edge of graph.get(current)?.edges ?? []) {
        if (edge.kind !== 'static' || !edge.target || paths.has(edge.target)) continue;
        paths.set(edge.target, [...paths.get(current), edge.target]); queue.push(edge.target);
    }
    return paths;
}

export async function buildAudit() {
    const catalogPath = 'packages/cem-components/dist/catalog/cem.components.catalog.json';
    const catalog = JSON.parse(await text(catalogPath));
    assert.equal(catalog.components.length, catalog.$generated.componentCount);
    const tags = new Set(catalog.components.map(c => c.tag)); assert.equal(tags.size, catalog.components.length);
    const runtimePath = `${sourceRoot}lib/cem-elements.ts`, runtimeSource = await text(runtimePath);
    const registry = capabilityRegistry(runtimeSource, runtimePath);
    for (const entry of registry.values()) entry.source = relative(root, resolve(root, dirname(runtimePath), entry.source)).replace(/\.js$/, '.ts');
    const declared = await walk('packages/cem-components/src/components', p => p.endsWith('.xhtml'));
    assert.deepEqual(declared, catalog.components.map(c => c.implementation.source).sort(), 'catalog/declaration file-set drift');
    const components = await Promise.all(catalog.components.map(async entry => componentEvidence(entry, await text(entry.implementation.source), registry, tags)));
    const files = await walk(sourceRoot.slice(0,-1), p => p.endsWith('.ts') && !/\.(?:spec|stories|fixtures|test)\.ts$/.test(p));
    const fileSet = new Set(files), graph = new Map(), attributeReads = new Map(), inputHashes = [];
    for (const file of files) {
        const source = await text(file); inputHashes.push({ path: file, sha256: sha(source) });
        const evidence = importEvidence(source, file);
        for (const edge of evidence.edges) {
            const path = edge.specifier.startsWith('.') ? relative(root, resolve(root, dirname(file), edge.specifier)).replace(/\.js$/, '.ts') : null;
            edge.target = path && fileSet.has(path) ? path : null;
        }
        graph.set(file, evidence); attributeReads.set(file, behaviorAttributeReads(source, file));
    }
    for (const row of components) {
        inputHashes.push({ path: row.source, sha256: row.sha256 });
        assert(!row.behaviorSource || graph.has(row.behaviorSource), `missing behavior ${row.behaviorSource}`);
        row.behaviorStaticClosure = row.behaviorSource ? [...closure(graph, row.behaviorSource).keys()].sort() : [];
        row.behaviorAttributeReads = sorted(row.behaviorStaticClosure.flatMap(file => attributeReads.get(file) ?? []));
        row.modeSelectors = sorted([...row.modeSelectors, ...row.attributes.filter(a => /format|language|mode|type|src|href/.test(a)), ...row.behaviorAttributeReads.filter(a => selectors.has(a))]);
    }
    const facade = `${sourceRoot}lib/internal/runtime-support/native-runtime.ts`, facadeSource = await text(facade);
    const facadeEntrypoints = stringsFromVariable(facadeSource, facade, 'sharedEntrypoints').map(name => ({ name, classification: classifyNativeEntrypoint(name) }));
    const entryClosure = closure(graph, `${sourceRoot}index.ts`);
    assert(entryClosure.has(facade), 'expected current static full-runtime facade reachability');
    const binaryPath = 'packages/cem_ql/dist/wasm/cem_ql_bg.wasm', vendorPath = 'packages/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql_bg.wasm';
    const bytes = await readFile(resolve(root, binaryPath));
    assert.equal(sha(await readFile(resolve(root, vendorPath))), sha(bytes), 'stale vendored WASM');
    assert(WebAssembly.validate(bytes), 'invalid WASM');
    const module = new WebAssembly.Module(bytes), optimization = JSON.parse(await text('dist/reports/wasm-optimization/cem-ql.json'));
    assert.equal(optimization.outputSha256, sha(bytes), 'stale optimizer evidence');
    assert.equal(optimization.outputBytes, bytes.length);
    assert.equal(optimization.imports, WebAssembly.Module.imports(module).length);
    assert.equal(optimization.exports, WebAssembly.Module.exports(module).length);
    const rustFiles = [...await walk('packages/cem_ql/src', p => p.endsWith('.rs')), ...await walk('packages/cem_ml/src', p => p.endsWith('.rs'))];
    const nativeDeclarations = [], moduleRoots = [], embeddedInputs = [];
    for (const file of rustFiles) {
        const source = await text(file); inputHashes.push({ path: file, sha256: sha(source) });
        for (const match of source.matchAll(/\binclude_(?:str|bytes)!\(\s*"([^"]+)"\s*\)/g)) {
            const path = relative(root, resolve(root, dirname(file), match[1]));
            assert(!path.startsWith('..'), `embedded input outside repository: ${path}`);
            embeddedInputs.push({ owner:file, path });
            inputHashes.push({ path, sha256:sha(await readFile(resolve(root,path))) });
        }
        for (const match of source.matchAll(/#\[[^\]]*\bwasm_bindgen[^\]]*\bjs_name\s*=\s*(?:"([^"]+)"|([\w]+))[^\]]*\]/g)) {
            nativeDeclarations.push({ name: match[1] ?? match[2], source: file, line: source.slice(0,match.index).split('\n').length });
        }
        if (file.endsWith('/lib.rs')) for (const match of source.matchAll(/((?:^#\[[^\n]+\]\r?\n)*)^pub(?:\(crate\))?\s+mod\s+(\w+)\s*;/gm)) {
            moduleRoots.push({ crate: file, module: match[2], attributes: match[1].trim() || null });
        }
    }
    for (const path of ['Cargo.lock','Cargo.toml','packages/cem_ql/Cargo.toml','packages/cem_ml/Cargo.toml',catalogPath,'tools/scripts/build-cem-ql-wasm.mjs','tools/scripts/wasm-opt.mjs','tools/scripts/audit-components-wasm.mjs',...catalog.$generated.canonicalSources,...catalog.$generated.evidenceSources]) inputHashes.push({ path, sha256: sha(await readFile(resolve(root,path))) });
    const hashes = [...new Map(inputHashes.map(record => [record.path,record])).values()].sort((a,b) => a.path.localeCompare(b.path));
    const cargoTree = execFileSync('cargo', ['tree','--locked','--offline','--package','cem-ql','--target','wasm32-unknown-unknown','--edges','normal','--prefix','none','--format','{p} features=[{f}]'], { cwd: root, encoding: 'utf8' });
    return {
        schemaVersion: 1, scope: 'current-full-component-runtime-audit', sourceDigest: sha(JSON.stringify(hashes)), inputHashes:hashes,
        evidenceLimits: ['Metadata/import edges are conservative source evidence, not a complete Rust call graph or proof of removability.',
            'Named function libraries, runtime providers, slots and computed imports require native demand/handoff proof.',
            'Literal native includes are hashed; generated/computed includes and source scanners are not a complete compiler build-input manifest.',
            'WASM code/data sections are measured bytes, not attribution to a crate, language or removable source pack.',
            'Audited source hashes and artifact/optimizer hashes are distinct evidence; this script is not a complete reproducible-build attestation.',
            'No reduced profile exists yet; native live/linear memory and browser costs must be measured separately.'],
        components, capabilities: [...registry].map(([name, value]) => ({ name, ...value })),
        javascript: { sources: graph.size, staticEntryClosure: entryClosure.size, nativeFacadeWitness: entryClosure.get(facade),
            unresolvedDynamicImports: [...graph.values()].reduce((n,g) => n + g.unresolvedDynamicImports,0),
            graph: [...graph].map(([source, value]) => ({ source, ...value })), facadeEntrypoints },
        native: { cargoTree, moduleRoots, embeddedInputs, explicitWasmDeclarations: nativeDeclarations },
        binary: { path: binaryPath, vendorPath, sha256: sha(bytes), rawBytes: bytes.length,
            gzip9Bytes: gzipSync(bytes,{level:9}).length,
            brotli9Bytes: brotliCompressSync(bytes,{params:{[constants.BROTLI_PARAM_QUALITY]:9}}).length,
            sections: wasmSections(bytes), imports: WebAssembly.Module.imports(module), exports: WebAssembly.Module.exports(module), optimization },
        toolchain: { node: process.version, rustc: execFileSync('rustc',['--version'],{encoding:'utf8'}).trim(),
            wasmBindgen: execFileSync('wasm-bindgen',['--version'],{encoding:'utf8'}).trim(), binaryen: optimization.binaryen },
    };
}
export function renderAudit(report, prefix = '../') {
    const mib = n => (n / 1048576).toFixed(2), link = path => `[${path}](${prefix}${path})`;
    const counts = new Map(); for (const item of report.javascript.facadeEntrypoints) counts.set(item.classification,(counts.get(item.classification) ?? 0) + 1);
    return `# Component WASM capability, dependency and size audit\n\nGenerated by \`node tools/scripts/audit-components-wasm.mjs --write-doc\`.\nThis audits the current full runtime; it does not implement or measure a minimal\nprofile. Source digest: \`${report.sourceDigest}\`. Machine evidence and exact\nsource hashes are in \`dist/reports/components-wasm/audit.json\`.\n\n## Measured binary\n\n| Measure | Bytes | MiB |\n| --- | ---: | ---: |\n${[['Raw',report.binary.rawBytes],['Gzip level 9',report.binary.gzip9Bytes],['Brotli quality 9',report.binary.brotli9Bytes]].map(([label,n]) => `| ${label} | ${n.toLocaleString('en-US')} | ${mib(n)} |`).join('\n')}\n\nSHA-256: \`${report.binary.sha256}\`. Canonical and vendored copies match.\nThe matched Binaryen ${report.toolchain.binaryen} -Oz report preserves the public\ninterface: ${report.binary.imports.length} imports and ${report.binary.exports.length} exports.\nNative explicit \`js_name\` declarations: ${report.native.explicitWasmDeclarations.length}; these are\nsource declarations across cfgs, not the complete linked export inventory.\n\n| Section | Payload bytes | Framing bytes |\n| --- | ---: | ---: |\n${report.binary.sections.map(s => `| ${s.name} | ${s.payloadBytes.toLocaleString('en-US')} | ${s.framingBytes} |`).join('\n')}\n\nThe code section is ${(100 * report.binary.sections.find(s => s.name === 'code').payloadBytes / report.binary.rawBytes).toFixed(1)}% of the file; the data section is ${(100 * report.binary.sections.find(s => s.name === 'data').payloadBytes / report.binary.rawBytes).toFixed(1)}%.\nSections plus the eight-byte header account for the entire file. Code/data\nbytes cannot be attributed to individual languages from this stripped binary.\nCompression is a file measurement, not evidence of HTTP encoding or memory use.\n\n## Dependency roots and constraints\n\n- ${report.components.length} catalog declarations match their exact source file set;\n  ${report.capabilities.length} registered shared behaviors are resolved from TypeScript import metadata.\n- The public entry has ${report.javascript.staticEntryClosure} conservative static source dependencies.\n  The native facade is reached through:\n${report.javascript.nativeFacadeWitness.map(p => `  - ${link(p)}`).join('\n')}\n- The shared facade binds ${report.javascript.facadeEntrypoints.length} native entry points:\n${[...counts].sort().map(([k,n]) => `  - ${k}: ${n}`).join('\n')}\n- ${report.javascript.unresolvedDynamicImports} computed imports remain unresolvable by this static audit.\n  Literal dynamic imports and worker assets are recorded separately from static\n  edges; generated script strings and user-supplied providers need runtime proof.\n- ${report.native.embeddedInputs.length} literal native include references are recorded and their files hashed.\n  Cfgs, generated inputs, macro expansion and linked-byte attribution remain separate.\n- Cargo selected-package normal-dependency evidence is recorded below. It is\n  a build dependency/feature graph, not a linked-symbol size allocation.\n\n### Required split boundaries\n\n1. **CSV is not freely optional today.** ${link("packages/cem-components/src/components/cem-data-view/cem-data-view.xhtml")}\n   calls \`data:read(value, format)\`; the ${link("packages/cem_ql/src/eval/data.rs")}\n   evaluator delegates to ${link("packages/cem_ml/src/import.rs")} for JSON/CSV.\n   Keep required importers in core until native requirement/yield and typed tree\n   handoff work. \`cem-table\` itself is structural markup/slots, not a CSV parser.\n2. **Syntax coloring is a component capability.** \`plaintext-editor\` calls the\n   shared native highlighting endpoint. Deferring language tooling must retain\n   typing, selection/IME, highlighting correctness and original diagnostics.\n3. **The entry/facade and native exports need profile-specific roots.** Hiding\n   wrappers or moving schema bodies does not remove compiled implementations.\n   Legacy/XSLT and XPath function-library paths are present in the shared facade.\n4. **Controls, CSS, typed data and recovery are mandatory.** Memory, admission,\n   inventory and checkpoint exports cannot be classified as optional debug code.\n5. **No component removal proof is implied.** Slots, providers, expressions and\n   external templates remain dynamic extension points across the catalog.\n\n## Per-component capability and mode evidence\n\nEvery row requires native CEM-ML/CEM-QL rendering, CSS/scoping, typed value/source\nownership and admission/recovery controls. Selectors below are authored open\nattributes, not an exhaustive enumeration of their runtime values. Exact literal\nattributes/calls, capability precedence, direct component references and each\nbehavior's static import closure/literal attribute reads are retained in JSON.
These reads exclude computed/property-based attributes and are not a complete
behavioral mode enumeration. Dynamic\npayload/provider/template requirements apply even when the selector cell is empty.\n\n| Component | Intrinsic behavior | Mode/identity selectors | Conditional candidate; not removal proof |\n| --- | --- | --- | --- |\n${report.components.map(c => `| ${c.tag} | ${c.capability || 'none (shared render core)'} | ${c.modeSelectors.join(', ') || 'none authored'} | ${c.conditionalCandidates.join(', ') || 'provider/template demand'} |`).join('\n')}\n\n## Cargo normal-dependency/feature evidence\n\n\`cargo tree --locked --offline --package cem-ql --target wasm32-unknown-unknown --edges normal --prefix none --format '{p} features=[{f}]'\`\n\nWorkspace paths are normalized below; exact command output is in the JSON report.\n\n\`\`\`text\n${report.native.cargoTree.replaceAll(root,'<workspace>').trim()}\n\`\`\`\n\n## Limits and next proof\n\n${report.evidenceLimits.map(item => `- ${item}`).join('\n')}\n\nNo new JavaScript UI behavior, parser or component implementation is introduced.\nThe current full-runtime defaults, credential recovery refusal and 4 MiB\ncheckpoint/deadline contracts remain unchanged. Continue with the measured\nbrowser baseline and native module/handoff prototype in the\n[profile checklist](${prefix}docs/todo.md#component-application-wasm-profile-separate-target).\n`;
}

export function renderBrowserBaseline(report) {
    const mib = n => (n / 1048576).toFixed(2);
    const sample = (record, timing, field) => {
        const value = record[timing].native;
        return value.status === 'available' ? mib(value.sample.sample[field]) : 'unavailable';
    };
    return `# Component runtime browser baseline\n\nReference observations from the complete current runtime, generated by\n\`node tools/scripts/measure-components-wasm.mjs --write-doc\`. There is no\nminimal profile to compare yet. Machine response/phase records and numeric memory\nsamples are in \`dist/reports/components-wasm/browser-baseline.json\`.\n\nCore SHA-256: \`${report.coreSha256}\`. Chromium: ${report.chromium}; Node: ${report.node}.\n\n## Method and limits\n\n- Six isolated contexts: controls, native JSON/CSV data view and a source editor\n  in worker and explicitly selected fallback modes. No Studio or demo-viewer UI\n  bootstrap, saved user storage or credentials participate.\n- One observation per workload/mode in a shared Chromium process. Context/network\n  state is fresh; process/compiled-code caches can be reused. Timings include\n  automation overhead and are neither distributions nor performance comparisons.\n- Readiness is visible control readiness; first use is successful form editing,\n  JSON/CSV value rendering, or native CEM-ML paint with unchanged text/selection.\n  Phase boundaries wait for rendering and network idle; request phases use their\n  start time. Subsequent cost is already present in the full initial engine.\n- This uses the assembled local demo's matched, deduplicated adapters and no-store\n  server. Body size is decoded bytes; reported wire body size excludes headers.\n  Body hashes come from validated served files/fixtures and response lengths so\n  inspector-cache eviction does not require retaining response payloads. All\n  published responses match their deployment inventory hashes and all catalog\n  declaration bytes match the audited workspace. This\n  does not measure compressed CDN delivery or isolated npm consumer bundling.\n- Native samples describe only the selected fallback facade. Active worker and\n  aggregate browser memory are unavailable through these current public hooks.\n  CDP main-thread JS heap counters are recorded separately, never added to WASM\n  capacity or presented as total memory.\n\n## Observed network and first-use behavior\n\n| Workload | Mode | Ready ms | First use ms | Cold decoded MiB | Cold unique MiB | WASM requests / unique files | First-use requests |\n| --- | --- | ---: | ---: | ---: | ---: | --- | ---: |\n${report.results.map(r => `| ${r.kind} | ${r.mode} | ${Math.round(r.readyMs)} | ${Math.round(r.firstUseMs)} | ${mib(r.phases.cold.decodedBodyBytes)} | ${mib(r.phases.cold.uniqueDecodedBodyBytes)} | ${r.traffic.wasmRequests} / ${r.traffic.uniqueWasm.length} | ${r.phases['first-use'].requests} |`).join('\n')}\n\nOne unique binary is not one transfer or one heap. These worker observations\nload the same full WASM twice under no-store delivery; fallback loads it once.\nReady/success worker counters prove that default worker jobs actually execute.\nEach observed binary matches the canonical hash above. A future profile needs\nseparate checks for core byte size, duplicate acquisition/initialization and\noptional first-use cost. No optional language/tooling WASM was loaded on first\nuse here: those implementations are already in the full binary.\n\n## Numeric memory samples (MiB)\n\n| Workload | Mode | Main-thread JS heap at ready | Selected native live before / after use | Selected native capacity before / after use |\n| --- | --- | ---: | --- | --- |\n${report.results.map(r => `| ${r.kind} | ${r.mode} | ${mib(r.coldMemory.mainThreadJsHeapUsedBytes)} | ${sample(r,'coldMemory','allocatorLiveBytes')} / ${sample(r,'afterMemory','allocatorLiveBytes')} | ${sample(r,'coldMemory','linearMemoryBytes')} / ${sample(r,'afterMemory','linearMemoryBytes')} |`).join('\n')}\n\nUnavailable counters are not zero. Do not infer a memory saving, OOM threshold\nor checkpoint size from these file sizes or partial heap samples. Multi-run\nlatency, active-worker/aggregate memory and an all-capabilities/offline total\nremain follow-up baseline work in the [profile checklist](todo.md#component-application-wasm-profile-separate-target).\n\nThe fixtures add no production UI behavior or parsing. Existing component code\nruns in isolated storage. Credential recovery refusal, manual recovery and the\n4 MiB checkpoint/deadline contracts remain unchanged.\n`;
}

async function main() {
    assert(process.argv.slice(2).every(a => ['--write-doc','--check'].includes(a)), 'Usage: node tools/scripts/audit-components-wasm.mjs [--write-doc|--check]');
    const report = await buildAudit(), output = resolve(root,'dist/reports/components-wasm');
    await mkdir(output,{recursive:true});
    await writeFile(join(output,'audit.json'),`${JSON.stringify(report,null,2)}\n`);
    const markdown = renderAudit(report);
    if (process.argv.includes('--check')) assert.equal(await text('docs/cem-components-wasm-audit.md'), markdown, 'audit document drift; regenerate --write-doc');
    if (process.argv.includes('--write-doc')) await writeFile(resolve(root,'docs/cem-components-wasm-audit.md'),markdown);
    console.log(`Audited ${report.components.length} components, ${report.capabilities.length} shared capabilities, ${report.binary.rawBytes} WASM bytes; source digest ${report.sourceDigest}.`);
}
if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) await main();
