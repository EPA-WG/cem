/**
 * Host runtime-support boundary for direct CEM-QL query evaluation through the
 * `cem_ql` WASM module. This is intentionally parallel to `cem-ql-render.ts`,
 * but returns typed query items instead of light-DOM render-plan nodes.
 */

// eslint-disable-next-line @nx/enforce-module-boundaries -- generated WASM bindings are the Phase 3A internal runtime boundary.
import { evaluateQuerySource } from '../../../../../cem_ql/dist/wasm/cem_ql.js';
import { assertProcessingBoundaryValue, type SourceMapRef } from '../../projection.js';
import { ensureRuntimeReady, type RuntimeSupportDiagnostic } from './cem-ql-render.js';

export type CemQlAtomicType = 'string' | 'integer' | 'decimal' | 'double' | 'boolean' | 'any-uri' | 'null';

export type CemQlAtomicValue = string | number | boolean | null;

export interface CemQlAtomicItem {
    kind: 'atomic';
    type: CemQlAtomicType;
    value: CemQlAtomicValue;
}

export interface CemQlArrayItem {
    kind: 'array';
    items: CemQlQueryItem[];
}

export interface CemQlRecordItem {
    kind: 'record';
    fields: Record<string, CemQlQueryItem[]>;
}

export interface CemQlNodeItem {
    kind: 'node';
    id: string;
}

export interface CemQlResourceItem {
    kind: 'resource';
    id: string;
    contentType: string;
    schema: string | null;
    roles: string[];
    failAccessor: boolean;
}

export interface CemQlLambdaItem {
    kind: 'lambda';
    id: number;
}

export type CemQlQueryItem =
    | CemQlAtomicItem
    | CemQlArrayItem
    | CemQlRecordItem
    | CemQlNodeItem
    | CemQlResourceItem
    | CemQlLambdaItem;

export interface CemQlInputError {
    kind: 'input' | 'compile';
    code: string;
    message: string;
}

export interface CemQlEvalError {
    kind: 'eval';
    type: 'budget-exceeded' | 'unsupported' | 'type-error' | string;
    message: string;
    axis?: string;
}

export type CemQlQueryError = CemQlInputError | CemQlEvalError | Record<string, unknown>;

export interface CemQlQueryResult {
    items: CemQlQueryItem[];
    diagnostics: RuntimeSupportDiagnostic[];
    error: CemQlQueryError | null;
}

export type CemQlQueryBindings = Record<string, unknown>;

export interface CemQlStreamBinding {
    $stream: unknown[];
}

export interface CemQlNodeBinding {
    $node: string;
}

export interface CemQlAtomBinding {
    $atom: {
        type: CemQlAtomicType;
        value?: CemQlAtomicValue;
    };
}

export interface CemQlResourceBinding {
    $resource: {
        id: string;
        contentType: string;
        schema?: string | null;
        roles?: string[];
        failAccessor?: boolean;
    };
}

/**
 * Evaluate CEM-QL source against JSON-compatible policy bindings. Plain JSON
 * arrays remain CEM-QL array items; use `cemQlStream(...)` when a binding must
 * be a CEM-QL stream for set/pipeline rows.
 */
export async function evaluateCemQlQuery(
    source: string,
    bindings: CemQlQueryBindings = {}
): Promise<CemQlQueryResult> {
    assertProcessingBoundaryValue(bindings, 'CEM-QL query bindings');
    await ensureRuntimeReady();
    const resultJson = evaluateQuerySource(source, JSON.stringify(bindings));
    return mapCemQlQueryResult(JSON.parse(resultJson) as WasmQueryResult);
}

export function cemQlStream(items: readonly unknown[]): CemQlStreamBinding {
    return { $stream: [...items] };
}

export function cemQlNode(id: string): CemQlNodeBinding {
    return { $node: id };
}

export function cemQlAtom(type: CemQlAtomicType, value?: CemQlAtomicValue): CemQlAtomBinding {
    return value === undefined ? { $atom: { type } } : { $atom: { type, value } };
}

