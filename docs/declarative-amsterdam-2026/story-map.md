# Manifesto and platform story map

The presentation follows one document into a useful result. Each principle and
pitch section below has an existing native slide owner, a website section, an
audience message and an evidence boundary. This finishes the narrative mapping;
it does not establish public hosting, a release, a benchmark or a timed rehearsal.

## Delivery surfaces

- The native deck keeps its 30-slide order, 26 timed slides and four optional references.
  Existing slide content and current operational notes already carry the mapped claims.
- The Site guide is `/guides/declarative-amsterdam-2026/`, authored in
  `apps/cem-site/content/guides/declarative-amsterdam-2026.md` and published by the
  existing native Markdown → HTML → DOM → shared CEMT page graph. No app UI JavaScript is added.
- The [checked map](story-map.json) covers all 14 [manifesto](cem-manifesto.md)
  principles and seven [pitch](cem-platform-pitch.md) sections.
- The [presenter notes](presenter-notes.md) remain the exact-action authority.
  Frozen Login/Supplier outputs and privacy boundaries remain unchanged.

## Six public blocks

| Minutes | Audience question / website section | Existing native slides |
| --- | --- | --- |
| 0–7 | Start from a useful result | 01–05 |
| 7–17 | Keep your expertise and make one edit | 06–09 |
| 17–35 | Trust a check you can inspect and repair | 10–15 |
| 35–47 | Follow processing into a view | 16–20 |
| 47–54 | Choose a host and name the boundaries | 21–26 (25 optional) |
| 54–60 | Bring one useful integration | 30; 27–29 optional only |

The last six minutes remain questions and a bounded adoption discussion. Optional
scope, performance, catalog and Supplier variants add no scheduled time. The
source-pair edit, required schema failure/repair, inspection/formatting, semantic
comparison, query/expression, graph/provenance and light-DOM result remain the
required Login spine. Installation and browser warm-up happen before the clock.

## Principle and pitch coverage

`Demonstrated` refers to selected frozen/native or deployed Login/Supplier evidence.
`Bounded` keeps a supported mechanism or adoption option within its recorded limits.
`Direction` is an architectural or strategic goal; a diagram is not runtime proof.

