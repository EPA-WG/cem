#!/usr/bin/env python3
"""Replay genuine historical encoders; never relabel a current encoder's body."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

FIXTURES = Path(__file__).resolve().parent
WORKSPACE = FIXTURES.parents[4]
PRODUCERS = {
    2: "75fe1fa664cc30e57a8b42fe057c110060b608f9",
    3: "4ba5a9ff626c55de8fac506040cbdea0c33cffb4",
    4: "1d44d28284234e9a74bb80b97aab897bf85c9f97",
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def git(*args):
    return subprocess.check_output(["git", "-C", str(WORKSPACE), *args])


def main():
    parent = WORKSPACE / "dist/fixtures"
    parent.mkdir(parents=True, exist_ok=True)
    records = []
    env = {**os.environ, "CARGO_BUILD_JOBS": "2"}
    with tempfile.TemporaryDirectory(prefix="legacy-cemb-", dir=parent) as temporary:
        temporary = Path(temporary)
        for version, revision in PRODUCERS.items():
            codec = git("show", f"{revision}:packages/cem_ml/src/ast/format.rs")
            assert re.search(rb"pub const VERSION: u16 = " + str(version).encode() + rb";", codec)
            checkout = temporary / f"v{version}"
            checkout.mkdir()
            archive = subprocess.Popen(["git", "-C", str(WORKSPACE), "archive", revision], stdout=subprocess.PIPE)
            try:
                subprocess.run(["tar", "-xf", "-", "-C", str(checkout)], stdin=archive.stdout, check=True)
            finally:
                archive.stdout.close()
            assert archive.wait() == 0
            example = checkout / "packages/cem_ml/examples/legacy_cemb_generate.rs"
            example.parent.mkdir(exist_ok=True)
            shutil.copyfile(FIXTURES / "produce.rs", example)
            command = ["cargo", "+1.96.0", "rustc", "--locked", "--package", "cem-ml",
                       "--no-default-features", "--example", "legacy_cemb_generate",
                       "--target-dir", str(checkout / "dist/target/legacy-fixture"), "--",
                       "--check-cfg=cfg(legacy_references)"]
            if version >= 3:
                command += ["--cfg", "legacy_references"]
            subprocess.run(command, cwd=checkout, env=env, check=True)
            executable = checkout / "dist/target/legacy-fixture/debug/examples/legacy_cemb_generate"
            subprocess.run([str(executable), str(FIXTURES), str(version)], check=True)
            records.append({
                "wireVersion": version, "revision": revision,
                "encoderSha256": sha(git("show", f"{revision}:packages/cem_ml/src/ast/encode.rs")),
                "formatSha256": sha(codec),
                "cargoLockSha256": sha((checkout / "Cargo.lock").read_bytes()),
            })
    files = []
    for binary in sorted(FIXTURES.glob("v*-*.cemb")):
        expected = binary.with_suffix(".expected.json")
        name = re.sub(r"^v\d+-", "", binary.stem)
        source = FIXTURES / f"{name}.cem"
        data = binary.read_bytes()
        assert data[:4] == b"CEMB"
        files.append({"path": binary.name, "wireVersion": int.from_bytes(data[4:6], "little"),
                      "bytes": len(data), "sha256": sha(data), "source": source.name,
                      "sourceSha256": sha(source.read_bytes()), "expected": expected.name,
                      "expectedSha256": sha(expected.read_bytes())})
    receipt = {"schemaVersion": 1, "rustc": subprocess.check_output(["rustc", "+1.96.0", "--version"], text=True).strip(),
               "driverSha256": sha((FIXTURES / "produce.rs").read_bytes()), "producers": records, "files": files}
    (FIXTURES / "provenance.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"Generated {len(files)} immutable legacy fixtures with unchanged historical encoders.")


if __name__ == "__main__":
    main()
