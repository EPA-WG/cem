# Generic retained-payload item/group contract

Status: implemented declaration-metadata foundation, `cem-retained-payload/1`.
The six component-aware readers listed in the [coupling audit](cem-capability-coupling-audit.md)
remain separate migrations. This contract introduces no component JavaScript.

## Authoring

Declare the vocabulary on a `<cem-element>` declaration or its direct canonical
`<template>`, before the template body is compiled or an instance upgrades:

```html
<cem-element tag="example-choice"
             payload-item="entry option"
             payload-group="section optgroup"
             payload-topology="grouped">
  <template id="example-choice" type="text/cem-ml">{p | Choice view}</template>
</cem-element>
```

| Header | Meaning |
| --- | --- |
| `payload-item` | One to eight whitespace-separated expanded element names. Required whenever any contract header is present. |
| `payload-group` | For grouped topology, exactly one group name per item name, in corresponding order. Forbidden for the other topologies. |
| `payload-topology` | `flat` (default), `grouped`, or `recursive`. |

Each corresponding item/group pair is one vocabulary. Order is stable and
meaningful; it is not sorted during normalization. All names across all item
and group roles must be distinct after namespace expansion. A reader can use
the vocabulary index to retain the relationship between an item and its group.
Whether different vocabularies may coexist in one document is a reader rule.

Names identify **retained semantic names**, not DOM selectors. A bare `entry`
and `Q{}entry` mean the same unqualified name. `Q{urn:catalog}entry` names an
element by namespace URI and local name, independent of its source prefix.
Namespaces and local names are case-sensitive. A literal `prefix:entry`, CSS
selector, wildcard, path or ID is not a declaration name.

The existing payload snapshot stores implicit HTML names as unqualified names;
the matcher preserves that convention. An explicitly retained XHTML namespace
requires `Q{http://www.w3.org/1999/xhtml}entry`. Empty and null retained namespace
values both mean no namespace. The classifier does not infer namespaces from
the rendered DOM or broaden an explicit namespace to match an unqualified name.

Bounds are eight vocabularies, 8 KiB of UTF-8 per decoded source header, 256 Unicode
scalars per XML NCName local name and 1,024 scalars per namespace URI. Namespace
values cannot contain whitespace, control characters, BOM or braces. Native
metadata additionally retains its existing 1 MiB transport bound. The local-name
profile follows the XML 1.0 NCName character ranges, including non-ASCII names.

## Topology and semantic validation

`flat` describes direct item siblings; `grouped` describes direct items and
groups containing their own items; `recursive` describes item children forming
a tree. These are structural contracts for readers to enforce. This foundation
normalizes the declaration and classifies elements; it does not walk a live DOM,
harvest descendants or construct a replacement payload tree.

Reader-specific rules remain explicit: required values/labels, duplicate values,
disabled inheritance, rich content, mixed-vocabulary policy and domain bounds.
Migrations must preserve their diagnostics and original retained nodes/keys,
source ownership, maps and topology. They must not introduce another parser or
translate the input through a JSON/DOM data substitute. The declaration metadata
JSON in compiled builds is an existing named control boundary, not a data AST.

## Ownership and lifecycle

A contract is one **atomic header block**. An owner and template may each supply
a complete block if their normalized contracts agree. Partial inheritance and
conflicts are rejected; an empty header is invalid, not an instruction to clear
an inherited field. Absence of all three headers means no contract.

Inline declarations, local `src="#template"` references and selected remote
declaration/template fragments use the same normalization. Source loading copies
metadata only from the selected template's direct declaration owner; unrelated
ancestors and neighboring declarations supply none. The cached source document
is not edited. A document loaded without selecting a declaration fragment keeps
the existing document-source semantics; it does not infer an owner vocabulary.

The normalized value and all names/vocabulary arrays are frozen and retained
on the compiled declaration. `CemProducedElementBehaviorContext.payloadContract`
is available during `constructed` and before the first `beforeRender` model
normalization, including upgrade of pre-existing instances. Children may still
arrive later; their lifecycle remains the existing retained-payload lifecycle.
Instance attributes, data-island slices and rendered nodes cannot replace this
value. Header edits require a new registration; they do not silently redefine
an existing browser constructor. Reconnection retains the original contract.

The package exports `resolveRetainedPayloadContract`, the readonly contract/name/
vocabulary types, and `classifyRetainedPayloadElement`. Classification returns
`{ role, vocabulary }` or null, reads only a retained element's semantic name,
and leaves its key, children, attributes and source records untouched. It grants
no source lease, resource read, DOM ownership or checkpoint authority. Readers
must obtain their nodes from the original admitted payload, not a selector result
or a matching serialized identifier.

## Registration, compiled builds and readiness

Normalized contracts participate in registration identity and the processing
compile input. Equivalent header spelling/whitespace yields the same contract;
changing a vocabulary or topology changes identity even when the body is equal.
Existing registrations without a contract retain their prior identity inputs.

Native header ingestion decodes predefined and numeric XML character references
once through the shared `cem-ml` attribute decoder before name validation. It
does not expand DTD/custom entities or reconstruct a document. Browser source
attributes arrive already decoded by the browser.

The native declaration builder validates the same header rules and emits an
optional `payloadContract` field in pinned `cem-declaration-metadata/1` control
metadata. Old headers omit that field. Native admission validates nested names,
topology, ambiguity and bounds after checking the original metadata hash. Pinned
registration accepts an explicit wrapper block only if it agrees with admitted
metadata; overrides are rejected before publishing the constructor. The browser
freezes a copy of the native-admitted contract before lifecycle callbacks run.

This does **not** extend the passive SSR readiness/hydration profile. Native SSR
headers and browser canonical adoption refuse annotated declarations until that
profile explicitly carries and validates the metadata. Refusal must preserve
the original source/rendered range; it must never drop the contract and claim
readiness. Full reader adoption and its recovery/SSR evidence remain in TODO.

## Evidence

Native tests cover header normalization/admission, original hash pinning,
malformed rehashed metadata, legacy-header compatibility and explicit SSR
refusal. Shared framework stories cover worker/fallback source and compiled
registration, pre-existing-instance upgrade, first-callback availability,
namespace/prefix matching, source-fragment isolation, forged instance metadata,
conflicts, immutable state and reconnects. The reader migrations have not yet
been accepted by these metadata-only fixtures.

Focused verification on 2026-11-16 passes 43 runtime unit tests, 14 browser stories
(source and compiled worker/fallback lanes), and 12 native cases across contract
validation, compiled metadata, shared XML decoding and SSR refusal. Runtime
build/typecheck/lint/package and component declarative gates pass; existing lint
warnings remain. No component CSS, public theme tokens, timeout or worker-budget
settings changed, so this foundation makes no new visual/theme acceptance claim.
