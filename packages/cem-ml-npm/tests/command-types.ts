import type * as Browser from '../dist/wasm/browser/cem_ml.js';
import type * as Node from '../dist/wasm/node/cem_ml.js';

type Equal<Left, Right> =
  (<Value>() => Value extends Left ? 1 : 2) extends
  (<Value>() => Value extends Right ? 1 : 2)
    ? true
    : false;
type Assert<Value extends true> = Value;

export type BrowserNodeRequestParity = Assert<
  Equal<Browser.CommandServiceRequestV1, Node.CommandServiceRequestV1>
>;
export type BrowserNodeResultParity = Assert<
  Equal<Browser.CommandServiceResultV1, Node.CommandServiceResultV1>
>;
export type BrowserNodeCapabilityParity = Assert<
  Equal<Browser.CommandServiceHostCapabilitiesV1, Node.CommandServiceHostCapabilitiesV1>
>;

export const parseOperation = {
  kind: 'parse',
  inputId: 'input:0',
  projection: 'ast',
  preserveSourceOffsets: true,
} satisfies Extract<Browser.PortableOperationRequestV1, { kind: 'parse' }>;

export const progress = {
  protocolVersion: 1,
  requestId: 'request:1',
  operationId: 1,
  sequence: 1,
  stage: 'accepted',
  completed: 0,
  total: 3,
} satisfies Browser.CommandServiceProgressV1;

export const control = {
  protocolVersion: 1,
  requestId: 'request:1',
  operationId: 1,
  selectedScope: 0,
  disposition: 'accepted',
} satisfies Browser.CommandServiceControlAckV1;

export const artifactRead = {
  protocolVersion: 1,
  requestId: 'request:1',
  handle: {
    handleId: 1,
    kind: 'output',
    contentType: 'application/cem',
    byteLength: 3,
    sha256: '0'.repeat(64),
  },
  offset: 0,
  byteLength: 3,
  eof: true,
} satisfies Browser.CommandServiceArtifactReadV1;

export const capabilities = {
  currentRevision: (request) => ({
    project: request.project,
    resourceVersions: {},
  }),
  readResource: () => ({
    version: { revision: 1, sha256: '0'.repeat(64) },
    bytes: [],
    identity: null,
  }),
  prepareWrite: () => ({ token: 'transaction:1' }),
  commitWrite: () => ({ uri: 'mem://output.cem' }),
  rollbackWrite: () => undefined,
} satisfies Browser.CommandServiceHostCapabilitiesV1;

declare const browserRuntime: typeof import('../dist/wasm/browser/cem_ml.js');
declare const nodeRuntime: typeof import('../dist/wasm/node/cem_ml.js');
declare const currentRevision: Browser.CommandRevisionLedgerJsonCallbackV1;
declare const readResource: Browser.CommandResourceReadJsonCallbackV1;
declare const prepareWrite: Browser.CommandPrepareWriteJsonCallbackV1;
declare const commitWrite: Browser.CommandCommitWriteJsonCallbackV1;
declare const rollbackWrite: Browser.CommandRollbackWriteJsonCallbackV1;
declare const onProgress: Browser.CommandProgressJsonCallbackV1;

export const browserExecution: Promise<string> = browserRuntime.executeCommandServiceV1(
  '{}',
  '{}',
  currentRevision,
  readResource,
  prepareWrite,
  commitWrite,
  rollbackWrite,
  onProgress,
);
export const nodeExecution: Promise<string> = nodeRuntime.executeCommandServiceV1(
  '{}',
  '{}',
  currentRevision,
  readResource,
  prepareWrite,
  commitWrite,
  rollbackWrite,
  onProgress,
);
export const artifactWire: Browser.CommandArtifactReadWireResponseV1 =
  browserRuntime.readCommandArtifactV1('request:1', 1, 0, 1024);
