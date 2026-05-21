//! AC-S-2 RELAX NG compact fixture: emit cem-core/1 `.rnc`, convert it
//! to `.rng` through Trang, then validate positive and negative
//! documents with `xmllint --relaxng`.
//!
//! The fixture skips when Trang or xmllint is absent so local development
//! remains usable without Java/libxml2 tooling. CI/release images should
//! provide both tools to exercise the compact emitter as a real consumer.

use std::env;
use std::fs;
use std::process::Command;

use cem_ml::schema::compiler::CompilerOptions;
use cem_ml::schema::compiler::{rng_compact::RngCompactEmitter, EmissionCursor, SchemaEmitter};
use cem_ml::schema::ir::CompiledSchema;

const VALID_MULTI_STATE_XML: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<button xmlns:cem=\"https://cem.dev/ns/core/1\" id=\"save\" class=\"primary\" role=\"button\" aria-label=\"Save\" data-track=\"save\" cem:action=\"primary\" cem:state=\"loading hover\">Save</button>\n",
);

// Structural negative: a `cem:state` token outside the schema-wide
// state matrix. Per-annotation state narrowing (e.g. `cem:badge` ⇒
// state ∈ {default}) is an AC-S-8 semantic rule, not a structural
// RELAX NG constraint — the mirror checks state tokens against the
// global matrix only (see rng_xml.rs module header).
const UNKNOWN_STATE_TOKEN_XML: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<span xmlns:cem=\"https://cem.dev/ns/core/1\" cem:state=\"not-a-real-state\">Done</span>\n",
);

#[test]
fn cem_core_rnc_round_trips_through_trang_and_xmllint() {
    if env::var_os("CEM_ML_SCHEMA_ORACLE_SKIP").is_some() {
        eprintln!("info: CEM_ML_SCHEMA_ORACLE_SKIP set — skipping AC-S-2 Trang/xmllint oracle");
        return;
    }
    let trang = match resolve_trang() {
        Some(path) => path,
        None => {
            if schema_oracle_required() {
                panic!(
                    "trang binary not found while schema oracle is required; run `yarn nx run @epa-wg/trang-native:build` or set CEM_ML_TRANG"
                );
            }
            eprintln!(
                "info: trang binary not found (set CEM_ML_TRANG, run `nx run @epa-wg/trang-native:build`, or install Trang on PATH) — skipping AC-S-2 compact round-trip oracle",
            );
            return;
        }
    };
    eprintln!("info: using trang at {}", trang.display());
    // xmllint is a separate optional dependency: missing it lets us
    // still exercise the Trang round-trip but skip the schema-driven
    // validation assertions.
    let xmllint = resolve_on_path("xmllint", "CEM_ML_XMLLINT");

    let schema = CompiledSchema::cem_core();
    let opts = CompilerOptions::default();
    let mut cursor = EmissionCursor::new(&schema);
    let artifact = RngCompactEmitter
        .emit(&schema, &opts, &mut cursor)
        .expect("rng_compact emitter produced an artifact");

    let tmp = env::temp_dir().join("cem_ml_rng_compact_roundtrip");
    fs::create_dir_all(&tmp).expect("tmp dir");
    let rnc_path = tmp.join("cem-core.rnc");
    let rng_path = tmp.join("cem-core-from-rnc.rng");
    fs::write(&rnc_path, &artifact.bytes).expect("write rnc");

    let trang_output = Command::new(&trang)
        .args([
            rnc_path.to_str().expect("rnc path utf-8"),
            rng_path.to_str().expect("rng path utf-8"),
        ])
        .output()
        .expect("invoke trang");
    assert!(
        trang_output.status.success(),
        "Trang rejected the emitted RELAX NG compact grammar:\n--- stderr ---\n{}\n--- stdout ---\n{}",
        String::from_utf8_lossy(&trang_output.stderr),
        String::from_utf8_lossy(&trang_output.stdout),
    );

    let Some(xmllint) = xmllint else {
        if schema_oracle_required() {
            panic!(
                "`xmllint` not on PATH while schema oracle is required; install libxml2-utils or set CEM_ML_XMLLINT"
            );
        }
        eprintln!(
            "info: `xmllint` not on PATH — Trang round-trip OK, skipping xmllint validation",
        );
        return;
    };

    assert_validation(
        &xmllint,
        &rng_path,
        &tmp.join("valid-multi-state.xml"),
        VALID_MULTI_STATE_XML,
        true,
        "valid multi-state + pass-through attributes",
    );
    assert_validation(
        &xmllint,
        &rng_path,
        &tmp.join("unknown-state-token.xml"),
        UNKNOWN_STATE_TOKEN_XML,
        false,
        "cem:state token outside the schema-wide state matrix",
    );
}

