# CEM-ML: Evolutionary Architecture for the Declarative Web Stack

## Proposed session update

Thank you for the suggestion to make the session more accessible and
interactive. I would like to turn the presentation into a 60-minute
mini-tutorial. The session will assume no previous experience with CEM-ML and
will show what an evolutionary architecture for the declarative web stack looks
like in practice, using CEM-ML and the implemented platform modules as a working
example.

Participants will follow one small document through validation, inspection,
conversion, transformation, and rendering. A public CEM-ML CLI release and
tested installation notes for NodeJS and Linux (may be native in OSX, Windows too) is planned before the
conference's October review deadline. The same exercise will also be prepared
for a presenter-led walkthrough so that a local setup problem does not prevent
anyone from following the session.

## Brief introduction

What if the languages and tools in a web application were not a tower of fixed,
unrelated standards, but an architecture able to evolve from shared declarative
contracts?

CEM-ML is the foundation of an open-source evolutionary architecture platform
for declarative web applications. It treats a language, its schema, its
transformations, and its presentation rules as related parts of one document
lifecycle. Instead of adding another hard-coded converter for every pair of
formats, CEM-ML lets schema packages describe how structured content is read,
validated, queried, transformed, formatted, and rendered.

The language is deliberately small and visual. Structure uses braces,
attributes use `@`, and `|` separates a node's header from its content:

```cem
@doc cem-ml 1
@ns cem = "https://cem.dev/ns/core/1"
@ns html = "http://www.w3.org/1999/xhtml"
@default html

{main @cem:screen=login |
    {h1 | Sign in}
    {form @cem:form=sign-in @method=post |
        {label @for=email | Email}
        {input @id=email @name=email @type=email @required}
        {button @type=submit | Sign in}
}   }
```

This is recognizable as a web document, but its meaning is not trapped in one
browser rendering step. During the tutorial, participants will change this
document, introduce and repair a validation error, inspect the resulting typed
structure and source-linked diagnostic, convert a second input format into the
same model, and transform the result into a working web view. One source stays
visible while its lifecycle expands.

That lifecycle is the platform's common semantic spine. CEM-ML currently reads
canonical CEM-ML and schema-packaged formats including CSV, JSON, YAML, CSS,
SCSS, Markdown, and the HTML/XML family. The same engine provides parsing,
validation, inspection, conversion, reporting, and transformation-graph
execution. Source mappings remain attached to the content, allowing a
diagnostic or generated result to be traced back to the bytes and transformation
that produced it.

The platform already applies this model beyond a command-line demonstration:

- The Rust `cem_ml` engine and its schema packages define the shared document
  lifecycle and compile to native and WebAssembly runtimes.
- CEM-QL supplies declarative queries, expressions, and templates over the same
  typed content rather than introducing an application-specific data model.
- The native CLI and synchronized browser/Node deployment projects expose the
  same operations to terminals, build systems, workers, and browser hosts.
- `<cem-element>` uses the browser runtime as a declarative, light-DOM component
  substrate: data, events, queries, and templates remain markup while the
  reusable runtime owns the necessary imperative machinery.
- Consumer Semantic Theme, CEM Components, CEM Studio, and CEM Site demonstrate
  how tokens, controls, a local-first workbench, and a static application can
  grow from that substrate rather than from framework-local UI behavior.

The architecture is evolutionary because those contracts are its DNA. A new
content type, schema version, query vocabulary, formatter, transform, component,
or host can join the lifecycle without redefining the whole stack. The platform
does not claim that every layer is finished: public CLI distribution is being
prepared, and the component catalog is actively migrating from compatibility
code to CEM-ML declarations. Those migrations are themselves useful evidence
for the central idea—the architecture is designed to replace and evolve its
parts while preserving shared semantics, diagnostics, and source identity.

## Mini-tutorial journey

The proposed 60 minutes are organized around visible outcomes:

1. **0–7 minutes — Why evolve the stack?** Start with the cost of disconnected
   formats, validators, build steps, component runtimes, and source mappings.
2. **7–17 minutes — Meet CEM-ML.** Read and edit the small login document,
   covering only the syntax needed for the exercise.
3. **17–35 minutes — Ask the document questions.** Validate it, inspect its
   structure, follow a source-linked error, and convert equivalent structured
   input through the same lifecycle.
4. **35–47 minutes — Turn data into an application view.** Apply a declarative
   query/template transformation and render the result as light DOM.
5. **47–54 minutes — Reveal the platform.** Connect the exercise to the CLI,
   WebAssembly runtime, `<cem-element>`, theme, components, Studio, and Site.
6. **54–60 minutes — What should evolve next?** Discuss participant use cases
   and which languages, schemas, or application layers could join the model.

The goal is not for everyone to understand the implementation in one hour. It
is for participants to leave having seen and changed CEM-ML, having used its
core lifecycle, and having a concrete answer to the question: what becomes
possible when the declarative web stack is designed to evolve?
