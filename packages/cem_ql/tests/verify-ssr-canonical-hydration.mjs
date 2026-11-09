import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const temp = await mkdtemp(path.join(os.tmpdir(), 'cem-canonical-resume-'));
const source = path.join(temp, 'source.html');
const output = path.join(temp, 'rendered.html');
const builtin = path.join(root, 'packages/cem_ml_transform_cem_ql/templates/ssr-page.cemt');
const cli = path.join(root, `dist/target/cem_ml_cli/debug/cem-ml${process.platform === 'win32' ? '.exe' : ''}`);
const template = '{article | {$ datadom.slots.heading}}';
async function render(body, versions, extra = [], head = '') {
    return renderSource(source, `<!doctype html><html><head><title>Canonical SSR</title>${head}</head><body>${body}</body></html>`, versions, extra);
}
async function renderSource(source, content, versions, extra = []) {
    await writeFile(source, content);
    execFileSync(cli, ['transform', source, '--template', builtin, '--template-content-type', 'text/cem-ml',
        '--template-entrypoint', 'ssr:render-page', '--to-content-type', 'text/html', '--out', output,
        ...['page-url=https://ssr.example.test/', `consumer-versions=${versions}`, 'max-depth=8',
            'max-instances=8', 'max-render-passes=8', 'max-resources=16', 'max-bytes=1048576',
            'wall-ms=60000', 'cpu-ms=10000'].flatMap(value => ['--param', value]), ...extra]);
    return readFile(output, 'utf8');
}
let server, browser;
try {
    const html = await render(`<cem-element tag="x-resume" version="1.2.3"><template id="x-resume" type="text/cem-ml">${template}</template></cem-element><x-resume><strong slot="heading">Island input</strong></x-resume>`, 'x-resume=1.9.0');
    const childSource = '{x-child | {em @slot=heading | Nested input}}';
    const renderNested = parentSource => render([
        `<cem-element tag="x-parent" version="1.2.3"><template id="x-parent" type="text/cem-ml">${parentSource}</template></cem-element>`,
        `<cem-element tag="x-child" version="1.2.3"><template id="x-child" type="text/cem-ml">${template}</template></cem-element>`,
        '<x-parent></x-parent>',
    ].join(''), 'x-parent=1.9.0,x-child=1.9.0');
    const nested = await renderNested(`{section | ${childSource}}`);
    const direct = await renderNested(childSource);
    const styled = await render([
        '<cem-element tag="x-parent" version="1.2.3"><template id="x-parent" type="text/cem-ml">',
        '{style |```section {color:rgb(7,31,55)}```}',
        `{section | ${childSource} {$ datadom.slots.heading}}</template></cem-element>`,
        '<cem-element tag="x-child" scope="ssr-test" version="1.2.3"><template id="x-child" type="text/cem-ml">',
        '{style @scope=ssr-test |```@keyframes pulse {from {opacity:.5} to {opacity:1}} article {font-size:23px;animation:pulse 60s}```}',
        `${template}</template></cem-element>`,
        '<x-parent><template><style>section {background-color:rgb(29,47,71)}</style><strong slot="heading">Styled input</strong></template></x-parent>',
        '<section>Outside scope</section>',
    ].join(''), 'x-parent=1.9.0,x-child=1.9.0');
    const foreign = await render([
        `<cem-element tag="x-foreign" version="1.2.3"><template id="x-foreign" type="text/cem-ml">${template}</template></cem-element>`,
        '<x-foreign><svg slot="heading" viewBox="0 0 24 24" xml:lang="en">',
        '<defs><path id="shape" d="M0 0h1"></path><linearGradient id="paint"></linearGradient></defs>',
        '<use xlink:href="#shape"></use></svg><math slot="heading"><mi>x</mi></math></x-foreign>',
    ].join(''), 'x-foreign=1.9.0');
    const cemSource = await render([
        '<cem-element tag="x-source" version="1.2.3"><template id="x-source" type="text/cem-ml">{article | Visible}</template></cem-element>',
        '<x-source><template type="cem-ml">{item | {$ 1 div 0}}</template></x-source>',
    ].join(''), 'x-source=1.9.0');
    const forms = await render([
        '<cem-element tag="x-profile" version="1.2.3"><template id="x-profile" type="text/cem-ml">',
        '{slice @name=message | Ready}{slice @name=flag | false}{slice @name=choice}',
        '{article | {p | {$ message}}{form @id=profile-form @name=profile |',
        '{input @type=text @name=message @value=Ready @required @slice=message}',
        '{input @type=checkbox @name=flag @value=yes @checked @slice=flag}',
        '{select @name=choice @slice=choice | {option @value=a | Alpha}{option @value=b | Beta}}',
        '}}</template></cem-element><x-profile></x-profile>',
    ].join(''), 'x-profile=1.9.0');
    const mirror = path.join(temp, 'mirror');
    await mkdir(mirror);
    await writeFile(path.join(mirror, 'theme.css'), 'article { color: rgb(12,34,56) }');
    const mapped = await render([
        '<cem-element tag="x-mapped" version="1.2.3"><template id="x-mapped" type="text/cem-ml">',
        '{style |```@import "./theme.css";```}{article | Mapped}',
        '</template></cem-element><x-mapped></x-mapped><article>Outside</article>',
    ].join(''), 'x-mapped=1.9.0', ['--resolver-read-map', `https://ssr.example.test/assets/=${mirror}`],
    '<base href="/assets/">');
    const externalDeclaration = '<cem-element tag="x-external" version="1.2.3">'
        + '<template id="x-external" type="text/cem-ml">{article | From external declaration}</template></cem-element>';
    await writeFile(path.join(mirror, 'external.html'), externalDeclaration);
    const externalPage = await render(
        '<cem-element tag="x-external" version="1.2.3" src="./external.html#x-external"></cem-element><x-external></x-external>',
        'x-external=1.9.0', ['--resolver-read-map', `https://ssr.example.test/assets/=${mirror}`],
        '<base href="/assets/">',
    );
    const payloadText = '{item | Mapped payload {$ 1 div 0}}';
    await writeFile(path.join(mirror, 'payload.cem'), payloadText);
    const externalPayloadPage = await render([
        '<cem-element tag="x-payload" version="1.2.3"><template id="x-payload" type="text/cem-ml">',
        '{article | {$ datadom.payload.children.targets.children}}</template></cem-element>',
        '<x-payload><template type="text/cem-ml" data-cem-payload-src="./payload.cem"></template></x-payload>',
    ].join(''), 'x-payload=1.9.0', ['--resolver-read-map', `https://ssr.example.test/assets/=${mirror}`],
    '<base href="/assets/">');
    const component = path.join(mirror, 'components');
    await mkdir(path.join(component, 'styles'), { recursive: true });
    await writeFile(path.join(component, 'card.xhtml'), [
        '<cem-element xmlns="http://www.w3.org/1999/xhtml" tag="x-card" version="1.2.3" scope="card">',
        '<template id="x-card" type="text/cem-ml">',
        "{style @scope=card |```@import './styles/card.css';```}{article | XHTML &amp; CSS}",
        '</template></cem-element>',
    ].join(''));
    await writeFile(path.join(component, 'styles/card.css'),
        "@import './color.css'; article {padding:3px}");
    await writeFile(path.join(component, 'styles/color.css'), 'article {color:rgb(17,34,51)}');
    const xhtmlPage = await render([
        '<cem-element tag="x-card" version="1.2.3" scope="card" src="./components/card.xhtml#x-card"></cem-element>',
        '<x-card></x-card><x-card></x-card><article>Outside scope</article>',
    ].join(''), 'x-card=1.9.0', ['--resolver-read-map', `https://ssr.example.test/assets/=${mirror}`],
    '<base href="/assets/">');
    const authored = '{cem-element @tag=x-authored @version="1.2.3" | {template @id=x-authored @type="text/cem-ml" |```{article | From source}```}}{x-authored}';
    const cemPage = await renderSource(path.join(temp, 'source.cem'),
        `{html | {head | {title | Native page}}{body | ${authored}}}`,
        'x-authored=1.9.0', ['--data-content-type', 'application/vnd.cem.page+cem']);
    const markdownPage = await renderSource(path.join(temp, 'source.cem.md'),
        `# Native page\n\n\`\`\`cem-ml\n${authored}\n\`\`\`\n`, 'x-authored=1.9.0');
    const authoredPages = ['/cem.html', '/cem-markdown.html'];
    const pages = new Map([
        ['/ssr.html', html], ['/nested.html', nested], ['/direct.html', direct], ['/styled.html', styled],
        ['/foreign.html', foreign], ['/source.html', cemSource], ['/forms.html', forms],
        ['/mapped.html', mapped], [authoredPages[0], cemPage], [authoredPages[1], markdownPage],
        ['/external.html', externalPage.replace(/ data-cem-ssr-declaration="[^"]*"/gu, '')],
        ['/assets/external.html', externalDeclaration],
        ['/external-payload.html', externalPayloadPage.replace(/ data-cem-ssr-payload="[^"]*"/gu, '')], ['/xhtml.html', xhtmlPage],
    ]);
    assert.match(html, /<article><strong\b[^>]*\bslot="heading"[^>]*>Island input<\/strong><\/article>/);
    assert.match(html, /data-cem-ssr-profile="cem-native-inline\/1"/);
    assert.match(nested, /<article><em\b[^>]*\bslot="heading"[^>]*>Nested input<\/em><\/article>/);
    assert.equal((styled.match(/data-cem-style-key="cem-ssr-closed-css-1-/g) ?? []).length, 2);
    assert.match(styled, /data-cem-instance-style="0"/);
    server = http.createServer(async (req, res) => {
        try {
            const pathname = new URL(req.url, 'http://localhost').pathname;
            if (pages.has(pathname)) {
                res.setHeader('Content-Type', 'text/html; charset=utf-8');
                res.end(pages.get(pathname));
                return;
            }
            const file = path.resolve(root, `.${pathname}`);
            assert.ok(file.startsWith(path.join(root, 'packages/cem-elements/dist/')));
            res.setHeader('Content-Type', pathname.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
            res.end(await readFile(file));
        } catch { res.statusCode = 404; res.end(); }
    });
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const origin = `http://127.0.0.1:${server.address().port}`;
    browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_CHROME });
    const requests = [], errors = [];
    const cases = ['compatible', 'missing', 'malformed', 'incompatible', 'unknown-schema', 'unknown-profile', 'tampered-range', 'stale', 'duplicate'];
    for (const scenario of cases) {
        const page = await browser.newPage();
        page.on('pageerror', error => errors.push(error.message));
        await page.route('**/*', route => {
            if (new URL(route.request().url()).origin === origin) return route.continue();
            requests.push(route.request().url()); return route.abort();
        });
        await page.goto(`${origin}/ssr.html`);
        const result = await page.evaluate(async scenario => {
            const { CemElementRuntime } = await import('/packages/cem-elements/dist/lib/cem-elements.js');
            const host = document.querySelector('x-resume');
            const declaration = document.querySelector('cem-element');
            const island = host.querySelector('template[data-cem-island]');
            const root = island.content.firstElementChild;
            const field = name => [...root.firstElementChild.children].find(f => f.getAttribute('name') === name);
            const retained = host.querySelector('article');
            const version = field('declarationVersion');
            declaration.setAttribute('version', scenario === 'incompatible' ? '2.0.0' : '1.9.0');
            if (scenario === 'missing') version.remove();
            if (scenario === 'malformed') version.textContent = 'v1';
            if (scenario === 'incompatible') {
                declaration.querySelector('template').innerHTML = '{section | {$ datadom.slots.heading}}';
                retained.querySelector('strong').textContent = 'Stale output';
            }
            if (scenario === 'unknown-profile') island.setAttribute('data-cem-ssr-profile', 'unknown/2');
            if (scenario === 'unknown-schema') { root.setAttribute('version', '0.3.0'); field('version').textContent = '0.3.0'; }
            if (scenario === 'tampered-range') retained.querySelector('strong').textContent = 'Tampered output';
            const duplicate = scenario === 'duplicate' ? host.cloneNode(true) : null;
            if (duplicate) host.after(duplicate);
            const runtime = new CemElementRuntime({ processingWorkerFactory: () => { throw new Error('Use main-thread runtime'); } });
            if (scenario === 'stale') {
                const support = runtime.renderSupport.bind(runtime);
                runtime.renderSupport = scope => {
                    const bindings = support(scope);
                    return { ...bindings, admitCanonicalSsrInstance: async (...args) => {
                        const decision = await bindings.admitCanonicalSsrInstance(...args);
                        retained.querySelector('strong').textContent = 'Changed during admission';
                        return decision;
                    } };
                };
            }
            const before = host.innerHTML;
            runtime.install(window);
            runtime.registerDeclaration(declaration);
            await runtime.whenDeclarationSettled(declaration);
            await runtime.whenRenderSettled(host);
            if (duplicate) await runtime.whenRenderSettled(duplicate);
            const retainedNow = host.querySelector('article') === retained;
            const unchanged = host.innerHTML === before;
            const text = host.querySelector('article > strong, section > strong')?.textContent;
            const diagnostics = runtime.diagnosticsFor(host).map(d => d.code);
            let reconnect = null, invalidated = null, legacy = null;
            if (scenario === 'compatible') {
                const restored = host.cloneNode(true);
                const restoredOutput = restored.querySelector('article');
                host.remove(); document.body.append(host); await runtime.whenRenderSettled(host);
                reconnect = host.querySelector('article') === retained;
                const payload = [...root.children].find(e => e.localName === 'cem-payload:payload');
                payload.querySelector('strong').textContent = 'Updated island';
                await new Promise(resolve => setTimeout(resolve, 0));
                await runtime.whenRenderSettled(host);
                invalidated = host.querySelector('article > strong')?.textContent;
                host.remove(); document.body.append(restored); await runtime.whenRenderSettled(restored);
                legacy = {
                    retained: restored.querySelector('article') === restoredOutput,
                    marked: restored.querySelector('template[data-cem-ssr-profile]') !== null,
                    diagnostics: runtime.diagnosticsFor(restored).map(d => d.code),
                };
            }
            return { retainedNow, unchanged, text, diagnostics, reconnect, invalidated, legacy,
                duplicateRefusals: duplicate ? [host, duplicate].filter(node => runtime.diagnosticsFor(node)
                    .some(d => d.code === 'cem-element.hydration_native_refused')).length : 0 };
        }, scenario);
        if (scenario === 'compatible') {
            assert.equal(result.retainedNow, true, JSON.stringify(result)); assert.equal(result.reconnect, true);
            assert.equal(result.invalidated, 'Updated island', JSON.stringify(result)); assert.deepEqual(result.diagnostics, []);
            assert.deepEqual(result.legacy, { retained: true, marked: false, diagnostics: [] });
        } else if (['missing', 'malformed', 'incompatible'].includes(scenario)) {
            assert.equal(result.retainedNow, false, JSON.stringify(result));
            assert.equal(result.text, 'Island input');
            assert.ok(result.diagnostics.includes(`cem-element.hydration_declaration_version_${scenario === 'missing' ? 'missing' : 'incompatible'}`));
        } else if (scenario === 'duplicate') {
            assert.equal(result.duplicateRefusals, 1, JSON.stringify(result));
        } else {
            assert.equal(result.retainedNow, true); assert.ok(result.diagnostics.includes('cem-element.hydration_native_refused'), JSON.stringify(result));
            if (scenario === 'stale') assert.equal(result.text, 'Changed during admission');
            else assert.equal(result.unchanged, true);
        }
        await page.close();
    }
    const accepted = ['compatible', 'child-first', 'direct-root'];
    const nestedCases = [...accepted, 'child-version', 'child-output', 'child-payload', 'child-marker', 'duplicate-id', 'stale-child', 'stale-declaration'];
    for (const scenario of nestedCases) {
        const page = await browser.newPage();
        page.on('pageerror', error => errors.push(error.message));
        await page.route('**/*', route => {
            if (new URL(route.request().url()).origin === origin) return route.continue();
            requests.push(route.request().url()); return route.abort();
        });
        await page.goto(`${origin}/${scenario === 'direct-root' ? 'direct' : 'nested'}.html`);
        const result = await page.evaluate(async ({ scenario, adopt }) => {
            const { CemElementRuntime } = await import('/packages/cem-elements/dist/lib/cem-elements.js');
            const parent = document.querySelector('x-parent');
            const child = parent.querySelector('x-child');
            const hosts = [parent, child];
            const islands = hosts.map(host => host.querySelector(':scope > template[data-cem-island]'));
            const field = (island, name) => island.content.querySelector(`[name="${name}"]`);
            const payload = islands[1].content.querySelector('em');
            const section = parent.querySelector('section');
            const article = child.querySelector('article');
            const projected = article.querySelector('em');
            if (scenario === 'child-first') document.body.prepend(document.querySelector('cem-element[tag="x-child"]'));
            if (scenario === 'child-version') field(islands[1], 'declarationVersion').textContent = '2.0.0';
            if (scenario === 'child-output') projected.textContent = 'Changed output';
            if (scenario === 'child-payload') payload.textContent = projected.textContent = 'Changed input';
            if (scenario === 'child-marker') islands[1].removeAttribute('data-cem-ssr-profile');
            if (scenario === 'duplicate-id') field(islands[1], 'instanceId').textContent = field(islands[0], 'instanceId').textContent;
            const before = parent.innerHTML;
            const runtime = new CemElementRuntime({ processingWorkerFactory: () => { throw new Error('Use main-thread runtime'); } });
            if (scenario === 'stale-child') {
                const support = runtime.renderSupport.bind(runtime);
                runtime.renderSupport = scope => {
                    const bindings = support(scope);
                    return { ...bindings, admitCanonicalSsrInstance: async (...args) => {
                        const decision = await bindings.admitCanonicalSsrInstance(...args);
                        payload.textContent = 'Changed during admission';
                        return decision;
                    } };
                };
            }
            runtime.install(window);
            if (scenario === 'stale-declaration') document.querySelector('cem-element[tag="x-child"]').setAttribute('version', '2.0.0');
            await Promise.all([...document.querySelectorAll('cem-element')].map(node => runtime.whenDeclarationSettled(node)));
            await Promise.all(hosts.map(host => runtime.whenRenderSettled(host)));
            const unchanged = parent.innerHTML === before;
            const retained = parent.querySelector('section') === section && parent.querySelector('x-child') === child
                && child.querySelector('article') === article && article.querySelector('em') === projected;
            const marks = islands.map(island => island.hasAttribute('data-cem-ssr-profile'));
            const diagnostics = hosts.map(host => runtime.diagnosticsFor(host).map(diagnostic => diagnostic.code));
            const leakedMetadata = islands[1].content.querySelector('[name="data-cem-template-artifact-id"]') !== null;
            let updated = null;
            if (adopt) {
                payload.textContent = 'Live child input';
                await new Promise(resolve => setTimeout(resolve, 0));
                await runtime.whenRenderSettled(child);
                updated = child.querySelector('article em')?.textContent;
            }
            return { unchanged, retained, marks, diagnostics, leakedMetadata, updated };
        }, { scenario, adopt: accepted.includes(scenario) });
        const detail = `${scenario} ${JSON.stringify(result)}`;
        assert.equal(result.retained, true, detail);
        if (accepted.includes(scenario)) {
            assert.deepEqual(result.marks, [false, false], detail);
            assert.deepEqual(result.diagnostics, [[], []], detail);
            assert.equal(result.leakedMetadata, false, detail);
            assert.equal(result.updated, 'Live child input', detail);
        } else {
            assert.deepEqual(result.marks, [true, scenario !== 'child-marker'], detail);
            assert.ok(result.diagnostics[0].includes('cem-element.hydration_native_refused'), detail);
            if (scenario !== 'child-marker') assert.ok(result.diagnostics[1].includes('cem-element.hydration_native_refused'), detail);
            if (scenario !== 'stale-child') assert.equal(result.unchanged, true, detail);
        }
        await page.close();
    }
    const cssCases = ['compatible', 'compatible-worker', 'missing-declaration-style', 'changed-declaration-style', 'wrong-style-key',
        'missing-instance-style', 'changed-instance-style', 'unresolved-payload-style', 'unadmitted-shared-owner',
        'disabled-style', 'changed-cssom', 'stale-style'];
    for (const scenario of cssCases) {
        const page = await browser.newPage();
        page.on('pageerror', error => errors.push(error.message));
        await page.route('**/*', route => {
            if (new URL(route.request().url()).origin === origin) return route.continue();
            requests.push(route.request().url()); return route.abort();
        });
        await page.goto(`${origin}/styled.html`);
        const result = await page.evaluate(async scenario => {
            const { CemElementRuntime } = await import('/packages/cem-elements/dist/lib/cem-elements.js');
            const parent = document.querySelector('x-parent');
            const child = parent.querySelector('x-child');
            const section = parent.querySelector('section');
            const article = child.querySelector('article');
            const declarations = [...document.querySelectorAll('cem-element')];
            const styles = [...declarations.map(d => d.querySelector('style')), parent.querySelector(':scope > style[data-cem-instance-style]')];
            const sheets = styles.map(style => style.sheet);
            const initial = [getComputedStyle(section).color, getComputedStyle(section).backgroundColor, getComputedStyle(article).fontSize];
            const islands = [parent, child].map(host => host.querySelector(':scope > template[data-cem-island]'));
            if (scenario === 'missing-declaration-style') styles[1].remove();
            if (scenario === 'changed-declaration-style') styles[1].textContent = '@scope (x-child) { article {font-size:32px} }';
            if (scenario === 'wrong-style-key') styles[0].setAttribute('data-cem-style-key', 'wrong-key');
            if (scenario === 'missing-instance-style') styles[2].remove();
            if (scenario === 'changed-instance-style') styles[2].textContent = '@scope { section {background-color:black} }';
            if (scenario === 'unresolved-payload-style') islands[0].content.querySelector('style').textContent = '@import "https://ssr.example.test/unresolved.css";';
            if (scenario === 'disabled-style') styles[1].sheet.disabled = true;
            if (scenario === 'changed-cssom') styles[2].sheet.insertRule('section {background-color:black}', 0);
            if (scenario === 'unadmitted-shared-owner') {
                const extra = document.createElement('cem-element');
                extra.setAttribute('tag', 'x-extra');
                extra.setAttribute('scope', 'ssr-test');
                extra.innerHTML = '<template type="text/cem-ml">{style @scope=ssr-test |```@import "https://ssr.example.test/unadmitted.css";```}{span | Extra}</template>';
                document.body.append(extra);
            }
            const before = parent.innerHTML;
            const beforeDeclarations = declarations.map(d => d.innerHTML);
            const runtime = scenario === 'compatible-worker' ? new CemElementRuntime()
                : new CemElementRuntime({ processingWorkerFactory: () => { throw new Error('Use main-thread runtime'); } });
            if (scenario === 'stale-style') {
                const support = runtime.renderSupport.bind(runtime);
                runtime.renderSupport = scope => {
                    const bindings = support(scope);
                    return { ...bindings, admitCanonicalSsrInstance: async (...args) => {
                        const decision = await bindings.admitCanonicalSsrInstance(...args);
                        styles[1].textContent = '@scope (x-child) { article {font-size:42px} }';
                        return decision;
                    } };
                };
            }
            runtime.install(window);
            await Promise.all(declarations.map(node => runtime.whenDeclarationSettled(node)));
            await Promise.all([parent, child].map(host => runtime.whenRenderSettled(host)));
            const afterStyles = [...declarations.map(d => d.querySelector('style')), parent.querySelector(':scope > style[data-cem-instance-style]')];
            return {
                retained: parent.querySelector('section') === section && parent.querySelector('x-child') === child && child.querySelector('article') === article,
                unchanged: parent.innerHTML === before && declarations.every((d, i) => d.innerHTML === beforeDeclarations[i]),
                sameStyles: afterStyles.every((style, i) => style === styles[i] && style.sheet === sheets[i]),
                counts: [...declarations.map(d => d.querySelectorAll('style').length), parent.querySelectorAll(':scope > style').length],
                marks: islands.map(island => island.hasAttribute('data-cem-ssr-profile')),
                diagnostics: [parent, child].map(host => runtime.diagnosticsFor(host).map(d => d.code)),
                colors: [getComputedStyle(section).color, getComputedStyle(section).backgroundColor],
                outside: getComputedStyle(document.body.querySelector(':scope > section')).backgroundColor,
                childFont: getComputedStyle(article).fontSize,
                animation: getComputedStyle(article).animationName,
                initial,
                mode: runtime.processingHost(runtime.declarationForInstance(parent)).mode,
            };
        }, scenario);
        const detail = `${scenario} ${JSON.stringify(result)}`;
        assert.equal(result.retained, true, detail);
        if (scenario === 'compatible' || scenario === 'compatible-worker') {
            assert.deepEqual(result.marks, [false, false], detail);
            assert.deepEqual(result.diagnostics, [[], []], detail);
            assert.deepEqual(result.counts, [1, 1, 1], detail);
            assert.equal(result.sameStyles, true, detail);
            assert.deepEqual(result.initial, ['rgb(7, 31, 55)', 'rgb(29, 47, 71)', '23px'], detail);
            assert.deepEqual(result.colors, ['rgb(7, 31, 55)', 'rgb(29, 47, 71)'], detail);
            assert.equal(result.outside, 'rgba(0, 0, 0, 0)', detail);
            assert.equal(result.childFont, '23px', detail);
            assert.match(result.animation, /^pulse-cem-/);
            assert.equal(result.mode, scenario === 'compatible-worker' ? 'worker' : 'main-thread', detail);
        } else {
            assert.deepEqual(result.marks, [true, true], detail);
            assert.ok(result.diagnostics.every(codes => codes.includes('cem-element.hydration_native_refused')), detail);
            if (scenario !== 'stale-style') assert.equal(result.unchanged, true, detail);
        }
        await page.close();
    }
    const payloadCases = [
        'foreign-compatible', 'foreign-range', 'foreign-active',
        'source-compatible', 'source-invalid', 'source-markup', 'source-incompatible', 'source-stale',
    ];
    for (const scenario of payloadCases) {
        const page = await browser.newPage();
        page.on('pageerror', error => errors.push(error.message));
        await page.route('**/*', route => {
            if (new URL(route.request().url()).origin === origin) return route.continue();
            requests.push(route.request().url()); return route.abort();
        });
        const foreign = scenario.startsWith('foreign');
        await page.goto(`${origin}/${foreign ? 'foreign' : 'source'}.html`);
        const result = await page.evaluate(async ({ scenario, foreign }) => {
            const { CemElementRuntime } = await import('/packages/cem-elements/dist/lib/cem-elements.js');
            const host = document.querySelector(foreign ? 'x-foreign' : 'x-source');
            const declaration = document.querySelector('cem-element');
            const island = host.querySelector('template[data-cem-island]');
            const article = host.querySelector('article');
            const svg = article.querySelector('svg');
            const math = article.querySelector('math');
            const envelope = island.content.querySelector('template');
            const xlink = 'http://www.w3.org/1999/xlink';
            const xml = 'http://www.w3.org/XML/1998/namespace';
            const expandedNames = () => {
                const current = host.querySelector('article');
                const svg = current.querySelector('svg');
                return [svg.namespaceURI, svg.getAttribute('viewBox'), svg.getAttributeNS(xml, 'lang'),
                    svg.querySelector('[id="paint"]').localName, svg.querySelector('use').getAttributeNS(xlink, 'href'),
                    current.querySelector('math').namespaceURI];
            };
            if (scenario === 'foreign-range') svg.setAttribute('viewBox', '0 0 48 48');
            if (scenario === 'foreign-active') {
                island.content.querySelector('use').setAttributeNS(xlink, 'xlink:href', 'https://ssr.example.test/active.svg');
            }
            if (scenario === 'source-invalid') envelope.content.textContent = '{unclosed |';
            if (scenario === 'source-markup') envelope.content.append(document.createElement('p'));
            if (scenario === 'source-incompatible') declaration.setAttribute('version', '2.0.0');
            const runtime = new CemElementRuntime({ processingWorkerFactory: () => { throw new Error('Use main-thread runtime'); } });
            if (scenario === 'source-stale') {
                const support = runtime.renderSupport.bind(runtime);
                runtime.renderSupport = scope => {
                    const bindings = support(scope);
                    return { ...bindings, admitCanonicalSsrInstance: async (...args) => {
                        const decision = await bindings.admitCanonicalSsrInstance(...args);
                        envelope.content.textContent = '{item | Changed during admission}';
                        return decision;
                    } };
                };
            }
            const before = host.innerHTML;
            runtime.install(window);
            await runtime.whenDeclarationSettled(declaration);
            await runtime.whenRenderSettled(host);
            const result = {
                retained: host.querySelector('article') === article && article.querySelector('svg') === svg
                    && article.querySelector('math') === math && island.content.querySelector('template') === envelope,
                unchanged: host.innerHTML === before,
                marked: island.hasAttribute('data-cem-ssr-profile'),
                diagnostics: runtime.diagnosticsFor(host).map(d => d.code),
            };
            if (scenario === 'foreign-compatible') {
                result.expandedNames = expandedNames();
                island.content.querySelector('mi').textContent = 'y';
                await new Promise(resolve => setTimeout(resolve, 0));
                await runtime.whenRenderSettled(host);
                result.updated = host.querySelector('article mi')?.textContent;
                result.updatedNames = expandedNames();
            }
            if (scenario === 'source-compatible') {
                result.literal = envelope.content.textContent;
                result.output = article.textContent;
                const renderToken = runtime.renderTokens.get(host);
                envelope.content.textContent = '{item | Updated {$ 1 div 0}}';
                await new Promise(resolve => setTimeout(resolve, 0));
                await runtime.whenRenderSettled(host);
                result.invalidated = runtime.renderTokens.get(host) > renderToken;
                result.updated = host.querySelector('article')?.textContent;
                result.afterEdit = runtime.diagnosticsFor(host).map(d => d.code);
            }
            return result;
        }, { scenario, foreign });
        const detail = `${scenario} ${JSON.stringify(result)}`;
        assert.equal(result.retained, true, detail);
        if (scenario.endsWith('-compatible')) {
            assert.equal(result.marked, false, detail);
            assert.deepEqual(result.diagnostics, [], detail);
            if (foreign) {
                assert.deepEqual(result.expandedNames, ['http://www.w3.org/2000/svg', '0 0 24 24', 'en',
                    'linearGradient', '#shape', 'http://www.w3.org/1998/Math/MathML'], detail);
                assert.deepEqual(result.updatedNames, result.expandedNames, detail);
                assert.equal(result.updated, 'y', detail);
            } else {
                assert.equal(result.literal, '{item | {$ 1 div 0}}', detail);
                assert.equal(result.output, 'Visible', detail);
                assert.equal(result.invalidated, true, detail);
                assert.equal(result.updated, 'Visible', detail);
                assert.deepEqual(result.afterEdit, [], detail);
            }
        } else {
            assert.equal(result.marked, true, detail);
            assert.ok(result.diagnostics.includes('cem-element.hydration_native_refused'), detail);
            if (scenario !== 'source-stale') assert.equal(result.unchanged, true, detail);
        }
        await page.close();
    }
    const formCases = ['dirty-compatible', 'worker-compatible', 'slice-tampered', 'forged-form-state',
        'private-field', 'external-owner', 'shadowed-form', 'stale-form'];
    for (const scenario of formCases) {
        const page = await browser.newPage();
        page.on('pageerror', error => errors.push(error.message));
        await page.route('**/*', route => {
            if (new URL(route.request().url()).origin === origin) return route.continue();
            requests.push(route.request().url()); return route.abort();
        });
        await page.goto(`${origin}/forms.html`);
        const result = await page.evaluate(async scenario => {
            const { CemElementRuntime } = await import('/packages/cem-elements/dist/lib/cem-elements.js');
            const host = document.querySelector('x-profile');
            const declaration = document.querySelector('cem-element');
            const island = host.querySelector('template[data-cem-island]');
            const form = host.querySelector('form');
            const message = form.querySelector('input[name=message]');
            const flag = form.querySelector('input[name=flag]');
            const choice = form.querySelector('select');
            const paragraph = host.querySelector('p');
            const comments = [...host.childNodes].filter(node => node.nodeType === Node.COMMENT_NODE);
            const retained = () => host.querySelector('p') === paragraph && host.querySelector('form') === form
                && form.querySelector('input') === message && form.querySelector('input[name=flag]') === flag
                && form.querySelector('select') === choice && comments.every(comment => comment.parentNode === host);
            const state = () => [message.value, flag.checked, choice.value];
            const inputState = ['Pre-hydration edit', false, 'b'];
            [message.value, flag.checked, choice.value] = inputState;
            message.focus();
            message.setSelectionRange(3, 8);
            let formdataEvents = 0;
            form.addEventListener('formdata', () => formdataEvents++);
            const external = () => {
                const input = document.createElement('input');
                input.name = 'external';
                input.value = 'External private value';
                input.setAttribute('form', form.id);
                document.body.append(input);
                return input;
            };
            if (scenario === 'slice-tampered') island.content.querySelector('[name="flag"]').textContent = 'true';
            if (scenario === 'forged-form-state') {
                const entry = document.createElement('cem-form:form');
                entry.textContent = 'Forged value';
                island.content.firstElementChild.children[6].append(entry);
            }
            if (scenario === 'private-field') {
                message.type = 'password';
                message.value = 'Private password';
                const template = declaration.querySelector('template');
                template.content.textContent = template.content.textContent.replace('@type=text', '@type=password');
            }
            if (scenario === 'external-owner') external();
            if (scenario === 'shadowed-form') {
                message.name = 'setAttribute';
                const template = declaration.querySelector('template');
                template.content.textContent = template.content.textContent.replace('@type=text @name=message', '@type=text @name=setAttribute');
                if (typeof form.setAttribute === 'function') throw new Error('Fixture must shadow the form method');
            }
            const runtime = scenario === 'worker-compatible' ? new CemElementRuntime()
                : new CemElementRuntime({ processingWorkerFactory: () => { throw new Error('Use main-thread runtime'); } });
            if (scenario === 'stale-form') {
                const prepare = runtime.prepareCanonicalSsrState.bind(runtime);
                runtime.prepareCanonicalSsrState = async (...args) => {
                    const baseline = await prepare(...args);
                    message.value = 'Changed while preparing baseline';
                    return baseline;
                };
            }
            const before = host.innerHTML;
            runtime.install(window);
            await runtime.whenDeclarationSettled(declaration);
            await runtime.whenRenderSettled(host);
            const result = {
                retained: retained(), unchanged: host.innerHTML === before,
                marked: island.hasAttribute('data-cem-ssr-profile'),
                diagnostics: runtime.diagnosticsFor(host).map(d => d.code),
                messages: runtime.diagnosticsFor(host).map(d => d.message),
                formdataEvents,
            };
            if (scenario.endsWith('-compatible') && !result.marked) {
                result.mode = runtime.processingHost(runtime.declarationForInstance(host)).mode;
                result.state = state();
                result.focus = document.activeElement === message && message.selectionStart === 3 && message.selectionEnd === 8;
                const initial = runtime.readInstanceSnapshot(host);
                result.slices = [initial.slices.message, initial.slices.flag, initial.slices.choice];
                result.form = initial.formData.profile;
                result.valid = initial.validationState.profile?.valid;
                message.value = 'Browser update';
                message.dispatchEvent(new Event('change', { bubbles: true }));
                await runtime.whenRenderSettled(host);
                flag.click();
                choice.value = 'a';
                choice.dispatchEvent(new Event('change', { bubbles: true }));
                await runtime.whenRenderSettled(host);
                const updated = runtime.readInstanceSnapshot(host);
                result.updated = [updated.slices.message, updated.slices.flag, updated.slices.choice, paragraph.textContent];
                result.afterState = state();
                result.afterRetained = retained();
                result.afterDiagnostics = runtime.diagnosticsFor(host).map(d => d.code);
                result.localSubmit = !form.dispatchEvent(new SubmitEvent('submit', { bubbles: true, cancelable: true }));
                await runtime.whenRenderSettled(host);
                const outside = external();
                message.type = 'password';
                message.value = 'Entered private value';
                message.dispatchEvent(new Event('change', { bubbles: true }));
                const snapshot = runtime.readInstanceSnapshot(host);
                result.privacy = !JSON.stringify(snapshot).includes('Entered private value')
                    && !JSON.stringify(snapshot).includes('External private value');
                outside.remove();
                result.formdataEvents = formdataEvents;
            }
            return result;
        }, scenario);
        const detail = `${scenario} ${JSON.stringify(result)}`;
        assert.equal(result.retained, true, detail);
        assert.equal(result.formdataEvents, 0, detail);
        if (scenario.endsWith('-compatible')) {
            assert.equal(result.marked, false, detail);
            assert.deepEqual(result.diagnostics, [], detail);
            assert.equal(result.mode, scenario === 'worker-compatible' ? 'worker' : 'main-thread', detail);
            assert.deepEqual(result.state, ['Pre-hydration edit', false, 'b'], detail);
            assert.equal(result.focus, true, detail);
            assert.deepEqual(result.slices, ['Ready', false, null], detail);
            assert.deepEqual(result.form, { message: 'Pre-hydration edit', choice: 'b' }, detail);
            assert.equal(result.valid, true, detail);
            assert.deepEqual(result.updated, ['Browser update', 'yes', 'a', 'Browser update'], detail);
            assert.deepEqual(result.afterState, ['Browser update', true, 'a'], detail);
            assert.equal(result.afterRetained, true, detail);
            assert.equal(result.localSubmit, true, detail);
            assert.equal(result.privacy, true, detail);
            assert.deepEqual(result.afterDiagnostics, [], detail);
        } else {
            assert.equal(result.marked, true, detail);
            assert.equal(result.unchanged, true, detail);
            assert.ok(result.diagnostics.includes('cem-element.hydration_native_refused'), detail);
        }
        await page.close();
    }
    const page = await browser.newPage();
    page.on('pageerror', error => errors.push(error.message));
    await page.route('**/*', route => {
        if (new URL(route.request().url()).origin === origin) return route.continue();
        requests.push(route.request().url()); return route.abort();
    });
    await page.goto(`${origin}/mapped.html`);
    const mappedResult = await page.evaluate(async () => {
        const { CemElementRuntime } = await import('/packages/cem-elements/dist/lib/cem-elements.js');
        const host = document.querySelector('x-mapped');
        const output = host.querySelector('article');
        const declaration = document.querySelector('cem-element');
        const style = declaration.querySelector('style');
        const initial = getComputedStyle(output).color;
        const runtime = new CemElementRuntime({ processingWorkerFactory: () => { throw new Error('Use main-thread runtime'); } });
        runtime.install(window);
        await runtime.whenDeclarationSettled(declaration);
        await runtime.whenRenderSettled(host);
        return {
            baseUrl: document.baseURI,
            initial, current: getComputedStyle(output).color,
            outside: getComputedStyle(document.querySelector('body > article')).color,
            retained: host.querySelector('article') === output && declaration.querySelector('style') === style,
            imported: style.textContent.includes('@import'),
            marked: host.querySelector('template[data-cem-ssr-profile]') !== null,
            diagnostics: runtime.diagnosticsFor(host).map(d => d.code),
        };
    });
    assert.equal(mappedResult.baseUrl, `${origin}/assets/`);
    assert.equal(mappedResult.initial, 'rgb(12, 34, 56)');
    assert.equal(mappedResult.current, mappedResult.initial);
    assert.notEqual(mappedResult.outside, mappedResult.initial);
    assert.equal(mappedResult.retained, true);
    assert.equal(mappedResult.imported, false);
    assert.equal(mappedResult.marked, true);
    assert.ok(mappedResult.diagnostics.includes('cem-element.hydration_native_refused'));
    await page.close();
    for (const [route, tag, text] of [
        ['/external.html', 'x-external', 'From external declaration'],
        ['/external-payload.html', 'x-payload', payloadText],
    ]) {
        const external = await browser.newPage();
        external.on('pageerror', error => errors.push(error.message));
        const payloadReads = [];
        await external.route('**/*', route => {
            const url = new URL(route.request().url());
            if (url.pathname === '/assets/payload.cem') payloadReads.push(url.pathname);
            if (url.origin === origin) return route.continue();
            requests.push(url.href); return route.abort();
        });
        await external.goto(`${origin}${route}`);
        const externalResult = await external.evaluate(async tag => {
            const { CemElementRuntime } = await import('/packages/cem-elements/dist/lib/cem-elements.js');
            const host = document.querySelector(tag);
            const declaration = document.querySelector('cem-element');
            const article = host.querySelector('article');
            const before = host.innerHTML;
            const runtime = new CemElementRuntime({ processingWorkerFactory: () => { throw new Error('Use main-thread runtime'); } });
            runtime.install(window);
            await runtime.whenDeclarationSettled(declaration);
            await runtime.whenRenderSettled(host);
            return {
                text: article?.textContent,
                unchanged: host.innerHTML === before && host.querySelector('article') === article,
                marked: host.querySelector('template[data-cem-ssr-profile]') !== null,
                diagnostics: runtime.diagnosticsFor(host).map(d => d.code),
            };
        }, tag);
        assert.deepEqual(externalResult, {
            text, unchanged: true, marked: true,
            diagnostics: ['cem-element.hydration_native_refused'],
        }, route);
        assert.deepEqual(payloadReads, []);
        await external.close();
    }
    const proofRoot = path.join(temp, 'verified');
    await mkdir(proofRoot);
    const proofDeclaration = '<cem-element xmlns="http://www.w3.org/1999/xhtml" tag="x-proof" version="1.2.3">'
        + '<template id="x-proof" type="text/cem-ml">{article | {$ datadom.payload.children.targets.children}}</template></cem-element>';
    const proofPayload = '{item | Verified {$ 1 div 0}}';
    await writeFile(path.join(proofRoot, 'card.xhtml'), proofDeclaration);
    await writeFile(path.join(proofRoot, 'payload.cem'), proofPayload);
    const proofPage = await render([
        '<cem-element tag="x-proof" version="1.2.3" src="./card.xhtml#x-proof"></cem-element>',
        '<x-proof><template type="text/cem-ml" data-cem-payload-src="./payload.cem"></template></x-proof>',
    ].join(''), 'x-proof=1.9.0', ['--resolver-read-map', `${origin}/verified/=${proofRoot}`],
    `<base href="${origin}/verified/">`);
    pages.set('/resource-proof.html', proofPage);
    const resourceCases = ['matching', 'missing-declaration', 'changed-declaration', 'missing-payload',
        'changed-payload', 'redirect', 'tampered-island', 'missing-receipt', 'stale-stub'];
    for (const scenario of resourceCases) {
        const page = await browser.newPage();
        page.on('pageerror', error => errors.push(error.message));
        const reads = [];
        await page.route('**/*', route => {
            const url = new URL(route.request().url());
            if (url.origin !== origin) {
                requests.push(url.href);
                return route.abort();
            }
            if (!url.pathname.startsWith('/verified/')) return route.continue();
            reads.push(url.pathname);
            if (url.pathname.endsWith('/card.xhtml')) {
                if (scenario === 'missing-declaration') return route.fulfill({ status: 404 });
                return route.fulfill({ contentType: 'application/xhtml+xml',
                    body: scenario === 'changed-declaration' ? proofDeclaration.replace('article', 'section') : proofDeclaration });
            }
            if (url.pathname.endsWith('/payload.cem')) {
                if (scenario === 'missing-payload') return route.fulfill({ status: 404 });
                if (scenario === 'redirect') return route.fulfill({ status: 302, headers: { location: '/verified/replacement.cem' } });
                return route.fulfill({ contentType: 'text/cem-ml',
                    body: scenario === 'changed-payload' ? proofPayload.replace('Verified', 'Tampered') : proofPayload });
            }
            return route.fulfill({ status: 404 });
        });
        await page.goto(`${origin}/resource-proof.html`);
        const result = await page.evaluate(async scenario => {
            const { CemElementRuntime } = await import('/packages/cem-elements/dist/lib/cem-elements.js');
            const host = document.querySelector('x-proof');
            const declaration = document.querySelector('cem-element');
            const island = host.querySelector('template[data-cem-island]');
            const article = host.querySelector('article');
            if (scenario === 'tampered-island') {
                island.content.querySelector('template[type="text/cem-ml"]').content.textContent = '{item | Tampered {$ 1 div 0}}';
            }
            if (scenario === 'missing-receipt') island.removeAttribute('data-cem-ssr-payload');
            if (scenario === 'stale-stub') declaration.setAttribute('src', './replacement.xhtml#x-proof');
            const nodes = [...host.childNodes];
            const before = host.innerHTML;
            const runtime = new CemElementRuntime({ processingWorkerFactory: () => { throw new Error('Use main-thread runtime'); } });
            runtime.install(window);
            await runtime.whenDeclarationSettled(declaration);
            await runtime.whenRenderSettled(host);
            return {
                text: article.textContent,
                retained: host.childNodes.length === nodes.length && nodes.every((node, index) => host.childNodes[index] === node),
                unchanged: host.innerHTML === before,
                marked: island.hasAttribute('data-cem-ssr-profile'),
                diagnostics: [...runtime.diagnosticsFor(host), ...runtime.diagnosticsFor(declaration)].map(d => d.code),
            };
        }, scenario);
        const detail = `${scenario} ${JSON.stringify(result)}`;
        assert.equal(result.text, proofPayload, detail);
        assert.equal(result.retained, true, detail);
        if (scenario === 'matching') {
            assert.equal(result.marked, false, detail);
            assert.deepEqual(result.diagnostics, [], detail);
            assert.deepEqual(reads, ['/verified/card.xhtml', '/verified/payload.cem']);
        } else {
            assert.equal(result.marked, true, detail);
            assert.equal(result.unchanged, true, detail);
            assert.ok(result.diagnostics.includes('cem-element.hydration_native_refused'), detail);
            assert.ok(!reads.some(url => url.includes('replacement')), detail);
        }
        await page.close();
    }
    const xhtml = await browser.newPage();
    xhtml.on('pageerror', error => errors.push(error.message));
    const xhtmlReads = [];
    await xhtml.route('**/*', route => {
        const url = new URL(route.request().url());
        if (url.origin !== origin) {
            requests.push(url.href);
            return route.abort();
        }
        if (url.pathname.startsWith('/assets/')) xhtmlReads.push(url.pathname);
        return route.continue();
    });
    await xhtml.goto(`${origin}/xhtml.html`);
    const xhtmlResult = await xhtml.evaluate(() => {
        const style = document.querySelector('cem-element > style[data-cem-declaration-style="shared"]');
        return {
            styles: document.querySelectorAll('cem-element > style').length,
            imports: style?.textContent.includes('@import'),
            cards: Array.from(document.querySelectorAll('x-card > article'), article => [
                article.textContent, getComputedStyle(article).color, getComputedStyle(article).padding,
            ]),
            outside: getComputedStyle(document.querySelector('body > article')).color,
        };
    });
    assert.equal(xhtmlResult.styles, 1);
    assert.equal(xhtmlResult.imports, false);
    assert.deepEqual(xhtmlResult.cards, Array(2).fill(['XHTML & CSS', 'rgb(17, 34, 51)', '3px']));
    assert.notEqual(xhtmlResult.outside, 'rgb(17, 34, 51)');
    assert.deepEqual(xhtmlReads, []);
    await xhtml.close();
    for (const route of authoredPages) {
        const page = await browser.newPage();
        page.on('pageerror', error => errors.push(error.message));
        await page.route('**/*', request => {
            if (new URL(request.request().url()).origin === origin) return request.continue();
            requests.push(request.request().url()); return request.abort();
        });
        await page.goto(`${origin}${route}`);
        const result = await page.evaluate(async () => {
            const { CemElementRuntime } = await import('/packages/cem-elements/dist/lib/cem-elements.js');
            const host = document.querySelector('x-authored');
            const article = host.querySelector('article');
            const declaration = document.querySelector('cem-element');
            const runtime = new CemElementRuntime({ processingWorkerFactory: () => { throw new Error('Use main-thread runtime'); } });
            runtime.install(window);
            await runtime.whenDeclarationSettled(declaration);
            await runtime.whenRenderSettled(host);
            return {
                text: article?.textContent,
                retained: host.querySelector('article') === article,
                diagnostics: runtime.diagnosticsFor(host).map(d => d.code),
            };
        });
        assert.equal(result.text, 'From source', JSON.stringify(result));
        assert.equal(result.retained, true, JSON.stringify(result));
        assert.ok(!result.diagnostics.includes('cem-element.hydration_native_refused'), JSON.stringify(result));
        await page.close();
    }
    assert.deepEqual(errors, []); assert.deepEqual(requests, []);
    const total = cases.length + nestedCases.length + cssCases.length + payloadCases.length + formCases.length + authoredPages.length + resourceCases.length + 4;
    console.log(`Canonical CLI hydration: ${total}/${total}, including ${nestedCases.length} nested, ${cssCases.length} stylesheet, ${payloadCases.length} foreign/source, ${formCases.length} form, two authored source profiles plus verified resources, mapped CSS, declarations and payloads, zero off-origin requests`);
} finally {
    await browser?.close();
    if (server) await new Promise(resolve => server.close(resolve));
    await rm(temp, { recursive: true, force: true });
}
