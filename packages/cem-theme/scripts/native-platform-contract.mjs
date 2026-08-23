/**
 * Supported Phase 8 native consumer and CI toolchain contract.
 *
 * Keep version pins here so generators, validators, compile gates, and docs
 * cannot silently choose different native build environments.
 */

export const CEM_NATIVE_PLATFORM_CONTRACT = Object.freeze({
    ios: Object.freeze({
        packageName: "CEMTokens",
        swiftToolsVersion: "6.1",
        swiftLanguageMode: "6",
        xcodeVersion: "16.4",
        iosDeploymentTarget: "15.0",
        macosDeploymentTarget: "13",
    }),
    android: Object.freeze({
        rootProjectName: "cem-token-platforms",
        namespace: "org.epawg.cem.tokens",
        sampleNamespace: "org.epawg.cem.example",
        androidGradlePluginVersion: "9.2.0",
        gradleVersion: "9.4.1",
        jdkVersion: 17,
        kotlinVersion: "2.3.21",
        composeBomVersion: "2026.08.00",
        activityComposeVersion: "1.13.0",
        compileSdk: 37,
        minSdk: 23,
        targetSdk: 36,
    }),
});
