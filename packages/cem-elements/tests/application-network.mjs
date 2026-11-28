// Real loopback HTTP and WebSocket endpoints record only requests that actually reached them.
import assert from 'node:assert/strict';
import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { chromium, firefox } from 'playwright';
import { WebSocketServer } from 'ws';

const hits = [];
const sockets = new WebSocketServer({ noServer: true });
let streamClosed;
const dist = new URL('../dist/', import.meta.url);
function endpoint() {
    const server = http.createServer(async (req, res) => {
        const url = new URL(req.url, 'http://' + req.headers.host);
        if (url.pathname.startsWith('/runtime/')) {
            const file = new URL(url.pathname.slice(9), dist);
            if (!file.href.startsWith(dist.href)) return res.writeHead(404).end();
            res.setHeader('content-type', file.href.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
            try { res.end(await readFile(file)); } catch { res.writeHead(404).end(); }
            return;
        }
        if (url.pathname === '/' || url.pathname === '/frame') {
            res.setHeader('content-type', 'text/html');
            res.end('<!doctype html><title>Application network checks</title><main></main>');
            return;
        }
        hits.push([url.hostname, url.port, req.method, url.pathname]);
        res.setHeader('Access-Control-Allow-Origin', '*');
        if (url.pathname === '/blue/declaration.xhtml') {
            res.writeHead(200, { 'content-type': 'application/xhtml+xml' }).end(
                '<cem-element xmlns="http://www.w3.org/1999/xhtml"><template id="view" type="text/cem-ml">{p | Granted document}</template></cem-element>');
            return;
        }
        if (url.pathname.endsWith('/redirect')) {
            res.writeHead(302, { location: url.searchParams.get('to') }).end();
        } else if (url.pathname.endsWith('/stream')) {
            res.writeHead(200, { 'content-type': 'text/plain' });
            res.write('first');
            streamClosed = new Promise(resolve => res.on('close', resolve));
        } else {
            res.writeHead(200, { 'content-type': 'application/json' }).end('{"ok":true}');
        }
    });
    server.on('upgrade', (req, socket, head) => {
        hits.push([req.headers.host, 'websocket', req.url]);
        sockets.handleUpgrade(req, socket, head, ws => ws.on('message', data => ws.send(data.toString())));
    });
    return server;
}

const servers = [endpoint(), endpoint()];
try {
    for (const server of servers) await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const ports = servers.map(server => server.address().port);
    const hosts = ['firewall.test', 'api.firewall.test', 'api-east.firewall.test', 'deep.api.firewall.test', 'alien.test'];
    for (const engine of [chromium, firefox]) {
        const browser = await engine.launch(engine === chromium
            ? { args: ['--no-proxy-server', '--host-resolver-rules=' + hosts.map(h => 'MAP ' + h + ' 127.0.0.1').join(',')] }
            : { firefoxUserPrefs: { 'network.dns.localDomains': hosts.join(','), 'network.proxy.type': 0 } });
        try {
            for (const mode of ['light', 'iframe']) {
                const page = await browser.newPage();
                try {
                    // A test host exposing one redirect hop. Only the two local fixtures are reachable.
                    await page.exposeFunction('readHop', async address => {
                        const url = new URL(address);
                        assert.ok(hosts.includes(url.hostname) && ports.includes(Number(url.port)));
                        const response = await fetch('http://127.0.0.1:' + url.port + url.pathname + url.search,
                            { redirect: 'manual' });
                        return { status: response.status, headers: Object.fromEntries(response.headers), body: await response.text() };
                    });
                    await page.goto('http://127.0.0.1:' + ports[0]);
                    await verify(page, mode, ports);
                    console.log(engine.name(), mode, 'passed');
                } finally { await page.close(); }
            }
        } finally { await browser.close(); }
    }
} finally {
    for (const ws of sockets.clients) ws.terminate();
    sockets.close();
    for (const server of servers) { server.closeAllConnections(); server.close(); }
}

async function verify(page, mode, ports) {
    await page.evaluate(async ({ mode, ports }) => {
        const api = await import('/runtime/index.js');
        const create = (doc, name, content) => {
            const app = doc.createElement('cem-application');
            app.id = name;
            app.innerHTML = '<browser-policy>' + content + '</browser-policy>';
            return app;
        };
        function rules(host, path, port, extras = '') {
            return '<allow transport="http" scheme="http" methods="GET HEAD" host="' + host + '" path="' + path
                + '" port="' + port + '" redirect="checked" ' + extras + '></allow>';
        }
        const socketRule = '<allow transport="websocket" scheme="ws" host="firewall.test" path="/blue/socket" credentials="include" port="'+ports[0]+'"></allow>';
        const outer = ports.map(port => rules('**.firewall.test', '/blue/**', port) + rules('firewall.test', '/blue/**', port)).join('') + socketRule;
        const inner = rules('api.firewall.test', '/blue/exact-api', ports[0]) + rules('api-*.firewall.test', '/blue/*/report-*.json', ports[0])
            + rules('*.firewall.test', '/blue/one', ports[0]) + rules('api.firewall.test', '/red', ports[0])
            + rules('**.firewall.test', '/blue/deep', ports[0])
            + ports.map(port => rules('firewall.test', '/blue/**', port)).join('') + socketRule;
        const exp = (id, content, delegate = false) => '<export api="network" to="' + id + '" delegate="' + delegate + '">' + content + '</export>';
        const root = create(document, 'shell', exp('middle', outer, true) + exp('peer', outer));
        const middle = create(document, 'middle', exp('editor', outer).replace('<export ', '<export scope="workspace" '));
        const peer = create(document, 'peer', exp('untrusted', outer));
        const untrusted = create(document, 'untrusted', '');
        document.querySelector('main').append(root); root.append(middle, peer); peer.append(untrusted);
        let doc = document;
        if (mode === 'iframe') {
            const frame = document.createElement('iframe');
            frame.src = '/frame';
            const loaded = new Promise(resolve => frame.onload = resolve);
            middle.append(frame); await loaded; doc = frame.contentDocument;
        }
        const leaf = create(doc, 'editor', '<filter api="network">' + inner + '</filter>');
        leaf.firstElementChild.setAttribute('scope', 'workspace');
        (mode === 'iframe' ? doc.body : middle).append(leaf);
        const template = document.createElement('template');
        template.innerHTML = exp('shell', outer, true);
        api.grantCemApplicationNetwork(null, root, template.content.firstElementChild);
        const issueMiddle = () => api.grantCemApplicationNetwork(root, middle);
        const issueLeaf = () => api.grantCemApplicationNetwork(middle, leaf);
        const middleGrant = issueMiddle();
        issueLeaf(); api.grantCemApplicationNetwork(root, peer);
        for (const [issuer, child] of [[peer, untrusted], [root, leaf], [leaf, middle]]) {
            let refused = false;
            try { api.grantCemApplicationNetwork(issuer, child); } catch { refused = true; }
            if (!refused) throw Error('Unauthorized delegation');
        }
        const url = (path, host = 'firewall.test', port = ports[0]) => 'http://' + host + ':' + port + path;
        function request(address, method = 'GET') {
            return { authoredUrl: address, baseUrl: location.href, resolvedUrl: address, method, headers: {},
                resolverIdentity: 'test', contextIdentity: 'test', resourcePolicyStamp: 'test', signal: new AbortController().signal,
                policy: { allowCrossOrigin: true, redirect: 'follow', maxResponseBytes: 4096, timeoutMs: 3000 } };
        }
        const hop = async address => {
            const { status, headers, body } = await window.readHop(address);
            return new Response(body, { status, headers });
        };
        window.run = async (path, host, port, checked, who = leaf) => {
            try {
                const response = await api.openCemApplicationHttpResource(who, request(url(path, host, port)), checked ? hop : undefined);
                for await (const chunk of response.body) { /* drain the granted read */ }
                return 'ok';
            } catch { return 'denied'; }
        };
        window.testDeclaration = async (path, tag) => {
            // Install in the element's own realm, as independently loaded iframe apps do.
            const runtime = doc.defaultView.networkRuntime;
            const declaration = doc.createElement('cem-element');
            declaration.setAttribute('tag', tag);
            declaration.innerHTML = '<template><http-request slice="remote" method="HEAD" url="' + url(path)
                + '"></http-request><output></output></template>';
            leaf.append(declaration);
            await runtime.whenDeclarationSettled(declaration);
            const element = doc.createElement(tag);
            leaf.append(element);
            await runtime.whenRenderSettled(element);
            return () => runtime.readInstanceSnapshot(element).slices.remote?.state;
        };
        window.testExternal = async granted => {
            const app = granted ? leaf : leaf.appendChild(doc.createElement('cem-application'));
            if (!granted) app.id = 'private-view';
            const declaration = doc.createElement('cem-element');
            const tag = granted ? 'external-granted' : 'external-refused';
            declaration.setAttribute('tag', tag);
            declaration.setAttribute('src', url('/blue/declaration.xhtml#view'));
            app.append(declaration);
            await doc.defaultView.networkRuntime.whenDeclarationSettled(declaration).catch(() => {});
            return !!doc.defaultView.customElements.get(tag);
        };
        window.startLive = async () => {
            const { body } = await api.openCemApplicationHttpResource(leaf, request(url('/blue/stream')));
            const iterator = body[Symbol.asyncIterator]();
            const first = await iterator.next();
            if (new TextDecoder().decode(first.value) !== 'first') throw Error('missing first chunk');
            const next = iterator.next().then(() => 'delivered', () => 'revoked');
            const received = [], closed = [];
            let socket;
            await new Promise((resolve, reject) => {
                socket = api.openCemApplicationWebSocket(leaf, url('/blue/socket').replace('http:', 'ws:'), {
                    open() { socket.send('ready'); }, message(data) { received.push(data); resolve(); },
                    closed(reason) { closed.push(reason); reject(Error(reason)); },
                });
            });
            middleGrant.revoke();
            let sendDenied = false;
            try { socket.send('late'); } catch { sendDenied = true; }
            const denied = await window.run('/blue/exact');
            const peerStatus = await window.run('/blue/exact', undefined, undefined, false, peer);
            issueMiddle();
            const stale = await window.run('/blue/exact');
            issueLeaf();
            const fresh = await window.run('/blue/exact');
            return { next: await next, sendDenied, denied, peerStatus, stale, fresh, received, closed };
        };
    }, {mode, ports});
    const frame = mode === 'iframe' ? page.frames().find(frame => frame.url().endsWith('/frame')) : page;
    await frame.evaluate(async () => {
        const { installCemElementRuntime } = await import('/runtime/index.js');
        window.networkRuntime = installCemElementRuntime(window, {
            httpResourcePolicy: { allowCrossOrigin: true }, processingWorkerFactory() { throw Error('Use owned fallback'); },
            loadHttpResource() { throw Error('Legacy page loader must never carry an application request'); },
        });
    });
    for (const [path, tag, expected] of [['/blue/declaration', 'remote-allowed', 'loaded'], ['/red', 'remote-refused', 'failed']]) {
        const before = hits.length;
        await page.evaluate(async ([path, tag]) => { window.readDeclaration = await window.testDeclaration(path, tag); }, [path, tag]);
        await page.waitForFunction(expected => window.readDeclaration() === expected, expected, { timeout: 10000 });
        assert.equal(hits.length - before, expected === 'loaded' ? 1 : 0);
    }
    const beforeExternal = hits.length;
    assert.equal(await page.evaluate(() => window.testExternal(true)), true);
    assert.equal(hits.length, beforeExternal + 1);
    assert.equal(await page.evaluate(() => window.testExternal(false)), false, 'An ungranted app cannot reuse the external declaration cache');
    assert.equal(hits.length, beforeExternal + 1, 'No request from the ungranted app');
    const attempt = async (path, host, expected, options = {}) => {
        const before = hits.length;
        const status = await page.evaluate(args => window.run(...args), [path, host, ports[0], !!options.checked]);
        assert.equal(status, expected, host + path);
        assert.equal(hits.length - before, options.hits ?? (expected === 'ok' ? 1 : 0), 'destination hit count for ' + path);
    };
    for (const [host, path, expected] of [
        ['firewall.test','/blue','ok'], ['firewall.test','/blueprint','denied'],
        ['api.firewall.test','/blue/exact-api','ok'], ['api-east.firewall.test','/blue/exact-api','denied'],
        ['api.firewall.test','/red','denied'], ['api.firewall.test','/blue/one','ok'],
        ['deep.api.firewall.test','/blue/one','denied'], ['deep.api.firewall.test','/blue/deep','ok'],
        ['alien.test','/blue/deep','denied'], ['api-east.firewall.test','/blue/a/report-7.json','ok'],
        ['api.firewall.test','/blue/a/report-7.json','denied'], ['api-east.firewall.test','/blue/a/b/report-7.json','denied'],
        ['firewall.test','/blue/a%2Fb','denied'],
    ]) await attempt(path, host, expected);
    const redirect = target => '/blue/redirect?to=' + encodeURIComponent(target);
    const other = 'http://firewall.test:' + ports[1];
    await attempt(redirect(other + '/blue/final'), 'firewall.test', 'denied', { hits: 1 });
    await attempt(redirect(other + '/blue/final'), 'firewall.test', 'ok', { checked: true, hits: 2 });
    await attempt(redirect(other + redirect('/red')), 'firewall.test', 'denied', { checked: true, hits: 2 });
    assert.deepEqual(await page.evaluate(() => window.startLive()), { next: 'revoked', sendDenied: true, denied: 'denied',
        peerStatus: 'ok', stale: 'denied', fresh: 'ok', received: ['ready'], closed: ['grant-revoked'] });
    await Promise.race([streamClosed, new Promise((_, reject) => {
        const timeout = setTimeout(() => reject(Error('Revoked HTTP stream stayed open')), 5000); timeout.unref();
    })]);
}
