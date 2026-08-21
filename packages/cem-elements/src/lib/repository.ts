export const CEM_REPOSITORY_PROTOCOL_VERSION = 1 as const;

export type CemRepositoryLifecycleState =
    | 'idle'
    | 'opening'
    | 'migrating'
    | 'ready'
    | 'blocked'
    | 'unavailable'
    | 'failed'
    | 'closed';

export interface CemRepositoryDiagnostic {
    code: string;
    severity: 'info' | 'warning' | 'error' | 'fatal';
    message: string;
}

export interface CemRepositoryRequest {
    protocolVersion: typeof CEM_REPOSITORY_PROTOCOL_VERSION;
    repository: string;
    operation: string;
    requestRevision: number;
    parameters?: unknown;
}

export interface CemRepositoryQueryResult {
    protocolVersion: typeof CEM_REPOSITORY_PROTOCOL_VERSION;
    repository: string;
    operation: string;
    requestRevision: number;
    repositoryRevision: number;
    value: unknown;
    diagnostics: CemRepositoryDiagnostic[];
}

export interface CemRepositoryCommandResult extends CemRepositoryQueryResult {
    changeCursor?: number;
}

export interface CemRepositoryStatus {
    protocolVersion: typeof CEM_REPOSITORY_PROTOCOL_VERSION;
    repository: string;
    state: CemRepositoryLifecycleState;
    repositoryRevision: number;
    schemaVersion?: number;
    usage?: number;
    quota?: number;
    persisted?: boolean;
    diagnostics: CemRepositoryDiagnostic[];
}

export interface CemRepositoryChange {
    protocolVersion: typeof CEM_REPOSITORY_PROTOCOL_VERSION;
    repository: string;
    cursor: number;
    repositoryRevision: number;
}

export interface CemRepositoryPort {
    query(request: CemRepositoryRequest, signal?: AbortSignal): Promise<CemRepositoryQueryResult>;
    execute(request: CemRepositoryRequest, signal?: AbortSignal): Promise<CemRepositoryCommandResult>;
    subscribe(cursor: number, notify: (change: CemRepositoryChange) => void): () => void;
    status(): Promise<CemRepositoryStatus>;
}

/** Read-only repository surface permitted inside declarative render lifecycles. */
export interface CemRepositoryReader {
    query(request: CemRepositoryRequest, signal?: AbortSignal): Promise<CemRepositoryQueryResult>;
    subscribe(identity: string, cursor: number, notify: (change: CemRepositoryChange) => void): () => void;
    status(identity: string): Promise<CemRepositoryStatus>;
}

export class CemRepositoryContractError extends Error {
    constructor(
        readonly code:
            | 'cem.repository.invalid_identity'
            | 'cem.repository.invalid_operation'
            | 'cem.repository.invalid_request_revision'
            | 'cem.repository.protocol_unsupported'
            | 'cem.repository.already_registered'
            | 'cem.repository.not_registered'
            | 'cem.repository.response_mismatch'
            | 'cem.repository.not_clone_safe',
        message: string,
    ) {
        super(message);
        this.name = 'CemRepositoryContractError';
    }
}

/**
 * Scope-local registry for clone-safe repositories. Templates and components
 * address only logical repository and operation identities; physical browser
 * databases, stores, indexes, and transaction handles remain host-private.
 */
export class CemRepositoryRegistry implements CemRepositoryReader {
    private readonly ports = new Map<string, CemRepositoryPort>();

    register(identity: string, port: CemRepositoryPort): () => void {
        assertRepositoryIdentity(identity);
        if (this.ports.has(identity)) {
            throw new CemRepositoryContractError(
                'cem.repository.already_registered',
                `repository \`${identity}\` is already registered`,
            );
        }
        this.ports.set(identity, port);
        return () => {
            if (this.ports.get(identity) === port) this.ports.delete(identity);
        };
    }

    has(identity: string): boolean {
        return this.ports.has(identity);
    }

    /**
     * Return a capability-narrowed facade for CEM runtimes. The facade has no
     * mutation method, so render-driven code cannot acquire `execute`.
     */
    readOnly(): CemRepositoryReader {
        return Object.freeze({
            query: this.query.bind(this),
            subscribe: this.subscribe.bind(this),
            status: this.status.bind(this),
        });
    }

