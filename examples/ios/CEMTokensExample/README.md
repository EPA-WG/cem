# CEMTokensExample

Minimal SwiftUI example showing how a consumer can use generated CEM token constants after adding
`dist/lib/token-platforms/ios/CEMTokens.swift` to an app target.

This fixture intentionally keeps the generated file out of source control. Rebuild it with:

```bash
yarn nx run @epa-wg/cem-theme:build:token-platforms
```

Then copy or link `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift` into the app target.
