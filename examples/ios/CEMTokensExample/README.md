# CEMTokensExample

Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>

Minimal SwiftUI example showing how a consumer can use the generated CEM Swift Package. Color, spacing, shape,
typography, weight, and minimum target size all resolve from CEM token constants.

This fixture intentionally keeps the generated file out of source control. Rebuild it with:

```bash
yarn nx run @epa-wg/cem-theme:build:token-platforms
```

Then add `packages/cem-theme/dist/lib/token-platforms/ios/` as a local Swift Package and import `CEMTokens`. The
standalone `CEMTokens.swift` remains available for consumers that need the copy workflow.

The supported-host compile gate is:

```bash
yarn nx run @epa-wg/cem-theme:compile:ios-platform
```
