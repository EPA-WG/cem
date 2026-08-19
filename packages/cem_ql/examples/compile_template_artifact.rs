//! Minimal native build tool for Phase 3C component-template artifacts.

use std::env;
use std::fs;
use std::path::Path;

use cem_ql::render::CompileTemplateOptions;
use cem_ql::template_artifact::{compile_template_artifact, TemplateArtifactSourceMapMode};
use serde_json::json;

fn main() {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() < 4 {
        fail(
            "usage: compile_template_artifact <source.cem> <output.cem-template.bin> <dev|prod> <policy-stamp> [host-binding ...]",
        );
    }
    let source_path = &arguments[0];
    let output_path = &arguments[1];
    let source_map_mode = match arguments[2].as_str() {
        "dev" => TemplateArtifactSourceMapMode::Dev,
        "prod" => TemplateArtifactSourceMapMode::Prod,
        value => fail(&format!("unsupported source-map mode `{value}`")),
    };
    let policy_stamp = &arguments[3];
    let host_bindings = arguments[4..].to_vec();
    let source = fs::read_to_string(source_path)
        .unwrap_or_else(|error| fail(&format!("read `{source_path}`: {error}")));
    let artifact = compile_template_artifact(
        &source,
        &CompileTemplateOptions {
            host_bindings,
            ..CompileTemplateOptions::default()
        },
        source_map_mode,
    );
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|error| fail(&format!("create `{}`: {error}", parent.display())));
    }
    fs::write(output_path, &artifact.bytes)
        .unwrap_or_else(|error| fail(&format!("write `{output_path}`: {error}")));

    let manifest_path = format!("{output_path}.json");
    let manifest = json!({
        "kind": "template-artifact",
        "payloadKey": {
            "contentType": "cem-template-artifact",
            "sourceHash": artifact.identity.source_hash.header_value(),
            "cemMlVersion": artifact.identity.cem_ml_version,
            "cemQlVersion": artifact.identity.cem_ql_version,
            "sourceMapMode": artifact.identity.source_map_mode.as_str(),
        },
        "cacheKey": artifact.content_hash.header_value(),
        "formatVersion": artifact.identity.artifact_version,
        "policyStamp": policy_stamp,
        "bytes": Path::new(output_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(output_path),
    });
    fs::write(
        &manifest_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&manifest).expect("artifact manifest serializes")
        ),
    )
    .unwrap_or_else(|error| fail(&format!("write `{manifest_path}`: {error}")));
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(2);
}
