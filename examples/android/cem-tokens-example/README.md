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

Current repository validation is offline:

```bash
yarn nx run @epa-wg/cem-theme:build:token-platforms
```

That target builds the Android XML and Compose token artifacts and runs `validate-platforms.mjs`. A true Kotlin/Compose
compile still requires copying these files into a supported Android Gradle project or running a future CI job with Java,
Gradle, the Android Gradle plugin, and Kotlin available.
