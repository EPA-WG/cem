# CEM Android Token Example

Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>

Minimal Compose example showing how a consumer can use generated CEM Android resources and constants after running:

```bash
yarn nx run @epa-wg/cem-theme:build:token-platforms
```

Use these generated files in an Android app module:

- `packages/cem-theme/dist/lib/token-platforms/android/values/cem-tokens.xml`
- `packages/cem-theme/dist/lib/token-platforms/android/values-night/cem-tokens.xml`
- `packages/cem-theme/dist/lib/token-platforms/android/compose/CEMTokens.kt`

This directory is a source fixture, not a full Gradle project.
