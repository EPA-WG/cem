# Declarative Amsterdam 2026: presentation design and adoption plan

Internal preparation plan, revised 2026-10-10. This document organizes the
audience, evidence, demonstrations, timing, and adoption decision for the CEM
presentation. It is separate from the message delivered to prospective users.

## Document ownership and session constraints

| Document | Authority |
| --- | --- |
| [CEM platform pitch](cem-platform-pitch.md) | Audience-facing positioning, platform goals, and adoption message |
| [CEM manifesto](cem-manifesto.md) | Principles behind the platform's direction |
| [Platform evidence](platform-evidence.md) | Capability facts, implementation boundaries, sources, and proof gaps |
| This plan | Internal narrative design, demonstration coverage, and preparation dependencies |
| [Earlier presentation brief](presentation.md) | Historical tutorial proposal, conference facts, correspondence status, and biography |
| [Active work in docs/todo.md](../todo.md#current-work-declarative-amsterdam-2026-cem-presentation) | Sole active execution checklist, including fixtures and verification work |

Use the public pitch for introduction and spoken copy; do not maintain another
long introduction here. This plan supersedes the historical syntax-led journey
as the presentation design. It does not change organizer agreements.

Plan for the **60-minute slot** recorded in the earlier brief, including
questions and recovery time. Preserve beginner accessibility and interaction:
assume no previous CEM-ML experience, explain terms when needed, and make the
presenter-led route sufficient for participants without a working local setup.
The conference record contains unresolved title, format, deadline, and email
questions; resolve those through its sources rather than treating this plan
as new organizer confirmation.

## Audience priorities and adoption outcome

The strongest initial audience owns structured data and must process it for
several consumers, delivery channels, or partner integrations. The conference
entry point should respect XML expertise while making the wider market clear.

| Priority / audience | Familiar starting point | Decision the presentation should enable | Evidence to prioritize |
| --- | --- | --- | --- |
| Primary: XML/XSLT developers | XML, XPath, XSLT, publishing pipelines | Whether CEM extends existing expertise into other formats and delivery paths | XML/CEM-ML and supported XPath/CEM-QL comparisons, followed by useful outputs |
| Primary adjacent: data and automation engineers | Feeds, conversion, batch jobs, validation, CI | Whether one processing definition can replace repeated rules | Multi-source CLI processing with several output branches |
| Service and tooling developers | Backends, import/export, editors, domain services | Whether the engine fits behind an existing product interface | Library invocation returning artifacts and diagnostics |
| Secondary conference, strong online: web UI developers | HTML, components, binding, build tools, SSR | Whether processing and interaction can be reused across hosts | Standalone and embedded views, explicit state and customization |
| Platform and integration engineers | Schemas, APIs, onboarding, access controls, budgets | Whether change and authority boundaries are inspectable | Connector replacement, compatibility checks, and a real enforced denial |
| Product and partner owners | Distribution, delivery cost, customer workflows | Whether a bounded integration deserves investment | Measured repeated work and a capability used inside a partner workflow |

The desired exit action is one bounded pilot: a difficult format boundary,
repeated processing rule, service integration, or capability a partner needs
to embed. A platform replacement decision follows evidence from that pilot.

## Message hierarchy and editorial choices

Open with the market and framework goals from the public pitch. Establish
repeated integration work across data services, pipelines, and applications;
then demonstrate what CEM makes reusable. Embedding supplies a commercial
destination while CLI and library use retain independent value.

| Message level | Attraction point | Presentation treatment |
| --- | --- | --- |
| Lead | Shared rules across formats, locations, and outputs | Familiar source comparison and visible multi-source result |
| Lead | Processing reusable through CLI, library, and browser hosts | One agreed result through specified runtime entry points |
| Lead | Security insulation around independently supplied scopes | Explain all six layers and demonstrate only verified enforcement paths |
| Lead | Embeddable capability and traceable results | Show usefulness inside a workflow, then diagnose a failure |
| Explain | Evolutionary Architecture | One dedicated explanation after a working payoff, plus recurring proof |
| Prove selectively | Connectors, version adapters, state continuity, SSR/resume | Choose a few changes that make preserved contracts visible |
| Support | Native/WASM portability, concurrency, demand loading | Named runtime profiles and measured workloads |
| Catalog | 32 checked-in schema packages and extension mechanisms | Linked capability catalog; demonstrate one custom vocabulary |
| Direction | Complete vendor containment, full progressive processing, bundler parity | Explicitly identified goals with proof requirements |

Use each feature to answer a practical question before naming its mechanism.
Avoid a package inventory or notation tour as the organizing structure.
The AI-era argument is a strategic thesis about connecting more available
software to existing workflows. Do not use an unsupported tenfold app-growth
claim or imply that AI-generated declarations are safe by construction.

### Writing style for the presentation

Use the user's concise style throughout the pitch, slides, website, and spoken
copy. Keep it direct and easy to scan.

- Lead with the benefit or action. Use concrete words and active verbs.
- Keep sentences focused and paragraphs short. Connect each sentence to the
  previous one so the argument develops naturally.
- Cut repetition and abstract lead-ins. Keep details that change the meaning.
- Use lists for parallel choices and tables for comparisons.
- Keep capability limits brief and precise; link supporting evidence.

Preferred example: “Adopt pretty and tabular formatters, color output in HTML
and terminals.” The [pitch](cem-platform-pitch.md) is the style reference.

When describing schema choices, retain all three: “Pick, extend, or create
your own schemas.”

## Proposed 60-minute run

This is a rehearsal allocation, not a finished slide sequence. The exact
examples depend on verified builds and the proof choices below.

| Minutes | Purpose | Main visible proof | Participant involvement |
| --- | --- | --- | --- |
| 0–5 | Establish market, recurring work, and platform goals | Supplier information needed by a feed, service, and ordering workflow | Recognize a similar integration problem |
| 5–12 | Make CEM familiar | XML beside CEM-ML; supported XPath beside equivalent CEM-QL | Predict the selected records |
| 12–22 | Show reusable processing and the development loop | XML + CSV, explicit mapping, YAML/JSON outputs; edit, inspect, repair, rerun a check | Choose or make one business-rule change |
| 22–27 | Explain Evolutionary Architecture | Replace one source boundary while preserving expected output | Identify what changed and what stayed stable |
| 27–37 | Carry processing into delivery | CLI result, library invocation, then an embedded view; show packaging dependencies | Follow the same definition across hosts |
| 37–47 | Make security insulation concrete | Six-layer scope model, configured denied access, surrounding work continues | Predict whether a scoped request should be admitted |
| 47–53 | Show state/performance depth and the next adoption step | One verified SSR/resume or bounded concurrency proof, with measurements | Relate the result to a potential pilot |
| 53–60 | Questions, recovery, and adoption decision | Reproducible example and public entry point when available | Name a useful first integration |

Keep the core path runnable without participant installation. Optional
variants must have independent entry points so they can move to questions or
the later website without breaking the story. If time slips, shorten optional
variants before removing the verified result, the security boundary explanation,
or the pilot decision. Rehearse remote-readable source and result panes.

## Development lifecycle proof journey

The audience should see continuity from source selection to operating and
changing software. Studio is the interactive workbench; CLI and library
interfaces connect the engine to existing editors, hosts, and automation.
The [lifecycle evidence](platform-evidence.md#development-lifecycle) records
the current foundations for this journey.

| Stage | Presenter action / deliverable | Definition or evidence carried forward |
| --- | --- | --- |
| Choose sources and consumers | Identify formats, locations, outputs, and expected results; select or customize connectors | Source/output identities and resource bindings |
| Pick, extend, or create schemas | Pick an existing package, extend its rules, or create a domain schema | Vocabulary, types, validation rules, and diagnostic behavior |
| Import or develop modules | Reuse a transform; edit one query, mapping, or template | Explicit input/output and dependency contracts |
| Compose | Connect sources, stages, branches, outputs, and optional view; identify scopes | Transformation graph and requested host resources |
| Iterate and debug | Edit, run, inspect an intermediate result, locate and repair a failure | Source-linked diagnostics and a reproducible case |
| Verify | Run expected-output and invalid-input checks; add the selected compatibility and denied-effect cases | Executable evidence protecting named contracts |
| Package | Show versioned definitions, dependencies, and required assets | Identified resources and repeatable build inputs |
| Execute | Invoke the CLI, library host, and browser operation under specified profiles | Shared semantics plus explicit host bindings |
| Operate | Inspect failure, progress, output, and relevant resource use | Runtime evidence tied to editable sources |
| Evolve | Change one source, connector, version, module, or host; rerun its checks | Localized change and an explicit compatibility result |

Security and performance participate throughout: identify requirements during
composition, check enforceable boundaries and workloads, bind host policy at
deployment, then inspect behavior during execution. The service host still
owns endpoints, scheduling, authentication, and operational deployment.

## Central demonstration and baseline deliverables

Use a supplier availability capability instead of the historical login form.
Product descriptions arrive as XML and stock as CSV. Explicitly match records
by product ID, convert numeric values under a stated rule, select availability,
and branch to a YAML partner feed, a JSON service payload, and a web view.
Graph joins organize artifacts; the domain query owns the business matching.

```mermaid
flowchart LR
    X["XML product catalog"] --> I["Import and explicit mapping"]
    C["CSV stock"] --> I
    I --> T["Retained typed data"]
    T --> Q["Domain query and transformation"]
    Q --> Y["YAML partner feed"]
    Q --> J["JSON service payload"]
    Q --> V["Availability view"]
    V --> P["Partner ordering portal"]
```

Prepare one source set, mapping, domain rule, expected-result set, and failure
case as the reference across the selected hosts. The composed example remains
proposed until its fixtures, commands, and outputs are verified; individual
repository operations do not establish the whole demonstration.

| Entry point | Baseline demonstration | Host responsibility and boundary |
| --- | --- | --- |
| Linux native CLI | One automated run produces multiple file artifacts and diagnostics | Bind permitted input/output paths and capture the reproducible command |
| Library in a small service or tool | Invoke supported processing, inspect returned artifacts, return the explicit JSON output | Own transport, identity, scheduling, error handling, and publication |
| Node/WASM | Run the selected common engine operation | Declare supported I/O and runtime requirements |
| Browser/WASM | Inspect the result and compose the availability view into a partner portal | Supply task context, resources, theme, and allowed actions |

Use the [CLI/library evidence](platform-evidence.md#cli-library-and-pipeline-applications)
to choose actual entry points. Record the precise engine/runtime versions and
supported profiles. Do not imply that every native operation or connector is
available identically in every host.

## Familiar comparisons and schema breadth

Keep XML and CEM-ML side by side with corresponding names, attributes, values,
and source positions highlighted. Pair supported XPath and CEM-QL expressions
with an equivalent question and comparable output. Explain language differences
where the examples need them; do not imply complete XPath/XSLT conformance.
Existing (X)HTML and bounded XSLT support are bridges to familiar work.
An optional CSS-selector comparison can provide a web-developer entry point
when it asks the same supported question; identify differences in selection
semantics instead of implying language interchangeability.

CSV-to-YAML alone proves conversion. XML + CSV computation with an explicit
join and numeric rule proves more of the neutrality argument. Preserve format
distinctions: types, namespaces, ordering, lexical details, and mixed content
need explicit mappings and loss reporting when relevant.

The [schema catalog](platform-evidence.md#schema-package-catalog) lists all
32 checked-in packages by purpose. Use it as evidence of breadth, with parsing,
validation, query, formatting, and rendering support identified independently.
Show the choice to pick, extend, or create a schema. Demonstrate an extension
with one added vocabulary term or constraint and its diagnostic.
An arbitrary new lexical syntax also requires a parser/import adapter; a schema
definition alone does not generate universal parser support.

For the diagnosis proof, select an incorrect availability result and trace it
through the transformation to its stock cell or product node. Also inspect a
dependency behind a generated asset. Verify each traced path before showing it;
the point is an explainable failure, not a promise of universal provenance.

## Variant and proof coverage

Keep these variants as a coverage map. Select the live subset during rehearsal;
preserve the others as independent examples for later material. None is a
completion claim until its composed path is verified.

| Variant | Visible change | Contract / proof | Place in the narrative |
| --- | --- | --- | --- |
| Source format | Change stock CSV to JSON | Same agreed domain output; format mapping changes explicitly | Main evolution proof candidate |
| Schema extension | Add a product constraint and violate it | Useful source-linked rejection, then successful repair | Main development loop |
| Resource location | File → permitted HTTPS → host-supplied memory | Same query/result, updated provenance, retained denial of ungranted locations | Connector proof candidate |
| Publication | Return the result through another supported publication path | Named output semantics and host access policy preserved | Connector extension |
| Version adapter | Supplier v2 field/structure → explicit v1-compatible transformation | Version range and meaning-preserving limits declared; consumer checks still pass | Evolution extension |
| Processing host | CLI → library → selected WASM host | Agreed outputs and diagnostics for the supported profile | Main delivery proof |
| Embedding | Standalone view → partner-hosted view | Same capability receives host context and keeps behavior coherent | Main application payoff |
| Host styling | Same view in two themes | Provider-declared tokens/hooks work; incompatible customization is reported | Web-developer extension |
| Presentation context | Compact indicator → detailed panel → dedicated page | Same domain result with deliberate task-specific presentation | Proposed UI variants, not a universal mode API |
| Multiple instances | Two suppliers or product selections on one page | Independent instance context and selection state | Ownership proof |
| Explicit effects | Read-only result → permitted reservation/persistence service | Grant or withhold host service; no ambient authority inferred | Application/security bridge |
| Denied resource | Request an ungranted location or publication | Denial before the identified I/O path; separate permitted work completes | Main security proof candidate |
| Cross-tier state | Server output → browser continuation → state update | Compatible state/revision/declaration identity; stale revision handled | Lifecycle depth candidate |
| Resource pressure | Constrain or cancel one admitted workload | Measured limit/cancellation behavior and independent sibling result | Performance/security candidate |

Format adapters handle representation. **Extensible, customizable connectors**
handle access and delivery through host resolver/publication interfaces.
Demonstrate these as separate extension boundaries, including a connector for
a team's own source or destination where appropriate. Host-managed storage
does not imply a built-in connector for every database or vendor service.

Provider ownership and independent release schedules matter. The host supplies
the selected product, locale, theme, permitted actions, and admission policy.
Start with a read-only result; add a host service only through a declared effect
boundary. Compatibility failures may require a version adapter, consumer work,
or rejection. They are not automatically repaired by a common runtime.

## Scope security: explain the mechanism and prove the boundary

Use the DCE scope/inclusion model as the architectural explanation: each
independently supplied application, pipeline stage, or connector is surrounded
by applicable insulation layers. Declarations request access, hosts determine
grants, and runtime mechanisms must enforce the boundary during execution.
Memory, Storage, CPU, and Network apply to processing; browser interfaces add
DOM and CSS. Contracts describe and configure insulation; they do not replace it.

The [six-layer evidence matrix](platform-evidence.md#six-resource-layers) is
the capability authority. The table below selects proof candidates from it;
these candidates are not yet verified presentation demonstrations.

| Layer | Repository-backed control to evaluate | Bounded proof candidate | Remaining full-insulation proof |
| --- | --- | --- | --- |
| Memory | Accounted allocations, input/artifact bounds, inherited ceilings | Exceed a known limit and inspect its failure while sibling work completes | Private state/reference isolation and complete per-vendor memory accounting |
| Storage | Host repository ports, narrowed read-only access, export/redaction | Permit an assigned read, withhold mutation, exclude private export fields | Per-vendor namespace/operation authority and quotas on every path |
| CPU | Fixed executors, ancestor permits, child caps, cancellation/deadlines, bounded workers | Constrain/cancel a workload; show deterministic independent output | Preemptive limits for arbitrary code and complete descendant consumption control |
| Network | Host resolvers and pre-access policy decisions | Deny one configured request before resolver I/O; allow another | All egress paths, credentials, redirects, alternate APIs, and revocation |
| DOM, browser | Inert state islands, owned ranges, nested ownership; no live DOM in workers | Update one instance while preserving sibling state/focus | Hostile same-page script and active-markup confinement |
| CSS, browser | Native scoped rules, ownership, hooks, limits, diagnostics | Two host themes and a rejected unsupported construct | All style effects, inheritance, and resource-bearing constructs |

Show a scope/inclusion diagram with a supplier receiving product and stock
data, a shipping contribution receiving only region and parcel dimensions,
and the view receiving results. Annotate requested and granted resources.
The full multi-vendor composition is a proposed proof; identify exactly which
part of the diagram each live denial establishes.

Nested scopes must retain ancestor safeguards, narrow delegated authority,
and respect aggregate ceilings. Include sibling behavior and descendant work
in relevant negative cases. Revocation, generated output becoming active,
callbacks, and privileged connectors/extensions are separate enforcement paths
to establish. CEM's light DOM and scoped CSS alone do not confine hostile code.

Use three explicit evidence labels during preparation and delivery:
**demonstrated in this build**, **implemented but not covered by this example**,
and **architectural goal**. The independent-vendor benefit stays central while
the proof remains specific about engine, host, profile, and trust assumptions.

## Evolutionary Architecture: one explanation, recurring evidence

Reserve the dedicated 22–27-minute explanation after the first useful result.
In the eventual deck it merits its own slide. Connect it to the
[authors' account of incremental change and fitness functions](https://evolutionaryarchitecture.com/precis.html).
Repeat three questions with each significant change: what changes, what remains
stable, and which executable check proves it?

| Change dimension | Preserved property | Evidence to prepare |
| --- | --- | --- |
| Syntax or location | Agreed meaning and correct source attribution | Mapping-equivalence cases; resource-binding swap; forbidden source still denied |
| Schema or version | Declared compatibility and rejection behavior | Pinned valid/invalid cases and the explicit v2-to-v1 adapter |
| Query or transform | Named output semantics | Expected-result comparison and relevant transformation lineage |
| Runtime | Operation semantics within advertised host capability | Same input across selected native/Node/browser paths |
| Render/deployment tier | State, revision, declaration identity | Resume/update plus stale-revision behavior |
| Vendor or connector | Public interface and granted authority | Compatibility and denied-action cases for its actual enforcement path |

Tie this to cost of change, security, and performance. A faster implementation
must preserve allowed behavior; a new boundary must still meet the selected
latency and resource requirements. Extensibility alone is insufficient proof
of guided evolution.

## Measurement and transformation-graph coverage

Choose one workload that demonstrates a useful outcome. Keep input, output,
operation, runtime version, machine/browser, and policy profile explicit.
Measure cold and warm behavior separately and include correctness checks.

| Workload concern | Measure | Interpretation boundary |
| --- | --- | --- |
| Interactive delivery | Time to first useful result, main-thread blocking, transfer | Earlier output is distinct from earlier completion |
| Pipeline/service execution | Completion time, throughput, request latency, cost per completed workload | Compare equivalent workloads and include host overhead |
| Resource consumption | Peak memory, accounted allocations, cancellation latency | Accounted engine bytes are not necessarily the whole process heap |
| Concurrency | Worker count, startup, transfer/serialization, sibling progress | More threads or WASM do not guarantee a faster workload |
| Progressive processing | Stage start/output times and buffering | Identify which loading/validation/transformation/formatting/output stages actually overlap |

Retained structures can avoid conversions while materializing a complete
input. Demonstrate actual bounds and backpressure where supported; end-to-end
progressive consumption and arbitrary streaming queries remain separate proof
requirements. Demand loading and SSR/resume need their own observable cases.

The graph's existing use to assemble Studio assets is a packaging example.
For the goal of modern bundler parity, maintain capability evidence for module
resolution, assets, caching, splitting, tree shaking, source maps, development
updates, and plugins. A graph demonstration does not establish all of these.
Use [processing evidence](platform-evidence.md#processing-delivery-and-performance)
to choose the claims and variants that are ready.

## Commercial adoption and pilot acceptance

Show who benefits: providers reuse a capability across consumers; hosts reduce
reconstruction and onboarding work; users keep task context and avoid repeated
navigation or data entry. Measure these effects rather than presenting a
guaranteed saving.

| Repeated work | Mechanism under evaluation | Pilot measure |
| --- | --- | --- |
| Data interpretation per consumer | Schemas, explicit mappings, shared queries | Hours and mapping defects per new source/consumer |
| Rules repeated in jobs, services, and tools | CLI/library reuse of processing modules | Rule implementations and effort needed for one change |
| Provider interaction rebuilt by each host | Reusable view, behavior, and extension points | Time to a functioning partner integration |
| Lifecycle/state glue | State identity and explicit resume | State-loss defects and host-specific code changed |
| Failure investigation | Source-linked diagnostics and graph provenance | Time to locate the responsible input or rule |
| Resource-access review | Explicit mediation and verified boundaries | Review hours, granted privileges, demonstrated denials |
| Migration across consumers | Versioned contracts and adapters | Unrelated modules touched per update |

Agree on a baseline implementation and requirements before measuring. The
pilot supports broader adoption when it preserves agreed output, localizes
the selected source change, exposes a failure's source, and reduces measured
integration effort. It must also meet the baseline's accessibility, security,
and performance requirements. A result that exposes a missing capability is
useful evidence for deciding scope; it is not a successful replacement claim.

Estimate economic value from measured engineering hours avoided times loaded
cost, plus operating savings, less adoption and ongoing maintenance. Business
authorization, authentication, semantic mapping, accessibility, and operational
ownership remain explicit integration responsibilities.

## Comparison preparation

Choose comparisons relevant to the selected audience question. Acknowledge
existing strengths, then demonstrate the particular CEM combination. These
sources provide context, not implementation requirements for CEM.

| Neighboring solution | Strength to acknowledge | Comparison to prepare |
| --- | --- | --- |
| [Enterprise transformation tools](https://www.ibm.com/support/pages/getting-started-websphere-transformation-extender-design-studio-datapower) | Established transformation/integration, including non-XML and multiple execution modes in relevant products | Changes in format, location, and host using CEM's extension boundaries |
| [Saxon/SaxonJS](https://www.saxonica.com/html/saxonjs/index.html) | Mature XML/JSON, XPath/XSLT, browser/Node and interactive XSLT | Specific supported profiles, lifecycle integration, and application composition |
| [XProc](https://spec.xproc.org/3.0/xproc/) | Declarative pipelines over multiple document types | Connection of retained processing to state, views, and delivery |
| [Invisible XML](https://invisiblexml.org/) | Declarative grammars for bringing other syntax into XML workflows | Schema-package lifecycle and the actual adapter requirements |
| [Module Federation](https://webpack.js.org/concepts/module-federation/) | Independent builds, runtime loading, shared modules | Data, state, and authority contracts alongside delivery |
| [Sandboxed iframes](https://html.spec.whatwg.org/multipage/iframe-embed-object.html#attr-iframe-sandbox) | Browser-enforced restrictions for embedded content | Coordination of application contracts with real enforcement |
| [Vite / modern bundlers](https://vite.dev/guide/features.html) | Production optimization and established development workflows | Verified graph capabilities and the remaining parity goals |

Do not imply complete standards support, hostile-vendor containment, or
drop-in build-system replacement. Enterprise integration precedent also
includes [Intel SOA Expressway's cost/complexity framing](https://www.intel.com/pressroom/archive/releases/2008/20080421comp.htm);
use it to explain a recognizable problem rather than announce a new category.

## Preparation dependencies, fallback, and later website

The public pitch and manifesto establish the message; the evidence record
selects defensible claims; verified examples enable rehearsal; rehearsal then
determines the final slide and website mapping. This document creates no
website, slides, fixture, benchmark, or release by itself.

| Deliverable | Dependency / acceptance evidence |
| --- | --- |
| Reproducible core example | Source fixtures, exact builds, commands/library calls, expected artifacts, and failure cases |
| Familiar comparisons | Verified supported expressions and equivalent highlighted results |
| Security/performance proof | Named host/profile, observed allowed and denied behavior, measured workload, explicit limitations |
| Installation and access notes | Tested Linux/Node paths and working browser access; other platforms only when tested |
| Presenter fallback | Saved outputs and source snapshots or a recorded run matching the demonstrated version |
| Rehearsal record | Timed core path, remote legibility, recovery points, and selected optional variants |
| Later public website | Immediate result, familiar source, one meaningful edit, inspection, and a reproducible download/entry point |

The later site should offer routes for automation, embedding processing in a
service/tool, and composing an application. Each example should stand alone.
Choose page and slide structure only after the message and proofs stabilize.
Preserve the presenter-led path if local installation or connectivity fails.

The [historical conference record](presentation.md#conference-facts-and-correspondence-status)
owns unresolved organizer facts and unavailable correspondence. The
[evidence record](platform-evidence.md#demonstration-evidence-and-gaps) owns
capability gaps. The [EPA/DWA source notes](platform-evidence.md#ideas-from-epa-and-dwa)
explain inspiration retained from those repositories without adopting their
old implementation recipes. Execution and completion tracking remain solely
in [docs/todo.md](../todo.md#current-work-declarative-amsterdam-2026-cem-presentation).
