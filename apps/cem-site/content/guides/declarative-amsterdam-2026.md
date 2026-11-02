# One document, reusable capabilities

Start with a small Login view. Change its source, catch a mistake, repair it,
inspect its structure and turn it into a useful browser result. The same journey
introduces CEM's wider purpose: keep data and processing rules useful as sources,
consumers and execution hosts change.

Follow the Declarative Amsterdam demonstration without installing anything, or
use the tested local transfer kit described in the
[participant setup](https://github.com/EPA-WG/cem/blob/712ff2e45b89d159abba7b8447ab53e5efb8ae68/docs/declarative-amsterdam-2026/participant-setup.md).
The handout identifies its tested host and limits; it does not offer a public kit
download or establish public installation support.
[Discussion #24](https://github.com/EPA-WG/cem/discussions/24) is the resource hub.

## Start from a useful result

**0–7 minutes.** A source document produces a small sign-in view. The opening's
Thought → DNA → CEM sequence is a metaphor for a structure that can evolve.
Ask what should remain true when one part changes: a heading's meaning, a
validation rule, a source location or a permitted action.

For XML/XSLT developers, the entry point is familiar document processing. For
web developers, it is the visible result. For platform owners, it is the cost
of maintaining the same rule across several delivery paths. Choose sources;
pick, extend or create schemas; compose modules; test, package, run and evolve.
CEM supplies processing contracts, not a replacement for the team's CI or
operational ownership.

## Keep your expertise and make one edit

**7–17 minutes.** Read corresponding HTML and CEM-ML excerpts:

```html
<h1 id="login-title">Sign in</h1>
```

```cem
{h1 @id=login-title | Sign in}
```

These are excerpts; the complete sources carry their vocabulary and namespace
context. Change the heading, predict the result, then save before executing a
configuration. An unsaved editor draft and the saved execution input are distinct.
Keep an independent baseline copy, and restore it before comparison.

XML, XPath and XSLT expertise remains useful through supported profiles and
explicit adapters. Different syntaxes retain different capabilities. The
[Login inputs and expected results](https://github.com/EPA-WG/cem/blob/712ff2e45b89d159abba7b8447ab53e5efb8ae68/docs/declarative-amsterdam-2026/login-presenter/README.md)
identify the exact sources and configurations.

## Trust a check you can inspect and repair

**17–35 minutes.** Validate the prepared syntax-valid form that lacks its required
method. Select `demo.login.method_required`, inspect its original source span,
add `@method=post`, save and rerun. The repair removes the error; three known
reference warnings remain. A parser accepting syntax is different from a domain
schema accepting an instance.

Inspect native structure, compare pretty and tabular formatting, and read the
source-color export. Compare the 21 named Login semantic values across HTML and
CEM-ML. This is a bounded comparison, not byte equality or universal lossless
conversion. Namespace mirrors, wrappers and lexical differences remain explicit.

Then use Supplier as a short companion diagram: XML products and CSV stock feed
one graph producing HTML, JSON and YAML. The
[Supplier inputs and expected results](https://github.com/EPA-WG/cem/blob/712ff2e45b89d159abba7b8447ab53e5efb8ae68/docs/declarative-amsterdam-2026/supplier-presenter/README.md)
include native failure/repair references. Live stock experiments belong after the
required Login journey, when a relevant question and remaining time permit.

## Follow processing into a view

**35–47 minutes.** Ask the same prepared question with XPath and CEM-QL. Evaluate
the standalone expression, then run the graph with data and template as separate
inputs. Change a saved template instruction, rebuild, and follow an output range
back to its authored template rule.

Run the script-free light-DOM Login view, change its local action label, save and
rerun. This live view reads the HTML resource independently of the CEM graph.
Its 15 advisory attribute warnings and the recorded CLI/worker validation
difference remain documented. A source coordinate printed by Inspect is not
automatically an interactive Provenance link; computed output does not promise
universal per-character data lineage.

Embedding puts a useful capability inside the workflow that needs it. A provider
maintains the capability; the host supplies context, permitted actions and visual
integration. Saving, fetching or authenticating introduces host-owned effects.
The sign-in view demonstrates presentation, not authentication or session security.

## Choose a host and name the boundaries

**47–54 minutes.** Place each owner around the demonstrated route:

| Owner | Role in the story |
| --- | --- |
| Rust engine and schema packages | Shared typed processing, contracts and diagnostics. |
| CEM-QL and transformation modules | Reusable queries, expressions and templates with defined profiles. |
| Native CLI and library | Batch/CI invocation and processing embedded in existing services or tools. |
| Node/browser WASM and workers | Supported host entry points with explicit resource and publication interfaces. |
| `cem-element` | Declarative browser data, events and templates in ordinary light DOM. |
| Consumer Semantic Theme and CEM Components | Shared semantic styling and reusable declarative controls. |
| Studio and Site | Project authoring/inspection and a published documentation surface. |

Use the [CLI guide](/guides/cem-ml/cli/), [runtime contract](/reference/cem-ml/wasm-runtime/),
[component catalog](/components/) and [theme tokens](/tokens/) to explore a
particular entry point. Format adapters handle representations; connectors use
host resolver/publication interfaces. Keeping a rule independent of location does
not remove permission checks or source attribution.

Six-layer vendor insulation across memory, storage, CPU, network, DOM and CSS is
an architectural target with partial controls. Light DOM and CSS ownership do
not provide hostile-script isolation; the supplied security diagram illustrates
the goal. AI-assisted authoring does not make declarations safe by construction.

State continuity and retained data can reduce repeated work within supported
contracts. Performance benefits require a measured workload. No benchmark,
automatic parallel speedup, full streaming pipeline or complete bundler parity
is claimed. Entered passwords/OTP block recovery until explicitly cleared;
automatic whole-Studio recovery remains disabled. Never use real credentials in
the exercise or export them in a checkpoint. The shared recovery budget remains
4 MiB with a five-second handoff deadline.

## Bring one useful integration

**54–60 minutes.** Keep the final six minutes for questions. Bring one format
boundary, reusable processing rule or capability to embed. Agree on inputs,
expected outputs and a meaningful change, then check which contracts survived.
Measure correctness, integration effort and resource behavior before deciding
whether to expand the pilot.

If a live operation fails, name the failure and open its frozen reference.
A saved result explains the expected behavior; it does not prove the latest
edit succeeded. Preserve projects and drafts. Installation happens before the
clock, and optional scope, performance and package references add no scheduled
time. Continue through [resources and updates](https://github.com/EPA-WG/cem/discussions/24).
