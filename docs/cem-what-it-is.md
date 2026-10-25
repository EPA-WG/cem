# What CEM is

CEM is a schema-driven language and toolchain for structured content,
transformations and declarative user interfaces. It brings document processing,
data processing, application UI and build pipelines into one ecosystem.

The central flow is:

**Source + schema → retained typed tree → query / template / graph → artifact or live UI.**

**CEM** names the ecosystem. **CEM-ML** is its structured markup language,
**CEM-QL** its query and computation language, and **CEMT** its template and
transformation resources.

## Languages and document model

CEM is:

- **A schema definition language.** CEM-ML schemas describe structure, datatypes,
  constraints, references, diagnostics and behavior contracts. Schemas are
  documents that the same toolchain can inspect, validate and transform.

- **A structured markup and data syntax.** CEM-ML combines namespaced nodes,
  attributes and mixed content with typed values, references, expressions and
  content-type scopes. It encompasses XML/HTML-style structures and vocabularies
  such as XHTML, SVG, MathML and XSLT. Its broader representation model is the
  useful sense of “superset”: native XML, HTML and XSLT source syntax enters
  through the corresponding parser or adapter.

- **A common document-processing model.** Imported documents become retained
  typed CEM trees. Queries and transformations consume those trees through shared
  node capabilities. AST, DOM and event projections provide different views of
  that structure while retaining source identity and locations.

- **A collection of 30+ schema packages.** The current catalog has 32 registered
  packages covering markup, data, styles, queries, templates, schemas,
  configurations and structural projections. Examples include XML, HTML, JSON,
  YAML, CSV, Markdown, CSS/SCSS, SVG/MathML, XPath, XSLT, JSON Schema and RELAX NG.
  Packages can contain multiple identities or profiles; the count includes
  control and artifact formats as well as source languages.

- **A parser and validator platform.** The shared engine runs registered parsing
  and validation contracts for supported sources and applies supplied CEM schemas
  to instance data. Language syntax, schema-source validation and application
  instance validation are separate operations.

- **A model for documents containing several languages.** Host schemas declare
  typed-content boundaries and embedded expression slots. Language-owned parsers
  handle supported content and expressions, including CEM-QL and XPath, with
  nested scopes and original source coordinates. Explicit raw payloads can be
  retained for a later processing stage.

## Queries, transformations and presentation

- **A query and computation language.** CEM-QL provides expressions, functions,
  modules and operations over native values and document nodes. The same language
  supports data selection, template expressions and schema behavior.

- **A template and transformation library.** Reusable CEMT modules construct
  documents and data, compose layouts, and implement schema-owned formatting and
  coloring stages. XPath and supported XSLT programs participate in the native
  processing stack alongside CEM-QL and CEMT.

- **A universal conversion framework.** Source and target adapters meet at a
  common typed-tree boundary, allowing supported conversions to share parsing,
  validation and source handling. Conversion follows registered routes and
  compatible data shapes; an authored template supplies domain-specific mapping.

- **A formatter and syntax colorer.** Schema-owned profiles produce compact,
  readable or tabular presentations, with terminal, HTML and Markdown coloring
  where supported. Formatting, coloring and final byte serialization are
  distinct stages, so presentation can share native structure and source maps.

- **An XPath and XSLT processor.** CEM includes native XPath compilation and
  evaluation, and stylesheet-to-executable processing for its supported XSLT
  profile. These are implemented language profiles with documented limits;
  individual standards and operations have their own compatibility contracts.

- **A source-aware processing platform.** Diagnostics identify original resources
  and ranges, including embedded languages. Transformation provenance connects
  generated output to source instructions and captured inputs. Computed values
  retain the provenance that the operation can establish.

- **A reusable artifact system.** Retained native trees and compiled artifacts
  support repeated work without reparsing or textual serialization at every
  stage. Explicit binary artifact formats support applicable native, WASM and
  worker handoffs; runtime handles retain their own ownership rules.

## Builds and execution

