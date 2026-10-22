import { createCemMlWasmGeneration, type CemMlWasmGeneration } from '@epa-wg/cem-ml/wasm/factory';
import type { CommandInvocationV1, CommandPresentationPlanV1, CommandPresentationV1, CommandServiceResultV1 } from '@epa-wg/cem-ml/wasm';
import type { ParsedCemMlCommand } from './command.js';
import { buildCommandInvocationWith, projectCommandPresentationWith, type BrowserCommandInvocationOptions, type BrowserCommandInvocationResolver, type InvocationRuntime } from './invocation-core.js';
import { CommandNativeAdmission, CommandNativeClientCheckpoint, type CommandNativeDrain } from './native-admission.js';
import { parseCommandNativeInventory, type CommandNativeEmbeddingOwnership, type CommandNativeRetirementHandoff } from './native-embedding.js';
import { registerNativeGeneration, completeNativeRetirement, claimNativeGenerationAdmission, issueNativeGenerationBindingTicket, type CemNativeGenerationBindingTicket, type CemNativeGenerationParticipant, type CemNativeGenerationAdmission, type CemNativeGenerationRetirement, type CemNativeRecoveryScope, type CemNativeRecoveryEnrollment } from './generation-lifecycle.js';

/** Pure native authoring only: no command execution, artifact handles or host writes.
 * Captured methods belong to this exact physical generation and never retarget. */
