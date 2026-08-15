import type {
    CommandInvocationV1,
    CommandPresentationV1,
    CommandServiceRequestV1,
    CommandServiceResultV1,
} from '@epa-wg/cem-ml/wasm';

import type { ParsedCemMlCommand } from './command.js';
import {
    createNodeCommandServiceClient,
    NodeCommandServiceClient,
    type NodeCommandServiceExecuteOptions,
    type NodeCommandServiceHandle,
} from './node-command.js';
import { createNodeCommandHost, NodeCommandHost, type NodeCommandHostOptions } from './node-host.js';
import {
    buildNodeCommandInvocation,
    projectNodeCommandPresentation,
    type NodeCommandInvocationOptions,
} from './node-invocation.js';

const STDOUT_URI = 'cem-stdio://stdout';

export interface NodeCommandServiceOptions extends NodeCommandHostOptions {
    readonly host?: NodeCommandHost;
    readonly startupTimeoutMs?: number;
    readonly onWorkerFailure?: (error: Error) => void;
}

export interface NodeParsedCommandRun {
    readonly invocation: CommandInvocationV1;
    readonly handle: NodeCommandServiceHandle;
}

export class NodeCommandService {
    readonly host: NodeCommandHost;
    readonly client: NodeCommandServiceClient;

    private constructor(host: NodeCommandHost, client: NodeCommandServiceClient) {
        this.host = host;
        this.client = client;
    }

    static async create(options: NodeCommandServiceOptions = {}): Promise<NodeCommandService> {
        const host = options.host ?? createNodeCommandHost(options);
        const client = await createNodeCommandServiceClient({
            host,
            ...(options.startupTimeoutMs === undefined ? {} : { startupTimeoutMs: options.startupTimeoutMs }),
            ...(options.onWorkerFailure === undefined ? {} : { onWorkerFailure: options.onWorkerFailure }),
        });
        return new NodeCommandService(host, client);
    }

    execute(
        request: CommandServiceRequestV1,
        options: NodeCommandServiceExecuteOptions = {},
    ): NodeCommandServiceHandle {
        this.host.trackRequest(request);
        const handle = this.client.execute(request, options);
        void handle.result().then(
            () => void this.host.releaseRequest(request.requestId).catch(() => undefined),
            () => void this.host.releaseRequest(request.requestId).catch(() => undefined),
        );
        return handle;
    }

    async run(
        parsed: ParsedCemMlCommand,
        invocationOptions: NodeCommandInvocationOptions = {},
        executeOptions: NodeCommandServiceExecuteOptions = {},
    ): Promise<NodeParsedCommandRun> {
        const invocation = await buildNodeCommandInvocation(parsed, this.host, invocationOptions);
        return { invocation, handle: this.execute(invocation.request, executeOptions) };
    }

    async publish(
        invocation: CommandInvocationV1,
        result: CommandServiceResultV1,
    ): Promise<CommandPresentationV1> {
        const presentation = projectNodeCommandPresentation(invocation.presentation, result);
        const primary = this.host.takeCommittedStream(STDOUT_URI);
        const combined: CommandPresentationV1 = {
            writes: [
                ...presentation.writes,
                ...(primary.byteLength === 0
                    ? []
                    : [{ target: 'stdout' as const, bytes: [...primary] }]),
            ],
        };
        await this.host.publishPresentation(combined.writes);
        return combined;
    }

    close(): Promise<void> {
        return this.client.close();
    }
}

export function createNodeCommandService(
    options: NodeCommandServiceOptions = {},
): Promise<NodeCommandService> {
    return NodeCommandService.create(options);
}
