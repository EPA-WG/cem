import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { homedir, platform, release, arch, cpus, totalmem } from 'node:os';
import { resolve, join } from 'node:path';
import { chromium } from 'playwright';
import { readDemoInventory } from './demo-deployment.mjs';
import { createDemoSiteHandler } from './serve-demo-site.mjs';
import { summarizeTraffic } from './audit-components-wasm.mjs';
import { browserProcessMemory, summarizeStartup, renderStartupReport } from './browser-runtime-metrics.mjs';
import { prepareComponentBenchmark } from './component-benchmark-deployment.mjs';
import { traceWasmAcquisition, nativeHeapInspection } from './wasm-acquisition-trace.mjs';

const root = resolve(import.meta.dirname, '../..');
const args=process.argv.slice(2).filter(arg=>arg!=='--');
const usage='Usage: node tools/scripts/measure-components-wasm.mjs [deployment | --prepare] [--runs=3] [--case=selected|controls|data|editor] [--trace-wasm] [--write-doc]';
if(args.length===1 && args[0]==='--help') { console.log(usage); process.exit(0); }
assert(args.every(arg=>['--write-doc','--prepare','--trace-wasm'].includes(arg)||/^--runs=\d+$/.test(arg)||/^--case=(selected|controls|data|editor)$/.test(arg)||!arg.startsWith('-')) && args.filter(arg=>!arg.startsWith('-')).length<=1,usage);
assert(args.filter(arg=>arg.startsWith('--runs=')).length<=1 && args.filter(arg=>arg.startsWith('--case=')).length<=1,usage);
const tracing=args.includes('--trace-wasm');
const runs=Number(args.find(arg=>arg.startsWith('--runs='))?.slice(7)??(tracing?1:3));
assert(Number.isInteger(runs)&&runs>=1&&runs<=20,'runs must be between 1 and 20');
const only=args.find(arg=>arg.startsWith('--case='))?.slice(7);
assert(!tracing||only!=='selected','WASM acquisition diagnostics cover the full runtime only');
assert(!args.includes('--prepare')||!args.some(arg=>!arg.startsWith('-')),'--prepare uses its own isolated benchmark destination');
const deployment = resolve(args.includes('--prepare') ? join(root,'dist/fixtures/component-browser-startup') : args.find(arg=>!arg.startsWith('-')) ?? join(homedir(),'cem-bin'));
if(args.includes('--prepare')) await prepareComponentBenchmark(root,deployment);
const output = resolve(root,'dist/reports/components-wasm');
const expected = createHash('sha256').update(await readFile(resolve(root,'packages/cem_ql/dist/wasm/cem_ql_bg.wasm'))).digest('hex');
assert.equal(createHash('sha256').update(await readFile(join(deployment,'packages/cem_ql/dist/wasm/cem_ql_bg.wasm'))).digest('hex'),expected,'deployment/core cohort mismatch');
const inventory=await readDemoInventory(deployment); assert(inventory,'deployment inventory required');
const deployedHashes=new Map(inventory.files.map(file=>[`/${file.path}`,file.sha256]));
const catalog=JSON.parse(await readFile(resolve(root,'packages/cem-components/dist/catalog/cem.components.catalog.json'),'utf8'));
for(const component of catalog.components) {
    assert.deepEqual(await readFile(resolve(root,component.implementation.source)),await readFile(resolve(deployment,component.implementation.source)),`stale declaration ${component.tag}`);
}
const selectedDir=resolve(root,'packages/cem_ql/dist/wasm-selected-application');
const selectedFiles=new Map(await Promise.all(['cem_ql_factory.js','cem_ql_bg.wasm'].map(async name=>[
    `/__component_audit__/selected/${name}`,await readFile(join(selectedDir,name))])));
