# CEM platform pitch: evidence and proof requirements

Reviewed 2026-10-10 against repository revision `9de4302a`, current local
documentation, the supplied DCE PDF, and primary external sources.
This was a source/documentation audit; no new runtime tests or benchmarks were run.

This record supports the [manifesto](cem-manifesto.md), audience-facing
[platform pitch](cem-platform-pitch.md), and internal
[presentation plan](presentation-plan.md). **Implemented** means supported by
current code, tests, or recorded verification, not that the conference demo or
public release has been verified. **Partial** identifies a bounded capability.
**Direction** identifies a goal or proposed demonstration.

**Delivery update, 2026-10-11:** Sasha confirms Studio is not ready for
participant use. [Discussion #24](https://github.com/EPA-WG/cem/discussions/24)
is the stable resource destination; add Studio and other access links there as
they become available. The implementation evidence below does not establish
public availability.

## DCE promises carried forward

The supplied
[DCE-presentation.pdf](/home/suns/aWork/Syngrafact/DCE-presentation.pdf)
is a local source outside this repository. Page/slide 6 was rendered and
visually inspected; its diagram explicitly names **Memory, Storage, CPU,
DOM, CSS and Network** as resources for the host and each embedded application.

| Original deck | Principle to retain | Evolution into the CEM pitch |
| --- | --- | --- |
| Slides 2–3: integration expands the attack surface and repeats expensive work | Composition needs resource boundaries and reusable integration | Explain one provider/host workflow and the exact controls around it |
| Slide 5: declarative custom elements as a microfrontend mechanism | A reusable UI should participate in a host's workflow | Connect component declarations to data, state, dependencies and authority |
| Slide 6: each party has its own guarded resources | Nested inclusions should preserve boundaries across six resource layers | Show requested authority, host grants, enforcement and denied actions |
| Slides 7–8: templates, modularity, HTTP, storage and discoverability | A declarative platform needs practical application behavior and debugging | Demonstrate a useful capability and trace its result to the source |
| Later roadmap: productization and standards work | Adoption needs delivery, examples and ecosystem work | Use a small verified pilot before making broad replacement claims |

The slide's blanket contrast between JavaScript and insulated DCE resources
is a vision, not a demonstrated property of all current CEM applications.
Browser-enforced iframe restrictions already exist; shared-realm scripts,
declarative scopes and separately isolated runtimes have different authority.

The earlier deck's reliance on browser-native XSLT is also historical.
Current CEM uses its own common native/WASM engine and a bounded explicit
XSLT compilation/runtime profile.

## Ideas from EPA and DWA

Reviewed at the user's request as **idea sources, not guides**. These older
proposals do not change CEM's implementation contracts, establish current
capabilities, or create new roadmap obligations. The following are positioning
choices and proposed proof scenarios.

| Source | Useful idea | Effect on the current pitch |
| --- | --- | --- |
| [EPA concept](https://github.com/EPA-WG/EPA-concept/blob/master/README.md) | Independent provider/host releases; multiple configured instances | Explain the commercial ownership boundary and test separate instance state |
| [Microapplication / pagelet](https://github.com/EPA-WG/EPA-concept/blob/master/microapplication.md) | A capability works in host context; the host can supply persistence | Start with a read-only view, then add declared services for effects |
| [EPA feature sketch](https://github.com/EPA-WG/EPA-concept/blob/master/EPA-features.md) | Several presentation modes and host interaction | Propose compact, expanded and dedicated views of one capability |
| [Multi-provider workflow note](https://github.com/EPA-WG/EPA-concept/blob/master/web-3.0.md) | Specialist products can contribute to one experience | Describe the customer workflow directly; omit the historical “Web 3.0” taxonomy |
| [DWA proposals](https://github.com/EPA-WG/dwa) | Explicit version adaptation and controlled customization | Add a tested v2-to-v1 feed adapter as an evolution example and make extension points explicit |

The synthesis is a stronger business proposition: **a provider maintains a
useful capability; each host supplies context and explicitly governs its
effects.** A proposed proof shows two supplier panels with different inputs
and independent selections, then gives one a permitted host action. The
presentation modes and complete application contract still need implementation
and verification; this review created no fixtures.

The EPA closure/redefined-global/ES5/ShadowRoot isolation recipe is not adopted.
It neither proves hostile-code containment nor matches CEM's light-DOM design.
Its incomplete feature table is not competitive evidence.

DWA's declarative-only security, automatic parallelism, universal compatibility
and optimization claims remain unproven here. Do not adopt its suggestions to
omit accessibility support or bypass validation based on past successful use.
Current CEM contracts and measured behavior govern those decisions.

## CLI, library, and pipeline applications

CEM's processing foundation supports adoption independently of a web interface.
These sources establish entry points and graph capabilities; the proposed
supplier CLI/service/view comparison still needs a composed fixture.

| Surface | Evidence | Positioning use |
| --- | --- | --- |
| Native library | [Core library ownership](../../packages/cem_ml/README.md), [typed graph API](../../packages/cem_ml/src/engine.rs) | Embed parsing, validation, query, transformation and reporting in existing software |
| CLI | [Native CLI](../../packages/cem_ml_cli/README.md) | Apply reusable library services through command arguments, streams, host I/O and exit policy; useful in batch work and CI |
| Node/browser library access | [WASM library](../../packages/cem-ml-npm/README.md), [typed command services and host adapters](../../packages/cem-ml-cli-npm/README.md) | Embed processing in services, developer tools and browser experiences through supported interfaces |
| Several sources and output branches | [Graph lowering](../../packages/cem_ml/src/transform_graph_request.rs), [graph conformance tests](../../packages/cem_ml_cli/tests/cem_native_module_conformance.rs) | Import/convert/join/transform/export nodes, source grouping, named exports, reports and source-map sidecars |
| Web asset delivery | [Studio assembler and workbench](../../packages/cem-studio/README.md) | Use a graph for deployable assets as well as data products |

A host service can invoke the library and return its output through an endpoint.
This does not establish a built-in HTTP server, distributed queue, scheduling
service, authentication system, or automatic deployment. CLI automation uses
the surrounding job runner; service deployment uses the surrounding host.
Native, Node and browser I/O capabilities differ.

Graph joins group source bindings and artifacts. Matching product records by ID
requires explicit query/transformation logic. Multiple declared exports do not
imply arbitrary dynamic splitting or a transaction across all destinations.

## Development lifecycle

The pitch describes the developer's lifecycle and the definitions shared
between its stages. These are the supporting platform surfaces; the full
supplier walkthrough remains a demonstration to prepare and verify.

| Lifecycle foundation | Evidence | Claim supported |
| --- | --- | --- |
| Source and schema choices | [Schema-package development model](../../packages/cem_ml/schema-packages/README.md), [resolver interfaces](../../packages/cem_ml/src/resolver.rs) | Pick, extend, or create versioned schemas for vocabulary, types, constraints and diagnostics; explicit resource access and format boundaries |
| Module authoring and reuse | [Transformation adapter](../../packages/cem_ml_transform_cem_ql/README.md), [query/template modules](../../packages/cem_ql/README.md) | Reusable transformations, explicit module integration and the documented XSLT profile |
| Composition and delivery artifacts | [Graph implementation](../../packages/cem_ml/src/transform_graph_request.rs), [Studio graph assembly](../../packages/cem-studio/README.md) | Dependencies, source combinations, output branches and explicit asset assembly |
| Interactive development and diagnosis | [Studio workbench](../../packages/cem-studio/README.md), [common engine](../../packages/cem_ml/README.md) | Source/result inspection, previews, traces, graph views, diagnostics and portable projects |
| Repeatable checks | [Schema-package example requirements](../../packages/cem_ml/schema-packages/README.md), [CLI graph tests](../../packages/cem_ml_cli/tests/cem_native_module_conformance.rs), [component lifecycle](../cem-element-lifecycle-principle.md) | Fixtures, expected results, error cases and compatibility checks supply patterns for testing consumer solutions |
| Browser, server and CI/CD execution | [Native CLI](../../packages/cem_ml_cli/README.md), [native library](../../packages/cem_ml/README.md), [Node/browser hosts](../../packages/cem-ml-cli-npm/README.md) | The shared engine can be invoked by interactive hosts, embedded services and automated jobs |

The connected lifecycle uses the team's existing test runners, CI/CD
orchestration, source control, deployment and operational systems. CEM provides
processing, artifacts, diagnostics and host interfaces those systems can use.
Studio inspection and tracing support debugging; complete IDE/LSP/DAP tooling
remains separately tracked in the [roadmap](../../roadmap.md). The platform's
own runtime release packaging is distinct from release automation supplied by
a consuming team for its application or service.

## Enterprise transformation precedent

The useful historical comparison is the enterprise XML/SOA processing market:
validation, transformation, service integration, and controlled data exchange.

- Intel's [2005 Sarvega acquisition announcement](https://www.intel.com/pressroom/archive/releases/2005/20050817corp.htm)
  connects XML processing acceleration, security and network integration with
  its enterprise platform strategy.
- Intel's [2008 SOA Expressway for Healthcare announcement](https://www.intel.com/pressroom/archive/releases/2008/20080421comp.htm)
  describes a software product intended to reduce integration cost and connect
  different data formats across healthcare networks. It is evidence for the
  market need, not for an XSLT-only or hardware-only product characterization.
- IBM's [DataPower/Transformation Extender introduction](https://www.ibm.com/support/pages/getting-started-websphere-transformation-extender-design-studio-datapower)
  supplies an explicit hardware-appliance precedent with XML/XPath/XSLT
  processing. It also describes non-XML transformations, several input/output
  cards, batch/API/web-service execution, and integration with other products.

These predecessors cannot fairly be characterized as universally restricted
to XML or one data location. CEM's proposition is the combination of extensible
formats, host-controlled resource access, a common native/WASM engine, retained
provenance, and application composition. Similar workload classes do not prove
drop-in compatibility, feature parity, or equal hardware acceleration.

### Format and location independence

| Boundary | Current evidence | Meaning of the positioning claim |
| --- | --- | --- |
| Representation | [Schema-package architecture](../../packages/cem_ml/schema-packages/README.md), [shared data import contract](../cem-data-import-principle.md) | Supported formats meet at a common typed boundary. A new syntax needs its parser/import adapter; an output format needs an exporter; semantic mapping remains explicit |
| Resource location | [Resolver API and registration](../../packages/cem_ml/src/resolver.rs) | Reads, writes, optional listing and prepared publication are host extension points. Native default access covers local paths and local file URIs; other locations require registered support |
| Host I/O | [Node/browser host interfaces](../../packages/cem-ml-cli-npm/README.md), [WASM command bridge](../../packages/cem-ml-npm/README.md) | Node provides files, HTTPS reads and application-supplied streams; browser hosts provide resource and publication capabilities. In-memory inputs and returned artifacts support library embedding |

**Extensible, customizable connectors** are the platform claim.
`ResourceResolver` and `ResolverRegistry` expose the extension interfaces for
custom reads, writes, optional listing and prepared publication. Teams can
implement or adapt connectors to their own systems while keeping processing
contracts shared. Format adapters separately extend representation support.
Connector implementations integrate with host-controlled permissions,
credentials, transport behavior and publication semantics.

The proposed proof keeps the domain query and input values fixed while changing
the stock binding from a local file to permitted HTTPS or host-supplied memory.
It compares domain results, checks the new source attribution, changes a
supported output publication path, and verifies that a denied location remains
denied. This is a demonstration requirement, not a newly executed test.

## Six resource layers

The architectural proposition is security insulation around each independently
supplied scope, across applications, pipeline stages, and connectors. Memory,
Storage, CPU, DOM, CSS, and Network are distinct enforcement layers. A vendor
declares its data access, effects, and resource needs; the host grants
authority; the applicable runtime layers enforce the resulting boundaries and
attribute denials. Nested inclusion preserves the insulation, delegated
authority, and aggregate resource budgets.
Existing controls below are useful building blocks, with different scopes and
threat models.

Acceptance requires observable insulation at the relevant layer. A declared
permission, ownership label, or resource counter establishes only its documented
behavior. In particular, allocation accounting is distinct from private-memory
access isolation, and CSS ownership is distinct from containment of all style
effects. The full architecture must close every effect path across a boundary.

The proposed contribution-level example separates supplier availability,
shipping quotation, and UI rendering. Each scope receives an explicit input
projection and permitted actions. Acceptance must cover data crossing, reads,
publication, network effects, descendant budgets, active output, and revocation.
It is not yet an implemented end-to-end demonstration. The positioning concerns
an integrated security architecture across the declarative composition;
it does not establish that capability-based security itself originated in CEM.

| Layer | Existing foundation | Boundary still to establish for arbitrary vendor scopes | Convincing proof |
| --- | --- | --- | --- |
| Memory | Hierarchical accounted allocations, input/artifact bounds and inherited ceilings | A complete per-vendor memory isolation/limit guarantee; accounted bytes are not the whole process heap | Exceed a known admitted limit, inspect the typed failure, preserve a sibling operation |
| Storage | Host-owned repository ports, narrowed read-only query access, explicit state export/redaction | Per-vendor key/namespace authority and quotas across every storage path; a local-storage binding alone is not isolation | Read allowed catalog data with mutation capability absent; verify private fields never enter the export |
| CPU | Fixed native CPU/I/O executors, ancestor permits, stricter child caps, cancellation/deadlines and bounded workers | General preemptive limits for arbitrary in-process code; cooperative engine control is a different guarantee | Constrain/cancel one workload while an independent sibling completes; inspect deterministic output |
| DOM | Inert state islands, owned render ranges and nested component ownership; workers receive no live DOM handles | Hostile same-page-script confinement and a complete policy for active markup and event effects | Update one component while preserving sibling state/focus; inspect capabilities crossing the worker boundary |
| CSS | Native `@scope`, declaration/shared/instance ownership, public styling hooks, limits and diagnostics | A security boundary covering all style effects, inheritance and resource-bearing constructs | Render in two host themes, demonstrate public hooks and a rejected unsupported construct |
| Network | Host resolvers and policy decisions before access, explicit schemes and retained policy identity | Comprehensive per-vendor egress, credentials, redirects and all alternate request paths | Deny one configured request before resolver I/O; allow a separate permitted request |

Sources:
[scheduler policy](../../packages/cem_ml/src/scheduler/policy.rs),
[hierarchical operation control](../../packages/cem_ml/src/operation_control.rs),
[native executor](../../packages/cem_ml/src/scheduler/executor.rs),
[scope-cap tests](../../packages/cem_ml/tests/scheduler_runtime.rs),
[declaration scopes](../../packages/cem-elements/src/lib/declaration-scope.ts),
[repository ports](../../packages/cem-elements/README.md),
[lifecycle/state export](../cem-element-lifecycle-principle.md),
[CSS ownership contract](../cem-ml-uid-and-scoped-css-design.md),
and [resolver policy](../../packages/cem_ml/src/resolver.rs).

Two implementation details materially limit the public security claim:

- The [plugin boundary](../../packages/cem_ml/src/plugin.rs) states that plugins
  run in-process with host privileges. The
  [plugin runtime](../../packages/cem_ml/src/plugin/runtime.rs) checks elapsed
  invocation time after the call returns. This is not a sandbox or preemptive
  interruption of arbitrary native code.
- Resolver policy defaults to passthrough in the current implementation.
  Demonstrate a specifically configured restrictive host policy; do not claim
  every CEM network request is denied by default.

### Nested scopes and inclusions

The following is a **proposed application security contract for the pitch**,
not a declaration that a unified vendor sandbox is implemented:

```text
effective child authority
    = authority the parent may delegate
      ∩ resources/actions the child requests
      ∩ host policy for that inclusion
```

Each included scope needs a vendor/package identity and a versioned contract.
An include, import, transformation, generated view or nested component must
not silently expand authority. Aggregate memory/CPU consumption must remain
within ancestor ceilings even when every child individually fits its limit.
Cross-scope data is an explicit grant; sibling visibility is not implied.

Revocation/disposal must stop later work from committing effects through old
handles. Cache reuse must respect both content identity and the policy under
which it was admitted. Server-to-browser continuation must recreate permitted
host capabilities instead of serializing credentials or privileged handles.

These rules need one observable allowed case and one denied case per channel,
including indirect effects through callbacks, plugins and active output.
The host/runtime remains trusted infrastructure. A data boundary cannot
contain arbitrary code that can bypass it through ambient host privileges.

For consumers, successful enforcement would limit data exposure, unwanted
effects and disruptions from neighboring integrations. For product teams, it
would make granting, reviewing and revoking authority more explicit. Quantify
those benefits only after the corresponding enforcement is tested.

Browser context:
[HTML iframe sandbox](https://html.spec.whatwg.org/multipage/iframe-embed-object.html#attr-iframe-sandbox),
[Permissions Policy](https://www.w3.org/TR/permissions-policy/),
and [WebAssembly security](https://webassembly.org/docs/security/).
These supply useful mechanisms and constraints, not evidence that CEM has
already assembled complete six-layer isolation.

## Schema package catalog

**Count: 32 package manifests** under
[packages/cem_ml/schema-packages](../../packages/cem_ml/schema-packages).
This includes internal contracts and three module-map revisions. It is a
catalog of schema packages, not 32 unrelated fully implemented languages.

Every link below points to the checked-in `v1/package.cem` manifest.

| Purpose | Package identities |
| --- | --- |
| Data and markup — 9 | [csv](../../packages/cem_ml/schema-packages/csv/v1/package.cem), [html](../../packages/cem_ml/schema-packages/html/v1/package.cem), [json](../../packages/cem_ml/schema-packages/json/v1/package.cem), [markdown](../../packages/cem_ml/schema-packages/markdown/v1/package.cem), [mathml](../../packages/cem_ml/schema-packages/mathml/v1/package.cem), [svg](../../packages/cem_ml/schema-packages/svg/v1/package.cem), [xhtml](../../packages/cem_ml/schema-packages/xhtml/v1/package.cem), [xml](../../packages/cem_ml/schema-packages/xml/v1/package.cem), [yaml](../../packages/cem_ml/schema-packages/yaml/v1/package.cem) |
| Query, style and transformation languages — 5 | [css](../../packages/cem_ml/schema-packages/css/v1/package.cem), [css-selector](../../packages/cem_ml/schema-packages/css-selector/v1/package.cem), [scss](../../packages/cem_ml/schema-packages/scss/v1/package.cem), [xpath](../../packages/cem_ml/schema-packages/xpath/v1/package.cem), [xslt](../../packages/cem_ml/schema-packages/xslt/v1/package.cem) |
| External schema resources — 2 | [json-schema](../../packages/cem_ml/schema-packages/json-schema/v1/package.cem), [relax-ng](../../packages/cem_ml/schema-packages/relax-ng/v1/package.cem) |
| CEM authoring and schema languages — 7 | [cem-ml](../../packages/cem_ml/schema-packages/cem-ml/v1/package.cem), [cem-ql](../../packages/cem_ml/schema-packages/cem-ql/v1/package.cem), [cem-native-template](../../packages/cem_ml/schema-packages/cem-native-template/v1/package.cem), [cem-element-template](../../packages/cem_ml/schema-packages/cem-element-template/v1/package.cem), [cem-transform](../../packages/cem_ml/schema-packages/cem-transform/v1/package.cem), [schema](../../packages/cem_ml/schema-packages/schema/v1/package.cem), [schema-package](../../packages/cem_ml/schema-packages/schema-package/v1/package.cem) |
| Platform contracts — 6 | [cem-data-island](../../packages/cem_ml/schema-packages/cem-data-island/v1/package.cem), [cli-command](../../packages/cem_ml/schema-packages/cli-command/v1/package.cem), [module-map](../../packages/cem_ml/schema-packages/module-map/v1/package.cem), [module-map-v2](../../packages/cem_ml/schema-packages/module-map-v2/v1/package.cem), [module-map-v3](../../packages/cem_ml/schema-packages/module-map-v3/v1/package.cem), [studio-project](../../packages/cem_ml/schema-packages/studio-project/v1/package.cem) |
| Structural projections — 3 | [cem-ast-projection](../../packages/cem_ml/schema-packages/cem-ast-projection/v1/package.cem), [cem-dom-projection](../../packages/cem_ml/schema-packages/cem-dom-projection/v1/package.cem), [cem-events-projection](../../packages/cem_ml/schema-packages/cem-events-projection/v1/package.cem) |

Capability depth must accompany the catalog when it becomes interactive:

| Capability | Evidence and limit |
| --- | --- |
| Common data import/query | [Import principle](../cem-data-import-principle.md): XML/JSON/YAML/CSV enter retained typed trees. CSV fields remain strings; YAML query import has a bounded profile excluding aliases, anchors and explicit tags. Shared evaluation does not imply identical paths for different vocabularies. |
| Schema reuse, extension, and creation | [Package guide](../../packages/cem_ml/schema-packages/README.md) and [custom behavior example](../../packages/cem_ml/schema-packages/schema/v1/examples/custom-behavior-schema.cem): pick existing schemas, extend their rules, or create your own with declarative content/field contracts, constraints and diagnostics. A new lexical syntax still needs a parser/import adapter. |
| XPath / CEM-QL / CSS selectors | [Native query parity tests](../../packages/cem_ml_cli/tests/query_cli.rs) exercise these languages against common inputs and source provenance. Use a supported equivalent query, not a claim that their semantics are interchangeable. |
| XSLT | [Current runtime profile](../xslt-runtime-lowering.md) and [CEM-QL README](../../packages/cem_ql/README.md): bounded XSLT 3.0 compilation/execution, including named/matched templates, parameters, modes, imports/includes, grouping and sorting. Full conformance is not claimed. |
| SCSS | [Package README](../../packages/cem_ml/schema-packages/scss/v1/README.md): focused native parsing, expansion, policy and typed CSS handoff; full Sass parity remains incomplete. |
| JSON Schema | [Package README](../../packages/cem_ml/schema-packages/json-schema/v1/README.md): source/dialect checks; not full instance validation. |
| RELAX NG | [Package README](../../packages/cem_ml/schema-packages/relax-ng/v1/README.md): typed source handling and checks; not a complete grammar compiler/instance validator. |

Some catalog overview paragraphs and the XSLT package README lag newer
implementation records. Prefer the current owning runtime contract, package
README and focused tests when they conflict with older roadmap prose.

## Demonstration evidence and gaps

| Proposed proof | Existing anchor | Work still required |
| --- | --- | --- |
| XML beside CEM-ML | [Reverse conversion tests](../../packages/cem_ml/tests/reverse_conversion.rs) | Author one shared catalog, run the project's syntax-converter helper, native tabular formatting, and semantic comparison |
| XPath beside CEM-QL, optional CSS selector | [CLI query tests](../../packages/cem_ml_cli/tests/query_cli.rs) | Select supported equivalent expressions; compare result identity, values and source locations |
| CSV to YAML | [Conversion implementation/tests](../../packages/cem_ml/src/real.rs) | Freeze exact command/output for the example; label it conversion |
| XML catalog + CSV stock join → YAML and view | [Data import tests](../../packages/cem_ql/tests/data_import.rs), [template rendering tests](../../packages/cem_ql/tests/template_render.rs), [multiple retained bindings](../../packages/cem_ql/src/api/wasm/xslt_bundle.rs) | Complete composition has not been established. Define ID reconciliation, numeric casts, missing stock and duplicate-ID policy, then verify all output paths |
| Swap CSV for JSON | [Shared import contract](../cem-data-import-principle.md) | Explicitly map both to the agreed domain vocabulary before claiming unchanged downstream behavior |
| Add a validation rule | [Custom schema](../../packages/cem_ml/schema-packages/schema/v1/examples/custom-behavior-schema.cem), [strict variant](../../packages/cem_ml/schema-packages/schema/v1/examples/custom-behavior-schema-strict.cem) | Pin the actual schema/diagnostic and demonstrate failure and repair |
| Standalone / embedded view with host themes | [Component pattern](../component-development-pattern.md), [CSS contract](../cem-ml-uid-and-scoped-css-design.md) | Build the chosen availability component using production declarations and public styling hooks |
| Enforced denied action | [Resolver tests](../../packages/cem_ml/src/resolver.rs), [scope policy tests](../../packages/cem_ml/tests/scheduler_runtime.rs) | Select a current control and show denial with the correct threat-model label |

These are specifications, not newly created fixtures. Add actionable fixture
items to [docs/todo.md](../todo.md) before implementation. Use the repository's
syntax-converter helper when producing actual CEM-ML/CEM-QL comparison examples.

## Processing, delivery and performance

| Attraction | Implemented foundation | Boundary / measurement needed |
| --- | --- | --- |
| Native parallel execution | [Scheduler](../../packages/cem_ml/src/scheduler/executor.rs) has bounded CPU/I/O workers, dependencies and deterministic commit; validation can suspend for schema resources | Measure actual overlap and outputs for the selected operation; do not infer every stage parallelizes |
| Browser/Node WASM workers | [Universal CLI](../../packages/cem-ml-cli-npm/README.md) exposes bounded pools and isolated runtime instances | Browser command service deliberately uses one dedicated worker. A pool API does not mean Studio runs each graph stage in parallel |
| Projection streaming | [Projection code](../../packages/cem_ml/src/projection.rs) shares sealed chunks and supports ordered native parallel routes | Its WASM routing is sequential; [source pulling](../../packages/cem_ml/src/source.rs) is currently synchronous |
| Progressive document consumption | Native events/artifact lifecycle provides a foundation | [Loader plan](../cem-data-loader-plan.md) defers progressive browser AST consumption. Byte streaming does not prove progressive validation/query/render; sorting and some queries need materialization |
| Typed transformation graph | [Studio](../../packages/cem-studio/README.md) uses it as final production assembler; explicit module maps cover JS/CSS/worker/WASM assets | [Module-map contract](../cem-ml-cli-contract.md) excludes inferred npm transitive graphs and automatic bundling. Tree shaking, code splitting and broader bundler parity need separate evidence |
| Component SSR and resume | [Recorded edge/SSR gate](../cem-elements-edge-ssr-gate.md), [normative lifecycle](../cem-element-lifecycle-principle.md) establish state identity, output adoption and fail-closed cases | Recorded evidence is dated; rerun the selected example. [Whole-page CLI SSR](../cem-ml-cli-ssr-proposal.md) is still proposed |
| Demand loading | Inert declarations, fallback content before upgrade, external declaration loading and lazy worker initialization | No blanket automatic viewport hydration or arbitrary segment unloading/reloading claim |
| Linux native, Node WASM, browser WASM | [Deployment contract](../cem-ml-deployment-contract.md), [native CLI](../../packages/cem_ml_cli/README.md), [universal hosts](../../packages/cem-ml-cli-npm/README.md) | Record exact public artifacts, installation and runtime capabilities. Source support does not establish a published release |

Performance hypotheses connect to customer outcomes as follows:

- Retaining typed owners can remove intermediate parse/serialize work, reducing
  CPU, allocations and opportunities for inconsistent interpretation.
- Independent workers can keep interaction responsive and shorten suitable
  workloads; startup, copying and scheduling can outweigh gains on small ones.
- Progressive stages can reduce time to first result and peak working set only
  when downstream consumers support progressive operation.
- Compatible SSR output adoption can avoid an initial rerender; it still has
  state-transfer, verification and runtime startup costs.
- A graph can make dependencies and reuse explicit. Correct cache reuse needs
  content, version and policy identity; an unsafe cache can erase security gains.

Benchmark equivalent input, output and validation requirements. Report cold and
warm runs separately, runtime/host versions, input size, time to first useful
result, total duration, peak memory, bytes transferred and cancellation latency.
A smaller output or reduced correctness check is not a like-for-like speedup.

For consumers, the desired effects are less waiting, responsive interaction,
preserved state and fewer unnecessary transfers. For product teams, they are
lower repeated processing, predictable resource use and clearer capacity
planning. No comparative performance or monetary saving was established here.

## Publication rules for this pitch

Use a three-part statement when a feature is discussed: **customer outcome,
observable mechanism, current scope**. Keep the bold vision in the manifesto;
use a visible current/partial/planned label where an example requires it.

Claims to avoid until their evidence exists:

- 32 fully implemented external languages or schema-only parsing of arbitrary syntax.
- Complete XSLT/XPath/Sass conformance inferred from a registered schema package.
- Automatic semantic equivalence or lossless interchange between arbitrary formats.
- Six-layer hostile-vendor isolation inferred from declarative scopes, CSS or WASM.
- End-to-end streaming and multithreading for every operation on every host.
- Bundler parity inferred from the existence of a transformation graph.
- Universal whole-page SSR or automatic demand hydration inferred from component resume.
- Measured savings, universal speedups, or a tenfold AI-driven app-market increase.

The competitive sources are linked in the
[planning comparison](presentation-plan.md#comparison-preparation).
The [Evolutionary Architecture précis](https://evolutionaryarchitecture.com/precis.html)
supports the controlled-change framing. GitHub's
[2025 Octoverse report](https://github.blog/news-insights/octoverse/octoverse-a-new-developer-joins-github-every-second-as-ai-leads-typescript-to-1/)
measures software-development activity, including AI-related repositories;
that is not a measurement of the number of available applications.

The embedding section's adoption argument is a market thesis informed by that
growth, not a measured rule that every product must be embedded to survive.
[OWASP's third-party JavaScript guidance](https://cheatsheetseries.owasp.org/cheatsheets/Third_Party_Javascript_Management_Cheat_Sheet.html)
documents host-page privileges, data exposure, vendor agreements, and operational
coordination. This supports the stated risk of code sharing the host's context.
The [HTML sandbox contract](https://html.spec.whatwg.org/multipage/iframe-embed-object.html#attr-iframe-sandbox)
already provides restrictions for framed content; embedding does not inherently
require exposing the whole host. Lower integration cost and faster adoption are
the intended benefits of runtime insulation, not measured CEM outcomes.
