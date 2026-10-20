import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { createCemDeclarationScope } from './declaration-scope.js';
import type { DataIslandSnapshot } from './cem-elements.js';
import { preparePatchFramesForRange } from './projection.js';
import { cemProcessingHostForScope } from './internal/runtime-support/processing-host-runtime.js';
import { createCemProcessingTextSource, type CemProcessingRenderDiffInput } from './internal/runtime-support/processing-host.js';
export default {title:'CEM Elements/Retained Pure Render Reuse', tags:['test']} satisfies Meta;
type Story = StoryObj;
let sequence = 0;
function scenario(fallback: boolean): Story {
    return { render: () => '<section aria-label="Retained pure render reuse"></section>', play: async ({canvasElement}) => {
        const canvas = canvasElement.querySelector('section')!;
        const scope = createCemDeclarationScope({document});
        const host = cemProcessingHostForScope(scope, {
            workerScriptUrl:new URL('./internal/runtime-support/processing-worker.js', import.meta.url),
            workerFactory:request => {if (fallback) throw new Error('fixture selects fallback'); return new Worker(request.scriptUrl,{type:request.type,name:request.name});},
            poolPolicy:{workerCount:1,maxWorkers:1},
        });
        const tag = `cem-pure-${++sequence}`, templateArtifactId = `${tag}-template`;
        const source = '{attribute @name=notice @select=\'"ready"\'}{http-request @slice=page @url=./records.json}{input @value=Original}{p | {$datadom.attributes.label}}';
        const compiled = await host.compile({language:'cem-ml', producedTag:tag, templateArtifactId, registrationIdentity:`registration:${tag}`,
            source:createCemProcessingTextSource(source), sourceRef:{kind:'inline',value:tag}, resolverIdentity:'test', scopePolicyStamp:'policy', sourceMapMode:'dev'}).result;
        const snapshot: DataIslandSnapshot = {instanceId:`${tag}-instance`, producedTag:tag, declarationTag:'cem-element', templateArtifactId,
            dataRevision:'same-revision', outputTarget:'light-dom', scopePolicyStamp:'policy', privacyPolicyStamp:'privacy', sourceMapMode:'dev',
            hostAttributes:{}, dataset:{}, slices:{}, validationState:{}, eventPayloads:{},
            payload:{roots:[],byKey:{},choices:[],options:[],data:[],dataByValue:{},optionsByValue:{},elementsByAttribute:{},slots:{},text:''}};
        const input: CemProcessingRenderDiffInput = {artifact:compiled.artifact, snapshot,
            revision:{instanceId:snapshot.instanceId, templateArtifactId, dataRevision:snapshot.dataRevision, scopePolicyStamp:'policy', outputTarget:'light-dom'},
            data:{label:'Ada'}, scopeUid:tag};
        const bounds = {start:document.createComment('start'),end:document.createComment('end')};
        canvas.append(bounds.start,bounds.end);
        try {
            const mutable = structuredClone(input);
            const pending = host.renderDiff(mutable);
            mutable.data.label = 'Changed after dispatch';
            mutable.revision.dataRevision = 'mutated';
            const first = await pending.result;
            expect(first.revision.dataRevision).toBe('same-revision');
            expect(first.renderReuse?.hit).toBe(false);
            expect(preparePatchFramesForRange(bounds,first.frames,first.revision,document).commit(first.revision).status).toBe('applied');
            expect(canvas.textContent).toContain('Ada');
            expect(canvas.textContent).not.toContain('Changed after dispatch');
            const control = canvas.querySelector('input')!;
            control.value = 'User edit'; control.focus(); control.setSelectionRange(1,4);
            const hit = await host.renderDiff({...input, previousRenderPlan:first.nextRenderPlan}).result;
            expect(hit.renderReuse?.hit).toBe(true);
            expect(hit.renderReuse?.evaluations).toBe(first.renderReuse?.evaluations);
            expect(hit.renderReuse?.admissions).toBe(first.renderReuse!.admissions + 1);
            expect(hit.resourceControls).toHaveLength(1);
            expect(hit.resourceControls).toEqual(first.resourceControls);
            expect(hit.hostAttributeUpdates).toEqual([{name:'notice',value:'ready'}]);
            expect(preparePatchFramesForRange(bounds,hit.frames,hit.revision,document).commit(hit.revision).status).toBe('applied');
            expect(canvas.querySelector('input')).toBe(control); expect(control.value).toBe('User edit');
            expect(document.activeElement).toBe(control); expect([control.selectionStart,control.selectionEnd]).toEqual([1,4]);
            // Same revision text cannot claim unchanged native inputs.
            const changed = await host.renderDiff({...input, data:{label:'Lin'}, previousRenderPlan:hit.nextRenderPlan}).result;
            expect(changed.renderReuse?.hit).toBe(false);
            expect(preparePatchFramesForRange(bounds,changed.frames,changed.revision,document).commit(changed.revision).status).toBe('applied');
            expect(canvas.textContent).toContain('Lin'); expect(control.value).toBe('User edit');
            const job = host.renderDiff({...input,data:{label:'cancelled'},previousRenderPlan:changed.nextRenderPlan});
            const rejected = expect(job.result).rejects.toThrow('cancelled');
            await host.cancel({targetJobId:job.jobId,reason:'superseded'}).result;
            await rejected;
            expect(canvas.textContent).not.toContain('cancelled');
            expect(host.mode).toBe(fallback ? 'main-thread' : 'worker');
        } finally { await host.dispose({reason:'scope-disposed'}).result; scope.dispose(); canvas.replaceChildren(); }
    }};
}
export const WorkerAdmission: Story = scenario(false);
export const FallbackAdmission: Story = scenario(true);