const selectedSha256=createHash('sha256').update(selectedFiles.get('/__component_audit__/selected/cem_ql_bg.wasm')).digest('hex');
const selectedOptimization=JSON.parse(await readFile(resolve(root,'dist/reports/wasm-optimization/cem-ql-selected-application.json'),'utf8'));
assert.equal(selectedSha256,selectedOptimization.outputSha256,'selected optimizer/artifact mismatch');
selectedFiles.set('/__component_audit__/cart.json',await readFile(resolve(root,'packages/cem_ql/fixtures/component-application/shopping-cart/cart.json')));
const fixtures = {
    selected: { tags:[],html:'' },
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
const selectedRunner = `import {createCemComponentWasmBindings} from './selected/cem_ql_factory.js';
let app;
export async function initialize() {
 const module=await WebAssembly.compileStreaming(fetch(new URL('./selected/cem_ql_bg.wasm',import.meta.url)));
 app=createCemComponentWasmBindings(); await app.initialize(module); app.applicationIdentity();
}
export async function firstUse() {
 app.validateVariables('Cart',new TextEncoder().encode('{"id":"cart-1"}'),'app://shopping-cart/variables.json');
 const bytes=new Uint8Array(await (await fetch(new URL('./cart.json',import.meta.url))).arrayBuffer());
 const html=app.retainResponse('Cart',bytes,'app://shopping-cart/cart.json');
 if(!/Tea.*value="2".*USD 18\\.00/s.test(html)) throw Error('selected cart native render mismatch');
 return true;
}
export function memory() { return {status:'available',endpoint:'selected entry heap only',
 linearMemoryBytes:app.memory().buffer.byteLength,allocator:{status:'unavailable',reason:'selected ABI does not expose allocator live bytes'}}; }`;
const selectedWorker = `import * as api from './selected-runner.mjs';
onmessage=async({data})=> {try {
 if(!['initialize','firstUse','memory'].includes(data.operation)) throw Error('unknown measurement operation');
 postMessage({id:data.id,value:await api[data.operation]()});
 }catch(error){postMessage({id:data.id,error:String(error)});} };`;
const selectedBootstrap = `window.auditWorkerCounters={created:0,ready:0,success:0,failure:0,operations:{}};
const mode=new URL(import.meta.url).searchParams.get('mode');
let api;
if(mode==='worker') {
 const worker=new Worker(new URL('./selected-worker.mjs',import.meta.url),{type:'module'});
 auditWorkerCounters.created++; const pending=new Map(); let next=0;
 const call=operation=>new Promise((resolve,reject)=>{const id=++next;pending.set(id,{resolve,reject,operation});worker.postMessage({id,operation});});
 worker.onmessage=({data})=>{const request=pending.get(data.id);if(!request)return;pending.delete(data.id);
  if(data.error){auditWorkerCounters.failure++;request.reject(Error(data.error));return;}
  if(request.operation==='initialize')auditWorkerCounters.ready++;
  if(request.operation==='firstUse')auditWorkerCounters.success++;
  request.resolve(data.value);};
 api={initialize:()=>call('initialize'),firstUse:()=>call('firstUse'),memory:()=>call('memory')};
} else api=await import('./selected-runner.mjs');
await api.initialize();window.auditSelectedUse=()=>api.firstUse();window.auditSelectedMemory=()=>api.memory();window.auditSelectedReady=true;`;
for(const [path,source] of [['selected-runner.mjs',selectedRunner],['selected-worker.mjs',selectedWorker],['selected-bootstrap.mjs',selectedBootstrap]])
 selectedFiles.set('/__component_audit__/'+path,Buffer.from(source));
function html(fixture, mode) {
    if(fixture===fixtures.selected) return `<!doctype html><html lang="en"><head><meta charset="utf-8"><script type="module" src="/__component_audit__/selected-bootstrap.mjs?mode=${mode}"></script></head><body></body></html>`;
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
    if (selectedFiles.has(url.pathname)) { const bytes=selectedFiles.get(url.pathname); response.writeHead(200,{'content-type':url.pathname.endsWith('.wasm')?'application/wasm':url.pathname.endsWith('.json')?'application/json':'text/javascript','cache-control':'no-store','content-length':bytes.length}).end(bytes); }
    else if (url.pathname === '/__component_audit__/bootstrap.mjs') response.writeHead(200,{'content-type':'text/javascript','cache-control':'no-store'}).end(bootstrap);
    else if (url.pathname === '/__component_audit__/') {
        const fixture = fixtures[url.searchParams.get('case')], mode=url.searchParams.get('mode');
        if (!fixture || !['worker','fallback'].includes(mode)) response.writeHead(400).end();
        else response.writeHead(200,{'content-type':'text/html','cache-control':'no-store'}).end(html(fixture,mode));
    } else if (url.pathname === '/favicon.ico') response.writeHead(204).end();
    else void handler(request,response);
});
await new Promise(done => server.listen(0,'127.0.0.1',done));
const origin=`http://127.0.0.1:${server.address().port}`;
let browser,browserServer,chromiumVersion;
const results=[];
const kinds=Object.keys(fixtures).filter(kind=>(!tracing||kind!=='selected')&&(!only||kind===only));
const plan=[];
for(let run=0;run<runs;run++) { const order=kinds.slice(run%kinds.length).concat(kinds.slice(0,run%kinds.length));
 for(const kind of order) for(const mode of run%2?['fallback','worker']:['worker','fallback']) plan.push({run:run+1,kind,mode}); }
