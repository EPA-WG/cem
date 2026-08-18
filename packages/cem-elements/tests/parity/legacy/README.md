# Legacy Parity Fixtures

File-backed fixtures for the Phase 3.1 legacy `<custom-element>` parity inventory.

These fixtures are intentionally source fixtures first: each behavior has a legacy HTML+XSLT declaration and, where
useful, a canonical CEM-ML twin. Every legacy template must opt in with the exact
`lang="custom-element-v0"` annotation; the manifest check rejects an unannotated
template. Storybook owns the current browser runtime assertions, while the next Phase
3 task promotes every paired file into rendered and accessibility acceptance.

Run the fixture manifest check with:

```bash
yarn nx run cem-elements:verify-legacy-fixtures
```
