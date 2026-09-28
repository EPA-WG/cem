# Component bundle build

Run `yarn nx run @epa-wg/cem-components:build:bundle` from the workspace root.
The graph imports canonical XHTML through the shared XML-to-CEM AST boundary,
collects the retained artifacts and applies `components.cemt`. It exports
`dist/components.xhtml`; the package continues to ship the individual sources.

Each declaration is copied under a container with a relative `xml:base` pointing
to its original file. Fragment consumers therefore keep the original dependency
base after package installation. Capability attributes remain explicit on the
consumer declaration, matching individual fragment loading.

The copy template is compact deliberately: formatting whitespace in a CEMT body
can become output text. Native and browser checks protect unchanged canonical
template text, XML entity escaping, declaration metadata and unique template IDs.
Do not format this transform by adding body whitespace without checking those
assertions. Default XHTML namespace declarations are supplied by the output root;
XML attributes retain their `xml:` prefix.

`cem-bundle.html` demonstrates all bundled components. The playground verifier
runs it from both the repository and isolated package archives, checks source
bases and stylesheet ownership, exercises selection, and verifies duplicate
registration diagnostics.
