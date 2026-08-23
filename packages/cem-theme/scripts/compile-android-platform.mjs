/** Compile the generated Android library and Compose consumer on a supported host. */

import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { CEM_NATIVE_PLATFORM_CONTRACT } from "./native-platform-contract.mjs";

const PACKAGE_ROOT = path.resolve(new URL("..", import.meta.url).pathname);
const GENERATED_ROOT = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/android");
const REPORT_ROOT = path.join(PACKAGE_ROOT, "dist/reports/native");
const contract = CEM_NATIVE_PLATFORM_CONTRACT.android;

function run(command, args, options = {}) {
    const result = spawnSync(command, args, {
        cwd: options.cwd,
        encoding: "utf8",
        env: process.env,
    });
    const output = `${result.stdout ?? ""}${result.stderr ?? ""}`.trim();
    if (result.error || result.status !== 0) {
        if (output) console.error(output);
        throw result.error ?? new Error(`${command} ${args.join(" ")} exited ${result.status}`);
    }
    return output;
}

async function main() {
    if (!process.env.ANDROID_HOME && !process.env.ANDROID_SDK_ROOT) {
        throw new Error("compile-android-platform requires ANDROID_HOME or ANDROID_SDK_ROOT; use the native-android CI job");
    }

    const gradleCommand = process.env.CEM_ANDROID_GRADLE_COMMAND || "gradle";
    const gradleVersion = run(gradleCommand, ["--version"]);
    const javaVersion = run("java", ["-version"]);
    if (!gradleVersion.includes(`Gradle ${contract.gradleVersion}`)) {
        throw new Error(`expected Gradle ${contract.gradleVersion}`);
    }
    if (!new RegExp(`version "${contract.jdkVersion}(?:[."]|$)`).test(javaVersion)) {
        throw new Error(`expected JDK ${contract.jdkVersion}`);
    }

    const temporaryRoot = await fs.mkdtemp(path.join(os.tmpdir(), "cem-android-consumer-"));
    const consumerRoot = path.join(temporaryRoot, "cem-token-platforms");
    try {
        await fs.cp(GENERATED_ROOT, consumerRoot, { recursive: true });
        run(
            gradleCommand,
            ["--no-daemon", "--stacktrace", ":cem-tokens:assembleRelease", ":sample:assembleDebug"],
            { cwd: consumerRoot },
        );

        await fs.mkdir(REPORT_ROOT, { recursive: true });
        await fs.writeFile(
            path.join(REPORT_ROOT, "android-compile-report.json"),
            `${JSON.stringify(
                {
                    version: 1,
                    platform: "android",
                    contract,
                    actual: {
                        gradle: gradleVersion.match(/Gradle [^\n]+/)?.[0] ?? "unknown",
                        java: javaVersion.split("\n")[0],
                    },
                    checks: ["android-library-release-assemble", "compose-consumer-debug-assemble"],
                    cleanConsumer: true,
                    failHardViolations: 0,
                },
                null,
                2,
            )}\n`,
            "utf8",
        );
    } finally {
        await fs.rm(temporaryRoot, { recursive: true, force: true });
    }

    console.log("compile-android-platform: Android library and Compose clean consumer compiled");
}

main().catch((error) => {
    console.error(error);
    process.exit(1);
});
