# Canonical XHTML declarations and scoped CSS for SSR

Base commit `792f98f421de7a37cd7952b307af0c59f653bb9b`.

- [x] Admit version-matched standalone XHTML component declarations through
  the bounded resolver and the native XML importer. Preserve public source
  ranges for the CEM-ML template, including XML entity boundaries.
- [x] Resolve declaration stylesheet imports from the declaration URL and
  nested imports from their stylesheet URLs. Keep the document base for
  instance styles. Share one declaration stylesheet closure across hosts.
- [x] Preserve prior HTML, reports and source maps on XML, identity or CSS
  refusal. Verify both hosts and their stylesheet usage in the readiness proof.
- [x] Verify Chromium applies the published shared style only to the matching
  hosts. The standalone SSR fixture made zero resource requests.

Verification covered 19 CLI SSR tests, 15 native export tests and 53 Chromium
scenarios. The profile requires an explicit `.xhtml` URL and matching tag,
version and optional scope. External template imports, network policies and
resource-dependent browser adoption remain separate work.
