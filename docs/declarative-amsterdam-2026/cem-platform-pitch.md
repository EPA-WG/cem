# CEM Platform Pitch

**Make your data and capabilities reusable across systems.**

CEM enables enterprises and software vendors to build **declarative**
data services, pipelines, and applications. It is neutral to data formats,
source locations, and execution tiers. Its scopes are designed to
insulate each vendor's application or pipeline stage at runtime.

Cover the full software development lifecycle. Choose sources and formats.
Pick, extend, or create your own schemas. Reuse or develop modules, then compose,
debug, test, and package. Run, operate, and evolve the result in browsers,
servers, and CI/CD pipelines.

## Keep your data useful across systems

New sources and consumers often repeat the same business rules across batch
jobs, services, and web applications. Every change means more work and more
chances for errors.

CEM brings supported formats into a shared typed model. Combine multiple inputs
and produce multiple outputs, each in its own format. A supplier's XML catalog and CSV
stock feed could produce a YAML partner feed, a JSON response, and an
interactive availability view from one processing graph.

Keep your XML, XPath, and XSLT expertise. Compare XML with CEM-ML and XPath
with CEM-QL. Use a defined XSLT profile and (X)HTML delivery. Explore
**32 schema packages** for data, markup, queries, transformations, and platform
contracts in the [schema catalog](platform-evidence.md#schema-package-catalog).

Pick, extend, or create your own **schemas**. Define vocabulary, types, and
validation rules.
Along with your schemas, define pretty and tabular formatters and color output in HTML and terminal.
Deliver formatters and colorers with your schemas. Build, extend, or customize
**connectors** to read from your sources and publish to your destinations—files,
services, or your own storage systems. Change formats or locations while keeping
the processing rules that still apply.

## Defining schemas across every stage: from design and development to ongoing operations.

Use schemas to define expected data and catch errors at their source. Develop
transformation modules independently, then connect them into a pipeline.
Inspect its dependencies, intermediate results, and outputs.

**Edit, run, inspect, repair, and test.** Bring CLI and library interfaces into
existing tools and checks. CEM Studio is a workbench in development.
Source-linked diagnostics and traces connect results to their inputs and rules.

Package schemas, modules, and assets. Bind them to runtime resources and
policies. Use diagnostics to investigate failures and compatibility checks
to protect expected behavior as dependencies change.

## Compose vendors with security insulation

Give each vendor a controlled place in your product. CEM's scope architecture
targets six runtime insulation layers:

| Layer | Designed controls |
| --- | --- |
| Memory | Private state, shared references, and allocation |
| Storage | Stores, records, operations, and quotas |
| CPU | Execution and resource limits, including nested work |
| Network | Destinations, operations, and credentials |
| DOM | Rendered regions and browser interactions |
| CSS | Style effects and resource access through CSS |

Declarations request access. Hosts grant it. Runtime mechanisms enforce it.
Authorized data and effects cross controlled interfaces. Nested scopes inherit
safeguards and stay within parent resource limits. DOM and CSS controls apply
in the browser.

Combine stock processing, quotation, and UI from different vendors. Each
provider maintains its part; the host controls access to data and resources.
Full insulation is the architectural target. The
[security reference](platform-evidence.md#six-resource-layers) records current
enforcement and gaps.

## Embedding applications

> Put a supplier's availability check inside a buyer's ordering workflow.
>
> Put a specialist transformation inside the pipeline that needs it.

As AI accelerates software creation, more apps and services compete for a place
in customers' workflows. Embedding them within other products is matter of adoption,  hence a survival.
But shared the host's context expose its data and privacy. Protecting the host then requires security reviews, organizational safeguards, and coordination between hosts and providers, adding cost and integration delay.
Runtime insulation can reduce this burden and negating the adoption barrier.

CEM's **Declarative Embeddable Web Application** concept pairs reusable
interaction, state, and processing with scope insulation. The provider maintains
its capability, while the host supplies context, permitted actions, and visual
integration. This division helps providers reach customers through partners and
lets hosts add capabilities with less integration work. Users, in turn, can stay
within their workflow without repeated navigation or data entry.

## Put processing where the work happens

- **Linux native CLI:** batch processing, publishing, validation, and CI/CD.
- **Library:** embed processing in services, editors, and tools.
- **Node and browser WASM:** run supported operations in those hosts.
- **Declarative components:** deliver interactive views.

A transformation graph connects multiple sources, stages, and outputs for
data processing or web assets assembly. Retained data avoids repeated
conversions where possible. Bounded parallel execution and suitable streaming
stages can deliver useful results earlier. Component server rendering and
browser resume connect initial delivery to interaction.

Aim for responsive interfaces, higher throughput, and predictable resource use.
Benefits depend on the workload and host.

## Evolve without repeating the work

Formats, locations, vendors, and runtimes change. Keep each change local to
the parts it affects.

CEM applies **Evolutionary Architecture** through explicit schemas, module
and resource boundaries, and checks that protect agreed behavior. Map a new
feed to an existing vocabulary. Adapt a vendor's new version to the consumer's
contract. Move processing to another supported host and verify the same results.

Keep useful rules, reduce consumer migrations, and trace failures to their
cause. Each integration adds definitions and checks the next one can reuse.

## Bring one useful integration

Connect a source, reuse a transformation, or embed a partner capability.
Define inputs and expected outputs. Make one meaningful change and see how
much work you keep.

**Bring your data. Keep your expertise. Compose the capability your customers need.**

Follow [resources and updates](https://github.com/EPA-WG/cem/discussions/24)
for links to examples, releases, and Studio as they become available.
