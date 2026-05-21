//! AC-S-2 RELAX NG XML parity fixture: validate canonical CEM-annotated
//! XML against the emitted `cem-core/1` `.rng` through
//! `xmllint --relaxng` (libxml2).
//!
//! Per OQ-SC-5 (resolved): `xmllint` is the chosen native validator.
//! When the binary is absent or `CEM_ML_SCHEMA_PARITY_SKIP=1` is set, the
//! fixture **skips** (recorded as `info`, not a failure) so contributors
//! without libxml2 installed can run the rest of the suite. The Nx CI
//! image ships libxml2 by default.

use std::env;
use std::fs;
use std::process::Command;

use cem_ml::schema::compiler::CompilerOptions;
use cem_ml::schema::compiler::{rng_xml::RngXmlEmitter, EmissionCursor, SchemaEmitter};
use cem_ml::schema::ir::CompiledSchema;

/// CEM-annotated fixture documents. Element names are unprefixed (so
/// matched by the grammar's `<anyName/>`); the cem-namespaced
/// attributes carry enum values from the cem-core/1 vocabulary.
const BASIC_CEM_ANNOTATED_FIXTURE_XML: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<root xmlns:cem=\"https://cem.dev/ns/core/1\" cem:badge=\"success\">\n",
    "  <child cem:action=\"primary\" cem:state=\"hover\"/>\n",
    "  <child cem:message=\"sent\"/>\n",
    "</root>\n",
);

const PASS_THROUGH_ATTRS_FIXTURE_XML: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<button xmlns:cem=\"https://cem.dev/ns/core/1\" id=\"save\" class=\"primary\" role=\"button\" aria-label=\"Save\" data-track=\"save\" cem:action=\"primary\" cem:state=\"loading hover\">Save</button>\n",
);

// Structural negative: a `cem:state` token outside the schema-wide
// state matrix. Per-annotation state narrowing (e.g. `cem:badge` ⇒
// state ∈ {default}) is an AC-S-8 semantic rule, NOT a structural
// RELAX NG constraint — the mirror validates state tokens against the
// global matrix only (see rng_xml.rs module header).
const UNKNOWN_STATE_TOKEN_FIXTURE_XML: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<span xmlns:cem=\"https://cem.dev/ns/core/1\" cem:state=\"not-a-real-state\">Done</span>\n",
);

const UNKNOWN_CEM_ATTR_FIXTURE_XML: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<span xmlns:cem=\"https://cem.dev/ns/core/1\" cem:made-up=\"x\">Done</span>\n",
);

#[test]
fn cem_core_rng_validates_canonical_fixture_through_xmllint() {
    if schema_parity_skip_requested() {
        if schema_parity_required() {
            panic!(
                "CEM_ML_SCHEMA_PARITY_SKIP cannot be used when schema parity validation is required"
            );
        }
        eprintln!(
            "info: CEM_ML_SCHEMA_PARITY_SKIP set — skipping AC-S-2 xmllint parity validation (OQ-SC-5 escape hatch)"
        );
        return;
    }
    let xmllint = match resolve_xmllint() {
        Some(path) => path,
        None => {
            if schema_parity_required() {
                panic!(
                    "`xmllint` not on PATH while schema parity validation is required; install libxml2-utils or set CEM_ML_XMLLINT"
                );
            }
            eprintln!(
                "info: `xmllint` not on PATH — skipping AC-S-2 xmllint parity validation (OQ-SC-5; install libxml2 to exercise)"
            );
            return;
        }
    };

    // Emit the grammar.
    let schema = CompiledSchema::cem_core();
    let opts = CompilerOptions::default();
    let mut cursor = EmissionCursor::new(&schema);
    let artifact = RngXmlEmitter
        .emit(&schema, &opts, &mut cursor)
        .expect("rng_xml emitter produced an artifact");

    // Write files into a temp dir so xmllint can read them.
    let tmp = env::temp_dir().join("cem_ml_rng_xml_parity");
    fs::create_dir_all(&tmp).expect("tmp dir");
    let rng_path = tmp.join("cem-core.rng");
    fs::write(&rng_path, &artifact.bytes).expect("write rng");

    assert_validation(
        &xmllint,
        &rng_path,
        &tmp.join("basic.xml"),
        BASIC_CEM_ANNOTATED_FIXTURE_XML,
        true,
        "basic CEM annotated fixture",
    );
    assert_validation(
        &xmllint,
        &rng_path,
        &tmp.join("pass-through.xml"),
        PASS_THROUGH_ATTRS_FIXTURE_XML,
        true,
        "pass-through attributes and multi-state list",
    );
    assert_validation(
        &xmllint,
        &rng_path,
        &tmp.join("unknown-state-token.xml"),
        UNKNOWN_STATE_TOKEN_FIXTURE_XML,
        false,
        "cem:state token outside the schema-wide state matrix",
    );
    assert_validation(
        &xmllint,
        &rng_path,
        &tmp.join("unknown-cem-attr.xml"),
        UNKNOWN_CEM_ATTR_FIXTURE_XML,
        false,
        "unknown active-CEM namespace attribute",
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
        "xmllint validation mismatch for {label}; expected valid={expect_valid}:\n\
         --- stderr ---\n{}\n--- stdout ---\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
}

fn resolve_xmllint() -> Option<std::path::PathBuf> {
    if let Ok(explicit) = env::var("CEM_ML_XMLLINT") {
        let p = std::path::PathBuf::from(explicit);
        if p.exists() {
            return Some(p);
        }
    }
    // Best-effort PATH lookup. Linux/macOS use `which`; Windows uses
    // `where` (cem-ml CI runs Linux today, so the `which` arm is the
    // only one exercised in practice).
    let probe = if cfg!(target_os = "windows") {
        Command::new("where").arg("xmllint").output()
    } else {
        Command::new("which").arg("xmllint").output()
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

fn schema_parity_required() -> bool {
    env::var_os("CEM_ML_SCHEMA_PARITY_REQUIRED").is_some() || env::var_os("CI").is_some()
}

fn schema_parity_skip_requested() -> bool {
    env::var_os("CEM_ML_SCHEMA_PARITY_SKIP").is_some()
}
