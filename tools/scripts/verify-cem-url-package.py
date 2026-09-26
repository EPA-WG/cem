#!/usr/bin/env python3
"""Package and test the CEM URL fork without uploading to a registry."""

import hashlib
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import tomllib


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "vendor/url"
TARGET = ROOT / "dist/target/cem_url_package"


def run(*args):
    subprocess.run(args, cwd=ROOT, check=True)


def main():
    manifest = tomllib.loads((SOURCE / "Cargo.toml").read_text())
    package = manifest["package"]
    assert package["name"] == "cem-url"
    assert package["publish"] == ["crates-io"], "fork must declare its release registry"
    assert package["repository"] == "https://github.com/EPA-WG/cem"
    assert package["license"] == "MIT OR Apache-2.0"
    upstream = tomllib.loads((SOURCE / "UPSTREAM-Cargo.toml").read_text())["package"]
    provenance = package["metadata"]["cem-upstream"]
    assert (provenance["package"], provenance["version"]) == (upstream["name"], upstream["version"])
    assert (SOURCE / "UPSTREAM-Cargo.toml").read_bytes() == (SOURCE / "Cargo.toml.orig").read_bytes()
    assert hashlib.sha256((SOURCE / "debug_metadata/url.natvis").read_bytes()).hexdigest() == (
        "3e3e66a7e4d193b05aef4c3bdd2460e28a1c3eaa9999e9de447810894b0acc18")
    version = package["version"]
    consumer = tomllib.loads((ROOT / "packages/cem_ql/Cargo.toml").read_text())
    assert consumer["dependencies"]["url"]["package"] == "cem-url"
    assert consumer["dependencies"]["url"]["version"] == f"={version}"

    # Dirty checkouts are review artifacts; final publication requires a clean commit.
    # Cargo verifies the extracted archive by building it; never use --no-verify here.
    run("cargo", "package", "--locked", "--offline", "--allow-dirty",
        "--manifest-path", str(SOURCE / "Cargo.toml"), "--target-dir", str(TARGET))
    stem = f"cem-url-{version}"
    archive = TARGET / "package" / f"{stem}.crate"
    with tarfile.open(archive) as packed:
        def read(name):
            return packed.extractfile(f"{stem}/{name}").read()

        required = ["LICENSE-MIT", "LICENSE-APACHE", "tests/LICENSE-WPT.md",
                    "README.md", "README.upstream.md", "UPSTREAM-Cargo.toml",
                    "CEM-PATCH.md", "CEM-PARSER.patch", "CEM-PACKAGING.patch", "CEM-FIXED-CASES.txt",
                    "RELEASING.md"]
        # Every shipped source and fixture must equal the reviewed checkout.
        for folder in ["src", "tests", "debug_metadata"]:
            required.extend(str(p.relative_to(SOURCE)) for p in (SOURCE / folder).rglob("*")
                            if p.is_file())
        for name in required:
            assert read(name) == (SOURCE / name).read_bytes(), f"archive drift: {name}"
        normalized = tomllib.loads(read("Cargo.toml").decode())
        assert normalized["package"]["name"] == "cem-url"
        assert normalized["package"]["version"] == version
        assert normalized["package"]["publish"] == ["crates-io"]
        assert not {"workspace", "patch", "replace"}.intersection(normalized)

        def check_dependencies(table):
            for key, value in table.items():
                if key in ("dependencies", "dev-dependencies", "build-dependencies"):
                    for name, dependency in value.items():
                        assert "version" in dependency, f"missing registry version: {name}"
                        assert not {"path", "git", "registry"}.intersection(dependency), name
                elif isinstance(value, dict):
                    check_dependencies(value)

        check_dependencies(normalized)
        vcs = json.loads(read(".cargo_vcs_info.json"))

    # Use an extraction outside the workspace: published manifests have no
    # [workspace] table and must not inherit the repository's workspace.
    with tempfile.TemporaryDirectory(prefix="cem-url-package-") as temporary:
        with tarfile.open(archive) as packed:
            packed.extractall(temporary, filter="data")
        run("cargo", "test", "--locked", "--offline", "--all-features",
            "--manifest-path", str(Path(temporary) / stem / "Cargo.toml"),
            "--target-dir", str(TARGET / "tests"), "--test", "unit", "--test", "url_wpt")
        run("cargo", "check", "--locked", "--offline", "--no-default-features",
            "--manifest-path", str(Path(temporary) / stem / "Cargo.toml"),
            "--target-dir", str(TARGET / "tests"))
    report = {"package": "cem-url", "version": version,
              "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
              "vcs": vcs, "verifiedFiles": len(required), "published": False}
    (TARGET / "package" / "cem-url-review.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