| Source section | Audience message | Stable native slide IDs | Website section | Scope / evidence |
| --- | --- | --- | --- | --- |
| [Manifesto: Carry definitions through the whole development lifecycle](cem-manifesto.md#carry-definitions-through-the-whole-development-lifecycle) | Keep one definition useful from editing through operation | `p4`, `p8`, `p9`, `p10` | Start from a useful result | demonstrated; [login-presenter/README.md](login-presenter/README.md) |
| [Manifesto: Make processing reusable across delivery channels](cem-manifesto.md#make-processing-reusable-across-delivery-channels) | Use shared rules for more than one useful output | `p14`, `p16`, `p20` | Trust a check you can inspect and repair | bounded; [supplier-presenter/README.md](supplier-presenter/README.md) |
| [Manifesto: Meet people where their work happens](cem-manifesto.md#meet-people-where-their-work-happens) | Put a capability inside the workflow that needs it | `p18`, `p19` | Follow processing into a view | bounded; [platform-evidence.md](platform-evidence.md) |
| [Manifesto: Let people keep their language and expertise](cem-manifesto.md#let-people-keep-their-language-and-expertise) | Begin with the document knowledge people already have | `p5`, `p6`, `p13`, `p15` | Keep your expertise and make one edit | demonstrated; [login-presenter/README.md](login-presenter/README.md) |
| [Manifesto: Separate processing from data location](cem-manifesto.md#separate-processing-from-data-location) | Keep representation, location and processing boundaries explicit | `p3`, `p14`, `p21` | Choose a host and name the boundaries | bounded; [platform-evidence.md](platform-evidence.md) |
| [Manifesto: Make contracts reusable](cem-manifesto.md#make-contracts-reusable) | Give schemas, queries, templates and views clear owners | `p8`, `p15`, `p16`, `p20` | Choose a host and name the boundaries | demonstrated; [login-presenter/README.md](login-presenter/README.md) |
| [Manifesto: Make effects explicit](cem-manifesto.md#make-effects-explicit) | Separate a view from the authority to save or act | `p18`, `p19`, `p22` | Follow processing into a view | bounded; [platform-evidence.md](platform-evidence.md) |
| [Manifesto: Make change a normal operation](cem-manifesto.md#make-change-a-normal-operation) | Change one input and check what remained true | `p4`, `p10`, `p13`, `p16`, `p24` | Bring one useful integration | demonstrated; [login-walkthrough.md](login-walkthrough.md) |
| [Manifesto: Insulate every included scope](cem-manifesto.md#insulate-every-included-scope) | Requested authority needs host grants and real enforcement | `h209a9db50e1b8a77_0_0`, `p22`, `p27` | Choose a host and name the boundaries | direction; [platform-evidence.md](platform-evidence.md) |
| [Manifesto: Let state survive movement and rendering](cem-manifesto.md#let-state-survive-movement-and-rendering) | Keep state ownership explicit across execution boundaries | `p23`, `p27` | Choose a host and name the boundaries | bounded; [participant-kit-verification.md](participant-kit-verification.md) |
| [Manifesto: Treat performance as a property of the whole lifecycle](cem-manifesto.md#treat-performance-as-a-property-of-the-whole-lifecycle) | Measure the whole path to a useful result | `p23` | Choose a host and name the boundaries | bounded; [platform-evidence.md](platform-evidence.md) |
| [Manifesto: Keep explanations attached to results](cem-manifesto.md#keep-explanations-attached-to-results) | Explain a failure or output through its owning source | `p9`, `p11`, `p17` | Trust a check you can inspect and repair | demonstrated; [login-presenter/README.md](login-presenter/README.md) |
| [Manifesto: Make generated software reviewable](cem-manifesto.md#make-generated-software-reviewable) | Review generated declarations with the same checks | `p19`, `p22` | Choose a host and name the boundaries | direction; [platform-evidence.md](platform-evidence.md) |
| [Manifesto: Earn adoption through a useful first step](cem-manifesto.md#earn-adoption-through-a-useful-first-step) | Earn a broader role through one bounded integration | `p24` | Bring one useful integration | bounded; [presentation-plan.md](presentation-plan.md) |
| [Pitch: Keep your data useful across systems](cem-platform-pitch.md#keep-your-data-useful-across-systems) | Begin with the document knowledge people already have | `p5`, `p6`, `p13`, `p15` | Keep your expertise and make one edit | demonstrated; [login-presenter/README.md](login-presenter/README.md) |
| [Pitch: Defining schemas across every stage: from design and development to ongoing operations.](cem-platform-pitch.md#defining-schemas-across-every-stage:-from-design-and-development-to-ongoing-operations) | Keep one definition useful from editing through operation | `p4`, `p8`, `p9`, `p10` | Start from a useful result | demonstrated; [login-presenter/README.md](login-presenter/README.md) |
| [Pitch: Compose vendors with security insulation](cem-platform-pitch.md#compose-vendors-with-security-insulation) | Requested authority needs host grants and real enforcement | `h209a9db50e1b8a77_0_0`, `p22`, `p27` | Choose a host and name the boundaries | direction; [platform-evidence.md](platform-evidence.md) |
| [Pitch: Embedding applications](cem-platform-pitch.md#embedding-applications) | Put a capability inside the workflow that needs it | `p18`, `p19` | Follow processing into a view | bounded; [platform-evidence.md](platform-evidence.md) |
| [Pitch: Put processing where the work happens](cem-platform-pitch.md#put-processing-where-the-work-happens) | Keep representation, location and processing boundaries explicit | `p3`, `p14`, `p21` | Choose a host and name the boundaries | bounded; [platform-evidence.md](platform-evidence.md) |
| [Pitch: Evolve without repeating the work](cem-platform-pitch.md#evolve-without-repeating-the-work) | Change one input and check what remained true | `p4`, `p10`, `p13`, `p16`, `p24` | Bring one useful integration | demonstrated; [login-walkthrough.md](login-walkthrough.md) |
| [Pitch: Bring one useful integration](cem-platform-pitch.md#bring-one-useful-integration) | Earn a broader role through one bounded integration | `p24` | Bring one useful integration | bounded; [presentation-plan.md](presentation-plan.md) |

## Speaking boundaries

| Source section | Keep explicit |
| --- | --- |
| Carry definitions through the whole development lifecycle | The demonstrated path is the frozen Login journey, not a complete IDE or deployment service. |
| Make processing reusable across delivery channels | Supplier is a companion overview; a service embedding demonstration is not claimed. |
| Meet people where their work happens | The Login view is not authentication, and provider/host integration remains an explicit contract. |
| Let people keep their language and expertise | Compare 21 named values on the selected inputs; universal syntax interchangeability is not promised. |
| Separate processing from data location | Adapters handle representation; host resolver/publication interfaces govern access. No live location-swap proof is scheduled. |
| Make contracts reusable | New lexical syntaxes still require their own parser/import adapter. |
| Make effects explicit | Declarative markup and light DOM do not by themselves grant or confine business actions. |
| Make change a normal operation | Use the saved template edit and named semantic comparison; no untested version-adapter exercise is added. |
| Insulate every included scope | Six-layer hostile-vendor confinement is a target with partial controls; the diagram is not an enforcement test. |
| Let state survive movement and rendering | Offline drafts/copies are verified; this talk does not demonstrate arbitrary SSR/resume or automatic whole-Studio recovery. |
| Treat performance as a property of the whole lifecycle | Mechanisms are described without benchmark numbers, automatic speedup, full streaming or bundler-parity claims. |
| Keep explanations attached to results | Printed Inspect coordinates are not interactive links; graph provenance identifies authored rules, not universal data lineage. |
| Make generated software reviewable | AI adoption is a thesis; generated declarations are not safe by construction and no AI demonstration is added. |
| Earn adoption through a useful first step | Define pilot measures; do not promise savings, a public download, or clean installation evidence. |

The website repeats these limits near the corresponding benefit: supported
language profiles and 21 compared fields; saved-input execution; the difference
between Inspect coordinates and actual graph provenance; a view rather than
authentication; partial scope enforcement; state portability and credential
refusal; workload-dependent performance; and no public kit download promise.

## Audience and adoption checks

XML/XSLT developers get familiar source pairs, supported XPath/CEM-QL questions,
typed structure and explicit conversion limits. Web developers get a visible edit,
ordinary light DOM and provider/host ownership. Platform/integration owners get
the same rules across sources/outputs, host resource interfaces and an explicit
separation of the six-layer target from current enforcement.

The closing action is one bounded pilot: name an input, expected output and a
meaningful change; measure correctness, integration effort and resource behavior.
AI-assisted generation and partner embedding explain the opportunity, without
invented app-growth figures, automatic safety or promised economic savings.

## Verification and publication boundary

The mapping gate rejects missing source sections, stale slide owners, altered
public intervals, missing evidence/website sections, promoted insulation claims
and invented public hosting. Native Site publication and its route allowlist
remain authoritative; the build output is local until separately hosted. The
built-route check validates headings, source excerpts, the real HTML owner table,
all three pinned resource links, internal destinations and the absence of script.

No native deck rewrite is needed for this mapping: its existing content and
reviewed operational notes cover the listed sections. Original slides, notes,
images and native animation are preserved. Full-session rehearsal and current
public installation/hosting verification remain the next delivery work.

## Verification record — 2026-11-01

The native Site build passed using existing upstream artifacts. The guide declares
GFM explicitly so its Markdown platform-role table uses the shared native table
converter; the Site's CommonMark routes are unchanged. The focused
built-route browser check verifies six headings, both source excerpts, an actual
HTML platform-role table, pinned participant links, internal destinations and
credential boundaries. Forty resource/notes/palette/story tests and Studio lint
pass. Run the Site build before `@epa-wg/cem-studio:verify:presentation-story`.
The nine opening-animation tests remain a separate unchanged gate.

The whole-site verifier currently stops at `/guides/cem-ml/cli/`: that unchanged
route declares `relativeLinkPolicy: none` although its package README now contains
repository-relative links. The new story route uses the existing site-owned
link policy and passes its focused checks. Whole-site publication is not claimed
until the CLI route's canonical-source policy and subsequent gates pass. This
follow-up belongs to release/hosting acceptance, not an additional on-stage proof.
