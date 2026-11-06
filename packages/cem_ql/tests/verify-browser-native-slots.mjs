import assert from 'node:assert/strict';
import http from 'node:http';
import { mkdirSync, writeFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
const root = path.resolve(fileURLToPath(new URL('../../../', import.meta.url)));
const server = http.createServer(async (req, res) => {
    try {
        const pathname = new URL(req.url, 'http://localhost').pathname;
        if (pathname === '/fixture') { res.setHeader('Content-Type', 'text/html'); res.end('<!doctype html><html><body></body></html>'); return; }
        const vendor = '/packages/cem-elements/dist/lib/internal/runtime-support/vendor/';
        const local = pathname.startsWith(vendor) && ['cem_ql.js', 'cem_ql_bg.wasm'].includes(path.basename(pathname))
            ? `/packages/cem_ql/dist/wasm/${path.basename(pathname)}` : pathname;
        const file = path.resolve(root, `.${decodeURIComponent(local)}`);
        assert.ok(file.startsWith(`${root}${path.sep}`));
        res.setHeader('Content-Type', { '.js': 'text/javascript', '.mjs': 'text/javascript', '.json': 'application/json', '.wasm': 'application/wasm' }[path.extname(file)] || 'application/octet-stream');
        res.end(await readFile(file));
    } catch (error) { console.error(`Fixture asset unavailable: ${req.url}: ${error.message}`); res.writeHead(404); res.end('missing'); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
let browser;
try {
    browser = await chromium.launch({headless:true,executablePath:process.env.CEM_CHROME});
    const origin=`http://127.0.0.1:${server.address().port}`;
    const results=[];
    for (const profile of ['fallback-plan','fallback-legacy','fallback-retained','worker-retained']) {
        const page=await browser.newPage();
        await page.goto(`${origin}/fixture`);
        let timer;
        try {
            const result=await Promise.race([page.evaluate(async profile=>{
                const {CemElementRuntime}=await import('/packages/cem-elements/dist/lib/cem-elements.js');
                const trace=[];
                const runtime=new CemElementRuntime({declarationTag:'test-declaration',onProcessingTrace:e=>trace.push(e),
                    ...(profile.startsWith('fallback')?{processingWorkerFactory:()=>{throw Error('fixture fallback');}}:{}),
                    ...(profile.endsWith('plan')||profile.endsWith('legacy')?{retainedStylesheets:false}:{})});
                runtime.install(window);
                runtime.usesProcessingHost=()=>!profile.endsWith('plan');
                const container=document.createElement('div');
                container.innerHTML='<test-declaration tag="x-native-slots"><template id="x-native-slots" type="text/cem-ml">{section @part=heading | {$ datadom.slots.heading}}{section @part=again | {$ datadom.slots.heading}}</template></test-declaration><x-native-slots><template><b slot="heading">One &amp; two</b><!--retained--><i slot="heading">&lt;three&gt;</i><style>:scope b { color: rgb(7, 8, 9); }</style><template slot="heading"><style>b { color: red; }</style><u>Inert</u></template></template></x-native-slots>';
                const host=container.querySelector('x-native-slots');
                const original=Array.from(host.querySelector('template').content.childNodes);
                document.body.append(container);
                const declaration=container.querySelector('test-declaration');
                runtime.registerDeclaration(declaration);
                await runtime.whenDeclarationSettled(declaration); await runtime.whenRenderSettled(host);
                const check=(value,message)=>{if(!value)throw Error(`${profile}: ${message}; diagnostics=${JSON.stringify(runtime.diagnosticsFor(host))}`);};
                const island=host.querySelector('template[data-cem-island]');
                const payload=Array.from(island.content.firstElementChild.children).find(n=>n.localName.endsWith(':payload'));
                const sections=()=>Array.from(host.querySelectorAll('section'));
                check(sections().length===2,'missing named slot output');
                check(sections().every(n=>n.textContent==='One & two<three>'),'decoded ordered repeated slots');
                check(original.every((n,i)=>payload.childNodes[i]===n),'original payload nodes moved or replaced');
                check(sections().every(n=>n.querySelector('template')?.content.querySelector('u')?.textContent==='Inert'),'nested inert template content lost');
                check(host.querySelectorAll('section style').length===0,'payload style duplicated into rendered content');
                check(getComputedStyle(host.querySelector('section b')).color==='rgb(7, 8, 9)','payload stylesheet lost ownership');
                const produced=host.querySelector('section b');
                check(produced.getAttribute('data-cem-source-fidelity')==='dom-canonical','DOM origin falsely labelled author bytes');
                check(produced.getAttribute('data-cem-source-frame')?.includes('cem-dom://'),'runtime DOM source identity lost');
                // The public observer must recapture content and slot reassignment.
                original[0].firstChild.textContent='Changed & literal';
                original[2].setAttribute('slot','other');
                const deadline=performance.now()+10000;
                while(sections().some(n=>n.textContent!=='Changed & literal')&&performance.now()<deadline) {
                    await new Promise(resolve=>setTimeout(resolve,10)); await runtime.whenRenderSettled(host);
                }
                check(sections().every(n=>n.textContent==='Changed & literal'),'payload mutation did not replace stale native input');
                check(original.every((n,i)=>payload.childNodes[i]===n),'recapture replaced canonical input');
                check(original[0]!==host.querySelector('section b'),'projection adopted inert source node');
                // Exercise lowered capture limits and verify one-shot native cleanup.
                const helpers=runtime.renderSupport(runtime.declarationForInstance(host).declarationScope);
                const native=await import('/packages/cem-elements/dist/lib/internal/runtime-support/native-runtime.js');
                const liveValues=()=>JSON.parse(native.commandNativeResourceInventory()).registries.find(n=>n.name==='native-values').live;
                const before=liveValues();
                const handle=native.beginDomPayloadCapture('cem-dom://fixture/abandoned',document.URL,document.baseURI,JSON.stringify({maxBytes:1048576,maxValues:100,maxDepth:64}));
                check(liveValues()===before+1,'open capture missing from retirement inventory');
                check(native.disposeDomPayloadCapture(handle),'abandoned capture not disposed');
                let refused=false;
                try {await helpers.captureDomPayload(payload,{maxBytes:1048576,maxValues:1,maxDepth:64});} catch {refused=true;}
                check(refused,'lowered native capture bounds ignored');
                check(liveValues()===before,'failed or successful capture leaked its session');
                if(profile.startsWith('worker'))check(trace.some(e=>e.kind==='dispatch')&&!trace.some(e=>e.kind==='fallback'),'worker profile silently fell back');
                else if(!profile.endsWith('plan'))check(trace.some(e=>e.kind==='fallback'),'fallback profile was not exercised');
                const unexpected=runtime.diagnosticsFor(host).filter(d=>!(profile.startsWith('fallback')&&d.code==='cem.processing_host.worker_startup_fallback'&&d.severity==='warning'));
                check(unexpected.length===0,'unexpected renderer diagnostics');
                return {profile,repeatedSlots:true,decodedValues:true,mutationRecaptured:true,sourceDomIdentityRetained:true,sourceProvenance:true,stylesheetOwner:true,failedCaptureDisposed:true};
            },profile),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error(`${profile}: retained-slot verification timed out`)),60000);})]);
            results.push(result);
        } finally {clearTimeout(timer);await page.close();}
    }
    const report=path.join(root,'dist/reports/cem-browser-native-slots.json');
    mkdirSync(path.dirname(report),{recursive:true});
    writeFileSync(report,`${JSON.stringify({browser:browser.version(),results},null,2)}\n`);
    console.log(`Browser retained native slots: ${results.length}/${results.length} profiles`);
} finally {await browser?.close();await new Promise(resolve=>server.close(resolve));}