fn assert_validation(
    xmllint: &std::path::Path,
    rng_path: &std::path::Path,
    xml_path: &std::path::Path,
    xml: &str,
    expect_valid: bool,
    label: &str,
) {
    fs::write(xml_path, xml).expect("write fixture xml");
    let output = Command::new(xmllint)
        .args([
            "--noout",
            "--relaxng",
            rng_path.to_str().expect("rng path utf-8"),
            xml_path.to_str().expect("xml path utf-8"),
        ])
        .output()
        .expect("invoke xmllint");
    assert!(
        output.status.success() == expect_valid,
        "xmllint validation mismatch for {label}; expected valid={expect_valid}:\n--- stderr ---\n{}\n--- stdout ---\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
}

/// Locate the Trang binary, preferring (in order):
/// 1. `CEM_ML_TRANG` env var (escape hatch / explicit path).
/// 2. `@epa-wg/trang-native` workspace package: the GraalVM-compiled
///    binary lives at `<workspace>/node_modules/@epa-wg/trang-native/
///    build/native/<triple>/trang` after either `yarn nx run
///    @epa-wg/trang-native:build` (workspace) or the postinstall
///    download (consumer).
/// 3. `trang` on PATH (system install).
fn resolve_trang() -> Option<std::path::PathBuf> {
    // 1 + 3 — env var and PATH probe are both handled by resolve_on_path.
    if let Ok(explicit) = env::var("CEM_ML_TRANG") {
        let p = std::path::PathBuf::from(explicit);
        if p.exists() {
            return Some(p);
        }
    }
    if let Some(path) = trang_native_package_path() {
        if path.exists() {
            return Some(path);
        }
    }
    resolve_on_path("trang", "CEM_ML_TRANG")
}

fn trang_native_package_path() -> Option<std::path::PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR"); // <repo>/packages/cem_ml
    let workspace = std::path::Path::new(manifest_dir).parent()?.parent()?;
    let triple = host_triple()?;
    let binary = if cfg!(target_os = "windows") { "trang.exe" } else { "trang" };
    // Workspace symlink (yarn workspace) and consumer install both land
    // under node_modules/@epa-wg/trang-native; the build output sits at
    // build/native/<triple>/, postinstall extracts to bin/native/<triple>/.
    let candidates = [
        workspace.join("node_modules/@epa-wg/trang-native/build/native").join(triple).join(binary),
        workspace.join("node_modules/@epa-wg/trang-native/bin/native").join(triple).join(binary),
    ];
    candidates.into_iter().find(|p| p.exists())
}

fn host_triple() -> Option<&'static str> {
    let triple = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("windows", "x86_64") => "windows-x86_64",
        ("macos", "aarch64") => "macos-aarch64",
        _ => return None,
    };
    Some(triple)
}

fn resolve_on_path(name: &str, env_var: &str) -> Option<std::path::PathBuf> {
    if let Ok(explicit) = env::var(env_var) {
        let p = std::path::PathBuf::from(explicit);
        if p.exists() {
            return Some(p);
        }
    }
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

fn schema_oracle_required() -> bool {
    env::var_os("CEM_ML_SCHEMA_ORACLE_REQUIRED").is_some() || env::var_os("CI").is_some()
}
