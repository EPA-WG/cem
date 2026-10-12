# Declarative Amsterdam 2026 — slide story and speaker notes

Source-aligned authoring script, 2026-10-11. Twenty-five main slides occupy
60 minutes; four appendix slides are outside the timed run. The final six
minutes belong to questions and adoption discussion. The title follows the
[published program](https://declarative.amsterdam/program), and the intervals
preserve the [discussion #24](https://github.com/EPA-WG/cem/discussions/24)
timeline. [presentation.md](presentation.md) remains the agenda authority.

Use the slide copy as audience text. Speaker notes explain the demonstration,
participation prompts, and evidence boundaries; do not print them as slide text.
Code assets must use the approved [preview workflow](code-preview.md).
The verified asset manifest and command transcript take precedence over any
proposed example wording below. Do not invent a diagnostic or command result
to fill a demonstration slot.

The native deck's timing is mirrored below; its current readback is
`dist/declarative-amsterdam-2026/slides/raw-after-teaser-final.json`. Current asset evidence
is `dist/declarative-amsterdam-2026/slide-assets/evidence.json`: native validation
demonstrates the teaching schema's required-method error and repair; XPath and
CEM-QL select matching source spans from a namespace-repaired XML export.
The sign-in screenshot is the canonical static HTML example. The complete
template and `<cem-element>` login run and generated-result transformation
trace remain preparation work. The corresponding notes below describe the
intended session checkpoint, not additional executed evidence.

## 01 · CEM — DNA for structured data

**Time:** 00:00–00:45 · 45 seconds

**Slide copy:** A template for today's structure and tomorrow's evolution.
Thought → DNA → CEM.

**Visual:** Editable thought branches, a DNA helix and a template with current
and future variants. Use the canonical CEM light palette.

**Speaker notes:** In the beginning was a thought. It felt lonely, so it
changed—and gave rise to other thoughts. In the biological world, DNA offers
a vivid parallel: a pattern that life can copy and express, while variation
makes evolution possible. CEM is an attempt to create DNA for structured data:
a declarative template for a system's current state and a structure its future
versions can build upon. Tell this as a metaphor, not a scientific chronology.

**Sources:** [Opening idea and design](opening-teaser.md).

## 02 · CEM-ML declarative document lifecycle engine

**Time:** 00:45–01:10 · 25 seconds

**Slide copy**

Evolutionary architecture for the declarative web stack

Sasha Firsov · Declarative Amsterdam · 12 November 2026

**Visual:** Quiet title slide; CEM mark or typographic wordmark, generous space.

**Speaker notes:** “We will follow a small document through a complete working
journey: edit it, check it, inspect it, transform it, and see it in a browser.
The broader question is how the same definitions can stay useful as formats,
partners, and execution environments change.” Keep the published session title;
the architecture line is thematic framing rather than a claimed organizer-approved
rename. Introduce yourself briefly and move straight to the result.

**Sources:** [Presentation brief](presentation.md#presentation-brief),
[official program](https://declarative.amsterdam/program).

## 03 · Change the source. See the result.

**Time:** 01:10–03:00 · 1 minute 50 seconds

**Slide copy**

Read → edit → validate → inspect → transform → render

One small document. A visible result at each step.

**Visual:** Finished login view beside the approved colored CEM-ML source;
highlight the heading in both.

**Speaker notes:** “This is our sign-in view. You do not need previous CEM-ML
experience, and you can follow every step on this screen without installing
anything. We will change its heading, identify an intentional mistake, and
follow the corrected document into its rendered view.” Invite participants to
notice the matching source and visible label. Explain that this is a view, not
an authentication service: identity, credentials, and session handling belong to
the surrounding application. Preserve the source/result pairing during the run.

**Sources:** [Login source](../../examples/cem-ml/login.cem),
[HTML counterpart](../../examples/semantic/login.html),
[exercise design](presentation.md#exercise-design).

## 04 · Make capabilities reusable across systems

**Time:** 03:00–04:30 · 1 minute 30 seconds

**Slide copy**

CEM enables declarative data services, pipelines, and applications.

Neutral to formats. Explicit about locations. Portable across supported tiers.

**Visual:** One processing rule at the center; formats, resource locations, and
execution hosts as three independently replaceable boundaries.

**Speaker notes:** “A business rule often gets rewritten for a batch feed, a
service response, and an interactive view. The useful goal is to keep that rule
while changing how data arrives and where the result is needed. This matters
to XML and publishing teams, to service and automation developers, and to web
teams embedding someone else's capability.” Ask for a quick show of hands:
who maintains the same rule in more than one delivery path? Explain that
neutrality is achieved through explicit adapters and host interfaces; it does
not erase differences between source vocabularies or permissions.

**Sources:** [Platform pitch](cem-platform-pitch.md),
[format and location independence](platform-evidence.md#format-and-location-independence).

## 05 · Keep useful rules as systems change

**Time:** 04:30–07:00 · 2 minutes 30 seconds

**Slide copy**

Choose sources → pick, extend, or create schemas → develop modules

Compose and test → package and run → operate and evolve

Keep the contracts. Check the change.

**Visual:** A six-stage lifecycle with schemas, source identity, expected
outputs, and resource policy carried underneath it. Change one source adapter
without redrawing the whole chain.

**Speaker notes:** “Evolutionary architecture asks what may change and what
must remain true. A source may change format, a vendor may publish a new
version, or processing may move from a job into a service. We want document
meaning, expected results, source attribution, and permitted effects to remain
explicit.” Walk through the complete development lifecycle in one pass. “CEM's
definitions and diagnostics can participate in your existing editor, tests,
CI, and deployment. The payoff is a localized change with a check showing what
was preserved.” Return to these contracts during each demo checkpoint.

**Sources:** [Manifesto: make change a normal operation](cem-manifesto.md#make-change-a-normal-operation),
[development lifecycle evidence](platform-evidence.md#development-lifecycle).

## 06 · Keep your markup knowledge

**Time:** 07:00–09:30 · 2 minutes 30 seconds

**Slide copy**

The same names, attributes, and content — another authoring form.

**Visual:** Verified short HTML/XML and CEM-ML excerpts, side by side, in the
approved coloring. Connect corresponding heading, form, and input names.

**Speaker notes:** “Start with what you already recognize. Here is an element
name, here are its attributes, and here is its text. In the CEM-ML form, braces
delimit the node and the separator introduces content. The document still
describes the same intended view.” Point to matching elements rather than
reading punctuation aloud. Explain only the syntax needed by the next edit.
State which source representation is actually shown: the core alternate input
is the existing HTML counterpart, even when discussing XML-style structure.

**Sources:** [Login pair](presentation.md#exercise-design),
[familiar comparisons](presentation-plan.md#familiar-comparisons-and-schema-breadth).

## 07 · Read the document, not a new framework

**Time:** 09:30–12:00 · 2 minutes 30 seconds

**Slide copy**

Namespace · node · attribute · content

**Visual:** Approved native-tabular login image at readable size; four short
annotations. Keep the generated indentation and attribute ordering intact.

**Speaker notes:** “The namespace tells us whose vocabulary a name belongs to.
The default namespace saves repetition for familiar HTML names, while a prefix
can identify another vocabulary. A schema supplies the rules for interpreting
and validating those names and values.” Show the document declaration and then
the heading, input, and action attribute. Ask participants to locate the text
that produces the visible heading. The native CLI formatted this source; the
existing browser CEM highlighter supplied the approved color presentation.

**Sources:** [Approved preview method](code-preview.md),
[canonical source](../../examples/cem-ml/login.cem).

## 08 · Make one change

**Time:** 12:00–15:00 · 3 minutes

**Slide copy**

Change the heading or a label.

Predict the result. Run it. Compare.

**Visual:** Before/after source region and corresponding view region. Keep the
single edited value prominent.

**Speaker notes:** Give one exact instruction using the frozen tutorial file.
“Choose a label that makes sense to your users. Before running the operation,
point to the part of the view you expect to change.” Allow a short pause for
participants, then run the same edit visibly. Anyone without a local setup can
predict the result or suggest the text. “We have changed an input definition;
the next stages should make its effect observable without hiding where it came
from.” If local setup fails, continue with the presenter's saved checkpoint.

**Sources:** [Participation and fallback](presentation.md#delivery-setup-and-fallback).

## 09 · Pick, extend, or create your own schemas

**Time:** 15:00–17:00 · 2 minutes

**Slide copy**

Pick an existing contract.

Extend its rules.

Create your own vocabulary and types.

Deliver formatters and colorers with your schemas.

**Visual:** Three equal options converging on validation and development tools;
small formatter/colorer attachments.

**Speaker notes:** “These are three different adoption paths. You can begin
with an existing package, add rules for your domain, or define the vocabulary
you need. The schema becomes useful during authoring, validation, diagnosis,
and presentation.” Explain that a custom lexical syntax also needs its parser
or import adapter; schema definition alone does not parse arbitrary languages.
“Next we will deliberately violate a rule while keeping the syntax valid, so
you can see the difference between reading a document and accepting it.”

**Sources:** [Schema package catalog](platform-evidence.md#schema-package-catalog),
[schema package guide](../../packages/cem_ml/schema-packages/README.md).

## 10 · Catch the mistake at its source

**Time:** 17:00–20:00 · 3 minutes

**Slide copy**

Valid syntax does not guarantee a valid document.

A useful diagnostic identifies the rule and the source.

**Visual:** Verified invalid edit and actual diagnostic; align the location
with the offending source span. Show diagnostic text, not color alone.

**Speaker notes:** “The parser can read this document, but the selected schema
does not accept it. Which part of the message tells us why?” Let participants
identify the offending value or structure before explaining the rule. Run the
recorded failing command and show its actual non-success result. Distinguish
this intentional schema failure from incidental unresolved-reference warnings
in the initial preview. Do not substitute a fabricated error message if the
prepared route fails; use its recorded checkpoint and describe the failure.

**Sources:** [Validation proof requirement](presentation.md#exercise-design),
verified tutorial diagnostic transcript supplied during deck generation.

## 11 · Repair the cause. Run the check again.

**Time:** 20:00–23:00 · 3 minutes

**Slide copy**

Locate → repair → validate

Keep the failing case as a check.

**Visual:** Invalid source → one highlighted repair → recorded successful
validation. Use the same source location as the previous slide.

**Speaker notes:** “What is the smallest change that satisfies this rule?”
Take one answer, apply the prepared repair, and run the identical validation
path again. Compare the results rather than simply announcing success. “This
is also a useful development artifact: an accepted input and a rejected input
protect our expectation when the schema or implementation changes.” Connect
the visible repair back to Evolutionary Architecture: a named contract and a
repeatable check make future change assessable.

**Sources:** [Exercise design](presentation.md#exercise-design),
[development lifecycle](platform-evidence.md#development-lifecycle).

## 12 · Inspect the meaning behind the syntax

**Time:** 23:00–26:00 · 3 minutes

**Slide copy**

Inspect structure, typed values, and source locations.

**Visual:** Short native-tabular typed-tree output beside its source. Connect a
node name, attribute value, and retained source span.

**Speaker notes:** “The original spelling is one surface. The retained typed
tree gives processing stages a common representation of structure and value
meaning. Inspection lets us check what was actually imported before blaming
the next stage.” Locate the participant's edit in the structural output and
explain any type or namespace information visible in this particular result.
Keep the structural display in CEM-ML, as the repository requires. Do not
present JSON as the implicit internal tree representation.

**Sources:** [Presentation boundary](../../CLAUDE.md#cem-ml-presentation-boundary),
[shared import principle](../cem-data-import-principle.md).

## 13 · Make structure readable

**Time:** 26:00–29:00 · 3 minutes

**Slide copy**

Adopt pretty and tabular formatters, color output in HTML and terminals.

Save plain output. Inspect a concise report.

**Visual:** The same small structure with approved tabular coloring and a
concise real report. Avoid a wall of command-line flags.

**Speaker notes:** “Presentation should help people inspect the same meaning
in an editor, terminal, review, or web page. A formatter controls layout; a
colorer makes roles easier to distinguish. Neither should silently change the
meaning.” Show the actual formatted source and report captured for the demo.
Explain that the saved text has no terminal escape codes. The approved slide
assets use native CLI tabular formatting followed by the existing CEM browser
HTML colorer; do not claim the native CLI alone produced colored HTML.

**Sources:** [Code preview workflow and evidence](code-preview.md).

## 14 · Change representation. Preserve agreed meaning.

**Time:** 29:00–33:00 · 4 minutes

**Slide copy**

HTML input → shared typed model → explicit output

Compare the fields that matter.

**Visual:** Existing HTML counterpart and the selected converted output,
connected through the typed model. Call out agreed heading, input, and action
fields, using the verified comparison result.

**Speaker notes:** “Conversion changes representation. A domain
transformation changes the information or view we produce. Here we first test
the representation boundary.” Show the recorded CEM-ML-to-XML export and its
XML import path. The current export omits namespace declarations; the fixture
restores the exact bindings from the CEM-ML source without changing nodes or
values. The program's HTML-counterpart comparison remains the intended complete
checkpoint and needs separate verification. Ask participants to compare the
named semantic fields, not whitespace or quote style. “A shared model
lets later stages work with retained meaning, but arbitrary formats are not
automatically interchangeable or lossless.” Return to the same login
checkpoint before the query section.

**Sources:** [Alternate input boundary](presentation.md#exercise-design),
[import principle](../cem-data-import-principle.md).

## 15 · Multiple inputs. Multiple useful outputs.

**Time:** 33:00–35:00 · 2 minutes

**Slide copy**

XML catalog + CSV stock

One domain rule → YAML feed + JSON response + availability view

Companion example

**Visual:** Two input branches, explicit mapping and domain computation, three
output branches. Distinguish the representation adapters from the business rule.

**Speaker notes:** “The login keeps today's exercise small. The same platform
direction also serves data pipelines: combine a product catalog with stock,
then supply partners, services, and a UI from the agreed domain result.” Explain
that product-ID matching and numeric conversion are explicit business logic;
a graph join alone does not reconcile records. This is the companion design,
not a newly verified end-to-end demonstration. Its purpose is to show why the
format boundary matters beyond browser forms.

**Sources:** [Supplier companion](presentation-plan.md#companion-supplier-data-across-inputs-outputs-and-hosts),
[demonstration gaps](platform-evidence.md#demonstration-evidence-and-gaps).

## 16 · Ask a familiar question

**Time:** 35:00–37:00 · 2 minutes

**Slide copy**

XPath beside CEM-QL

One question. Compare the selected result.

**Visual:** Verified equivalent expressions and the common result, using the
same login input. Keep source text large enough for remote viewing.

**Speaker notes:** Read the query as a question about the document before
explaining its notation. “If you know XPath, start with this expression. Here
is the supported CEM-QL expression asking the same question of this input.”
Run both and compare identity, values, and source locations where recorded.
Ask participants to predict which node will be selected after their edit.
These examples show bounded parity, not interchangeable language semantics or
complete XPath conformance.

**Sources:** [Native CLI query tests](../../packages/cem_ml_cli/tests/query_cli.rs),
verified tutorial query transcript supplied during deck generation.

## 17 · Declare the transformation

**Time:** 37:00–39:00 · 2 minutes

**Slide copy**

Select data. Evaluate an expression. Apply a template.

**Visual:** Small verified query/expression/template excerpt beside its
generated view. Highlight the expression responsible for the visible result.

**Speaker notes:** “A query identifies relevant information. An expression
computes a value. A template describes the output structure that uses it.”
Walk through only the prepared supported operations, then apply the participant
edit to make the effect visible. Explain that processing is a reusable module;
the surrounding host decides how to supply resources and publish the result.
If discussing XSLT, describe its defined runtime profile, not full standards
conformance. Use the saved output if the live operation fails.

**Sources:** [CEM-QL](../../packages/cem_ql/README.md),
[XSLT runtime profile](../xslt-runtime-lowering.md), verified tutorial template.

## 18 · Follow the result back to its source

**Time:** 39:00–41:00 · 2 minutes

**Slide copy**

Source → query / template → output

Explain the value. Locate the rule.

**Visual:** A compact transformation graph with actual generated result,
retained source-byte span, and transformation trace linked by one highlighted
path.

**Speaker notes:** “A result is more useful when we can explain it. Which
source contributed this value? Which transformation produced this part of the
view?” Follow one actual captured trace rather than touring all graph nodes.
Explain that a transformation graph also describes dependencies, branches, and
explicit artifacts in a pipeline. Current provenance must be demonstrated for
the selected operation; a graph alone does not prove universal source mapping,
automatic asset optimization, or modern bundler parity.

**Sources:** [Required provenance](presentation.md#published-agenda-and-topic-coverage),
[graph evidence](platform-evidence.md#cli-library-and-pipeline-applications).

## 19 · Render into the document you can inspect

**Time:** 41:00–44:00 · 3 minutes

**Slide copy**

Declarative data, events, queries, and templates

`<cem-element>` → ordinary light DOM

**Visual:** Static login preview, visibly identified as such. The intended
complete demonstration adds a readable browser DOM excerpt and connects the
edited label through the declaration to its rendered element.

**Speaker notes:** “Here is the result in the browser. Light DOM means these
rendered elements are part of the ordinary document tree that browser tools
can inspect.” The current image shows the static HTML counterpart; it does not
prove query/template execution or `<cem-element>` integration. Before the live
session, verify the complete path, identify the bindings it uses, and demonstrate
the edit's visible effect. Distinguish the declaration from generated output.
Light DOM is an integration and inspection choice; it is not itself a security
boundary against arbitrary same-page JavaScript.

**Sources:** [cem-elements](../../packages/cem-elements/README.md),
[declarative UI principle](../declarative-ui-principle.md).

## 20 · Embedding applications

**Time:** 44:00–47:00 · 3 minutes

**Slide copy**

> Put a supplier's availability check inside a buyer's ordering workflow.
>
> Put a specialist transformation inside the pipeline that needs it.

The provider maintains the capability. The host supplies context and permitted
actions.

**Visual:** Provider capability inside a host workflow; a visible boundary with
named inputs, actions, and styling hooks.

**Speaker notes:** “More software competes for a place in existing workflows.
Providers gain reach when their capabilities fit inside another product;
hosts gain useful functionality without rebuilding it. Users can continue
their task without repeated navigation or data entry.” Explain the split of
responsibility: independent provider maintenance and host-controlled integration.
“Runtime insulation aims to make that integration easier to assess, control,
and maintain.” Treat adoption and cost savings as the business thesis, not
measured CEM outcomes or a quantified AI app-growth claim.

**Sources:** [Embedding pitch](cem-platform-pitch.md#embedding-applications),
[EPA/DWA idea synthesis](platform-evidence.md#ideas-from-epa-and-dwa).

## 21 · Find your place in the platform

**Time:** 47:00–49:00 · 2 minutes

**Slide copy**

Rust engine + schema packages

Native / WASM → CLI · library · Node · browser · workers

`<cem-element>` → Consumer Semantic Theme · CEM Components

CEM Studio — in development · CEM Site

**Visual:** Layered platform map; highlight the path taken by the login. Keep
tools and application surfaces above the common processing layer.

**Speaker notes:** “We have used a small path through a wider platform. The
Rust engine and schema packages supply common processing. Native and WASM
hosts expose it to commands, embedded libraries, and browser work. The
declarative component layer connects processing to interaction, with semantic
theme definitions and reusable components.” Locate Studio as the workbench in
development and Site as the platform's publishing/presentation surface. Studio
is not ready for participants. These names identify roles, not a claim that
all migrations, public releases, or runtime capabilities are complete.

**Sources:** [Required platform map](presentation.md#published-agenda-and-topic-coverage),
[implementation boundaries](presentation.md#implementation-claims-and-preparation-boundaries).

## 22 · Put processing where the work happens

**Time:** 49:00–50:30 · 1 minute 30 seconds

**Slide copy**

CLI: batch and CI/CD

Library: services and tools

Node / browser WASM: supported host operations

Connectors read your sources and publish to your destinations.

**Visual:** The shared processing rule projected into three hosts; connector
ports point to files, services, and host-supplied storage.

**Speaker notes:** “Adoption does not require starting with a UI. Run a
validation or transformation in a job, embed it behind an existing service,
or use the supported WASM interfaces in a tool or browser.” Define connectors
plainly as customizable resource access and publication interfaces. The host
supplies credentials, transport, permissions, and operational integration.
Native, Node, and browser I/O differ; the shared engine does not mean every
connector or operation is available identically everywhere. Direct installation
and version questions to the resource discussion.

**Sources:** [CLI/library applications](platform-evidence.md#cli-library-and-pipeline-applications),
[connectors](platform-evidence.md#format-and-location-independence).

## 23 · Insulate each vendor's contribution

**Time:** 50:30–52:30 · 2 minutes

**Slide copy**

Memory · Storage · CPU · Network · DOM · CSS

Request access → grant authority → enforce at runtime

Nested scopes stay within the parent's safeguards.

Architecture target; current controls are partial.

**Visual:** Host boundary with two vendor scopes and a nested contribution;
six resource layers surround each applicable scope. Draw explicit permitted
crossings, not unrestricted arrows between vendors.

**Speaker notes:** “The goal goes beyond naming separate modules. Each
included contribution needs runtime insulation around what it can read,
change, and consume. A declaration requests access; the host grants authority;
mechanisms must enforce the boundary. A nested contribution must not gain more
authority or escape the aggregate resource budget.” Explain why this matters
to integration reviews and predictable behavior. Clearly identify the model as
the target: existing allocation accounting, scheduling, resolvers, ownership,
and CSS scopes have narrower guarantees. Same-page scripts and privileged native
plugins are not made hostile-code safe by declaring a scope.

**Sources:** [Six layers and current gaps](platform-evidence.md#six-resource-layers),
[nested scopes](platform-evidence.md#nested-scopes-and-inclusions).

## 24 · Make performance a lifecycle property

**Time:** 52:30–54:00 · 1 minute 30 seconds

**Slide copy**

Retain typed data. Bound independent work. Resume compatible state.

Measure time to a useful result, throughput, and resource use.

**Visual:** Input → processing → usable result, with retained data,
concurrency/resource budgets, and component SSR/resume attached to their stages.

**Speaker notes:** “Performance belongs to the whole path, not one parser
benchmark. Retained structures can avoid repeated conversions. Bounded workers
can help suitable independent workloads. Compatible component SSR output can
be adopted when the browser resumes.” Streaming depends on both producer and
consumer support; some operations require materialization. Component resume is
bounded support, while whole-page CLI SSR remains proposed. Do not give a
universal speedup or saving: compare equivalent work and measure the host,
input size, first useful result, total duration, and resource use.

**Sources:** [Processing and performance evidence](platform-evidence.md#processing-delivery-and-performance),
[component lifecycle](../cem-element-lifecycle-principle.md).

## 25 · Bring one useful integration

**Time:** 54:00–60:00 · 6 minutes

**Slide copy**

A format boundary. A repeated rule. A capability to embed.

Bring your data. Keep your expertise.

Resources and updates

https://github.com/EPA-WG/cem/discussions/24

**Visual:** Three small pilot options and a readable QR code/link. Keep this
slide visible throughout the question period.

**Speaker notes:** “Which boundary in your work would make a useful first
experiment? What input, expected output, and failure case would convince you
the integration is worth keeping?” Spend most of this block listening and
answering. Connect questions to a bounded pilot rather than requesting a
platform-wide replacement decision. Explain that discussion #24 is the single
place for examples, CLI releases, installation notes, recordings, and Studio
access as those links become available. If a support question exceeds verified
evidence, identify the required check rather than promising unsupported parity.

**Sources:** [Stable resource discussion](https://github.com/EPA-WG/cem/discussions/24),
[adoption outcome](presentation-plan.md#audience-priorities-and-adoption-outcome).

## 26 · Appendix — 32 schema packages

**Time:** Outside the timed run; show in response to a relevant question.

**Slide copy**

Data and markup: csv, html, json, markdown, mathml, svg, xhtml, xml, yaml

Query, style, transformation: css, css-selector, scss, xpath, xslt

External schema resources: json-schema, relax-ng

CEM languages: cem-ml, cem-ql, cem-native-template, cem-element-template,
cem-transform, schema, schema-package

Platform contracts: cem-data-island, cli-command, module-map, module-map-v2,
module-map-v3, studio-project

Structural projections: cem-ast-projection, cem-dom-projection,
cem-events-projection

32 package manifests, including internal contracts and revisions.

**Visual:** Six grouped catalog panels. Keep the count qualification visible.

**Speaker notes:** “This is package breadth, not a claim of 32 complete
external languages. A package can define a data vocabulary, a language profile,
an internal platform contract, or a structural projection.” Capability depth
must be checked separately for parsing, validation, querying, transformation,
formatting, and output. Point to the catalog for profile and implementation
references. Emphasize extension as the adoption path, rather than treating
the count as a feature-parity score.

**Sources:** [Catalog and limits](platform-evidence.md#schema-package-catalog).

## 27 · Appendix — Defined profiles, explicit boundaries

**Time:** Outside the timed run.

**Slide copy**

XML / JSON / YAML / CSV → retained typed trees

XPath / CEM-QL / CSS selectors → supported query operations

XSLT → defined compilation and runtime profile

Native / Node / browser → host-specific I/O and capabilities

**Visual:** Representation, language, and runtime rows with explicit boundaries.

**Speaker notes:** Use this slide to answer support questions precisely. CSV
fields remain strings until an explicit rule interprets them; YAML import has
a bounded profile. Query examples establish selected parity, not identical
language semantics. The XSLT profile includes substantial operations but does
not claim full conformance. JSON Schema and RELAX NG package presence does not
establish complete instance validation. Native, Node, and browser hosts expose
different resource access. Custom connectors extend the host interfaces;
arbitrary new syntax also needs an import adapter.

**Sources:** [Schema capability depth](platform-evidence.md#schema-package-catalog),
[runtime profile](../xslt-runtime-lowering.md),
[deployment contract](../cem-ml-deployment-contract.md).

## 28 · Appendix — Six layers: controls and remaining boundaries

**Time:** Outside the timed run.

**Slide copy**

Memory: accounted allocations ≠ full private-memory isolation

Storage: host ports ≠ universal per-vendor store authority

CPU: bounded/cooperative work ≠ preemption of arbitrary native code

Network: configured resolver policy ≠ universal denied-by-default egress

DOM: owned ranges ≠ hostile same-page-script confinement

CSS: scope ownership ≠ containment of every style effect

**Visual:** Six paired rows: existing foundation / remaining boundary. Keep the
architecture-target label consistent with slide 22.

**Speaker notes:** “The value of this model depends on observable enforcement
at every crossing. An ownership label, permission declaration, or resource
counter is not sufficient by itself.” Current plugins execute in process with
host privileges; elapsed-time checks after return do not preempt arbitrary
plugin code. Resolver defaults are passthrough unless the host supplies a
restrictive policy. A convincing proof needs allowed and denied cases,
indirect effects, nested budgets, revocation, and sibling behavior. Capability
security is not claimed as a CEM invention; the goal is its integration across
the declarative composition.

**Sources:** [Six-layer evidence](platform-evidence.md#six-resource-layers),
[plugin runtime](../../packages/cem_ml/src/plugin/runtime.rs),
[resolver](../../packages/cem_ml/src/resolver.rs).

## 29 · Appendix — Reproduce the walkthrough

**Time:** Outside the timed run.

**Slide copy**

Source + schema + engine version

Commands + expected results + recovery checkpoints

Resources and updates → discussion #24

**Visual:** Three artifact groups with links; include only verified commands
and versions resolved by the deck's evidence manifest.

**Speaker notes:** Identify the exact version and route used to produce the
shown artifacts. A local source build does not establish a publicly installable
release. Participant instructions require separate clean-install verification
for each advertised host. The source and outputs should be available locally
for presenter recovery; the stable discussion receives distribution links as
they become ready. Studio is not required to follow the CLI/browser exercise.
Do not display guessed package versions or unsupported native macOS/Windows
installation instructions.

**Sources:** [Delivery and fallback](presentation.md#delivery-setup-and-fallback),
[preview generator](generate-code-preview.mjs),
[resources](https://github.com/EPA-WG/cem/discussions/24).

## Timing check

| Proposal block | Slides | Minutes |
| --- | --- | --- |
| Opening and Evolutionary Architecture | 01–04 | 0–7 |
| Read and edit the login | 05–08 | 7–17 |
| Validate, inspect, diagnose, convert | 09–14 | 17–35 |
| Query, template, and light-DOM rendering | 15–19 | 35–47 |
| Platform and extension paths | 20–23 | 47–54 |
| Questions and first adoption experiment | 24 | 54–60 |

If time slips, shorten the supplier companion and catalog explanation. Keep
the edit, failure/repair, inspection, conversion, rendered result, required
platform map, and six-minute question period. The evidence state of the final
assets belongs in the generated transcript and preparation checklist, not in
guessed screenshots or invisible assumptions.
