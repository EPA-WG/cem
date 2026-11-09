# Mapped external declarations for native SSR

Base commit `1d4501f686d0fcdc3423cc310720a6ce4f106b99`.

- [x] Resolve an authored HTML body-registry declaration from the effective
  document base through the bounded SSR reader. Require an exact public
  response URI, matching tag and version, and one resource-free CEM-ML
  template in a standalone HTML fragment.
- [x] Retain the external template owner and source ranges. Include the
  resource count, byte length, public URI and content hash in the native
  readiness proof. Account for the read and compilation under the original
  operation.
- [x] Preserve prior HTML, source maps and readiness reports when a resolver
  refuses the read, the response exceeds its budget or the declaration is
  outside the admitted profile.
- [x] Keep the published HTML range in Chromium when browser adoption lacks
  the server's external declaration proof.

Verification passed 16 CLI SSR tests, 13 native and 13 stripped export
tests, three stripped publication tests, and 51 Chromium scenarios.
The browser fixture used a same-origin declaration and made zero
off-origin requests.
