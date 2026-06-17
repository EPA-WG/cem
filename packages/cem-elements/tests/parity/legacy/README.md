# Legacy Parity Fixtures

File-backed fixtures for the Phase 3.1 legacy `<custom-element>` parity inventory.

These fixtures are intentionally source fixtures first: each behavior has a legacy HTML+XSLT declaration and, where
useful, a canonical CEM-ML twin. Storybook still owns the current browser runtime assertions; the next Phase 3.1 task
promotes these files into the release gate.

Run the fixture manifest check with:

```bash
yarn nx run cem-elements:verify-legacy-fixtures
```

