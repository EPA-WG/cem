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

const MAX_DISCOVERY_PASSES = 8;
const initializeRuntime = (runtime as unknown as { readonly default: () => Promise<unknown> }).default;
let runtimeInitialization: Promise<unknown> | undefined;

export interface BrowserCommandInvocationOptions {
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

export interface BrowserCommandInvocationResource {
    readonly uri: string;
    readonly bytes: Uint8Array | readonly number[];
}

export type BrowserCommandInvocationResolver = (
    requirement: CommandInvocationResourceRequirementV1,
) => Promise<readonly BrowserCommandInvocationResource[]>;

export class BrowserCommandInvocationError extends Error {
    readonly code: string;
    readonly exitCode: number;

    constructor(code: string, message: string, exitCode: number) {
        super(message);
        this.name = 'BrowserCommandInvocationError';
        this.code = code;
        this.exitCode = exitCode;
    }
}

export async function buildBrowserCommandInvocation(
    parsed: ParsedCemMlCommand,
    resolver: BrowserCommandInvocationResolver,
    options: BrowserCommandInvocationOptions = {},
): Promise<CommandInvocationV1> {
    await initializeCommandRuntime();
    const environment: CommandInvocationEnvironmentV1 = {
        requestId: options.requestId ?? crypto.randomUUID(),
        projectId: options.projectId ?? 'cem-ml-browser-cli',
        projectRevision: options.projectRevision ?? 0,
        resourceRevision: options.resourceRevision ?? 0,
        cwd: options.cwd ?? '/',
        resolverPolicyStamp: options.resolverPolicyStamp ?? 'browser-explicit-v1',
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
            throw new BrowserCommandInvocationError(
                response.error.code,
                response.error.message,
                response.error.exitCode,
            );
        }
        await resolveRequirements(resolver, resources, response.requirements);
    }
    throw new BrowserCommandInvocationError(
        'cem.browser_command.discovery_limit',
        `command resource discovery exceeded ${MAX_DISCOVERY_PASSES} passes`,
        7,
    );
}

export function projectBrowserCommandPresentation(
    plan: CommandPresentationPlanV1,
    result: CommandServiceResultV1,
): CommandPresentationV1 {
    const value = JSON.parse(
        runtime.projectCommandPresentationV1(JSON.stringify(plan), JSON.stringify(result)),
    ) as unknown;
    if (isRecord(value) && isRecord(value.error)) {
        throw new BrowserCommandInvocationError(
            typeof value.error.code === 'string' ? value.error.code : 'cem.command.presentation',
            typeof value.error.message === 'string' ? value.error.message : 'command presentation failed',
            7,
        );
    }
    return value as CommandPresentationV1;
}

async function initializeCommandRuntime(): Promise<void> {
    runtimeInitialization ??= initializeRuntime();
    await runtimeInitialization;
}

async function resolveRequirements(
    resolver: BrowserCommandInvocationResolver,
    resources: CommandUriMapV1<VirtualResourceV1>,
    requirements: readonly CommandInvocationResourceRequirementV1[],
): Promise<void> {
    for (const requirement of requirements) {
        let resolved: readonly BrowserCommandInvocationResource[];
        try {
            resolved = await resolver(requirement);
        } catch (error) {
            throw new BrowserCommandInvocationError(
                errorCode(error, 'cem.browser_command.resolve'),
                error instanceof Error ? error.message : String(error),
                6,
            );
        }
        if (requirement.kind === 'read' && !resolved.some(({ uri }) => uri === requirement.uri)) {
            throw new BrowserCommandInvocationError(
                'cem.browser_command.resource_missing',
                `resolver did not return required resource ${requirement.uri}`,
                6,
            );
        }
        for (const resource of resolved) {
            if (resource.uri.length === 0) {
                throw new BrowserCommandInvocationError(
                    'cem.browser_command.resource_uri',
                    'resolver returned a resource with an empty URI',
                    6,
                );
            }
            if (resources[resource.uri] !== undefined) continue;
            resources[resource.uri] = {
                bytes: [...resource.bytes],
                identity: requirement.identity,
            };
        }
    }
}

function parseBuildResponse(json: string): CommandInvocationBuildResponseV1 {
    try {
        return JSON.parse(json) as CommandInvocationBuildResponseV1;
    } catch (error) {
        throw new BrowserCommandInvocationError(
            'cem.browser_command.invocation_response',
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