try {
    for (const {run,kind,mode} of plan) {
        const launchStarted=performance.now();
        browserServer=await chromium.launchServer({headless:true,executablePath:process.env.CEM_STUDIO_CHROME});
        browser=await chromium.connect(browserServer.wsEndpoint()); chromiumVersion=browser.version();
        const browserLaunchMs=performance.now()-launchStarted, browserPid=browserServer.process().pid;
        const context=await browser.newContext({viewport:{width:1280,height:800},serviceWorkers:'block'});
        const page=await context.newPage(); page.setDefaultTimeout(30000);
        const blankMemory=await browserProcessMemory(browserPid);
        let phase='cold'; const requestPhases=new WeakMap(), pending=new Set(), responses=[], errors=[];
        context.on('request',request=>requestPhases.set(request,phase));
        context.on('response',response=> {
            const url=new URL(response.url()); if(url.origin!==origin) return;
            const promise=(async()=> {
                const headers=response.headers();
                // Inspect only counters/headers. DevTools body reads can be evicted
                // for both worker scripts and large WASMs; match our served bytes.
                let bytes;
                if(selectedFiles.has(url.pathname)) bytes=selectedFiles.get(url.pathname);
                else if(url.pathname==='/__component_audit__/bootstrap.mjs') bytes=Buffer.from(bootstrap);
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
        const acquisition=tracing?await traceWasmAcquisition(cdp):undefined;
        const memory=async()=> {
            const host=await cdp.send('Runtime.getHeapUsage');
            const native=kind==='selected' ? await page.evaluate(()=>window.auditSelectedMemory()) : mode==='fallback' ? await page.evaluate(async()=> {
                try {
                    const bindings=await import('/packages/cem-elements/dist/lib/internal/runtime-support/native-runtime.js');
                    return {status:'available',endpoint:'selected fallback facade only',sample:JSON.parse(bindings.runtimeMemoryStatus())};
                } catch { return {status:'unavailable',reason:'selected fallback native endpoint not exposed/ready'}; }
            }) : {status:'unavailable',reason:'active worker native counters are not exposed by the current public component host'};
            return {mainThreadJsHeapUsedBytes:host.usedSize,mainThreadJsHeapTotalBytes:host.totalSize,
                mainThreadEmbedderHeapBytes:host.embedderHeapUsedSize, mainThreadBackingStorageBytes:host.backingStorageSize,
                browserFamily:await browserProcessMemory(browserPid),activeWorkers:page.workers().length,native};
        };
        const fence=async()=> {await page.waitForLoadState('networkidle');while(pending.size) await Promise.all([...pending]);};
        const started=performance.now();
        await page.goto(`${origin}/__component_audit__/?case=${kind}&mode=${mode}`);
        if(kind==='selected') await page.waitForFunction(()=>window.auditSelectedReady===true);
        else if(kind==='controls') {
            await page.getByRole('button',{name:'Apply',exact:true}).waitFor();
            await page.getByRole('textbox',{name:'Item',exact:true}).waitFor();
        } else if(kind==='data') await page.locator('cem-data-view > [part~=root]').waitFor();
        else await page.getByRole('textbox',{name:'Source',exact:true}).waitFor();
        const readyMs=performance.now()-started; await fence(); const coldMemory=await memory();
        phase='first-use'; const used=performance.now();
        if(kind==='selected') assert.equal(await page.evaluate(()=>window.auditSelectedUse()),true);
        else if(kind==='controls') {
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
        let acquisitionEvidence;
        if(acquisition) {
            const before=acquisition.requests.length;
            const main=await page.evaluate(nativeHeapInspection);
            assert.equal(acquisition.requests.length,before,'page inspection caused WASM acquisition');
            const workers=await acquisition.inspectWorkers(page,nativeHeapInspection);
            await fence();
            acquisitionEvidence={page:main,workers,requests:acquisition.finish(),timing:'diagnostic debugger instrumentation; excluded from startup distributions'};
            assert.equal(acquisitionEvidence.requests.length,responses.filter(response=>response.path.split('?')[0].endsWith('.wasm')).length,'realm acquisition ledger differs from observed transfers');
        }
        const traffic=summarizeTraffic(responses);
        assert(traffic.uniqueWasm.length>0,'native rendering must load WASM');
        for(const wasm of traffic.uniqueWasm) assert.equal(wasm.sha256,kind==='selected'?selectedSha256:expected,'unexpected second/mismatched runtime');
        const workerCounters=await page.evaluate(()=>window.auditWorkerCounters);
        if(mode==='worker') {assert(workerCounters.ready>0 && workerCounters.success>0,'default worker must actually process successful jobs');assert.equal(workerCounters.failure,0);}
        else assert.equal(workerCounters.created,0,'fallback fixture must not create a worker');
        if(mode==='worker') assert(coldMemory.activeWorkers>0 && afterMemory.activeWorkers>0,'a live dedicated worker is required');
        else assert.equal(afterMemory.activeWorkers,0,'fallback must have no active worker');
        const result={kind,mode,run,acquisitionEvidence,browserPid,browserLaunchMs,blankMemory,samples:1,readyMs,firstUseMs,coldMemory,afterMemory,workerCounters,traffic,
            phases:Object.fromEntries(['cold','first-use'].map(name=>[name,summarizeTraffic(responses.filter(r=>r.phase===name))])),responses};
        results.push(result);
        console.log(`${kind}/${mode} run ${run}: ready ${Math.round(readyMs)} ms, first use ${Math.round(firstUseMs)} ms, ${traffic.wasmRequests} WASM requests / ${traffic.uniqueWasm.length} unique file.`);
        await cdp.detach();await context.close();await browser.close();await browserServer.close();browser=undefined;browserServer=undefined;
    }
    await mkdir(output,{recursive:true});
    const report={schemaVersion:2,scope:'selected-native-entry-and-separate-full-runtime-workloads',runs,
        transport:'assembled-demo deduplicated adapters; local no-store HTTP; selected original compiled cart/JSON API fixture',
        coreSha256:expected,selectedSha256,chromium:chromiumVersion,node:process.version,
        platform:`${platform()} ${release()} ${arch()}`,cpu:cpus()[0]?.model,cpuCount:cpus().length,hostMemoryBytes:totalmem(),
        measuredAt:new Date().toISOString(),
        harnessHashes:Object.fromEntries(await Promise.all(['measure-components-wasm.mjs','browser-runtime-metrics.mjs','component-benchmark-deployment.mjs',...(tracing?['wasm-acquisition-trace.mjs']:[])].map(async name=>[
            name,createHash('sha256').update(await readFile(resolve(root,'tools/scripts',name))).digest('hex')]))),
        fixtureHashes:Object.fromEntries([...selectedFiles].map(([path,bytes])=>[path,createHash('sha256').update(bytes).digest('hex')])),
        methods:{cold:'fresh browser process/profile/context per observation; OS/server caches may be warm; browser launch separate',
            timing:'automation wall time; rotated workload/mode order; descriptive median/min/max; no percentile or budget claim',
            memory:'browser-family Linux smaps_rollup PSS, stable process identities/membership; main JS and native capacity separate, never added to PSS',
            sampling:'snapshots after network/render idle outside timed intervals; no explicit GC; diagnostic work occurs between readiness and first use',
            workers:'live dedicated workers observed by Playwright, separate from creation/job counters and OS processes'},
        summary:tracing?[]:summarizeStartup(results),results};
    await writeFile(join(output,tracing?'browser-wasm-acquisition.json':'browser-startup.json'),`${JSON.stringify(report,null,2)}\n`);
    if(tracing&&args.includes('--write-doc')) {
        const evidence={contract:'cem-wasm-acquisition-diagnosis/1',coreSha256:expected,chromium:chromiumVersion,node:process.version,measuredAt:report.measuredAt,harnessHashes:report.harnessHashes,
            sourceHashes:Object.fromEntries(results.flatMap(row=>row.responses.filter(response=>response.path.split('?')[0].endsWith('.js')||response.path.split('?')[0].endsWith('.mjs')).map(response=>[response.path,response.sha256]))),
            limits:['Debugger instrumentation invalidates latency comparisons.','Worker stacks unavailable; original performance resource entries and initialized native exports identify worker acquisitions.'],
            results:results.map(row=>({kind:row.kind,mode:row.mode,run:row.run,workers:row.afterMemory.activeWorkers,workerCounters:row.workerCounters,traffic:row.traffic,evidence:row.acquisitionEvidence}))};
        await writeFile(resolve(root,'docs/cem-components-wasm-acquisition.json'),`${JSON.stringify(evidence,null,2)}\n`);
    }
    if(!tracing&&args.includes('--write-doc')) {
        await writeFile(resolve(root,'docs/cem-components-wasm-startup.md'),renderStartupReport(report));
        const compact = sample => { const {processes,...aggregate}=sample; return aggregate; };
        const evidence={...report,results:results.map(row=>{
            const {responses,...sample}=row;
            return {...sample,blankMemory:compact(row.blankMemory),
                coldMemory:{...row.coldMemory,browserFamily:compact(row.coldMemory.browserFamily)},
                afterMemory:{...row.afterMemory,browserFamily:compact(row.afterMemory.browserFamily)}};
        })};
        await writeFile(resolve(root,'docs/cem-components-wasm-startup.json'),`${JSON.stringify(evidence,null,2)}\n`);
    }
} finally {await browser?.close();await browserServer?.close();server.closeAllConnections();await new Promise(done=>server.close(done));}
