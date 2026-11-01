import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {Worker,isMainThread,parentPort,workerData} from 'node:worker_threads';
async function load(directory, name) {
    const m=await import(`${pathToFileURL(resolve(directory,'query_source_probe.js')).href}?rule=${name}`);
    m.initSync({module:await readFile(resolve(directory,'query_source_probe_bg.wasm'))});return m;
}
async function exercise(directory) {
    const a=await load(directory,'a'), b=await load(directory,'b');
    for (const native of [false,true]) for (const imported of [false,true]) {
        const name=`rule-${native?'native':'query'}-${imported?'imported':'direct'}`;
        const bytes=new Uint8Array(await readFile(resolve(directory,`${name}.bin`))), pins=await readFile(resolve(directory,`${name}.json`),'utf8');
        assert.throws(()=>a.ruleSourceOpenV1(bytes,pins));
        a.probeRuleProvision(pins); b.probeRuleProvision(pins);
        const owner=a.ruleSourceOpenV1(bytes,pins), other=b.ruleSourceOpenV1(bytes,pins);
        assert.ok(Object.isFrozen(owner));assert.throws(()=>a.ruleSourceCaptureV1({...owner}));assert.throws(()=>b.ruleSourceReleaseV1(owner));assert.throws(()=>a.ruleSourceCaptureV1(other));
        assert.deepEqual(a.ruleSourceCaptureV1(owner),bytes);
        if (imported) assert.throws(()=>a.probeRuleValidate(owner,'same'),'imported rule fabricated a consumer datatype');
        else {
            assert.equal(JSON.parse(a.probeRuleValidate(owner,'same')).accepted,true);
            const canary='entered-rule-credential-4ac6e0c3';a.probeRuleValidate(owner,canary);
            assert.equal(Buffer.from(a.ruleSourceCaptureV1(owner)).includes(Buffer.from(canary)),false);
        }
        const changed=JSON.parse(pins); changed.profile.profile.diagnostic.local+='other';
        assert.throws(()=>a.ruleSourceOpenV1(bytes,JSON.stringify(changed)));
        assert.deepEqual(a.ruleSourceCaptureV1(owner),bytes);
        const before=JSON.parse(a.probeInventory()).liveRoots;
        a.probeRuleFence();assert.throws(()=>a.ruleSourceCaptureV1(owner));
        assert.equal(JSON.parse(a.probeInventory()).liveRoots,before,'retirement hid retained native roots');
        a.probeRuleUnprovision(); a.ruleSourceReleaseV1(owner);
        b.ruleSourceReleaseV1(other);b.probeRuleUnprovision();
        assert.equal(JSON.parse(a.probeInventory()).liveRoots,0);assert.equal(JSON.parse(b.probeInventory()).liveRoots,0);
    }
}
if (!isMainThread) {try {await exercise(workerData.directory);parentPort.postMessage({ok:true});}catch(e){parentPort.postMessage({error:String(e.stack??e)});}}
else {
    await exercise(process.argv[2]);
    const worker=new Worker(new URL(import.meta.url),{workerData:{directory:process.argv[2]}});
    try {await new Promise((ok,fail)=>{const t=setTimeout(()=>fail(Error('rule worker deadline')),5000);worker.once('error',fail);worker.once('message',r=>{clearTimeout(t);r.ok?ok():fail(Error(r.error));});});}finally{await worker.terminate();}
    console.log('Custom profile and direct/imported query/native rule leaves passed independent heaps and a real worker, with original code/tokens and no fabricated consumers.');
}
