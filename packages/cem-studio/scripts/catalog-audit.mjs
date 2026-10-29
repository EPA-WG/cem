/** Build-time audit of named catalog/documentation control records, not source parsing. */
export function auditCatalog({ projects, generatedProjects = [], registry, operations, workflows }) {
    const fail = (code, detail) => { throw new Error(`cem.catalog.${code}: ${detail}`); };
    const unique = (values, label) => {
        if (new Set(values).size !== values.length) fail('duplicate', label);
    };
    const requireDocumentation = (id, resources, entries, text, servedFiles = []) => {
        const document = resources.find(resource => resource.path.toLowerCase() === 'readme.md');
        if (document.contentType?.split(';')[0].trim().toLowerCase() !== 'text/markdown'
            || document.schema && document.schema !== 'https://cem.dev/ns/data/markdown/1') fail('readme_identity', id);
        if (typeof text !== 'string' || !text.trim()) fail('readme', `${id} has no README text`);
        if (!/^# [^\n]+\r?$/mu.test(text.split('\n')[0])) fail('readme_title', id);
        if (!/^# [^\n]+\r?\n\s*\r?\n[^#\s][^\n]*(?:\n[^#\n][^\n]*)?(?:\r?\n|$)/u.test(text)) fail('readme_purpose', id);
        // Lint the explicit authored documentation convention; this is not a
        // Markdown importer or a runtime source parser.
        const essential = text.split(/\r?\n## Essential files\r?\n/u)[1]?.split(/\r?\n## /u)[0] ?? '';
        const links = [...essential.matchAll(/\[([^\]\n]+)\]\(([^)\s]+)\)/gu)];
        if (!links.length) fail('readme_files', id);
        const linked = new Set();
        for (const match of links) {
            const href = match[2];
            let path;
            try {
                const url = new URL(href, 'https://cem-doc.invalid/README.md');
                if (url.origin === 'https://cem-doc.invalid' && !url.search && !href.startsWith('/') && !/^[a-z][a-z0-9+.-]*:/iu.test(href)) path = decodeURIComponent(url.pathname.slice(1));
            } catch { /* malformed references fail the same closed inventory check */ }
            if (!resources.some(resource => resource.path === path && resource !== document)) fail('readme_file_reference', `${id}: ${href}`);
            const remainder = essential.slice(match.index + match[0].length).split('\n')[0];
            if (!/\p{L}/u.test(remainder)) fail('readme_file_purpose', `${id}: ${href}`);
            linked.add(path);
        }
        if (['schema-examples', 'feature-tour'].includes(id)) for (const resource of resources.filter(resource => resource.id.startsWith('example-'))) {
            if (!linked.has(resource.path)) fail('showcase_file_documentation', `${id}: ${resource.path}`);
        }
        const codes = new Set([...text.matchAll(/`([^`\n]+)`/g)].map(match => match[1]));
        for (const file of servedFiles) if (!linked.has(file.path) || !codes.has(file.asset)) {
            fail('served_file_documentation', `${id}: ${file.path} → ${file.asset}`);
        }
        for (const entry of entries) if (!codes.has(entry.id)) fail('configuration_documentation', `${id}/${entry.id}`);
        for (const resource of resources.filter(resource => resource.path.toLowerCase() !== 'readme.md')) {
            for (const identity of [resource.contentType, resource.schema].filter(Boolean)) {
                if (!codes.has(identity)) fail('identity_documentation', `${id}/${resource.id}: ${identity}`);
            }
        }
    };
    const available = new Set(operations.filter(item => item.availability === 'available').map(item => item.operation));
    const commandCounts = new Map();
    const countCommand = command => commandCounts.set(command, (commandCounts.get(command) ?? 0) + 1);
    const resources = [], inputs = [], dependencies = [], expected = [], projectRows = [];
    const resourceReferences = ['expectedOutput', 'expectedOutputBytes', 'expectedSemanticContract',
        'expectedNativeNodes', 'expectedNativeItems', 'expectedNativeRanges'];
    unique(projects.map(({ definition }) => definition.id), 'project IDs');
    const expectedWorkflows = new Map();
    for (const { definition, readme, missingSources = [], bundleResourceCount } of projects) {
        const { id, sources, configurations } = definition;
        unique(sources.map(source => source.id), `${id} source IDs`);
        unique(sources.map(source => source.path), `${id} project paths`);
        unique(configurations.map(configuration => configuration.id), `${id} configuration IDs`);
        if (missingSources.length) fail('source_missing', `${id}: ${missingSources.join(', ')}`);
        const byId = new Map(sources.map(source => [source.id, source]));
        const documentation = sources.filter(source => source.path.toLowerCase() === 'readme.md');
        if (documentation.length !== 1 || documentation[0].id !== readme?.id) fail('readme_resource', id);
        requireDocumentation(id, sources, configurations, readme.text);
        const reference = (resourceId, configuration, field) => {
            const resource = byId.get(resourceId);
            if (!resource) fail('resource_reference', `${id}/${configuration.id} ${field}: ${resourceId}`);
            return { project: id, configuration: configuration.id, resource: resource.id,
                contentType: resource.contentType, schema: resource.schema ?? null };
        };
        for (const configuration of configurations) {
            const command = configuration.argv?.[0];
            if (!available.has(command)) fail('operation', `${id}/${configuration.id}: ${command}`);
            countCommand(command);
            const input = reference(configuration.source, configuration, 'source');
            inputs.push(input);
            for (const resourceId of configuration.dependencies ?? []) dependencies.push(reference(resourceId, configuration, 'dependency'));
            const fixtures = resourceReferences.filter(key => configuration[key] !== undefined).map(key => [key, configuration[key]]);
            fixtures.push(...Object.entries(configuration.expectedGraphOutputs ?? {}));
            for (const asset of configuration.expectedNativeAssets ?? []) {
                for (const key of ['source', 'sourceMap', 'expectedOutput']) if (asset[key]) fixtures.push([key, asset[key]]);
            }
            for (const pair of configuration.expectedGraphFailures ?? []) for (const value of pair) fixtures.push(['expectedGraphFailures', value]);
            for (const refusal of configuration.expectedGraphRefusals ?? []) {
                for (const key of ['source', 'replacement']) fixtures.push([key, refusal[key]]);
            }
            for (const [field, resourceId] of fixtures) expected.push({ ...reference(resourceId, configuration, field), field });
            expectedWorkflows.set(`${id}/${configuration.id}`, new Set([configuration.source, ...(configuration.dependencies ?? [])]));
        }
        for (const source of sources) resources.push({ project: id, resource: source.id, contentType: source.contentType, schema: source.schema ?? null });
        projectRows.push({ id, name: definition.name, readme: documentation[0].source,
            sourceCount: sources.length, configurationCount: configurations.length,
            storedResources: bundleResourceCount ?? sources.length + configurations.length,
            repairCount: configurations.filter(configuration => configuration.repair).length,
            commands: [...new Set(configurations.map(configuration => configuration.argv[0]))].sort() });
    }
    const generatedRows = [];
    for (const { id, project, readme, commands = [], servedFiles = [] } of generatedProjects) {
        const docs = project.resources.filter(resource => resource.path.toLowerCase() === 'readme.md');
        if (docs.length !== 1 || docs[0].id !== readme?.id) fail('readme_resource', id);
        const entries = project.entries.filter(entry => entry.kind !== 'subproject');
        unique(entries.map(entry => entry.id), `${id} generated entry IDs`);
        unique(commands.map(command => command.entryId), `${id} generated command entry IDs`);
        requireDocumentation(id, project.resources, entries, readme.text, servedFiles);
        for (const { entryId, command, source } of commands) {
            const entry = entries.find(entry => entry.id === entryId);
            if (!entry?.resourceIds.includes(source)) fail('generated_reference', `${id}/${entryId}`);
            if (!available.has(command)) fail('operation', `${id}/${entryId}: ${command}`);
            if (id === 'schema-examples') {
                countCommand(command);
                const resource = project.resources.find(resource => resource.id === source);
                inputs.push({ project: id, configuration: entryId, resource: source, contentType: resource.contentType, schema: resource.schema ?? null });
                // The existing inventory names portable workbenches; package validations have their own registry matrix.
                if (entryId.startsWith('workbench-')) expectedWorkflows.set(`${id}/${entryId}`, new Set(entry.resourceIds));
            }
        }
        if (commands.length !== entries.length) fail('generated_commands', id);
        generatedRows.push({ id, readme: readme.source ?? null, resourceCount: project.resources.length, configurationCount: entries.length });
    }
    const seen = new Set();
    for (const { project, resource, configuration } of workflows) {
        const key = `${project}/${configuration}`;
        if (!expectedWorkflows.get(key)?.has(resource)) fail('workflow_reference', `${key}: ${resource}`);
        if (seen.has(key)) fail('workflow_duplicate', key);
        seen.add(key);
    }
    for (const key of expectedWorkflows.keys()) if (!seen.has(key)) fail('workflow_missing', key);

    // This is inventory grouping, not an importer, command dispatcher or admission allowlist.
    const family = schema => schema === 'https://cem.dev/ns/query/cem-ql/1#expression' ? 'https://cem.dev/ns/query/cem-ql/1' : schema;
    const mediaType = value => value?.split(';')[0].trim().toLowerCase();
    const matches = (rows, item) => rows.filter(row => family(row.schema) === item.schemaUri);
    const packages = registry.map(item => {
        const primary = matches(inputs, item), declared = matches(resources, item);
        return { packageId: item.packageId, schema: item.schemaUri,
            authoredDeclaredProjects: [...new Set(declared.map(row => row.project))].sort(),
            primaryInputProjects: [...new Set(primary.map(row => row.project))].sort(),
            declaredDependencyProjects: [...new Set(matches(dependencies, item).map(row => row.project))].sort(),
            expectedFixtureProjects: [...new Set(matches(expected, item).map(row => row.project))].sort(),
            selectors: item.contentTypes.map(type => ({ contentType: type.value,
                declaredPrimaryInputs: primary.filter(row => mediaType(row.contentType) === mediaType(type.value)).map(row => `${row.project}/${row.configuration}`) })) };
    });
    const authoredConfigurations = projectRows.reduce((count, row) => count + row.configurationCount, 0);
    const generatedCatalog = generatedRows.find(row => row.id === 'schema-examples');
    return { schemaVersion: 1,
        scope: 'Declared catalog/documentation coverage; native/worker/deployed verification and current resource admission remain separate evidence.',
        counts: { authoredProjects: projectRows.length, authoredSources: resources.length,
            authoredConfigurations,
            catalogProjects: projectRows.length + (generatedCatalog ? 1 : 0),
            catalogConfigurations: authoredConfigurations + (generatedCatalog?.configurationCount ?? 0),
            catalogStoredResources: projectRows.reduce((count, row) => count + row.storedResources, 0) + (generatedCatalog?.resourceCount ?? 0),
            starterResources: generatedRows.find(row => row.id === 'starter')?.resourceCount ?? 0,
            documentedWorkflows: seen.size, registeredPackages: packages.length,
            registeredSelectorClaims: packages.reduce((count, row) => count + row.selectors.length, 0) },
        operations: operations.map(item => ({ ...item, declaredConfigurations: commandCounts.get(item.operation) ?? 0 })),
        projects: projectRows, generatedProjects: generatedRows, packages,
        declaredPrimaryInputs: inputs, declaredDependencies: dependencies, declaredExpectedFixtures: expected,
        unregisteredSourceSchemas: [...new Set(resources.map(row => row.schema).filter(schema => schema && !registry.some(item => item.schemaUri === family(schema))))].sort() };
}

export function inventoryWorkflows(markdown) {
    const body = markdown.split('<!-- existing-samples:start -->')[1]?.split('<!-- existing-samples:end -->')[0];
    if (!body) throw new Error('cem.catalog.workflow_inventory: missing existing-samples markers');
    return body.split('\n').filter(line => line.startsWith('| `')).map(line => {
        const cells = line.split('|').slice(1);
        return Object.fromEntries(['project', 'resource', 'configuration'].map((key, index) => [key, cells[index].match(/`([^`]+)`/)?.[1]]));
    });
}

export function catalogCoverageMarkdown(report) {
    const code = value => `\`${value}\``;
    const projects = report.projects.map(row => `| ${code(row.id)} | [README](../${row.readme}) | ${row.sourceCount} | ${row.configurationCount} | ${row.repairCount} | ${row.commands.map(code).join(', ')} |`);
    const generatedSection = `## Generated projects\n\nThe selectable catalog has **${report.counts.catalogProjects} projects** with **${report.counts.catalogConfigurations} declared configurations**\nand **${report.counts.catalogStoredResources} stored resources**. The starter adds ${report.counts.starterResources} resources.\nThe legacy Feature Tour seed shares the Schema examples workflows and is not\ncounted twice. Declared configurations are not a uniform UI execution guarantee.\n\n| Generated project | Instructions | Stored resources | Declared configurations |\n| --- | --- | --- | --- |\n${report.generatedProjects.map(row => `| ${code(row.id)} | ${row.readme ? `[README](../${row.readme})` : '—'} | ${row.resourceCount} | ${row.configurationCount} |`).join('\n')}`;
    const packages = report.packages.map(row => `| ${code(row.packageId)} | ${row.authoredDeclaredProjects.map(code).join(', ') || '—'} | ${row.primaryInputProjects.map(code).join(', ') || '—'} | ${row.selectors.filter(type => type.declaredPrimaryInputs.length).length}/${row.selectors.length} |`);
    const gaps = report.packages.flatMap(row => row.selectors.filter(type => !type.declaredPrimaryInputs.length).map(type => `| ${code(row.packageId)} | ${code(type.contentType)} |`));
    return `# Studio catalog documentation and declared capability coverage\n\nGenerated by the catalog audit from authored project manifests, generated bundles,\nnative registry/capability metadata and the existing workflow inventory.\nRun \`yarn nx run @epa-wg/cem-studio:verify:catalog-audit\` after changing them.\n\n${report.scope}\n\nThere are **${report.counts.authoredProjects} authored projects**, **${report.counts.authoredSources} authored source resources**\nand **${report.counts.authoredConfigurations} authored configurations**. The inventory names\n**${report.counts.documentedWorkflows} workflows**, including five generated portable workbenches;\n${report.counts.registeredPackages} generated package validations are represented by the native package matrix.\nThe starter owns no configurations. Source resources include reference fixtures\nand documentation: their presence does not establish executable format support.\n\n## Projects and documentation\n\n| Project | Instructions | Authored resources | Configurations | Repair configurations | Authored commands |\n| --- | --- | --- | --- | --- | --- |\n${projects.join('\n')}\n\n${generatedSection}\n\n## Native operation surface\n\nCounts below describe authored argv commands and generated package/workbench\ncommands, not entry display kinds or a uniform pass matrix. \`run-check\` uses\n\`validate --config\`; it does not demonstrate native \`check\`. Service capability\nnegotiation does not require a source-project configuration.\n\n| Operation | Browser availability | Declared configurations |\n| --- | --- | --- |\n${report.operations.map(row => `| ${code(row.operation)} | ${code(row.availability)} | ${row.declaredConfigurations} |`).join('\n')}\n\n## Registered families and declared input selectors\n\nPrimary inputs name a configuration's source resource, including generated\nmanifest-selected validation cases. Dependencies and expected fixtures are\nreported separately in the JSON audit; packaging a dependency does not prove\nthat a native command imports it. The CEM-QL expression mode belongs to its\nexisting package. Native control schemas outside this registry remain valid\nauthored identities, not a second package registry.\n\n| Registered package | Authored resource declarations | Primary input projects | Primary selector claims represented |\n| --- | --- | --- | --- |\n${packages.join('\n')}\n\n## Selectors without a declared primary-input case\n\nThese are coverage gaps, not a new dispatch/activation allowlist. Some selectors\nare result formats, compiled binaries, or deliberately refused external-entity\ninputs. Native command parsing and current resource admission remain authoritative.\nGenerated binary validations exercise native header checks; read-only binary\nresources remain non-runnable in Studio. Requested exports are not input support.\n\n| Registered package | Selector |\n| --- | --- |\n${gaps.join('\n')}\n\n## Evidence and preserved limits\n\nThe audit checks README/resource/configuration references and complete workflow\ncoverage. Native grammar, data imports, validation and execution stay in Rust;\nthis tool does not parse project data or interpret native ASTs. README prose is\nreviewed against the native fixtures and [format limits](studio-content-types.md).\n\nThe native Feature Tour verifier records all declared configuration executions.\nNative Inspect verifies its original output and exit separately from a companion\ndiagnostic view; that view does not prove the retained AST failed or was clean.\nExact native Parse and companion-view limits remain explicit in acceptance evidence.\nDeclared configurations therefore do not imply uniform clean native execution.\nParse/Inspect coordinate presentation is distinct from owned source-map\npublication; explicit projection JSON is an export boundary, never an internal\nAST handoff. AST JSON shape, output-only adapters and header-only validation\nlimits remain documented in the relevant project READMEs and [TODO](todo.md).\n\nGenerated Schema examples, Feature Tour and starter READMEs are ordinary Markdown\nresources. Existing project/source IDs and starter default selection remain\nstable; cloned documentation is independent of catalog originals. Reusable UI\nstays declarative in shared CEM components. Entered passwords/one-time codes\nblock recovery until explicitly cleared; automatic whole-Studio recovery remains\ndisabled. Saved projects and drafts are not cleared by this audit.\n`;
}
