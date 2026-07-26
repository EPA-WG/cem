# CEM-ML Resource And AST Stream Lifecycle

Status: Draft · Type: Runtime Contract

This document defines the CEM-ML stack contract for resource and asset lifecycle state, AST streams, revisions, and
hydration. Runtime-specific loaders such as `<cem-element src>`, `<http-request url>`, module maps, asset references,
style resources, localized content, and future package/resource declarations MUST map their host-specific behavior onto
this model before templates observe the resource.

The lifecycle belongs to the CEM-ML stack because lifecycle state is template-visible data. A template may render from a
declared, scheduled, waiting, in-progress, streaming, loaded, or failed resource before any content AST stream has emitted
events.

## Scope

This contract covers:

- portable resource lifecycle state names;
- resource revisions and monotonic state transitions;
- AST event stream identity and ordering;
- stream-derived query projections;
- diagnostics and failure states;
- hydration and de-hydration capabilities;
- renderability from lifecycle state across browser, CLI, WASM, SSR, worker, and edge runtimes.

This contract does not define a transport. Individual resource primitives still own transport-specific policy, such as
HTTP methods and headers, browser fetch behavior, package resolution, file-system access, cache storage, or asset
decoding.

When a resource stream or stream-derived projection crosses a CEM-owned boundary, the
preferred representation is the in-process typed structure or canonical CEM binary/chunk
format. Hosts SHOULD avoid JSON serialization when they can consume the CEM binary or
query API directly. JSON projections are debug/interchange views, not the primary
resource lifecycle transport.

## Primary Principle: Generic Source Import

Every supported input content type MUST enter the CEM engine through the same generic
source import model:

```text
source bytes + source identity + content identity
  -> decoded source stream with byte ranges
  -> registered parser/adapter events
  -> CEM-owned internal DOM/AST stream
  -> schema validation, formatting, colorizing, transform, projection, and export
```

The internal DOM/AST stream is the stable engine spine. It is not a browser DOM,
`serde_json::Value`, raw text, host object graph, response object, or command-local
projection. Format-specific parsers and adapters MAY use native parser structures
internally, but those structures MUST be lowered to CEM-owned DOM/AST events before
engine behavior observes the content.

The generic import contract applies to CEM-ML, HTML, XML, SVG, MathML, CSS, JSON,
YAML, CSV, CEM-QL, native-template, XSLT compatibility sources, projection artifacts,
and future registered content types. A type-specific converter may provide a fast path
only when it produces the same observable DOM/AST, diagnostics, source-map, and
artifact metadata as the generic import path.

For every accepted source:

- the source identity MUST include the requested URI, resolved URI when different,
  content type, schema or namespace identity, and active resolver/policy stamp when
  available;
- decoded bytes MUST preserve absolute byte offsets into the original source, including
  BOM handling and line-ending normalization decisions;
- each emitted element, attribute, scalar value, token, parser fact, and diagnostic MUST
  carry a source-map stack or an explicit reduced-fidelity source-map contract;
- generated nodes from compatibility lowering or parser recovery MUST carry a generated
  source-map frame that points back to the originating source range and adapter id;
- parser facts and source-map spans MUST be available to schema-owned validators,
  formatters, colorizers, transforms, projections, reports, and CLI previews through the
  same DOM/AST boundary;
- unsupported or ambiguous input identity MUST fail closed or emit deterministic
  lifecycle diagnostics before the source is parsed as a different syntax.

Direct command-specific parsing, validation, conversion, or preview rendering that
bypasses this import model is a deviation, not an alternate architecture. Such
deviations MUST be tracked as remediation work and removed or narrowed until the
observable behavior matches the generic source import contract.

## Resource State

Every external resource state exposed to templates MUST carry:

- resource kind, such as `template-src`, `http-request`, `asset`, or future resource kind;
- stable resource identity for the current revision;
- lifecycle state;
- source identity as far as it has been resolved;
- request or lookup metadata where applicable;
- response or result metadata when available;
- stream handle or stream-derived projection when content events are available;
- diagnostics accumulated for the current revision.

