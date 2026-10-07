# cem-element reference-to-ID consumption

Status: adopted and implemented for the shared native render/export path on
2026-10-07. This is a CEM AST consumer contract. CEM-ML retains reference syntax
and original owners; it neither chooses DOM relationships nor extracts IDs.

A cem-element transformation evaluates its template with that instance's supplied
`datadom` and native bindings. At final DOM export, the native consumer recognizes
node/reference values in relationship attributes and maps their terminal element
targets to produced placements. A target must have exactly one placement in the
current producer forest. It may occur after the invoker. Selecting the same source
node twice for output makes its placement ambiguous; the consumer never chooses
one by position or an ID scan. Use distinct constructed nodes when both copies
need independent relationships.

The runtime enables the mode with its persisted instance identity. It supplies no
ID for the context root. Generic CEM-QL rendering remains available without this
mode. CLI/embedding callers opt into the native projection API explicitly.

## Relationship attributes

Only native node/reference values activate this consumer. Plain strings retain
existing browser or convenience semantics. Mixed lexical text and native targets
fail target validation rather than forming an ID by concatenating text.

| Attributes | Native target contract |
| --- | --- |
| `commandfor`, `popovertarget`, `interestfor`, `form`, `list`, `aria-activedescendant`, `aria-details`, `aria-errormessage`, `for` outside `output` | Exactly one element |
| `aria-controls`, `aria-labelledby`, `aria-describedby`, `aria-owns` | Nonempty ordered element sequence; repetitions retained |
| `headers`, `output`'s `for` | Nonempty element sequence exported as unique ID tokens in first-occurrence order |
| `command-target`, `interaction`, `trigger-for`, `parent-item` | Exactly one element/provider; its owning interaction capability validates the semantic role |