export class OwnedBrowserCommandInvocation {
    #runtimeId = `browser-invocation:${crypto.randomUUID()}`;
    get runtimeId(): string { return this.#runtimeId; }
    #factory: CemMlWasmGeneration;
    #admission: CommandNativeAdmission;
    #checkpoint: CommandNativeClientCheckpoint;
    #participant: CemNativeGenerationParticipant | undefined;
    #closed = false;
    #pending = 0;
    #revision = 0;
    #gate: () => boolean;
    #runtime: InvocationRuntime;

    private constructor(factory: CemMlWasmGeneration, gate: () => boolean) {
        this.#factory = factory; this.#gate = gate;
        this.#admission = new CommandNativeAdmission(() => factory.call('runtimeMemoryStatus', []) as string);
        this.#checkpoint = new CommandNativeClientCheckpoint(() => this.#pending === 0, factory.memory());
        this.#runtime = Object.freeze({
            authorTransformConfigV1: (json: string) => this.#call('authorTransformConfigV1', [json]),
            buildCommandInvocationV1: (parsed: string, environment: string, resources: string) => this.#call('buildCommandInvocationV1', [parsed, environment, resources]),
            projectCommandPresentationV1: (plan: string, result: string) => this.#call('projectCommandPresentationV1', [plan, result]),
        });
    }
    static create(): Promise<OwnedBrowserCommandInvocation> { return OwnedBrowserCommandInvocation.#create(() => true); }
    static async #create(gate: () => boolean, signal?: AbortSignal): Promise<OwnedBrowserCommandInvocation> {
        if (signal?.aborted) throw Error('cem.runtime.allocation.cancelled');
        const factory = createCemMlWasmGeneration();
        await factory.initialize();
        try {
            if (signal?.aborted) throw Error('cem.runtime.allocation.cancelled');
            if (!(factory.memory() instanceof WebAssembly.Memory)) throw Error('cem.runtime.allocation.identity');
            const text = factory.call('runtimeMemoryStatus', []);
            if (typeof text !== 'string' || text.length > 2048) throw Error('cem.runtime.allocation.headroom');
            const status = JSON.parse(text) as { softPressure?: unknown; hardPressure?: unknown };
            if (status.softPressure !== false || status.hardPressure !== false) throw Error('cem.runtime.allocation.headroom');
            return new OwnedBrowserCommandInvocation(factory, gate);
        } catch (error) { factory.dispose(); throw error; }
    }
    #open(): void {
        if (this.#closed || !this.#gate()) throw Error('cem.runtime.generation.retired');
        this.#checkpoint.assertExecution();
    }
    #changed(): void { this.#revision++; this.#checkpoint.changed(); }
    #call(name: string, args: unknown[]): string {
        this.#open();
        const release = this.#admission.admit(); this.#changed();
        try { return this.#factory.call(name, args) as string; }
        catch (error) { this.#admission.invalidate(); throw error; }
        finally { release(); this.#changed(); }
    }
    authorTransformConfigV1 = (json: string): string => this.#runtime.authorTransformConfigV1(json);
    projectPresentation = (plan: CommandPresentationPlanV1, result: CommandServiceResultV1): CommandPresentationV1 =>
        projectCommandPresentationWith(this.#runtime, plan, result);
    buildInvocation = async (parsed: ParsedCemMlCommand, resolver: BrowserCommandInvocationResolver,
        options: BrowserCommandInvocationOptions = {}): Promise<CommandInvocationV1> => {
        this.#open(); this.#pending++; this.#changed();
        try { return await buildCommandInvocationWith(this.#runtime, this.#admission, parsed, resolver, options); }
        finally { this.#pending--; this.#changed(); }
    };
    claimNativeEmbedding(): CommandNativeEmbeddingOwnership { this.#open(); return this.#checkpoint.claimEmbedding(); }
    releaseNativeEmbedding(lease: CommandNativeEmbeddingOwnership): void { this.#checkpoint.releaseEmbedding(lease); }
    generationParticipant(): CemNativeGenerationParticipant {
        if (this.#closed || !this.#gate()) throw Error('cem.runtime.generation.retired');
        return this.#participant ??= registerNativeGeneration(this.#factory.memory() as WebAssembly.Memory, this.runtimeId, async (admitted, signal) => {
            const fresh = await OwnedBrowserCommandInvocation.#create(admitted, signal);
            return { identity: fresh.#factory.memory() as WebAssembly.Memory, runtimeId: fresh.runtimeId, runtime: fresh,
                current: () => !fresh.#closed, discard: () => fresh.close() };
        });
    }
    recoveryWitness(): () => boolean {
        if (this.#closed || !this.#gate() || this.#pending) throw Error('cem.command.recovery.busy');
        const revision = this.#revision;
        // Retirement closes this generation; only new work invalidates the witness.
        return () => revision === this.#revision && this.#pending === 0;
    }
    closeNativeAdmission(ownership: readonly CommandNativeEmbeddingOwnership[], timeoutMs = 5000): Promise<CommandNativeDrain> {
        if (this.#closed || !this.#gate()) return Promise.reject(Error('cem.runtime.generation.retired'));
        return this.#checkpoint.capture(async () => {
            this.#admission.close(); this.#factory.call('closeCommandNativeAdmission', []);
            const drain = this.#admission.checkpoint(await this.#admission.drain(timeoutMs));
            const inventory = parseCommandNativeInventory(this.#factory.call('commandNativeResourceInventory', []));
            return { ...drain, nativeInventory: inventory,
                nativeClosure: JSON.parse(this.#factory.call('validateCommandNativeClosure', [inventory.revision]) as string) as unknown };
        }, timeoutMs, ownership, 'close');
    }
    isNativeCheckpointCurrent(proof: CommandNativeDrain): boolean { return !this.#closed && this.#checkpoint.current(proof); }
    async retireNativeGeneration(proof: CommandNativeDrain, ownership: readonly CommandNativeEmbeddingOwnership[],
        handoff: CommandNativeRetirementHandoff): Promise<CemNativeGenerationRetirement> {
        this.#checkpoint.requireCoordinated(proof);
        const participant = this.generationParticipant();
        // The factory owns the original native RAII permit, freezes mutation and
        // checks these original host witnesses before dropping all WASM bindings.
        const result = await this.#checkpoint.retire(proof, ownership, handoff,
            async () => this.#factory.prepareRetirement(proof.nativeClosure?.inventoryRevision ?? -1),
            async () => { this.#factory.abortRetirement(); }, async () => {
                this.#factory.dispose(() => handoff.current() === true && this.#checkpoint.current(proof));
                this.#closed = true;
            });
        return completeNativeRetirement(participant, result.checkpoint);
    }
    close(): void {
        if (this.#closed) return;
        this.#closed = true; this.#checkpoint.invalidate(); this.#admission.close();
        this.#factory.dispose();
    }
}

/** Host binding is the only retargetable surface. Native aliases stay fenced.
 * Complete mixed groups use original prepareResume tickets. */
export class BrowserCommandInvocationBinding {
    #runtime: OwnedBrowserCommandInvocation;
    #closed = false;
    #enrollment: CemNativeRecoveryEnrollment | undefined;
    #scope: Pick<CemNativeRecoveryScope, 'enroll'> | undefined;
    #id: string | undefined;
    #hostWitness: () => (() => boolean);
    #revoke: (() => void) | undefined;
    private constructor(runtime: OwnedBrowserCommandInvocation, hostWitness: () => (() => boolean)) {
        this.#runtime = runtime; this.#hostWitness = hostWitness;
    }
    static async create(options: { recoveryScope?: Pick<CemNativeRecoveryScope, 'enroll'>; id?: string; hostWitness?: () => (() => boolean) } = {}): Promise<BrowserCommandInvocationBinding> {
        const { recoveryScope, id = `invocation:${crypto.randomUUID()}`, hostWitness = () => () => true } = options;
        const pending = recoveryScope?.enroll({ id, kind: 'command-invocation', prepare: async () => { throw Error('cem.command.recovery.busy'); } });
        let runtime: OwnedBrowserCommandInvocation | undefined;
        try {
            runtime = await OwnedBrowserCommandInvocation.create();
            const binding = new BrowserCommandInvocationBinding(runtime, hostWitness);
            binding.#scope = recoveryScope; binding.#id = id;
            pending?.remove(); binding.#enroll(); return binding;
        } catch (error) { pending?.remove(); runtime?.close(); throw error; }
    }
    #enroll(): void {
        if (!this.#scope || !this.#id) return;
        this.#enrollment = this.#scope.enroll({ id: this.#id, kind: 'command-invocation', prepare: async () => {
            const runtime = this.runtime, currentNative = runtime.recoveryWitness(), currentHost = this.#hostWitness();
            if (typeof currentHost !== 'function' || currentHost() !== true) throw Error('cem.command.recovery.host');
            return { participants: [runtime.generationParticipant()], current: () => currentNative() && currentHost() === true && currentNative() };
        } });
    }
    get runtime(): OwnedBrowserCommandInvocation {
        if (this.#closed) throw Error('cem.runtime.generation.retired');
        return this.#runtime;
    }
    authorTransformConfigV1 = (json: string): string => this.runtime.authorTransformConfigV1(json);
    buildInvocation = (parsed: ParsedCemMlCommand, resolver: BrowserCommandInvocationResolver, options?: BrowserCommandInvocationOptions): Promise<CommandInvocationV1> =>
        this.runtime.buildInvocation(parsed, resolver, options);
    projectPresentation = (plan: CommandPresentationPlanV1, result: CommandServiceResultV1): CommandPresentationV1 => this.runtime.projectPresentation(plan, result);
    /** Capture before retirement; a complete shared owner transaction adopts
     * this host route together with rendering/preview/command peers. */
    prepareResume(): CemNativeGenerationBindingTicket {
        const previous = this.runtime, nativeCurrent = previous.recoveryWitness(), hostCurrent = this.#hostWitness();
        const current = () => !this.#closed && this.#runtime === previous && nativeCurrent() && hostCurrent() === true;
        return issueNativeGenerationBindingTicket(previous.generationParticipant(), current, async context => {
            const fresh = context.generation.runtime;
            if (!(fresh instanceof OwnedBrowserCommandInvocation) || !current()) throw Error('cem.command.recovery.binding');
            const freshCurrent = fresh.recoveryWitness(), stagedCurrent = () => current() && freshCurrent();
            const previousRevoke = this.#revoke;
            return { owner: this, current: stagedCurrent, bind: () => {
                if (!context.active() || context.signal.aborted || !stagedCurrent()) throw Error('cem.command.recovery.binding');
                this.#enrollment?.remove(); this.#runtime = fresh; this.#revoke = context.revoke; this.#enroll();
            }, bound: () => { previousRevoke?.(); }, discard: () => {
                if (this.#runtime === fresh) { this.#enrollment?.remove(); this.#closed = true; }
            } };
        });
    }
    /** One complete pure-authoring admission; never silently consume a mixed group. */
    resume(admission: CemNativeGenerationAdmission): void {
        const authority = claimNativeGenerationAdmission(admission);
        try {
            if (this.#closed || authority.generations.length !== 1) throw Error('cem.command.recovery.binding');
            const generation = authority.generations[0];
            if (generation.previousRuntimeId !== this.#runtime.runtimeId || !(generation.runtime instanceof OwnedBrowserCommandInvocation)
                || !authority.current()) throw Error('cem.command.recovery.binding');
            const previousRevoke = this.#revoke;
            this.#enrollment?.remove(); this.#runtime = generation.runtime; this.#revoke = authority.revoke;
            previousRevoke?.();
            authority.onRevoke(() => { if (this.#runtime === generation.runtime) { this.#enrollment?.remove(); this.#closed = true; } });
            this.#enroll();
        } catch (error) { authority.revoke(); throw error; }
    }
    close(): void {
        if (this.#closed) return;
        this.#closed = true; this.#enrollment?.remove(); this.#revoke?.(); this.#runtime.close();
    }
}
export const createBrowserCommandInvocationBinding = BrowserCommandInvocationBinding.create;