export function cemQlResource(resource: CemQlResourceBinding['$resource']): CemQlResourceBinding {
    return {
        $resource: {
            id: resource.id,
            contentType: resource.contentType,
            schema: resource.schema ?? null,
            roles: [...(resource.roles ?? [])],
            failAccessor: resource.failAccessor ?? false,
        },
    };
}

export function mapCemQlQueryResult(result: unknown): CemQlQueryResult {
    const wasmResult = (result ?? {}) as WasmQueryResult;
    const mapped = {
        items: (wasmResult.items ?? []).map(mapItem),
        diagnostics: (wasmResult.diagnostics ?? []).map(mapDiagnostic),
        error: wasmResult.error == null ? null : mapError(wasmResult.error),
    };
    assertProcessingBoundaryValue(mapped, 'CEM-QL query result');
    return mapped;
}

interface WasmQueryResult {
    items?: WasmQueryItem[];
    diagnostics?: WasmDiagnostic[];
    error?: unknown;
}

type WasmQueryItem =
    | {
          kind: 'atomic';
          type: CemQlAtomicType;
          value: CemQlAtomicValue;
      }
    | {
          kind: 'array';
          items?: WasmQueryItem[];
      }
    | {
          kind: 'record';
          fields?: Record<string, WasmQueryItem[]>;
      }
    | {
          kind: 'node';
          id?: string;
      }
    | {
          kind: 'resource';
          id?: string;
          contentType?: string;
          schema?: string | null;
          roles?: string[];
          failAccessor?: boolean;
      }
    | {
          kind: 'lambda';
          id?: number;
      };

interface WasmDiagnostic {
    code?: string;
    severity?: string;
    message?: string;
    byteOffset?: number | null;
}

function mapItem(item: WasmQueryItem): CemQlQueryItem {
    switch (item.kind) {
        case 'atomic':
            return {
                kind: 'atomic',
                type: item.type,
                value: item.value,
            };
        case 'array':
            return {
                kind: 'array',
                items: (item.items ?? []).map(mapItem),
            };
        case 'record':
            return {
                kind: 'record',
                fields: mapRecordFields(item.fields ?? {}),
            };
        case 'node':
            return {
                kind: 'node',
                id: item.id ?? '',
            };
        case 'resource':
            return {
                kind: 'resource',
                id: item.id ?? '',
                contentType: item.contentType ?? '',
                schema: item.schema ?? null,
                roles: [...(item.roles ?? [])],
                failAccessor: item.failAccessor ?? false,
            };
        case 'lambda':
            return {
                kind: 'lambda',
                id: item.id ?? 0,
            };
    }
}

function mapRecordFields(fields: Record<string, WasmQueryItem[]>): Record<string, CemQlQueryItem[]> {
    return Object.fromEntries(
        Object.entries(fields).map(([name, values]) => [name, values.map(mapItem)])
    ) as Record<string, CemQlQueryItem[]>;
}

function mapError(error: unknown): CemQlQueryError {
    if (error !== null && typeof error === 'object' && !Array.isArray(error)) {
        return { ...(error as Record<string, unknown>) };
    }
    return {
        kind: 'eval',
        type: 'unknown',
        message: String(error),
    };
}

function mapDiagnostic(diagnostic: WasmDiagnostic): RuntimeSupportDiagnostic {
    const byteOffset = typeof diagnostic.byteOffset === 'number' ? diagnostic.byteOffset : undefined;
    return {
        code: diagnostic.code ?? 'cem.ql.wasm.diagnostic',
        severity: coerceSeverity(diagnostic.severity),
        message: diagnostic.message ?? 'cem_ql query diagnostic',
        byteOffset,
        sourceMapRef: frameFrom(byteOffset),
    };
}

function frameFrom(byteOffset: number | null | undefined): SourceMapRef | undefined {
    if (typeof byteOffset !== 'number') {
        return undefined;
    }
    return { fidelity: 'author-byte-exact', frame: `cem:${byteOffset}` };
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