The resource state is a CEM data source. Templates MAY render from any lifecycle state, including states before transport
starts or before an AST stream has emitted content events.

## Resource Role And Content-Type Context

A host primitive declares the resource role before the CEM-ML engine parses resource content:

- template-source resources are expected to produce declaration-template artifacts;
- response/data resources are expected to produce CEM AST streams or stream-derived projections;
- future asset resources are expected to produce resource-specific AST streams, metadata streams, or derived artifacts.

For example, `<cem-element src>` declares a template-source role and `<http-request url>` declares a response/data role.
Those host bindings pass the expected content-type context into CEM-ML. After that handoff, CEM-ML owns the normalized
content-type negotiation, parser/plugin dispatch, lifecycle transitions, AST stream production, projections, diagnostics,
and hydration behavior.

The host primitive MUST pass the CEM-ML engine:

- resource kind and role;
- active context identity;
- active expected content-type set;
- provided content-type metadata when known;
- source identity and source-range capability;
- host security policy identity;
- registered parser/plugin registry identity;
- lifecycle state and revision identity.

The resource role decides how the AST stream is consumed. It does not bypass the CEM-ML content-type registry, lifecycle,
diagnostics, hydration, or stream projection rules.

## Expected And Provided Content Types

External resource loading has two content-type concepts:

- **Expected content type set** — what the active CEM-ML engine context is willing to ingest for this resource role.
- **Provided content type** — what the resource payload or selected source actually supplies.

The expected content type is not a single hardcoded default. By default it is the active accepted content-type set
provided to the CEM-ML engine at the resource load boundary. That set is passed through, and may be modified by, the
context tree.

The active accepted set is derived from:

- engine built-ins;
- registered content-type plugins;
- build, CLI, browser-WASM, SSR, or edge host configuration;
- module-map or host-loader metadata;
- active context-tree policy inherited from the declaring document or parent region;
- any local declaration or host override that explicitly narrows the accepted set.

Context-tree policy MAY extend, narrow, or remap the accepted set for descendants, but it MUST do so explicitly and
deterministically. Resource payload or metadata MUST NOT expand its own accepted content-type set simply by naming a type
or embedding metadata; expansion requires an already-registered plugin and active context policy.

In CLI and browser-WASM hosts, the accepted content-type registry is supplied by build/config state and registered
plugins. The CEM-ML engine MUST negotiate against that registry rather than inferring support only from file extension or
DOM shape.

Provided type is derived in this order:

1. Host loader metadata, such as HTTP `Content-Type` response header or equivalent file/package metadata.
2. Explicit target marker such as `<template type="text/cem-ml">` or `<template lang="custom-element-xslt">`.
3. Module-map or resource-map entry metadata, when a host resolver supplied it.
4. File extension fallback, only when the active registry and host policy permit extension fallback.
5. Structural sniffing, only through registered sniffers for the active context.

Mismatch handling MUST be explicit:

- If the provided type is accepted by the active registry and context policy, parsing continues.
- If the active registry has a parser, converter, or declared pass-through adapter for the provided type, parsing
  continues and diagnostics MAY record the conversion or downgrade.
- If the provided type is unsupported, unsafe, or incompatible with the active expected set, the resource revision MUST
  fail closed with diagnostics.
- Silent fallback to a different content type is not allowed.

## Context Tree Propagation

A resource load occurs within a context tree. The active context determines the accepted content types, plugin registry,
module-map policy, schema bindings, security policy, diagnostics mode, hydration policy, and compatibility posture.

When a loaded document, AST stream, or stream-derived subtree introduces a nested context, that nested context MAY modify
the accepted content-type set for its descendants. Such modification MUST be represented at the CEM artifact boundary so
CLI, browser-WASM, SSR, and edge execution make the same decision from the same context state.

