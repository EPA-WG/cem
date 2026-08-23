/** Compile the generated Swift Package and iOS SwiftUI consumer on an Apple host. */

import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { CEM_NATIVE_PLATFORM_CONTRACT } from "./native-platform-contract.mjs";

const PACKAGE_ROOT = path.resolve(new URL("..", import.meta.url).pathname);
const GENERATED_ROOT = path.join(PACKAGE_ROOT, "dist/lib/token-platforms/ios");
const REPORT_ROOT = path.join(PACKAGE_ROOT, "dist/reports/native");
const contract = CEM_NATIVE_PLATFORM_CONTRACT.ios;

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

function versionTuple(value) {
    const match = value.match(/(\d+)\.(\d+)(?:\.(\d+))?/);
    if (!match) throw new Error(`cannot parse version from: ${value}`);
    return match.slice(1).map((part) => Number(part ?? 0));
}

function assertMinimum(actualText, minimumText, label) {
    const actual = versionTuple(actualText);
    const minimum = versionTuple(minimumText);
    for (let index = 0; index < 3; index += 1) {
        if (actual[index] > minimum[index]) return;
        if (actual[index] < minimum[index]) {
            throw new Error(`${label} ${actualText} is below supported ${minimumText}`);
        }
    }
}

async function main() {
    if (process.platform !== "darwin") {
        throw new Error("compile-ios-platform requires macOS with Xcode; use the native-ios CI job");
    }

    const swiftVersion = run("swift", ["--version"]);
    const xcodeVersion = run("xcodebuild", ["-version"]);
    assertMinimum(swiftVersion, contract.swiftToolsVersion, "Swift");
    assertMinimum(xcodeVersion, contract.xcodeVersion, "Xcode");

    const temporaryRoot = await fs.mkdtemp(path.join(os.tmpdir(), "cem-ios-consumer-"));
    const consumerRoot = path.join(temporaryRoot, "CEMTokens");
    try {
        await fs.cp(GENERATED_ROOT, consumerRoot, { recursive: true });
        run("swift", ["package", "dump-package", "--package-path", consumerRoot]);
        run("swift", ["build", "--package-path", consumerRoot, "--configuration", "release"]);

        const sdkPath = run("xcrun", ["--sdk", "iphonesimulator", "--show-sdk-path"]);
        const architecture = process.arch === "arm64" ? "arm64" : "x86_64";
        const target = `${architecture}-apple-ios${contract.iosDeploymentTarget}-simulator`;
        const moduleRoot = path.join(temporaryRoot, "Modules");
        await fs.mkdir(moduleRoot, { recursive: true });
        run("xcrun", [
            "--sdk",
            "iphonesimulator",
            "swiftc",
            "-parse-as-library",
            "-emit-module",
            "-module-name",
            contract.packageName,
            "-swift-version",
            contract.swiftLanguageMode,
            "-target",
            target,
            "-sdk",
            sdkPath,
            "-emit-module-path",
            path.join(moduleRoot, `${contract.packageName}.swiftmodule`),
            path.join(consumerRoot, "Sources/CEMTokens/CEMTokens.swift"),
        ]);
        run("xcrun", [
            "--sdk",
            "iphonesimulator",
            "swiftc",
            "-typecheck",
            "-parse-as-library",
            "-swift-version",
            contract.swiftLanguageMode,
            "-target",
            target,
            "-sdk",
            sdkPath,
            "-I",
            moduleRoot,
            path.join(consumerRoot, "Examples/CEMTokensExampleApp.swift"),
        ]);

        await fs.mkdir(REPORT_ROOT, { recursive: true });
        await fs.writeFile(
            path.join(REPORT_ROOT, "ios-compile-report.json"),
            `${JSON.stringify(
                {
                    version: 1,
                    platform: "ios",
                    contract,
                    actual: {
                        swift: swiftVersion.split("\n")[0],
                        xcode: xcodeVersion.split("\n")[0],
                        target,
                    },
                    checks: ["swift-package-manifest", "swift-package-release-build", "ios-swiftui-typecheck"],
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

    console.log("compile-ios-platform: Swift Package and iOS SwiftUI clean consumer compiled");
}

main().catch((error) => {
    console.error(error);
    process.exit(1);
});
