use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cem_ml::schema::compiler::CompilerOptions;
use cem_ml::schema::compiler::{ts_dts::TsDtsEmitter, EmissionCursor, SchemaEmitter};
use cem_ml::schema::ir::{CompiledSchema, SemVer};

pub fn resolve_tsc() -> Option<PathBuf> {
    if let Ok(explicit) = env::var("CEM_ML_TSC") {
        let p = PathBuf::from(explicit);
        if p.exists() {
            return Some(p);
        }
    }

    let workspace_tsc =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../node_modules/.bin/tsc");
    if workspace_tsc.exists() {
        return Some(workspace_tsc);
    }

    let probe = if cfg!(target_os = "windows") {
        Command::new("where").arg("tsc").output()
    } else {
        Command::new("which").arg("tsc").output()
    };
    let output = probe.ok()?;
    if !output.status.success() {
        return None;
    }
    let first_line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()?
        .trim()
        .to_owned();
    if first_line.is_empty() {
        None
    } else {
        Some(PathBuf::from(first_line))
    }
}

pub fn prepare_ts_project(test_name: &str) -> PathBuf {
    let tmp = env::temp_dir().join(test_name);
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("node_modules/@epa-wg/cem-ml/wasm")).expect("wasm stub dir");
    fs::create_dir_all(tmp.join("node_modules/@epa-wg/cem-ml/schema/core/1.0.0"))
        .expect("schema 1.0.0 dir");
    fs::create_dir_all(tmp.join("node_modules/@epa-wg/cem-ml/schema/core/2.0.0"))
        .expect("schema 2.0.0 dir");

    fs::write(
        tmp.join("node_modules/@epa-wg/cem-ml/wasm/index.d.ts"),
        concat!(
            "declare const runtimeValidatedBrand: unique symbol;\n",
            "export type Validated<T> = T & { readonly [runtimeValidatedBrand]: true };\n",
            "export declare function asValidated<T>(value: T): Validated<T>;\n",
            "export declare function tryValidated<T>(value: T): Validated<T> | undefined;\n",
        ),
    )
    .expect("write wasm stub");

    write_schema_dts(&tmp, "1.0.0", CompiledSchema::cem_core());
    let mut schema_v2 = CompiledSchema::cem_core();
    schema_v2.version_identity.embedded_version = SemVer::new(2, 0, 0);
    schema_v2.version_identity.fingerprint_input = "2.0.0".to_owned();
    schema_v2.source.version = "2.0.0".to_owned();
    write_schema_dts(&tmp, "2.0.0", schema_v2);

    fs::write(
        tmp.join("tsconfig.json"),
        concat!(
            "{\n",
            "  \"compilerOptions\": {\n",
            "    \"strict\": true,\n",
            "    \"noEmit\": true,\n",
            "    \"target\": \"ES2022\",\n",
            "    \"module\": \"NodeNext\",\n",
            "    \"moduleResolution\": \"NodeNext\",\n",
            "    \"lib\": [\"ES2022\", \"DOM\"]\n",
            "  },\n",
            "  \"include\": [\"fixture.ts\"]\n",
            "}\n",
        ),
    )
    .expect("write tsconfig");

    tmp
}

pub fn run_tsc(tsc: &Path, project: &Path) -> std::process::Output {
    Command::new(tsc)
        .args([
            "--noEmit",
            "-p",
            project.to_str().expect("project path utf-8"),
        ])
        .current_dir(project)
        .output()
        .expect("invoke tsc")
}

fn write_schema_dts(tmp: &Path, version: &str, schema: CompiledSchema) {
    let opts = CompilerOptions::default();
    let mut cursor = EmissionCursor::new(&schema);
    let artifact = TsDtsEmitter
        .emit(&schema, &opts, &mut cursor)
        .expect("ts_dts emitter produced an artifact");
    fs::write(
        tmp.join(format!(
            "node_modules/@epa-wg/cem-ml/schema/core/{version}/cem-core.d.ts"
        )),
        artifact.bytes,
    )
    .expect("write schema dts");
}
