# Discussion #24 resource comment — draft

Archived review copy from `3bdd9942`. The existing discussion comment was
subsequently updated on 2026-11-03. The exact published body is in
`discussion-resource-comment-published.md`; its separate readback is recorded in
`discussion-resource-comment-delivery.json`. The notices below describe the
prepublication review stage and remain as the guarded draft fixture.

Do not post yet. Review copy for the existing resource comment; no update has
been sent. The public `4d21f9cd…` deployment passes general acceptance, all 23
selected offline Login/Supplier configurations and the 14 Studio destinations.
No comment has been sent.

- [CEM demo site](https://green-morning-fd6e.trans-acf.workers.dev/)
- [Declarative Amsterdam presentation slides](https://docs.google.com/presentation/d/1v5JEclFycuXuYk5Mnpwb0alxdKIi30J6XgvLBmawxI4/edit?usp=sharing)
- [CEM formats playground](https://green-morning-fd6e.trans-acf.workers.dev/packages/cem-demo-element/demo/playground)

## Slide resources

All 30 physical slides are included below, in the current native deck order.
Slides 25 and 27–29 are optional references. Demonstration rows open ordinary
Studio projects, resources and configurations; reference rows link to the
supporting source or explanation. Slide 16 has separate query and expression
links. Clone a sample before editing. Stable IDs identify slides after reordering.

| Slide | Stable slide ID | Demonstration or topic | Example or supporting reference |
| --- | --- | --- | --- |
| 01 | `p1` | CEM — DNA for structured data (reference) | [Opening idea: DNA for structured data](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/opening-teaser.md) |
| 02 | `h36c7c826c4eee100_0_0` | CEM-ML — Declarative document lifecycle engine (reference) | [Session brief and tutorial plan](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/presentation.md#presentation-brief) |
| 03 | `p2` | read | [login: readme](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=readme&build=read-edit) — `read-edit` |
| 04 | `p3` | Make capabilities reusable across systems (reference) | [Reusable processing across formats and locations](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/platform-evidence.md#format-and-location-independence) |
| 05 | `p4` | Keep useful rules as systems change (reference) | [Contracts that survive a change](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/cem-manifesto.md#make-change-a-normal-operation) |
| 06 | `p5` | Keep your markup knowledge (reference) | [HTML Login source](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/examples/semantic/login.html); [CEM-ML Login source](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/examples/cem-ml/login.cem) |
| 07 | `p6` | A small vocabulary for the whole journey (reference) | [Namespace, nodes, attributes and content in Login](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/examples/cem-ml/login.cem) |
| 08 | `p7` | edit | [login: login-cem](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-cem&build=validate) — `validate` |
| 09 | `p8` | Pick, extend, or create your own schemas (reference) | [Schema package guide](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/packages/cem_ml/schema-packages/README.md) |
| 10 | `p9` | fail | [login: login-invalid](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-invalid&build=validation-repair) — `validation-repair` |
| 11 | `p10` | repair | [login: login-invalid](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-invalid&build=validation-repair) — `validation-repair` |
| 12 | `p11` | inspect | [login: login-cem](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-cem&build=inspect) — `inspect` |
| 13 | `p12` | format | [login: login-cem](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-cem&build=format-pretty) — `format-pretty`, `format-tabular`, `source-colors` |
| 14 | `p13` | compare | [login: login-html](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-html&build=semantics-html) — `semantics-html`, `semantics-cem`, `html-conversion` |
| 15 | `p14` | companion pipeline overview | [Supplier transformation graph](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=supplier-pipeline&file=supplier-graph&build=availability) — `availability` |
| 16 | `p15` | query | [login: login-cem](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-cem&build=semantics-html-cemql) — `semantics-html-cemql`, `semantics-cem-cemql` |
| 16 | `p15` | expression | [login: login-expression](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-expression&build=expression-html) — `expression-html`, `expression-cem` |
| 17 | `p16` | transform | [login: login-view-graph](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-view-graph&build=render-graph) — `render-graph`, `render-html`, `render-cem` |
| 18 | `p17` | source provenance | [Login template and generated output](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-view-template&build=render-graph) — `render-graph` |
| 19 | `p18` | live | [login: login-live-view](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=login&file=login-live-view&build=live-view) — `live-view` |
| 20 | `p19` | Embedding applications (reference) | [Provider and host responsibilities](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/cem-platform-pitch.md#embedding-applications) |
| 21 | `p20` | Find your place in the platform (reference) | [Platform layers and implementation boundaries](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/presentation.md#published-agenda-and-topic-coverage) |
| 22 | `p21` | Put processing where the work happens (reference) | [CLI, library and pipeline hosts](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/platform-evidence.md#cli-library-and-pipeline-applications) |
| 23 | `h209a9db50e1b8a77_0_0` | Browser vs DCE security (reference) | [DCE security promise and current limits](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/platform-evidence.md#dce-promises-carried-forward) |
| 24 | `p22` | Insulate each vendor’s contribution (reference) | [Memory, Storage, CPU, Network, DOM and CSS scopes](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/platform-evidence.md#six-resource-layers) |
| 25 | `p27` | Six layers: foundations and boundaries (optional reference) | [Six-layer foundations and enforcement gaps](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/platform-evidence.md#six-resource-layers) |
| 26 | `p23` | Make performance a lifecycle property (reference) | [Lifecycle performance mechanisms and limits](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/platform-evidence.md#processing-delivery-and-performance); [Measured public startup](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/public-demo-4d21f9cd-verification.md) |
| 27 | `p28` | Follow the source and the evidence (optional reference) | [Local kit and frozen reference results (dated)](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/participant-resources.md); [Verified public workflow and exact build](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/public-demo-4d21f9cd-verification.md) |
| 28 | `p25` | 32 schema packages (optional reference) | [Schema package catalog and capability limits](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/platform-evidence.md#schema-package-catalog); [Package source guide](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/packages/cem_ml/schema-packages/README.md) |
| 29 | `p26` | Reuse expertise within explicit profiles (optional reference) | [XSLT runtime profile](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/xslt-runtime-lowering.md); [Native, Node and browser deployment contracts](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/cem-ml-deployment-contract.md) |
| 30 | `p24` | supplier | [supplier-pipeline: supplier-stock](https://green-morning-fd6e.trans-acf.workers.dev/studio/?collection=amsterdam-2026&sample=supplier-pipeline&file=supplier-stock&build=stock-source) — `stock-source`, `availability` |

## Examples already shared in this discussion

These original examples remain available alongside the guided Studio route:

- Slide 03: [Login form CEM-ML](https://green-morning-fd6e.trans-acf.workers.dev/packages/cem-demo-element/demo/playground?title=Login+form+CEM-ML&description=A+sign-in+form+with+labeled+email+and+password+fields.&source=%40doc+cem-ml+1%0A%40ns+cem+%3D+%22https%3A%2F%2Fcem.dev%2Fns%2Fcore%2F1%22%0A%40ns+html+%3D+%22http%3A%2F%2Fwww.w3.org%2F1999%2Fxhtml%22%0A%40default+html%0A%7Bmain+%40aria-labelledby%3Dlogin-title+%40cem%3Ascreen%3Dlogin+%7C%0A++++%7Bh1+%40id%3Dlogin-title+%7C+Sign+in%7D%0A++++%7Bform+%40action%3D%22%23session%22+%40method%3Dpost+%40cem%3Aform%3Dsign-in+%7C%0A++++++++%7Blabel+%40for%3Demail+%7C+Email%7D%0A++++++++%7Binput+%40autocomplete%3Demail+%40id%3Demail+%40name%3Demail+%40required+%40type%3Demail%7D%0A++++++++%7Blabel+%40for%3Dpassword+%7C+Password%7D%0A++++++++%7Binput+%40autocomplete%3Dcurrent-password+%40id%3Dpassword+%40name%3Dpassword+%40required+%40type%3Dpassword%7D%0A++++++++%7Bbutton+%40type%3Dsubmit+%40cem%3Aaction%3Dprimary+%7C+Sign+in%7D%0A%7D+++%7D%0A&input=cem&output=html)
- Slide 04: [7e. Embedded XSLT fragment](https://green-morning-fd6e.trans-acf.workers.dev/packages/cem-elements/demo/external-template)
- Slide 04 source: [XSLT template fragment](https://green-morning-fd6e.trans-acf.workers.dev/packages/cem-elements/demo/html-template.xhtml#embedded-xslt)
- Slide 05: [schema packages](https://github.com/EPA-WG/cem/tree/develop/packages/cem_ml/schema-packages)
- Slide 08: [Hello, CEM-ML](https://green-morning-fd6e.trans-acf.workers.dev/packages/cem-demo-element/demo/playground?title=Hello%2C+CEM-ML&description=The+same+greeting+written+in+CEM-ML+and+rendered+as+HTML.&source=%7Barticle+%7C%0A++++%7Bh2+%7C+Hello%2C+CEM-ML%7D%0A++++%7Bp+%7C+Edit+this+CEM-ML%2C+then+choose+another+output+format.%7D%0A++++%7Bbutton+%40type%3Dbutton+%7C+A+live+HTML+button%7D%0A%7D%0A&input=cem&output=html)

Playground links prefill the source and formats; choose **Render** to produce
the preview.

The fragment-library link uses ordinary HTTPS so it opens from GitHub; use the
browser's View source action to inspect its markup.

Use checked-in or disposable example data. Do not enter real credentials.
Entered passwords or one-time codes block recovery until explicitly cleared;
they must never be exported. The Login view demonstrates local interaction,
not authentication or a credential service. Security and performance references
state their supported profiles and remaining boundaries.

The [local kit and saved reference index](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/participant-resources.md) documents the dated local-transfer fallback.
The [public demo verification record](https://github.com/EPA-WG/cem/blob/bc51404ef57639c4b627ce88f615b629cbfeb5e8/docs/declarative-amsterdam-2026/public-demo-4d21f9cd-verification.md)
records the deployed build and successful public acceptance. Source references
are pinned to `bc51404e`; the separate kit instructions retain their recorded
host and installation limits. The native deck remains the slide-order authority.