- **A build toolchain.** Versioned configurations and module maps describe
  resource inputs, transformations, dependencies and outputs. Builds can process
  folder trees, publish static assets, generate several artifacts and record
  their identities and source maps.

- **A dependency-aware transformation graph processor.** Graphs compose imports,
  conversions, named inputs, templates, joins and exports under bounded
  scheduling and failure policies. The current graph phase queue executes stages
  sequentially. Native CPU/I/O scheduling and browser worker execution provide
  separate concurrency facilities; physical parallel execution depends on the
  operation and host.

- **A Markdown-and-components site-building platform.** Markdown content,
  metadata, CEM-ML/CEMT layouts, reusable Web Components and folder-based assets
  can form a site project. The existing Markdown site example demonstrates
  native Markdown-to-HTML conversion, shared layouts, relative links and asset
  publication using an explicit graph. It uses a Hugo-inspired folder structure.

- **A processing stack with several delivery forms.** Rust libraries, a native
  `cem-ml` CLI, synchronized WASM libraries, Node/browser command hosts and Web
  Components expose shared processing semantics. Hosts supply their filesystem,
  network, worker and resource-access policies.

- **An inspectable, bounded runtime.** Processing exposes diagnostics, source
  views, graph results and execution traces, with limits and cancellation at
  defined boundaries. The embedding host controls resource admission and optional
  schema-package activation.

## Declarative applications and tools

- **A Declarative Custom Element implementation.** DCEs are declarative Web
  Components built on `cem-element`: templates, data islands, event-to-data
  bindings and shared browser capabilities drive light-DOM rendering. Reusable
  JavaScript behavior belongs in `cem-elements`; styled component composition
  belongs in `cem-components`.

- **A foundation for server and build-time rendering.** Native templates can
  produce HTML outside the browser, and Edge/SSR reference hosts exercise initial
  rendering, update plans and hydration metadata. A general whole-page DCE CLI
  compilation pipeline remains a proposal; its status is separate from the
  implemented native renderer and SSR processing boundary.

- **A semantic design system.** `cem-theme` defines shared tokens for appearance,
  spacing, density, interaction and other UI dimensions. Declarative components
  consume those tokens with keyboard, accessibility and theme contracts.
  Generated CSS and cross-platform token outputs share canonical source tables.

- **An offline-capable project environment.** CEM Studio opens ordinary projects
  containing resources, schemas and configurations. It provides source editing,
  validation, graph/build controls, artifact previews and provenance. Saved
  copies stay independent of example projects, and cached runtimes and resources
  support offline reopening and execution.

## How to read these capabilities

Support is defined per language, schema, operation and runtime profile. The
capability inventory is the detailed reference: registration alone does not
establish every parser, exporter or execution feature. In particular, JSON
Schema and RELAX NG currently provide schema-source validation rather than
general instance-validation engines, and XSLT execution follows a bounded
supported profile. Optional WASM deployments require host-authorized admission
of the relevant source packages.

The [roadmap](../roadmap.md) describes further delivery work. The capabilities
above describe the platform and identify the major implementation boundaries
where a broader product claim would be premature.

## Explore further

- [Content types, schemas and operation coverage](studio-content-types.md)
- [Schema packages and their authoring contracts](../packages/cem_ml/schema-packages/README.md)
- [CEM-ML syntax](cem-ml-syntax.md) and [CEM-QL runtime](../packages/cem_ql/README.md)
- [Native engine](../packages/cem_ml/README.md), [CLI](../packages/cem_ml_cli/README.md)
  and [WASM library](../packages/cem-ml-npm/README.md)
- [Shared import boundary](cem-data-import-principle.md) and
  [declarative UI principle](declarative-ui-principle.md)
- [SSR processing gate](cem-elements-edge-ssr-gate.md) and
  [whole-page CLI compilation proposal](cem-ml-cli-ssr-proposal.md)
- [Markdown site example](../examples/projects/markdown-site/README.md) and
  [Studio example projects](../examples/projects/README.md)
- [Semantic theme](../packages/cem-theme/README.md) and
  [declarative components](../packages/cem-components/README.md)
