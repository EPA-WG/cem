# Scoped CSS fragment bindings

Status: deferred design; not implemented. Automatic contextual `url(#id)`
handling is on the [wishlist](wishlist.md#cem-elements-runtime), not the active
implementation path. The immediate authoring approach uses an explicit CSS
custom property containing a complete `url("#unique-target")` value and an
explicit matching target ID in the template body. CSS consumes that property
with `filter: var(--filter-url)`; a property containing only an ID cannot be
concatenated inside `url()`.

DCE now exposes `$instanceID` as a template binding; it does not install a CSS
property. See the [explicit authoring sample](../packages/cem-elements/README.md)
and [scoped CSS demo](../packages/cem-elements/demo/scoped-css.html).
The audit below describes the earlier counter-based allocator; new identities
now use UUIDs. Restored identities remain caller-owned, and automatic admission
and collision handling remain deferred.

The remaining sections preserve the proposed automatic binding design for
future consideration. They do not require URL rewriting, generated resource
slots, or an automatic ID/reference binding service in the current runtime.

## Binding model

A fragment-only CSS resource reference in a rendered template, such as
`filter:url(#local)`, identifies a resource in that template's produced output.
Importing the template from another module changes source provenance, not the
owner of its produced DOM. An explicit `url(./filters.svg#local)` remains an
external resource resolved through the closest module map and owning source URL.
Do not turn a missing local resource into an external/module-map fallback.
References to an unrendered source document must use an explicit source URL.

CSS fragment URLs resolve in a DOM tree; `@scope` does not create a private ID
namespace. Repeated light-DOM instances therefore require unique target IDs and
bound references. See [CSS Values](https://www.w3.org/TR/css-values-4/#local-urls).
The browser performs the final resource lookup after the DOM commit. Compilation
and rendering establish the binding without querying the live document for a
first matching authored ID.

The immutable native artifact retains these conceptual records (exact Rust API
names are implementation details):

| Record | Required information |
| --- | --- |
| Local target | template artifact identity, authored ID, target render-node identity, lexical/repeat ownership, source range |
| Local reference | CSS node identity, decoded fragment, originating style/template identity, lexical ownership, stable binding slot, source range |
| Rendered binding | owner identity, target occurrence identity, emitted DOM ID, slot value, revision and diagnostic state |

CEM-ML import owns URL decoding and classification. Render/compiler consumers
use typed CEM nodes; there is no CSS reparsing, parser-AST traversal, JSON AST
handoff, or live-DOM ID search. Fragment percent-decoding follows URL/element-ID
reference semantics exactly once, after CSS string/escape decoding. Malformed
or unsupported fragment syntax diagnoses instead of being guessed.

## Existing identity as prefix

For resources produced per instance, use the existing fully qualified instance
owner identity, including declaration identity where needed. Declaration identity
alone is suitable only for a resource that is actually emitted once and owned by
that declaration. Do not automatically hoist per-instance SVG definitions into
shared resources. Internal render-node identity is a target discriminator, not
an instance discriminator: the same template node can occur in many instances.

A logical key is:

```text
(owner kind, admitted owner identity, template artifact identity,
 target render-node identity, stable repeat occurrence identity, authored ID)
```

Encode each tuple component as UTF-8 hex, separated by `-`, under the reserved
`cem-r1-` prefix. Hex components cannot contain the separator; empty components
are retained. This is deterministic and injective, including Unicode, rather
than delimiter concatenation of arbitrary IDs or a truncated digest. CSS URL
serialization still uses the shared string encoder. A shorter illustrative ID
such as `instance-42--local` is not the wire encoding.

The existing browser allocator currently emits `cem-instance-N` from a counter
on each runtime. That string alone is not proof of document-wide uniqueness.
Implementation must trace the complete existing runtime/declaration/instance
identity tuple and admit it through a document-wide ownership registry shared
by participating runtimes. Reusing the same key for the same retained owner is
valid; two live owners claiming the same key diagnose and fail admission.
Admission must also reject a reserved physical ID already owned by an unrelated
connected node; the registry must not merely assume all document IDs originated
in CEM. This collision check is distinct from resolving authored IDs by a global
DOM search. Do not silently allocate a second identity system or reroll IDs during
hydration.
If current serialized identity cannot distinguish owners, extend the existing
identity contract before enabling these bindings.

Repeat occurrences use stable render keys, never merely the current array index
when keyed identity exists. An unkeyed structural occurrence can be deterministic
for one revision but does not promise persistence across reordering. Duplicate
keys or multiple targets for one reference are ambiguous and must not pick the
first DOM match.

## Owner identity audit: allocator decision pending

The audit found no existing document-unique instance identity to reuse as-is:

- `CemElementRuntime.instanceId()` in
  `packages/cem-elements/src/lib/cem-elements.ts` allocates `cem-instance-N` from
  `this.instanceSequence`, a counter on each runtime object.
- Declaration `scopeUid` derives from produced tag, seed and declaration
  occurrence. It identifies a declaration and is shared across its instances;
  it cannot qualify a repeated instance by itself.
- Hydration restores `snapshot.instanceId` into the per-runtime WeakMap. The
  allocator has no document-wide reservation/admission registry. Restored and
  newly allocated IDs therefore need coordinated admission before they can serve
  as resource-owner prefixes.
- Native `cem_ml`/`cem_ql` currently has no corresponding instance-owner identity
  allocator to enforce uniqueness independently of these host inputs.

Combining declaration identity with the runtime-local counter does not establish
a document-wide guarantee across equivalent registrations, separately created
runtimes or separately produced SSR fragments. Native tuple encoding can preserve
uniqueness only after ownership inputs themselves are admitted.

**Recommended decision:** upgrade the existing `instanceId` allocation contract,
not add a parallel resource identity. New instances receive document-unique IDs
from the shared admission service. Browser allocation may use a UUID candidate
with an actual document-registry collision check; deterministic SSR may use an
explicit document/render namespace plus stable occurrence identity. Both paths
populate the same existing semantic `instanceId` field. The precise allocator
must be documented and tested with the selected host lifecycle.

Restore already serialized IDs unchanged when admission succeeds. Reconnecting
the same owner reuses its reservation. Two distinct live owners claiming the same
restored ID fail admission; do not rewrite only CSS resources, silently reroll the
restored instance, or invalidate unrelated hosts. Fresh allocation must reserve
against all restored owners before their IDs are used for bindings. Native
binding inputs receive only admitted ownership identities.

**Alternative:** keep runtime-local allocation and require every host/SSR caller
to provide a unique namespace, persisted with instance identity. This exposes a
new caller obligation and needs defined behavior when namespaces are absent or
reused; it cannot be implemented as an optional best-effort prefix.

The choice changes the identity/lifecycle contract and may require a coordinated
hydration schema/version extension for admission state or deterministic namespaces.
Implementation is paused before allocation, serialization or browser behavior
changes. First add collision/restoration fixtures after this decision; do not
claim document-wide binding safety from the hex-encoding helper alone.

## Shared CSS and per-owner values

Compile a local URL occurrence into a reserved custom-property slot. Slot names
encode the effective style artifact identity and retained reference identity,
without any instance identity. A schematic example is:

```css
/* Installed once for the effective declaration/context. */
@scope (cem-picture) {
  [part="image"] { filter: var(--cem-internal-frag-r1-SLOT, url("about:invalid")); }
}
```

The render output has a unique target ID and a runtime-owned binding value:

```html
<!-- Schematic IDs/slot names, not their wire encoding. -->
<cem-picture style='--cem-internal-frag-r1-SLOT: url("#INSTANCE-TARGET")'>
  <svg><defs><filter id="INSTANCE-TARGET">…</filter></defs></svg>
  <div part="image">…</div>
</cem-picture>
```

CSS custom properties carry complete `url(...)` values through `var()`;
[their values inherit and substitute at computed-value time](https://www.w3.org/TR/css-variables-1/).
Local fragments stay fragment-only; external URL values must already be absolute
before entering a binding, so variable substitution cannot change their base.
No `@property` registration or generated instance-ID selector is required.

This design introduces a narrow runtime-owned resource-binding channel using
reserved custom properties in the host/produced carrier's style declaration.
It does not permit authored inline presentation, component JavaScript, public
`--cem-*` token overrides, arbitrary CSS strings, or cloning declaration styles.
Native render output contains typed binding operations; browser/SSR adapters only
materialize them. CSSOM updates touch only the reserved properties and preserve
unrelated host attributes/styles. Serialized binding values are output state,
never new payload or authored input on reconnect.

The initial supported carrier is the produced DCE host. Bind every slot used by
its effective private/shared stylesheet set on that host, including explicit
invalid-resource values for missing bindings. Each nested DCE initializes its own
slots before commit so the same declaration cannot inherit a parent's valid
binding when its own target is absent. Distinct style artifacts have distinct
slot names. Named shared styles resolve their references against each receiving
instance's generated resource namespace; a missing target affects that receiver,
not other members of the shared scope.

A repeat-local target needs a repeat-local carrier and unambiguous reference
ownership. A host-wide CSS rule cannot choose among several copies of `#local`.
Initially suppress that binding with an ambiguity diagnostic; do not arbitrarily
bind the first occurrence. A later carrier extension must define how every
produced root of a repeat gets its value, handles nested DCE boundaries, and
preserves projected-content ownership. Identity encoding includes occurrence
identity now so that extension does not require changing the ID format.

## ID and reference consistency

Allocate generated IDs only for resources participating in managed local
bindings, plus explicitly enrolled local DOM references. Keep the authored ID in
the retained semantic model; the physical DOM receives the generated ID. Update
all enrolled references to that target in the same render transaction, including
fragment `href`/SVG `href` (including namespaced XLink) and supported IDREF/IDREFS
attributes such as `for`, `aria-labelledby` and `aria-describedby`. CSS-valued SVG
presentation attributes must use the same native CSS import/binding boundary.
Unknown reference grammars must diagnose before renaming a participating target;
do not ship partial rewrites of known consumers. Classification belongs to the owning
HTML/SVG content-type boundary, not a browser-wide string replacement.

Do not rename unrelated IDs, user-projected IDs, host IDs, or targets owned by
nested DCEs. A CSS URL cannot reach into another instance through authored-name
lookup. Explicit external/public references remain explicit. Public authored IDs
that external code depends on are not eligible for silent renaming: diagnose a
binding/ownership conflict until an explicit resource export contract exists.
Reserved physical resource IDs are implementation details, not public selectors.

References inside custom-property values are compiled at their originating
style site, preserving the origin's template/context. Passing a value through
`var()` does not reclassify its target against the consumer's source module.
Resource-only URLs are distinct from fragment navigation and ordinary module
imports. String-valued CSS resource grammars require their own native import
support; this design does not imply support for all `image-set()`/future syntax.

## Lifecycle and failure behavior

1. Compile the static target/reference manifest and shared stylesheet slots.
2. Render into retained output, index evaluated authored IDs of owned nodes, and assign/admit
   physical IDs for that revision. Match targets by lexical ownership and
   occurrence, not by global ID lookup. Conditional/dynamic IDs are indexed after
   native evaluation; changes rebind within the same revision and never mutate
   an immutable source manifest.
3. Produce target ID writes, enrolled reference writes and slot values as one
   revisioned binding plan. Missing/ambiguous references receive
   `url("about:invalid")`; never fall back to an ancestor slot or an authored ID.
4. Materialize detached output, then commit it and its binding values together.
   Host bindings settle before the runtime reports rendering/style readiness.
   On target removal invalidate the slot in the same commit. A blocked or failed
   binding never initiates an external fallback fetch.
5. Reconnect reuses owner/occurrence identity and current bindings. Disposal
   releases ownership registrations and runtime-owned properties. Adoption into
   another document must re-admit identities before activation.

SSR uses the same native binder and encoding. Serialize the identity, binding
revision, artifact/context identities and scoped-reference state through the
existing DOM-native data island, with generated IDs and reserved properties in
rendered output. This is a versioned semantic extension, not a sibling JSON
manifest or a second hydration authority. Hydration verifies ownership, target
IDs, enrolled references and slot values before adopting output; mismatches use
the lifecycle contract's fail-closed rules. A serialization version change must
be coordinated across context and hydration schemas before shipping.

Immutable parsed trees and reference manifests may be shared. Bound IDs and slot
values are keyed by admitted owner, occurrence, artifact/context and revision;
they must never enter a source-content-only cache. Only actual declaration-owned
singleton resources may use declaration-wide bound IDs.

Planned diagnostics include missing target, ambiguous target/occurrence,
identity collision, unsupported carrier, public-ID conflict and hydration binding
mismatch. Attach both the reference source and relevant target/owner provenance.
A failed binding disables its resource use; successful sibling bindings and other
instances continue. This is functional ownership, not a security boundary against
scripts or arbitrary page CSS modifying the DOM.

## Implementation sequence and acceptance

The active [todo list](todo.md) owns the fixture checklist. Implement native
classification and identity admission first, then binding plans and code generation,
then browser/worker/SSR transport and hydration. Before enabling installation,
prove two same-declaration instances bind different resources while sharing one
stylesheet; prove a nested instance cannot inherit a missing local binding;
prove keyed reorder/hydration preserve IDs; and prove imported-template provenance
does not turn generated-DOM references into source-document fetches.

The proposed runtime-owned inline resource channel, IDREF handling and hydration
extension must be reflected in their owning contracts when implemented. Until
those checks pass, existing runtime behavior remains unchanged.
