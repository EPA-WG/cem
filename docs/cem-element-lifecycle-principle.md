# A DCE Instance Has One Durable Data Island and Derived Rendered Output

**Status: normative.** This is the basic lifecycle rule for produced custom
element instances managed by `cem-elements`. It takes precedence over lifecycle
examples and design text that reuse an authored payload envelope as the data
island or treat rendered children as payload after an island exists.

## The rule

A produced DCE instance owns one stable, direct, inert WHATWG template marked
`data-cem-island="instance"`. The template contains exactly one
`cem-island:context-root`, currently at lifecycle version `0.1.2`. Its qualified
DOM is lowered into the canonical namespace-aware CEM AST instance state.
Rendered light DOM is a derived projection outside that template; it is never
the data source once the island exists.

The three instance regions have distinct ownership:

```text
produced DCE instance
├── template[data-cem-island="instance"]  canonical inert state
│   └── cem-island:context-root            CEM AST context/error boundary
│       ├── cem-hydration:data
│       ├── cem-attributes:attributes      canonical host attributes
│       ├── cem-dataset:dataset            validated attributes projection
│       ├── cem-payload:payload            explicit content/schema handoff
│       ├── cem-slices:slices
│       ├── cem-resources:resources        portable semantic runtime state
│       ├── cem-form:form-state
│       ├── cem-validation:validation-state
│       └── cem-events:event-state
└── rendered range                         derived or provisional output
```

The island marker is a runtime and serialized-lifecycle identity. It is not an
authoring hook or an authored CSS selector. The prefixed names and `xmlns:*`
attributes are semantic namespace input in an HTML document: outside XHTML,
SVG, and MathML, the browser does not create custom DOM namespaces. The
`cem-element` HTML adapter therefore resolves lexical prefixes and namespace
declaration attributes while lowering the subtree into CEM AST. XML/XHTML input
uses its native namespace information and must produce the same expanded names.

The island schema owns topology and the context-root error boundary. Qualified
domain namespaces own each part's meaning, while payload, resource result,
slice, and event subtrees preserve the namespace and schema of the AST that
produced them. A source-language schema such as JSON is never selected merely
because state is record-shaped; foreign languages enter only through an
explicit content-type/schema handoff.

## First connection and payload capture

When no direct marked island exists, the instance is in first-connection mode.
The runtime extracts author payload from exactly one of these forms:

1. ordinary instance child nodes, which may act as visible progressive-enhancement
   fallback before upgrade; or
2. one unmarked direct `<template>` with no other meaningful sibling, whose
   `content` is an explicit inert payload envelope. Per-instance CSS requires this
   form so it cannot affect the page before upgrade.

An unmarked direct `<template type="cem-ml">` or
`<template type="text/cem-ml">` is instead a CEM-ML source payload boundary.
The shorthand is accepted at author ingress and canonical serialization uses
`type="text/cem-ml"`. The template source remains in the payload part; its
validated namespace-aware AST is derived and must not be serialized as a second
competing authority.

An unmarked template mixed with meaningful siblings, or multiple direct unmarked
templates, is ambiguous and fails closed. A literal template intended as payload
must be nested inside the explicit payload envelope.

The runtime creates a new marked island and context root, moves the extracted
nodes into `cem-payload:payload`, records its explicit content type and schema,
and initializes every domain part in deterministic order. It does not relabel an
ordinary payload envelope as the island. Moving retains source-node identity
during capture; it does not make those nodes visible.

The declaration and the complete island state then produce a new rendered
projection. Source payload nodes remain inert in the island while materialized
render nodes occupy the instance's owned render range.

## Reconnection, serialized HTML, and hydration

The presence of one direct marked island selects resume mode before the runtime
examines any other child. In resume mode:

- lexical namespace bindings are resolved and the complete island is admitted
  through its context and domain schemas before any work begins;
- island payload and portable semantic runtime state are reused;
- no sibling node is captured or moved into payload;
- a valid identity-matched rendered range is adopted without an initial rerender;
- provisional content, including a server- or loader-supplied `loading...` view,
  is replaced when the instance renders on load; and
- invalid, incomplete, unknown, duplicated, out-of-order, identity-mismatched,
  or unsafe serialized regions diagnose and fail closed without turning rendered
  output into input data. Existing SSR output remains frozen static content; no
  render, event, resource, or capability work starts.

This rule applies both to a new element parsed from SSR HTML and to an existing
in-memory instance reconnecting to a document.

## Hydration data stays in the DOM model

Browser hydration data is HTML/XML DOM data carried by the marked island. It is
available to the first client render and remains part of the island for later
updates. Browser hydration must not require switching from the HTML/XML parser to
a JSON parser or treating a sibling JSON script as a second state authority. The
context-root and hydration-data versions must agree; the current pre-1.0 contract
is `0.1.2`.

Hydration has two valid outcomes:

1. **Render on load.** The island and any hydration-provided data are used as
   render input. Provisional or loading output is replaced by the committed
   projection.
2. **Adopt retained output.** When island version, instance identity, declaration
   artifact, data revision, scope policy, source fidelity, and render boundaries
   agree, the existing rendered range is retained and client invalidation takes
   over without an initial rerender.

DOM-native hydration data and rendered output must agree on revision identity.
On mismatch, retained output is not trusted. The runtime renders from the island
when the island schema is understood; it never reconstructs payload from the
stale output.

Structured-clone `DataIslandSnapshot` records remain valid processing-boundary
transport for workers, Edge, or other non-DOM hosts. They are derived from the
island and are not a parallel browser hydration authority. Policy-controlled SSR
serialization may omit or redact sensitive/transient sections; omitted data is not
recoverable from rendered DOM.

## State authority and portable capabilities

`cem-attributes:attributes` is authoritative after serialized load. Attribute
nodes retain exact DOM strings, including the empty string. The live host is
reconciled from those nodes after admission. `cem-dataset:dataset` is a dedicated
transformation/indexing view, not an independent authority; it must exactly equal
the DOMStringMap projection of canonical `data-*` attributes or adoption fails.

The resource part contains the full portable semantic runtime state: resource
identity and kind, lifecycle status, revision/cursor, policy stamps, diagnostics,
source provenance, safe request/response metadata, and typed result AST. Live
host capability objects—abort controllers, listeners, subscriptions, streams,
DOM nodes, storage API handles, and credentials—are not AST data. A browser or
server host recreates permitted capabilities from semantic state under its
current resolver, privacy, and security policies.

## Mutation and ownership consequences

- Payload-section mutations invalidate rendering while the original nodes remain
  inert.
- Host attribute, derived dataset, slice, form, validation, event, resource, and
  hydration changes update the namespace-aware island through one revisioned
  runtime transaction before a processing snapshot is derived.
- Runtime-authored island synchronization must not recursively trigger a second
  render transaction.
- Render commits may replace only the owned render range and must preserve the
  island node.
- Nested initialized DCE instances retain their own islands and render ownership.
- Serialization must preserve the island before the rendered range so reparsing
  selects resume mode before payload discovery.
