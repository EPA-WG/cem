import { randomUUID } from 'node:crypto';

import * as runtime from '@epa-wg/cem-ml/wasm';
import type {
    CommandInvocationBuildResponseV1,
    CommandInvocationEnvironmentV1,
    CommandInvocationResourceRequirementV1,
    CommandInvocationV1,
    CommandPresentationPlanV1,
    CommandPresentationV1,
    CommandServiceResultV1,
    CommandUriMapV1,
    VirtualResourceV1,
} from '@epa-wg/cem-ml/wasm';

import type { ParsedCemMlCommand } from './command.js';
import { NodeCommandHost } from './node-host.js';

const MAX_DISCOVERY_PASSES = 8;

export interface NodeCommandInvocationOptions {
    readonly requestId?: string;
    readonly projectId?: string;
    readonly projectRevision?: number;
    readonly resourceRevision?: number;
    readonly cwd?: string;
    readonly resolverPolicyStamp?: string;
    readonly safetyPolicyStamp?: string;
    readonly budgetPolicyStamp?: string;
    readonly stdoutIsTerminal?: boolean;
}

export class NodeCommandInvocationError extends Error {
    readonly code: string;
    readonly exitCode: number;

    constructor(code: string, message: string, exitCode: number) {
        super(message);
        this.name = 'NodeCommandInvocationError';
        this.code = code;
        this.exitCode = exitCode;
    }
}

export async function buildNodeCommandInvocation(
    parsed: ParsedCemMlCommand,
    host: NodeCommandHost,
    options: NodeCommandInvocationOptions = {},
): Promise<CommandInvocationV1> {
    const environment: CommandInvocationEnvironmentV1 = {
        requestId: options.requestId ?? randomUUID(),
        projectId: options.projectId ?? 'cem-ml-node-cli',
        projectRevision: options.projectRevision ?? 0,
        resourceRevision: options.resourceRevision ?? 0,
        cwd: options.cwd ?? host.cwd,
        resolverPolicyStamp: options.resolverPolicyStamp ?? 'node-file-https-stream-v1',
        safetyPolicyStamp: options.safetyPolicyStamp ?? 'portable-v1',
        budgetPolicyStamp: options.budgetPolicyStamp ?? 'common-default-v1',
        stdoutIsTerminal: options.stdoutIsTerminal ?? false,
    };
    const resources: CommandUriMapV1<VirtualResourceV1> = {};

    for (let pass = 0; pass < MAX_DISCOVERY_PASSES; pass += 1) {
        const response = parseBuildResponse(
            runtime.buildCommandInvocationV1(
                JSON.stringify(parsed),
                JSON.stringify(environment),
                JSON.stringify(resources),
            ),
        );
        if (response.state === 'ready') return response.invocation;
        if (response.state === 'error') {
            throw new NodeCommandInvocationError(
                response.error.code,
                response.error.message,
                response.error.exitCode,
            );
        }
        await resolveRequirements(host, resources, response.requirements);
    }
    throw new NodeCommandInvocationError(
        'cem.node_command.discovery_limit',
        `command resource discovery exceeded ${MAX_DISCOVERY_PASSES} passes`,
        7,
    );
}

export function projectNodeCommandPresentation(
    plan: CommandPresentationPlanV1,
    result: CommandServiceResultV1,
): CommandPresentationV1 {
    const value = JSON.parse(
        runtime.projectCommandPresentationV1(JSON.stringify(plan), JSON.stringify(result)),
    ) as unknown;
    if (isRecord(value) && isRecord(value.error)) {
        throw new NodeCommandInvocationError(
            typeof value.error.code === 'string' ? value.error.code : 'cem.command.presentation',
            typeof value.error.message === 'string' ? value.error.message : 'command presentation failed',
            7,
        );
    }
    return value as CommandPresentationV1;
}

async function resolveRequirements(
    host: NodeCommandHost,
    resources: CommandUriMapV1<VirtualResourceV1>,
    requirements: readonly CommandInvocationResourceRequirementV1[],
): Promise<void> {
    for (const requirement of requirements) {
        const uris = requirement.kind === 'glob' ? await host.expandGlob(requirement.uri) : [requirement.uri];
        for (const uri of uris) {
            if (resources[uri] !== undefined) continue;
            let bytes: Uint8Array;
            try {
                bytes = await host.read(uri);
            } catch (error) {
                throw new NodeCommandInvocationError(
                    errorCode(error, 'cem.node_host.read'),
                    error instanceof Error ? error.message : String(error),
                    6,
                );
            }
            resources[uri] = {
                bytes: [...bytes],
                identity: requirement.identity,
            };
        }
    }
}

function parseBuildResponse(json: string): CommandInvocationBuildResponseV1 {
    try {
        return JSON.parse(json) as CommandInvocationBuildResponseV1;
    } catch (error) {
        throw new NodeCommandInvocationError(
            'cem.node_command.invocation_response',
            `Rust command invocation response is invalid: ${error instanceof Error ? error.message : String(error)}`,
            7,
        );
    }
}

function errorCode(error: unknown, fallback: string): string {
    if (isRecord(error) && typeof error.code === 'string') return error.code;
    return fallback;
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}
