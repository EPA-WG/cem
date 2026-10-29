import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { homedir } from 'node:os';
import { resolve, join } from 'node:path';
import { chromium } from 'playwright';
import { readDemoInventory } from './demo-deployment.mjs';
import { createDemoSiteHandler } from './serve-demo-site.mjs';
import { summarizeTraffic, renderBrowserBaseline } from './audit-components-wasm.mjs';

const root = resolve(import.meta.dirname, '../..');
const args=process.argv.slice(2);
assert(args.every(arg=>arg==='--write-doc'||!arg.startsWith('-')) && args.filter(arg=>!arg.startsWith('-')).length<=1, 'Usage: node tools/scripts/measure-components-wasm.mjs [deployment] [--write-doc]');
const deployment = resolve(args.find(arg=>!arg.startsWith('-')) ?? join(homedir(),'cem-bin'));
const output = resolve(root,'dist/reports/components-wasm');
const expected = createHash('sha256').update(await readFile(resolve(root,'packages/cem_ql/dist/wasm/cem_ql_bg.wasm'))).digest('hex');
assert.equal(createHash('sha256').update(await readFile(join(deployment,'packages/cem_ql/dist/wasm/cem_ql_bg.wasm'))).digest('hex'),expected,'deployment/core cohort mismatch');
const inventory=await readDemoInventory(deployment); assert(inventory,'deployment inventory required');
const deployedHashes=new Map(inventory.files.map(file=>[`/${file.path}`,file.sha256]));
const catalog=JSON.parse(await readFile(resolve(root,'packages/cem-components/dist/catalog/cem.components.catalog.json'),'utf8'));
for(const component of catalog.components) {
    assert.deepEqual(await readFile(resolve(root,component.implementation.source)),await readFile(resolve(deployment,component.implementation.source)),`stale declaration ${component.tag}`);
}
const fixtures = {
    controls: { tags:['cem-action','cem-text-field'], html:'<form><cem-text-field label="Item" name="item" value="Sample"></cem-text-field><cem-action label="Apply" type="button"></cem-action></form>' },
    data: { tags:['cem-data-view'], html:'<cem-data-view label="Inventory" format="json" value=""></cem-data-view>' },
    editor: { tags:['cem-source-editor'], html:'<cem-source-editor label="Source" content-type="text/plain"></cem-source-editor>' },
};
const bootstrap = `import { installCemElementRuntime } from '/packages/cem-elements/dist/index.js';
const fallback = new URL(import.meta.url).searchParams.get('mode') === 'fallback';
window.auditWorkerCounters = { created:0,ready:0,success:0,failure:0,operations:{} };
window.auditRuntime = installCemElementRuntime(window, { processingPoolPolicy: { queueSize:512 },
 processingWorkerFactory: ({scriptUrl,name}) => {
  if(fallback) throw Error('fixture selects fallback');
  const worker=new Worker(scriptUrl,{name,type:'module'}); window.auditWorkerCounters.created++;
  worker.addEventListener('message',({data})=> {
   if(data?.direction==='ready' && data.mode==='worker') window.auditWorkerCounters.ready++;
   if(data?.direction==='response') {
    if(data.outcome==='success') {
     window.auditWorkerCounters.success++;
     if(typeof data.operation==='string' && data.operation.length<64) window.auditWorkerCounters.operations[data.operation]=(window.auditWorkerCounters.operations[data.operation]??0)+1;
    } else window.auditWorkerCounters.failure++;
   }
  }); return worker;
 } });`;
