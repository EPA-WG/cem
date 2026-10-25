# CEM-ML project

Use this ordinary editable project as a small starting point for a document or
site. The default source remains resource `source`, path `data/main.cem`, with
content type `application/cem` and schema `https://cem.dev/ns/cem-ml/1`:

```cem
@doc cem-ml 1
{main | My project}
```

`README.md` is resource `readme`, with content type
`text/markdown; charset=utf-8` and schema
`https://cem.dev/ns/data/markdown/1`. It is documentation, not a build input or a
configuration. The original CEM-ML source stays first and remains the default
`html-v1:source` document-preview choice.

## Editing and expected output

Create and name the project through Studio, then select `data/main.cem` in the
project's file tree. The main area uses the shared source editor. Save the source
before running commands against its persisted revision. Change the main text and
use the ordinary document preview to see the corresponding HTML main element.
That preview uses the shared builder; it does not add a saved command to the
project or execute JavaScript from source.

This starter has **no persisted configurations**, expected-output fixtures or
predefined diagnostic/repair buffers. A default document preview is not a
transformation-graph configuration. Add resources and an authored command, run or
graph configuration when a repeatable build is needed; keep input identities and
output artifacts explicit. For complete worked configurations, clone an example
from the normal Data, Documents and UI, Queries and transformations, or Schema
authoring folders.

## Provenance and repairs

Native source commands retain original source coordinates where supported;
Studio selects only admitted buffers and the locations actually published by
native diagnostics or source maps. Printed projection numbers do not supply a
source owner. Parse/Inspect presentation does not currently guarantee navigable
artifact-source references, and a simple document preview is not a promise of
arbitrary template or input-character lineage.

There is no intentionally invalid starter file. If an edit causes native
validation to fail, inspect its reported diagnostic and restore the original
source shown above, or repair the edited syntax and rerun. Missing source
coordinates remain missing; the UI does not manufacture them. Saving an edit
updates only this project. New projects, cloned examples and existing saved
drafts retain independent identities and are not reset by template updates.

## Shared controls and privacy

Navigation, source editing, command/results controls and previews use the shared
CEM components. This project contains no project-specific UI JavaScript or new
storage key. Page theme preferences remain separate from example content and
project data. Offline editing and rebuilding use the ordinary Studio cache and
saved resources; they require the relevant assets to have been loaded first.

Entered passwords and one-time codes block recovery until explicitly cleared
by the user. Automatic whole-Studio recovery remains disabled. The starter does
not capture credentials, grant external reads or replay actions during recovery.
