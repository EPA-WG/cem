/**
 * Host runtime-support boundary for the `cem_ql` WASM render engine
 * (design [`cem-element-wasm-proposal.md` §5/§6](../../../../../../docs/cem-element-wasm-proposal.md)).
 *
 * This module is the Phase 3A internal form of the reusable runtime-support layer.
 * It is authored as if it will be extracted to `@epa-wg/cem-runtime-support`: it
 * knows nothing about `customElements`, declaration discovery, produced-element
 * lifecycle, or that its caller is `<cem-element>`. It only turns a canonical
 * CEM-ML source string plus serializable host/data bindings into a serializable
 * {@link RenderPlanNode} list, by calling the `cem_ql` WASM render boundary
 * (C2.2 exports) and mapping its JSON plan into the projection-layer shape so the
 * existing `materializeRenderPlan` can commit it unchanged.
 *
 * Topology: Phase 3A runs `cem_ml`/`cem_ql` WASM on the main thread. The async
 * surface here is deliberately worker-ready — a worker-backed primary path
 * (design §4.3) can replace the body without changing callers.
 */

import initCemQlWasm, {
    renderTemplateSource,
    version as cemQlVersion,
} from '../../../../../cem_ql/dist/wasm/cem_ql.js';
import type { RenderPlanNode, SourceMapRef } from '../../projection.js';

export interface RuntimeSupportDiagnostic {
    code: string;
    severity: 'info' | 'warning' | 'error' | 'fatal';
    message: string;
    byteOffset?: number;
}

export interface CemQlRenderResult {
    nodes: RenderPlanNode[];
    diagnostics: RuntimeSupportDiagnostic[];
}

export interface CemQlRenderOptions {
    /** Prefix for deterministic, pre-order render-node ids (typically the produced tag). */
    renderNodeIdPrefix?: string;
}

let initPromise: Promise<void> | undefined;
let ready = false;

/**
 * Lazily instantiate the `cem_ql` WASM module once per host. Safe to call
 * repeatedly; concurrent callers share one initialization promise.
 */
export function ensureRuntimeReady(): Promise<void> {
    if (!initPromise) {
        initPromise = Promise.resolve()
            .then(() => initCemQlWasm())
            .then(() => {
                ready = true;
            });
    }
    return initPromise;
}

/** Synchronous readiness probe so callers can choose a fallback before awaiting. */
export function isRuntimeReady(): boolean {
    return ready;
}

/** The `cem_ql` engine version; only meaningful after {@link ensureRuntimeReady}. */
export function runtimeVersion(): string {
    return cemQlVersion();
}

/**
 * Render a canonical CEM-ML template against host/data bindings through the
 * `cem_ql` WASM render boundary, returning a serializable render plan plus
 * diagnostics. Awaits WASM initialization on first use.
 */
export async function renderCemMlTemplate(
    source: string,
    data: Record<string, unknown>,
    options: CemQlRenderOptions = {}
): Promise<CemQlRenderResult> {
    await ensureRuntimeReady();
    const planJson = renderTemplateSource(source, JSON.stringify(data ?? {}));
    const plan = JSON.parse(planJson) as WasmRenderPlan;

    const prefix = options.renderNodeIdPrefix ?? 'cem-node';
    let sequence = 0;
    const nextRenderNodeId = (): string => {
        sequence += 1;
        return `${prefix}-${sequence}`;
    };

    return {
        nodes: (plan.nodes ?? []).map((node) => mapNode(node, nextRenderNodeId)),
        diagnostics: (plan.diagnostics ?? []).map(mapDiagnostic),
    };
}

interface WasmRenderPlan {
    nodes?: WasmRenderNode[];
    diagnostics?: WasmDiagnostic[];
}

type WasmRenderNode =
    | { kind: 'text'; text: string; byteOffset?: number | null }
    | { kind: 'comment'; text: string; byteOffset?: number | null }
    | {
          kind: 'element';
          tag: string;
          attributes?: WasmRenderAttribute[];
          children?: WasmRenderNode[];
          byteOffset?: number | null;
      };

interface WasmRenderAttribute {
    name: string;
    value: string;
}

interface WasmDiagnostic {
    code?: string;
    severity?: string;
    message?: string;
    byteOffset?: number | null;
}

function mapNode(node: WasmRenderNode, nextRenderNodeId: () => string): RenderPlanNode {
    if (node.kind === 'text') {
        return { kind: 'text', text: node.text, sourceMapRef: frameFrom(node.byteOffset) };
    }
    if (node.kind === 'comment') {
        return { kind: 'comment', text: node.text, sourceMapRef: frameFrom(node.byteOffset) };
    }
    // Assign the render-node id before recursing so ids follow a deterministic
    // pre-order sequence, matching the DOM/projection path.
    const renderNodeId = nextRenderNodeId();
    return {
        kind: 'element',
        namespace: null,
        tag: node.tag,
        attributes: (node.attributes ?? []).map((attribute) => ({
            name: attribute.name,
            value: attribute.value,
        })),
        renderNodeId,
        children: (node.children ?? []).map((child) => mapNode(child, nextRenderNodeId)),
        sourceMapRef: frameFrom(node.byteOffset),
    };
}

function frameFrom(byteOffset: number | null | undefined): SourceMapRef | undefined {
    if (typeof byteOffset !== 'number') {
        return undefined;
    }
    return { fidelity: 'author-byte-exact', frame: `cem:${byteOffset}` };
}

function mapDiagnostic(diagnostic: WasmDiagnostic): RuntimeSupportDiagnostic {
    return {
        code: diagnostic.code ?? 'cem.ql.wasm.diagnostic',
        severity: coerceSeverity(diagnostic.severity),
        message: diagnostic.message ?? 'cem_ql render diagnostic',
        byteOffset: typeof diagnostic.byteOffset === 'number' ? diagnostic.byteOffset : undefined,
    };
}

function coerceSeverity(severity: string | undefined): RuntimeSupportDiagnostic['severity'] {
    switch (severity) {
        case 'fatal':
        case 'error':
        case 'warning':
        case 'info':
            return severity;
        default:
            return 'error';
    }
}
