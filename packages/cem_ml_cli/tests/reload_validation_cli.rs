use cem_ml::{
    ast::reload::ReloadSource,
    ast::reload::{ReferenceReloadBundle, ReloadLimits},
    import::import_bytes_with_lexical_scopes,
    schema::vocab::CompiledSchema,
    source::SourceId,
};
use std::{fs, process::Command};

#[test]
fn explicit_validate_and_check_reload_preserve_missing_capture_outcomes() {
    let root = std::env::temp_dir().join(format!("cem-reload-validation-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let text = b"{#input}";
    let capture = import_bytes_with_lexical_scopes(
        text,
        "text/cem-ml",
        "memory:original.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap()
    .captured;
    let limits = ReloadLimits::default();
    let mut bundle = ReferenceReloadBundle::export(
        &capture,
        vec![
            ReloadSource::new(SourceId(0), "memory:original.cem", text, true),
            ReloadSource::new(SourceId(1), "memory:original.cem", text, true),
        ],
        limits,
    )
    .unwrap();
    bundle.lexical = None;
    let file = root.join("source.reload");
    fs::write(&file, bundle.encode(limits).unwrap()).unwrap();
    for command in ["validate", "check"] {
        let output = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .arg(command)
            .arg(&file)
            .args(["--reload-bundle", "--format", "json"])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(1),
            "{command}: {} {}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            report["reportAst"]["validation"]["complete"], false,
            "{report}"
        );
        assert!(report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "cem.reference.missing_lexical_metadata"));
    }
    fs::write(&file, b"not a bundle").unwrap();
    let rejected = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .arg("validate")
        .arg(&file)
        .arg("--reload-bundle")
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("reload"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_identity_flag_requires_explicit_reload_admission() {
    use clap::Parser;
    for command in ["validate", "check"] {
        assert!(cem_ml_cli::cli::Cli::try_parse_from([
            "cem-ml",
            command,
            "input",
            "--reload-source-id",
            "1"
        ])
        .is_err());
        assert!(cem_ml_cli::cli::Cli::try_parse_from([
            "cem-ml",
            command,
            "input",
            "--reload-bundle",
            "--reload-source-id",
            "1"
        ])
        .is_ok());
    }
}