    async query(request: CemRepositoryRequest, signal?: AbortSignal): Promise<CemRepositoryQueryResult> {
        const normalized = cloneAndValidateRequest(request);
        const result = await this.port(normalized.repository).query(normalized, signal);
        return cloneAndValidateResult(normalized, result);
    }

    async execute(request: CemRepositoryRequest, signal?: AbortSignal): Promise<CemRepositoryCommandResult> {
        const normalized = cloneAndValidateRequest(request);
        const result = await this.port(normalized.repository).execute(normalized, signal);
        return cloneAndValidateResult(normalized, result);
    }

    subscribe(identity: string, cursor: number, notify: (change: CemRepositoryChange) => void): () => void {
        assertRepositoryIdentity(identity);
        assertNonNegativeInteger(cursor, 'subscription cursor', 'cem.repository.invalid_request_revision');
        return this.port(identity).subscribe(cursor, (change) => {
            const cloned = cloneRepositoryValue(change);
            if (cloned.repository !== identity) {
                throw new CemRepositoryContractError(
                    'cem.repository.response_mismatch',
                    `repository subscription returned \`${cloned.repository}\` for \`${identity}\``,
                );
            }
            notify(cloned);
        });
    }

    async status(identity: string): Promise<CemRepositoryStatus> {
        assertRepositoryIdentity(identity);
        const status = cloneRepositoryValue(await this.port(identity).status());
        if (status.repository !== identity) {
            throw new CemRepositoryContractError(
                'cem.repository.response_mismatch',
                `repository status returned \`${status.repository}\` for \`${identity}\``,
            );
        }
        return status;
    }

    private port(identity: string): CemRepositoryPort {
        const port = this.ports.get(identity);
        if (!port) {
            throw new CemRepositoryContractError(
                'cem.repository.not_registered',
                `repository \`${identity}\` is not registered`,
            );
        }
        return port;
    }
}

function cloneAndValidateRequest(request: CemRepositoryRequest): CemRepositoryRequest {
    const cloned = cloneRepositoryValue(request);
    if (cloned.protocolVersion !== CEM_REPOSITORY_PROTOCOL_VERSION) {
        throw new CemRepositoryContractError(
            'cem.repository.protocol_unsupported',
            `repository protocol ${String(cloned.protocolVersion)} is unsupported`,
        );
    }
    assertRepositoryIdentity(cloned.repository);
    assertRepositoryOperation(cloned.operation);
    assertNonNegativeInteger(cloned.requestRevision, 'request revision', 'cem.repository.invalid_request_revision');
    return cloned;
}

function cloneAndValidateResult<T extends CemRepositoryQueryResult>(request: CemRepositoryRequest, result: T): T {
    const cloned = cloneRepositoryValue(result);
    if (
        cloned.protocolVersion !== CEM_REPOSITORY_PROTOCOL_VERSION ||
        cloned.repository !== request.repository ||
        cloned.operation !== request.operation ||
        cloned.requestRevision !== request.requestRevision
    ) {
        throw new CemRepositoryContractError(
            'cem.repository.response_mismatch',
            `repository \`${request.repository}\` returned a response for another request`,
        );
    }
    assertNonNegativeInteger(
        cloned.repositoryRevision,
        'repository revision',
        'cem.repository.invalid_request_revision',
    );
    return cloned;
}

function assertRepositoryIdentity(identity: string): void {
    if (!/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/.test(identity)) {
        throw new CemRepositoryContractError(
            'cem.repository.invalid_identity',
            `repository identity \`${identity}\` must be lower-kebab-case`,
        );
    }
}

function assertRepositoryOperation(operation: string): void {
    if (!/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/.test(operation)) {
        throw new CemRepositoryContractError(
            'cem.repository.invalid_operation',
            `repository operation \`${operation}\` must be lower-kebab-case`,
        );
    }
}

function assertNonNegativeInteger(value: number, label: string, code: 'cem.repository.invalid_request_revision'): void {
    if (!Number.isSafeInteger(value) || value < 0) {
        throw new CemRepositoryContractError(code, `${label} must be a non-negative safe integer`);
    }
}

function cloneRepositoryValue<T>(value: T): T {
    try {
        return structuredClone(value);
    } catch (error) {
        throw new CemRepositoryContractError(
            'cem.repository.not_clone_safe',
            `repository envelopes must be structured-clone safe: ${error instanceof Error ? error.message : String(error)}`,
        );
    }
}