function html(fixture, mode) {
    return `<!doctype html><html lang="en" class="cem-theme-light"><head><meta charset="utf-8">
<link rel="stylesheet" href="/packages/cem-theme/dist/lib/css/cem-combined.css">
<link rel="stylesheet" href="/packages/cem-components/dist/styles.css">
<script type="module" src="/__component_audit__/bootstrap.mjs?mode=${mode}"></script></head><body>
<cem-element hidden><template type="text/cem-ml">${fixture.tags.map(tag => `{cem-element @tag=${tag} @src="/packages/cem-components/src/components/${tag}/${tag}.xhtml#${tag}"}`).join('\n')}</template></cem-element>
<main>${fixture.html}</main></body></html>`;
}
const handler = createDemoSiteHandler({root:deployment});
const server = createServer((request,response) => {
    const url = new URL(request.url,'http://fixture.test');
    if (url.pathname === '/__component_audit__/bootstrap.mjs') response.writeHead(200,{'content-type':'text/javascript','cache-control':'no-store'}).end(bootstrap);
    else if (url.pathname === '/__component_audit__/') {
        const fixture = fixtures[url.searchParams.get('case')], mode=url.searchParams.get('mode');
        if (!fixture || !['worker','fallback'].includes(mode)) response.writeHead(400).end();
        else response.writeHead(200,{'content-type':'text/html','cache-control':'no-store'}).end(html(fixture,mode));
    } else if (url.pathname === '/favicon.ico') response.writeHead(204).end();
    else void handler(request,response);
});
await new Promise(done => server.listen(0,'127.0.0.1',done));
const origin=`http://127.0.0.1:${server.address().port}`;
const browser=await chromium.launch({headless:true,executablePath:process.env.CEM_STUDIO_CHROME});
const results=[];
try {
    for (const mode of ['worker','fallback']) for (const kind of Object.keys(fixtures)) {
        const context=await browser.newContext({viewport:{width:1280,height:800},serviceWorkers:'block'});
        const page=await context.newPage(); page.setDefaultTimeout(30000);
        let phase='cold'; const requestPhases=new WeakMap(), pending=new Set(), responses=[], errors=[];
        context.on('request',request=>requestPhases.set(request,phase));
        context.on('response',response=> {
            const url=new URL(response.url()); if(url.origin!==origin) return;
            const promise=(async()=> {
                const headers=response.headers();
                // Inspect only counters/headers. DevTools body reads can be evicted
                // for both worker scripts and large WASMs; match our served bytes.
                let bytes;
                if(url.pathname==='/__component_audit__/bootstrap.mjs') bytes=Buffer.from(bootstrap);
                else if(url.pathname==='/__component_audit__/') bytes=Buffer.from(html(fixtures[url.searchParams.get('case')],url.searchParams.get('mode')));
                else if(url.pathname==='/favicon.ico') bytes=Buffer.alloc(0);
                else {
                    const path=resolve(deployment, `.${decodeURIComponent(url.pathname)}`);
                    assert(path.startsWith(`${deployment}/`)); bytes=await readFile(path);
                }
                const bodyHashSource='served artifact/fixture; inventory hash and response length verified when present';
                const responseHash=createHash('sha256').update(bytes).digest('hex');
                if(!url.pathname.startsWith('/__component_audit__/') && url.pathname!=='/favicon.ico') assert.equal(responseHash,deployedHashes.get(url.pathname),`deployment inventory drift: ${url.pathname}`);
                if(headers['content-length']) assert.equal(bytes.length,Number(headers['content-length']));
                assert(!headers['content-encoding'],'compressed response needs a decoded-size measurement');
                const sizes=await response.request().sizes().catch(()=>null);
                responses.push({path:`${url.pathname}${url.search}`, phase:requestPhases.get(response.request()), status:response.status(),
                    contentType:headers['content-type']??null, contentEncoding:headers['content-encoding']??null,bodyHashSource,
                    bodyBytes:bytes.length,wireBodyBytes:sizes?.responseBodySize??null,sha256:responseHash});
            })().catch(error=>errors.push(`network: ${error.message}`)).finally(()=>pending.delete(promise));
            pending.add(promise);
        });
        page.on('pageerror',error=>errors.push(error.message));
        const cdp=await context.newCDPSession(page);
        const memory=async()=> {
            const host=await cdp.send('Runtime.getHeapUsage');
            const native=mode==='fallback' ? await page.evaluate(async()=> {
                try {
                    const bindings=await import('/packages/cem-elements/dist/lib/internal/runtime-support/native-runtime.js');
                    return {status:'available',endpoint:'selected fallback facade only',sample:JSON.parse(bindings.runtimeMemoryStatus())};
                } catch { return {status:'unavailable',reason:'selected fallback native endpoint not exposed/ready'}; }
            }) : {status:'unavailable',reason:'active worker native counters are not exposed by the current public component host'};
            return {mainThreadJsHeapUsedBytes:host.usedSize,mainThreadJsHeapTotalBytes:host.totalSize,
                aggregateHostMemory:{status:'unavailable',reason:'main-thread JS heap is not total browser/worker/WASM memory'},native};
        };
        const fence=async()=> {await page.waitForLoadState('networkidle');while(pending.size) await Promise.all([...pending]);};
        const started=performance.now();
        await page.goto(`${origin}/__component_audit__/?case=${kind}&mode=${mode}`);
        if(kind==='controls') {
            await page.getByRole('button',{name:'Apply',exact:true}).waitFor();
            await page.getByRole('textbox',{name:'Item',exact:true}).waitFor();
        } else if(kind==='data') await page.locator('cem-data-view > [part~=root]').waitFor();
        else await page.getByRole('textbox',{name:'Source',exact:true}).waitFor();
        const readyMs=performance.now()-started; await fence(); const coldMemory=await memory();
        phase='first-use'; const used=performance.now();
        if(kind==='controls') {
            await page.getByRole('textbox',{name:'Item',exact:true}).fill('Edited');
            await page.getByRole('button',{name:'Apply',exact:true}).click();
            assert.equal(await page.locator('form').evaluate(form=>new FormData(form).get('item')),'Edited');
        } else if(kind==='data') {
            await page.locator('cem-data-view').evaluate(host=>host.setAttribute('value','{"item":"Tea","qty":2}'));
            await page.locator('cem-data-view [part~=value][data-kind=string]').filter({hasText:'Tea'}).waitFor();
            await page.locator('cem-data-view').evaluate(host=>{host.setAttribute('format','csv');host.setAttribute('value','item,qty\nBeans,3\n');});
            await page.locator('cem-data-view [part~=value][data-kind=string]').filter({hasText:'Beans'}).waitFor();
        } else {
            const control=page.getByRole('textbox',{name:'Source',exact:true});
            await control.fill('{p @class="note" | Hello}');
            await control.evaluate(element=>{element.focus(); const range=document.createRange();range.setStart(element.firstChild,0);range.setEnd(element.firstChild,2);const selection=window.getSelection();selection.removeAllRanges();selection.addRange(range);});
            await page.locator('cem-source-editor').evaluate(host=>host.setAttribute('content-type','text/cem-ml'));
            await page.waitForFunction(()=>[...CSS.highlights.keys()].some(name=>name.startsWith('cem-source-')));
            assert.equal(await control.innerText(),'{p @class="note" | Hello}');
            assert.equal(await page.evaluate(()=>window.getSelection().toString()),'{p');
        }
        const firstUseMs=performance.now()-used; await fence(); const afterMemory=await memory();
        assert.deepEqual(errors,[],`${kind}/${mode}`);
        assert(responses.every(response=>response.status<400),JSON.stringify(responses.filter(r=>r.status>=400)));
        const traffic=summarizeTraffic(responses);
        assert(traffic.uniqueWasm.length>0,'native rendering must load WASM');
        for(const wasm of traffic.uniqueWasm) assert.equal(wasm.sha256,expected,'unexpected second/mismatched runtime');
        const workerCounters=await page.evaluate(()=>window.auditWorkerCounters);
        if(mode==='worker') {assert(workerCounters.ready>0 && workerCounters.success>0,'default worker must actually process successful jobs');assert.equal(workerCounters.failure,0);}
        else assert.equal(workerCounters.created,0,'fallback fixture must not create a worker');
        const result={kind,mode,samples:1,readyMs,firstUseMs,coldMemory,afterMemory,workerCounters,traffic,
            phases:Object.fromEntries(['cold','first-use'].map(name=>[name,summarizeTraffic(responses.filter(r=>r.phase===name))])),responses};
        results.push(result);
        console.log(`${kind}/${mode}: ready ${Math.round(readyMs)} ms, first use ${Math.round(firstUseMs)} ms, ${traffic.wasmRequests} WASM requests / ${traffic.uniqueWasm.length} unique file.`);
        await cdp.detach();await context.close();
    }
    await mkdir(output,{recursive:true});
    const report={schemaVersion:1,scope:'current-full-profile-only',transport:'assembled-demo deduplicated adapters; matching canonical WASM; no Studio/demo-viewer bootstrap',coreSha256:expected,chromium:browser.version(),node:process.version,
        methods:{cold:'fresh isolated context in a shared browser process (code caches may be reused), default module worker or explicit unsupported-worker fallback; no demo/Studio wrappers',
            timing:'end-to-end automation wall time; one observation per workload/mode, no statistical or minimum-size claim',
            network:'decoded bodies and wire body sizes separated; current local no-store server; unique URLs separate from repeated fetches; body hashes/sizes come from matching served artifacts/fixtures with response length validation; response body size from browser API is separate; no inspector payload retention',
            memory:'main-thread CDP JS heap counters; selected fallback native sample where ready, active-worker and aggregate memory explicitly unavailable'},results};
    await writeFile(join(output,'browser-baseline.json'),`${JSON.stringify(report,null,2)}\n`);
    if(args.includes('--write-doc')) await writeFile(resolve(root,'docs/cem-components-wasm-baseline.md'),renderBrowserBaseline(report));
} finally {await browser.close();server.closeAllConnections();await new Promise(done=>server.close(done));}
