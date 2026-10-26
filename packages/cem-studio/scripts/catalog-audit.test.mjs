import assert from 'node:assert/strict';
import { test } from 'node:test';
import { auditCatalog, inventoryWorkflows, catalogCoverageMarkdown } from './catalog-audit.mjs';
import { createExampleProjects } from './example-projects.mjs';
import { parseCemStudioProjectBundle } from '../src/file-system-provider.js';
import { fileURLToPath } from 'node:url';

test('preserves caller namespace validation argv as an authored native command', async () => {
    const projects = await createExampleProjects(fileURLToPath(new URL('../../..', import.meta.url)), '0.1.0');
    const project = projects.find(item => item.record.id === 'xpath-catalog');
    const bundle = parseCemStudioProjectBundle(project.archive);
    const entry = bundle.project.entries.find(item => item.id === 'validate-join');
    const command = bundle.project.resources.find(item => item.id === entry.runConfigResourceId);
    assert.equal(command.schema, 'https://cem.dev/ns/cli/command/1');
    const body = JSON.parse(new TextDecoder().decode(bundle.contents[command.id]));
    assert.deepEqual(body.argv, ['validate', 'queries/supplier-join.xpath', '--content-type', 'application/vnd.cem.xpath',
        '--schema', 'https://cem.dev/ns/query/xpath/1', '--namespace', 'c=urn:example:catalog', '--format', 'json', '--no-color']);
    const legacy = bundle.project.entries.find(item => item.id === 'validate-filter');
    assert.equal(bundle.project.resources.find(item => item.id === legacy.runConfigResourceId).schema, 'https://cem.dev/ns/cli/run-config/1');
});

function fixture() {
    return {
        projects: [{ definition: { id: 'demo', name: 'Demo', sources: [
            { id: 'input', path: 'data/input.xml', contentType: 'application/xml', schema: 'urn:registered:xml' },
            { id: 'expected', path: 'expected/output.json', contentType: 'application/json' },
            { id: 'readme', path: 'README.md', source: 'examples/projects/demo/README.md', contentType: 'text/markdown' },
        ], configurations: [{ id: 'inspect-source', source: 'input', kind: 'inspection', argv: ['inspect', '$source'], expectedOutput: 'expected' }] },
        readme: { id: 'readme', text: 'Use `inspect-source`. `application/xml` `urn:registered:xml` `application/json`. Source coordinates are not owned publication.' } }],
        registry: [{ packageId: 'xml', schemaUri: 'urn:registered:xml', contentTypes: [{ value: 'application/xml' }, { value: 'text/xml' }] }],
        operations: [{ operation: 'inspect', availability: 'available' }, { operation: 'bench', availability: 'unavailable' }],
        workflows: [{ project: 'demo', resource: 'input', configuration: 'inspect-source' }],
    };
}

test('reports declared inputs separately from expected fixture/dependency metadata and selector gaps', () => {
    const data = fixture(); data.projects[0].definition.configurations[0].dependencies = ['expected'];
    const report = auditCatalog(data);
    assert.equal(report.counts.authoredConfigurations, 1);
    assert.deepEqual(report.packages[0].primaryInputProjects, ['demo']);
    assert.deepEqual(report.packages[0].selectors[0].declaredPrimaryInputs, ['demo/inspect-source']);
    assert.deepEqual(report.packages[0].selectors[1].declaredPrimaryInputs, []);
    assert.equal(report.operations.find(row => row.operation === 'bench').declaredConfigurations, 0);
    assert.match(catalogCoverageMarkdown(report), /not entry display kinds or a uniform pass matrix/);
    assert.match(catalogCoverageMarkdown(report), /read-only binary\s+resources remain non-runnable/);
});

