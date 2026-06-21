# Material Parity Fixtures

File-backed fixtures for the Phase 3.1 material component parity inventory.

Each fixture captures the characteristic legacy authoring surface for one material reference component and pairs it with
a canonical CEM-ML twin used by the current `<cem-element>` substrate. These are source fixtures first; Storybook still
owns the browser runtime assertions until the release-gate promotion task wires these files into executable tests.

Run the fixture manifest check with:

```bash
yarn nx run cem-elements:verify-material-fixtures
```