Context propagation rules:

- Parent context policy applies until a child context explicitly changes it.
- Child contexts MAY narrow accepted types for security or compatibility.
- Child contexts MAY extend accepted types only through plugins already registered by the host/build configuration.
- Context changes MUST be deterministic and observable in diagnostics or artifact metadata.
- A serialized or cached artifact MUST include enough context identity to prevent reuse under an incompatible registry or
  security policy.

## Registered Content-Type Plugins

A content-type plugin declares how a source type crosses into the CEM AST stream or artifact boundary. The same registry
is used by template resources, response/data resources, and future asset resources, although the accepted set may differ
by resource kind and context.

A plugin record SHOULD include:

- content-type identifiers and aliases;
- optional file-extension hints;
- optional structural sniffers;
- parser or converter entrypoints;
- output artifact kind;
- supported version range;
- security classification;
- diagnostics emitted on rejection, downgrade, or conversion.

Plugins are registered by CLI, build, browser-WASM, SSR, or edge host configuration. Resource payload or metadata MUST NOT
activate an unregistered plugin by naming it in metadata. A provided content type is compatible only when the active
registry has a parser, converter, or declared pass-through adapter for it under the current context policy.

## Portable Lifecycle States

The portable lifecycle states are:

- `declared` — the authored resource reference has been discovered and captured, but resolver, lookup, transport, or
  decode work has not started. Templates may render from declaration metadata and an empty content projection.
- `scheduled` — host policy accepted the resource and load work has been queued. No transport, lookup, decode, or content
  stream is required to have started.
- `waiting` — the resource is blocked by scheduling, dependency, cache, backpressure, policy, transport, or storage
  availability. This is renderable state, not an error.
- `in-progress` — resolver, lookup, transport, cache read, decode, or parser work is active, but validated content AST
  events may still be unavailable.
- `streaming` — validated AST events are available for the current revision. The stream or stream-derived projection may
  be partial.
- `loaded` — terminal success for the current revision. No more AST events are expected, and the final artifact or
  stream-derived projection is stable for that revision.
- `failed` — terminal failure for the current revision. No more content events are expected for that revision, and
  diagnostics MUST explain the policy, resolution, transport, storage, content-type, parse, decode, or render-boundary
  failure.

Hosts MAY expose additional internal states, but template-visible state MUST map to this vocabulary.

## Revisions And Transitions

For a single resource revision, state transitions MUST be monotonic except where the scheduler moves between `scheduled`
and `waiting` before work begins:

```text
declared -> scheduled | failed
scheduled -> waiting | in-progress | failed
waiting -> scheduled | in-progress | failed
in-progress -> streaming | loaded | failed
streaming -> streaming | loaded | failed
loaded -> terminal
failed -> terminal
```

A new resource revision starts at `declared` when inputs change, an explicit refresh is requested, cache is invalidated,
or context policy changes. A runtime MUST NOT mutate a `loaded` or `failed` terminal state in place for the same revision.

Each accepted AST event MUST advance the resource revision or a stream sequence number deterministically. Runtimes MAY
coalesce events into render frames, but the observable result MUST be deterministic for the same ordered event stream and
context.

## AST Stream Lifecycle

AST stream lifecycle is nested inside resource lifecycle:

- Before `streaming`, a stream handle MAY be absent or present only as an empty not-yet-started handle.
- During `streaming`, accepted AST events update the stream handle or stream-derived projection.
- At `loaded`, the stream MUST emit an end-of-stream marker or equivalent host signal.
- At `failed`, the stream MUST emit or be associated with diagnostics sufficient for template error rendering and host
  observability.

AST streams are not host JavaScript objects, browser DOM nodes, raw strings, `Response` objects, or direct file handles.
They are CEM-owned event streams, stream handles, binary chunks, or stream-derived projections.

## Engine Boundary