for (const [name, mutate, code] of [
    ['missing README resource', data => data.projects[0].definition.sources.pop(), 'readme_resource'],
    ['empty README', data => { data.projects[0].readme.text = ''; }, 'readme'],
    ['binary documentation metadata', data => { data.projects[0].definition.sources.at(-1).contentType = 'application/octet-stream'; }, 'readme_identity'],
    ['conflicting documentation schema', data => { data.projects[0].definition.sources.at(-1).schema = 'urn:registered:xml'; }, 'readme_identity'],
    ['prefix-only configuration mention', data => { data.projects[0].readme.text = data.projects[0].readme.text.replace('`inspect-source`', '`inspect-source-other`'); }, 'configuration_documentation'],
    ['missing source identity', data => { data.projects[0].readme.text = data.projects[0].readme.text.replace('urn:registered:xml', ''); }, 'identity_documentation'],
    ['identity suffix masquerading as a match', data => { data.projects[0].readme.text = data.projects[0].readme.text.replace('`application/xml`', '`application/xml-other`'); }, 'identity_documentation'],
    ['schema suffix masquerading as a match', data => { data.projects[0].readme.text = data.projects[0].readme.text.replace('`urn:registered:xml`', '`urn:registered:xml-old`'); }, 'identity_documentation'],
    ['missing source file', data => { data.projects[0].missingSources = ['data/input.xml']; }, 'source_missing'],
    ['dangling primary input', data => { data.projects[0].definition.configurations[0].source = 'absent'; }, 'resource_reference'],
    ['dangling dependency', data => { data.projects[0].definition.configurations[0].dependencies = ['absent']; }, 'resource_reference'],
    ['dangling binary golden', data => { data.projects[0].definition.configurations[0].expectedOutputBytes = 'absent'; }, 'resource_reference'],
    ['dangling graph golden', data => { data.projects[0].definition.configurations[0].expectedGraphOutputs = { 'generated/index.html': 'absent' }; }, 'resource_reference'],
    ['unavailable browser command', data => { data.projects[0].definition.configurations[0].argv = ['bench']; }, 'operation'],
    ['missing workflow row', data => { data.workflows = []; }, 'workflow_missing'],
    ['stale workflow resource', data => { data.workflows[0].resource = 'expected'; }, 'workflow_reference'],
    ['duplicate workflow', data => { data.workflows.push({ ...data.workflows[0] }); }, 'workflow_duplicate'],
    ['duplicate configuration', data => { data.projects[0].definition.configurations.push(data.projects[0].definition.configurations[0]); }, 'duplicate'],
]) test(`rejects ${name}`, () => {
    const data = fixture(); mutate(data);
    assert.throws(() => auditCatalog(data), new RegExp(`cem.catalog.${code}:`));
});

test('does not turn registry selectors into a source dispatch allowlist or infer command semantics from entry kind', () => {
    const data = fixture(); data.projects[0].definition.sources[0].contentType = 'application/cem';
    data.projects[0].definition.configurations[0].kind = 'conversion';
    data.projects[0].readme.text += ' `application/cem`';
    const report = auditCatalog(data);
    assert.equal(report.operations[0].declaredConfigurations, 1);
    assert.deepEqual(report.packages[0].selectors[0].declaredPrimaryInputs, []);
    data.projects[0].definition.sources[0].schema = 'urn:example:instance'; data.projects[0].readme.text += ' `urn:example:instance`';
    assert.deepEqual(auditCatalog(data).unregisteredSourceSchemas, ['urn:example:instance']);
});

test('groups an explicit MIME with parameters while retaining the original declared identity', () => {
    const data = fixture(); data.projects[0].definition.sources[0].contentType = 'application/xml; charset=utf-8';
    data.projects[0].readme.text += ' `application/xml; charset=utf-8`';
    const report = auditCatalog(data);
    assert.deepEqual(report.packages[0].selectors[0].declaredPrimaryInputs, ['demo/inspect-source']);
    assert.equal(report.declaredPrimaryInputs[0].contentType, 'application/xml; charset=utf-8');
});

test('generated documentation is required without counting compatibility seed commands twice', () => {
    const data = fixture();
    const generated = { id: 'feature-tour', project: { resources: [
        { id: 'input', path: 'data/input.xml', contentType: 'application/xml', schema: 'urn:registered:xml' },
        { id: 'readme', path: 'README.md', contentType: 'text/markdown' },
    ], entries: [{ id: 'workbench-inspect', kind: 'inspection', resourceIds: ['input'] }, { id: 'group', kind: 'subproject' }] },
    readme: { id: 'readme', text: '`workbench-inspect` `application/xml` `urn:registered:xml`' },
    commands: [{ entryId: 'workbench-inspect', command: 'inspect', source: 'input' }] };
    data.generatedProjects = [generated];
    assert.equal(auditCatalog(data).operations[0].declaredConfigurations, 1);
    generated.project.entries.push({ id: 'workbench-other', kind: 'inspection', resourceIds: ['input'] });
    generated.readme.text += ' `workbench-other`';
    generated.commands.push({ ...generated.commands[0] });
    assert.throws(() => auditCatalog(data), /cem.catalog.duplicate/);
    generated.commands.pop(); generated.project.entries.pop();
    generated.project.resources.pop(); assert.throws(() => auditCatalog(data), /cem.catalog.readme_resource/);
});

test('reads the explicit workflow inventory boundary without scanning unrelated prose', () => {
    const rows = inventoryWorkflows('Other `fake` references\n<!-- existing-samples:start -->\n| Project | Resource | Configuration |\n| --- | --- | --- |\n| `demo` | `input` | `inspect-source` |\n<!-- existing-samples:end -->');
    assert.deepEqual(rows, fixture().workflows);
    assert.throws(() => inventoryWorkflows('no table'), /cem.catalog.workflow_inventory/);
});
