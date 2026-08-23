# CEM Android Token Example

Copyright (c) 2026 Sasha Firsov <https://github.com/sashafirsov>

Minimal Compose example showing how a consumer can use generated CEM Android resources and constants after running:

```bash
yarn nx run @epa-wg/cem-theme:build:token-platforms
```

Open the self-contained generated Gradle project under
`packages/cem-theme/dist/lib/token-platforms/android/`, or copy its `cem-tokens` library module into an existing
Android project. The legacy standalone paths remain available:

- `packages/cem-theme/dist/lib/token-platforms/android/values/cem-tokens.xml`
- `packages/cem-theme/dist/lib/token-platforms/android/values-night/cem-tokens.xml`
- `packages/cem-theme/dist/lib/token-platforms/android/compose/CEMTokens.kt`

This directory is the checked-in Compose source fixture copied into the generated `sample` module.

Credential-free repository validation is:

```bash
yarn nx run @epa-wg/cem-theme:build:token-platforms
```

The supported-host Gradle/Kotlin/Compose compile gate is:

```bash
yarn nx run @epa-wg/cem-theme:compile:android-platform
```