An external resource MUST cross the CEM-ML runtime boundary as a CEM-owned resource state, artifact, or AST event stream
before templates observe it. This does not require the source stream to be started or fully loaded before first render; it
requires that all rendered output is driven by CEM resource states and validated CEM stream events rather than raw
external input.

There is no raw remote-DOM append path and no direct `Response`, `Headers`, `Document`, DOM node, JavaScript object, host
object graph, or raw string handoff into the template engine.

Resource roles consume the CEM boundary differently:

- template-source streams compile, lower, or incrementally project to declaration-template artifacts;
- response/data streams expose stream handles or stream-derived projections to CEM-QL;
- asset streams expose metadata, decoded resource projections, or host-approved handles according to their plugin
  contract.

## Render Semantics

Templates MAY render from lifecycle state, metadata, diagnostics, and any available stream-derived projection. Rendering
does not require the content stream to have started.

If a template reads a segment that is not available yet, the runtime MUST do one of the following deterministically:

- render from the current lifecycle state and empty projection;
- suspend the dependent region;
- schedule hydration or loading under host policy;
- attach diagnostics and render a failed diagnostic state.

Later lifecycle or stream events MUST produce deterministic revisions, suspension, or diagnostics.

## Engine Diagnostics

The CEM-ML engine owns diagnostics produced after host acquisition hands a resource to the engine boundary. Engine
diagnostics SHOULD use stable codes and MUST be attached to the resource revision and owning declaration or instance.

Engine diagnostic categories include:

- unsupported content type or parser/converter absence;
- content-type mismatch against the active expected set;
- unsafe content-type downgrade or rejected conversion;
- AST parse failure;
- declaration-template artifact compile/lower failure;
- stream projection failure;
- render-boundary failure;
- hydration or de-hydration failure;
- context-tree or plugin-policy incompatibility.

Host bindings may prefix, wrap, or transport these diagnostics, but they MUST NOT redefine the engine semantics.

## Hydration And De-Hydration

Hydration and de-hydration are planned capabilities of the AST stream lifecycle. Current runtimes MAY implement them, but
this draft does not require every host to support them yet.

Hydration is the recovery of AST stream content, template-as-data segments, asset metadata, or stream-derived projections
from a durable artifact, cache entry, serialized stream checkpoint, source range, or host loader. De-hydration is the
release or serialization of those same stream segments while preserving enough resource identity and lifecycle metadata to
recover them later on demand.

Hydration state is orthogonal to the portable resource lifecycle state. A resource may remain `streaming` or `loaded`
while some AST segments are de-hydrated, as long as template-visible reads either resolve from retained state, trigger
hydration under host policy, suspend deterministically, or produce diagnostics. De-hydration MUST NOT silently change a
resource revision, source identity, content-type decision, or security context.

Future implementations that de-hydrate AST stream segments SHOULD preserve:

- resource revision and stream sequence identity;
- source ranges or chunk identities needed for recovery;
- parser/content-type registry identity;
- context-tree and security-policy identity;
- diagnostics emitted before de-hydration;
- enough dependency metadata to know which template reads require hydration.

Hydration failure MUST move the affected resource revision to `failed` or attach a deterministic diagnostic to the
resource state when the host can continue rendering unaffected segments.

## Host-Specific Bindings

Specific resource primitives bind this generic lifecycle to their transport and artifact rules:

- `<cem-element src>` binds template-source resources to declaration-template artifacts.
- `<http-request url>` binds HTTP response resources to request/response metadata and response AST streams.
- Future asset resources bind decoded asset metadata or asset content streams to the same lifecycle model.

Those bindings MAY add required metadata and diagnostics, but they MUST NOT replace the portable lifecycle vocabulary when
state is template-visible.

## Related Documents

- [`cem-element` external resource loading contract](./cem-element-src-loading-contract.md)
- [CEM Elements HTTP request resource design](./cem-elements-http-request-design.md)
- [CEM-ML stack design](./cem-ml-stack-design.md)
- [CEM QL stack design](./cem-ql-stack-design.md)