The initial ARIA contract follows the [WAI-ARIA 1.2 Recommendation](https://www.w3.org/TR/wai-aria-1.2/),
whose [details](https://www.w3.org/TR/wai-aria-1.2/#aria-details) and
[error-message](https://www.w3.org/TR/wai-aria-1.2/#aria-errormessage) relationships
each designate one element. A future ARIA profile can adopt the draft list-valued
forms explicitly. HTML's [output `for`](https://html.spec.whatwg.org/multipage/form-elements.html#attr-output-for)
is a token set, whereas [label `for`](https://html.spec.whatwg.org/multipage/forms.html#attr-label-for)
designates one element. This consumer enforces target shape and placement;
browser and provider semantics still govern the selected element's role.

For example, `@commandfor={#target}` writes the produced target's ID to the native
attribute. `@command-target={#provider}` designates the provider. The action
capability continues to choose its actual native surface for a built-in command,
or the provider endpoint for a custom command. Native `commandfor` designates the
exact selected element and is never redirected to a descendant surface.

The consumer emits runtime-owned `data-cem-node-ref-command-target` and
corresponding interaction-property markers beside lexical IDs. These distinguish
consumed typed inputs from existing `@local-name` strings. They are projection
metadata, not an authoring syntax, context ID, selector language, or crossing
grant. Templates cannot author them. The action capability consumes the markers for `command-target` and `interaction`;
the popup and composite-menu capabilities consume `trigger-for` and `parent-item`.
These shared providers observe endpoint/marker changes and release their claimed
ARIA attributes on rebinding or disconnect. A popup endpoint must be a native
button/link or a provider with a direct native control. It replaces the generated
default base; an explicitly authored base plus an external trigger, or an endpoint
inside the popup itself, is a conflict.
A submenu endpoint must be an actual control of one connected composite-menu
provider. Each control has at most one submenu; cycles and conflicting panels
disable execution. Invalid explicit endpoints never select an intrinsic fallback.

The existing `interaction-name`, `interaction-scope`, and `@local-name` inputs
remain on their explicit compatibility path. They never become AST references,
and their lookup cannot escape the nearest existing compatibility scope. Existing
legacy `#id` interaction strings remain a browser compatibility input; new typed
inputs use CEM-ML `#` construction. Native HTML ID strings retain browser meaning.
No CEM-QL URL handler, CEM-wide ID registry, or implicit flattened CEM lookup is
introduced. URL fragments remain loader/public-export conventions.

## Evaluation, scopes and incomplete links

The default browser consumer follows already-materialized reference edges in the
admitted producer invocation. Normal template reference expressions have already
run with that invocation's inputs. An authored source reference returned from
another native tree is still a source node. Its expression must be evaluated by
an explicit lifecycle host with its original captured bindings and runtime
context; the projection does not guess that context from document ownership.

Native callers use `project_element_reference_ids_with_host` with the existing
`ReferenceResolutionHost<Node = Item>`. `CemQlSchemaDeclarationHost` exposes
`element_reference_host(requesting_scope)` as an adapter to its already-registered
contexts, policies and directed grants. This creates no scopes or authority.
The adapter resolves relationship value slots. Authored references in the body
must already have their producer lifecycle completion before materialization;
the export walk does not evaluate those source expressions.
Independent vendor scopes require explicit grants even when both targets will
appear in one output forest. Destination limits constrain the subtree together
with request limits. All relationship value slots in a producer use one bounded
owning traversal; neither crossing a scope nor moving to another attribute resets
work accounting. Owning slot containment adds no reference depth. The source's effective
schema controls unresolved-link diagnostics. Pending, warning or ignored
incomplete chains cannot become successful ID relationships.

Browser/worker hosts can now supply `CemElementRuntimeOptions.elementReferenceInputs`
for each render. The callback receives the instance, captured snapshot and lifecycle abort signal and can
return an asynchronous `CemElementReferenceInputs` value. This separate host
channel carries explicit source bundles, source selections, context readiness,
requesting source, effective policy overrides and directed grants.
`refreshElementReferences(instance)` invalidates pending work and requests a new
invocation after the host replaces context/readiness/authority. Neither
snapshot JSON nor authored documents create this authority. Selections are passive
native queries over original retained source owners and stage declared native
slices atomically. They all use the input frame before staging; selection order
creates no override dependency. Ready sources explicitly receive the completed
invocation frame, including `datadom.slices`; unready contexts stay pending.

`api::element_references::ElementReferenceExecution` supplies the native handoff.
It requires original lexical capture and builds a fresh lifecycle host per
invocation. The browser transport uses the existing explicit **debug CEMB reload**
boundary, then disposes local source handles after consumption. CEMB remains an
experimental codec, not a promised stable production format. Policy overrides
choose nonzero depth/work bounds and standard unresolved dispositions; the native
API also accepts complete schema-owned policies. Preparing bindings, importing an
owner or setting readiness grants no crossing. All relationship slots still share
request and destination accounting.

Worker protocol v15 carries these host inputs; main-thread fallback and Node SSR
use the same retained native ingress. Replacement/disposal aborts pending
invocations and releases the publication queue before late callbacks complete. Missing contexts/grants and warning/ignored
incomplete links publish no replacement plan and preserve the last committed
relationships. Existing hydration snapshots carry produced IDs and persisted
instance identity; contexts/grants must be supplied afresh by the host and are
never serialized into the island. Relationships to elements produced by another
instance still require a separately designed placement/authority channel; the
consumer rejects targets outside its own forest.

Resolution follows reference chains, not targets' owning descendants. Metadata
and DOM export walk owning structure under the existing operation control and
value budgets. Original AST nodes, source maps and authored target lists stay
unchanged. Projection uses a temporary native identity-to-placement map, then
writes browser lexical values into a new output plan. Intermediate native trees
are never serialized/reparsed or substituted with DOM/JSON records.

## Focus and geometry slots specified for the next provider slice

The accepted native forms are `@focus-target={#node}`, `@return-focus={#node}`,
`@anchor={#node}` and `@boundary={#node}`. Each selects exactly one produced element
through the same bounded lifecycle/placement contract. Literal `auto`/`none`,
`invoker`/`pointer`/`selection` and `viewport` retain their existing meanings.

A focus target must be connected, visible, enabled and focusable within the
semantic surface. A return target must be connected and eligible when the closing
reason permits restoration; it need not be inside the surface. An anchor supplies
geometry rather than activation or focus ownership. A boundary supplies the
fitting rectangle, intersected with the visual viewport. Selecting one never
changes the other relationships. Missing focus targets use the accepted safe
semantic fallback with diagnostics; unavailable geometry follows the declared
anchor-lost/opening policy. Cross-instance targets require the future placement
channel, even when a DOM ID happens to exist.

These four slots are specified but not yet enabled by the native exporter or
shared provider capabilities. Their implementation and role/readiness fixtures
are explicit todo actions. This preserves the distinction between the accepted
interaction design and currently shipped behavior.

## ID ownership and publication

An existing target ID is preserved. A referenced empty, whitespace-containing or
ambiguous authored ID fails the relationship. An unreferenced target gets no ID.
When needed, the consumer generates
`<persisted-instance-identity>-ref-<produced-occurrence-path>`. Each target gets
one generated ID shared by its invokers. Unchanged instance identity and placement
produce the same ID across renders; distinct instances produce distinct IDs.
Changing output placement may change the generated ID and its relationships in
the same transaction. Hydration retains already-produced IDs through the existing
render plan and data-island ownership contract.

Generated IDs cannot overwrite or collide with IDs already present in the
producer forest. The browser interaction capability also rejects ambiguous native
ID lookup. Authors exposing explicit IDs retain their existing browser uniqueness
responsibility across instances.

Missing, non-element, empty, multiple single-target or ambiguous placements,
reserved metadata, denied crossings, stale contexts, and exhausted/cancelled
traversal return attributed diagnostics and no replacement plan. Native callers
can inspect the error without changing the retained input plan. The shared worker,
main-thread fallback and SSR processing boundary use the same native algorithm;
this mode adds no component JavaScript or browser AST evaluator.

Fixtures and subsequent lifecycle/interaction adoption actions, with adjacent
verification scenarios, are maintained in [todo.md](todo.md#deferred-cem-element-reference-consumption).
