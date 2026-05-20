//! AC-S-4 verification fixture: emit `cem-core/1` Rust headers and run
//! `cargo check` against a generated stub crate that imports them.
//!
//! Per OQ-SC-3 (resolved): rust_hdr is Tier A code behind a Tier B
//! gate. This fixture is **off by default** — it only runs when
//! `CEM_ML_EMIT_RUST=1` is set in the environment. That keeps the Tier
//! A test surface light (no `cargo check` subprocess per CI run) while
//! letting Tier B contributors and the per-release verification step
//! exercise it explicitly.

use std::env;
use std::fs;
use std::process::Command;

use cem_ml::schema::compiler::{
    rust_hdr::RustHdrEmitter, EmissionCursor, SchemaEmitter,
};
use cem_ml::schema::compiler::CompilerOptions;
use cem_ml::schema::ir::CompiledSchema;

#[test]
fn cem_core_rust_header_compiles_under_cargo_check() {
    if env::var_os("CEM_ML_EMIT_RUST").is_none() {
        eprintln!(
            "info: CEM_ML_EMIT_RUST not set — skipping AC-S-4 cargo check fixture (OQ-SC-3 Tier B gate)"
        );
        return;
    }
    let cargo = match env::var("CARGO") {
        Ok(p) => std::path::PathBuf::from(p),
        Err(_) => match resolve_on_path("cargo") {
            Some(p) => p,
            None => {
                eprintln!("info: `cargo` not on PATH — skipping AC-S-4 cargo check fixture");
                return;
            }
        },
    };

    // Emit the .rs header.
    let schema = CompiledSchema::cem_core();
    let opts = CompilerOptions {
        emit_rust: true,
        ..Default::default()
    };
    let mut cursor = EmissionCursor::new(&schema);
    let artifact = RustHdrEmitter
        .emit(&schema, &opts, &mut cursor)
        .expect("rust_hdr emitter produced an artifact");

    // Stub crate skeleton: Cargo.toml + src/lib.rs that consumes the
    // emitted file. Layout matches the design's
    // `cem_ml_schema_stub` reference in §3.4.2.6.
    let tmp = env::temp_dir().join("cem_ml_rust_hdr_stub");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("src")).expect("create stub src");
    fs::write(
        tmp.join("Cargo.toml"),
        concat!(
            "[package]\n",
            "name = \"cem_ml_schema_stub\"\n",
            "version = \"0.0.0\"\n",
            "edition = \"2021\"\n",
            "publish = false\n",
            "\n",
            "[lib]\n",
            "path = \"src/lib.rs\"\n",
        ),
    )
    .expect("write stub Cargo.toml");
    fs::write(tmp.join("src/cem_core.rs"), &artifact.bytes).expect("write emitted .rs");
    fs::write(
        tmp.join("src/lib.rs"),
        concat!(
            "//! AC-S-4 stub crate — verifies the emitted cem-core .rs compiles.\n",
            "pub mod cem_core;\n",
            "\n",
            "#[allow(dead_code)]\n",
            "fn assert_consts_present() {\n",
            "    let _: &str = cem_core::schema::SCHEMA_URI;\n",
            "    let _: &str = cem_core::schema::EMBEDDED_VERSION;\n",
            "}\n",
        ),
    )
    .expect("write stub lib.rs");

    // `--offline` keeps the subprocess from reaching the network; the
    // stub crate has zero dependencies, so the lockfile / index are
    // both irrelevant.
    let output = Command::new(&cargo)
        .args(["check", "--quiet", "--offline"])
        .current_dir(&tmp)
        .env_remove("RUSTC_WRAPPER")
        .output()
        .expect("invoke cargo check");

    assert!(
        output.status.success(),
        "`cargo check` rejected the emitted cem-core .rs:\n\
         --- stderr ---\n{}\n--- stdout ---\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
}

fn resolve_on_path(name: &str) -> Option<std::path::PathBuf> {
    let probe = if cfg!(target_os = "windows") {
        Command::new("where").arg(name).output()
    } else {
        Command::new("which").arg(name).output()
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
        Some(std::path::PathBuf::from(first_line))
    }
}
